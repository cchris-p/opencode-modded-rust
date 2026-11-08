//! Engine-neutral decision boundary for the stable core.
//!
//! This is the *only* entry point through which the runtime asks a volatile
//! policy layer to choose among known options. The core defines the typed
//! request/result contract; engines (Jev, Laya, deterministic rules, or a test
//! stub) live in policy crates and implement [`DecisionProvider`].
//!
//! Authority rule: providers **propose**, the runtime **disposes**. A
//! [`DecisionResult`] is advisory data; the caller validates it against its own
//! deterministic rules, permission authority, and invariants before acting.
//!
//! The boundary deliberately contains no engine-specific code and no
//! class-specific logic: decision classes are data-driven strings, so adding or
//! removing one is a policy change, never a core change
//! (`invariants/future-proofing.md`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

/// How sensitive the decision state is, which constrains which engines may see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Internal,
    Sensitive,
}

/// Cost and latency bounds a provider must respect for one decision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecisionBudget {
    pub max_cost_usd: f64,
    pub max_latency_ms: u64,
}

impl Default for DecisionBudget {
    fn default() -> Self {
        Self {
            max_cost_usd: 0.0,
            max_latency_ms: 0,
        }
    }
}

/// Authority the runtime currently permits for a decision class.
///
/// No class starts above [`AuthorityTier::Advisory`]; promotion is a policy
/// decision backed by evaluation evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityTier {
    /// The result may only inform the runtime; it cannot change a disposition.
    Advisory,
    /// The result may choose among pre-approved options under runtime validation.
    Gated,
    /// The result is a deterministic rule outcome owned by the runtime.
    Deterministic,
}

/// A typed question about runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    /// Data-driven class id (for example `workspace_intelligence.routing`).
    pub class: String,
    /// Serializable snapshot of the state the decision is about.
    pub state: serde_json::Value,
    /// The finite set of choices the runtime will accept.
    pub options: Vec<String>,
    /// Confidence below which the runtime must escalate or fall back.
    pub required_confidence: f32,
    /// Cost and latency bounds for serving this decision.
    pub budget: DecisionBudget,
    /// Sensitivity of the state, which limits eligible engines.
    pub sensitivity: Sensitivity,
    /// Authority the runtime permits for this class right now.
    pub authority: AuthorityTier,
    /// Policy version the caller expects the provider to report.
    pub policy_version: String,
}

/// A typed proposal. The runtime validates and disposes of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub class: String,
    /// Must be one of [`DecisionRequest::options`].
    pub choice: String,
    /// Provider-reported confidence in `0.0..=1.0`.
    pub confidence: f32,
    /// Engine that produced the result (for example `deterministic`).
    pub engine: String,
    /// Optional human-readable rationale for the audit log.
    pub rationale: Option<String>,
    /// Cost incurred by serving the decision.
    pub cost_usd: f64,
    /// Policy version the provider applied.
    pub policy_version: String,
}

/// Failure modes of the decision boundary.
#[derive(Debug, Error)]
pub enum DecisionError {
    #[error("no decision provider is registered for class `{class}`")]
    UnsupportedClass { class: String },
    #[error("provider returned `{choice}`, which is not one of the offered options")]
    InvalidChoice { choice: String },
    #[error("decision budget exceeded for class `{class}`")]
    BudgetExceeded { class: String },
    #[error("decision engine `{engine}` failed: {message}")]
    Engine { engine: String, message: String },
}

/// A replaceable decision engine.
///
/// Implementors live in policy crates. The core never names a concrete engine,
/// so swapping Jev for Laya or a stub requires no core change (swap test).
#[async_trait::async_trait]
pub trait DecisionProvider: Send + Sync {
    /// Stable engine identifier recorded in the decision log.
    fn engine(&self) -> &str;

    /// Answer the request. Returning a choice outside the offered options is an
    /// error; the runtime never trusts an unvalidated proposal.
    async fn decide(&self, request: &DecisionRequest) -> Result<DecisionResult, DecisionError>;
}

/// One deterministic rule for one decision class.
///
/// Rules are registered by policy code, so a new class needs no core change
/// (add test) and dropping a class needs no core change (delete test).
pub trait DecisionRule: Send + Sync {
    /// The class id this rule answers.
    fn class(&self) -> &str;

    /// Choose one of `request.options`. Implementations must be deterministic
    /// for a given request so the replay test can reproduce the decision.
    fn choose(&self, request: &DecisionRequest) -> Result<String, DecisionError>;
}

/// The deterministic rules rung of the degradation ladder.
///
/// Runs entirely offline and with no hosted engine, which satisfies the offline
/// test. The runtime can depend on this provider (or any other) through
/// [`DecisionProvider`] without changing the loop.
pub struct DeterministicProvider {
    rules: BTreeMap<String, Arc<dyn DecisionRule>>,
}

impl Default for DeterministicProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DeterministicProvider {
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
        }
    }

    /// Register a rule for its declared class. Replaces any existing rule for
    /// that class, which is how policy versions are rolled forward or back.
    pub fn with_rule(mut self, rule: impl DecisionRule + 'static) -> Self {
        self.rules.insert(rule.class().to_string(), Arc::new(rule));
        self
    }

    /// Remove a class at runtime. The caller keeps compiling and handling the
    /// resulting [`DecisionError::UnsupportedClass`], which is the delete test.
    pub fn without_class(mut self, class: &str) -> Self {
        self.rules.remove(class);
        self
    }

    pub fn supports(&self, class: &str) -> bool {
        self.rules.contains_key(class)
    }
}

#[async_trait::async_trait]
impl DecisionProvider for DeterministicProvider {
    fn engine(&self) -> &str {
        "deterministic"
    }

    async fn decide(&self, request: &DecisionRequest) -> Result<DecisionResult, DecisionError> {
        let rule =
            self.rules
                .get(&request.class)
                .ok_or_else(|| DecisionError::UnsupportedClass {
                    class: request.class.clone(),
                })?;

        let choice = rule.choose(request)?;
        if !request.options.iter().any(|option| option == &choice) {
            return Err(DecisionError::InvalidChoice { choice });
        }

        Ok(DecisionResult {
            class: request.class.clone(),
            choice,
            confidence: 1.0,
            engine: self.engine().to_string(),
            rationale: Some("deterministic rule".to_string()),
            cost_usd: 0.0,
            policy_version: request.policy_version.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedChoice {
        class: &'static str,
        choice: &'static str,
    }

    impl DecisionRule for FixedChoice {
        fn class(&self) -> &str {
            self.class
        }

        fn choose(&self, _request: &DecisionRequest) -> Result<String, DecisionError> {
            Ok(self.choice.to_string())
        }
    }

    struct RejectingRule {
        class: &'static str,
    }

    impl DecisionRule for RejectingRule {
        fn class(&self) -> &str {
            self.class
        }

        fn choose(&self, _request: &DecisionRequest) -> Result<String, DecisionError> {
            Ok("not-an-option".to_string())
        }
    }

    fn request(class: &str, options: &[&str]) -> DecisionRequest {
        DecisionRequest {
            class: class.to_string(),
            state: serde_json::json!({"seed": true}),
            options: options.iter().map(|option| option.to_string()).collect(),
            required_confidence: 0.5,
            budget: DecisionBudget::default(),
            sensitivity: Sensitivity::Internal,
            authority: AuthorityTier::Advisory,
            policy_version: "test-policy-1".to_string(),
        }
    }

    /// A fake class registered from outside the core, proving the add test.
    #[tokio::test]
    async fn deterministic_stub_answers_a_fake_class_without_a_core_change() {
        let provider = DeterministicProvider::new().with_rule(FixedChoice {
            class: "fake.test.class",
            choice: "yes",
        });

        let result = provider
            .decide(&request("fake.test.class", &["yes", "no"]))
            .await
            .expect("registered class should be answered");

        assert_eq!(result.choice, "yes");
        assert_eq!(result.engine, "deterministic");
        assert_eq!(result.confidence, 1.0);
        assert_eq!(result.policy_version, "test-policy-1");
        assert_eq!(result.cost_usd, 0.0);
    }

    /// Dropping a registration is the delete test: the caller still compiles and
    /// handles a typed error instead of the runtime breaking.
    #[tokio::test]
    async fn removing_a_class_degrades_to_a_typed_unsupported_error() {
        let provider = DeterministicProvider::new()
            .with_rule(FixedChoice {
                class: "fake.test.class",
                choice: "yes",
            })
            .without_class("fake.test.class");

        let error = provider
            .decide(&request("fake.test.class", &["yes", "no"]))
            .await
            .expect_err("removed class must not be answered");

        assert!(matches!(error, DecisionError::UnsupportedClass { .. }));
    }

    /// A second engine, standing in for Jev/Laya, that speaks the same boundary.
    struct FixedProvider {
        engine: &'static str,
        choice: &'static str,
    }

    #[async_trait::async_trait]
    impl DecisionProvider for FixedProvider {
        fn engine(&self) -> &str {
            self.engine
        }

        async fn decide(&self, request: &DecisionRequest) -> Result<DecisionResult, DecisionError> {
            Ok(DecisionResult {
                class: request.class.clone(),
                choice: self.choice.to_string(),
                confidence: 0.9,
                engine: self.engine.to_string(),
                rationale: None,
                cost_usd: 0.001,
                policy_version: request.policy_version.clone(),
            })
        }
    }

    /// The loop only sees the trait, so any provider can be swapped in (swap test).
    #[tokio::test]
    async fn a_caller_swaps_providers_without_changing_its_code() {
        async fn answer(provider: &dyn DecisionProvider, request: &DecisionRequest) -> String {
            provider
                .decide(request)
                .await
                .expect("decision should succeed")
                .choice
        }

        let request = request("fake.test.class", &["alpha", "beta"]);
        let deterministic = DeterministicProvider::new().with_rule(FixedChoice {
            class: "fake.test.class",
            choice: "alpha",
        });
        let hosted = FixedProvider {
            engine: "hosted-test-engine",
            choice: "beta",
        };

        assert_eq!(answer(&deterministic, &request).await, "alpha");
        assert_eq!(answer(&hosted, &request).await, "beta");
        assert_eq!(deterministic.engine(), "deterministic");
        assert_eq!(hosted.engine(), "hosted-test-engine");
    }

    #[tokio::test]
    async fn an_out_of_band_choice_is_rejected() {
        let provider = DeterministicProvider::new().with_rule(RejectingRule {
            class: "fake.test.class",
        });

        let error = provider
            .decide(&request("fake.test.class", &["yes", "no"]))
            .await
            .expect_err("a choice outside the options must not be accepted");

        assert!(matches!(error, DecisionError::InvalidChoice { .. }));
    }
}
