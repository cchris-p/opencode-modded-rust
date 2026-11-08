//! Persistent workspace-map supervisor (`SCOPE-017`).
//!
//! One [`WorkspaceEngine`] per canonical workspace root is owned by a dedicated
//! worker thread. The worker subscribes to `opencode-watcher`, coalesces
//! add/change/unlink events into a dirty set, and drives incremental engine
//! updates. Queries and other engine work are serialized on the same worker, so
//! no raw pointer is shared and no context is created per call.
//!
//! The supervisor records freshness measurements only. It never decides whether
//! a stale map should fall back (that is the runtime's policy), and it never
//! writes task state.

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use opencode_types::workspace::{
    is_code_path, is_excluded_dir, path_has_excluded_dir, MapFreshness, MapFreshnessReport,
    ReAnchorResult, WorkspaceSearch, WorkspaceSearchRequest, WorkspaceSearchResponse,
};
use opencode_types::RetrievalFallbackReason;
use opencode_watcher::{FileEvent, FileWatcher, WatcherConfig, WatcherEvent};

use crate::engine::{
    search_hit_candidate, DeltaEntryView, EngineCounters, MapItemView, PlanNodeInput,
    ReconcileView, SearchHitView, SearchRequest, WorkspaceEngine,
};

/// Default debounce window for coalescing watcher events.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(100);
/// Default idle TTL before an unused map is evicted.
pub const DEFAULT_TTL: Duration = Duration::from_secs(30 * 60);

/// Supervisor/worker error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorError {
    /// The workspace root is missing, not a directory, or the initial map build
    /// failed. Reported explicitly rather than silently degrading.
    Unavailable(String),
    /// The worker thread has stopped.
    WorkerStopped,
    /// A query job failed or panicked.
    JobFailed(String),
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "workspace map unavailable: {reason}"),
            Self::WorkerStopped => f.write_str("workspace map worker stopped"),
            Self::JobFailed(reason) => write!(f, "workspace map job failed: {reason}"),
        }
    }
}

impl std::error::Error for SupervisorError {}

type BoxedJob = Box<dyn FnOnce(&mut WorkspaceEngine) -> Box<dyn Any + Send> + Send + 'static>;
type BoxedReply = Sender<Result<Box<dyn Any + Send>, SupervisorError>>;

struct CommandMsg {
    job: BoxedJob,
    reply: BoxedReply,
}

enum MapEvent {
    File(WatcherEvent),
    Command(CommandMsg),
    Shutdown,
}

/// One persistent map, owned by its worker thread.
pub struct MapEntry {
    root: PathBuf,
    sender: Sender<MapEvent>,
    worker: Mutex<Option<JoinHandle<()>>>,
    refs: AtomicUsize,
    last_used: Mutex<Instant>,
    freshness: Arc<RwLock<MapFreshnessReport>>,
}

impl MapEntry {
    /// Canonical workspace root this map owns.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Latest recorded freshness measurement.
    pub fn freshness(&self) -> MapFreshnessReport {
        self.freshness
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Record the re-anchor result of a projected plan node (measurement only).
    pub fn record_re_anchor(&self, result: ReAnchorResult) {
        self.freshness
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .re_anchor = Some(result);
    }

    /// Materialize one projected plan node into the persistent map.
    pub fn materialize_plan_node(
        &self,
        task_id: String,
        input: PlanNodeInput,
    ) -> Result<(), SupervisorError> {
        self.with_engine(move |engine| engine.materialize_plan_node(&task_id, &input))
    }

    /// Run a closure against the worker-owned engine.
    ///
    /// The closure runs on the map's single worker thread, which serializes all
    /// engine access. It must be `Send` and `'static`; it must not block on this
    /// map again (that would deadlock the worker).
    pub fn with_engine<R, F>(&self, f: F) -> Result<R, SupervisorError>
    where
        F: FnOnce(&mut WorkspaceEngine) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (reply_tx, reply_rx) = mpsc::channel();
        let job: BoxedJob = Box::new(move |engine| Box::new(f(engine)) as Box<dyn Any + Send>);
        self.sender
            .send(MapEvent::Command(CommandMsg {
                job,
                reply: reply_tx,
            }))
            .map_err(|_| SupervisorError::WorkerStopped)?;
        let boxed = reply_rx
            .recv()
            .map_err(|_| SupervisorError::WorkerStopped)??;
        boxed
            .downcast::<R>()
            .map(|value| *value)
            .map_err(|_| SupervisorError::JobFailed("job returned an unexpected type".into()))
    }

    /// Incremental rebuild counters from the worker-owned engine.
    pub fn counters(&self) -> Result<EngineCounters, SupervisorError> {
        self.with_engine(|engine| engine.counters())
    }

    /// Tiered search over the persistent map.
    pub fn search(&self, request: SearchRequest) -> Result<Vec<SearchHitView>, SupervisorError> {
        self.with_engine(move |engine| engine.search(request).map_err(|error| error.0))?
            .map_err(SupervisorError::JobFailed)
    }

    /// Delta slice from the persistent map.
    pub fn delta(
        &self,
        task_id: Option<String>,
        stage: String,
    ) -> Result<Vec<DeltaEntryView>, SupervisorError> {
        self.with_engine(move |engine| engine.delta(task_id.as_deref(), &stage))
    }

    /// Resolve slice from the persistent map.
    pub fn resolve(
        &self,
        task_id: Option<String>,
        stage: String,
    ) -> Result<Vec<MapItemView>, SupervisorError> {
        self.with_engine(move |engine| engine.resolve(task_id.as_deref(), &stage))
    }

    /// Duplicate clusters from the persistent map.
    pub fn duplicates(&self, scope: Option<String>) -> Result<Vec<MapItemView>, SupervisorError> {
        self.with_engine(move |engine| engine.duplicates(scope.as_deref()))
    }

    /// Observability slice from the persistent map.
    pub fn observability(&self, symbol: String) -> Result<Vec<MapItemView>, SupervisorError> {
        self.with_engine(move |engine| engine.observability(&symbol))
    }

    /// Change-impact slice from the persistent map.
    pub fn change_impact(&self, files: Vec<String>) -> Result<Vec<MapItemView>, SupervisorError> {
        self.with_engine(move |engine| engine.change_impact(&files))
    }

    /// Reconcile materialized plan nodes against parsed state.
    pub fn reconcile_plan_nodes(&self) -> Result<Vec<ReconcileView>, SupervisorError> {
        self.with_engine(|engine| engine.reconcile_plan_nodes())
    }

    fn shutdown(&self) {
        let handle = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(handle) = handle {
            let _ = self.sender.send(MapEvent::Shutdown);
            let _ = handle.join();
        }
    }
}

/// Handle to a persistent map. Dropping it releases one reference; the map stays
/// resident until the registry's idle eviction or shutdown.
pub struct MapHandle {
    entry: Arc<MapEntry>,
}

impl MapHandle {
    pub fn root(&self) -> &Path {
        self.entry.root()
    }

    pub fn freshness(&self) -> MapFreshnessReport {
        self.entry.freshness()
    }

    pub fn counters(&self) -> Result<EngineCounters, SupervisorError> {
        self.entry.counters()
    }

    pub fn search(&self, request: SearchRequest) -> Result<Vec<SearchHitView>, SupervisorError> {
        self.entry.search(request)
    }

    pub fn delta(
        &self,
        task_id: Option<String>,
        stage: String,
    ) -> Result<Vec<DeltaEntryView>, SupervisorError> {
        self.entry.delta(task_id, stage)
    }

    pub fn resolve(
        &self,
        task_id: Option<String>,
        stage: String,
    ) -> Result<Vec<MapItemView>, SupervisorError> {
        self.entry.resolve(task_id, stage)
    }

    pub fn duplicates(&self, scope: Option<String>) -> Result<Vec<MapItemView>, SupervisorError> {
        self.entry.duplicates(scope)
    }

    pub fn observability(&self, symbol: String) -> Result<Vec<MapItemView>, SupervisorError> {
        self.entry.observability(symbol)
    }

    pub fn change_impact(&self, files: Vec<String>) -> Result<Vec<MapItemView>, SupervisorError> {
        self.entry.change_impact(files)
    }

    pub fn reconcile_plan_nodes(&self) -> Result<Vec<ReconcileView>, SupervisorError> {
        self.entry.reconcile_plan_nodes()
    }

    pub fn record_re_anchor(&self, result: ReAnchorResult) {
        self.entry.record_re_anchor(result);
    }

    pub fn materialize_plan_node(
        &self,
        task_id: String,
        input: PlanNodeInput,
    ) -> Result<(), SupervisorError> {
        self.entry.materialize_plan_node(task_id, input)
    }

    /// True when both handles point at the same worker-owned map.
    pub fn is_same_map(&self, other: &MapHandle) -> bool {
        Arc::ptr_eq(&self.entry, &other.entry)
    }
}

impl std::fmt::Debug for MapHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapHandle")
            .field("root", &self.entry.root)
            .finish()
    }
}

impl std::ops::Deref for MapHandle {
    type Target = MapEntry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl Drop for MapHandle {
    fn drop(&mut self) {
        self.entry.refs.fetch_sub(1, Ordering::AcqRel);
    }
}

/// One map per canonical workspace root, with idle eviction and shutdown.
pub struct WorkspaceMapRegistry {
    maps: Mutex<HashMap<PathBuf, Arc<MapEntry>>>,
    debounce: Duration,
    ttl: Duration,
}

impl Default for WorkspaceMapRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_DEBOUNCE, DEFAULT_TTL)
    }
}

impl WorkspaceMapRegistry {
    pub fn new(debounce: Duration, ttl: Duration) -> Self {
        Self {
            maps: Mutex::new(HashMap::new()),
            debounce,
            ttl,
        }
    }

    /// Acquire the persistent map for `root`, creating it on first use.
    ///
    /// Creation failure (missing root, engine/watcher failure) is returned as
    /// [`SupervisorError::Unavailable`], never silently ignored.
    pub fn acquire(&self, root: &str) -> Result<MapHandle, SupervisorError> {
        let canonical = canonical_root(root)?;
        let mut maps = self
            .maps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = maps.get(&canonical) {
            entry.refs.fetch_add(1, Ordering::AcqRel);
            *entry
                .last_used
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Instant::now();
            return Ok(MapHandle {
                entry: entry.clone(),
            });
        }

        let entry = self.spawn_map(canonical.clone())?;
        entry.refs.store(1, Ordering::Release);
        maps.insert(canonical, entry.clone());
        Ok(MapHandle { entry })
    }

    /// Number of resident maps (primarily for tests/observability).
    pub fn len(&self) -> usize {
        self.maps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Evict maps with no live handles idle for at least `ttl`.
    ///
    /// Passed explicitly so callers/tests can sweep with a zero TTL.
    pub fn sweep(&self, ttl: Duration) {
        let now = Instant::now();
        let mut maps = self
            .maps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let expired: Vec<PathBuf> = maps
            .iter()
            .filter(|(_, entry)| {
                entry.refs.load(Ordering::Acquire) == 0
                    && now.saturating_duration_since(
                        *entry
                            .last_used
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()),
                    ) >= ttl
            })
            .map(|(root, _)| root.clone())
            .collect();
        for root in expired {
            if let Some(entry) = maps.remove(&root) {
                entry.shutdown();
            }
        }
    }

    /// Evict idle maps using the registry's configured TTL.
    pub fn sweep_idle(&self) {
        self.sweep(self.ttl);
    }

    /// Stop every map and join its worker.
    pub fn shutdown(&self) {
        let entries: Vec<Arc<MapEntry>> = {
            let mut maps = self
                .maps
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            maps.drain().map(|(_, entry)| entry).collect()
        };
        for entry in entries {
            entry.shutdown();
        }
    }

    fn spawn_map(&self, root: PathBuf) -> Result<Arc<MapEntry>, SupervisorError> {
        let (event_tx, event_rx) = mpsc::channel();
        let (init_tx, init_rx) = mpsc::channel();
        let freshness = Arc::new(RwLock::new(MapFreshnessReport::default()));

        let worker_root = root.clone();
        let worker_events = event_tx.clone();
        let worker_freshness = freshness.clone();
        let debounce = self.debounce;
        let worker = thread::Builder::new()
            .name(format!("scopemux-map:{}", root.display()))
            .spawn(move || {
                worker_main(
                    worker_root,
                    worker_events,
                    event_rx,
                    worker_freshness,
                    debounce,
                    init_tx,
                )
            })
            .map_err(|error| {
                SupervisorError::Unavailable(format!("worker spawn failed: {error}"))
            })?;

        match init_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = worker.join();
                return Err(error);
            }
            Err(_) => {
                let _ = worker.join();
                return Err(SupervisorError::Unavailable(
                    "map worker exited during initialization".to_string(),
                ));
            }
        }

        Ok(Arc::new(MapEntry {
            root,
            sender: event_tx,
            worker: Mutex::new(Some(worker)),
            refs: AtomicUsize::new(0),
            last_used: Mutex::new(Instant::now()),
            freshness,
        }))
    }
}

impl Drop for WorkspaceMapRegistry {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// `WorkspaceSearch` capability backed by the shared registry.
///
/// The tool receives this through `ToolContext`; it only sees the neutral trait
/// and never FFI, model strings, or the registry itself.
pub struct RegistryWorkspaceSearch {
    registry: Arc<WorkspaceMapRegistry>,
}

impl RegistryWorkspaceSearch {
    pub fn new(registry: Arc<WorkspaceMapRegistry>) -> Self {
        Self { registry }
    }
}

impl WorkspaceSearch for RegistryWorkspaceSearch {
    fn search_code(&self, request: &WorkspaceSearchRequest) -> WorkspaceSearchResponse {
        let Ok(map) = self.registry.acquire(&request.workspace_root) else {
            return WorkspaceSearchResponse {
                candidates: Vec::new(),
                freshness: MapFreshness::Unknown,
                fallback_reason: Some(RetrievalFallbackReason::ProviderUnavailable),
            };
        };
        let freshness = map.freshness().state;
        match map.search(SearchRequest {
            query: request.query.clone(),
            anchor_file: None,
            max_hits: request.max_results,
        }) {
            Ok(hits) => {
                let candidates: Vec<_> =
                    hits.into_iter().filter_map(search_hit_candidate).collect();
                let fallback_reason = candidates
                    .is_empty()
                    .then_some(RetrievalFallbackReason::NoMatches);
                WorkspaceSearchResponse {
                    candidates,
                    freshness,
                    fallback_reason,
                }
            }
            Err(_) => WorkspaceSearchResponse {
                candidates: Vec::new(),
                freshness,
                fallback_reason: Some(RetrievalFallbackReason::ProviderUnavailable),
            },
        }
    }
}

fn canonical_root(root: &str) -> Result<PathBuf, SupervisorError> {
    let path = std::fs::canonicalize(root).map_err(|error| {
        SupervisorError::Unavailable(format!("cannot canonicalize {root}: {error}"))
    })?;
    if !path.is_dir() {
        return Err(SupervisorError::Unavailable(format!(
            "{root} is not a directory"
        )));
    }
    Ok(path)
}

fn worker_main(
    root: PathBuf,
    event_tx: Sender<MapEvent>,
    event_rx: Receiver<MapEvent>,
    freshness: Arc<RwLock<MapFreshnessReport>>,
    debounce: Duration,
    init_tx: Sender<Result<(), SupervisorError>>,
) {
    let mut engine = match WorkspaceEngine::open(&root.to_string_lossy()) {
        Ok(engine) => engine,
        Err(error) => {
            let _ = init_tx.send(Err(SupervisorError::Unavailable(error.0)));
            return;
        }
    };

    let watcher = match FileWatcher::new(WatcherConfig::default()) {
        Ok(watcher) => watcher,
        Err(error) => {
            let _ = init_tx.send(Err(SupervisorError::Unavailable(format!(
                "watcher create failed: {error}"
            ))));
            return;
        }
    };
    if let Err(error) = watcher.watch(&root) {
        let _ = init_tx.send(Err(SupervisorError::Unavailable(format!(
            "watcher failed for {}: {error}",
            root.display()
        ))));
        return;
    }

    // Bridge the tokio broadcast into the worker's std channel.
    let mut broadcast_rx = watcher.subscribe();
    let bridge_tx = event_tx.clone();
    thread::Builder::new()
        .name(format!("scopemux-map-watch:{}", root.display()))
        .spawn(move || loop {
            match broadcast_rx.blocking_recv() {
                Ok(event) => {
                    if bridge_tx.send(MapEvent::File(event)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        })
        .ok();

    if let Err(error) = build_initial(&mut engine, &root, &freshness) {
        let _ = init_tx.send(Err(error));
        return;
    }
    let _ = init_tx.send(Ok(()));

    while let Ok(first) = event_rx.recv() {
        match first {
            MapEvent::Shutdown => break,
            MapEvent::Command(command) => handle_command(&mut engine, command),
            MapEvent::File(event) => {
                let mut batch = vec![(event, Instant::now())];
                let deadline = Instant::now() + debounce;
                let mut commands = Vec::new();
                let mut shutdown = false;
                loop {
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    match event_rx.recv_timeout(deadline - now) {
                        Ok(MapEvent::File(event)) => batch.push((event, Instant::now())),
                        Ok(MapEvent::Command(command)) => commands.push(command),
                        Ok(MapEvent::Shutdown) => {
                            shutdown = true;
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
                apply_file_batch(&mut engine, &root, batch, &freshness);
                for command in commands {
                    handle_command(&mut engine, command);
                }
                if shutdown {
                    break;
                }
            }
        }
    }
}

fn build_initial(
    engine: &mut WorkspaceEngine,
    root: &Path,
    freshness: &RwLock<MapFreshnessReport>,
) -> Result<(), SupervisorError> {
    let started = Instant::now();
    let mut files = Vec::new();
    discover_supported_files(root, root, &mut files);
    for file in &files {
        engine.add_file(&file.to_string_lossy());
    }
    if !engine.parse_all() {
        return Err(SupervisorError::Unavailable(
            "initial project parse failed".to_string(),
        ));
    }
    if !engine.resolve_references() {
        tracing::warn!("scopemux supervisor: reference resolution returned false");
    }
    if !engine.rebuild_ir() || !engine.rebuild_info_blocks() {
        return Err(SupervisorError::Unavailable(
            "initial map build failed".to_string(),
        ));
    }

    let mut report = freshness
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    report.state = MapFreshness::Fresh;
    report.last_revision = Some(1);
    report.dirty_set_size = 0;
    report.watcher_lag_ms = None;
    report.last_rebuild_duration_ms = Some(started.elapsed().as_millis() as u64);
    Ok(())
}

fn discover_supported_files(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if name.starts_with('.') || is_excluded_dir(&name) {
                continue;
            }
            discover_supported_files(&path, root, out);
        } else if file_type.is_file() {
            let path_text = path.to_string_lossy();
            if is_code_path(&path_text) && !is_ignored(root, &path) {
                out.push(path);
            }
        }
    }
}

fn is_ignored(root: &Path, path: &Path) -> bool {
    if path_has_excluded_dir(path) {
        return true;
    }
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| name.starts_with('.'))
    })
}

fn apply_file_batch(
    engine: &mut WorkspaceEngine,
    root: &Path,
    batch: Vec<(WatcherEvent, Instant)>,
    freshness: &RwLock<MapFreshnessReport>,
) {
    if batch.is_empty() {
        return;
    }
    let started = batch
        .iter()
        .map(|(_, at)| *at)
        .min()
        .unwrap_or_else(Instant::now);

    // Coalesce to the last event per path and drop ignored/non-code paths.
    let mut events: HashMap<PathBuf, FileEvent> = HashMap::new();
    for (event, _) in batch {
        if !is_code_path(&event.file.to_string_lossy()) || is_ignored(root, &event.file) {
            continue;
        }
        events.insert(event.file, event.event);
    }
    if events.is_empty() {
        return;
    }

    let dirty_set_size = events.len();
    tracing::debug!(
        dirty_set_size,
        "scopemux supervisor: applying coalesced batch"
    );
    let mut failed = false;
    let mut applied = false;
    for (path, event) in &events {
        let path = path.to_string_lossy();
        match event {
            FileEvent::Add | FileEvent::Change => match engine.update_file(&path) {
                Ok(changed) => applied |= changed,
                Err(error) => {
                    failed = true;
                    tracing::warn!(
                        path = %path,
                        "scopemux supervisor: incremental update failed: {error}"
                    );
                }
            },
            FileEvent::Unlink => {
                // A removed file that was never in the map is not an error.
                let _ = engine.remove_file(&path);
                applied = true;
            }
        }
    }

    let rebuild_started = Instant::now();
    if applied && !engine.rebuild_ir() {
        failed = true;
        tracing::warn!("scopemux supervisor: incremental IR rebuild failed");
    }
    if applied && !engine.rebuild_info_blocks() {
        failed = true;
        tracing::warn!("scopemux supervisor: incremental InfoBlock rebuild failed");
    }
    let rebuild_duration = rebuild_started.elapsed();

    let mut report = freshness
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prior_re_anchor = report.re_anchor;
    let revision = report.last_revision.unwrap_or(0) + 1;
    report.state = if failed {
        MapFreshness::Stale
    } else {
        MapFreshness::Fresh
    };
    report.last_revision = Some(revision);
    report.dirty_set_size = 0;
    report.watcher_lag_ms = Some(started.elapsed().as_millis() as u64);
    report.last_rebuild_duration_ms = Some(rebuild_duration.as_millis() as u64);
    report.re_anchor = prior_re_anchor;
}

fn handle_command(engine: &mut WorkspaceEngine, command: CommandMsg) {
    let result = (command.job)(engine);
    let _ = command.reply.send(Ok(result));
}

/// Serialize native tests: the C core keeps process-global parser/query state and
/// is not safe for concurrent use.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::native_test_lock;
    use std::fs;

    fn write(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn wait_until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("condition not met before timeout");
    }

    fn search(handle: &MapHandle, query: &str) -> Vec<SearchHitView> {
        handle
            .search(SearchRequest {
                query: query.to_string(),
                anchor_file: None,
                max_hits: 20,
            })
            .expect("search should succeed")
    }

    #[test]
    fn registry_reuses_one_map_per_root() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.rs", "pub fn one() {}\n");
        let root = dir.path().to_string_lossy().to_string();

        let registry = WorkspaceMapRegistry::new(Duration::from_millis(20), Duration::ZERO);
        let first = registry.acquire(&root).unwrap();
        let second = registry.acquire(&root).unwrap();
        assert!(first.is_same_map(&second));
        assert_eq!(registry.len(), 1);

        drop(first);
        drop(second);
        registry.sweep(Duration::ZERO);
        assert_eq!(registry.len(), 0, "idle map should be evicted");
    }

    #[test]
    fn edit_updates_nodes_incrementally_through_the_watcher() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.rs", "pub fn before() {}\n");
        write(dir.path(), "b.rs", "pub fn keeper() {}\n");
        let root = dir.path().to_string_lossy().to_string();

        let registry =
            WorkspaceMapRegistry::new(Duration::from_millis(20), Duration::from_secs(60));
        let handle = registry.acquire(&root).unwrap();
        assert!(search(&handle, "added_symbol").is_empty());

        write(
            dir.path(),
            "a.rs",
            "pub fn before() {}\npub fn added_symbol() {}\n",
        );

        wait_until(|| handle.freshness().last_revision >= Some(2));
        assert!(
            search(&handle, "added_symbol")
                .iter()
                .any(|hit| hit.block.name.contains("added_symbol")),
            "edit should be visible through the persistent map"
        );
        let counters = handle.counters().unwrap();
        assert!(
            counters.last_rebuild_incremental,
            "watcher-driven update should reuse retained entries"
        );
        assert!(handle.freshness().last_rebuild_duration_ms.is_some());
    }

    #[test]
    fn unlink_removes_nodes() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.rs", "pub fn removable_symbol() {}\n");
        write(dir.path(), "b.rs", "pub fn keeper() {}\n");
        let root = dir.path().to_string_lossy().to_string();

        let registry =
            WorkspaceMapRegistry::new(Duration::from_millis(20), Duration::from_secs(60));
        let handle = registry.acquire(&root).unwrap();
        assert!(!search(&handle, "removable_symbol").is_empty());

        fs::remove_file(dir.path().join("a.rs")).unwrap();
        wait_until(|| {
            handle
                .freshness()
                .last_revision
                .is_some_and(|revision| revision >= 2)
        });

        assert!(
            search(&handle, "removable_symbol").is_empty(),
            "deleted file nodes must not remain"
        );
    }

    #[test]
    fn ignored_directories_never_enter_the_map() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.rs", "pub fn kept() {}\n");
        let root = dir.path().to_string_lossy().to_string();

        let registry =
            WorkspaceMapRegistry::new(Duration::from_millis(20), Duration::from_secs(60));
        let handle = registry.acquire(&root).unwrap();
        let revision = handle.freshness().last_revision;

        write(
            dir.path(),
            "target/debug/ignored.rs",
            "pub fn ignored_symbol() {}\n",
        );
        thread::sleep(Duration::from_millis(250));

        assert!(search(&handle, "ignored_symbol").is_empty());
        assert_eq!(
            handle.freshness().last_revision,
            revision,
            "ignored paths must not trigger a revision"
        );
    }

    #[test]
    fn no_cross_workspace_leakage() {
        let _guard = native_test_lock();
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        write(first.path(), "a.rs", "pub fn only_in_first() {}\n");
        write(second.path(), "a.rs", "pub fn only_in_second() {}\n");

        let registry =
            WorkspaceMapRegistry::new(Duration::from_millis(20), Duration::from_secs(60));
        let first_handle = registry.acquire(&first.path().to_string_lossy()).unwrap();
        let second_handle = registry.acquire(&second.path().to_string_lossy()).unwrap();
        assert!(!first_handle.is_same_map(&second_handle));

        assert!(!search(&first_handle, "only_in_first").is_empty());
        assert!(search(&second_handle, "only_in_first").is_empty());
        assert!(!search(&second_handle, "only_in_second").is_empty());
        assert!(search(&first_handle, "only_in_second").is_empty());
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn code_search_tool_answers_from_the_persistent_map() {
        use opencode_tool::code_search_tool::WorkspaceCodeSearchTool;
        use opencode_tool::{Tool as _, ToolContext};

        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.rs", "pub fn locate_me() -> u32 { 1 }\n");
        write(dir.path(), "README.md", "prose_only_token lives here\n");
        let root = dir.path().to_string_lossy().to_string();

        let registry = Arc::new(WorkspaceMapRegistry::new(
            Duration::from_millis(20),
            Duration::from_secs(60),
        ));
        // Prime the map so the capability does not race the initial build.
        drop(registry.acquire(&root).unwrap());
        let handle = registry.acquire(&root).unwrap();

        let capability: Arc<dyn opencode_types::workspace::WorkspaceSearch> =
            Arc::new(RegistryWorkspaceSearch::new(registry.clone()));
        let ctx = ToolContext::new("session".into(), "message".into(), root.clone())
            .with_workspace_search(Some(capability));

        let result = WorkspaceCodeSearchTool
            .execute(serde_json::json!({"query": "locate_me"}), ctx.clone())
            .await
            .unwrap();
        assert_eq!(
            result.metadata["backend"],
            serde_json::json!("scopemux"),
            "{}",
            result.output
        );
        assert!(result.output.contains("a.rs"), "{}", result.output);
        assert!(
            result.output.contains("sym:"),
            "block id missing: {}",
            result.output
        );

        // The watcher updates the map after an edit; the tool sees the new symbol
        // without a per-call filesystem walk.
        write(
            dir.path(),
            "a.rs",
            "pub fn locate_me() -> u32 { 1 }\npub fn added_after_watch() -> u32 { 2 }\n",
        );
        wait_until(|| {
            handle
                .freshness()
                .last_revision
                .is_some_and(|revision| revision >= 2)
        });
        let result = WorkspaceCodeSearchTool
            .execute(
                serde_json::json!({"query": "added_after_watch"}),
                ctx.clone(),
            )
            .await
            .unwrap();
        assert_eq!(result.metadata["backend"], serde_json::json!("scopemux"));
        assert!(
            result.output.contains("added_after_watch"),
            "updated symbol missing: {}",
            result.output
        );

        // Prose mode always uses grep, never the code map.
        let result = WorkspaceCodeSearchTool
            .execute(
                serde_json::json!({"query": "prose_only_token", "mode": "prose"}),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(result.metadata["backend"], serde_json::json!("grep"));
        assert_eq!(
            result.metadata["fallbackReason"],
            serde_json::json!("prose_mode")
        );
    }

    #[test]
    fn freshness_reports_and_reanchor_are_recorded() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.rs", "pub fn fresh() {}\n");

        let registry = WorkspaceMapRegistry::default();
        let handle = registry.acquire(&dir.path().to_string_lossy()).unwrap();
        let report = handle.freshness();
        assert_eq!(report.state, MapFreshness::Fresh);
        assert_eq!(report.last_revision, Some(1));
        assert_eq!(report.dirty_set_size, 0);

        handle.record_re_anchor(ReAnchorResult::Shifted);
        assert_eq!(handle.freshness().re_anchor, Some(ReAnchorResult::Shifted));
    }

    #[test]
    fn missing_root_is_reported_as_unavailable() {
        let _guard = native_test_lock();
        let registry = WorkspaceMapRegistry::default();
        let error = registry
            .acquire("/definitely/not/a/workspace")
            .expect_err("missing root must fail");
        assert!(matches!(error, SupervisorError::Unavailable(_)));
    }
}
