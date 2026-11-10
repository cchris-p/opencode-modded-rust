//! Capability profiles for workspace-intelligence (ScopeMux) activation.
//!
//! Activation is policy data (`FUTURE-PROOFING-015`), not a model-id branch in
//! the provider. A profile names the models it applies to, the enabled rungs,
//! the languages it may parse, and the token budget. The runtime treats the
//! profile as a hard bound and falls back to the generic repository-local
//! provider when no profile matches or a profile is invalid.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::SourceLanguage;

/// Default token budget the runtime gives the map for one retrieval request,
/// used when a profile does not set one.
pub const DEFAULT_WORKSPACE_INTELLIGENCE_TOKEN_BUDGET: usize = 400;

/// Workspace-intelligence provider a profile selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceIntelligenceProvider {
    /// The repository-local generic provider.
    #[default]
    Generic,
    /// The native ScopeMux map-backed provider.
    Scopemux,
}

/// A rung of the workspace-intelligence capability ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkspaceIntelligenceRung {
    /// Parser/index: symbols, references, search.
    W0,
    /// Map: delta, observability, refactor opportunities.
    W1,
    /// Task projection: plan nodes, reconciliation signals.
    W2,
    /// Map-backed tools: locate/search over the node registry.
    W3,
    /// Decision integration: retrieval sufficiency, compaction, review triage.
    W4,
}

impl WorkspaceIntelligenceRung {
    /// Every rung, in ladder order.
    pub const ALL: [Self; 5] = [Self::W0, Self::W1, Self::W2, Self::W3, Self::W4];

    /// Stable wire name (`W0`..`W4`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::W0 => "W0",
            Self::W1 => "W1",
            Self::W2 => "W2",
            Self::W3 => "W3",
            Self::W4 => "W4",
        }
    }
}

impl fmt::Display for WorkspaceIntelligenceRung {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkspaceIntelligenceRung {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_uppercase().as_str() {
            "W0" => Ok(Self::W0),
            "W1" => Ok(Self::W1),
            "W2" => Ok(Self::W2),
            "W3" => Ok(Self::W3),
            "W4" => Ok(Self::W4),
            other => Err(format!("unknown workspace-intelligence rung `{other}`")),
        }
    }
}

/// One workspace-intelligence capability profile.
///
/// `models` is a list of selectors matched against the active
/// `provider/model`: `"*"` matches everything, `"deepseek/*"` matches every
/// DeepSeek model, `"*/deepseek-flash"` matches that model on any provider,
/// and a bare token matches either side. An empty list matches every model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIntelligenceProfile {
    /// Stable profile name recorded in the session and decision log.
    pub name: String,
    /// Provider this profile enables.
    #[serde(default)]
    pub provider: WorkspaceIntelligenceProvider,
    /// Disabled profiles never activate.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Provider/model selectors. Empty matches every model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    /// Enabled rungs. Empty means every implemented rung.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rungs: Vec<WorkspaceIntelligenceRung>,
    /// Languages the map may parse. Empty means every supported language.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<SourceLanguage>,
    /// Token budget for one retrieval request.
    #[serde(default = "default_token_budget", alias = "token_budget")]
    pub token_budget: usize,
    /// Fallback provider when the map is absent, fails, or cannot parse.
    #[serde(default)]
    pub fallback: WorkspaceIntelligenceProvider,
    /// Marks an experiment profile so evaluation can distinguish it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub experiment: bool,
}

/// Why a profile was rejected. Rejected profiles never activate.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceIntelligenceProfileError {
    #[error("workspace-intelligence profile name is empty")]
    EmptyName,
    #[error("workspace-intelligence profile `{name}` has a zero token budget")]
    ZeroTokenBudget { name: String },
    #[error(
        "workspace-intelligence profile `{name}` uses an unsupported fallback provider; only `generic` is allowed"
    )]
    UnsupportedFallback { name: String },
}

impl WorkspaceIntelligenceProfile {
    /// Validate the profile. A rejected profile falls back to generic.
    pub fn validate(&self) -> Result<(), WorkspaceIntelligenceProfileError> {
        if self.name.trim().is_empty() {
            return Err(WorkspaceIntelligenceProfileError::EmptyName);
        }
        if self.token_budget == 0 {
            return Err(WorkspaceIntelligenceProfileError::ZeroTokenBudget {
                name: self.name.clone(),
            });
        }
        if self.fallback != WorkspaceIntelligenceProvider::Generic {
            return Err(WorkspaceIntelligenceProfileError::UnsupportedFallback {
                name: self.name.clone(),
            });
        }
        Ok(())
    }

    /// Whether this profile selects the active model.
    pub fn matches_model(&self, provider_id: Option<&str>, model_id: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }
        if self.models.is_empty() {
            return true;
        }

        let provider = provider_id.unwrap_or_default().to_ascii_lowercase();
        let model = model_id.unwrap_or_default().to_ascii_lowercase();

        self.models.iter().any(|selector| {
            let selector = selector.trim().to_ascii_lowercase();
            if selector.is_empty() {
                return false;
            }
            if selector == "*" {
                return true;
            }
            match selector.split_once('/') {
                Some((provider_pattern, model_pattern)) => {
                    glob_match(provider_pattern, &provider) && glob_match(model_pattern, &model)
                }
                None => glob_match(&selector, &model) || glob_match(&selector, &provider),
            }
        })
    }

    /// Whether the map may parse this language under the profile.
    pub fn allows_language(&self, language: SourceLanguage) -> bool {
        self.languages.is_empty() || self.languages.contains(&language)
    }

    /// Whether the profile enables a rung.
    pub fn enables_rung(&self, rung: WorkspaceIntelligenceRung) -> bool {
        self.rungs.is_empty() || self.rungs.contains(&rung)
    }
}

fn default_true() -> bool {
    true
}

fn default_token_budget() -> usize {
    DEFAULT_WORKSPACE_INTELLIGENCE_TOKEN_BUDGET
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Minimal `*` glob match, case-sensitive on already-lowercased input.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_text = 0usize;

    while t < text.len() {
        if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            star_text = t;
            p += 1;
        } else if let Some(star_pos) = star {
            p = star_pos + 1;
            star_text += 1;
            t = star_text;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(models: Vec<&str>) -> WorkspaceIntelligenceProfile {
        WorkspaceIntelligenceProfile {
            name: "test".to_string(),
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
    fn selectors_match_provider_model_pairs() {
        assert!(profile(vec!["deepseek/deepseek-flash"])
            .matches_model(Some("deepseek"), Some("deepseek-flash")));
        assert!(profile(vec!["deepseek/*"]).matches_model(Some("deepseek"), Some("anything")));
        assert!(profile(vec!["*/deepseek-flash"])
            .matches_model(Some("openrouter"), Some("deepseek-flash")));
        assert!(profile(vec!["qwen*"]).matches_model(Some("ollama"), Some("qwen3:30b")));
        assert!(profile(vec![]).matches_model(Some("anything"), Some("at-all")));
        assert!(
            !profile(vec!["ollama/qwen*"]).matches_model(Some("deepseek"), Some("deepseek-flash"))
        );
    }

    #[test]
    fn disabled_or_unmatched_profiles_do_not_activate() {
        let mut disabled = profile(vec!["*"]);
        disabled.enabled = false;
        assert!(!disabled.matches_model(Some("deepseek"), Some("deepseek-flash")));
    }

    #[test]
    fn languages_can_be_bounded() {
        let mut scoped = profile(vec!["*"]);
        scoped.languages = vec![SourceLanguage::Rust];
        assert!(scoped.allows_language(SourceLanguage::Rust));
        assert!(!scoped.allows_language(SourceLanguage::Python));
    }

    #[test]
    fn validation_rejects_bad_profiles() {
        let mut bad = profile(vec!["*"]);
        bad.token_budget = 0;
        assert!(bad.validate().is_err());

        let mut bad = profile(vec!["*"]);
        bad.fallback = WorkspaceIntelligenceProvider::Scopemux;
        assert!(bad.validate().is_err());

        let mut bad = profile(vec!["*"]);
        bad.name = "  ".to_string();
        assert!(bad.validate().is_err());
    }

    #[test]
    fn serde_parses_the_config_shape() {
        let parsed: WorkspaceIntelligenceProfile = serde_json::from_str(
            r#"{
                "name": "deepseek-experiment",
                "provider": "scopemux",
                "models": ["deepseek/deepseek-flash"],
                "rungs": ["W0", "W1"],
                "languages": ["rust", "python"],
                "tokenBudget": 400,
                "fallback": "generic",
                "experiment": true
            }"#,
        )
        .expect("profile should parse");
        assert_eq!(parsed.rungs.len(), 2);
        assert!(parsed.matches_model(Some("deepseek"), Some("deepseek-flash")));
    }
}
