//! `RetrievalProvider` adapter over the safe engine (`SCOPE-016`).
//!
//! The provider translates retrieval requests into engine queries and maps the
//! returned blocks into shared [`opencode_types::RetrievalCandidate`] evidence.
//! It owns no FFI pointer and makes no lifecycle, activation, prompt, or fallback
//! decisions.

use std::sync::Arc;

use opencode_retrieval::{RetrievalError, RetrievalProvider};
use opencode_types::RetrievalRequest;

use crate::engine::{
    candidate_from_block, confidence_from_score, search_hit_candidate, MapItemView, PlanNodeInput,
    SearchRequest,
};
use crate::supervisor::MapHandle;
use crate::{language_for_path, workspace_supported};

/// The native provider. See the module docs.
#[derive(Clone)]
pub struct ScopemuxProvider {
    exact_facts: Option<Arc<dyn opencode_retrieval::ExactFactProvider>>,
}

impl ScopemuxProvider {
    pub fn new() -> Self {
        Self { exact_facts: None }
    }

    /// Attach an optional exact-fact source (for example LSP-backed facts).
    pub fn with_exact_facts(
        exact_facts: Option<Arc<dyn opencode_retrieval::ExactFactProvider>>,
    ) -> Self {
        Self { exact_facts }
    }
}

impl Default for ScopemuxProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RetrievalProvider for ScopemuxProvider {
    fn name(&self) -> &str {
        "scopemux"
    }

    async fn retrieve(
        &self,
        request: &RetrievalRequest,
    ) -> Result<opencode_types::RetrievalResponse, RetrievalError> {
        use opencode_types::RetrievalRepresentation;

        if !workspace_supported(request) {
            return Err(RetrievalError::Unavailable(
                "workspace language is not supported by scopemux-core".to_string(),
            ));
        }

        // Mixed-language workspaces degrade per file: supported files are parsed
        // and unsupported ones are skipped with explicit provenance rather than
        // failing the whole request.
        let unsupported: Vec<String> = request
            .seed_files
            .iter()
            .chain(request.changed_files.iter())
            .map(|p| p.strip_prefix("file://").unwrap_or(p).to_string())
            .filter(|p| language_for_path(p).is_none())
            .collect();

        let seeds: Vec<String> = request
            .seed_files
            .iter()
            .chain(request.changed_files.iter())
            .map(|p| p.strip_prefix("file://").unwrap_or(p).to_string())
            .filter(|p| language_for_path(p).is_some())
            .collect();

        // SCOPE-019: use the persistent per-workspace map instead of creating a
        // context per retrieval. The handle is released before the enrichment
        // await, so the provider never blocks on the worker across an await.
        let map = crate::shared_registry()
            .acquire(&request.workspace_root)
            .map_err(|error| RetrievalError::Unavailable(error.to_string()))?;
        let freshness = map.freshness().state;

        // SCOPE-003: materialize the task's projected plan nodes into the map so
        // the delta/observability/duplicate slices can see target state.
        // SCOPE-004: select the representation the stage asked for.
        materialize_plan_nodes(&map, request);
        let candidates = match request.representation {
            RetrievalRepresentation::Search => search_candidates(&map, request, &seeds)?,
            RetrievalRepresentation::Delta => delta_candidates(&map, request)?,
            RetrievalRepresentation::Anchors => resolve_candidates(&map, request)?,
            RetrievalRepresentation::Observability => observability_candidates(&map, request)?,
            RetrievalRepresentation::Duplicates => duplicates_candidates(&map)?,
            RetrievalRepresentation::Impact => impact_candidates(&map, request)?,
            RetrievalRepresentation::Review => review_candidates(&map, request)?,
            RetrievalRepresentation::Reconcile => Vec::new(),
        };

        // Reconciliation evidence is always reported; the runtime decides
        // whether to act on it. It never advances task stage or completion.
        let plan_signals = reconcile_plan_nodes(&map);
        drop(map);

        // SCOPE-013: optional exact-fact enrichment at the anchors the map already
        // selected. Kept outside the engine block so no core pointer is held
        // across the await. Bounded, best-effort, and never required.
        let (candidates, mut provider_notes) =
            enrich_with_exact_facts(candidates, self.exact_facts.as_ref(), request).await;
        for path in &unsupported {
            provider_notes.push(format!("skipped unsupported language: {path}"));
        }
        let fallback_reason = candidates
            .is_empty()
            .then_some(opencode_types::RetrievalFallbackReason::NoMatches);

        Ok(opencode_types::RetrievalResponse {
            candidates,
            provider: self.name().to_string(),
            representation: request.representation,
            plan_signals,
            provider_notes,
            fallback_reason,
            freshness: Some(freshness),
        })
    }
}

/// Maximum exact-fact queries one retrieval request may issue. LSP is expensive at
/// scale; enrichment only follows anchors the map already selected.
const MAX_EXACT_FACT_ANCHORS: usize = 8;

/// Maximum search hits returned for one `Search` request.
///
/// `token_budget` is a prompt budget, not a hit count, so it must not cap the
/// number of candidates here.
const SEARCH_MAX_HITS: usize = 20;

/// Enrich map candidates with optional exact semantic facts at their anchors.
///
/// The map's heuristic candidates are the anchor set; LSP is asked only for a
/// definition at a bounded number of them. Exact results become `confidence = exact`
/// candidates with server provenance. No provider, an unsupported server, a timeout,
/// or a failure leaves the candidates unchanged and records a fallback note.
async fn enrich_with_exact_facts(
    mut candidates: Vec<opencode_types::RetrievalCandidate>,
    exact_facts: Option<&Arc<dyn opencode_retrieval::ExactFactProvider>>,
    request: &RetrievalRequest,
) -> (Vec<opencode_types::RetrievalCandidate>, Vec<String>) {
    use std::collections::HashSet;

    use opencode_retrieval::{ExactFactKind, ExactFactOutcome, ExactFactRequest};
    use opencode_types::{RetrievalCandidateKind, RetrievalConfidence, RetrievalOrigin};

    let Some(provider) = exact_facts else {
        return (candidates, Vec::new());
    };

    if !matches!(
        request.representation,
        opencode_types::RetrievalRepresentation::Search
            | opencode_types::RetrievalRepresentation::Anchors
            | opencode_types::RetrievalRepresentation::Impact
            | opencode_types::RetrievalRepresentation::Review
    ) {
        return (candidates, Vec::new());
    }

    let mut anchors: Vec<(String, String)> = Vec::new();
    let mut seen = HashSet::new();
    for candidate in &candidates {
        let Some(symbol) = &candidate.symbol else {
            continue;
        };
        let path = candidate
            .path
            .strip_prefix("file://")
            .unwrap_or(&candidate.path)
            .to_string();
        if language_for_path(&path).is_none() {
            continue;
        }
        if seen.insert((path.clone(), symbol.clone())) {
            anchors.push((path, symbol.clone()));
        }
        if anchors.len() >= MAX_EXACT_FACT_ANCHORS {
            break;
        }
    }

    let mut notes = Vec::new();
    let mut added = HashSet::new();
    for (path, symbol) in anchors {
        let Some((line, character)) = symbol_anchor_position(&path, &symbol) else {
            continue;
        };
        let outcome = provider
            .exact_facts(ExactFactRequest {
                path: path.clone(),
                line,
                character,
                kind: ExactFactKind::Definition,
            })
            .await;
        match outcome {
            ExactFactOutcome::Exact(fact) => {
                for location in fact.locations {
                    if !added.insert((location.file.clone(), location.start_line)) {
                        continue;
                    }
                    candidates.push(opencode_types::RetrievalCandidate {
                        kind: RetrievalCandidateKind::Symbol,
                        path: location.file,
                        symbol: None,
                        block_id: None,
                        snippet: None,
                        provenance: format!(
                            "scopemux: exact definition for {symbol} at {path}:{line} (server: {})",
                            fact.server_id
                        ),
                        confidence: RetrievalConfidence::Exact,
                        score: 1.0,
                        estimated_tokens: None,
                        origin: Some(RetrievalOrigin::Parsed),
                        lifecycle: None,
                    });
                }
            }
            ExactFactOutcome::Unavailable { reason } => {
                notes.push(format!(
                    "exact semantic facts unavailable ({reason:?}); heuristic edges retained"
                ));
                break;
            }
        }
    }

    (candidates, notes)
}

/// Locate a symbol's first occurrence in a file for use as an LSP anchor. This is a
/// best-effort anchor for an already-selected map candidate, not a resolver.
fn symbol_anchor_position(path: &str, symbol: &str) -> Option<(u32, u32)> {
    let content = std::fs::read_to_string(path).ok()?;
    let name = symbol.rsplit("::").next().unwrap_or(symbol);
    let name = name.rsplit('.').next().unwrap_or(name);
    if name.is_empty() {
        return None;
    }
    for (line_index, line) in content.lines().enumerate() {
        if let Some(byte_index) = line.find(name) {
            let character = line[..byte_index].chars().count() as u32;
            return Some((line_index as u32, character));
        }
    }
    None
}

/// Search over the canonical InfoBlock registry (`Search` representation).
fn search_candidates(
    map: &MapHandle,
    request: &RetrievalRequest,
    seeds: &[String],
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let anchor_file = seeds.first().cloned();
    let hits = map
        .search(SearchRequest {
            query: request.objective.clone(),
            anchor_file,
            max_hits: SEARCH_MAX_HITS,
        })
        .map_err(|error| RetrievalError::Failed(error.to_string()))?;
    Ok(hits.into_iter().filter_map(search_hit_candidate).collect())
}

/// Delta between current (parsed) and target (planned) state (`Delta`).
fn delta_candidates(
    map: &MapHandle,
    request: &RetrievalRequest,
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let entries = map
        .delta(
            request.task_id.clone(),
            stage_label(&request.stage).to_string(),
        )
        .map_err(|error| RetrievalError::Failed(error.to_string()))?;
    let mut candidates = Vec::new();
    for entry in entries {
        let label = match entry.kind {
            0 => "delta add",
            1 => "delta change",
            2 => "delta remove",
            3 => "delta reuse",
            _ => "delta",
        };
        let provenance = if entry.provenance.is_empty() {
            format!("scopemux: {label}")
        } else {
            format!("scopemux: {label}: {}", entry.provenance)
        };
        let confidence = confidence_from_score(entry.confidence);
        if let Some(block) = entry.block {
            if let Some(candidate) =
                candidate_from_block(block, provenance, confidence, entry.confidence, None)
            {
                candidates.push(candidate);
            }
        }
    }
    Ok(candidates)
}

/// Seed nodes and anchors resolved for a task and stage (`Anchors`).
fn resolve_candidates(
    map: &MapHandle,
    request: &RetrievalRequest,
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let items = map
        .resolve(
            request.task_id.clone(),
            stage_label(&request.stage).to_string(),
        )
        .map_err(|error| RetrievalError::Failed(error.to_string()))?;
    Ok(map_query_candidates(items))
}

/// Observability blocks attached to anchored symbols (`Observability`).
///
/// Symbols come from the request's explicit `seed_symbols` plus the projected
/// symbols of its plan nodes, so a review slice can resolve the symbol a task
/// intends to touch to its observability points and covering tests.
fn observability_candidates(
    map: &MapHandle,
    request: &RetrievalRequest,
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let mut symbols = request.seed_symbols.clone();
    for draft in &request.plan_nodes {
        if let Some(symbol) = &draft.projected_symbol {
            if !symbols.contains(symbol) {
                symbols.push(symbol.clone());
            }
        }
    }

    let mut candidates = Vec::new();
    for symbol in &symbols {
        let items = map
            .observability(symbol.clone())
            .map_err(|error| RetrievalError::Failed(error.to_string()))?;
        candidates.extend(map_query_candidates(items));
    }
    Ok(candidates)
}

/// Duplicate/refactor-opportunity clusters (`Duplicates`).
fn duplicates_candidates(
    map: &MapHandle,
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let items = map
        .duplicates(None)
        .map_err(|error| RetrievalError::Failed(error.to_string()))?;
    Ok(map_query_candidates(items))
}

/// Impact of changed files: their blocks plus anchored plans (`Impact`).
fn impact_candidates(
    map: &MapHandle,
    request: &RetrievalRequest,
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let files: Vec<String> = request
        .changed_files
        .iter()
        .map(|p| p.strip_prefix("file://").unwrap_or(p).to_string())
        .collect();
    let items = map
        .change_impact(files)
        .map_err(|error| RetrievalError::Failed(error.to_string()))?;
    Ok(map_query_candidates(items))
}

/// Composed review slice: delta, then duplicates and observability (`Review`).
fn review_candidates(
    map: &MapHandle,
    request: &RetrievalRequest,
) -> Result<Vec<opencode_types::RetrievalCandidate>, RetrievalError> {
    let mut candidates = delta_candidates(map, request)?;
    candidates.extend(duplicates_candidates(map)?);
    candidates.extend(observability_candidates(map, request)?);

    // Keep the first occurrence of each (path, symbol, provenance) triple so a
    // node reached by more than one slice is not duplicated.
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|candidate| {
        seen.insert((
            candidate.path.clone(),
            candidate.symbol.clone(),
            candidate.provenance.clone(),
        ))
    });
    Ok(candidates)
}

/// Convert map query items into candidates.
fn map_query_candidates(items: Vec<MapItemView>) -> Vec<opencode_types::RetrievalCandidate> {
    use opencode_types::RetrievalCandidateKind;

    let mut candidates = Vec::new();
    for item in items {
        let provenance = if item.provenance.is_empty() {
            format!("scopemux: {}", item.reason)
        } else {
            format!("scopemux: {}: {}", item.reason, item.provenance)
        };
        // `4` is `PROJECT_MAP_QUERY_DUPLICATES`; `5` is
        // `PROJECT_MAP_QUERY_OBSERVABILITY` in `ProjectMapQueryKind`.
        let kind_override = match item.kind {
            4 => Some(RetrievalCandidateKind::RefactorOpportunity),
            5 => Some(RetrievalCandidateKind::Observability),
            _ => None,
        };
        let confidence = confidence_from_score(item.confidence);
        if let Some(block) = item.block {
            if let Some(candidate) = candidate_from_block(
                block,
                provenance,
                confidence,
                item.confidence,
                kind_override,
            ) {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

/// Runtime stage label recorded on map requests (not interpreted by the core).
fn stage_label(stage: &opencode_types::TaskStage) -> &'static str {
    use opencode_types::TaskStage;
    match stage {
        TaskStage::Selected => "selected",
        TaskStage::ContextPrepared => "context_prepared",
        TaskStage::Implementing => "implementing",
        TaskStage::Verifying => "verifying",
        TaskStage::Reviewing => "reviewing",
        TaskStage::Repairing => "repairing",
        TaskStage::Completed => "completed",
    }
}

/// Materialize projected plan nodes into the persistent map. Best-effort: a
/// malformed draft is skipped rather than failing retrieval.
fn materialize_plan_nodes(map: &MapHandle, request: &RetrievalRequest) {
    use opencode_types::stage_lifecycle;

    if request.plan_nodes.is_empty() {
        return;
    }

    let task_id = request
        .task_id
        .clone()
        .unwrap_or_else(|| "task".to_string());
    let provenance = format!("task:{task_id}:{:?}", request.stage).to_lowercase();
    let lifecycle = stage_lifecycle(&request.stage).as_ffi();

    for draft in &request.plan_nodes {
        let input = PlanNodeInput {
            slug: draft.slug.clone(),
            kind: draft.kind.as_ffi(),
            title: draft.title.clone(),
            desired_shape: draft.desired_shape.clone(),
            rationale: draft.rationale.clone(),
            projected_symbol: draft.projected_symbol.clone(),
            file_path: draft.file_path.clone(),
            provenance: Some(provenance.clone()),
            lifecycle,
            anchors: draft.anchors.clone(),
        };
        let _ = map.materialize_plan_node(task_id.clone(), input);
    }
}

/// Reconcile materialized plan nodes against parsed state and report evidence.
/// The runtime decides whether to act; this never advances stage or completion.
fn reconcile_plan_nodes(map: &MapHandle) -> Vec<opencode_types::PlanReconciliationSignal> {
    use opencode_types::{PlanLifecycle, PlanReconciliationSignal, ReAnchorResult};

    let signals: Vec<PlanReconciliationSignal> = map
        .reconcile_plan_nodes()
        .unwrap_or_default()
        .into_iter()
        .map(|entry| PlanReconciliationSignal {
            plan_node_id: entry.plan_node_id,
            previous: PlanLifecycle::from_ffi(entry.previous_lifecycle),
            current: PlanLifecycle::from_ffi(entry.new_lifecycle),
        })
        .collect();

    // Record the measurement only; the runtime decides whether to re-plan.
    if !signals.is_empty() {
        let result = if signals
            .iter()
            .any(|signal| signal.current == PlanLifecycle::Conflict)
        {
            ReAnchorResult::Shifted
        } else if signals
            .iter()
            .any(|signal| signal.current == PlanLifecycle::Stale)
        {
            ReAnchorResult::Missing
        } else {
            ReAnchorResult::Anchored
        };
        map.record_re_anchor(result);
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::native_test_lock;
    use crate::{
        default_provider, language_for_path, model_scope_enabled, provider_for, workspace_supported,
    };
    use opencode_types::{MapRole, RetrievalRepresentation, RetrievalRequest, TaskStage};

    fn request(root: &str, seeds: Vec<&str>) -> RetrievalRequest {
        RetrievalRequest {
            objective: "find the entry point".to_string(),
            stage: TaskStage::Implementing,
            workspace_root: root.to_string(),
            seed_files: seeds.into_iter().map(str::to_string).collect(),
            seed_symbols: Vec::new(),
            changed_files: Vec::new(),
            role: MapRole::Project,
            representation: RetrievalRepresentation::Search,
            token_budget: None,
            task_id: None,
            reopen_reason: None,
            plan_nodes: Vec::new(),
        }
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_provider_handles_absolute_file_seed() {
        // Regression: real prompts pass absolute `file://` seeds with an empty
        // objective. The core's search-index text builder must grow for
        // absolute paths and node content instead of failing.
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sample.rs");
        std::fs::write(&file, "pub fn helper(v: u32) -> u32 {\n    v + 1\n}\n").unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec![]);
        req.seed_files = vec![format!("file://{}", file.display())];
        req.objective = String::new();
        req.changed_files = Vec::new();

        let response = ScopemuxProvider::new()
            .retrieve(&req)
            .await
            .expect("scopemux retrieve should succeed for an absolute file seed");
        assert_eq!(response.provider, "scopemux");
        assert!(
            !response.candidates.is_empty(),
            "expected at least one candidate for an absolute file seed"
        );
    }

    #[test]
    fn language_mapping_covers_supported_languages() {
        assert_eq!(language_for_path("a.c"), Some(1));
        assert_eq!(language_for_path("a.cpp"), Some(2));
        assert_eq!(language_for_path("a.py"), Some(3));
        assert_eq!(language_for_path("a.js"), Some(4));
        assert_eq!(language_for_path("a.ts"), Some(5));
        assert_eq!(language_for_path("a.rs"), Some(6));
        assert_eq!(language_for_path("a.txt"), None);
    }

    #[test]
    fn workspace_supported_requires_a_supported_seed() {
        assert!(!workspace_supported(&request("/ws", vec!["notes.txt"])));
        assert!(workspace_supported(&request("/ws", vec!["lib.rs"])));
        assert!(workspace_supported(&request(
            "/ws",
            vec!["lib.rs", "main.c"]
        )));
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_provider_returns_candidates_for_c_file() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.objective = "add".to_string();
        let response = ScopemuxProvider::new()
            .retrieve(&req)
            .await
            .expect("scopemux retrieve should succeed for a C workspace");

        assert_eq!(response.provider, "scopemux");
        assert!(
            !response.candidates.is_empty(),
            "expected at least one scopemux candidate"
        );
        assert!(
            response
                .candidates
                .iter()
                .all(|candidate| candidate.origin == Some(opencode_types::RetrievalOrigin::Parsed)),
            "parsed facts should be labeled with parsed origin"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_provider_returns_candidates_for_rust_file() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub struct Worker {\n    pub count: u32,\n}\n\npub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["lib.rs"]);
        req.objective = "add".to_string();
        let response = ScopemuxProvider::new()
            .retrieve(&req)
            .await
            .expect("scopemux retrieve should succeed for a Rust workspace");

        assert_eq!(response.provider, "scopemux");
        assert!(
            !response.candidates.is_empty(),
            "expected at least one scopemux candidate for a Rust file"
        );
        assert!(
            response
                .candidates
                .iter()
                .all(|candidate| candidate.origin == Some(opencode_types::RetrievalOrigin::Parsed)),
            "parsed facts should be labeled with parsed origin"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_provider_reports_plan_reconciliation_evidence() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{PlanLifecycle, PlanNodeDraft, PlanNodeKind};

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.objective = "add".to_string();
        req.task_id = Some("task_plan".to_string());
        req.plan_nodes = vec![
            PlanNodeDraft {
                slug: "implemented".to_string(),
                kind: PlanNodeKind::ModifySymbol,
                title: "implement add".to_string(),
                desired_shape: None,
                rationale: None,
                projected_symbol: Some("add".to_string()),
                file_path: None,
                anchors: Vec::new(),
            },
            PlanNodeDraft {
                slug: "stale".to_string(),
                kind: PlanNodeKind::Remove,
                title: "remove legacy".to_string(),
                desired_shape: None,
                rationale: None,
                projected_symbol: None,
                file_path: None,
                anchors: vec!["sym:does_not_exist".to_string()],
            },
        ];

        let response = ScopemuxProvider::new()
            .retrieve(&req)
            .await
            .expect("scopemux retrieve should succeed");

        let implemented = response
            .plan_signals
            .iter()
            .find(|signal| signal.plan_node_id == "plan:task_plan:implemented")
            .expect("projected symbol present in parsed state should emit a signal");
        assert_eq!(implemented.current, PlanLifecycle::Implemented);
        assert_eq!(implemented.previous, PlanLifecycle::InProgress);

        let stale = response
            .plan_signals
            .iter()
            .find(|signal| signal.plan_node_id == "plan:task_plan:stale")
            .expect("vanished anchor should emit a divergence signal");
        assert!(stale.current.is_divergence());
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_plan_reconciliation_records_reanchor_measurement() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{PlanNodeDraft, PlanNodeKind};

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.objective = "add".to_string();
        req.task_id = Some("task_reanchor".to_string());
        req.plan_nodes = vec![PlanNodeDraft {
            slug: "vanished".to_string(),
            kind: PlanNodeKind::Remove,
            title: "remove legacy".to_string(),
            desired_shape: None,
            rationale: None,
            projected_symbol: None,
            file_path: None,
            anchors: vec!["sym:does_not_exist".to_string()],
        }];

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();
        assert!(
            response
                .plan_signals
                .iter()
                .any(|signal| signal.current.is_divergence()),
            "vanished anchor should emit a divergence signal"
        );

        let map = crate::shared_registry()
            .acquire(&req.workspace_root)
            .unwrap();
        assert!(
            map.freshness().re_anchor.is_some(),
            "reconciliation should record a re-anchor measurement"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_delta_slice_returns_projected_entries() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{
            PlanNodeDraft, PlanNodeKind, RetrievalOrigin, RetrievalRepresentation,
        };

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.stage = TaskStage::ContextPrepared;
        req.role = MapRole::Slice;
        req.representation = RetrievalRepresentation::Delta;
        req.task_id = Some("task_delta".to_string());
        req.plan_nodes = vec![PlanNodeDraft {
            slug: "new-helper".to_string(),
            kind: PlanNodeKind::NewSymbol,
            title: "add a helper".to_string(),
            desired_shape: Some("fn helper()".to_string()),
            rationale: Some("objective".to_string()),
            projected_symbol: Some("helper".to_string()),
            file_path: None,
            anchors: Vec::new(),
        }];

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert_eq!(response.representation, RetrievalRepresentation::Delta);
        assert!(
            !response.candidates.is_empty(),
            "delta slice should report the projected change"
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.provenance.contains("delta")),
            "delta entries should carry delta provenance: {:?}",
            response.candidates
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.origin == Some(RetrievalOrigin::Planned)),
            "the projected node should be labeled planned"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_repair_slice_reanchors_reopen_reason() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{
            MapRole, PlanNodeDraft, PlanNodeKind, RetrievalOrigin, RetrievalRepresentation,
        };

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let add_path = dir.path().join("add.c").to_string_lossy().to_string();
        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.stage = TaskStage::Repairing;
        req.role = MapRole::Project;
        req.representation = RetrievalRepresentation::Impact;
        req.changed_files = vec![add_path.clone()];
        req.reopen_reason = Some("verification failed for `add`".to_string());
        req.task_id = Some("task_repair".to_string());
        req.plan_nodes = vec![
            PlanNodeDraft {
                slug: "objective".to_string(),
                kind: PlanNodeKind::ModifySymbol,
                title: "add".to_string(),
                desired_shape: None,
                rationale: Some("objective: add".to_string()),
                projected_symbol: Some("add".to_string()),
                file_path: Some(add_path.clone()),
                anchors: vec![format!("file:{add_path}")],
            },
            PlanNodeDraft {
                slug: "reopen-verification-failed-for-add".to_string(),
                kind: PlanNodeKind::RefactorOpportunity,
                title: "verification failed for `add`".to_string(),
                desired_shape: None,
                rationale: Some("reopen: verification failed for `add`".to_string()),
                projected_symbol: None,
                file_path: Some(add_path.clone()),
                anchors: vec![format!("file:{add_path}")],
            },
        ];

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert_eq!(response.representation, RetrievalRepresentation::Impact);
        assert!(
            !response.candidates.is_empty(),
            "repair slice should re-anchor the reopen reason"
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.path.ends_with("add.c")),
            "repair slice should include the affected anchor: {:?}",
            response.candidates
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.provenance.contains("changed file")),
            "repair slice should carry changed-file impact provenance: {:?}",
            response.candidates
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.origin == Some(RetrievalOrigin::Planned)),
            "prior plan nodes anchored to the changed file should surface: {:?}",
            response.candidates
        );
        assert!(
            response
                .plan_signals
                .iter()
                .any(|signal| signal.plan_node_id.ends_with(":objective")),
            "the realized objective should still reconcile: {:?}",
            response.plan_signals
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_mixed_language_workspace_degrades_per_file() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.md"), "# notes\n").unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c", "notes.md"]);
        req.objective = "add".to_string();

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert!(
            !response.candidates.is_empty(),
            "supported files should still produce map candidates"
        );
        assert!(
            response
                .candidates
                .iter()
                .all(|candidate| !candidate.path.ends_with("notes.md")),
            "prose must not be parsed as code: {:?}",
            response.candidates
        );
        assert!(
            response
                .provider_notes
                .iter()
                .any(|note| note.contains("skipped unsupported language: notes.md")),
            "unsupported files should carry explicit provenance: {:?}",
            response.provider_notes
        );
        assert_eq!(response.fallback_reason, None);
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_supported_workspace_with_no_matches_reports_no_matches() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.representation = RetrievalRepresentation::Reconcile;

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert!(response.candidates.is_empty());
        assert_eq!(
            response.fallback_reason,
            Some(opencode_types::RetrievalFallbackReason::NoMatches)
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_search_uses_the_persistent_whole_workspace_registry() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.c"), "int alpha(void) { return 1; }\n").unwrap();
        std::fs::write(dir.path().join("b.c"), "int beta(void) { return 2; }\n").unwrap();

        // The provider now queries the persistent whole-workspace map, so a
        // file on disk is visible even when it is not an explicit seed.
        let mut req = request(&dir.path().to_string_lossy(), vec!["a.c"]);
        req.objective = "beta".to_string();
        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.path.ends_with("b.c")),
            "the persistent map must include unseeded workspace files: {:?}",
            response.candidates
        );

        // Repeated retrieval reuses the same map entry (no per-call context).
        let root = dir.path().to_string_lossy().to_string();
        let first = crate::shared_registry().acquire(&root).unwrap();
        let second = crate::shared_registry().acquire(&root).unwrap();
        assert!(
            first.is_same_map(&second),
            "provider retrieval must reuse one persistent map per workspace"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_duplicates_slice_flags_matching_symbols() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{RetrievalCandidateKind, RetrievalRepresentation};

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.c"),
            "int dup(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("b.c"),
            "int dup(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["a.c", "b.c"]);
        req.representation = RetrievalRepresentation::Duplicates;

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert_eq!(response.representation, RetrievalRepresentation::Duplicates);
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.kind == RetrievalCandidateKind::RefactorOpportunity),
            "matching symbols should be flagged as a refactor opportunity"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_observability_slice_resolves_symbol_points() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{PlanNodeDraft, PlanNodeKind, RetrievalCandidateKind};

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("add.c"),
            "int add(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["add.c"]);
        req.representation = RetrievalRepresentation::Observability;
        req.seed_symbols = vec!["add".to_string()];
        req.task_id = Some("task_obs".to_string());
        req.plan_nodes = vec![PlanNodeDraft {
            slug: "observe-add".to_string(),
            kind: PlanNodeKind::ObservabilityPoint,
            title: "log add inputs".to_string(),
            desired_shape: None,
            rationale: Some("debugging".to_string()),
            projected_symbol: Some("add".to_string()),
            file_path: None,
            anchors: Vec::new(),
        }];

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert_eq!(
            response.representation,
            RetrievalRepresentation::Observability
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.kind == RetrievalCandidateKind::Observability),
            "observability plan node anchored to the symbol should resolve: {:?}",
            response.candidates
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_review_slice_composes_delta_and_duplicates() {
        use opencode_retrieval::RetrievalProvider;
        use opencode_types::{PlanNodeDraft, PlanNodeKind, RetrievalCandidateKind};

        let _guard = native_test_lock();

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.c"),
            "int dup(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("b.c"),
            "int dup(int a, int b) {\n  return a + b;\n}\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["a.c", "b.c"]);
        req.stage = TaskStage::Reviewing;
        req.role = MapRole::Project;
        req.representation = RetrievalRepresentation::Review;
        req.task_id = Some("task_review".to_string());
        req.plan_nodes = vec![PlanNodeDraft {
            slug: "modify-dup".to_string(),
            kind: PlanNodeKind::ModifySymbol,
            title: "change dup".to_string(),
            desired_shape: None,
            rationale: Some("criterion".to_string()),
            projected_symbol: Some("dup".to_string()),
            file_path: None,
            anchors: Vec::new(),
        }];

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert_eq!(response.representation, RetrievalRepresentation::Review);
        assert!(
            !response.candidates.is_empty(),
            "review slice should compose delta and duplicate signals"
        );
        assert!(
            response
                .candidates
                .iter()
                .any(|candidate| candidate.kind == RetrievalCandidateKind::RefactorOpportunity),
            "review slice should include duplicate/refactor flags: {:?}",
            response.candidates
        );
    }

    #[test]
    fn model_scope_is_local_qwen_only() {
        assert!(model_scope_enabled(Some("ollama"), Some("qwen3:30b")));
        assert!(model_scope_enabled(Some("OLLAMA"), Some("Qwen2.5-Coder")));
        assert!(!model_scope_enabled(Some("ollama"), Some("llama3:8b")));
        assert!(!model_scope_enabled(
            Some("deepseek"),
            Some("deepseek-flash")
        ));
        assert!(!model_scope_enabled(Some("ollama"), None));
        assert!(!model_scope_enabled(None, Some("qwen3:30b")));
        assert!(!model_scope_enabled(None, None));
    }

    #[test]
    fn provider_for_keeps_generic_outside_the_scoped_model() {
        // Off-scope models and the absence of a model fall back to the generic
        // provider regardless of the native feature.
        assert_eq!(
            provider_for(Some("deepseek"), Some("deepseek-flash")).name(),
            "generic"
        );
        assert_eq!(
            provider_for(Some("ollama"), Some("llama3:8b")).name(),
            "generic"
        );
        assert_eq!(provider_for(None, None).name(), "generic");
    }

    #[cfg(feature = "native")]
    #[test]
    fn provider_for_activates_scopemux_for_local_qwen() {
        assert_eq!(
            provider_for(Some("ollama"), Some("qwen3:30b")).name(),
            "scopemux"
        );
    }

    #[test]
    fn default_provider_matches_feature() {
        let provider = default_provider();
        #[cfg(not(feature = "native"))]
        assert_eq!(provider.name(), "generic");
        #[cfg(feature = "native")]
        assert_eq!(provider.name(), "scopemux");
    }

    /// Stub exact-fact source for enrichment tests.
    #[cfg(feature = "native")]
    #[derive(Clone)]
    struct StubExactFacts {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        outcome: opencode_retrieval::ExactFactOutcome,
    }

    #[cfg(feature = "native")]
    #[async_trait::async_trait]
    impl opencode_retrieval::ExactFactProvider for StubExactFacts {
        async fn exact_facts(
            &self,
            _request: opencode_retrieval::ExactFactRequest,
        ) -> opencode_retrieval::ExactFactOutcome {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.outcome.clone()
        }
    }

    #[cfg(feature = "native")]
    fn exact_fact(target: &str) -> opencode_retrieval::ExactFact {
        opencode_retrieval::ExactFact {
            kind: opencode_retrieval::ExactFactKind::Definition,
            locations: vec![opencode_retrieval::ExactFactLocation {
                file: target.to_string(),
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 1,
            }],
            server_id: "stub-server".to_string(),
            document_version: Some(1),
            file_hash: 7,
            cached: false,
        }
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn exact_fact_enrichment_labels_edges_exact_with_provenance() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
        )
        .unwrap();
        let target = dir.path().join("target.rs");
        std::fs::write(&target, "pub struct Target;\n").unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["lib.rs"]);
        req.objective = "add".to_string();

        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let provider =
            ScopemuxProvider::with_exact_facts(Some(std::sync::Arc::new(StubExactFacts {
                calls: calls.clone(),
                outcome: opencode_retrieval::ExactFactOutcome::Exact(exact_fact(
                    &target.to_string_lossy(),
                )),
            })));

        let response = provider.retrieve(&req).await.unwrap();

        assert_eq!(
            response.provider, "scopemux",
            "enrichment must not change the provider identity"
        );
        assert!(
            calls.load(std::sync::atomic::Ordering::SeqCst) >= 1,
            "expected at least one exact-fact anchor query"
        );
        assert!(
            calls.load(std::sync::atomic::Ordering::SeqCst) <= MAX_EXACT_FACT_ANCHORS,
            "exact-fact queries must stay bounded"
        );
        let exact = response
            .candidates
            .iter()
            .find(|candidate| candidate.provenance.contains("exact definition"))
            .expect("exact edge should be present");
        assert_eq!(exact.confidence, opencode_types::RetrievalConfidence::Exact);
        assert!(exact.provenance.contains("stub-server"));
        assert!(response.provider_notes.is_empty());
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn unavailable_exact_facts_keep_heuristic_candidates() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["lib.rs"]);
        req.objective = "add".to_string();

        let baseline = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let provider =
            ScopemuxProvider::with_exact_facts(Some(std::sync::Arc::new(StubExactFacts {
                calls: calls.clone(),
                outcome: opencode_retrieval::ExactFactOutcome::Unavailable {
                    reason: opencode_retrieval::ExactFactUnavailable::NoServer,
                },
            })));

        let response = provider.retrieve(&req).await.unwrap();

        assert_eq!(
            response.candidates, baseline.candidates,
            "an unavailable exact-fact source must leave the map result unchanged"
        );
        assert!(
            response
                .candidates
                .iter()
                .all(|candidate| !candidate.provenance.contains("exact definition")),
            "unavailable facts must not produce exact edges"
        );
        assert!(
            response
                .provider_notes
                .iter()
                .any(|note| note.contains("unavailable")),
            "fallback reason should be recorded: {:?}",
            response.provider_notes
        );
        assert!(
            !response.candidates.is_empty(),
            "heuristic map candidates must remain"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn exact_fact_queries_are_bounded_per_request() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        let mut source = String::new();
        for i in 0..40 {
            source.push_str(&format!("pub fn helper_{i}() -> u32 {{ {i} }}\n"));
        }
        std::fs::write(dir.path().join("lib.rs"), source).unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["lib.rs"]);
        req.objective = "helper".to_string();
        req.token_budget = Some(40);

        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let provider =
            ScopemuxProvider::with_exact_facts(Some(std::sync::Arc::new(StubExactFacts {
                calls: calls.clone(),
                outcome: opencode_retrieval::ExactFactOutcome::Exact(
                    opencode_retrieval::ExactFact {
                        kind: opencode_retrieval::ExactFactKind::Definition,
                        locations: Vec::new(),
                        server_id: "stub-server".to_string(),
                        document_version: None,
                        file_hash: 0,
                        cached: false,
                    },
                ),
            })));

        provider.retrieve(&req).await.unwrap();

        let calls = calls.load(std::sync::atomic::Ordering::SeqCst);
        assert!(calls > 0, "expected at least one anchor to be queried");
        assert!(
            calls <= MAX_EXACT_FACT_ANCHORS,
            "expected at most {MAX_EXACT_FACT_ANCHORS} exact-fact queries, got {calls}"
        );
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn without_a_fact_provider_the_map_is_unchanged() {
        use opencode_retrieval::RetrievalProvider;

        let _guard = native_test_lock();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lib.rs"),
            "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
        )
        .unwrap();

        let mut req = request(&dir.path().to_string_lossy(), vec!["lib.rs"]);
        req.objective = "add".to_string();

        let response = ScopemuxProvider::new().retrieve(&req).await.unwrap();

        assert!(response.provider_notes.is_empty());
        assert!(response
            .candidates
            .iter()
            .all(|candidate| !candidate.provenance.contains("exact definition")));
    }
}
