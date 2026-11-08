use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_tokens_input: u64,
    pub max_tokens_output: u64,
    pub custom_headers: Vec<(String, String)>,
    pub ollama_think: Option<bool>,
    pub test_mock_type: String,
}

impl EngineConfig {
    pub fn from_config(config: &Config) -> Result<Self> {
        Ok(Self {
            provider: config.provider(),
            model: config.model_for_provider(),
            api_key: config.string_or("OCO_API_KEY", ""),
            base_url: config
                .string("OCO_API_URL")
                .filter(|value| !value.trim().is_empty()),
            max_tokens_input: config.u64_or("OCO_TOKENS_MAX_INPUT", 4096),
            max_tokens_output: config.u64_or("OCO_TOKENS_MAX_OUTPUT", 500),
            custom_headers: config.custom_headers()?,
            ollama_think: config
                .get("OCO_OLLAMA_THINK")
                .map(|_| config.bool_or("OCO_OLLAMA_THINK", false)),
            test_mock_type: config.string_or("OCO_TEST_MOCK_TYPE", "commit-message"),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    OpenAi,
    Anthropic,
    Gemini,
    Ollama,
    Test,
}

pub struct Engine {
    config: EngineConfig,
    backend: Backend,
    client: reqwest::Client,
}

impl Engine {
    pub fn from_config(config: &Config) -> Result<Self> {
        let engine_config = EngineConfig::from_config(config)?;
        let backend = match engine_config.provider.to_ascii_lowercase().as_str() {
            "openai" | "deepseek" | "openrouter" | "azure" | "groq" | "mistral" | "aimlapi"
            | "flowise" => Backend::OpenAi,
            "anthropic" => Backend::Anthropic,
            "gemini" | "google" => Backend::Gemini,
            provider if provider.starts_with("ollama") || provider.starts_with("mlx") => {
                Backend::Ollama
            }
            "test" => Backend::Test,
            other => bail!("unsupported OCO_AI_PROVIDER: {other}"),
        };

        if backend != Backend::Test
            && backend != Backend::Ollama
            && engine_config.api_key.trim().is_empty()
        {
            bail!(
                "OCO_API_KEY is not set for provider '{}'. Set it with `oco config set OCO_API_KEY=<value>` or in the environment.",
                engine_config.provider
            );
        }

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            config: engine_config,
            backend,
            client,
        })
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub async fn generate(&self, messages: &[ChatMessage]) -> Result<String> {
        let content = match self.backend {
            Backend::Test => self.generate_test(messages),
            Backend::OpenAi => self.generate_openai(messages).await?,
            Backend::Anthropic => self.generate_anthropic(messages).await?,
            Backend::Gemini => self.generate_gemini(messages).await?,
            Backend::Ollama => self.generate_ollama(messages).await?,
        };
        if content.trim().is_empty() {
            bail!(
                "{} returned an empty message (model {})",
                self.config.provider,
                self.config.model
            );
        }
        Ok(content)
    }

    fn generate_test(&self, _messages: &[ChatMessage]) -> String {
        match self.config.test_mock_type.as_str() {
            "prompt-module-commitlint-config" => {
                "{\n  \"localLanguage\": \"english\",\n  \"commitFix\": \"fix(server): Change 'port' variable to uppercase 'PORT'\",\n  \"commitFeat\": \"feat(server): Allow server to listen on a port specified through environment variable\",\n  \"commitDescription\": \"Change 'port' variable to uppercase 'PORT'. Allow server to listen on a port specified through environment variable.\"\n}".to_string()
            }
            _ => "fix(testAi.ts): test commit message".to_string(),
        }
    }

    fn openai_url(&self, default_base: &str) -> String {
        let base = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| default_base.to_string());
        chat_completions_url(&base)
    }

    async fn generate_openai(&self, messages: &[ChatMessage]) -> Result<String> {
        let provider = self.config.provider.to_ascii_lowercase();
        let default_base = match provider.as_str() {
            "deepseek" => "https://api.deepseek.com/v1",
            "openrouter" => "https://openrouter.ai/api/v1",
            "groq" => "https://api.groq.com/openai/v1",
            "mistral" => "https://api.mistral.ai/v1",
            "azure" => "https://api.openai.com/v1",
            _ => "https://api.openai.com/v1",
        };
        let url = self.openai_url(default_base);

        let mut payload = json!({
            "model": self.config.model,
            "messages": messages_json(messages),
        });
        if let Some(object) = payload.as_object_mut() {
            if is_reasoning_model(&self.config.model) {
                object.insert(
                    "max_completion_tokens".to_string(),
                    json!(self.config.max_tokens_output),
                );
            } else {
                object.insert("temperature".to_string(), json!(0));
                object.insert("top_p".to_string(), json!(0.1));
                object.insert(
                    "max_tokens".to_string(),
                    json!(self.config.max_tokens_output),
                );
            }
        }

        let mut request = self.client.post(&url).json(&payload);
        if !self.config.api_key.trim().is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.config.api_key));
        }
        if provider == "openrouter" {
            request = request
                .header("HTTP-Referer", "https://github.com/di-sukharev/opencommit")
                .header("X-Title", "OpenCommit");
        }
        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        let value = send_json(request, &self.config.provider, &self.config.model, &url).await?;
        let content = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(content_to_string)
            .ok_or_else(|| {
                format_error(
                    &self.config.provider,
                    &self.config.model,
                    &url,
                    "missing choices[0].message.content",
                )
            })?;
        Ok(content)
    }

    fn anthropic_url(&self) -> String {
        let base = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());
        if base.ends_with("/messages") {
            base
        } else {
            format!("{}/messages", base.trim_end_matches('/'))
        }
    }

    async fn generate_anthropic(&self, messages: &[ChatMessage]) -> Result<String> {
        let url = self.anthropic_url();
        let system = messages
            .iter()
            .filter(|message| message.role == Role::System)
            .map(|message| message.content.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let rest: Vec<Value> = messages
            .iter()
            .filter(|message| message.role != Role::System)
            .map(|message| {
                json!({
                    "role": if message.role == Role::Assistant { "assistant" } else { "user" },
                    "content": message.content,
                })
            })
            .collect();

        let mut payload = json!({
            "model": self.config.model,
            "messages": rest,
            "temperature": 0,
            "max_tokens": self.config.max_tokens_output,
        });
        if let Some(object) = payload.as_object_mut() {
            if !system.is_empty() {
                object.insert("system".to_string(), json!(system));
            }
            if !self.config.model.contains("-4-5") {
                object.insert("top_p".to_string(), json!(0.1));
            }
        }

        let mut request = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload);
        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        let value = send_json(request, &self.config.provider, &self.config.model, &url).await?;
        let mut text = String::new();
        if let Some(parts) = value.get("content").and_then(Value::as_array) {
            for part in parts {
                if part.get("type").and_then(Value::as_str) == Some("text") {
                    if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                        text.push_str(chunk);
                    }
                }
            }
        }
        if text.is_empty() {
            return Err(format_error(
                &self.config.provider,
                &self.config.model,
                &url,
                "missing content[].text",
            ));
        }
        Ok(text)
    }

    fn gemini_url(&self) -> String {
        let base = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());
        let base = base.trim_end_matches('/');
        format!(
            "{}/models/{}:generateContent?key={}",
            base, self.config.model, self.config.api_key
        )
    }

    async fn generate_gemini(&self, messages: &[ChatMessage]) -> Result<String> {
        let url = self.gemini_url();
        let system = messages
            .iter()
            .filter(|message| message.role == Role::System)
            .map(|message| message.content.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let contents: Vec<Value> = messages
            .iter()
            .filter(|message| message.role != Role::System)
            .map(|message| {
                json!({
                    "parts": [{ "text": message.content }],
                    "role": if message.role == Role::Assistant { "model" } else { "user" },
                })
            })
            .collect();

        let mut payload = json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": self.config.max_tokens_output,
                "temperature": 0,
                "topP": 0.1,
            },
            "safetySettings": [
                { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_LOW_AND_ABOVE" },
                { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_LOW_AND_ABOVE" },
                { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_LOW_AND_ABOVE" },
                { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_LOW_AND_ABOVE" }
            ]
        });
        if let Some(object) = payload.as_object_mut() {
            if !system.is_empty() {
                object.insert(
                    "systemInstruction".to_string(),
                    json!({ "parts": [{ "text": system }] }),
                );
            }
        }

        let mut request = self.client.post(&url).json(&payload);
        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        let value = send_json(request, &self.config.provider, &self.config.model, &url).await?;
        let mut text = String::new();
        if let Some(parts) = value
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(Value::as_array)
        {
            for part in parts {
                if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                    text.push_str(chunk);
                }
            }
        }
        if text.is_empty() {
            return Err(format_error(
                &self.config.provider,
                &self.config.model,
                &url,
                "missing candidates[0].content.parts[].text",
            ));
        }
        Ok(text)
    }

    fn ollama_url(&self) -> (String, bool) {
        let base = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        let trimmed = base.trim_end_matches('/');
        if trimmed.ends_with("/api/chat") {
            return (trimmed.to_string(), false);
        }
        if trimmed.ends_with("/v1/chat/completions") {
            return (trimmed.to_string(), true);
        }
        if trimmed.ends_with("/v1") {
            return (format!("{trimmed}/chat/completions"), true);
        }
        (format!("{trimmed}/api/chat"), false)
    }

    async fn generate_ollama(&self, messages: &[ChatMessage]) -> Result<String> {
        let (url, openai_compatible) = self.ollama_url();
        let model = if self.config.model.trim().is_empty() {
            "mistral".to_string()
        } else {
            self.config.model.clone()
        };

        if openai_compatible {
            let payload = json!({
                "model": model,
                "messages": messages_json(messages),
                "temperature": 0,
                "top_p": 0.1,
            });
            let mut request = self.client.post(&url).json(&payload);
            if !self.config.api_key.trim().is_empty() {
                request =
                    request.header("Authorization", format!("Bearer {}", self.config.api_key));
            }
            for (key, value) in &self.config.custom_headers {
                request = request.header(key, value);
            }
            let value = send_json(request, &self.config.provider, &model, &url).await?;
            let content = value
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("message"))
                .and_then(|message| message.get("content"))
                .and_then(content_to_string)
                .ok_or_else(|| {
                    format_error(
                        &self.config.provider,
                        &model,
                        &url,
                        "missing choices[0].message.content",
                    )
                })?;
            return Ok(content);
        }

        let mut payload = json!({
            "model": model,
            "messages": messages_json(messages),
            "options": { "temperature": 0, "top_p": 0.1 },
            "stream": false,
        });
        if let Some(think) = self.config.ollama_think {
            if let Some(object) = payload.as_object_mut() {
                object.insert("think".to_string(), json!(think));
            }
        }
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload);
        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }
        let value = send_json(request, &self.config.provider, &model, &url).await?;
        value
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| {
                format_error(
                    &self.config.provider,
                    &model,
                    &url,
                    "missing message.content",
                )
            })
    }
}

fn messages_json(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            json!({
                "role": message.role.as_str(),
                "content": message.content,
            })
        })
        .collect()
}

fn content_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                    text.push_str(chunk);
                }
            }
            Some(text)
        }
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

async fn send_json(
    request: reqwest::RequestBuilder,
    provider: &str,
    model: &str,
    url: &str,
) -> Result<Value> {
    let response = request.send().await.map_err(|error| {
        anyhow!(
            "{} request failed (model {}, host {}): {}",
            provider,
            model,
            host_of(url),
            error
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let fragment: String = body.chars().take(500).collect();
        bail!(
            "{} request failed (model {}, host {}, status {}): {}",
            provider,
            model,
            host_of(url),
            status.as_u16(),
            fragment
        );
    }
    response.json().await.map_err(|error| {
        anyhow!(
            "{} returned an invalid JSON response (model {}, host {}): {}",
            provider,
            model,
            host_of(url),
            error
        )
    })
}

fn host_of(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(ToString::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn format_error(provider: &str, model: &str, url: &str, detail: &str) -> anyhow::Error {
    anyhow!(
        "{} returned an unexpected response (model {}, host {}): {}",
        provider,
        model,
        host_of(url),
        detail
    )
}

fn chat_completions_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

fn is_reasoning_model(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    if lower.starts_with("gpt-5") {
        return true;
    }
    let mut chars = lower.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some('o'), Some(next)) if next.is_ascii_digit() && next != '0'
    )
}
