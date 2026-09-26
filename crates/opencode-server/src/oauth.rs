use std::collections::HashMap;
use std::sync::Arc;

use opencode_plugin::subprocess::PluginLoader;
use opencode_provider::{AuthError, AuthInfo, AuthManager, AuthMethodType, Authorization};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMethodInfo {
    #[serde(rename = "type")]
    pub method_type: String,
    pub label: String,
}

pub struct ProviderAuth {
    auth_manager: Arc<AuthManager>,
}

impl ProviderAuth {
    pub fn new(auth_manager: Arc<AuthManager>) -> Self {
        Self { auth_manager }
    }

    pub async fn methods(loader: &PluginLoader) -> HashMap<String, Vec<AuthMethodInfo>> {
        let bridges = loader.auth_bridges().await;
        bridges
            .iter()
            .map(|(provider, bridge)| {
                let methods = bridge
                    .methods()
                    .iter()
                    .map(|method| AuthMethodInfo {
                        method_type: method.method_type.clone(),
                        label: method.label.clone(),
                    })
                    .collect::<Vec<_>>();
                (provider.clone(), methods)
            })
            .collect()
    }

    pub async fn authorize(
        loader: &PluginLoader,
        provider_id: &str,
        method: usize,
        inputs: Option<HashMap<String, String>>,
    ) -> Result<Authorization, AuthError> {
        let bridge = loader
            .auth_bridge(provider_id)
            .await
            .ok_or_else(|| AuthError::OauthMissing(provider_id.to_string()))?;
        let result = bridge
            .authorize(method, inputs)
            .await
            .map_err(|error| AuthError::OauthAuthorizeFailed(error.to_string()))?;

        let method_type = match result.method.as_deref() {
            Some("code") => AuthMethodType::Code,
            _ => AuthMethodType::Auto,
        };

        Ok(Authorization {
            url: result.url.unwrap_or_default(),
            method: method_type,
            instructions: result.instructions.unwrap_or_default(),
        })
    }

    pub async fn callback(
        &self,
        loader: &PluginLoader,
        provider_id: &str,
        code: Option<&str>,
    ) -> Result<(), AuthError> {
        let bridge = loader
            .auth_bridge(provider_id)
            .await
            .ok_or_else(|| AuthError::OauthMissing(provider_id.to_string()))?;
        let result = bridge
            .callback(code)
            .await
            .map_err(|error| AuthError::OauthCallbackFailed(error.to_string()))?;

        let (target_provider, auth) = parse_callback_auth(provider_id, &result)?;
        self.auth_manager.set(&target_provider, auth).await;

        Ok(())
    }

    pub async fn set_api_key(&self, provider_id: &str, key: String) {
        self.auth_manager
            .set(provider_id, AuthInfo::Api { key })
            .await;
    }

    pub async fn remove(&self, provider_id: &str) {
        self.auth_manager.remove(provider_id).await;
    }
}

/// Validate a plugin `auth.callback` result and map it to persisted auth.
///
/// Extracted from [`ProviderAuth::callback`] so the validation and error paths
/// are unit testable without a live plugin host.
fn parse_callback_auth(
    provider_id: &str,
    result: &serde_json::Value,
) -> Result<(String, AuthInfo), AuthError> {
    let auth_type = result.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if auth_type != "success" {
        let detail = if auth_type.is_empty() {
            "plugin returned no auth result type".to_string()
        } else {
            format!("plugin returned auth result type '{auth_type}'")
        };
        return Err(AuthError::OauthCallbackFailed(detail));
    }

    // Plugin callback can override target provider (e.g. copilot enterprise).
    let target_provider = result
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or(provider_id)
        .to_string();

    if let Some(key) = result
        .get("key")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("apiKey").and_then(|v| v.as_str()))
        .or_else(|| result.get("token").and_then(|v| v.as_str()))
    {
        return Ok((
            target_provider,
            AuthInfo::Api {
                key: key.to_string(),
            },
        ));
    }

    let access = result
        .get("access")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let refresh = result
        .get("refresh")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if access.is_empty() && refresh.is_empty() {
        return Err(AuthError::OauthCallbackFailed(
            "plugin returned no access or refresh token".to_string(),
        ));
    }

    Ok((
        target_provider,
        AuthInfo::OAuth {
            access,
            refresh,
            expires: result.get("expires").and_then(|v| v.as_i64()),
            account_id: result
                .get("accountId")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            enterprise_url: result
                .get("enterpriseUrl")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_callback_reports_plugin_failure_type() {
        let error = parse_callback_auth("openai", &json!({ "type": "failed" })).unwrap_err();
        assert!(
            error.to_string().contains("'failed'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parse_callback_reports_missing_result_type() {
        let error = parse_callback_auth("openai", &json!({})).unwrap_err();
        assert!(
            error.to_string().contains("no auth result type"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parse_callback_reports_missing_tokens() {
        let error = parse_callback_auth("openai", &json!({ "type": "success" })).unwrap_err();
        assert!(
            error.to_string().contains("no access or refresh token"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parse_callback_maps_oauth_tokens() {
        let (provider, auth) = parse_callback_auth(
            "openai",
            &json!({
                "type": "success",
                "access": "access-token",
                "refresh": "refresh-token",
                "expires": 123,
                "accountId": "acct_1",
            }),
        )
        .unwrap();

        assert_eq!(provider, "openai");
        match auth {
            AuthInfo::OAuth {
                access,
                refresh,
                expires,
                account_id,
                ..
            } => {
                assert_eq!(access, "access-token");
                assert_eq!(refresh, "refresh-token");
                assert_eq!(expires, Some(123));
                assert_eq!(account_id.as_deref(), Some("acct_1"));
            }
            other => panic!("expected oauth auth, got {other:?}"),
        }
    }

    #[test]
    fn parse_callback_prefers_api_key_and_target_provider() {
        let (provider, auth) = parse_callback_auth(
            "openai",
            &json!({
                "type": "success",
                "provider": "github-copilot-enterprise",
                "key": "copilot-key",
            }),
        )
        .unwrap();

        assert_eq!(provider, "github-copilot-enterprise");
        assert!(matches!(auth, AuthInfo::Api { key } if key == "copilot-key"));
    }
}
