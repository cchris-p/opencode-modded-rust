//! Native Ollama endpoint configuration.
//!
//! The product talks to Ollama over HTTP only. It never shells out to the
//! `ollama` binary, so endpoint resolution has to work from configuration and
//! environment alone.
//!
//! Resolution precedence (high to low):
//!   1. CLI `--model` (`ollama/<tag>` selects the model)
//!   2. `OLLAMA_REMOTE_HOST` / `OLLAMA_REMOTE_MODEL`
//!   3. `~/.config/opencode/ollama.json` (`{ "host": "...", "model": "..." }`)
//!   4. `OLLAMA_HOST`
//!   5. `http://127.0.0.1:11434`

use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const OLLAMA_PROVIDER_ID: &str = "ollama";
pub const OLLAMA_DEFAULT_HOST: &str = "http://127.0.0.1:11434";

/// Where a resolved value came from. The label is user-facing: it is printed in
/// the connection-failure diagnostic and in `debug ollama`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OllamaSource {
    CliModel,
    EnvRemoteHost,
    EnvRemoteModel,
    ConfigFile,
    EnvOllamaHost,
    Default,
}

impl OllamaSource {
    pub fn label(&self) -> String {
        match self {
            OllamaSource::CliModel => "cli --model".to_string(),
            OllamaSource::EnvRemoteHost => "environment (OLLAMA_REMOTE_HOST)".to_string(),
            OllamaSource::EnvRemoteModel => "environment (OLLAMA_REMOTE_MODEL)".to_string(),
            OllamaSource::ConfigFile => "config file".to_string(),
            OllamaSource::EnvOllamaHost => "environment (OLLAMA_HOST)".to_string(),
            OllamaSource::Default => "runtime default".to_string(),
        }
    }

    fn is_explicit(&self) -> bool {
        !matches!(self, OllamaSource::Default)
    }
}

/// Parsed `ollama.json` payload. Unknown fields are ignored so the file stays
/// forward-compatible.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OllamaConfigFile {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Environment inputs for resolution. Kept separate from process env so tests
/// can exercise precedence without mutating global state.
#[derive(Debug, Clone, Default)]
pub struct OllamaEnv {
    pub remote_host: Option<String>,
    pub remote_model: Option<String>,
    pub ollama_host: Option<String>,
}

impl OllamaEnv {
    pub fn from_process() -> Self {
        let read = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        Self {
            remote_host: read("OLLAMA_REMOTE_HOST"),
            remote_model: read("OLLAMA_REMOTE_MODEL"),
            ollama_host: read("OLLAMA_HOST"),
        }
    }
}

/// Fully resolved Ollama endpoint configuration.
#[derive(Debug, Clone)]
pub struct OllamaResolution {
    /// Base host, always with a scheme and without a trailing `/v1`.
    pub host: String,
    /// OpenAI-compatible base URL: `host` + `/v1`.
    pub base_url: String,
    /// Bare model tag, when one was configured.
    pub model: Option<String>,
    /// `ollama/<tag>` reference for the resolved model.
    pub model_ref: Option<String>,
    pub host_source: OllamaSource,
    pub model_source: Option<OllamaSource>,
    /// Absolute path of the config file consulted (whether or not it exists).
    pub config_path: PathBuf,
    pub config_found: bool,
    pub config_error: Option<String>,
}

impl OllamaResolution {
    /// True when the user configured an endpoint or model explicitly (remote
    /// env, config file, or `OLLAMA_HOST`). The bare runtime default does not
    /// count, so an ordinary launch never probes localhost.
    pub fn explicitly_configured(&self) -> bool {
        self.host_source.is_explicit() || self.model_source.is_some()
    }

    pub fn host_label(&self) -> String {
        display_source(&self.host_source, &self.config_path)
    }

    pub fn model_label(&self) -> Option<String> {
        self.model_source
            .as_ref()
            .map(|source| display_source(source, &self.config_path))
    }

    /// Multi-line diagnostic used when the endpoint cannot be reached.
    pub fn connection_failure_message(&self, reason: &str) -> String {
        let config_state = if self.config_found {
            "found"
        } else {
            "not found"
        };
        let mut message = String::new();
        message.push_str("Ollama endpoint connection failed.\n");
        message.push_str(&format!("  resolved URL: {}\n", self.base_url));
        message.push_str(&format!(
            "  config file consulted: {} ({config_state})\n",
            self.config_path.display()
        ));
        message.push_str(&format!("  host source: {}\n", self.host_label()));
        if let Some(model) = self.model_label() {
            message.push_str(&format!("  model source: {model}\n"));
        }
        if let Some(error) = &self.config_error {
            message.push_str(&format!("  config file error: {error}\n"));
        }
        message.push_str(&format!("  reason: {reason}"));
        message
    }
}

fn display_source(source: &OllamaSource, config_path: &Path) -> String {
    match source {
        OllamaSource::ConfigFile => config_path.display().to_string(),
        _ => source.label(),
    }
}

/// Normalize a host into `scheme://authority` with no trailing slash and no
/// trailing `/v1`. The OpenAI-compatible base URL is derived by appending `/v1`.
pub fn normalize_host(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return OLLAMA_DEFAULT_HOST.to_string();
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    let mut normalized = with_scheme.trim_end_matches('/').to_string();
    if let Some(stripped) = normalized.strip_suffix("/v1") {
        normalized = stripped.trim_end_matches('/').to_string();
    }
    normalized
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Parse an `ollama/<tag>` CLI model reference. Any other provider prefix
/// returns `None`; a bare model (no provider prefix) is not treated as Ollama.
pub fn cli_ollama_model(cli_model: Option<&str>) -> Option<String> {
    let raw = cli_model?.trim();
    let (provider, model) = raw.split_once('/')?;
    if provider.eq_ignore_ascii_case(OLLAMA_PROVIDER_ID) {
        let model = model.trim();
        (!model.is_empty()).then(|| model.to_string())
    } else {
        None
    }
}

/// Pure resolver used by tests and by the process-backed [`resolve`] wrapper.
pub fn resolve_with(
    cli_model: Option<&str>,
    env: &OllamaEnv,
    config_path: &Path,
    config: Option<&OllamaConfigFile>,
) -> OllamaResolution {
    let file_host = config.and_then(|config| clean(&config.host));
    let file_model = config.and_then(|config| clean(&config.model));

    let (host_raw, host_source) = if let Some(host) = clean(&env.remote_host) {
        (host, OllamaSource::EnvRemoteHost)
    } else if let Some(host) = file_host {
        (host, OllamaSource::ConfigFile)
    } else if let Some(host) = clean(&env.ollama_host) {
        (host, OllamaSource::EnvOllamaHost)
    } else {
        (OLLAMA_DEFAULT_HOST.to_string(), OllamaSource::Default)
    };

    let cli_model = cli_model.map(str::trim).filter(|value| !value.is_empty());
    let cli_ollama = cli_ollama_model(cli_model);

    let (model, model_source) = if let Some(model) = cli_ollama {
        (Some(model), Some(OllamaSource::CliModel))
    } else if cli_model.is_some() {
        // An explicit non-Ollama model suppresses Ollama model defaults while
        // still allowing an explicitly configured Ollama host to register.
        (None, None)
    } else if let Some(model) = clean(&env.remote_model) {
        (Some(model), Some(OllamaSource::EnvRemoteModel))
    } else if let Some(model) = file_model {
        (Some(model), Some(OllamaSource::ConfigFile))
    } else {
        (None, None)
    };

    let host = normalize_host(&host_raw);
    let base_url = format!("{host}/v1");
    let model_ref = model
        .as_ref()
        .map(|model| format!("{OLLAMA_PROVIDER_ID}/{model}"));

    OllamaResolution {
        host,
        base_url,
        model,
        model_ref,
        host_source,
        model_source,
        config_path: config_path.to_path_buf(),
        config_found: config.is_some(),
        config_error: None,
    }
}

/// Default config file path: `$OPENCODE_OLLAMA_CONFIG` when set, otherwise
/// `~/.config/opencode/ollama.json`. This intentionally follows the vanilla
/// OpenCode global config location on every OS.
pub fn default_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("OPENCODE_OLLAMA_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config/opencode/ollama.json")
}

fn read_config_file(path: &Path) -> (Option<OllamaConfigFile>, bool, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str::<OllamaConfigFile>(&raw) {
            Ok(config) => (Some(config), true, None),
            Err(error) => (None, true, Some(format!("{}: {error}", path.display()))),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, false, None),
        Err(error) => (None, false, Some(format!("{}: {error}", path.display()))),
    }
}

/// Resolve from the current process environment and default config path.
pub fn resolve(cli_model: Option<&str>) -> OllamaResolution {
    resolve_from(cli_model, OllamaEnv::from_process(), &default_config_path())
}

/// Resolve from explicit environment inputs and config path.
pub fn resolve_from(
    cli_model: Option<&str>,
    env: OllamaEnv,
    config_path: &Path,
) -> OllamaResolution {
    let (config, found, error) = read_config_file(config_path);
    let mut resolution = resolve_with(cli_model, &env, config_path, config.as_ref());
    resolution.config_found = found;
    resolution.config_error = error;
    resolution
}

/// Probe the endpoint over HTTP. `/v1/models` is tried first because the
/// provider is registered as OpenAI-compatible; `/api/tags` is the native
/// fallback for servers that do not expose the OpenAI route.
pub async fn probe(resolution: &OllamaResolution, timeout: Duration) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?;

    let models_url = format!("{}/models", resolution.base_url);
    let mut reasons = Vec::new();

    match client.get(&models_url).send().await {
        Ok(response) if response.status().is_success() => return Ok(()),
        Ok(response) => reasons.push(format!(
            "GET {models_url} returned HTTP {}",
            response.status()
        )),
        Err(error) => reasons.push(format!("GET {models_url}: {}", error_chain(&error))),
    }

    let tags_url = format!("{}/api/tags", resolution.host);
    match client.get(&tags_url).send().await {
        Ok(response) if response.status().is_success() => return Ok(()),
        Ok(response) => reasons.push(format!(
            "GET {tags_url} returned HTTP {}",
            response.status()
        )),
        Err(error) => reasons.push(format!("GET {tags_url}: {}", error_chain(&error))),
    }

    Err(reasons.join("; "))
}

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

impl fmt::Display for OllamaResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base_url)?;
        if let Some(model) = &self.model_ref {
            write!(f, " model={model}")?;
        }
        write!(f, " host_source={}", self.host_label())?;
        if let Some(model_source) = self.model_label() {
            write!(f, " model_source={model_source}")?;
        }
        write!(f, " config={}", self.config_path.display())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(remote_host: Option<&str>, remote_model: Option<&str>, host: Option<&str>) -> OllamaEnv {
        OllamaEnv {
            remote_host: remote_host.map(str::to_string),
            remote_model: remote_model.map(str::to_string),
            ollama_host: host.map(str::to_string),
        }
    }

    fn config(host: Option<&str>, model: Option<&str>) -> OllamaConfigFile {
        OllamaConfigFile {
            host: host.map(str::to_string),
            model: model.map(str::to_string),
        }
    }

    #[test]
    fn host_precedence_remote_env_beats_file_and_ollama_host() {
        let path = Path::new("/home/test/.config/opencode/ollama.json");
        let file = config(Some("http://file:11434"), Some("file-model"));
        let resolution = resolve_with(
            None,
            &env(
                Some("http://remote:11434"),
                None,
                Some("http://ollama-host:11434"),
            ),
            path,
            Some(&file),
        );
        assert_eq!(resolution.host, "http://remote:11434");
        assert_eq!(resolution.host_source, OllamaSource::EnvRemoteHost);
    }

    #[test]
    fn host_precedence_file_beats_ollama_host() {
        let path = Path::new("/home/test/.config/opencode/ollama.json");
        let file = config(Some("http://file:11434"), None);
        let resolution = resolve_with(
            None,
            &env(None, None, Some("http://ollama-host:11434")),
            path,
            Some(&file),
        );
        assert_eq!(resolution.host, "http://file:11434");
        assert_eq!(resolution.host_source, OllamaSource::ConfigFile);
    }

    #[test]
    fn host_falls_back_to_ollama_host_then_default() {
        let path = Path::new("/tmp/ollama.json");
        let with_host = resolve_with(None, &env(None, None, Some("127.0.0.1:1234")), path, None);
        assert_eq!(with_host.host, "http://127.0.0.1:1234");
        assert_eq!(with_host.base_url, "http://127.0.0.1:1234/v1");
        assert_eq!(with_host.host_source, OllamaSource::EnvOllamaHost);

        let default = resolve_with(None, &env(None, None, None), path, None);
        assert_eq!(default.host, OLLAMA_DEFAULT_HOST);
        assert_eq!(default.base_url, "http://127.0.0.1:11434/v1");
        assert_eq!(default.host_source, OllamaSource::Default);
        assert!(!default.explicitly_configured());
    }

    #[test]
    fn model_precedence_cli_beats_env_beats_file() {
        let path = Path::new("/tmp/ollama.json");
        let file = config(None, Some("file-model"));
        let cli = resolve_with(
            Some("ollama/cli-model"),
            &env(None, Some("env-model"), None),
            path,
            Some(&file),
        );
        assert_eq!(cli.model.as_deref(), Some("cli-model"));
        assert_eq!(cli.model_source, Some(OllamaSource::CliModel));

        let env_win = resolve_with(None, &env(None, Some("env-model"), None), path, Some(&file));
        assert_eq!(env_win.model.as_deref(), Some("env-model"));
        assert_eq!(env_win.model_source, Some(OllamaSource::EnvRemoteModel));

        let file_win = resolve_with(None, &env(None, None, None), path, Some(&file));
        assert_eq!(file_win.model.as_deref(), Some("file-model"));
        assert_eq!(file_win.model_source, Some(OllamaSource::ConfigFile));
    }

    #[test]
    fn non_ollama_cli_model_suppresses_configured_model_default() {
        let path = Path::new("/tmp/ollama.json");
        let file = config(None, Some("file-model"));
        let resolution = resolve_with(
            Some("deepseek/deepseek-chat"),
            &env(None, Some("env-model"), None),
            path,
            Some(&file),
        );
        assert_eq!(resolution.model, None);
        assert_eq!(resolution.model_source, None);
    }

    #[test]
    fn normalize_host_adds_scheme_and_strips_v1() {
        assert_eq!(normalize_host("127.0.0.1:11434"), "http://127.0.0.1:11434");
        assert_eq!(
            normalize_host("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434"
        );
        assert_eq!(
            normalize_host("https://ollama.example.com/"),
            "https://ollama.example.com"
        );
        assert_eq!(normalize_host("  "), OLLAMA_DEFAULT_HOST);
    }

    #[test]
    fn model_ref_is_prefixed_with_provider_id() {
        let resolution = resolve_with(
            None,
            &env(None, None, None),
            Path::new("/tmp/ollama.json"),
            Some(&config(Some("http://remote:11434"), Some("qwen3:30b"))),
        );
        assert_eq!(resolution.model_ref.as_deref(), Some("ollama/qwen3:30b"));
        assert!(resolution.explicitly_configured());
    }

    #[test]
    fn connection_failure_message_names_url_config_file_and_reason() {
        let path = Path::new("/home/test/.config/opencode/ollama.json");
        let resolution = resolve_with(
            None,
            &env(Some("http://missing:11434"), Some("qwen3:30b"), None),
            path,
            None,
        );
        let message = resolution.connection_failure_message("connection refused");
        assert!(message.contains("http://missing:11434/v1"));
        assert!(message.contains("/home/test/.config/opencode/ollama.json"));
        assert!(message.contains("not found"));
        assert!(message.contains("connection refused"));
    }
}
