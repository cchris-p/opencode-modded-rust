//! Native `scopemux-core` retrieval provider.
//!
//! When the `native` feature is enabled this implements
//! [`opencode_retrieval::RetrievalProvider`] over the `scopemux-core` C API
//! (project IR, canonical InfoBlock registry, indexed search). Otherwise
//! [`default_provider`] returns the generic repository-local provider, so
//! default builds are unaffected.

use opencode_retrieval::RetrievalProvider;
use opencode_types::RetrievalRequest;

// Link the `tree-sitter` crate's bundled C runtime (its `cc` build emits
// `rustc-link-lib=static=tree-sitter`). The Rust API is unused here; the import
// only exists so the native C core resolves its `ts_*` symbols against the
// single runtime the rest of the product already links.
#[cfg(feature = "native")]
use tree_sitter as _;

/// Select the retrieval provider for the runtime.
///
/// Native builds prefer the scopemux provider; if it is unavailable or errors,
/// the session falls back to the generic provider. Non-native builds always
/// return the generic provider.
///
/// This ignores the active model. Runtime prompt assembly uses
/// [`provider_for`] so scopemux only activates on its scoped local model.
pub fn default_provider() -> Box<dyn RetrievalProvider> {
    #[cfg(feature = "native")]
    {
        Box::new(ScopemuxProvider::new())
    }
    #[cfg(not(feature = "native"))]
    {
        Box::new(opencode_retrieval::GenericRepositoryProvider::new())
    }
}

/// Provider id whose local models may use the scopemux provider.
pub const SCOPEMUX_LOCAL_PROVIDER_ID: &str = "ollama";

/// Model id prefix that activates the scopemux provider on the local path.
pub const SCOPEMUX_LOCAL_MODEL_PREFIX: &str = "qwen";

/// Whether the active model is the local path scopemux is scoped to: the Ollama
/// provider serving a Qwen model.
pub fn model_scope_enabled(provider_id: Option<&str>, model_id: Option<&str>) -> bool {
    let provider_matches =
        provider_id.is_some_and(|id| id.eq_ignore_ascii_case(SCOPEMUX_LOCAL_PROVIDER_ID));
    let model_matches = model_id.is_some_and(|model| {
        model
            .trim()
            .to_ascii_lowercase()
            .starts_with(SCOPEMUX_LOCAL_MODEL_PREFIX)
    });
    provider_matches && model_matches
}

/// Select the retrieval provider for the active model.
///
/// The scopemux provider is scoped to the local Qwen-via-Ollama model so
/// improvements can be experimented on a small local model; every other model
/// keeps the generic repository-local provider as the default and fallback.
/// Non-native builds always return the generic provider.
pub fn provider_for(
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> Box<dyn RetrievalProvider> {
    // Only consulted by native builds; keep the parameters used either way.
    let _ = (provider_id, model_id);

    #[cfg(feature = "native")]
    {
        if model_scope_enabled(provider_id, model_id) {
            return Box::new(ScopemuxProvider::new());
        }
    }

    Box::new(opencode_retrieval::GenericRepositoryProvider::new())
}

/// Map a file path to the `scopemux-core` `Language` enum value, or `None` when
/// the extension is not a language `scopemux-core` parses.
pub fn language_for_path(path: &str) -> Option<i32> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "c" | "h" => Some(1),                           // LANG_C
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => Some(2), // LANG_CPP
        "py" => Some(3),                                // LANG_PYTHON
        "js" | "mjs" | "cjs" => Some(4),                // LANG_JAVASCRIPT
        "ts" | "tsx" => Some(5),                        // LANG_TYPESCRIPT
        "rs" => Some(6),                                // LANG_RUST
        _ => None,
    }
}

/// True when at least one referenced file is in a language `scopemux-core`
/// can parse.
pub fn workspace_supported(request: &RetrievalRequest) -> bool {
    request
        .seed_files
        .iter()
        .chain(request.changed_files.iter())
        .any(|p| language_for_path(p).is_some())
}

/// The native provider. See the module docs.
#[cfg(feature = "native")]
#[derive(Debug, Default, Clone, Copy)]
pub struct ScopemuxProvider;

#[cfg(feature = "native")]
impl ScopemuxProvider {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "native")]
mod ffi {
    use std::ffi::c_void;
    use std::os::raw::{c_char, c_int};

    #[repr(C)]
    pub struct ProjectContext {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct ProjectInfoBlock {
        pub id: *mut c_char,
        pub name: *mut c_char,
        pub qualified_name: *mut c_char,
        pub file_path: *mut c_char,
        pub node: *mut c_void,
        pub node_type: c_int,
        pub language: c_int,
        pub kind: c_int,
        pub tier: c_int,
        pub estimated_tokens: usize,
        pub related_symbol_count: usize,
        pub origin: c_int,
        pub lifecycle: c_int,
        pub provenance: *mut c_char,
        pub confidence: f32,
        pub plan_kind: c_int,
        pub desired_shape: *mut c_char,
        pub rationale: *mut c_char,
        pub anchor_list: *mut c_char,
    }

    #[repr(C)]
    pub struct ProjectPlanNode {
        pub id: *mut c_char,
        pub task_id: *mut c_char,
        pub slug: *mut c_char,
        pub title: *mut c_char,
        pub desired_shape: *mut c_char,
        pub rationale: *mut c_char,
        pub provenance: *mut c_char,
        pub projected_symbol: *mut c_char,
        pub file_path: *mut c_char,
        pub kind: c_int,
        pub lifecycle: c_int,
        pub confidence: f32,
        pub anchor_ids: *mut *mut c_char,
        pub anchor_count: usize,
        pub anchor_capacity: usize,
    }

    #[repr(C)]
    pub struct ProjectPlanNodeReconciliationEntry {
        pub node: *const ProjectPlanNode,
        pub previous_lifecycle: c_int,
        pub new_lifecycle: c_int,
    }

    #[repr(C)]
    pub struct ProjectPlanNodeReconciliationResult {
        pub entries: *mut ProjectPlanNodeReconciliationEntry,
        pub entry_count: usize,
        pub stale_count: usize,
        pub conflict_count: usize,
        pub implemented_count: usize,
    }

    #[repr(C)]
    pub struct ProjectSearchRequest {
        pub query_text: *const c_char,
        pub anchor_symbol: *const c_char,
        pub anchor_file_path: *const c_char,
        pub min_tier: c_int,
        pub max_tier: c_int,
        pub include_related: bool,
        pub include_dependencies: bool,
        pub max_hits: usize,
        pub origin_mask: u32,
        pub lifecycle_mask: u32,
    }

    #[repr(C)]
    pub struct ProjectSearchHit {
        pub block: *const ProjectInfoBlock,
        pub score: usize,
        pub name_match: bool,
        pub text_match: bool,
        pub relationship_match: bool,
    }

    #[repr(C)]
    pub struct ProjectSearchResult {
        pub hits: *mut ProjectSearchHit,
        pub hit_count: usize,
        pub total_match_count: usize,
    }

    // WI-036 delta view. Layout mirrors `ProjectDeltaEntry`/`ProjectDeltaResult`.
    #[repr(C)]
    pub struct ProjectDeltaEntry {
        pub kind: c_int,
        pub block: *const ProjectInfoBlock,
        pub plan_node: *const ProjectPlanNode,
        pub anchors: *const c_char,
        pub projected_shape: *const c_char,
        pub provenance: *const c_char,
        pub confidence: f32,
        pub lifecycle: c_int,
        pub estimated_tokens: usize,
    }

    #[repr(C)]
    pub struct ProjectDeltaResult {
        pub task_id: *const c_char,
        pub stage: *const c_char,
        pub entries: *mut ProjectDeltaEntry,
        pub entry_count: usize,
        pub add_count: usize,
        pub change_count: usize,
        pub remove_count: usize,
        pub reuse_count: usize,
        pub estimated_tokens: usize,
    }

    // WI-036 map query surface. Layout mirrors `ProjectMapResultItem`/
    // `ProjectMapQueryResult`.
    #[repr(C)]
    pub struct ProjectMapResultItem {
        pub block: *const ProjectInfoBlock,
        pub kind: c_int,
        pub reason: *const c_char,
        pub provenance: *const c_char,
        pub confidence: f32,
        pub estimated_tokens: usize,
        pub distance: usize,
    }

    #[repr(C)]
    pub struct ProjectMapQueryResult {
        pub kind: c_int,
        pub items: *mut ProjectMapResultItem,
        pub item_count: usize,
        pub estimated_tokens: usize,
    }

    extern "C" {
        pub fn project_context_create(root_directory: *const c_char) -> *mut ProjectContext;
        pub fn project_context_free(project: *mut ProjectContext);
        pub fn project_add_file(
            project: *mut ProjectContext,
            filepath: *const c_char,
            language: c_int,
        ) -> bool;
        pub fn project_parse_all_files(project: *mut ProjectContext) -> bool;
        pub fn project_resolve_references(project: *mut ProjectContext) -> bool;
        pub fn project_context_rebuild_info_blocks(project: *mut ProjectContext) -> bool;
        pub fn project_context_search_info_blocks(
            project: *mut ProjectContext,
            request: *const ProjectSearchRequest,
            out_result: *mut ProjectSearchResult,
        ) -> bool;
        pub fn project_search_result_free(result: *mut ProjectSearchResult);
        pub fn project_context_plan_node_create(
            project: *mut ProjectContext,
            task_id: *const c_char,
            slug: *const c_char,
            kind: c_int,
        ) -> *mut ProjectPlanNode;
        pub fn project_context_plan_node_set_title(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            title: *const c_char,
        ) -> bool;
        pub fn project_context_plan_node_set_desired_shape(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            desired_shape: *const c_char,
        ) -> bool;
        pub fn project_context_plan_node_set_rationale(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            rationale: *const c_char,
        ) -> bool;
        pub fn project_context_plan_node_set_provenance(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            provenance: *const c_char,
        ) -> bool;
        pub fn project_context_plan_node_set_projected_symbol(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            symbol_name: *const c_char,
        ) -> bool;
        pub fn project_context_plan_node_set_file_path(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            file_path: *const c_char,
        ) -> bool;
        pub fn project_context_plan_node_set_lifecycle(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            lifecycle: c_int,
        ) -> bool;
        pub fn project_context_plan_node_add_anchor(
            project: *mut ProjectContext,
            node: *mut ProjectPlanNode,
            anchor_block_id: *const c_char,
        ) -> bool;
        pub fn project_context_reconcile_plan_nodes(
            project: *mut ProjectContext,
            out_result: *mut ProjectPlanNodeReconciliationResult,
        ) -> bool;
        pub fn project_plan_node_reconciliation_result_free(
            result: *mut ProjectPlanNodeReconciliationResult,
        );
        pub fn project_context_compute_delta(
            project: *mut ProjectContext,
            task_id: *const c_char,
            stage: *const c_char,
            out_result: *mut ProjectDeltaResult,
        ) -> bool;
        pub fn project_context_query_resolve(
            project: *mut ProjectContext,
            task_id: *const c_char,
            stage: *const c_char,
            out_result: *mut ProjectMapQueryResult,
        ) -> bool;
        pub fn project_context_query_duplicates(
            project: *mut ProjectContext,
            scope: *const c_char,
            out_result: *mut ProjectMapQueryResult,
        ) -> bool;
        pub fn project_context_query_observability(
            project: *mut ProjectContext,
            symbol: *const c_char,
            out_result: *mut ProjectMapQueryResult,
        ) -> bool;
        pub fn project_context_query_change_impact(
            project: *mut ProjectContext,
            files: *const *const c_char,
            file_count: usize,
            out_result: *mut ProjectMapQueryResult,
        ) -> bool;
        pub fn project_map_query_result_free(result: *mut ProjectMapQueryResult);
        pub fn project_delta_result_free(result: *mut ProjectDeltaResult);
    }
}

#[cfg(feature = "native")]
#[async_trait::async_trait]
impl RetrievalProvider for ScopemuxProvider {
    fn name(&self) -> &str {
        "scopemux"
    }

    async fn retrieve(
        &self,
        request: &RetrievalRequest,
    ) -> Result<opencode_types::RetrievalResponse, opencode_retrieval::RetrievalError> {
        use std::ffi::CString;

        use opencode_retrieval::RetrievalError;
        use opencode_types::RetrievalRepresentation;

        if !workspace_supported(request) {
            return Err(RetrievalError::Unavailable(
                "workspace language is not supported by scopemux-core".to_string(),
            ));
        }

        // Point scopemux-core at the Tree-sitter queries shipped with the
        // pinned source (recorded at build time) unless already configured.
        if std::env::var_os("SCMU_QUERIES_DIR").is_none() {
            if let Some(queries_dir) = option_env!("SCOPEMUX_QUERIES_DIR") {
                std::env::set_var("SCMU_QUERIES_DIR", queries_dir);
            }
        }

        let seeds: Vec<String> = request
            .seed_files
            .iter()
            .chain(request.changed_files.iter())
            .map(|p| p.strip_prefix("file://").unwrap_or(p).to_string())
            .filter(|p| language_for_path(p).is_some())
            .collect();

        let root = CString::new(request.workspace_root.as_str())
            .map_err(|e| RetrievalError::Failed(format!("invalid workspace root: {e}")))?;

        unsafe {
            let ctx = ffi::project_context_create(root.as_ptr());
            if ctx.is_null() {
                return Err(RetrievalError::Failed(
                    "project_context_create returned null".to_string(),
                ));
            }

            // Ensure cleanup on every exit path.
            let guard = ProjectContextGuard(ctx);

            for seed in &seeds {
                if let Ok(path) = CString::new(seed.as_str()) {
                    ffi::project_add_file(ctx, path.as_ptr(), 0); // LANG_UNKNOWN: auto-detect
                }
            }

            if !ffi::project_parse_all_files(ctx) {
                return Err(RetrievalError::Failed("project parse failed".to_string()));
            }
            if !ffi::project_resolve_references(ctx) {
                tracing::warn!("scopemux: project_resolve_references returned false");
            }
            if !ffi::project_context_rebuild_info_blocks(ctx) {
                return Err(RetrievalError::Failed(
                    "info block rebuild failed".to_string(),
                ));
            }

            // SCOPE-003: materialize the task's projected plan nodes into the
            // map so the delta/observability/duplicate slices can see target
            // state. SCOPE-004: select the representation the stage asked for.
            materialize_plan_nodes(ctx, request);
            let candidates = match request.representation {
                RetrievalRepresentation::Search => search_candidates(ctx, request, &seeds)?,
                RetrievalRepresentation::Delta => delta_candidates(ctx, request),
                RetrievalRepresentation::Anchors => resolve_candidates(ctx, request),
                RetrievalRepresentation::Observability => observability_candidates(ctx, request),
                RetrievalRepresentation::Duplicates => duplicates_candidates(ctx, request),
                RetrievalRepresentation::Impact => impact_candidates(ctx, request),
                RetrievalRepresentation::Review => review_candidates(ctx, request),
                RetrievalRepresentation::Reconcile => Vec::new(),
            };

            // Reconciliation evidence is always reported; the runtime decides
            // whether to act on it. It never advances task stage or completion.
            let plan_signals = reconcile_plan_nodes(ctx);

            drop(guard);

            Ok(opencode_types::RetrievalResponse {
                candidates,
                provider: self.name().to_string(),
                representation: request.representation,
                plan_signals,
            })
        }
    }
}

/// Search over the canonical InfoBlock registry (`Search` representation).
#[cfg(feature = "native")]
fn search_candidates(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
    seeds: &[String],
) -> Result<Vec<opencode_types::RetrievalCandidate>, opencode_retrieval::RetrievalError> {
    use std::ffi::CString;

    use opencode_retrieval::RetrievalError;
    use opencode_types::RetrievalConfidence;

    let query = CString::new(request.objective.as_str())
        .map_err(|e| RetrievalError::Failed(format!("invalid objective: {e}")))?;
    let anchor = seeds.first().and_then(|s| CString::new(s.as_str()).ok());

    let search_request = ffi::ProjectSearchRequest {
        query_text: query.as_ptr(),
        anchor_symbol: std::ptr::null(),
        anchor_file_path: anchor.as_ref().map_or(std::ptr::null(), |a| a.as_ptr()),
        min_tier: 0,
        max_tier: 4,
        include_related: false,
        include_dependencies: true,
        max_hits: request.token_budget.unwrap_or(20),
        origin_mask: 0,    // all origins
        lifecycle_mask: 0, // all lifecycles
    };

    let mut result = ffi::ProjectSearchResult {
        hits: std::ptr::null_mut(),
        hit_count: 0,
        total_match_count: 0,
    };

    if !unsafe {
        ffi::project_context_search_info_blocks(ctx, &search_request, &mut result as *mut _)
    } {
        return Err(RetrievalError::Failed("project search failed".to_string()));
    }

    let _result_guard = SearchResultGuard(&mut result as *mut _);

    let mut candidates = Vec::new();
    for i in 0..result.hit_count {
        let hit = unsafe { &*result.hits.add(i) };
        let block = hit.block;
        if block.is_null() {
            continue;
        }
        let id = unsafe { cstr((*block).id) };

        // Parsed blocks carry confidence 1.0 and are exact facts; otherwise
        // fall back to the search match type.
        let confidence = if unsafe { (*block).confidence } >= 0.99 {
            RetrievalConfidence::Exact
        } else if hit.name_match || hit.text_match {
            RetrievalConfidence::High
        } else if hit.relationship_match {
            RetrievalConfidence::Medium
        } else {
            RetrievalConfidence::Low
        };

        let provenance = {
            let source = unsafe { cstr((*block).provenance) };
            if source.is_empty() {
                format!("scopemux search hit (id: {id})")
            } else {
                format!("scopemux: {source}")
            }
        };

        if let Some(candidate) =
            unsafe { block_candidate(block, provenance, confidence, hit.score as f32, None) }
        {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

/// Delta between current (parsed) and target (planned) state (`Delta`).
#[cfg(feature = "native")]
fn delta_candidates(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
) -> Vec<opencode_types::RetrievalCandidate> {
    use std::ffi::CString;

    let task_id = request
        .task_id
        .as_deref()
        .and_then(|id| CString::new(id).ok());
    let stage = CString::new(stage_label(&request.stage)).ok();

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
            ctx,
            task_id.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            stage.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            &mut result as *mut _,
        )
    };
    if !ok {
        return Vec::new();
    }
    let _guard = DeltaResultGuard(&mut result as *mut _);

    let mut candidates = Vec::new();
    for i in 0..result.entry_count {
        let entry = unsafe { &*result.entries.add(i) };
        let label = match entry.kind {
            0 => "delta add",
            1 => "delta change",
            2 => "delta remove",
            3 => "delta reuse",
            _ => "delta",
        };
        let provenance = {
            let source = cstr(entry.provenance);
            if source.is_empty() {
                format!("scopemux: {label}")
            } else {
                format!("scopemux: {label}: {source}")
            }
        };
        let confidence = confidence_from_score(entry.confidence);
        if let Some(candidate) =
            unsafe { block_candidate(entry.block, provenance, confidence, entry.confidence, None) }
        {
            candidates.push(candidate);
        }
    }

    candidates
}

/// Seed nodes and anchors resolved for a task and stage (`Anchors`).
#[cfg(feature = "native")]
fn resolve_candidates(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
) -> Vec<opencode_types::RetrievalCandidate> {
    use std::ffi::CString;

    let task_id = request
        .task_id
        .as_deref()
        .and_then(|id| CString::new(id).ok());
    let stage = CString::new(stage_label(&request.stage)).ok();

    let mut result = empty_map_result();
    let ok = unsafe {
        ffi::project_context_query_resolve(
            ctx,
            task_id.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            stage.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            &mut result as *mut _,
        )
    };
    if !ok {
        return Vec::new();
    }
    map_query_candidates(&mut result)
}

/// Observability blocks attached to anchored symbols (`Observability`).
///
/// Symbols come from the request's explicit `seed_symbols` plus the projected
/// symbols of its plan nodes, so a review slice can resolve the symbol a task
/// intends to touch to its observability points and covering tests.
#[cfg(feature = "native")]
fn observability_candidates(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
) -> Vec<opencode_types::RetrievalCandidate> {
    use std::ffi::CString;

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
        let Ok(symbol_c) = CString::new(symbol.as_str()) else {
            continue;
        };
        let mut result = empty_map_result();
        let ok = unsafe {
            ffi::project_context_query_observability(ctx, symbol_c.as_ptr(), &mut result as *mut _)
        };
        if ok {
            candidates.extend(map_query_candidates(&mut result));
        }
    }
    candidates
}

/// Duplicate/refactor-opportunity clusters (`Duplicates`).
///
/// Detection is scoped to the whole parsed project rather than one seed file,
/// because a duplicate is only visible across files. The provider only parses
/// the request's seeds, so this stays bounded.
#[cfg(feature = "native")]
fn duplicates_candidates(
    ctx: *mut ffi::ProjectContext,
    _request: &RetrievalRequest,
) -> Vec<opencode_types::RetrievalCandidate> {
    let mut result = empty_map_result();
    let ok = unsafe {
        ffi::project_context_query_duplicates(ctx, std::ptr::null(), &mut result as *mut _)
    };
    if !ok {
        return Vec::new();
    }
    map_query_candidates(&mut result)
}

/// Impact of changed files: their blocks plus anchored plans (`Impact`).
#[cfg(feature = "native")]
fn impact_candidates(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
) -> Vec<opencode_types::RetrievalCandidate> {
    use std::ffi::CString;

    let files: Vec<CString> = request
        .changed_files
        .iter()
        .filter_map(|p| CString::new(p.strip_prefix("file://").unwrap_or(p)).ok())
        .collect();
    let pointers: Vec<*const std::os::raw::c_char> = files.iter().map(|c| c.as_ptr()).collect();

    let mut result = empty_map_result();
    let ok = unsafe {
        ffi::project_context_query_change_impact(
            ctx,
            pointers.as_ptr(),
            pointers.len(),
            &mut result as *mut _,
        )
    };
    if !ok {
        return Vec::new();
    }
    map_query_candidates(&mut result)
}

/// Composed review slice: delta, then duplicates and observability (`Review`).
#[cfg(feature = "native")]
fn review_candidates(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
) -> Vec<opencode_types::RetrievalCandidate> {
    let mut candidates = delta_candidates(ctx, request);
    candidates.extend(duplicates_candidates(ctx, request));
    candidates.extend(observability_candidates(ctx, request));

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
    candidates
}

/// Convert map query result items into candidates and free the result.
#[cfg(feature = "native")]
fn map_query_candidates(
    result: &mut ffi::ProjectMapQueryResult,
) -> Vec<opencode_types::RetrievalCandidate> {
    use opencode_types::RetrievalCandidateKind;

    let mut candidates = Vec::new();
    for i in 0..result.item_count {
        let item = unsafe { &*result.items.add(i) };
        let reason = cstr(item.reason);
        let provenance = {
            let source = cstr(item.provenance);
            if source.is_empty() {
                format!("scopemux: {reason}")
            } else {
                format!("scopemux: {reason}: {source}")
            }
        };
        // `4` is `PROJECT_MAP_QUERY_DUPLICATES`; `5` is
        // `PROJECT_MAP_QUERY_OBSERVABILITY` in `ProjectMapQueryKind`.
        let kind_override = match item.kind {
            4 => Some(RetrievalCandidateKind::RefactorOpportunity),
            5 => Some(RetrievalCandidateKind::Observability),
            _ => None,
        };
        let confidence = confidence_from_score(item.confidence);
        if let Some(candidate) = unsafe {
            block_candidate(
                item.block,
                provenance,
                confidence,
                item.confidence,
                kind_override,
            )
        } {
            candidates.push(candidate);
        }
    }
    unsafe { ffi::project_map_query_result_free(result as *mut _) };
    candidates
}

#[cfg(feature = "native")]
fn empty_map_result() -> ffi::ProjectMapQueryResult {
    ffi::ProjectMapQueryResult {
        kind: 0,
        items: std::ptr::null_mut(),
        item_count: 0,
        estimated_tokens: 0,
    }
}

/// Build a candidate from a canonical InfoBlock.
///
/// Kind is derived from the stable id scheme (`sym:`/`file:`) unless the caller
/// knows the map slice implies a specific kind (`Observability`,
/// `RefactorOpportunity`).
#[cfg(feature = "native")]
unsafe fn block_candidate(
    block: *const ffi::ProjectInfoBlock,
    provenance: String,
    confidence: opencode_types::RetrievalConfidence,
    score: f32,
    kind_override: Option<opencode_types::RetrievalCandidateKind>,
) -> Option<opencode_types::RetrievalCandidate> {
    use opencode_types::{
        PlanLifecycle, RetrievalCandidate, RetrievalCandidateKind, RetrievalOrigin,
    };

    if block.is_null() {
        return None;
    }
    let block = &*block;
    let id = cstr(block.id);
    let name = cstr(block.name);
    let qualified_name = cstr(block.qualified_name);
    let file_path = cstr(block.file_path);

    let kind = kind_override.unwrap_or_else(|| {
        if id.starts_with("sym:") {
            RetrievalCandidateKind::Symbol
        } else if id.starts_with("file:") {
            RetrievalCandidateKind::File
        } else {
            RetrievalCandidateKind::Snippet
        }
    });
    let symbol = if !qualified_name.is_empty() {
        Some(qualified_name)
    } else if !name.is_empty() {
        Some(name)
    } else {
        None
    };

    Some(RetrievalCandidate {
        kind,
        path: file_path,
        symbol,
        snippet: None,
        provenance,
        confidence,
        score,
        estimated_tokens: Some(block.estimated_tokens),
        origin: Some(RetrievalOrigin::from_ffi(block.origin)),
        lifecycle: Some(PlanLifecycle::from_ffi(block.lifecycle)),
    })
}

#[cfg(feature = "native")]
fn confidence_from_score(value: f32) -> opencode_types::RetrievalConfidence {
    use opencode_types::RetrievalConfidence;
    if value >= 0.99 {
        RetrievalConfidence::Exact
    } else if value >= 0.85 {
        RetrievalConfidence::High
    } else if value >= 0.6 {
        RetrievalConfidence::Medium
    } else {
        RetrievalConfidence::Low
    }
}

/// Runtime stage label recorded on map requests (not interpreted by the core).
#[cfg(feature = "native")]
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

/// Materialize projected plan nodes into the map. Best-effort: a malformed
/// draft is skipped rather than failing retrieval.
#[cfg(feature = "native")]
fn materialize_plan_nodes(ctx: *mut ffi::ProjectContext, request: &RetrievalRequest) {
    use std::ffi::CString;

    use opencode_types::stage_lifecycle;

    if request.plan_nodes.is_empty() {
        return;
    }

    let task_id = request
        .task_id
        .clone()
        .unwrap_or_else(|| "task".to_string());
    let Ok(task_id_c) = CString::new(task_id.as_str()) else {
        return;
    };
    let provenance =
        CString::new(format!("task:{task_id}:{:?}", request.stage).to_lowercase()).ok();
    let lifecycle = stage_lifecycle(&request.stage).as_ffi();

    for draft in &request.plan_nodes {
        let Ok(slug) = CString::new(draft.slug.as_str()) else {
            continue;
        };
        let node = unsafe {
            ffi::project_context_plan_node_create(
                ctx,
                task_id_c.as_ptr(),
                slug.as_ptr(),
                draft.kind.as_ffi(),
            )
        };
        if node.is_null() {
            continue;
        }

        unsafe {
            if let Ok(title) = CString::new(draft.title.as_str()) {
                ffi::project_context_plan_node_set_title(ctx, node, title.as_ptr());
            }
            if let Some(shape) = draft
                .desired_shape
                .as_deref()
                .and_then(|s| CString::new(s).ok())
            {
                ffi::project_context_plan_node_set_desired_shape(ctx, node, shape.as_ptr());
            }
            if let Some(rationale) = draft
                .rationale
                .as_deref()
                .and_then(|s| CString::new(s).ok())
            {
                ffi::project_context_plan_node_set_rationale(ctx, node, rationale.as_ptr());
            }
            if let Some(symbol) = draft
                .projected_symbol
                .as_deref()
                .and_then(|s| CString::new(s).ok())
            {
                ffi::project_context_plan_node_set_projected_symbol(ctx, node, symbol.as_ptr());
            }
            if let Some(path) = draft
                .file_path
                .as_deref()
                .and_then(|s| CString::new(s).ok())
            {
                ffi::project_context_plan_node_set_file_path(ctx, node, path.as_ptr());
            }
            if let Some(provenance) = provenance.as_ref() {
                ffi::project_context_plan_node_set_provenance(ctx, node, provenance.as_ptr());
            }
            ffi::project_context_plan_node_set_lifecycle(ctx, node, lifecycle);
            for anchor in &draft.anchors {
                if let Ok(anchor_c) = CString::new(anchor.as_str()) {
                    ffi::project_context_plan_node_add_anchor(ctx, node, anchor_c.as_ptr());
                }
            }
        }
    }
}

/// Reconcile materialized plan nodes against parsed state and report evidence.
/// The runtime decides whether to act; this never advances stage or completion.
#[cfg(feature = "native")]
fn reconcile_plan_nodes(
    ctx: *mut ffi::ProjectContext,
) -> Vec<opencode_types::PlanReconciliationSignal> {
    use opencode_types::{PlanLifecycle, PlanReconciliationSignal};

    let mut signals = Vec::new();
    let mut reconcile = ffi::ProjectPlanNodeReconciliationResult {
        entries: std::ptr::null_mut(),
        entry_count: 0,
        stale_count: 0,
        conflict_count: 0,
        implemented_count: 0,
    };
    if !unsafe { ffi::project_context_reconcile_plan_nodes(ctx, &mut reconcile as *mut _) } {
        return signals;
    }

    unsafe {
        for i in 0..reconcile.entry_count {
            let entry = &*reconcile.entries.add(i);
            if entry.node.is_null() {
                continue;
            }
            signals.push(PlanReconciliationSignal {
                plan_node_id: cstr((*entry.node).id),
                previous: PlanLifecycle::from_ffi(entry.previous_lifecycle),
                current: PlanLifecycle::from_ffi(entry.new_lifecycle),
            });
        }
        ffi::project_plan_node_reconciliation_result_free(&mut reconcile as *mut _);
    }

    signals
}

#[cfg(feature = "native")]
fn cstr(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }
}

#[cfg(feature = "native")]
struct ProjectContextGuard(*mut ffi::ProjectContext);

#[cfg(feature = "native")]
impl Drop for ProjectContextGuard {
    fn drop(&mut self) {
        unsafe { ffi::project_context_free(self.0) }
    }
}

#[cfg(feature = "native")]
struct SearchResultGuard(*mut ffi::ProjectSearchResult);

#[cfg(feature = "native")]
impl Drop for SearchResultGuard {
    fn drop(&mut self) {
        unsafe { ffi::project_search_result_free(self.0) }
    }
}

#[cfg(feature = "native")]
struct DeltaResultGuard(*mut ffi::ProjectDeltaResult);

#[cfg(feature = "native")]
impl Drop for DeltaResultGuard {
    fn drop(&mut self) {
        unsafe { ffi::project_delta_result_free(self.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_types::{MapRole, RetrievalRepresentation, RetrievalRequest, TaskStage};

    /// The C core keeps process-global parser/query state and is not safe for
    /// concurrent use, so native tests serialize behind one lock.
    #[cfg(feature = "native")]
    fn native_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

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
}
