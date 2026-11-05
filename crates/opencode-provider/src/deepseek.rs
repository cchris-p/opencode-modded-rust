use async_trait::async_trait;
use reqwest::Client;

use crate::{ChatRequest, ChatResponse, ModelInfo, Provider, ProviderError, StreamResult};

const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_NPM: &str = "@ai-sdk/openai-compatible";

/// Canonical DeepSeek model metadata.
///
/// Values mirror the live listing at `https://api.deepseek.com/models`
/// (verified 2026-11-05) and models.dev, which agree on 1,048,576 context and
/// 393,216 max output for both models. Keep this as the single source of truth so
/// the bundled fallback catalog and the direct provider cannot drift again.
pub struct DeepSeekModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub release_date: &'static str,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub cost_per_million_input: f64,
    pub cost_per_million_output: f64,
}

pub const DEEPSEEK_MODELS: &[DeepSeekModelSpec] = &[
    DeepSeekModelSpec {
        id: "deepseek-v4-pro",
        name: "DeepSeek V4 Pro",
        release_date: "2026-08-01",
        context_window: 1_048_576,
        max_output_tokens: 393_216,
        supports_vision: false,
        supports_tools: true,
        cost_per_million_input: 0.55,
        cost_per_million_output: 2.19,
    },
    DeepSeekModelSpec {
        id: "deepseek-flash",
        name: "DeepSeek Flash",
        release_date: "2026-08-01",
        context_window: 1_048_576,
        max_output_tokens: 393_216,
        supports_vision: true,
        supports_tools: true,
        cost_per_million_input: 0.27,
        cost_per_million_output: 1.1,
    },
];

/// Runtime provider-model view of the canonical DeepSeek catalog.
pub fn bundled_models() -> Vec<ModelInfo> {
    DEEPSEEK_MODELS
        .iter()
        .map(|spec| ModelInfo {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            provider: "deepseek".to_string(),
            context_window: spec.context_window,
            max_output_tokens: spec.max_output_tokens,
            supports_vision: spec.supports_vision,
            supports_tools: spec.supports_tools,
            cost_per_million_input: spec.cost_per_million_input,
            cost_per_million_output: spec.cost_per_million_output,
        })
        .collect()
}

/// models.dev-shaped view of the canonical DeepSeek catalog, used by the
/// bundled fallback so a missing or corrupt cache never degrades the metadata.
pub fn bundled_models_dev() -> Vec<crate::models::ModelInfo> {
    DEEPSEEK_MODELS
        .iter()
        .map(|spec| {
            let mut input = vec!["text".to_string()];
            if spec.supports_vision {
                input.push("image".to_string());
            }

            crate::models::ModelInfo {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                family: None,
                release_date: Some(spec.release_date.to_string()),
                attachment: spec.supports_vision,
                reasoning: true,
                temperature: true,
                tool_call: spec.supports_tools,
                interleaved: Some(crate::models::ModelInterleaved::Bool(false)),
                cost: Some(crate::models::ModelCost {
                    input: spec.cost_per_million_input,
                    output: spec.cost_per_million_output,
                    cache_read: None,
                    cache_write: None,
                    context_over_200k: None,
                }),
                limit: crate::models::ModelLimit {
                    context: spec.context_window,
                    input: None,
                    output: spec.max_output_tokens,
                },
                modalities: Some(crate::models::ModelModalities {
                    input,
                    output: vec!["text".to_string()],
                }),
                experimental: None,
                status: Some("active".to_string()),
                options: std::collections::HashMap::new(),
                headers: None,
                provider: Some(crate::models::ModelProvider {
                    npm: Some(DEEPSEEK_NPM.to_string()),
                    api: Some(DEEPSEEK_API_URL.to_string()),
                }),
                variants: None,
            }
        })
        .collect()
}

#[derive(Debug)]
pub struct DeepSeekProvider {
    client: Client,
    api_key: String,
    models: Vec<ModelInfo>,
}

/// Map a selected effort variant onto the OpenAI-compatible `reasoning_effort`
/// field. The live listing advertises `low`/`high`/`max` (default `high`), so an
/// unsupported value is rejected before any network I/O instead of silently
/// falling back to the endpoint default.
fn apply_reasoning_effort(
    body: &mut serde_json::Value,
    variant: Option<&str>,
) -> Result<(), ProviderError> {
    let Some(variant) = variant else {
        return Ok(());
    };
    let effort = variant.trim().to_ascii_lowercase();
    if effort.is_empty() {
        return Ok(());
    }

    match effort.as_str() {
        "low" | "high" | "max" => {
            if let Some(object) = body.as_object_mut() {
                object.insert(
                    "reasoning_effort".to_string(),
                    serde_json::Value::String(effort),
                );
            }
            Ok(())
        }
        other => Err(ProviderError::InvalidRequest(format!(
            "unsupported DeepSeek reasoning effort '{other}'; expected one of low, high, max"
        ))),
    }
}

impl DeepSeekProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            models: bundled_models(),
        }
    }
}

#[async_trait]
impl Provider for DeepSeekProvider {
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
        self.models.iter().find(|m| m.id == id)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut body = crate::openai_chat::openai_chat_completions_body(&request)?;
        apply_reasoning_effort(&mut body, request.variant.as_deref())?;
        let response = self
            .client
            .post(DEEPSEEK_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| ProviderError::ApiError(e.to_string()))
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<StreamResult, ProviderError> {
        let mut stream_request = request;
        stream_request.stream = Some(true);
        let mut body = crate::openai_chat::openai_chat_completions_body(&stream_request)?;
        apply_reasoning_effort(&mut body, stream_request.variant.as_deref())?;

        let response = self
            .client
            .post(DEEPSEEK_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("{}: {}", status, body)));
        }

        let stream = crate::stream::openai_compat_sse_stream(response.bytes_stream());

        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_live_listing() {
        let provider = DeepSeekProvider::new("test-key");

        assert!(provider.get_model("deepseek-chat").is_none());
        assert!(provider.get_model("deepseek-reasoner").is_none());

        for id in ["deepseek-flash", "deepseek-v4-pro"] {
            let model = provider.get_model(id).expect("model is listed");
            assert_eq!(model.context_window, 1_048_576, "{id} context");
            assert_eq!(model.max_output_tokens, 393_216, "{id} output");
            assert!(model.supports_tools, "{id} tools");
        }

        let flash = provider.get_model("deepseek-flash").unwrap();
        assert!(flash.supports_vision);
    }

    #[test]
    fn models_dev_view_keeps_reasoning_and_image_input() {
        let models = bundled_models_dev();
        let flash = models
            .iter()
            .find(|model| model.id == "deepseek-flash")
            .expect("flash is listed");

        assert_eq!(flash.limit.context, 1_048_576);
        assert_eq!(flash.limit.output, 393_216);
        assert!(flash.reasoning);
        assert!(flash.tool_call);
        assert!(flash.attachment);
        let input = &flash.modalities.as_ref().unwrap().input;
        assert!(input.contains(&"text".to_string()));
        assert!(input.contains(&"image".to_string()));
    }

    #[test]
    fn reasoning_effort_maps_supported_levels_and_rejects_others() {
        let mut body = serde_json::json!({"model": "deepseek-flash"});
        apply_reasoning_effort(&mut body, Some(" HIGH ")).expect("high is supported");
        assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

        let mut body = serde_json::json!({"model": "deepseek-flash"});
        apply_reasoning_effort(&mut body, Some("max")).expect("max is supported");
        assert_eq!(body["reasoning_effort"], serde_json::json!("max"));

        let mut body = serde_json::json!({"model": "deepseek-flash"});
        let error = apply_reasoning_effort(&mut body, Some("medium"))
            .expect_err("medium is not advertised");
        assert!(error.to_string().contains("unsupported"));
        assert!(body.get("reasoning_effort").is_none());

        let mut body = serde_json::json!({"model": "deepseek-flash"});
        apply_reasoning_effort(&mut body, None).expect("no variant leaves the default");
        assert!(body.get("reasoning_effort").is_none());
    }
}
