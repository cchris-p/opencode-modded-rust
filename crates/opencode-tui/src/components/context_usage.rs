use crate::context::{Message, MessageRole, ProviderInfo};

/// Token/percentage summary for the current context window.
///
/// Mirrors vanilla's `usage()` memos: the token total is taken from the last
/// assistant message with output tokens (not a sum over the session), and the
/// percentage is computed against that message's model context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextUsage {
    pub tokens: u64,
    pub percent: Option<u64>,
}

pub(crate) fn last_assistant_message(messages: &[Message]) -> Option<&Message> {
    messages
        .iter()
        .rev()
        .find(|m| matches!(m.role, MessageRole::Assistant) && m.tokens.output > 0)
}

pub(crate) fn message_tokens(message: &Message) -> u64 {
    let t = &message.tokens;
    t.input + t.output + t.reasoning + t.cache_read + t.cache_write
}

pub(crate) fn total_cost(messages: &[Message]) -> f64 {
    messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::Assistant))
        .map(|m| m.cost)
        .sum()
}

pub(crate) fn model_context_limit(providers: &[ProviderInfo], model_id: Option<&str>) -> u64 {
    let Some(model_id) = model_id else {
        return 0;
    };
    providers
        .iter()
        .find_map(|provider| {
            provider
                .models
                .iter()
                .find(|model| {
                    model.id == model_id
                        || model
                            .id
                            .rsplit_once('/')
                            .map(|(_, suffix)| suffix == model_id)
                            .unwrap_or(false)
                })
                .map(|model| model.context_window)
        })
        .unwrap_or(0)
}

pub(crate) fn context_usage(
    messages: &[Message],
    providers: &[ProviderInfo],
    fallback_model: Option<&str>,
) -> Option<ContextUsage> {
    let last = last_assistant_message(messages)?;
    let tokens = message_tokens(last);
    if tokens == 0 {
        return None;
    }
    let limit = model_context_limit(providers, last.model.as_deref().or(fallback_model));
    let percent = (limit > 0).then(|| ((tokens as f64 / limit as f64) * 100.0).round() as u64);
    Some(ContextUsage { tokens, percent })
}

/// Vanilla `Locale.number`: `1200 -> "1.2K"`, `1000000 -> "1.0M"`.
pub(crate) fn format_compact_number(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

/// Vanilla `Number#toLocaleString` grouping used by the sidebar: `12345 -> "12,345"`.
pub(crate) fn format_grouped_number(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + (digits.len() / 3));
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Vanilla prompt/subagent footer text: `1.2K (56%)`.
pub(crate) fn format_context(usage: &ContextUsage) -> String {
    match usage.percent {
        Some(percent) => format!("{} ({}%)", format_compact_number(usage.tokens), percent),
        None => format_compact_number(usage.tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{MessageRole, TokenUsage};
    use chrono::Utc;

    fn message(role: MessageRole, output: u64, input: u64) -> Message {
        Message {
            id: "m".to_string(),
            role,
            content: String::new(),
            created_at: Utc::now(),
            agent: None,
            model: Some("provider/model".to_string()),
            mode: None,
            finish: None,
            error: None,
            completed_at: None,
            cost: 0.0,
            tokens: TokenUsage {
                input,
                output,
                reasoning: 0,
                cache_read: 0,
                cache_write: 0,
            },
            parts: Vec::new(),
            queued: false,
        }
    }

    fn provider(context_window: u64) -> ProviderInfo {
        ProviderInfo {
            id: "provider".to_string(),
            name: "Provider".to_string(),
            models: vec![crate::context::ModelInfo {
                id: "provider/model".to_string(),
                name: "Model".to_string(),
                context_window,
                max_output_tokens: 0,
                supports_vision: false,
                supports_tools: true,
            }],
        }
    }

    #[test]
    fn uses_last_assistant_message_not_session_sum() {
        let messages = vec![
            message(MessageRole::Assistant, 500, 500),
            message(MessageRole::User, 0, 0),
            message(MessageRole::Assistant, 100, 1900),
        ];
        let usage = context_usage(&messages, &[provider(10_000)], None).expect("usage");
        assert_eq!(usage.tokens, 2000);
        assert_eq!(usage.percent, Some(20));
    }

    #[test]
    fn ignores_assistant_messages_without_output() {
        let messages = vec![
            message(MessageRole::Assistant, 100, 900),
            message(MessageRole::Assistant, 0, 0),
        ];
        let usage = context_usage(&messages, &[provider(10_000)], None).expect("usage");
        assert_eq!(usage.tokens, 1000);
    }

    #[test]
    fn matches_vanilla_compact_format() {
        assert_eq!(format_compact_number(999), "999");
        assert_eq!(format_compact_number(1000), "1.0K");
        assert_eq!(format_compact_number(1234), "1.2K");
        assert_eq!(format_compact_number(1_000_000), "1.0M");
    }

    #[test]
    fn matches_vanilla_grouped_format() {
        assert_eq!(format_grouped_number(0), "0");
        assert_eq!(format_grouped_number(999), "999");
        assert_eq!(format_grouped_number(1234), "1,234");
        assert_eq!(format_grouped_number(12_345_678), "12,345,678");
    }
}
