use opencode_agent::{AgentInfo, AgentRegistry, PermissionDecision};
use opencode_config::load_config;
use opencode_permission::{
    evaluate as evaluate_permission, PermissionAction, PermissionRule, PermissionRuleset,
};
use opencode_provider::ToolDefinition;
use opencode_session::system::{EnvironmentContext, SystemPrompt};
use opencode_session::AgentParams;
use opencode_tool::create_default_registry;
use std::path::Path;

/// Resolution outcome for an agentic coding-session request.
///
/// This is the server-side analogue of the reference session's per-request
/// agent/system/tools resolution: given the session directory, the requested
/// agent (or the configured default, or `build`), and the active model, it
/// produces the agent prompt plus environment block, the permission-filtered
/// tool set, and the agent LLM parameters that a bare chat request was missing.
#[derive(Debug, Clone)]
pub struct ResolvedAgenticContext {
    pub agent_name: String,
    pub agent: AgentInfo,
    pub system_prompt: String,
    pub tools: Vec<ToolDefinition>,
    pub params: AgentParams,
}

/// Resolve the agentic context for a coding-session prompt.
///
/// - `directory`: the session working directory used to load project config.
/// - `requested_agent`: the agent name sent with the prompt, if any.
/// - `model_api_id`: the model id used to select the model system prompt.
/// - `provider_id`: the provider id used for the environment block.
/// - `supports_tools`: whether the active model advertises tool calling.
///
/// Mirrors the reference ordering: the agent prompt (or the model default when
/// the agent defines none) is followed by the environment/workspace block, and
/// tools are resolved from the default registry filtered by agent permission.
pub async fn resolve_agentic_context(
    directory: &str,
    requested_agent: Option<String>,
    model_api_id: &str,
    provider_id: &str,
    supports_tools: bool,
) -> ResolvedAgenticContext {
    let registry = build_agent_registry(directory);
    let agent_name = resolve_agent_name(&registry, requested_agent);
    let agent = registry
        .get(&agent_name)
        .cloned()
        .unwrap_or_else(|| AgentInfo::build());

    let system_prompt = build_system_prompt(&agent, model_api_id, provider_id, directory);
    let tools = if supports_tools {
        resolve_tools(&agent).await
    } else {
        Vec::new()
    };

    let params = AgentParams {
        max_tokens: agent.max_tokens,
        temperature: agent.temperature,
        top_p: agent.top_p,
    };

    ResolvedAgenticContext {
        agent_name,
        agent,
        system_prompt,
        tools,
        params,
    }
}

/// Load project config and build the agent registry (defaults to all builtins
/// when no config is present), matching the reference default-agent selection.
pub fn build_agent_registry(directory: &str) -> AgentRegistry {
    let config = load_config(Path::new(directory)).ok();
    AgentRegistry::from_optional_config(config.as_ref())
}

/// Resolve the active agent name: the requested agent, else `config.default_agent`,
/// else `build`. Falls back to `build` when the chosen agent does not exist in the
/// registry so a stale request still yields a usable agentic session.
pub fn resolve_agent_name(registry: &AgentRegistry, requested_agent: Option<String>) -> String {
    if let Some(name) = requested_agent {
        if registry.get(&name).is_some() {
            return name;
        }
        return "build".to_string();
    }

    if registry.get("build").is_some() {
        return "build".to_string();
    }

    let agent = registry.default_agent();
    agent.name.clone()
}

/// Assemble the system prompt in reference order: agent prompt (or the model
/// default prompt when the agent defines none), then the environment/workspace
/// block with the working directory, workspace root, git state, platform, and date.
pub fn build_system_prompt(
    agent: &AgentInfo,
    model_api_id: &str,
    provider_id: &str,
    directory: &str,
) -> String {
    let base = agent
        .system_prompt
        .clone()
        .unwrap_or_else(|| SystemPrompt::for_model(model_api_id).to_string());

    let env_ctx = EnvironmentContext::from_current(model_api_id, provider_id, directory);
    let env_block = SystemPrompt::environment(&env_ctx);

    format!("{}\n\n{}", base, env_block)
}

/// Resolve the permission-filtered tool set from the default registry.
///
/// Tools whose agent permission decision is `Deny` are excluded; `Allow` and
/// `Ask` tools are declared so the model can call them and the ask callback can
/// gate execution. This mirrors the reference `SessionTools.resolve` behavior of
/// attaching every tool the agent may use.
pub async fn resolve_tools(agent: &AgentInfo) -> Vec<ToolDefinition> {
    let registry = create_default_registry().await;
    let schemas = registry.list_schemas().await;
    schemas
        .into_iter()
        .filter(|schema| {
            schema.name != "invalid"
                && !matches!(
                    agent.tool_permission_decision(&schema.name),
                    PermissionDecision::Deny
                )
        })
        .map(|schema| ToolDefinition {
            name: schema.name,
            description: Some(schema.description),
            parameters: schema.parameters,
        })
        .collect()
}

/// Evaluate a tool-execution permission request against the agent ruleset.
///
/// Returns `true` when the request is explicitly allowed and `false` when it must
/// go to the ask UI (Ask) or is denied (Deny). This is used by the server ask
/// callback so `Allow` tool calls run silently instead of prompting on every
/// invocation, matching the reference permission evaluation.
pub fn is_permission_allowed(ruleset: &PermissionRuleset, permission: &str, pattern: &str) -> bool {
    matches!(
        evaluate_permission(permission, pattern, &[ruleset.clone()]).action,
        PermissionAction::Allow
    )
}

/// Classify a permission request against the agent ruleset.
///
/// Mirrors the reference decision model: `Allow` runs silently, `Deny` is an
/// error, and `Ask` goes to the user. When multiple patterns are supplied, the
/// strictest matching action wins (Deny > Ask > Allow) so one denied pattern is
/// never auto-allowed by a permissive sibling.
pub fn classify_permission(
    ruleset: &PermissionRuleset,
    permission: &str,
    patterns: &[String],
) -> PermissionDecision {
    let patterns: Vec<String> = if patterns.is_empty() {
        vec![permission.to_string()]
    } else {
        patterns.to_vec()
    };

    let mut worst: Option<PermissionDecision> = None;
    let rank = |d: PermissionDecision| match d {
        PermissionDecision::Deny => 2,
        PermissionDecision::Ask => 1,
        PermissionDecision::Allow => 0,
    };

    for pattern in &patterns {
        let rule = evaluate_permission(permission, pattern, &[ruleset.clone()]);
        let decision = permission_decision_from_rule(rule);
        if worst.is_none() || rank(decision) > rank(worst.unwrap()) {
            worst = Some(decision);
        }
    }

    worst.unwrap_or(PermissionDecision::Allow)
}

fn permission_decision_from_rule(rule: PermissionRule) -> PermissionDecision {
    match rule.action {
        PermissionAction::Allow => PermissionDecision::Allow,
        PermissionAction::Ask => PermissionDecision::Ask,
        PermissionAction::Deny => PermissionDecision::Deny,
    }
}

/// Convenience: resolve the merged ruleset for a session ask callback from an
/// agent ruleset plus an optional session-level allow/deny overlay.
///
/// Session overlays are appended after the agent ruleset so, like the reference
/// merge order, session rules win over agent defaults.
pub fn merged_ruleset(
    agent_rules: &PermissionRuleset,
    session_rules: &PermissionRuleset,
) -> PermissionRuleset {
    let mut merged = agent_rules.clone();
    merged.extend(session_rules.clone());
    merged
}

/// Convert a tool's `ask`/`deny` style overlay rules into a `PermissionRuleset`.
pub fn ruleset_from_session(session: &opencode_session::PermissionRuleset) -> PermissionRuleset {
    let mut rules: PermissionRuleset = Vec::new();
    for name in &session.allow {
        rules.push(PermissionRule {
            permission: name.clone(),
            pattern: "*".to_string(),
            action: PermissionAction::Allow,
        });
    }
    for name in &session.deny {
        rules.push(PermissionRule {
            permission: name.clone(),
            pattern: "*".to_string(),
            action: PermissionAction::Deny,
        });
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_agent_name_defaults_to_build() {
        let registry = AgentRegistry::new();
        assert_eq!(resolve_agent_name(&registry, None), "build");
        assert_eq!(
            resolve_agent_name(&registry, Some("build".to_string())),
            "build"
        );
        assert_eq!(
            resolve_agent_name(&registry, Some("explore".to_string())),
            "explore"
        );
        assert_eq!(
            resolve_agent_name(&registry, Some("does-not-exist".to_string())),
            "build"
        );
    }

    #[test]
    fn build_system_prompt_includes_model_prompt_env_and_directory() {
        let agent = AgentInfo::build();
        let prompt = build_system_prompt(
            &agent,
            "claude-sonnet-4-20250514",
            "anthropic",
            "/tmp/test-proj",
        );
        assert!(prompt.contains("OpenCode"));
        assert!(prompt.contains("Working directory: /tmp/test-proj"));
        assert!(prompt.contains("anthropic/claude-sonnet-4-20250514"));
    }

    #[test]
    fn build_system_prompt_uses_agent_prompt_when_set() {
        let agent = AgentInfo::build().with_system_prompt("Custom agent prompt body");
        let prompt = build_system_prompt(
            &agent,
            "claude-sonnet-4-20250514",
            "anthropic",
            "/tmp/test-proj",
        );
        assert!(prompt.starts_with("Custom agent prompt body"));
        assert!(prompt.contains("Working directory: /tmp/test-proj"));
    }

    #[tokio::test]
    async fn resolve_tools_filters_denied_and_invalid_tools() {
        let agent = AgentInfo::build();
        let tools = resolve_tools(&agent).await;
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read"), "read tool should be attached");
        assert!(names.contains(&"bash"), "bash tool should be attached");
        assert!(
            !names.contains(&"invalid"),
            "internal invalid tool should not be attached"
        );
        assert!(
            !names.contains(&"plan_exit"),
            "denied plan_exit tool should not be attached for build"
        );
    }

    #[test]
    fn classify_permission_respects_allow_and_deny() {
        let allow: PermissionRuleset = vec![PermissionRule {
            permission: "read".into(),
            pattern: "*".into(),
            action: PermissionAction::Allow,
        }];
        assert_eq!(
            classify_permission(&allow, "read", &[]),
            PermissionDecision::Allow
        );

        let deny: PermissionRuleset = vec![
            PermissionRule {
                permission: "*".into(),
                pattern: "*".into(),
                action: PermissionAction::Allow,
            },
            PermissionRule {
                permission: "read".into(),
                pattern: "*.env".into(),
                action: PermissionAction::Deny,
            },
        ];
        assert_eq!(
            classify_permission(&deny, "read", &["/proj/.env".to_string()]),
            PermissionDecision::Deny
        );
        assert_eq!(
            classify_permission(&deny, "read", &["/proj/src/lib.rs".to_string()]),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn merged_ruleset_appends_session_overlay() {
        let agent: PermissionRuleset = vec![PermissionRule {
            permission: "*".into(),
            pattern: "*".into(),
            action: PermissionAction::Allow,
        }];
        let session = opencode_session::PermissionRuleset {
            allow: vec![],
            deny: vec!["bash".to_string()],
            mode: None,
        };
        let merged = merged_ruleset(&agent, &ruleset_from_session(&session));
        assert_eq!(
            classify_permission(&merged, "bash", &[]),
            PermissionDecision::Deny
        );
    }
}
