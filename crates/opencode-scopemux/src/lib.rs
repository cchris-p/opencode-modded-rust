//! Native `scopemux-core` retrieval provider.
//!
//! When the `native` feature is enabled this implements
//! [`opencode_retrieval::RetrievalProvider`] over the `scopemux-core` C API
//! (project IR, canonical InfoBlock registry, indexed search). Otherwise
//! [`default_provider`] returns the generic repository-local provider, so
//! default builds are unaffected.

use opencode_retrieval::RetrievalProvider;
use opencode_types::{
    RetrievalRequest, WorkspaceIntelligenceProfile, WorkspaceIntelligenceProvider,
    DEFAULT_WORKSPACE_INTELLIGENCE_TOKEN_BUDGET,
};

// Link the `tree-sitter` crate's bundled C runtime (its `cc` build emits
// `rustc-link-lib=static=tree-sitter`). The Rust API is unused here; the import
// only exists so the native C core resolves its `ts_*` symbols against the
// single runtime the rest of the product already links.
#[cfg(feature = "native")]
use tree_sitter as _;

mod queries;
pub use queries::ensure_queries_dir;

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
    classify_freshness, MapEntry, MapHandle, RegistryWorkspaceSearch, SupervisorError,
    WorkspaceMapRegistry, DEFAULT_DEBOUNCE, DEFAULT_TTL, SLO_REBUILD_MS, SLO_WATCHER_LAG_MS,
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

/// Name of the built-in local Qwen experiment profile.
pub const QWEN_EXPERIMENT_PROFILE_NAME: &str = "qwen-ollama-experiment";

/// Capability profiles that govern workspace-intelligence activation.
///
/// The runtime resolves exactly one profile for the active model. No match, an
/// invalid profile, or a profile that names the generic provider keeps the
/// generic repository-local provider. Profiles are a hard bound: they only
/// choose a provider and budget, and never widen runtime policy.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceIntelligencePolicy {
    profiles: Vec<WorkspaceIntelligenceProfile>,
    rejected: Vec<(String, String)>,
}

impl WorkspaceIntelligencePolicy {
    /// Build a policy from profiles, dropping invalid ones.
    pub fn new(profiles: Vec<WorkspaceIntelligenceProfile>) -> Self {
        let mut kept = Vec::new();
        let mut rejected = Vec::new();
        for profile in profiles {
            match profile.validate() {
                Ok(()) => kept.push(profile),
                Err(error) => rejected.push((profile.name.clone(), error.to_string())),
            }
        }
        Self {
            profiles: kept,
            rejected,
        }
    }

    /// No profiles: every model uses the generic provider.
    pub fn generic_only() -> Self {
        Self::default()
    }

    /// The built-in experiment profile, preserving the historical Qwen path.
    pub fn qwen_experiment() -> Self {
        Self::new(vec![qwen_experiment_profile()])
    }

    /// Validated profiles, in match order.
    pub fn profiles(&self) -> &[WorkspaceIntelligenceProfile] {
        &self.profiles
    }

    /// Profiles dropped during validation with their reason.
    pub fn rejected(&self) -> &[(String, String)] {
        &self.rejected
    }

    /// First profile that matches the active model.
    pub fn resolve(
        &self,
        provider_id: Option<&str>,
        model_id: Option<&str>,
    ) -> Option<&WorkspaceIntelligenceProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.matches_model(provider_id, model_id))
    }

    /// Whether the active model selects the map-backed provider.
    pub fn selects_scopemux(&self, provider_id: Option<&str>, model_id: Option<&str>) -> bool {
        self.resolve(provider_id, model_id)
            .is_some_and(|profile| profile.provider == WorkspaceIntelligenceProvider::Scopemux)
    }

    /// Token budget for the active model, when a profile matches.
    pub fn token_budget_for(
        &self,
        provider_id: Option<&str>,
        model_id: Option<&str>,
    ) -> Option<usize> {
        self.resolve(provider_id, model_id)
            .map(|profile| profile.token_budget)
    }
}

/// The Qwen-via-Ollama experiment as a named capability profile.
pub fn qwen_experiment_profile() -> WorkspaceIntelligenceProfile {
    WorkspaceIntelligenceProfile {
        name: QWEN_EXPERIMENT_PROFILE_NAME.to_string(),
        provider: WorkspaceIntelligenceProvider::Scopemux,
        enabled: true,
        models: vec!["ollama/qwen*".to_string()],
        rungs: opencode_types::WorkspaceIntelligenceRung::ALL.to_vec(),
        languages: Vec::new(),
        token_budget: DEFAULT_WORKSPACE_INTELLIGENCE_TOKEN_BUDGET,
        fallback: WorkspaceIntelligenceProvider::Generic,
        experiment: true,
    }
}

/// Whether the active model is the built-in local Qwen experiment profile.
///
/// Kept for the existing QA contract; it now implements the named profile
/// rather than acting as the activation policy.
pub fn model_scope_enabled(provider_id: Option<&str>, model_id: Option<&str>) -> bool {
    WorkspaceIntelligencePolicy::qwen_experiment().selects_scopemux(provider_id, model_id)
}

/// Select the retrieval provider for the active model using the default
/// Qwen experiment profile. New callers should pass an explicit policy with
/// [`provider_for_with_policy`].
pub fn provider_for(
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> Box<dyn RetrievalProvider> {
    provider_for_with_facts(provider_id, model_id, None)
}

/// [`provider_for`] with an optional exact-fact source.
pub fn provider_for_with_facts(
    provider_id: Option<&str>,
    model_id: Option<&str>,
    exact_facts: Option<std::sync::Arc<dyn opencode_retrieval::ExactFactProvider>>,
) -> Box<dyn RetrievalProvider> {
    let policy = WorkspaceIntelligencePolicy::qwen_experiment();
    provider_for_with_policy(Some(&policy), provider_id, model_id, exact_facts)
}

/// Select the retrieval provider from an explicit capability-profile policy.
///
/// `None` or a policy with no matching profile keeps the generic
/// repository-local provider. Non-native builds always return the generic
/// provider. LSP exact facts remain optional enrichment.
pub fn provider_for_with_policy(
    policy: Option<&WorkspaceIntelligencePolicy>,
    provider_id: Option<&str>,
    model_id: Option<&str>,
    exact_facts: Option<std::sync::Arc<dyn opencode_retrieval::ExactFactProvider>>,
) -> Box<dyn RetrievalProvider> {
    // Only consulted by native builds; keep the parameters used either way.
    let _ = (provider_id, model_id, &exact_facts, policy);

    #[cfg(feature = "native")]
    {
        if policy.is_some_and(|policy| policy.selects_scopemux(provider_id, model_id)) {
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

    fn profile(name: &str, models: Vec<&str>) -> WorkspaceIntelligenceProfile {
        WorkspaceIntelligenceProfile {
            name: name.to_string(),
            provider: WorkspaceIntelligenceProvider::Scopemux,
            enabled: true,
            models: models.into_iter().map(str::to_string).collect(),
            rungs: Vec::new(),
            languages: Vec::new(),
            token_budget: DEFAULT_WORKSPACE_INTELLIGENCE_TOKEN_BUDGET,
            fallback: WorkspaceIntelligenceProvider::Generic,
            experiment: true,
        }
    }

    #[test]
    fn generic_only_policy_never_activates() {
        let policy = WorkspaceIntelligencePolicy::generic_only();
        assert!(!policy.selects_scopemux(Some("deepseek"), Some("deepseek-flash")));
        assert!(!policy.selects_scopemux(Some("ollama"), Some("qwen3:30b")));
        assert!(policy
            .token_budget_for(Some("deepseek"), Some("deepseek-flash"))
            .is_none());
    }

    #[test]
    fn a_deepseek_profile_can_enable_scopemux_without_a_code_branch() {
        let policy = WorkspaceIntelligencePolicy::new(vec![profile(
            "deepseek-experiment",
            vec!["deepseek/deepseek-flash"],
        )]);
        assert!(policy.selects_scopemux(Some("deepseek"), Some("deepseek-flash")));
        assert_eq!(
            policy.token_budget_for(Some("deepseek"), Some("deepseek-flash")),
            Some(DEFAULT_WORKSPACE_INTELLIGENCE_TOKEN_BUDGET)
        );
        assert!(!policy.selects_scopemux(Some("openai"), Some("gpt-5")));
    }

    #[test]
    fn a_generic_profile_explicitly_keeps_the_default_provider() {
        let mut disabled = profile("disabled", vec!["*"]);
        disabled.provider = WorkspaceIntelligenceProvider::Generic;
        let policy = WorkspaceIntelligencePolicy::new(vec![disabled]);
        assert!(!policy.selects_scopemux(Some("deepseek"), Some("deepseek-flash")));
    }

    #[test]
    fn invalid_profiles_are_rejected_and_fall_back_to_generic() {
        let mut invalid = profile("broken", vec!["*"]);
        invalid.token_budget = 0;
        let policy = WorkspaceIntelligencePolicy::new(vec![invalid]);
        assert!(policy.profiles().is_empty());
        assert_eq!(policy.rejected().len(), 1);
        assert!(!policy.selects_scopemux(Some("deepseek"), Some("deepseek-flash")));
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
/// Activation is the same profile policy used by the provider. The tool only
/// receives the neutral capability; it never parses model strings.
#[cfg(feature = "native")]
pub fn workspace_search_for(
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> Option<std::sync::Arc<dyn opencode_types::WorkspaceSearch>> {
    let policy = WorkspaceIntelligencePolicy::qwen_experiment();
    workspace_search_for_with_policy(Some(&policy), provider_id, model_id)
}

/// Build the map-backed search capability from an explicit profile policy.
#[cfg(feature = "native")]
pub fn workspace_search_for_with_policy(
    policy: Option<&WorkspaceIntelligencePolicy>,
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> Option<std::sync::Arc<dyn opencode_types::WorkspaceSearch>> {
    if policy.is_some_and(|policy| policy.selects_scopemux(provider_id, model_id)) {
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

/// Non-native build: no profile can inject a map capability.
#[cfg(not(feature = "native"))]
pub fn workspace_search_for_with_policy(
    _policy: Option<&WorkspaceIntelligencePolicy>,
    _provider_id: Option<&str>,
    _model_id: Option<&str>,
) -> Option<std::sync::Arc<dyn opencode_types::WorkspaceSearch>> {
    None
}
