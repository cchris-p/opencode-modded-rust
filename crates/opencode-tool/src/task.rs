use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    Metadata, PermissionRequest, ResolvedSubagent, Tool, ToolContext, ToolError, ToolResult,
};

pub struct TaskTool;

impl TaskTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskInput {
    description: String,
    prompt: String,
    subagent_type: String,
    task_id: Option<String>,
    command: Option<String>,
    #[serde(default)]
    background: bool,
}

/// Environment gate matching the reference `experimentalBackgroundSubagents`.
const BACKGROUND_FLAG_ENV: &str = "OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS";

fn background_enabled() -> bool {
    std::env::var(BACKGROUND_FLAG_ENV)
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false)
}

/// Reference `<task id="..." state="...">` wrapper, including the optional
/// `<summary>` and a `<task_result>`/`<task_error>` body.
fn render_task_output(session_id: &str, state: &str, summary: Option<&str>, text: &str) -> String {
    let tag = if state == "error" {
        "task_error"
    } else {
        "task_result"
    };
    let mut output = format!("<task id=\"{}\" state=\"{}\">", session_id, state);
    if let Some(summary) = summary {
        output.push_str(&format!("\n<summary>{}</summary>", summary));
    }
    output.push_str(&format!("\n<{}>\n{}\n</{}>", tag, text, tag));
    output.push_str("\n</task>");
    output
}

/// Default subagent tool denies: `todowrite` and `task` are denied unless the
/// subagent's own ruleset already declares a rule for them. Mirrors the
/// reference `childToolDenies` derivation in `tool/task.ts`.
fn derive_disabled_tools(resolved: &ResolvedSubagent) -> Vec<String> {
    let mut disabled = Vec::new();
    if !resolved.permits_todowrite {
        disabled.push("todowrite".to_string());
    }
    if !resolved.permits_task {
        disabled.push("task".to_string());
    }
    disabled
}

#[async_trait]
impl Tool for TaskTool {
    fn id(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Launch a specialized subagent to handle a complex task. Use this to delegate tasks that require specialized expertise or multi-step reasoning."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short (3-5 words) description of the task"
                },
                "prompt": {
                    "type": "string",
                    "description": "The task for the agent to perform"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "The type of specialized agent to use for this task"
                },
                "task_id": {
                    "type": "string",
                    "description": "This should only be set if you mean to resume a previous task (you can pass a prior task_id and the task will continue the same subagent session as before instead of creating a fresh one)"
                },
                "command": {
                    "type": "string",
                    "description": "The command that triggered this task"
                },
                "background": {
                    "type": "boolean",
                    "description": "Run the agent in the background. You will be notified when it completes. DO NOT sleep, poll, or proactively check on its progress"
                }
            },
            "required": ["description", "prompt", "subagent_type"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: TaskInput =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        // Background subagents are experimental in the reference and owned by
        // FEAT-048. Until that lands, `background: true` must fail the same way
        // the reference does when the experimental gate is off.
        if input.background {
            if !background_enabled() {
                return Err(ToolError::ExecutionError(
                    "Background subagents require OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true"
                        .to_string(),
                ));
            }
            return Err(ToolError::ExecutionError(
                "Background subagents are not implemented yet (tracked by FEAT-048)".to_string(),
            ));
        }

        let bypass_check = ctx
            .extra
            .get("bypassAgentCheck")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !bypass_check {
            ctx.ask_permission(
                PermissionRequest::new("task")
                    .with_pattern(&input.subagent_type)
                    .with_metadata("description", serde_json::json!(&input.description))
                    .with_metadata("subagent_type", serde_json::json!(&input.subagent_type))
                    .always_allow(),
            )
            .await?;
        }

        // Registry-driven lookup. Unknown/non-subagent types (and the depth
        // limit) surface here as an error before any session is created.
        let resolved = ctx.do_resolve_subagent(input.subagent_type.clone()).await?;
        let disabled_tools = derive_disabled_tools(&resolved);

        // The subagent's configured model wins over the parent message model.
        let preferred_model = match resolved.model.clone() {
            Some(model) => Some(model),
            None => ctx.do_get_last_model().await,
        };

        let session_id = if let Some(task_id) = &input.task_id {
            task_id.clone()
        } else {
            ctx.do_create_subsession(
                resolved.name.clone(),
                Some(input.description.clone()),
                preferred_model.clone(),
                disabled_tools,
            )
            .await?
        };

        let result_text = ctx
            .do_prompt_subsession(session_id.clone(), input.prompt.clone())
            .await?;
        let model = parse_model_ref(preferred_model.as_deref());

        let output = render_task_output(&session_id, "completed", None, &result_text);

        let mut metadata = Metadata::new();
        metadata.insert("parentSessionId".into(), serde_json::json!(ctx.session_id));
        metadata.insert("sessionId".into(), serde_json::json!(session_id));
        metadata.insert(
            "model".into(),
            serde_json::json!({
                "modelID": model.model_id,
                "providerID": model.provider_id,
            }),
        );

        Ok(ToolResult {
            title: input.description.clone(),
            output,
            metadata,
            truncated: false,
        })
    }
}

struct AgentModel {
    model_id: String,
    provider_id: String,
}

fn parse_model_ref(raw: Option<&str>) -> AgentModel {
    let Some(raw) = raw else {
        return AgentModel {
            model_id: "default".to_string(),
            provider_id: "default".to_string(),
        };
    };

    let pair = raw.split_once(':').or_else(|| raw.split_once('/'));
    if let Some((provider, model)) = pair {
        return AgentModel {
            model_id: model.to_string(),
            provider_id: provider.to_string(),
        };
    }

    AgentModel {
        model_id: raw.to_string(),
        provider_id: "default".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn resolved(
        name: &str,
        model: Option<&str>,
        permits_task: bool,
        permits_todowrite: bool,
    ) -> ResolvedSubagent {
        ResolvedSubagent {
            name: name.to_string(),
            model: model.map(str::to_string),
            permits_task,
            permits_todowrite,
        }
    }

    #[tokio::test]
    async fn task_resolves_agent_creates_child_and_wraps_output() {
        let create_calls = Arc::new(Mutex::new(Vec::<(
            String,
            Option<String>,
            Option<String>,
            Vec<String>,
        )>::new()));
        let prompt_calls = Arc::new(Mutex::new(Vec::<(String, String)>::new()));

        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into())
            .with_resolve_subagent(|name| async move {
                Ok(resolved(&name, Some("provider-x:model-y"), false, false))
            })
            .with_get_last_model(|_session_id| async move { Ok(Some("parent:p".into())) })
            .with_create_subsession({
                let create_calls = create_calls.clone();
                move |agent, title, model, disabled_tools| {
                    let create_calls = create_calls.clone();
                    async move {
                        create_calls
                            .lock()
                            .await
                            .push((agent, title, model, disabled_tools));
                        Ok("ses_child_123".to_string())
                    }
                }
            })
            .with_prompt_subsession({
                let prompt_calls = prompt_calls.clone();
                move |session_id, prompt| {
                    let prompt_calls = prompt_calls.clone();
                    async move {
                        prompt_calls.lock().await.push((session_id, prompt));
                        Ok("subagent output".to_string())
                    }
                }
            });

        let args = serde_json::json!({
            "description": "Investigate issue",
            "prompt": "Please inspect runtime behavior",
            "subagent_type": "explore"
        });

        let result = TaskTool::new().execute(args, ctx).await.unwrap();

        assert_eq!(result.title, "Investigate issue");
        assert_eq!(
            result.output,
            "<task id=\"ses_child_123\" state=\"completed\">\n<task_result>\nsubagent output\n</task_result>\n</task>"
        );
        assert_eq!(
            result.metadata.get("sessionId"),
            Some(&serde_json::json!("ses_child_123"))
        );
        assert_eq!(
            result.metadata.get("parentSessionId"),
            Some(&serde_json::json!("session-1"))
        );
        assert_eq!(
            result.metadata.get("model"),
            Some(&serde_json::json!({
                "modelID": "model-y",
                "providerID": "provider-x"
            }))
        );

        let create_calls = create_calls.lock().await.clone();
        assert_eq!(create_calls.len(), 1);
        assert_eq!(create_calls[0].0, "explore");
        assert_eq!(create_calls[0].1, Some("Investigate issue".to_string()));
        assert_eq!(create_calls[0].2, Some("provider-x:model-y".to_string()));
        assert_eq!(
            create_calls[0].3,
            vec!["todowrite".to_string(), "task".to_string()]
        );

        let prompt_calls = prompt_calls.lock().await.clone();
        assert_eq!(prompt_calls.len(), 1);
        assert_eq!(prompt_calls[0].0, "ses_child_123");
        assert_eq!(prompt_calls[0].1, "Please inspect runtime behavior");
    }

    #[tokio::test]
    async fn subagent_model_falls_back_to_parent_when_unset() {
        let captured_model = Arc::new(Mutex::new(None::<Option<String>>));

        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into())
            .with_resolve_subagent(|name| async move { Ok(resolved(&name, None, false, false)) })
            .with_get_last_model(|_session_id| async move { Ok(Some("parent:p".into())) })
            .with_create_subsession({
                let captured_model = captured_model.clone();
                move |_agent, _title, model, _disabled| {
                    let captured_model = captured_model.clone();
                    async move {
                        *captured_model.lock().await = Some(model);
                        Ok("ses_child".to_string())
                    }
                }
            })
            .with_prompt_subsession(|_id, _prompt| async move { Ok("ok".to_string()) });

        let args = serde_json::json!({
            "description": "Investigate",
            "prompt": "go",
            "subagent_type": "explore"
        });

        TaskTool::new().execute(args, ctx).await.unwrap();

        assert_eq!(
            captured_model.lock().await.clone(),
            Some(Some("parent:p".to_string()))
        );
    }

    #[tokio::test]
    async fn subagent_that_permits_task_and_todowrite_is_not_denied_them() {
        let captured_disabled = Arc::new(Mutex::new(Vec::<String>::new()));

        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into())
            .with_resolve_subagent(|name| async move { Ok(resolved(&name, None, true, true)) })
            .with_create_subsession({
                let captured_disabled = captured_disabled.clone();
                move |_agent, _title, _model, disabled| {
                    let captured_disabled = captured_disabled.clone();
                    async move {
                        *captured_disabled.lock().await = disabled;
                        Ok("ses_child".to_string())
                    }
                }
            })
            .with_prompt_subsession(|_id, _prompt| async move { Ok("ok".to_string()) });

        let args = serde_json::json!({
            "description": "Investigate",
            "prompt": "go",
            "subagent_type": "custom"
        });

        TaskTool::new().execute(args, ctx).await.unwrap();

        assert!(captured_disabled.lock().await.is_empty());
    }

    #[tokio::test]
    async fn unknown_subagent_errors_without_creating_a_session() {
        let created = Arc::new(Mutex::new(false));

        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into())
            .with_resolve_subagent(|name| async move {
                Err(ToolError::ExecutionError(format!(
                    "Unknown agent type: {} is not a valid agent type",
                    name
                )))
            })
            .with_create_subsession({
                let created = created.clone();
                move |_agent, _title, _model, _disabled| {
                    let created = created.clone();
                    async move {
                        *created.lock().await = true;
                        Ok("should_not_be_used".to_string())
                    }
                }
            });

        let args = serde_json::json!({
            "description": "Investigate",
            "prompt": "go",
            "subagent_type": "nope"
        });

        let error = TaskTool::new()
            .execute(args, ctx)
            .await
            .expect_err("unknown agent must error");
        assert!(error
            .to_string()
            .contains("Unknown agent type: nope is not a valid agent type"));
        assert!(!*created.lock().await, "no session should be created");
    }

    #[tokio::test]
    async fn depth_limit_error_propagates_from_resolver() {
        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into())
            .with_resolve_subagent(|_name| async move {
                Err(ToolError::ExecutionError(
                    "Subagent depth limit reached (1). Increase \"subagent_depth\" to allow nested subagents."
                        .to_string(),
                ))
            });

        let args = serde_json::json!({
            "description": "Nested",
            "prompt": "go",
            "subagent_type": "explore"
        });

        let error = TaskTool::new()
            .execute(args, ctx)
            .await
            .expect_err("depth limit must error");
        assert!(error
            .to_string()
            .contains("Subagent depth limit reached (1)"));
    }

    #[tokio::test]
    async fn task_reuses_existing_task_id_without_creating_subsession() {
        let created = Arc::new(Mutex::new(false));
        let prompted = Arc::new(Mutex::new(Vec::<(String, String)>::new()));

        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into())
            .with_resolve_subagent(|name| async move { Ok(resolved(&name, None, false, false)) })
            .with_get_last_model(|_session_id| async move { Ok(Some("provider-x:model-y".into())) })
            .with_create_subsession({
                let created = created.clone();
                move |_agent, _title, _model, _disabled| {
                    let created = created.clone();
                    async move {
                        *created.lock().await = true;
                        Ok("should_not_be_used".to_string())
                    }
                }
            })
            .with_prompt_subsession({
                let prompted = prompted.clone();
                move |session_id, prompt| {
                    let prompted = prompted.clone();
                    async move {
                        prompted.lock().await.push((session_id, prompt));
                        Ok("continued output".to_string())
                    }
                }
            });

        let args = serde_json::json!({
            "description": "Continue task",
            "prompt": "Continue where you left off",
            "subagent_type": "explore",
            "task_id": "ses_existing_42"
        });

        let result = TaskTool::new().execute(args, ctx).await.unwrap();

        assert!(!*created.lock().await);
        let prompted = prompted.lock().await.clone();
        assert_eq!(prompted.len(), 1);
        assert_eq!(prompted[0].0, "ses_existing_42");
        assert_eq!(prompted[0].1, "Continue where you left off");
        assert!(result
            .output
            .contains("<task id=\"ses_existing_42\" state=\"completed\">"));
    }

    /// The background gate is process-global, so this test alone mutates the
    /// env var. Keep it in one test to avoid cross-test interference.
    #[tokio::test]
    async fn background_requires_the_experimental_gate() {
        let previous = std::env::var(BACKGROUND_FLAG_ENV).ok();
        std::env::remove_var(BACKGROUND_FLAG_ENV);

        let ctx = ToolContext::new("session-1".into(), "message-1".into(), ".".into());
        let args = serde_json::json!({
            "description": "Background",
            "prompt": "go",
            "subagent_type": "explore",
            "background": true
        });

        let error = TaskTool::new()
            .execute(args, ctx)
            .await
            .expect_err("background must require the gate");
        assert!(error.to_string().contains(
            "Background subagents require OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true"
        ));

        if let Some(previous) = previous {
            std::env::set_var(BACKGROUND_FLAG_ENV, previous);
        }
    }

    #[test]
    fn render_task_output_matches_reference_for_completed_and_error() {
        assert_eq!(
            render_task_output("ses_1", "completed", None, "hi"),
            "<task id=\"ses_1\" state=\"completed\">\n<task_result>\nhi\n</task_result>\n</task>"
        );
        assert_eq!(
            render_task_output("ses_1", "error", Some("boom"), "bad"),
            "<task id=\"ses_1\" state=\"error\">\n<summary>boom</summary>\n<task_error>\nbad\n</task_error>\n</task>"
        );
        assert_eq!(
            render_task_output("ses_1", "running", None, "wait"),
            "<task id=\"ses_1\" state=\"running\">\n<task_result>\nwait\n</task_result>\n</task>"
        );
    }
}
