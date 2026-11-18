//! Provider request headers shared by every chat entry point.

use std::collections::HashMap;

use serde_json::Value;

/// OpenCode Go routes requests by conversation and rejects requests that omit
/// `x-opencode-session` with `400 MissingSessionID`. The free `opencode`
/// provider uses the same catalog family, so the header is attached to every
/// `opencode*` provider id (matching vanilla OpenCode's `providerID.startsWith`).
pub fn is_opencode_provider(provider_id: &str) -> bool {
    provider_id.starts_with("opencode")
}

fn user_agent() -> String {
    format!("opencode-modded-rust/{}", env!("CARGO_PKG_VERSION"))
}

/// Session headers vanilla OpenCode sends for a provider request.
pub fn session_headers(provider_id: &str, session_id: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if !is_opencode_provider(provider_id) || session_id.is_empty() {
        return headers;
    }

    headers.insert("x-opencode-session".to_string(), session_id.to_string());
    headers.insert("User-Agent".to_string(), user_agent());
    headers
}

/// Merge session headers into `provider_options["headers"]` without replacing
/// values a caller already set (for example plugin-supplied headers).
pub fn inject_session_headers(
    provider_options: &mut HashMap<String, Value>,
    provider_id: &str,
    session_id: &str,
) {
    let headers = session_headers(provider_id, session_id);
    if headers.is_empty() {
        return;
    }

    let entry = provider_options
        .entry("headers".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Some(map) = entry.as_object_mut() else {
        return;
    };

    for (name, value) in headers {
        map.entry(name).or_insert_with(|| Value::String(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_provider_gets_session_header() {
        let headers = session_headers("opencode-go", "ses_123");
        assert_eq!(
            headers.get("x-opencode-session").map(String::as_str),
            Some("ses_123")
        );
        assert!(headers.get("User-Agent").is_some());
    }

    #[test]
    fn other_providers_are_unchanged() {
        assert!(session_headers("deepseek", "ses_123").is_empty());
        assert!(session_headers("openai", "ses_123").is_empty());
    }

    #[test]
    fn inject_preserves_existing_headers() {
        let mut options = HashMap::from([(
            "headers".to_string(),
            serde_json::json!({"x-opencode-session": "plugin"}),
        )]);

        inject_session_headers(&mut options, "opencode-go", "ses_123");

        assert_eq!(
            options["headers"]["x-opencode-session"],
            serde_json::json!("plugin")
        );
        assert!(options["headers"]["User-Agent"].is_string());
    }
}
