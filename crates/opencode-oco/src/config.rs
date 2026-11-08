use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_FILE: &str = ".opencommit";

pub const DEFAULT_MAX_TOKENS_INPUT: u64 = 4096;
pub const DEFAULT_MAX_TOKENS_OUTPUT: u64 = 500;

pub const BOOLEAN_KEYS: &[&str] = &[
    "OCO_DESCRIPTION",
    "OCO_EMOJI",
    "OCO_EMOJI_POSITION_BEFORE_DESCRIPTION",
    "OCO_WHY",
    "OCO_ONE_LINE_COMMIT",
    "OCO_OMIT_SCOPE",
    "OCO_GITPUSH",
    "OCO_HOOK_AUTO_UNCOMMENT",
    "OCO_OLLAMA_THINK",
];

pub const NUMERIC_KEYS: &[&str] = &["OCO_TOKENS_MAX_INPUT", "OCO_TOKENS_MAX_OUTPUT"];

pub const PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "gemini",
    "ollama",
    "deepseek",
    "openrouter",
    "azure",
    "groq",
    "mistral",
    "mlx",
    "flowise",
    "aimlapi",
    "test",
];

pub const KEY_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "OCO_API_KEY",
        "The API key used to authenticate with the AI provider",
    ),
    (
        "OCO_TOKENS_MAX_INPUT",
        "Maximum number of input tokens sent to the model",
    ),
    (
        "OCO_TOKENS_MAX_OUTPUT",
        "Maximum number of output tokens requested from the model",
    ),
    (
        "OCO_DESCRIPTION",
        "Add a short description of WHY the changes were done after the commit message",
    ),
    (
        "OCO_EMOJI",
        "Preface the commit message with a GitMoji emoji",
    ),
    (
        "OCO_EMOJI_POSITION_BEFORE_DESCRIPTION",
        "Place the GitMoji emoji at the start of the description",
    ),
    ("OCO_MODEL", "The AI model used to generate commit messages"),
    (
        "OCO_LANGUAGE",
        "Language instruction used for the commit message",
    ),
    (
        "OCO_WHY",
        "Ask the model to explain why the changes were made",
    ),
    (
        "OCO_MESSAGE_TEMPLATE_PLACEHOLDER",
        "Placeholder replaced by the generated message in forwarded git args",
    ),
    (
        "OCO_PROMPT_MODULE",
        "Prompt module: conventional-commit or @commitlint",
    ),
    (
        "OCO_AI_PROVIDER",
        "AI provider: openai, anthropic, gemini, ollama, deepseek, openrouter, ...",
    ),
    (
        "OCO_ONE_LINE_COMMIT",
        "Generate a single-line commit message",
    ),
    (
        "OCO_TEST_MOCK_TYPE",
        "Mock response used by the test provider",
    ),
    ("OCO_API_URL", "Override the provider base URL"),
    (
        "OCO_PROXY",
        "HTTP(S) proxy URL (accepted for compatibility)",
    ),
    (
        "OCO_API_CUSTOM_HEADERS",
        "JSON object of additional provider request headers",
    ),
    (
        "OCO_OMIT_SCOPE",
        "Do not include a conventional-commit scope",
    ),
    ("OCO_GITPUSH", "Offer to run git push after committing"),
    (
        "OCO_HOOK_AUTO_UNCOMMENT",
        "Write the generated hook message without comment prefixes",
    ),
    ("OCO_OLLAMA_THINK", "Pass the think option to Ollama"),
];

#[derive(Debug, Clone)]
pub struct Config {
    values: BTreeMap<String, Value>,
}

impl Config {
    pub fn defaults() -> Self {
        let mut values = BTreeMap::new();
        values.insert(
            "OCO_TOKENS_MAX_INPUT".into(),
            Value::from(DEFAULT_MAX_TOKENS_INPUT),
        );
        values.insert(
            "OCO_TOKENS_MAX_OUTPUT".into(),
            Value::from(DEFAULT_MAX_TOKENS_OUTPUT),
        );
        values.insert("OCO_DESCRIPTION".into(), Value::Bool(false));
        values.insert("OCO_EMOJI".into(), Value::Bool(false));
        values.insert(
            "OCO_EMOJI_POSITION_BEFORE_DESCRIPTION".into(),
            Value::Bool(false),
        );
        values.insert("OCO_MODEL".into(), Value::String(default_model("openai")));
        values.insert("OCO_LANGUAGE".into(), Value::String("en".into()));
        values.insert(
            "OCO_MESSAGE_TEMPLATE_PLACEHOLDER".into(),
            Value::String("$msg".into()),
        );
        values.insert(
            "OCO_PROMPT_MODULE".into(),
            Value::String("conventional-commit".into()),
        );
        values.insert("OCO_AI_PROVIDER".into(), Value::String("openai".into()));
        values.insert("OCO_ONE_LINE_COMMIT".into(), Value::Bool(false));
        values.insert(
            "OCO_TEST_MOCK_TYPE".into(),
            Value::String("commit-message".into()),
        );
        values.insert("OCO_WHY".into(), Value::Bool(false));
        values.insert("OCO_OMIT_SCOPE".into(), Value::Bool(false));
        values.insert("OCO_GITPUSH".into(), Value::Bool(true));
        values.insert("OCO_HOOK_AUTO_UNCOMMENT".into(), Value::Bool(false));
        Self { values }
    }

    /// Load configuration with upstream precedence:
    /// process env > repo `.env` > `~/.opencommit` > defaults.
    pub fn load(global_path: &Path, cwd: &Path, env: &HashMap<String, String>) -> Result<Self> {
        let mut config = Self::defaults();

        if global_path.exists() {
            let text = fs::read_to_string(global_path)
                .with_context(|| format!("failed to read {}", global_path.display()))?;
            for (key, value) in parse_ini(&text) {
                config.values.insert(key, value);
            }
        }

        let env_path = cwd.join(".env");
        if env_path.exists() {
            let text = fs::read_to_string(&env_path)
                .with_context(|| format!("failed to read {}", env_path.display()))?;
            for (key, value) in parse_dotenv(&text) {
                config.values.insert(key, value);
            }
        }

        for (key, value) in env {
            if key.starts_with("OCO_") {
                config
                    .values
                    .insert(key.clone(), parse_scalar(value.trim()));
            }
        }

        config.values.retain(|_, value| !is_undefined(value));
        config.validate()?;
        Ok(config)
    }

    pub fn from_values(values: BTreeMap<String, Value>) -> Self {
        Self { values }
    }

    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn string(&self, key: &str) -> Option<String> {
        self.values.get(key).and_then(value_to_string)
    }

    pub fn string_or(&self, key: &str, default: &str) -> String {
        self.string(key).unwrap_or_else(|| default.to_string())
    }

    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        match self.values.get(key) {
            Some(Value::Bool(value)) => *value,
            Some(Value::String(value)) => matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            ),
            Some(Value::Number(value)) => value.as_u64().unwrap_or(0) != 0,
            _ => default,
        }
    }

    pub fn u64_or(&self, key: &str, default: u64) -> u64 {
        match self.values.get(key) {
            Some(Value::Number(value)) => value.as_u64().unwrap_or(default),
            Some(Value::String(value)) => value.trim().parse().unwrap_or(default),
            Some(Value::Bool(value)) => u64::from(*value),
            _ => default,
        }
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
    }

    pub fn validate(&self) -> Result<()> {
        for (key, value) in &self.values {
            // Upstream skips falsy values; an empty `OCO_API_URL=` line is how
            // opencommit-config clears the override, so it must stay valid.
            if matches!(value, Value::String(text) if text.is_empty()) {
                continue;
            }
            validate_value(key, value, self)?;
        }
        Ok(())
    }

    /// Merge `set` pairs into a copy of this config and persist globally.
    pub fn set_pairs(&mut self, pairs: &[(String, String)]) -> Result<()> {
        for (key, raw) in pairs {
            if !is_known_key(key) {
                let supported = known_keys().join(", ");
                bail!("Unsupported config key: {key}. Expected one of: {supported}");
            }
            let value = parse_scalar(raw);
            validate_value(key, &value, self)?;
            self.values.insert(key.clone(), value);
        }
        Ok(())
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
        }
        let mut out = String::new();
        for (key, value) in &self.values {
            out.push_str(key);
            out.push('=');
            out.push_str(&format_value(value));
            out.push('\n');
        }
        fs::write(path, out).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn provider(&self) -> String {
        self.string_or("OCO_AI_PROVIDER", "openai")
    }

    pub fn model_for_provider(&self) -> String {
        let provider = self.provider();
        let configured = self.string_or("OCO_MODEL", "");
        if configured.trim().is_empty() {
            return default_model(&provider);
        }
        configured
    }

    pub fn language_name(&self) -> String {
        let code = self.string_or("OCO_LANGUAGE", "en");
        language_name(&code).to_string()
    }

    pub fn custom_headers(&self) -> Result<Vec<(String, String)>> {
        let Some(raw) = self.string("OCO_API_CUSTOM_HEADERS") else {
            return Ok(Vec::new());
        };
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        let parsed: Value = serde_json::from_str(&raw)
            .context("OCO_API_CUSTOM_HEADERS must be a valid JSON object")?;
        let map = parsed
            .as_object()
            .context("OCO_API_CUSTOM_HEADERS must be a JSON object")?;
        let mut headers = Vec::new();
        for (key, value) in map {
            let value = value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| value.to_string());
            headers.push((key.clone(), value));
        }
        Ok(headers)
    }
}

pub fn default_model(provider: &str) -> String {
    match provider.to_ascii_lowercase().as_str() {
        "anthropic" => "claude-sonnet-4-20250514".to_string(),
        "gemini" => "gemini-1.5-flash".to_string(),
        "deepseek" => "deepseek-chat".to_string(),
        "openrouter" => "openai/gpt-4o-mini".to_string(),
        "ollama" | "mlx" => String::new(),
        _ => "gpt-4o-mini".to_string(),
    }
}

pub fn default_global_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DEFAULT_CONFIG_FILE)
}

pub fn language_name(code: &str) -> &'static str {
    match code {
        "en" | "English" | "english" => "english",
        "zh_CN" | "简体中文" | "中文" => "Chinese (Simplified)",
        "zh_TW" | "繁體中文" | "繁體" => "Chinese (Traditional)",
        "ja" | "Japanese" | "にほんご" => "Japanese",
        "ko" | "Korean" | "한국어" => "Korean",
        "cs" | "Czech" | "česky" => "Czech",
        "de" | "German" | "Deutsch" => "German",
        "fr" | "French" | "française" => "French",
        "it" | "Italian" | "italiano" => "Italian",
        "nl" | "Dutch" | "Nederlands" => "Dutch",
        "pt_br" | "Portuguese" | "português" => "Portuguese",
        "vi_VN" | "Vietnamese" | "tiếng Việt" => "Vietnamese",
        "es_ES" | "Spanish" | "español" => "Spanish",
        "sv" | "Swedish" | "Svenska" => "Swedish",
        "ru" | "Russian" | "русский" => "Russian",
        "id_ID" | "Bahasa" | "bahasa" => "Indonesian",
        "pl" | "Polish" | "Polski" => "Polish",
        "tr" | "Turkish" => "Turkish",
        "th" | "Thai" | "ไทย" => "Thai",
        _ => "english",
    }
}

fn is_known_key(key: &str) -> bool {
    KEY_DESCRIPTIONS.iter().any(|(known, _)| *known == key)
}

fn known_keys() -> Vec<&'static str> {
    KEY_DESCRIPTIONS.iter().map(|(key, _)| *key).collect()
}

pub fn validate_value(key: &str, value: &Value, config: &Config) -> Result<()> {
    if BOOLEAN_KEYS.contains(&key) {
        if !value.is_boolean() {
            bail!("wrong value for {key}: Must be boolean: true or false");
        }
        return Ok(());
    }
    if NUMERIC_KEYS.contains(&key) {
        let ok = match value {
            Value::Number(number) => number.as_u64().is_some(),
            Value::String(text) => text.trim().parse::<u64>().is_ok(),
            _ => false,
        };
        if !ok {
            bail!("wrong value for {key}: Must be a number");
        }
        return Ok(());
    }
    match key {
        "OCO_API_URL" => {
            let text = as_text(value);
            if !(text.starts_with("http://") || text.starts_with("https://")) {
                bail!("wrong value for OCO_API_URL: must start with http:// or https://");
            }
        }
        "OCO_PROXY" => {
            if let Some(text) = value.as_str() {
                if !(text.starts_with("http://") || text.starts_with("https://")) {
                    bail!("wrong value for OCO_PROXY: must start with http:// or https://");
                }
            }
        }
        "OCO_MESSAGE_TEMPLATE_PLACEHOLDER" => {
            let text = as_text(value);
            if !text.starts_with('$') {
                bail!("wrong value for OCO_MESSAGE_TEMPLATE_PLACEHOLDER: must start with $, for example '$msg'");
            }
        }
        "OCO_PROMPT_MODULE" => {
            let text = as_text(value);
            if text != "conventional-commit" && text != "@commitlint" {
                bail!(
                    "wrong value for OCO_PROMPT_MODULE: use '@commitlint' or 'conventional-commit'"
                );
            }
        }
        "OCO_AI_PROVIDER" => {
            let text = as_text(value);
            if !PROVIDERS.contains(&text.as_str())
                && !text.to_ascii_lowercase().starts_with("ollama")
                && !text.to_ascii_lowercase().starts_with("mlx")
            {
                bail!("wrong value for OCO_AI_PROVIDER: {text} is not supported");
            }
        }
        "OCO_LANGUAGE" => {
            if as_text(value).trim().is_empty() {
                bail!("wrong value for OCO_LANGUAGE: must not be empty");
            }
        }
        "OCO_API_CUSTOM_HEADERS" => {
            let text = as_text(value);
            if !text.trim().is_empty() {
                let parsed: Value = serde_json::from_str(&text).context(
                    "wrong value for OCO_API_CUSTOM_HEADERS: must be a valid JSON object",
                )?;
                if !parsed.is_object() {
                    bail!("wrong value for OCO_API_CUSTOM_HEADERS: must be a JSON object");
                }
            }
        }
        "OCO_TEST_MOCK_TYPE" => {
            let text = as_text(value);
            if text != "commit-message" && text != "prompt-module-commitlint-config" {
                bail!("wrong value for OCO_TEST_MOCK_TYPE: unsupported mock type {text}");
            }
        }
        "OCO_API_KEY" if config.provider() == "openai" && as_text(value).is_empty() => {
            bail!("wrong value for OCO_API_KEY: Empty value is not allowed");
        }
        _ => {}
    }
    Ok(())
}

fn as_text(value: &Value) -> String {
    value_to_string(value).unwrap_or_default()
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn is_undefined(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text == "undefined" || text == "null",
        _ => false,
    }
}

fn parse_scalar(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Value::String(String::new());
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        return parsed;
    }
    Value::String(unquote(trimmed))
}

fn parse_dotenv(text: &str) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        out.push((key.trim().to_string(), parse_scalar(value.trim())));
    }
    out
}

fn parse_ini(text: &str) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        out.push((key.trim().to_string(), parse_scalar(value.trim())));
    }
    out
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            let inner = &value[1..value.len() - 1];
            return inner
                .replace("\\\"", "\"")
                .replace("\\'", "'")
                .replace("\\\\", "\\")
                .replace("\\n", "\n");
        }
    }
    value.to_string()
}

fn format_value(value: &Value) -> String {
    match value {
        Value::String(text) => {
            if needs_quotes(text) {
                format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
            } else {
                text.clone()
            }
        }
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn needs_quotes(text: &str) -> bool {
    text.is_empty()
        || text
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '#' | ';' | '=' | '"' | '\\' | '[' | ']'))
}
