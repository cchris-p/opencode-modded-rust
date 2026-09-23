use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    SessionInspectRequest, SessionInspectResponse, SessionSummaryData, SessionTranscriptData, Tool,
    ToolContext, ToolError, ToolResult,
};

pub const SESSION_LIST_DEFAULT_LIMIT: usize = 20;
pub const SESSION_LIST_MAX_LIMIT: usize = 100;
pub const SESSION_READ_DEFAULT_LIMIT: usize = 40;
pub const SESSION_READ_MAX_LIMIT: usize = 200;

/// Read-only inspection of other sessions in the same workspace.
///
/// The tool never resumes, mutates, or imports another session's transcript
/// into the current context; it only lists sessions or returns a bounded,
/// paginated slice of one transcript on demand.
pub struct SessionTool;

#[derive(Debug, Deserialize)]
struct SessionToolInput {
    action: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[async_trait]
impl Tool for SessionTool {
    fn id(&self) -> &str {
        "session"
    }

    fn description(&self) -> &str {
        "Inspect sessions in the current workspace, read-only. Use action \"list\" to enumerate \
         the sessions available in this workspace (optionally filtered by `query`), or action \
         \"read\" with `id` to read a bounded slice of one session's transcript. Reading never \
         resumes or modifies the target session, and results are paginated with `limit`/`offset`."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "read"],
                    "description": "List workspace sessions, or read one session's transcript"
                },
                "id": {
                    "type": "string",
                    "description": "Target session id, slug, or title (required for action \"read\")"
                },
                "query": {
                    "type": "string",
                    "description": "Optional case-insensitive substring filter for action \"list\""
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Max sessions (list, default 20) or messages (read, default 40, max 200)"
                },
                "offset": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Message offset for action \"read\" (default 0)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: SessionToolInput =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        match input.action.as_str() {
            "list" => {
                let limit = input
                    .limit
                    .unwrap_or(SESSION_LIST_DEFAULT_LIMIT)
                    .clamp(1, SESSION_LIST_MAX_LIMIT);
                let query = input.query.filter(|query| !query.trim().is_empty());
                let response = ctx
                    .do_session_inspect(SessionInspectRequest::List { query, limit })
                    .await?;
                match response {
                    SessionInspectResponse::List { sessions, total } => {
                        Ok(format_session_list(sessions, total))
                    }
                    _ => Err(ToolError::ExecutionError(
                        "Session list returned an unexpected response".to_string(),
                    )),
                }
            }
            "read" => {
                let id = input.id.filter(|id| !id.trim().is_empty()).ok_or_else(|| {
                    ToolError::InvalidArguments("`id` is required for action \"read\"".to_string())
                })?;
                let limit = input
                    .limit
                    .unwrap_or(SESSION_READ_DEFAULT_LIMIT)
                    .clamp(1, SESSION_READ_MAX_LIMIT);
                let offset = input.offset.unwrap_or(0);
                let response = ctx
                    .do_session_inspect(SessionInspectRequest::Read { id, limit, offset })
                    .await?;
                match response {
                    SessionInspectResponse::Read(data) => Ok(format_session_transcript(data)),
                    _ => Err(ToolError::ExecutionError(
                        "Session read returned an unexpected response".to_string(),
                    )),
                }
            }
            other => Err(ToolError::InvalidArguments(format!(
                "Unknown action `{}`; expected \"list\" or \"read\"",
                other
            ))),
        }
    }
}

fn format_session_list(sessions: Vec<SessionSummaryData>, total: usize) -> ToolResult {
    let mut output = String::new();
    output.push_str("# Sessions (");
    output.push_str(&sessions.len().to_string());
    output.push_str(" of ");
    output.push_str(&total.to_string());
    output.push_str(")\n\n");

    if sessions.is_empty() {
        output.push_str("No sessions are available in this workspace.\n");
    } else {
        for session in &sessions {
            let title = if session.title.trim().is_empty() {
                "(untitled)"
            } else {
                session.title.as_str()
            };
            output.push_str(&format!(
                "- {} · {} [{}] — {}\n",
                session.id, title, session.status, session.directory
            ));
        }
    }

    let truncated = sessions.len() < total;
    if truncated {
        output.push_str(&format!(
            "\n{} more session(s) not shown; narrow with `query` or raise `limit`.\n",
            total - sessions.len()
        ));
    }

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("sessions".to_string(), serde_json::json!(sessions));
    metadata.insert("total".to_string(), serde_json::json!(total));

    ToolResult {
        title: format!("Sessions ({} of {})", sessions.len(), total),
        output,
        metadata,
        truncated,
    }
}

fn format_session_transcript(data: SessionTranscriptData) -> ToolResult {
    let title = if data.title.trim().is_empty() {
        "(untitled)"
    } else {
        data.title.as_str()
    };

    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", title));
    output.push_str(&format!("- Session: {}\n", data.id));
    output.push_str(&format!("- Status: {}\n", data.status));
    output.push_str(&format!("- Workspace: {}\n", data.directory));
    output.push_str(&format!(
        "- Messages: {} of {} (offset {})\n\n",
        data.returned, data.total, data.offset
    ));

    if data.messages.is_empty() {
        output.push_str("No messages in this range.\n");
    }

    for message in &data.messages {
        output.push_str(&format!(
            "## {} — {}\n",
            title_case(&message.role),
            format_time(message.created)
        ));
        if message.previews.is_empty() {
            output.push_str("(no content)\n\n");
        } else {
            for preview in &message.previews {
                output.push_str(preview);
                output.push('\n');
            }
            output.push('\n');
        }
    }

    let next_offset = data.offset + data.returned;
    let truncated = next_offset < data.total;
    if truncated {
        output.push_str(&format!(
            "Truncated: continue with action \"read\", id \"{}\", offset {}. \
             The target session was not modified.\n",
            data.id, next_offset
        ));
    }

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("session_id".to_string(), serde_json::json!(&data.id));
    metadata.insert("total".to_string(), serde_json::json!(data.total));
    metadata.insert("offset".to_string(), serde_json::json!(data.offset));
    metadata.insert("returned".to_string(), serde_json::json!(data.returned));

    ToolResult {
        title: format!("Session {} ({} of {})", data.id, data.returned, data.total),
        output,
        metadata,
        truncated,
    }
}

fn title_case(role: &str) -> String {
    let mut chars = role.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn format_time(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ms.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SessionTranscriptData, SessionTranscriptMessageData};

    fn ctx_with(
        response: impl Fn(SessionInspectRequest) -> SessionInspectResponse + Send + Sync + 'static,
    ) -> ToolContext {
        ToolContext::new(
            "ses_current".to_string(),
            "msg_current".to_string(),
            "/tmp/ws".to_string(),
        )
        .with_session_inspect(move |request| {
            let response = response(request);
            async move { Ok(response) }
        })
    }

    #[tokio::test]
    async fn list_formats_sessions() {
        let ctx = ctx_with(|request| {
            assert!(matches!(
                request,
                SessionInspectRequest::List { limit: 5, .. }
            ));
            SessionInspectResponse::List {
                sessions: vec![SessionSummaryData {
                    id: "ses_a".to_string(),
                    title: "Earlier bug hunt".to_string(),
                    status: "active".to_string(),
                    updated: 1_700_000_000_000,
                    directory: "/tmp/ws".to_string(),
                }],
                total: 3,
            }
        });

        let result = SessionTool
            .execute(serde_json::json!({"action": "list", "limit": 5}), ctx)
            .await
            .expect("list should succeed");

        assert!(result.output.contains("ses_a"));
        assert!(result.output.contains("Earlier bug hunt"));
        assert!(result.truncated);
        assert_eq!(result.metadata.get("total").unwrap(), &serde_json::json!(3));
    }

    #[tokio::test]
    async fn read_reports_pagination_hint() {
        let ctx = ctx_with(|request| match request {
            SessionInspectRequest::Read { id, limit, offset } => {
                assert_eq!(id, "ses_a");
                assert_eq!(limit, 2);
                assert_eq!(offset, 0);
                SessionInspectResponse::Read(SessionTranscriptData {
                    id: "ses_a".to_string(),
                    title: "Earlier bug hunt".to_string(),
                    status: "completed".to_string(),
                    directory: "/tmp/ws".to_string(),
                    total: 5,
                    offset,
                    returned: 2,
                    messages: vec![SessionTranscriptMessageData {
                        role: "user".to_string(),
                        created: 1_700_000_000_000,
                        previews: vec!["fix the flaky test".to_string()],
                    }],
                })
            }
            _ => panic!("expected a read request"),
        });

        let result = SessionTool
            .execute(
                serde_json::json!({"action": "read", "id": "ses_a", "limit": 2}),
                ctx,
            )
            .await
            .expect("read should succeed");

        assert!(result.output.contains("## User"));
        assert!(result.output.contains("fix the flaky test"));
        assert!(result.output.contains("offset 2"));
        assert!(result.truncated);
    }

    #[tokio::test]
    async fn read_requires_id() {
        let ctx = ctx_with(|_| panic!("callback should not run without an id"));
        let error = SessionTool
            .execute(serde_json::json!({"action": "read"}), ctx)
            .await
            .expect_err("missing id should fail");
        assert!(matches!(error, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn unknown_action_is_rejected() {
        let ctx = ctx_with(|_| panic!("callback should not run for unknown action"));
        let error = SessionTool
            .execute(serde_json::json!({"action": "delete"}), ctx)
            .await
            .expect_err("unknown action should fail");
        assert!(matches!(error, ToolError::InvalidArguments(_)));
    }

    #[tokio::test]
    async fn missing_callback_reports_unavailable() {
        let ctx = ToolContext::new(
            "ses_current".to_string(),
            "msg_current".to_string(),
            "/tmp/ws".to_string(),
        );
        let error = SessionTool
            .execute(serde_json::json!({"action": "list"}), ctx)
            .await
            .expect_err("missing callback should fail");
        assert!(matches!(error, ToolError::ExecutionError(_)));
    }
}
