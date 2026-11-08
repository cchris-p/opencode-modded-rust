use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

pub const MODELS_DEV_URL: &str = "https://models.opencode.ai";

/// Freshness window for the on-disk models.dev catalog cache. Mirrors vanilla
/// OpenCode's `Duration.minutes(5)` in `packages/core/src/models-dev.ts`; a cache
/// older than this is refetched on the next load or scheduled refresh.
pub const MODELS_DEV_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
    #[serde(default)]
    pub context_over_200k: Option<Box<ModelCost>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimit {
    pub context: u64,
    #[serde(default)]
    pub input: Option<u64>,
    pub output: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelModalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    #[serde(default)]
    pub npm: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelInterleaved {
    Bool(bool),
    Field { field: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelExperimentalModeProvider {
    #[serde(default)]
    pub body: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelExperimentalMode {
    #[serde(default)]
    pub cost: Option<ModelCost>,
    #[serde(default)]
    pub provider: Option<ModelExperimentalModeProvider>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelExperimentalModes {
    #[serde(default)]
    pub modes: HashMap<String, ModelExperimentalMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelExperimental {
    Legacy(bool),
    Modes(ModelExperimentalModes),
}

impl ModelExperimental {
    pub fn modes(&self) -> Option<&HashMap<String, ModelExperimentalMode>> {
        match self {
            ModelExperimental::Legacy(_) => None,
            ModelExperimental::Modes(modes) => Some(&modes.modes),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub temperature: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub interleaved: Option<ModelInterleaved>,
    #[serde(default)]
    pub cost: Option<ModelCost>,
    pub limit: ModelLimit,
    #[serde(default)]
    pub modalities: Option<ModelModalities>,
    #[serde(default)]
    pub experimental: Option<ModelExperimental>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub provider: Option<ModelProvider>,
    #[serde(default)]
    pub variants: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    #[serde(default)]
    pub api: Option<String>,
    pub name: String,
    pub env: Vec<String>,
    pub id: String,
    #[serde(default)]
    pub npm: Option<String>,
    pub models: HashMap<String, ModelInfo>,
}

pub type ModelsData = HashMap<String, ProviderInfo>;

pub struct ModelsRegistry {
    data: Arc<RwLock<Option<ModelsData>>>,
    cache_path: PathBuf,
    ttl: Duration,
    source_url: String,
}

impl ModelsRegistry {
    pub fn new(cache_path: PathBuf) -> Self {
        Self::with_ttl(cache_path, MODELS_DEV_TTL)
    }

    /// Build a registry with an explicit freshness window. Primarily useful for
    /// tests that need deterministic staleness without waiting on the clock.
    pub fn with_ttl(cache_path: PathBuf, ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(None)),
            cache_path,
            ttl,
            source_url: MODELS_DEV_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_source(cache_path: PathBuf, ttl: Duration, source_url: String) -> Self {
        Self {
            data: Arc::new(RwLock::new(None)),
            cache_path,
            ttl,
            source_url,
        }
    }

    pub async fn get(&self) -> ModelsData {
        {
            let data = self.data.read().await;
            if let Some(ref d) = *data {
                return d.clone();
            }
        }

        self.load().await
    }

    async fn load(&self) -> ModelsData {
        match self.read_cache().await {
            Some(cached) if self.cache_is_fresh().await => {
                self.store(cached.clone()).await;
                cached
            }
            Some(cached) => match self.fetch().await {
                Some(fresh) => fresh,
                None => {
                    tracing::debug!(
                        "models.dev catalog refresh failed; using the existing cached catalog"
                    );
                    self.store(cached.clone()).await;
                    cached
                }
            },
            None => self.fetch().await.unwrap_or_default(),
        }
    }

    async fn read_cache(&self) -> Option<ModelsData> {
        let content = tokio::fs::read_to_string(&self.cache_path).await.ok()?;
        serde_json::from_str::<ModelsData>(&content).ok()
    }

    async fn cache_is_fresh(&self) -> bool {
        let Ok(metadata) = tokio::fs::metadata(&self.cache_path).await else {
            return false;
        };
        let Ok(mtime) = metadata.modified() else {
            return false;
        };
        cache_mtime_is_fresh(mtime, SystemTime::now(), self.ttl)
    }

    async fn store(&self, parsed: ModelsData) {
        let mut data = self.data.write().await;
        *data = Some(parsed);
    }

    /// Fetch the catalog, writing a successful response to the on-disk cache and
    /// replacing the in-memory snapshot. Returns `None` on any failure so callers
    /// can keep serving the existing cache.
    async fn fetch(&self) -> Option<ModelsData> {
        let url = format!("{}/api.json", self.source_url);

        let response = reqwest::Client::new()
            .get(&url)
            .header("User-Agent", "opencode-rust")
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let text = response.text().await.ok()?;
        let parsed = serde_json::from_str::<ModelsData>(&text).ok()?;
        let _ = tokio::fs::write(&self.cache_path, &text).await;
        self.store(parsed.clone()).await;
        Some(parsed)
    }

    /// Refresh the catalog following the freshness policy. When `force` is false
    /// and the cache is still within the TTL this is a no-op; `force` always
    /// refetches. Returns `true` only when a fetch succeeded.
    pub async fn refresh(&self, force: bool) -> bool {
        if !force && self.cache_is_fresh().await {
            return false;
        }

        match self.fetch().await {
            Some(_) => true,
            None => {
                tracing::debug!("models.dev catalog refresh failed; keeping the existing cache");
                false
            }
        }
    }

    pub async fn get_provider(&self, provider_id: &str) -> Option<ProviderInfo> {
        let data = self.get().await;
        data.get(provider_id).cloned()
    }

    pub async fn get_model(&self, provider_id: &str, model_id: &str) -> Option<ModelInfo> {
        let data = self.get().await;
        data.get(provider_id)
            .and_then(|p| p.models.get(model_id).cloned())
    }

    pub async fn list_models_for_provider(&self, provider_id: &str) -> Vec<ModelInfo> {
        let data = self.get().await;
        data.get(provider_id)
            .map(|p| p.models.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Apply custom loaders and filtering to the loaded models data
    pub async fn get_with_customization(&self, enable_experimental: bool) -> ModelsData {
        let mut data = self.get().await;
        crate::bootstrap::apply_custom_loaders(&mut data);
        crate::bootstrap::filter_models_by_status(&mut data, enable_experimental);
        data
    }
}

impl Default for ModelsRegistry {
    fn default() -> Self {
        let cache_path = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("opencode")
            .join("models.json");
        Self::new(cache_path)
    }
}

/// Returns true when `mtime` is within `ttl` of `now`. A future mtime (clock
/// skew) is treated as fresh so a skewed clock does not force a refetch on every
/// load. Extracted so staleness detection is unit-testable without network or
/// filesystem mtime manipulation.
pub fn cache_mtime_is_fresh(mtime: SystemTime, now: SystemTime, ttl: Duration) -> bool {
    match now.duration_since(mtime) {
        Ok(age) => age < ttl,
        Err(_) => true,
    }
}

/// Ensure the on-disk models.dev catalog cache is populated and fresh before
/// provider bootstrapping reads it. `bootstrap_registry` only reads the cache
/// file and otherwise falls back to the bundled snapshot, so without this the
/// runtime provider/model list can silently lag the canonical catalog. When the
/// cache is stale (older than `MODELS_DEV_TTL`) this refetches; a failed fetch
/// keeps the existing cache.
pub async fn ensure_models_dev_cache() {
    let registry = ModelsRegistry::default();
    let _ = registry.get().await;
}

/// Force a models.dev catalog refetch, ignoring the freshness TTL, and leave the
/// refreshed cache on disk for the subsequent provider bootstrap to read.
/// Returns `true` when the fetch succeeded; failures are non-fatal and preserve
/// the existing cache.
pub async fn refresh_models_dev_cache() -> bool {
    let registry = ModelsRegistry::default();
    registry.refresh(true).await
}

pub fn default_model_limits() -> (u64, u64) {
    (4096, 128000)
}

pub fn get_model_context_limit(model_id: &str) -> u64 {
    let lower = model_id.to_lowercase();

    if lower.contains("gpt-4") || lower.contains("gpt-4") {
        if lower.contains("32k") {
            return 32768;
        }
        if lower.contains("128k") || lower.contains("turbo") {
            return 128000;
        }
        return 8192;
    }

    if lower.contains("claude-3") || lower.contains("claude-3") {
        return 200000;
    }

    if lower.contains("claude-2") {
        return 100000;
    }

    if lower.contains("gemini") {
        if lower.contains("pro") || lower.contains("ultra") {
            return 1000000;
        }
        return 32000;
    }

    if lower.contains("llama") {
        return 128000;
    }

    128000
}

pub fn supports_vision(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();

    lower.contains("vision")
        || lower.contains("gpt-4")
        || lower.contains("claude-3")
        || lower.contains("gemini")
        || lower.contains("qwen-vl")
}

pub fn supports_function_calling(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();

    !lower.contains("embedding") && !lower.contains("whisper") && !lower.contains("tts")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const SAMPLE_CATALOG: &str = r#"{"demo":{"name":"Demo","env":[],"id":"demo","models":{}}}"#;

    fn temp_cache_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!(
            "opencode-models-registry-{}-{}-{}",
            std::process::id(),
            label,
            nanos
        ));
        std::fs::create_dir_all(&dir).expect("create temp cache dir");
        dir.join("models.json")
    }

    fn write_sample_cache(path: &Path) {
        std::fs::write(path, SAMPLE_CATALOG).expect("write sample cache");
    }

    fn cleanup(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn ttl_boundary_classifies_fresh_and_stale() {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(300);

        assert!(cache_mtime_is_fresh(
            now - Duration::from_secs(60),
            now,
            ttl
        ));
        assert!(!cache_mtime_is_fresh(
            now - Duration::from_secs(301),
            now,
            ttl
        ));
        // A future mtime (clock skew) stays fresh instead of refetching forever.
        assert!(cache_mtime_is_fresh(now + Duration::from_secs(5), now, ttl));
    }

    #[tokio::test]
    async fn fresh_cache_is_served_without_refetch() {
        let path = temp_cache_path("fresh");
        write_sample_cache(&path);

        let registry = ModelsRegistry::with_source(
            path.clone(),
            MODELS_DEV_TTL,
            "http://127.0.0.1:1".to_string(),
        );

        let data = registry.get().await;
        assert!(data.contains_key("demo"));

        // A non-forced refresh on a fresh cache must not hit the unreachable source.
        assert!(!registry.refresh(false).await);

        cleanup(&path);
    }

    #[tokio::test]
    async fn stale_cache_refetch_failure_keeps_cached_catalog() {
        let path = temp_cache_path("stale");
        write_sample_cache(&path);

        // A zero TTL makes the cache immediately stale, so `get` attempts a
        // refetch. The source is unreachable, so the cached catalog is preserved.
        let registry = ModelsRegistry::with_source(
            path.clone(),
            Duration::ZERO,
            "http://127.0.0.1:1".to_string(),
        );

        let data = registry.get().await;
        assert!(data.contains_key("demo"));

        assert!(!registry.refresh(true).await);

        cleanup(&path);
    }

    #[tokio::test]
    async fn missing_cache_with_failed_fetch_returns_empty() {
        let path = temp_cache_path("missing");
        let registry = ModelsRegistry::with_source(
            path.clone(),
            MODELS_DEV_TTL,
            "http://127.0.0.1:1".to_string(),
        );

        assert!(registry.get().await.is_empty());

        cleanup(&path);
    }
}
