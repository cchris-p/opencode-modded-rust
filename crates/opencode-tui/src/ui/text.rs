use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

pub fn truncate(text: &str, max_width: usize) -> String {
    let width = UnicodeWidthStr::width(text);
    if width <= max_width {
        return text.to_string();
    }

    let mut result = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_str = ch.to_string();
        let ch_width = UnicodeWidthStr::width(ch_str.as_str());
        if current_width + ch_width + 3 > max_width {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }

    result.push_str("...");
    result
}

pub fn pad_left(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width >= width {
        return text.to_string();
    }
    format!("{}{}", " ".repeat(width - text_width), text)
}

pub fn pad_right(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width >= width {
        return text.to_string();
    }
    format!("{}{}", text, " ".repeat(width - text_width))
}

pub fn center_text(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width >= width {
        return text.to_string();
    }
    let left_pad = (width - text_width) / 2;
    format!("{}{}", " ".repeat(left_pad), text)
}

pub fn highlight_text<'a>(text: &'a str, color: ratatui::style::Color) -> Span<'a> {
    Span::styled(text, ratatui::style::Style::default().fg(color))
}

/// Abbreviate a leading home-directory prefix to `~`.
///
/// The home directory itself becomes `~`; a path nested under it keeps its
/// remainder (`/Users/me/proj` -> `~/proj`). Paths outside the home directory
/// (or when the home directory cannot be resolved) are returned unchanged.
pub fn abbreviate_home(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let Some(home) = dirs::home_dir() else {
        return path.to_string();
    };
    let Ok(rest) = std::path::Path::new(path).strip_prefix(&home) else {
        return path.to_string();
    };
    let rest = rest.to_string_lossy();
    if rest.is_empty() {
        "~".to_string()
    } else {
        format!("~/{}", rest)
    }
}

/// Bottom-left footer location label: `<shortened path>:<branch>`.
///
/// The home-relative path is always shown; the branch suffix is appended only
/// when a non-empty branch is known, so non-git and detached-HEAD workspaces
/// render just the path (no dangling `:`).
pub fn workspace_location_label(path: &str, branch: Option<&str>) -> String {
    let path = abbreviate_home(path);
    match branch.map(str::trim).filter(|branch| !branch.is_empty()) {
        Some(branch) => format!("{path}:{branch}"),
        None => path,
    }
}

#[cfg(test)]
mod tests {
    use super::{abbreviate_home, workspace_location_label};

    #[test]
    fn abbreviate_home_rewrites_nested_paths_and_home_itself() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let home = home.to_string_lossy().to_string();
        assert_eq!(abbreviate_home(&home), "~");
        assert_eq!(abbreviate_home(&format!("{home}/proj")), "~/proj");
        assert_eq!(abbreviate_home(&format!("{home}/a/b")), "~/a/b");
    }

    #[test]
    fn abbreviate_home_leaves_unrelated_paths_untouched() {
        assert_eq!(abbreviate_home("/opt/other"), "/opt/other");
        assert_eq!(abbreviate_home(""), "");
    }

    #[test]
    fn workspace_label_appends_non_empty_branch_only() {
        let path = "/opt/other";
        assert_eq!(
            workspace_location_label(path, Some("development")),
            "/opt/other:development"
        );
        assert_eq!(workspace_location_label(path, None), "/opt/other");
        assert_eq!(workspace_location_label(path, Some("")), "/opt/other");
        assert_eq!(workspace_location_label(path, Some("  ")), "/opt/other");
    }
}
