use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{Metadata, QuestionDef, QuestionOption, Tool, ToolContext, ToolError, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExitParams {}

pub struct PlanExitTool;

const PLAN_EXIT_DESCRIPTION: &str = r#"Use this tool when you have completed the planning phase and are ready to exit plan agent.

This tool will ask the user if they want to switch to build agent to start implementing the plan.

Call this tool:
- After you have written a complete plan to the plan file
- After you have clarified any questions with the user
- When you are confident the plan is ready for implementation

Do NOT call this tool:
- Before you have created or finalized the plan
- If you still have unanswered questions about the implementation
- If the user has indicated they want to continue planning"#;

/// Create a user message and a synthetic text part via the ToolContext callbacks,
/// matching the TS `Session.updateMessage()` + `Session.updatePart()` pattern.
async fn create_user_message_with_part(
    ctx: &ToolContext,
    agent: &str,
    model: &Option<String>,
    text: &str,
) -> Result<(), ToolError> {
    let now = chrono::Utc::now().timestamp_millis();
    let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
    let part_id = format!("prt_{}", uuid::Uuid::new_v4().simple());

    // Build the MessageV2.User info matching the TS MessageInfo::User shape
    let mut user_msg = serde_json::json!({
        "id": message_id,
        "sessionID": ctx.session_id,
        "role": "user",
        "time": { "created": now },
        "agent": agent,
    });
    if let Some(ref m) = model {
        user_msg["model"] = serde_json::json!(m);
    }

    // Persist the message (mirrors TS Session.updateMessage)
    ctx.do_update_message(user_msg).await?;

    // Build the synthetic text part matching the TS MessageV2.TextPart shape
    let text_part = serde_json::json!({
        "id": part_id,
        "messageID": message_id,
        "sessionID": ctx.session_id,
        "type": "text",
        "text": text,
        "synthetic": true,
    });

    // Persist the part (mirrors TS Session.updatePart)
    ctx.do_update_part(text_part).await?;

    Ok(())
}

#[async_trait]
impl Tool for PlanExitTool {
    fn id(&self) -> &str {
        "plan_exit"
    }

    fn description(&self) -> &str {
        PLAN_EXIT_DESCRIPTION
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(
        &self,
        _args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let plan_path = get_plan_path(&ctx);
        let plan_relative = plan_path_relative(&plan_path, &ctx.worktree);

        let questions = vec![QuestionDef {
            question: format!("Plan at {} is complete. Would you like to switch to the build agent and start implementing?", plan_relative),
            header: "Build Agent".to_string(),
            options: vec![
                QuestionOption {
                    label: "Yes".to_string(),
                    description: "Switch to build agent and start implementing the plan".to_string(),
                },
                QuestionOption {
                    label: "No".to_string(),
                    description: "Stay with plan agent to continue refining the plan".to_string(),
                },
            ],
            multiple: false,
            custom: false,
        }];

        let answers = ctx.question(questions).await?;

        let answer = answers
            .first()
            .and_then(|a| a.first())
            .map(|s| s.as_str())
            .unwrap_or("No");

        if answer == "No" {
            return Err(ToolError::QuestionRejected(
                "User rejected build mode switch".to_string(),
            ));
        }

        let model = ctx.do_get_last_model().await;

        // Create a user message + synthetic part (mirrors TS Session.updateMessage + updatePart).
        // The reference reports the absolute plan path in the injected message.
        let synthetic_text = format!(
            "The plan at {} has been approved, you can now edit files. Execute the plan",
            plan_path.display()
        );
        create_user_message_with_part(&ctx, "build", &model, &synthetic_text).await?;

        ctx.do_switch_agent("build".to_string(), model.clone())
            .await?;

        let mut metadata = Metadata::new();
        metadata.insert("agent".to_string(), serde_json::json!("build"));
        metadata.insert("session_id".to_string(), serde_json::json!(ctx.session_id));
        if let Some(ref m) = model {
            metadata.insert("model".to_string(), serde_json::json!(m));
        }

        Ok(ToolResult {
            output: "User approved switching to build agent. Wait for further instructions."
                .to_string(),
            title: "Switching to build agent".to_string(),
            metadata,
            truncated: false,
        })
    }
}

/// Resolve the session's plan file. The session prompt loop sets `plan_path` from
/// the shared `opencode_core::plan_file_path` helper so this matches the path the
/// plan-mode reminder advertised. The fallback keeps the same helper (never a
/// fixed `PLAN.md`) for callers that did not attach a resolved path.
fn get_plan_path(ctx: &ToolContext) -> PathBuf {
    if let Some(path) = &ctx.plan_path {
        return PathBuf::from(path);
    }
    let worktree = PathBuf::from(&ctx.worktree);
    let data_dir = opencode_core::opencode_data_dir().unwrap_or_else(|| worktree.join(".opencode"));
    opencode_core::plan_file_path(&worktree, &data_dir, &ctx.session_id, 0)
}

/// Render the plan path relative to the worktree, matching the reference
/// `path.relative(instance.worktree, plan)` (absolute fallback when outside it).
fn plan_path_relative(plan: &Path, worktree: &str) -> String {
    match plan.strip_prefix(worktree) {
        Ok(relative) if !relative.as_os_str().is_empty() => relative.display().to_string(),
        _ => plan.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolContext;

    #[test]
    fn plan_exit_is_the_only_plan_model_tool() {
        assert_eq!(PlanExitTool.id(), "plan_exit");
    }

    #[test]
    fn relative_path_strips_worktree_prefix() {
        let rel = plan_path_relative(Path::new("/repo/.opencode/plans/1-x.md"), "/repo");
        assert_eq!(rel, ".opencode/plans/1-x.md");
    }

    #[test]
    fn relative_path_falls_back_to_absolute() {
        let rel = plan_path_relative(Path::new("/other/plans/1-x.md"), "/repo");
        assert_eq!(rel, "/other/plans/1-x.md");
    }

    #[test]
    fn get_plan_path_prefers_context_path() {
        let mut ctx = ToolContext::new("ses".into(), "msg".into(), "/repo".into());
        ctx.plan_path = Some("/repo/.opencode/plans/42-slug.md".into());
        assert_eq!(
            get_plan_path(&ctx),
            PathBuf::from("/repo/.opencode/plans/42-slug.md")
        );
    }
}
