use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    pub content: String,
    pub is_error: bool,
}

impl AgentMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: Vec::new(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: Vec::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
        }
    }

    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls,
        }
    }

    pub fn tool_result(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        _is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_calls: vec![ToolCall {
                id: tool_call_id.into(),
                name: name.into(),
                arguments: serde_json::Value::Null,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<AgentMessage>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn with_system_prompt(prompt: impl Into<String>) -> Self {
        let mut conv = Self::new();
        conv.messages.push(AgentMessage::system(prompt));
        conv
    }

    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.messages.push(AgentMessage::user(content));
    }

    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.messages.push(AgentMessage::assistant(content));
    }

    pub fn add_assistant_message_with_tools(
        &mut self,
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) {
        self.messages
            .push(AgentMessage::assistant_with_tools(content, tool_calls));
    }

    pub fn add_tool_result(
        &mut self,
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) {
        self.messages.push(AgentMessage::tool_result(
            tool_call_id,
            name,
            content,
            is_error,
        ));
    }

    pub fn to_provider_messages(&self) -> Vec<opencode_provider::Message> {
        self.messages
            .iter()
            .map(|m| match m.role {
                MessageRole::System => opencode_provider::Message::system(&m.content),
                MessageRole::User => opencode_provider::Message::user(&m.content),
                MessageRole::Assistant if m.tool_calls.is_empty() => {
                    opencode_provider::Message::assistant(&m.content)
                }
                MessageRole::Assistant => opencode_provider::Message {
                    role: opencode_provider::Role::Assistant,
                    content: opencode_provider::Content::Parts({
                        let mut parts = Vec::new();
                        if !m.content.is_empty() {
                            parts.push(opencode_provider::ContentPart {
                                content_type: "text".to_string(),
                                text: Some(m.content.clone()),
                                ..Default::default()
                            });
                        }
                        parts.extend(m.tool_calls.iter().map(|tool_call| {
                            opencode_provider::ContentPart {
                                content_type: "tool_use".to_string(),
                                tool_use: Some(opencode_provider::ToolUse {
                                    id: tool_call.id.clone(),
                                    name: tool_call.name.clone(),
                                    input: tool_call.arguments.clone(),
                                }),
                                ..Default::default()
                            }
                        }));
                        parts
                    }),
                    cache_control: None,
                    provider_options: None,
                },
                MessageRole::Tool => opencode_provider::Message {
                    role: opencode_provider::Role::Tool,
                    content: opencode_provider::Content::Parts(vec![
                        opencode_provider::ContentPart {
                            content_type: "tool_result".to_string(),
                            tool_result: Some(opencode_provider::ToolResult {
                                tool_use_id: m
                                    .tool_calls
                                    .first()
                                    .map(|tool_call| tool_call.id.clone())
                                    .unwrap_or_default(),
                                content: m.content.clone(),
                                is_error: None,
                            }),
                            ..Default::default()
                        },
                    ]),
                    cache_control: None,
                    provider_options: None,
                },
            })
            .collect()
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencode_provider::{Content, Role};

    #[test]
    fn conversation_preserves_tool_use_and_result_parts() {
        let mut conversation = Conversation::new();
        conversation.add_assistant_message_with_tools(
            "I will read that file.",
            vec![ToolCall {
                id: "call_1".to_string(),
                name: "read".to_string(),
                arguments: serde_json::json!({ "filePath": "README.md" }),
            }],
        );
        conversation.add_tool_result("call_1", "read", "contents", false);

        let messages = conversation.to_provider_messages();
        assert!(matches!(messages[0].role, Role::Assistant));
        assert!(matches!(messages[1].role, Role::Tool));

        let Content::Parts(assistant_parts) = &messages[0].content else {
            panic!("assistant tool call should use parts content");
        };
        assert_eq!(
            assistant_parts[0].text.as_deref(),
            Some("I will read that file.")
        );
        let tool_use = assistant_parts[1]
            .tool_use
            .as_ref()
            .expect("missing tool_use part");
        assert_eq!(tool_use.id, "call_1");
        assert_eq!(tool_use.name, "read");

        let Content::Parts(tool_parts) = &messages[1].content else {
            panic!("tool result should use parts content");
        };
        let tool_result = tool_parts[0]
            .tool_result
            .as_ref()
            .expect("missing tool_result part");
        assert_eq!(tool_result.tool_use_id, "call_1");
        assert_eq!(tool_result.content, "contents");
    }
}
