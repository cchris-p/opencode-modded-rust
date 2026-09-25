//! Native `scopemux-core` retrieval provider.
//!
//! When the `native` feature is enabled this implements
//! [`opencode_retrieval::RetrievalProvider`] over the `scopemux-core` C API
//! (project IR, canonical InfoBlock registry, indexed search). Otherwise
//! [`default_provider`] returns the generic repository-local provider, so
//! default builds are unaffected.

use opencode_retrieval::RetrievalProvider;
use opencode_types::RetrievalRequest;

/// Select the retrieval provider for the runtime.
///
/// Native builds prefer the scopemux provider; if it is unavailable or errors,
/// the session falls back to the generic provider. Non-native builds always
/// return the generic provider.
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
        use opencode_types::{
            PlanLifecycle, RetrievalCandidate, RetrievalCandidateKind, RetrievalConfidence,
            RetrievalOrigin,
        };

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
            let _ = ffi::project_resolve_references(ctx);
            if !ffi::project_context_rebuild_info_blocks(ctx) {
                return Err(RetrievalError::Failed(
                    "info block rebuild failed".to_string(),
                ));
            }

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

            if !ffi::project_context_search_info_blocks(ctx, &search_request, &mut result as *mut _)
            {
                return Err(RetrievalError::Failed("project search failed".to_string()));
            }

            let result_guard = SearchResultGuard(&mut result as *mut _);

            let mut candidates = Vec::new();
            for i in 0..result.hit_count {
                let hit = &*result.hits.add(i);
                let block = hit.block;
                if block.is_null() {
                    continue;
                }
                let file_path = cstr((*block).file_path);
                let qualified_name = cstr((*block).qualified_name);
                let id = cstr((*block).id);

                let (kind, symbol) = if id.starts_with("sym:") {
                    (RetrievalCandidateKind::Symbol, Some(qualified_name.clone()))
                } else if id.starts_with("file:") {
                    (RetrievalCandidateKind::File, None)
                } else {
                    (RetrievalCandidateKind::Snippet, None)
                };

                // Parsed blocks carry confidence 1.0 and are exact facts;
                // otherwise fall back to the search match type.
                let confidence = if (*block).confidence >= 0.99 {
                    RetrievalConfidence::Exact
                } else if hit.name_match || hit.text_match {
                    RetrievalConfidence::High
                } else if hit.relationship_match {
                    RetrievalConfidence::Medium
                } else {
                    RetrievalConfidence::Low
                };

                let provenance = {
                    let source = cstr((*block).provenance);
                    if source.is_empty() {
                        format!("scopemux search hit (id: {id})")
                    } else {
                        format!("scopemux: {source}")
                    }
                };

                candidates.push(RetrievalCandidate {
                    kind,
                    path: file_path,
                    symbol,
                    snippet: None,
                    provenance,
                    confidence,
                    score: hit.score as f32,
                    estimated_tokens: Some((*block).estimated_tokens),
                    origin: Some(RetrievalOrigin::from_ffi((*block).origin)),
                    lifecycle: Some(PlanLifecycle::from_ffi((*block).lifecycle)),
                });
            }

            // SCOPE-003: project the task's plan-node drafts into the map,
            // reconcile them against parsed state, and report evidence. This
            // never advances task stage or completion; the runtime owns that.
            let plan_signals = project_and_reconcile_plan_nodes(ctx, request);

            drop(result_guard);
            drop(guard);

            Ok(opencode_types::RetrievalResponse {
                candidates,
                provider: self.name().to_string(),
                plan_signals,
            })
        }
    }
}

/// Materialize projected plan nodes into the map and report reconciliation
/// evidence. Projection is best-effort: a malformed draft is skipped rather
/// than failing retrieval.
#[cfg(feature = "native")]
fn project_and_reconcile_plan_nodes(
    ctx: *mut ffi::ProjectContext,
    request: &RetrievalRequest,
) -> Vec<opencode_types::PlanReconciliationSignal> {
    use std::ffi::CString;

    use opencode_types::{stage_lifecycle, PlanLifecycle, PlanReconciliationSignal};

    let mut signals = Vec::new();
    if request.plan_nodes.is_empty() {
        return signals;
    }

    let task_id = request
        .task_id
        .clone()
        .unwrap_or_else(|| "task".to_string());
    let Ok(task_id_c) = CString::new(task_id.as_str()) else {
        return signals;
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

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_types::{RetrievalRequest, RetrievalRole, TaskStage};

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
            role: RetrievalRole::Implementing,
            token_budget: None,
            task_id: None,
            plan_nodes: Vec::new(),
        }
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

    #[test]
    fn default_provider_matches_feature() {
        let provider = default_provider();
        #[cfg(not(feature = "native"))]
        assert_eq!(provider.name(), "generic");
        #[cfg(feature = "native")]
        assert_eq!(provider.name(), "scopemux");
    }
}
