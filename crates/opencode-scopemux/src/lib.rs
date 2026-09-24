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
/// the language is unsupported (`scopemux-core` does not parse Rust).
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
        use opencode_types::{RetrievalCandidate, RetrievalCandidateKind, RetrievalConfidence};

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

                let confidence = if hit.name_match || hit.text_match {
                    RetrievalConfidence::High
                } else if hit.relationship_match {
                    RetrievalConfidence::Medium
                } else {
                    RetrievalConfidence::Low
                };

                candidates.push(RetrievalCandidate {
                    kind,
                    path: file_path,
                    symbol,
                    snippet: None,
                    provenance: format!("scopemux search hit (id: {id})"),
                    confidence,
                    score: hit.score as f32,
                    estimated_tokens: Some((*block).estimated_tokens),
                });
            }

            drop(result_guard);
            drop(guard);

            Ok(opencode_types::RetrievalResponse {
                candidates,
                provider: self.name().to_string(),
            })
        }
    }
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
        }
    }

    #[test]
    fn language_mapping_covers_supported_and_rejects_rust() {
        assert_eq!(language_for_path("a.c"), Some(1));
        assert_eq!(language_for_path("a.cpp"), Some(2));
        assert_eq!(language_for_path("a.py"), Some(3));
        assert_eq!(language_for_path("a.js"), Some(4));
        assert_eq!(language_for_path("a.ts"), Some(5));
        assert_eq!(language_for_path("a.rs"), None);
    }

    #[test]
    fn workspace_supported_requires_a_supported_seed() {
        assert!(!workspace_supported(&request("/ws", vec!["lib.rs"])));
        assert!(workspace_supported(&request(
            "/ws",
            vec!["lib.rs", "main.c"]
        )));
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn native_provider_returns_candidates_for_c_file() {
        use opencode_retrieval::RetrievalProvider;

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
