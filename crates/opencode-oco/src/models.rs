use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::Runtime;

const CACHE_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCache {
    #[serde(default)]
    pub timestamp: u64,
    #[serde(default)]
    pub models: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct ModelsArgs {
    refresh: bool,
    provider: Option<String>,
}

pub async fn models_command(rt: &Runtime, args: &[String]) -> Result<()> {
    if matches!(
        args.first().map(String::as_str),
        Some("-h") | Some("--help")
    ) {
        print_models_help();
        return Ok(());
    }
    let parsed = parse_models_args(args)?;
    let config = rt.load_config()?;
    let provider = parsed
        .provider
        .clone()
        .unwrap_or_else(|| config.provider())
        .to_ascii_lowercase();
    let current_model = config.model_for_provider();

    let mut cache = read_cache(&rt.models_cache_path)?;
    let now = now_ms();

    let mut models = if parsed.refresh {
        Vec::new()
    } else {
        cache
            .models
            .get(&provider)
            .filter(|_| cache_is_fresh(&cache, now))
            .cloned()
            .unwrap_or_default()
    };

    if models.is_empty() {
        models = fetch_models(&config, &provider).await;
        if models.is_empty() {
            models = static_models(&provider);
        }
        cache.models.insert(provider.clone(), models.clone());
        cache.timestamp = now;
        let _ = write_cache(&rt.models_cache_path, &cache);
    }

    println!("\nAvailable models for {provider}:\n");
    if models.is_empty() {
        println!("  No models found");
    } else {
        for model in &models {
            if model == &current_model {
                println!("* {model}");
            } else {
                println!("  {model}");
            }
        }
    }
    println!();
    println!("Run `oco models --refresh` to update the model list");
    Ok(())
}

fn parse_models_args(args: &[String]) -> Result<ModelsArgs> {
    let mut parsed = ModelsArgs::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "-r" | "--refresh" => parsed.refresh = true,
            "-p" | "--provider" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    bail!("--provider requires a value");
                };
                parsed.provider = Some(value.clone());
            }
            value if value.starts_with("--provider=") => {
                parsed.provider = Some(value["--provider=".len()..].to_string());
            }
            value if value.starts_with("-p=") => {
                parsed.provider = Some(value["-p=".len()..].to_string());
            }
            other => bail!("unknown models argument: {other}"),
        }
        index += 1;
    }
    Ok(parsed)
}

fn print_models_help() {
    println!(
        "List and manage cached models for your AI provider\n\
         \n\
         Usage: oco models [--provider <id>] [--refresh]\n\
         \n\
         --provider, -p  Specify the provider (defaults to OCO_AI_PROVIDER)\n\
         --refresh, -r   Clear the cache and re-fetch models from the provider"
    );
}

fn read_cache(path: &Path) -> Result<ModelCache> {
    if !path.exists() {
        return Ok(ModelCache::default());
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn write_cache(path: &Path, cache: &ModelCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, serde_json::to_string_pretty(cache)?)
        .with_context(|| format!("failed to write {}", path.display()))
}

fn cache_is_fresh(cache: &ModelCache, now: u64) -> bool {
    cache.timestamp > 0 && now.saturating_sub(cache.timestamp) < CACHE_TTL_MS
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

async fn fetch_models(config: &Config, provider: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(client) => client,
        Err(_) => return Vec::new(),
    };
    let api_key = config.string_or("OCO_API_KEY", "");
    let base = config
        .string("OCO_API_URL")
        .filter(|value| !value.trim().is_empty());

    match provider {
        "ollama" => {
            let base = base.unwrap_or_else(|| "http://localhost:11434".to_string());
            let base = normalize_ollama_base(&base);
            let url = format!("{base}/api/tags");
            let Ok(response) = client.get(url).send().await else {
                return Vec::new();
            };
            if !response.status().is_success() {
                return Vec::new();
            }
            let Ok(value) = response.json::<Value>().await else {
                return Vec::new();
            };
            value
                .get("models")
                .and_then(Value::as_array)
                .map(|models| {
                    models
                        .iter()
                        .filter_map(|model| model.get("name").and_then(Value::as_str))
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_default()
        }
        "anthropic" => {
            let url = base
                .map(|base| format!("{}/models", base.trim_end_matches('/')))
                .unwrap_or_else(|| "https://api.anthropic.com/v1/models".to_string());
            let Ok(response) = client
                .get(url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
            else {
                return Vec::new();
            };
            parse_openai_style_models(response).await
        }
        "openai" | "deepseek" | "openrouter" | "groq" | "mistral" => {
            let default_base = match provider {
                "deepseek" => "https://api.deepseek.com/v1",
                "openrouter" => "https://openrouter.ai/api/v1",
                "groq" => "https://api.groq.com/openai/v1",
                "mistral" => "https://api.mistral.ai/v1",
                _ => "https://api.openai.com/v1",
            };
            let base = base.unwrap_or_else(|| default_base.to_string());
            let url = format!("{}/models", base.trim_end_matches('/'));
            let Ok(response) = client.get(url).bearer_auth(api_key).send().await else {
                return Vec::new();
            };
            parse_openai_style_models(response).await
        }
        _ => Vec::new(),
    }
}

async fn parse_openai_style_models(response: reqwest::Response) -> Vec<String> {
    if !response.status().is_success() {
        return Vec::new();
    }
    let Ok(value) = response.json::<Value>().await else {
        return Vec::new();
    };
    let mut models: Vec<String> = value
        .get("data")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    models
}

fn normalize_ollama_base(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if let Some(stripped) = trimmed.strip_suffix("/api/chat") {
        return stripped.to_string();
    }
    if let Some(stripped) = trimmed.strip_suffix("/v1/chat/completions") {
        return stripped.to_string();
    }
    if let Some(stripped) = trimmed.strip_suffix("/v1") {
        return stripped.to_string();
    }
    trimmed.to_string()
}

pub fn static_models(provider: &str) -> Vec<String> {
    let models: &[&str] = match provider {
        "openai" => &[
            "gpt-4o-mini",
            "gpt-4o",
            "gpt-4-turbo",
            "gpt-4",
            "o1-mini",
            "o1-preview",
            "o3-mini",
        ],
        "anthropic" => &[
            "claude-sonnet-4-20250514",
            "claude-opus-4-20250514",
            "claude-3-7-sonnet-20250219",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
        ],
        "gemini" => &[
            "gemini-1.5-flash",
            "gemini-1.5-pro",
            "gemini-1.0-pro",
            "gemini-2.0-flash",
        ],
        "deepseek" => &["deepseek-chat", "deepseek-reasoner"],
        "openrouter" => &[
            "openai/gpt-4o-mini",
            "openai/gpt-4o",
            "anthropic/claude-sonnet-4",
        ],
        "ollama" | "mlx" => &[],
        _ => &[],
    };
    models.iter().map(|model| model.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_fresh_within_ttl() {
        let now = now_ms();
        let cache = ModelCache {
            timestamp: now - 1000,
            models: BTreeMap::new(),
        };
        assert!(cache_is_fresh(&cache, now));
        let stale = ModelCache {
            timestamp: now - CACHE_TTL_MS - 1,
            models: BTreeMap::new(),
        };
        assert!(!cache_is_fresh(&stale, now));
    }

    #[test]
    fn static_models_cover_required_providers() {
        assert!(static_models("openai").contains(&"gpt-4o-mini".to_string()));
        assert!(static_models("anthropic")
            .iter()
            .any(|m| m.contains("claude")));
        assert!(static_models("gemini").iter().any(|m| m.contains("gemini")));
    }
}
