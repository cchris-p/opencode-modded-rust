/// Approximate token count.
///
/// Upstream OpenCommit uses tiktoken (`cl100k_base`). Pulling a tokenizer into
/// the native binary would add a large data dependency for a single budget
/// check, so we use a documented character heuristic: roughly four characters
/// per token. The budget only needs to be conservative; it is not used for
/// billing or exact context accounting.
pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    (text.chars().count() as u64).div_ceil(4)
}

/// Truncate `text` to at most `max_tokens` estimated tokens, once.
pub fn truncate_to_tokens(text: &str, max_tokens: u64) -> (String, bool) {
    if estimate_tokens(text) <= max_tokens {
        return (text.to_string(), false);
    }
    let max_chars = max_tokens.saturating_mul(4) as usize;
    let mut end = max_chars.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = text[..end].to_string();
    truncated.push_str("\n... [diff truncated to fit the token budget]");
    (truncated, true)
}

/// Remove reasoning content tags and common Markdown fences from model output.
pub fn clean_message(content: &str) -> String {
    let without_tags = remove_tag_blocks(content, "think");
    let without_tags = remove_tag_blocks(&without_tags, "thinking");
    let trimmed = without_tags.trim();
    let stripped = strip_code_fence(trimmed);
    let stripped = stripped.trim();
    let stripped = stripped.trim_matches('`').trim();
    collapse_spaces(stripped)
}

fn remove_tag_blocks(content: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + open.len()..];
        match after_open.find(&close) {
            Some(end) => rest = &after_open[end + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn strip_code_fence(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("```") {
        let rest = rest
            .strip_prefix(|c: char| c.is_ascii_alphabetic())
            .unwrap_or(rest);
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_alphabetic());
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim().to_string();
        }
    }
    content.to_string()
}

fn collapse_spaces(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut previous_space = false;
    for ch in content.chars() {
        if ch == ' ' || ch == '\t' {
            if !previous_space {
                out.push(' ');
            }
            previous_space = true;
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_roughly_four_chars_per_token() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn truncates_once_at_a_char_boundary() {
        let text = "abcdefghijklmnop";
        let (truncated, did_truncate) = truncate_to_tokens(text, 2);
        assert!(did_truncate);
        assert!(truncated.starts_with("abcdefgh"));
        assert!(truncated.ends_with("truncated to fit the token budget]"));
        assert!(!truncate_to_tokens("short", 100).1);
    }

    #[test]
    fn strips_reasoning_and_fences() {
        assert_eq!(
            clean_message("<think>secret</think>feat: add thing"),
            "feat: add thing"
        );
        assert_eq!(
            clean_message("```\nfeat: add thing\n```"),
            "feat: add thing"
        );
        assert_eq!(clean_message("`feat: add thing`"), "feat: add thing");
    }
}
