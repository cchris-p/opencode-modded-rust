//! OpenAI Chat Completions wire-format conversion.
//!
//! The internal `ChatRequest`/`Message` model is provider-neutral: text lives in
//! `Content::Text` and tool calls/results are Anthropic-style `Content::Parts`
//! (`tool_use` / `tool_result`). OpenAI-compatible `/chat/completions` endpoints
//! need a different wire shape:
//!
//! - `tools` items must be `{ "type": "function", "function": { name, description, parameters } }`.
//! - assistant tool calls must be `tool_calls: [{ id, type: "function", function: { name, arguments } }]`.
//! - tool results must be separate `{ role: "tool", tool_call_id, content }` messages.
//!
//! This module converts an internal request into that OpenAI chat wire body. It
//! is the chat-completions counterpart of `responses_convert.rs`.

use serde_json::{json, Value};

use crate::message::{Content, Message, Role, ToolDefinition};
use crate::ProviderError;

/// Build the OpenAI-compatible `/chat/completions` request body for an internal
/// `ChatRequest`, converting tools and messages into the OpenAI wire shape.
pub fn openai_chat_completions_body(request: &crate::ChatRequest) -> Result<Value, ProviderError> {
    let mut value =
        serde_json::to_value(request).map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

    let obj = value
        .as_object_mut()
        .ok_or_else(|| ProviderError::InvalidRequest("request is not an object".into()))?;

    // Replace tools with the OpenAI function-call shape.
    match &request.tools {
        Some(tools) => {
            obj.insert(
                "tools".to_string(),
                Value::Array(tools.iter().map(openai_chat_tool).collect()),
            );
        }
        None => {
            obj.remove("tools");
        }
    }

    // Replace messages with the OpenAI chat wire shape.
    obj.insert(
        "messages".to_string(),
        Value::Array(convert_messages(&request.messages)),
    );

    Ok(value)
}

/// Convert a single tool definition into `{ type: "function", function: {...} }`.
pub fn openai_chat_tool(tool: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
        }
    })
}

/// Convert tool definitions into an OpenAI `tools` array.
pub fn openai_chat_tools(tools: &[ToolDefinition]) -> Value {
    Value::Array(tools.iter().map(openai_chat_tool).collect())
}

/// Convert internal messages into OpenAI chat messages.
///
/// Rules:
/// - `system`/`user` text → `{ role, content: "..." }`.
/// - multimodal user content → `{ role, content: [text/image parts] }`.
/// - assistant text → `{ role: "assistant", content: "..." }` plus `tool_calls`
///   for each `tool_use` part.
/// - `tool_result` parts (whether on an assistant message or a `Role::Tool`
///   message) are emitted as separate `{ role: "tool", tool_call_id, content }`
///   messages immediately after their owning assistant message.
/// - reasoning parts are dropped (not part of the OpenAI chat content schema).
pub fn convert_messages(messages: &[Message]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for message in messages {
        match (&message.role, &message.content) {
            (Role::System, Content::Text(text)) => {
                out.push(json!({ "role": "system", "content": text }));
            }
            (Role::System, Content::Parts(parts)) => {
                let content = content_string_from_parts(parts);
                out.push(json!({ "role": "system", "content": content }));
            }
            (Role::User, Content::Text(text)) => {
                out.push(json!({ "role": "user", "content": text }));
            }
            (Role::User, Content::Parts(parts)) => {
                out.push(json!({
                    "role": "user",
                    "content": openai_user_content(parts)
                }));
            }
            (Role::Assistant, Content::Text(text)) => {
                out.push(json!({ "role": "assistant", "content": text }));
            }
            (Role::Assistant, Content::Parts(parts)) => {
                convert_assistant_parts(parts, &mut out);
            }
            (Role::Tool, Content::Parts(parts)) => {
                for part in parts {
                    if let Some(result) = &part.tool_result {
                        out.push(openai_tool_result_message(
                            &result.tool_use_id,
                            &result.content,
                        ));
                    } else if let Some(text) = part.text.as_ref() {
                        // A bare text part on a tool message carries no call id;
                        // keep it only if it can be tied to a preceding call.
                        if !text.trim().is_empty() {
                            out.push(json!({ "role": "tool", "content": text }));
                        }
                    }
                }
            }
            (Role::Tool, Content::Text(text)) => {
                out.push(json!({ "role": "tool", "content": text }));
            }
        }
    }
    out
}

/// Convert an assistant message's parts. `tool_use` parts become `tool_calls`;
/// `tool_result` parts become trailing `role: "tool"` messages.
fn convert_assistant_parts(parts: &[crate::ContentPart], out: &mut Vec<Value>) {
    let mut text_parts: Vec<&str> = Vec::new();
    let mut reasoning_parts: Vec<&str> = Vec::new();
    let mut tool_calls: Vec<Value> = Vec::new();
    let mut results: Vec<(String, String)> = Vec::new();

    for part in parts {
        match part.content_type.as_str() {
            "text" => {
                if let Some(text) = part.text.as_deref() {
                    if !text.is_empty() {
                        text_parts.push(text);
                    }
                }
            }
            // DeepSeek thinking (and compatible reasoning endpoints) require the
            // prior assistant `reasoning_content` to be passed back verbatim on
            // follow-up requests (BUG-006); surface it as a message field rather
            // than dropping it.
            "reasoning" => {
                if let Some(text) = part.text.as_deref() {
                    if !text.is_empty() {
                        reasoning_parts.push(text);
                    }
                }
            }
            "step-start" => {}
            "tool_use" => {
                if let Some(tool_use) = &part.tool_use {
                    tool_calls.push(json!({
                        "id": tool_use.id,
                        "type": "function",
                        "function": {
                            "name": tool_use.name,
                            "arguments": serde_json::to_string(&tool_use.input)
                                .unwrap_or_else(|_| "{}".to_string()),
                        }
                    }));
                }
            }
            "tool_result" => {
                if let Some(result) = &part.tool_result {
                    results.push((result.tool_use_id.clone(), result.content.clone()));
                } else if let Some(text) = part.text.as_deref() {
                    // Legacy provider responses attach text as the result body.
                    results.push((String::new(), text.to_string()));
                }
            }
            _ => {}
        }
    }

    let content = if text_parts.is_empty() {
        Value::Null
    } else {
        Value::String(text_parts.join("\n"))
    };

    // Emit the assistant shell only when it carries text, reasoning, or tool
    // calls. A message that contains only `tool_result` parts (the v1 loop stores
    // tool results as a separate assistant message) must not produce an empty
    // assistant message between a `tool_calls` assistant and its `role: tool`
    // replies, which OpenAI rejects.
    if !tool_calls.is_empty() || !text_parts.is_empty() || !reasoning_parts.is_empty() {
        let mut assistant = serde_json::Map::new();
        assistant.insert("role".into(), json!("assistant"));
        assistant.insert("content".into(), content);
        if !reasoning_parts.is_empty() {
            assistant.insert(
                "reasoning_content".into(),
                json!(reasoning_parts.join("\n")),
            );
        }
        if !tool_calls.is_empty() {
            assistant.insert("tool_calls".into(), Value::Array(tool_calls));
        }
        out.push(Value::Object(assistant));
    }

    for (call_id, result_text) in results {
        out.push(openai_tool_result_message(&call_id, &result_text));
    }
}

fn openai_tool_result_message(call_id: &str, content: &str) -> Value {
    let mut msg = serde_json::Map::new();
    msg.insert("role".into(), json!("tool"));
    if !call_id.is_empty() {
        msg.insert("tool_call_id".into(), json!(call_id));
    }
    msg.insert("content".into(), json!(content));
    Value::Object(msg)
}

/// Build a string content body from text parts (used for system messages and
/// fallbacks). Multimodal user content is handled by `openai_user_content`.
fn content_string_from_parts(parts: &[crate::ContentPart]) -> String {
    parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n")
}

/// OpenAI chat user content: a string when only text is present, otherwise an
/// array of `{ type: "text" }` / `{ type: "image_url" }` parts.
fn openai_user_content(parts: &[crate::ContentPart]) -> Value {
    let has_image = parts
        .iter()
        .any(|p| matches!(p.content_type.as_str(), "image_url" | "image"));
    let mut text: Vec<&str> = Vec::new();
    for p in parts {
        if p.content_type == "text" {
            if let Some(t) = p.text.as_deref() {
                if !t.is_empty() {
                    text.push(t);
                }
            }
        }
    }

    if !has_image {
        return Value::String(text.join("\n"));
    }

    let mut content: Vec<Value> = Vec::new();
    for p in parts {
        match p.content_type.as_str() {
            "text" => {
                if let Some(t) = p.text.as_deref() {
                    if !t.is_empty() {
                        content.push(json!({ "type": "text", "text": t }));
                    }
                }
            }
            "image_url" | "image" => {
                if let Some(url) = p.image_url.as_ref().map(|img| img.url.clone()) {
                    content.push(json!({ "type": "image_url", "image_url": { "url": url } }));
                }
            }
            _ => {}
        }
    }
    Value::Array(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{ContentPart, ToolResult, ToolUse};

    fn text_part(text: &str) -> ContentPart {
        ContentPart {
            content_type: "text".to_string(),
            text: Some(text.to_string()),
            ..Default::default()
        }
    }

    fn tool_use_part(id: &str, name: &str, input: Value) -> ContentPart {
        ContentPart {
            content_type: "tool_use".to_string(),
            tool_use: Some(ToolUse {
                id: id.to_string(),
                name: name.to_string(),
                input,
            }),
            ..Default::default()
        }
    }

    fn tool_result_part(id: &str, content: &str) -> ContentPart {
        ContentPart {
            content_type: "tool_result".to_string(),
            text: Some(content.to_string()),
            tool_result: Some(ToolResult {
                tool_use_id: id.to_string(),
                content: content.to_string(),
                is_error: Some(false),
            }),
            ..Default::default()
        }
    }

    fn reasoning_part(text: &str) -> ContentPart {
        ContentPart {
            content_type: "reasoning".to_string(),
            text: Some(text.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn assistant_reasoning_is_echoed_as_reasoning_content() {
        // DeepSeek thinking requires the prior assistant reasoning_content to be
        // passed back on the follow-up request (BUG-006).
        let messages = vec![Message {
            role: Role::Assistant,
            content: Content::Parts(vec![
                reasoning_part("step one"),
                reasoning_part(" step two"),
                tool_use_part("call_1", "ls", json!({ "path": "/tmp" })),
            ]),
            cache_control: None,
            provider_options: None,
        }];
        let out = convert_messages(&messages);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "assistant");
        assert_eq!(out[0]["reasoning_content"], "step one\n step two");
        assert!(out[0]["tool_calls"][0]["id"] == "call_1");
    }

    #[test]
    fn plain_text_messages_unchanged() {
        let messages = vec![Message::system("sys"), Message::user("hi")];
        let out = convert_messages(&messages);
        assert_eq!(out[0]["role"], "system");
        assert_eq!(out[0]["content"], "sys");
        assert_eq!(out[1]["role"], "user");
        assert_eq!(out[1]["content"], "hi");
    }

    #[test]
    fn assistant_tool_use_becomes_tool_calls_and_tool_messages() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: Content::Parts(vec![
                text_part("let me check"),
                tool_use_part("call_1", "bash", json!({ "cmd": "ls" })),
                tool_result_part("call_1", "ok"),
            ]),
            cache_control: None,
            provider_options: None,
        }];

        let out = convert_messages(&messages);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["role"], "assistant");
        assert_eq!(out[0]["content"], "let me check");
        assert_eq!(out[0]["tool_calls"][0]["id"], "call_1");
        assert_eq!(out[0]["tool_calls"][0]["type"], "function");
        assert_eq!(
            out[0]["tool_calls"][0]["function"]["arguments"],
            json!(r#"{"cmd":"ls"}"#)
        );
        assert_eq!(out[1]["role"], "tool");
        assert_eq!(out[1]["tool_call_id"], "call_1");
        assert_eq!(out[1]["content"], "ok");
    }

    #[test]
    fn tool_role_message_becomes_tool_message() {
        let messages = vec![Message {
            role: Role::Tool,
            content: Content::Parts(vec![tool_result_part("call_9", "result text")]),
            cache_control: None,
            provider_options: None,
        }];
        let out = convert_messages(&messages);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "tool");
        assert_eq!(out[0]["tool_call_id"], "call_9");
        assert_eq!(out[0]["content"], "result text");
    }

    #[test]
    fn separate_assistant_tool_result_message_emits_only_tool_messages() {
        // v1 loop stores tool results as their own assistant message containing
        // only tool_result parts. It must emit role:tool messages with no empty
        // assistant shell in between.
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: Content::Parts(vec![tool_use_part(
                    "call_1",
                    "ls",
                    json!({ "path": "/tmp" }),
                )]),
                cache_control: None,
                provider_options: None,
            },
            Message {
                role: Role::Assistant,
                content: Content::Parts(vec![tool_result_part("call_1", "files")]),
                cache_control: None,
                provider_options: None,
            },
        ];
        let out = convert_messages(&messages);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["role"], "assistant");
        assert!(out[0]["tool_calls"][0]["id"] == "call_1");
        assert_eq!(out[1]["role"], "tool");
        assert_eq!(out[1]["tool_call_id"], "call_1");
        assert_eq!(out[1]["content"], "files");
    }

    #[test]
    fn tools_are_wrapped_in_function_schema() {
        let tool = ToolDefinition {
            name: "read".to_string(),
            description: Some("read a file".to_string()),
            parameters: json!({ "type": "object", "properties": {} }),
        };
        let v = openai_chat_tool(&tool);
        assert_eq!(v["type"], "function");
        assert_eq!(v["function"]["name"], "read");
        assert_eq!(v["function"]["description"], "read a file");
        assert_eq!(v["function"]["parameters"]["type"], "object");
    }
}
