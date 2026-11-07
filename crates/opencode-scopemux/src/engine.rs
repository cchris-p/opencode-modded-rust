//! Safe `scopemux-core` engine layer (`SCOPE-016`).
//!
//! [`WorkspaceEngine`] owns exactly one `ProjectContext` for its whole lifetime
//! and exposes open/add/update/remove, incremental rebuild, counters, and the
//! query slices. No raw FFI pointer escapes this module.
//!
//! Thread affinity: one engine is owned by one worker/task. The engine holds a
//! raw pointer, so it is deliberately not `Send`/`Sync`; callers must not hold it
//! across an `await` or move it to another worker. This module contains no model
//! activation, prompt, task-state, or fallback policy.

use std::ffi::CString;
use std::fmt;
use std::os::raw::c_char;

use crate::ffi;

/// `LANG_UNKNOWN`: let the core auto-detect the language from the extension.
pub const LANG_UNKNOWN: i32 = 0;

/// Error from the safe engine layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError(pub String);

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

/// Owned copy of one canonical InfoBlock.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockView {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub file_path: String,
    pub kind: i32,
    pub estimated_tokens: usize,
    pub origin: i32,
    pub lifecycle: i32,
    pub provenance: String,
    pub confidence: f32,
}

impl BlockView {
    fn from_ptr(block: *const ffi::ProjectInfoBlock) -> Option<Self> {
        if block.is_null() {
            return None;
        }
        let block = unsafe { &*block };
        Some(Self {
            id: cstr(block.id),
            name: cstr(block.name),
            qualified_name: cstr(block.qualified_name),
            file_path: cstr(block.file_path),
            kind: block.kind,
            estimated_tokens: block.estimated_tokens,
            origin: block.origin,
            lifecycle: block.lifecycle,
            provenance: cstr(block.provenance),
            confidence: block.confidence,
        })
    }
}

/// One search hit with its owned block.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHitView {
    pub block: BlockView,
    pub score: usize,
    pub name_match: bool,
    pub text_match: bool,
    pub relationship_match: bool,
}

/// Search request for [`WorkspaceEngine::search`].
#[derive(Debug, Clone, Copy)]
pub struct SearchRequest<'a> {
    pub query: &'a str,
    pub anchor_file: Option<&'a str>,
    pub max_hits: usize,
}

/// One delta entry with its owned block.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaEntryView {
    pub kind: i32,
    pub block: Option<BlockView>,
    pub provenance: String,
    pub confidence: f32,
}

/// One map-query item (resolve/duplicates/observability/impact).
#[derive(Debug, Clone, PartialEq)]
pub struct MapItemView {
    pub kind: i32,
    pub block: Option<BlockView>,
    pub reason: String,
    pub provenance: String,
    pub confidence: f32,
}

/// One plan-node reconciliation entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileView {
    pub plan_node_id: String,
    pub previous_lifecycle: i32,
    pub new_lifecycle: i32,
}

/// Incremental rebuild counters reported by the core (`WI-018`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCounters {
    /// Files recomputed by the last IR rebuild (dirty set for an incremental run).
    pub last_recomputed_file_count: usize,
    /// Files present during the last IR rebuild.
    pub last_rebuild_file_count: usize,
    /// Whether the last IR rebuild reused retained clean-file entries.
    pub last_rebuild_incremental: bool,
    /// Files whose InfoBlocks were reused by the last registry rebuild.
    pub info_block_reused_file_count: usize,
    /// Files whose InfoBlocks were re-derived by the last registry rebuild.
    pub info_block_recomputed_file_count: usize,
}

/// Input for materializing one projected plan node into the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNodeInput {
    pub slug: String,
    pub kind: i32,
    pub title: String,
    pub desired_shape: Option<String>,
    pub rationale: Option<String>,
    pub projected_symbol: Option<String>,
    pub file_path: Option<String>,
    pub provenance: Option<String>,
    pub lifecycle: i32,
    pub anchors: Vec<String>,
}

/// Safe owner of one `ProjectContext`.
pub struct WorkspaceEngine {
    root: std::path::PathBuf,
    ctx: *mut ffi::ProjectContext,
}

impl WorkspaceEngine {
    /// Open a project context for `root` and point the core at its Tree-sitter
    /// queries.
    pub fn open(root: &str) -> Result<Self, EngineError> {
        // Point scopemux-core at the Tree-sitter queries shipped with the pinned
        // source (recorded at build time) unless already configured.
        if std::env::var_os("SCMU_QUERIES_DIR").is_none() {
            if let Some(queries_dir) = option_env!("SCOPEMUX_QUERIES_DIR") {
                std::env::set_var("SCMU_QUERIES_DIR", queries_dir);
            }
        }

        let root_str = root.to_string();
        let root = CString::new(root_str.as_str())
            .map_err(|e| EngineError(format!("invalid workspace root: {e}")))?;
        let ctx = unsafe { ffi::project_context_create(root.as_ptr()) };
        if ctx.is_null() {
            return Err(EngineError(
                "project_context_create returned null".to_string(),
            ));
        }
        Ok(Self {
            root: std::path::PathBuf::from(root_str),
            ctx,
        })
    }

    /// Register a file (does not parse until [`Self::parse_all`]).
    pub fn add_file(&mut self, path: &str) -> bool {
        let Ok(path) = CString::new(path) else {
            return false;
        };
        unsafe { ffi::project_add_file(self.ctx, path.as_ptr(), LANG_UNKNOWN) }
    }

    /// Parse every registered file.
    pub fn parse_all(&mut self) -> bool {
        unsafe { ffi::project_parse_all_files(self.ctx) }
    }

    /// Resolve references across the registered files.
    pub fn resolve_references(&mut self) -> bool {
        unsafe { ffi::project_resolve_references(self.ctx) }
    }

    /// Incrementally (re)parse or update one file from disk.
    ///
    /// Returns whether the file actually changed. Uses
    /// `project_update_file`/`project_update_file_from_string` under the hood, so
    /// an unchanged file is a no-op.
    pub fn update_file(&mut self, path: &str) -> Result<bool, EngineError> {
        // `project_update_file` opens the path directly on the process CWD, so
        // resolve a project-relative path against the engine root first.
        let resolved = self.resolve_path(path);
        let path =
            CString::new(resolved).map_err(|e| EngineError(format!("invalid file path: {e}")))?;
        let mut changed = false;
        let ok = unsafe {
            ffi::project_update_file(self.ctx, path.as_ptr(), LANG_UNKNOWN, &mut changed)
        };
        if ok {
            Ok(changed)
        } else {
            Err(EngineError("project_update_file failed".to_string()))
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        let path = path.strip_prefix("file://").unwrap_or(path);
        if std::path::Path::new(path).is_absolute() {
            path.to_string()
        } else {
            self.root.join(path).to_string_lossy().into_owned()
        }
    }

    /// Remove a file from the map.
    pub fn remove_file(&mut self, path: &str) -> Result<bool, EngineError> {
        let path =
            CString::new(path).map_err(|e| EngineError(format!("invalid file path: {e}")))?;
        let ok = unsafe { ffi::project_context_remove_file(self.ctx, path.as_ptr()) };
        if ok {
            Ok(true)
        } else {
            Err(EngineError(
                "project_context_remove_file failed".to_string(),
            ))
        }
    }

    /// Rebuild the project IR snapshot (full or incremental, core decides).
    pub fn rebuild_ir(&mut self) -> bool {
        unsafe { ffi::project_context_rebuild_ir(self.ctx) }
    }

    /// Rebuild the canonical InfoBlock registry from current project IR.
    pub fn rebuild_info_blocks(&mut self) -> bool {
        unsafe { ffi::project_context_rebuild_info_blocks(self.ctx) }
    }

    /// Incremental rebuild counters from the last IR/InfoBlock rebuild.
    pub fn counters(&self) -> EngineCounters {
        unsafe {
            EngineCounters {
                last_recomputed_file_count: ffi::project_context_last_recomputed_file_count(
                    self.ctx,
                ),
                last_rebuild_file_count: ffi::project_context_last_rebuild_file_count(self.ctx),
                last_rebuild_incremental: ffi::project_context_last_rebuild_was_incremental(
                    self.ctx,
                ),
                info_block_reused_file_count:
                    ffi::project_context_last_info_block_reused_file_count(self.ctx),
                info_block_recomputed_file_count:
                    ffi::project_context_last_info_block_recomputed_file_count(self.ctx),
            }
        }
    }

    /// Tiered search over the canonical InfoBlock registry.
    pub fn search(&self, request: SearchRequest<'_>) -> Result<Vec<SearchHitView>, EngineError> {
        let query =
            CString::new(request.query).map_err(|e| EngineError(format!("invalid query: {e}")))?;
        let anchor = request.anchor_file.and_then(|path| CString::new(path).ok());
        let search_request = ffi::ProjectSearchRequest {
            query_text: query.as_ptr(),
            anchor_symbol: std::ptr::null(),
            anchor_file_path: anchor.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            min_tier: 0,
            max_tier: 4,
            include_related: false,
            include_dependencies: true,
            max_hits: request.max_hits,
            origin_mask: 0,
            lifecycle_mask: 0,
        };
        let mut result = ffi::ProjectSearchResult {
            hits: std::ptr::null_mut(),
            hit_count: 0,
            total_match_count: 0,
        };
        let ok = unsafe {
            ffi::project_context_search_info_blocks(
                self.ctx,
                &search_request,
                &mut result as *mut _,
            )
        };
        if !ok {
            return Err(EngineError("project search failed".to_string()));
        }
        let _guard = SearchResultGuard(&mut result as *mut _);

        let mut hits = Vec::new();
        for i in 0..result.hit_count {
            let hit = unsafe { &*result.hits.add(i) };
            if let Some(block) = BlockView::from_ptr(hit.block) {
                hits.push(SearchHitView {
                    block,
                    score: hit.score,
                    name_match: hit.name_match,
                    text_match: hit.text_match,
                    relationship_match: hit.relationship_match,
                });
            }
        }
        Ok(hits)
    }

    /// Delta between current (parsed) and target (planned) state.
    pub fn delta(&self, task_id: Option<&str>, stage: &str) -> Vec<DeltaEntryView> {
        let task_id = task_id.and_then(|id| CString::new(id).ok());
        let stage = CString::new(stage).ok();
        let mut result = ffi::ProjectDeltaResult {
            task_id: std::ptr::null(),
            stage: std::ptr::null(),
            entries: std::ptr::null_mut(),
            entry_count: 0,
            add_count: 0,
            change_count: 0,
            remove_count: 0,
            reuse_count: 0,
            estimated_tokens: 0,
        };
        let ok = unsafe {
            ffi::project_context_compute_delta(
                self.ctx,
                task_id.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                stage.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                &mut result as *mut _,
            )
        };
        if !ok {
            return Vec::new();
        }
        let _guard = DeltaResultGuard(&mut result as *mut _);

        let mut entries = Vec::new();
        for i in 0..result.entry_count {
            let entry = unsafe { &*result.entries.add(i) };
            entries.push(DeltaEntryView {
                kind: entry.kind,
                block: BlockView::from_ptr(entry.block),
                provenance: cstr(entry.provenance),
                confidence: entry.confidence,
            });
        }
        entries
    }

    /// Seed nodes and anchors resolved for a task and stage.
    pub fn resolve(&self, task_id: Option<&str>, stage: &str) -> Vec<MapItemView> {
        let task_id = task_id.and_then(|id| CString::new(id).ok());
        let stage = CString::new(stage).ok();
        let mut result = empty_map_result();
        let ok = unsafe {
            ffi::project_context_query_resolve(
                self.ctx,
                task_id.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                stage.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                &mut result as *mut _,
            )
        };
        if !ok {
            return Vec::new();
        }
        map_items(&mut result)
    }

    /// Duplicate/refactor-opportunity clusters.
    pub fn duplicates(&self, scope: Option<&str>) -> Vec<MapItemView> {
        let scope = scope.and_then(|value| CString::new(value).ok());
        let mut result = empty_map_result();
        let ok = unsafe {
            ffi::project_context_query_duplicates(
                self.ctx,
                scope.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
                &mut result as *mut _,
            )
        };
        if !ok {
            return Vec::new();
        }
        map_items(&mut result)
    }

    /// Observability blocks attached to one symbol.
    pub fn observability(&self, symbol: &str) -> Vec<MapItemView> {
        let Ok(symbol) = CString::new(symbol) else {
            return Vec::new();
        };
        let mut result = empty_map_result();
        let ok = unsafe {
            ffi::project_context_query_observability(
                self.ctx,
                symbol.as_ptr(),
                &mut result as *mut _,
            )
        };
        if !ok {
            return Vec::new();
        }
        map_items(&mut result)
    }

    /// Impact of changed files: their blocks plus anchored plans.
    pub fn change_impact(&self, files: &[String]) -> Vec<MapItemView> {
        let files: Vec<CString> = files
            .iter()
            .filter_map(|path| CString::new(path.as_str()).ok())
            .collect();
        let pointers: Vec<*const c_char> = files.iter().map(|c| c.as_ptr()).collect();
        let mut result = empty_map_result();
        let ok = unsafe {
            ffi::project_context_query_change_impact(
                self.ctx,
                pointers.as_ptr(),
                pointers.len(),
                &mut result as *mut _,
            )
        };
        if !ok {
            return Vec::new();
        }
        map_items(&mut result)
    }

    /// Materialize one projected plan node into the map. Best-effort: a malformed
    /// input is skipped rather than failing retrieval.
    pub fn materialize_plan_node(&mut self, task_id: &str, input: &PlanNodeInput) {
        let Ok(task_id) = CString::new(task_id) else {
            return;
        };
        let Ok(slug) = CString::new(input.slug.as_str()) else {
            return;
        };
        let node = unsafe {
            ffi::project_context_plan_node_create(
                self.ctx,
                task_id.as_ptr(),
                slug.as_ptr(),
                input.kind,
            )
        };
        if node.is_null() {
            return;
        }

        unsafe {
            if let Ok(title) = CString::new(input.title.as_str()) {
                ffi::project_context_plan_node_set_title(self.ctx, node, title.as_ptr());
            }
            if let Some(shape) = input
                .desired_shape
                .as_deref()
                .and_then(|shape| CString::new(shape).ok())
            {
                ffi::project_context_plan_node_set_desired_shape(self.ctx, node, shape.as_ptr());
            }
            if let Some(rationale) = input
                .rationale
                .as_deref()
                .and_then(|rationale| CString::new(rationale).ok())
            {
                ffi::project_context_plan_node_set_rationale(self.ctx, node, rationale.as_ptr());
            }
            if let Some(symbol) = input
                .projected_symbol
                .as_deref()
                .and_then(|symbol| CString::new(symbol).ok())
            {
                ffi::project_context_plan_node_set_projected_symbol(
                    self.ctx,
                    node,
                    symbol.as_ptr(),
                );
            }
            if let Some(path) = input
                .file_path
                .as_deref()
                .and_then(|path| CString::new(path).ok())
            {
                ffi::project_context_plan_node_set_file_path(self.ctx, node, path.as_ptr());
            }
            if let Some(provenance) = input
                .provenance
                .as_deref()
                .and_then(|provenance| CString::new(provenance).ok())
            {
                ffi::project_context_plan_node_set_provenance(self.ctx, node, provenance.as_ptr());
            }
            ffi::project_context_plan_node_set_lifecycle(self.ctx, node, input.lifecycle);
            for anchor in &input.anchors {
                if let Ok(anchor) = CString::new(anchor.as_str()) {
                    ffi::project_context_plan_node_add_anchor(self.ctx, node, anchor.as_ptr());
                }
            }
        }
    }

    /// Reconcile materialized plan nodes against parsed state.
    pub fn reconcile_plan_nodes(&self) -> Vec<ReconcileView> {
        let mut signals = Vec::new();
        let mut reconcile = ffi::ProjectPlanNodeReconciliationResult {
            entries: std::ptr::null_mut(),
            entry_count: 0,
            stale_count: 0,
            conflict_count: 0,
            implemented_count: 0,
        };
        let ok = unsafe {
            ffi::project_context_reconcile_plan_nodes(self.ctx, &mut reconcile as *mut _)
        };
        if !ok {
            return signals;
        }

        unsafe {
            for i in 0..reconcile.entry_count {
                let entry = &*reconcile.entries.add(i);
                if entry.node.is_null() {
                    continue;
                }
                signals.push(ReconcileView {
                    plan_node_id: cstr((*entry.node).id),
                    previous_lifecycle: entry.previous_lifecycle,
                    new_lifecycle: entry.new_lifecycle,
                });
            }
            ffi::project_plan_node_reconciliation_result_free(&mut reconcile as *mut _);
        }
        signals
    }
}

impl Drop for WorkspaceEngine {
    fn drop(&mut self) {
        unsafe { ffi::project_context_free(self.ctx) }
    }
}

fn empty_map_result() -> ffi::ProjectMapQueryResult {
    ffi::ProjectMapQueryResult {
        kind: 0,
        items: std::ptr::null_mut(),
        item_count: 0,
        estimated_tokens: 0,
    }
}

fn map_items(result: &mut ffi::ProjectMapQueryResult) -> Vec<MapItemView> {
    let mut items = Vec::new();
    for i in 0..result.item_count {
        let item = unsafe { &*result.items.add(i) };
        items.push(MapItemView {
            kind: item.kind,
            block: BlockView::from_ptr(item.block),
            reason: cstr(item.reason),
            provenance: cstr(item.provenance),
            confidence: item.confidence,
        });
    }
    unsafe { ffi::project_map_query_result_free(result as *mut _) };
    items
}

fn cstr(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }
}

struct SearchResultGuard(*mut ffi::ProjectSearchResult);

impl Drop for SearchResultGuard {
    fn drop(&mut self) {
        unsafe { ffi::project_search_result_free(self.0) }
    }
}

struct DeltaResultGuard(*mut ffi::ProjectDeltaResult);

impl Drop for DeltaResultGuard {
    fn drop(&mut self) {
        unsafe { ffi::project_delta_result_free(self.0) }
    }
}

/// Serialize native tests: the C core keeps process-global parser/query state and
/// is not safe for concurrent use.
#[cfg(test)]
pub(crate) fn native_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(dir: &std::path::Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn persistent_context_sees_an_edit_after_incremental_update() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.rs", "pub fn first() -> u32 { 1 }\n");
        write_file(dir.path(), "b.rs", "pub fn second() -> u32 { 2 }\n");
        write_file(dir.path(), "c.rs", "pub fn third() -> u32 { 3 }\n");

        let root = dir.path().to_string_lossy().to_string();
        let mut engine = WorkspaceEngine::open(&root).unwrap();
        for file in ["a.rs", "b.rs", "c.rs"] {
            assert!(engine.add_file(file), "add {file}");
        }
        assert!(engine.parse_all(), "parse all");
        assert!(engine.rebuild_ir(), "full rebuild");
        assert!(engine.rebuild_info_blocks(), "initial info blocks");

        let full = engine.counters();
        assert!(!full.last_rebuild_incremental, "first rebuild is full");
        assert_eq!(full.last_rebuild_file_count, 3);
        assert_eq!(full.last_recomputed_file_count, 3);

        let hits = engine
            .search(SearchRequest {
                query: "added_symbol",
                anchor_file: None,
                max_hits: 20,
            })
            .unwrap();
        assert!(
            !hits
                .iter()
                .any(|hit| hit.block.name.contains("added_symbol")),
            "symbol must not exist before the edit"
        );

        write_file(
            dir.path(),
            "a.rs",
            "pub fn first() -> u32 { 1 }\npub fn added_symbol() -> u32 { 7 }\n",
        );
        assert!(engine.update_file("a.rs").unwrap(), "file changed");
        assert!(engine.rebuild_ir(), "incremental rebuild");
        assert!(engine.rebuild_info_blocks(), "info blocks after edit");

        let incremental = engine.counters();
        assert!(
            incremental.last_rebuild_incremental,
            "a one-file edit must rebuild incrementally"
        );
        assert!(
            incremental.last_recomputed_file_count < full.last_rebuild_file_count,
            "dirty set {} must be smaller than the full rebuild {}",
            incremental.last_recomputed_file_count,
            full.last_rebuild_file_count
        );

        let hits = engine
            .search(SearchRequest {
                query: "added_symbol",
                anchor_file: None,
                max_hits: 20,
            })
            .unwrap();
        assert!(
            hits.iter()
                .any(|hit| hit.block.name.contains("added_symbol")),
            "the persistent context must see the new symbol: {hits:?}"
        );
    }

    #[test]
    fn remove_file_drops_nodes() {
        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "a.rs", "pub fn removable_symbol() {}\n");
        write_file(dir.path(), "b.rs", "pub fn keeper() {}\n");

        let root = dir.path().to_string_lossy().to_string();
        let mut engine = WorkspaceEngine::open(&root).unwrap();
        assert!(engine.add_file("a.rs"));
        assert!(engine.add_file("b.rs"));
        assert!(engine.parse_all());
        assert!(engine.rebuild_ir());
        assert!(engine.rebuild_info_blocks());

        assert!(engine.remove_file("a.rs").unwrap());
        assert!(engine.rebuild_ir());
        assert!(engine.rebuild_info_blocks());

        let hits = engine
            .search(SearchRequest {
                query: "removable_symbol",
                anchor_file: None,
                max_hits: 20,
            })
            .unwrap();
        assert!(
            !hits.iter().any(|hit| hit.block.file_path.contains("a.rs")),
            "removed file nodes must not remain: {hits:?}"
        );
    }
}
