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

#[cfg(feature = "native")]
mod engine;
#[cfg(feature = "native")]
mod ffi;
#[cfg(feature = "native")]
mod provider;
#[cfg(feature = "native")]
mod supervisor;

#[cfg(feature = "native")]
pub use engine::{
    BlockView, DeltaEntryView, EngineCounters, EngineError, MapItemView, PlanNodeInput,
    ReconcileView, SearchHitView, SearchRequest, WorkspaceEngine,
};
#[cfg(feature = "native")]
pub use provider::ScopemuxProvider;
#[cfg(feature = "native")]
pub use supervisor::{
    MapEntry, MapHandle, RegistryWorkspaceSearch, SupervisorError, WorkspaceMapRegistry,
    DEFAULT_DEBOUNCE, DEFAULT_TTL,
};

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
    provider_for_with_facts(provider_id, model_id, None)
}

/// Same selection as [`provider_for`], but optionally attaches an exact-fact source.
/// LSP is never required: `None`, a missing server, or a failed query leaves the
/// heuristic map behavior intact.
pub fn provider_for_with_facts(
    provider_id: Option<&str>,
    model_id: Option<&str>,
    exact_facts: Option<std::sync::Arc<dyn opencode_retrieval::ExactFactProvider>>,
) -> Box<dyn RetrievalProvider> {
    // Only consulted by native builds; keep the parameters used either way.
    let _ = (provider_id, model_id, &exact_facts);

    #[cfg(feature = "native")]
    {
        if model_scope_enabled(provider_id, model_id) {
            return Box::new(ScopemuxProvider::with_exact_facts(exact_facts));
        }
    }

    Box::new(opencode_retrieval::GenericRepositoryProvider::new())
}

/// Map a file path to the `scopemux-core` `Language` enum value, or `None` when
/// the extension is not a language `scopemux-core` parses.
pub fn language_for_path(path: &str) -> Option<i32> {
    opencode_types::SourceLanguage::from_path(path)
        .map(opencode_types::SourceLanguage::scopemux_language_id)
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

#[cfg(test)]
mod policy_tests {
    use super::*;
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
    #[test]
    fn workspace_search_capability_follows_the_activation_scope() {
        assert!(workspace_search_for(Some("ollama"), Some("qwen3:30b")).is_some());
        assert!(workspace_search_for(Some("openai"), Some("gpt-5")).is_none());
        assert!(workspace_search_for(Some("ollama"), Some("llama3:8b")).is_none());
        assert!(workspace_search_for(None, None).is_none());
    }

    #[test]
    fn activation_is_the_qwen_only_rule() {
        assert!(model_scope_enabled(Some("ollama"), Some("qwen3:30b")));
        assert!(!model_scope_enabled(Some("ollama"), Some("llama3:8b")));
        assert!(!model_scope_enabled(None, None));
    }
}

/// Process-wide persistent map registry.
///
/// It owns the workers and lifecycle; it does not decide fallback or task state.
/// The provider and the tool capability resolve maps by workspace root from it.
#[cfg(feature = "native")]
pub fn shared_registry() -> std::sync::Arc<WorkspaceMapRegistry> {
    static REGISTRY: std::sync::OnceLock<std::sync::Arc<WorkspaceMapRegistry>> =
        std::sync::OnceLock::new();
    REGISTRY
        .get_or_init(|| std::sync::Arc::new(WorkspaceMapRegistry::default()))
        .clone()
}

/// Build the map-backed search capability when the active model is in scope.
///
/// Activation is the same single `model_scope_enabled` predicate used by the
/// provider. The tool only receives the neutral capability; it never parses model
/// strings.
#[cfg(feature = "native")]
pub fn workspace_search_for(
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> Option<std::sync::Arc<dyn opencode_types::WorkspaceSearch>> {
    if model_scope_enabled(provider_id, model_id) {
        Some(std::sync::Arc::new(RegistryWorkspaceSearch::new(
            shared_registry(),
        )))
    } else {
        None
    }
}

/// Non-native build: the persistent map is unavailable, so no capability is
/// injected and the tool keeps grep/generic behavior.
#[cfg(not(feature = "native"))]
pub fn workspace_search_for(
    _provider_id: Option<&str>,
    _model_id: Option<&str>,
) -> Option<std::sync::Arc<dyn opencode_types::WorkspaceSearch>> {
    None
}
