use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    ChatRequest, ChatResponse, Choice, Content, ContentPart, Message, ModelInfo, Provider,
    ProviderError, Role, StreamResult, ToolDefinition, ToolUse, Usage,
};

const DEEPSEEK_ANTHROPIC_URL: &str = "https://api.deepseek.com/anthropic/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// DeepSeek's Anthropic-Messages transport.
///
/// This is an opt-in alternative to `/chat/completions`, selected by the
/// `transport = "anthropic"` provider option. It is intentionally independent of
/// the Anthropic provider so DeepSeek-specific wire semantics (in-history system
/// updates, `output_config.effort`, cache usage fields) stay isolated.
#[derive(Debug)]
pub struct DeepSeekAnthropicProvider {
    client: Client,
    api_key: String,
    models: Vec<ModelInfo>,
}

impl DeepSeekAnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            models: crate::deepseek::bundled_models(),
        }
    }

    fn request_body(&self, request: &ChatRequest) -> Result<Value, ProviderError> {
        convert_request(request)
    }
}

#[async_trait]
impl Provider for DeepSeekAnthropicProvider {
    fn id(&self) -> &str {
        "deepseek"
    }

    fn name(&self) -> &str {
        "DeepSeek"
    }

    fn models(&self) -> Vec<ModelInfo> {
        self.models.clone()
    }

    fn get_model(&self, id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|model| model.id == id)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut body = self.request_body(&request)?;
        body["stream"] = Value::Bool(false);

        let response = self
            .client
            .post(DEEPSEEK_ANTHROPIC_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("{status}: {body}")));
        }

        let parsed: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))?;
        Ok(convert_response(parsed))
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<StreamResult, ProviderError> {
        let mut body = self.request_body(&request)?;
        body["stream"] = Value::Bool(true);

        let response = self
            .client
            .post(DEEPSEEK_ANTHROPIC_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("{status}: {body}")));
        }

        Ok(crate::stream::anthropic_compat_sse_stream(
            response.bytes_stream(),
        ))
    }
}

/// Convert the internal chat request into a DeepSeek Anthropic Messages body.
///
/// Leading system messages become the `system` block array. Any system message
/// that appears after conversation content is kept in history (the capability
/// `deepseek-flash` advertises); `deepseek-v4-pro` advertises leading-only, so
/// callers should not rely on late system updates for that model.
fn convert_request(request: &ChatRequest) -> Result<Value, ProviderError> {
    let mut system: Vec<Value> = Vec::new();
    let mut messages: Vec<Value> = Vec::new();
    let mut saw_content = false;
    let mut last_was_tool_result = false;

    for message in &request.messages {
        match message.role {
            Role::System => {
                let text = content_text(&message.content);
                if text.is_empty() {
                    continue;
                }
                if saw_content {
                    messages.push(json!({
                        "role": "system",
                        "content": [{"type": "text", "text": text}]
                    }));
                } else {
                    system.push(json!({"type": "text", "text": text}));
                }
            }
            Role::User => {
                saw_content = true;
                last_was_tool_result = false;
                messages.push(json!({
                    "role": "user",
                    "content": user_blocks(&message.content)
                }));
            }
            Role::Assistant => {
                saw_content = true;
                last_was_tool_result = false;
                let (blocks, trailing_tool_results) = assistant_blocks(&message.content);
                if !blocks.is_empty() {
                    messages.push(json!({"role": "assistant", "content": blocks}));
                }
                if !trailing_tool_results.is_empty() {
                    messages.push(json!({"role": "user", "content": trailing_tool_results}));
                    last_was_tool_result = true;
                }
            }
            Role::Tool => {
                saw_content = true;
                let blocks = tool_result_blocks(&message.content);
                if blocks.is_empty() {
                    continue;
                }
                if last_was_tool_result {
                    if let Some(last) = messages.last_mut() {
                        if let Some(content) = last.get_mut("content").and_then(Value::as_array_mut)
                        {
                            content.extend(blocks);
                            continue;
                        }
                    }
                }
                messages.push(json!({"role": "user", "content": blocks}));
                last_was_tool_result = true;
            }
        }
    }

    let mut body = json!({
        "model": request.model,
        "max_tokens": request.max_tokens.unwrap_or(8192),
        "messages": messages,
    });

    if !system.is_empty() {
        body["system"] = Value::Array(system);
    }

    if let Some(tools) = &request.tools {
        body["tools"] = Value::Array(tools.iter().map(anthropic_tool).collect());
    }

    if let Some(variant) = request.variant.as_deref() {
        let effort = variant.trim().to_ascii_lowercase();
        if !effort.is_empty() {
            match effort.as_str() {
                "low" | "high" | "max" => {
                    body["output_config"] = json!({"effort": effort});
                }
                other => {
                    return Err(ProviderError::InvalidRequest(format!(
                        "unsupported DeepSeek reasoning effort '{other}'; expected one of low, high, max"
                    )));
                }
            }
        }
    }

    Ok(body)
}

fn anthropic_tool(tool: &ToolDefinition) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.parameters,
    })
}

fn content_text(content: &Content) -> String {
    match content {
        Content::Text(text) => text.clone(),
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|part| part.text.clone())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn user_blocks(content: &Content) -> Vec<Value> {
    match content {
        Content::Text(text) => vec![json!({"type": "text", "text": text})],
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|part| {
                if let Some(text) = &part.text {
                    Some(json!({"type": "text", "text": text}))
                } else if let Some(tool_result) = &part.tool_result {
                    Some(json!({
                        "type": "tool_result",
                        "tool_use_id": tool_result.tool_use_id,
                        "content": tool_result.content,
                        "is_error": tool_result.is_error.unwrap_or(false),
                    }))
                } else {
                    part.image_url
                        .as_ref()
                        .and_then(|image| image_block(&image.url))
                }
            })
            .collect(),
    }
}

fn image_block(url: &str) -> Option<Value> {
    if let Some(rest) = url.strip_prefix("data:") {
        let (meta, data) = rest.split_once(',')?;
        let media_type = meta.strip_suffix(";base64").unwrap_or(meta);
        Some(json!({
            "type": "image",
            "source": {"type": "base64", "media_type": media_type, "data": data}
        }))
    } else {
        Some(json!({
            "type": "image",
            "source": {"type": "url", "url": url}
        }))
    }
}

fn assistant_blocks(content: &Content) -> (Vec<Value>, Vec<Value>) {
    let mut blocks = Vec::new();
    let mut trailing_tool_results = Vec::new();

    let parts = match content {
        Content::Text(text) => {
            return (vec![json!({"type": "text", "text": text})], Vec::new());
        }
        Content::Parts(parts) => parts,
    };

    for part in parts {
        if let Some(tool_use) = &part.tool_use {
            blocks.push(json!({
                "type": "tool_use",
                "id": tool_use.id,
                "name": tool_use.name,
                "input": tool_use.input,
            }));
        } else if let Some(text) = &part.text {
            if part.content_type == "reasoning" {
                // DeepSeek requires thinking blocks to be echoed back on tool
                // follow-ups. Our persisted reasoning text has no signature, and
                // the endpoint accepts an empty one.
                blocks.push(json!({
                    "type": "thinking",
                    "thinking": text,
                    "signature": "",
                }));
            } else {
                blocks.push(json!({"type": "text", "text": text}));
            }
        } else if let Some(tool_result) = &part.tool_result {
            trailing_tool_results.push(json!({
                "type": "tool_result",
                "tool_use_id": tool_result.tool_use_id,
                "content": tool_result.content,
                "is_error": tool_result.is_error.unwrap_or(false),
            }));
        }
    }

    (blocks, trailing_tool_results)
}

fn tool_result_blocks(content: &Content) -> Vec<Value> {
    match content {
        Content::Text(text) => {
            if text.is_empty() {
                Vec::new()
            } else {
                vec![json!({"type": "text", "text": text})]
            }
        }
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|part| {
                part.tool_result.as_ref().map(|tool_result| {
                    json!({
                        "type": "tool_result",
                        "tool_use_id": tool_result.tool_use_id,
                        "content": tool_result.content,
                        "is_error": tool_result.is_error.unwrap_or(false),
                    })
                })
            })
            .collect(),
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    content: Vec<AnthropicBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
}

fn convert_response(response: AnthropicResponse) -> ChatResponse {
    let mut content_parts = Vec::new();

    for block in response.content {
        match block.kind.as_str() {
            "text" => {
                if let Some(text) = block.text {
                    content_parts.push(ContentPart {
                        content_type: "text".to_string(),
                        text: Some(text),
                        ..Default::default()
                    });
                }
            }
            "thinking" => {
                if let Some(text) = block.thinking {
                    content_parts.push(ContentPart {
                        content_type: "reasoning".to_string(),
                        text: Some(text),
                        ..Default::default()
                    });
                }
            }
            "tool_use" => {
                if let (Some(id), Some(name)) = (block.id, block.name) {
                    content_parts.push(ContentPart {
                        content_type: "tool_use".to_string(),
                        tool_use: Some(ToolUse {
                            id,
                            name,
                            input: block.input.unwrap_or_else(|| json!({})),
                        }),
                        ..Default::default()
                    });
                }
            }
            _ => {}
        }
    }

    let content = if content_parts.len() == 1 && content_parts[0].content_type == "text" {
        Content::Text(content_parts.remove(0).text.unwrap_or_default())
    } else {
        Content::Parts(content_parts)
    };

    let finish_reason = match response.stop_reason.as_deref() {
        Some("tool_use") => Some("tool_calls".to_string()),
        Some("max_tokens") => Some("length".to_string()),
        Some("end_turn") | Some("stop_sequence") => Some("stop".to_string()),
        other => other.map(str::to_string),
    };

    ChatResponse {
        id: response.id,
        model: response.model,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                cache_control: None,
                provider_options: None,
            },
            finish_reason,
        }],
        usage: Some(Usage {
            // Anthropic's `input_tokens` is the uncached remainder; include cache
            // read/write so `prompt_tokens` is the full prompt total.
            prompt_tokens: response.usage.input_tokens
                + response.usage.cache_read_input_tokens
                + response.usage.cache_creation_input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens
                + response.usage.cache_read_input_tokens
                + response.usage.cache_creation_input_tokens
                + response.usage.output_tokens,
            cache_read_input_tokens: Some(response.usage.cache_read_input_tokens),
            cache_creation_input_tokens: Some(response.usage.cache_creation_input_tokens),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_request(text: &str) -> ChatRequest {
        let mut request = ChatRequest::new("deepseek-flash", vec![Message::user(text)]);
        request.max_tokens = Some(1024);
        request
    }

    #[test]
    fn leading_system_messages_become_a_system_block_array() {
        let mut request = user_request("hi");
        request.messages.insert(0, Message::system("stable base"));
        request
            .messages
            .insert(1, Message::system("dynamic context"));

        let body = convert_request(&request).expect("body builds");
        assert_eq!(
            body["system"],
            json!([
                {"type": "text", "text": "stable base"},
                {"type": "text", "text": "dynamic context"}
            ])
        );
        assert_eq!(body["messages"][0]["role"], "user");
    }

    #[test]
    fn tool_definitions_and_reasoning_effort_are_mapped() {
        let mut request = user_request("hi");
        request.tools = Some(vec![ToolDefinition {
            name: "bash".to_string(),
            description: Some("Run a command".to_string()),
            parameters: json!({"type": "object", "properties": {}}),
        }]);
        request.variant = Some("max".to_string());

        let body = convert_request(&request).expect("body builds");
        assert_eq!(body["tools"][0]["name"], "bash");
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(body["output_config"]["effort"], "max");

        request.variant = Some("medium".to_string());
        assert!(convert_request(&request).is_err());
    }

    #[test]
    fn assistant_tool_use_and_tool_results_round_trip() {
        let mut request = user_request("hi");
        request.messages.push(Message {
            role: Role::Assistant,
            content: Content::Parts(vec![
                ContentPart {
                    content_type: "reasoning".to_string(),
                    text: Some("thinking".to_string()),
                    ..Default::default()
                },
                ContentPart {
                    content_type: "tool_use".to_string(),
                    tool_use: Some(ToolUse {
                        id: "call_1".to_string(),
                        name: "bash".to_string(),
                        input: json!({"command": "ls"}),
                    }),
                    ..Default::default()
                },
            ]),
            cache_control: None,
            provider_options: None,
        });
        request.messages.push(Message {
            role: Role::Tool,
            content: Content::Parts(vec![ContentPart {
                content_type: "tool_result".to_string(),
                tool_result: Some(crate::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    content: "file.txt".to_string(),
                    is_error: Some(false),
                }),
                ..Default::default()
            }]),
            cache_control: None,
            provider_options: None,
        });

        let body = convert_request(&request).expect("body builds");
        assert_eq!(body["messages"][1]["role"], "assistant");
        assert_eq!(body["messages"][1]["content"][0]["type"], "thinking");
        assert_eq!(body["messages"][1]["content"][1]["type"], "tool_use");
        assert_eq!(body["messages"][2]["role"], "user");
        assert_eq!(body["messages"][2]["content"][0]["type"], "tool_result");
        assert_eq!(body["messages"][2]["content"][0]["tool_use_id"], "call_1");
    }

    #[test]
    fn response_tool_use_becomes_a_content_part() {
        let response: AnthropicResponse = serde_json::from_value(json!({
            "id": "msg_1",
            "model": "deepseek-flash",
            "stop_reason": "tool_use",
            "content": [
                {"type": "thinking", "thinking": "need a tool", "signature": "s"},
                {"type": "tool_use", "id": "call_1", "name": "bash", "input": {"command": "ls"}}
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "cache_read_input_tokens": 7,
                "cache_creation_input_tokens": 0
            }
        }))
        .expect("response parses");

        let converted = convert_response(response);
        assert_eq!(
            converted.choices[0].finish_reason.as_deref(),
            Some("tool_calls")
        );
        match &converted.choices[0].message.content {
            Content::Parts(parts) => {
                assert_eq!(parts[0].content_type, "reasoning");
                assert_eq!(parts[1].content_type, "tool_use");
                assert_eq!(parts[1].tool_use.as_ref().unwrap().id, "call_1");
            }
            other => panic!("expected parts, got {other:?}"),
        }
        let usage = converted.usage.expect("usage");
        assert_eq!(usage.cache_read_input_tokens, Some(7));
    }
}
