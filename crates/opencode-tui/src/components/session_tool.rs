use std::collections::HashMap;

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use serde_json::Value;

use crate::theme::Theme;

#[derive(Clone, Copy)]
pub enum ToolState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Threshold: tool results longer than this are "block" tools with expandable output
const BLOCK_RESULT_THRESHOLD: usize = 3;

/// Map tool name to a semantic glyph
pub fn tool_glyph(name: &str) -> &'static str {
    match name {
        "bash" | "shell" => "$",
        "read" | "readFile" | "read_file" => "→",
        "write" | "writeFile" | "write_file" => "←",
        "edit" | "editFile" | "edit_file" => "←",
        "glob" | "grep" | "search" | "ripgrep" => "✱",
        "list" | "ls" | "listDir" | "list_dir" => "→",
        "webfetch" | "web_fetch" | "fetch" => "%",
        "codesearch" | "code_search" => "◇",
        "websearch" | "web_search" => "◈",
        "task" | "subagent" => "#",
        "apply_patch" | "applyPatch" => "%",
        "todowrite" | "todo_write" | "todoRead" | "todo_read" => "☐",
        _ => "⚙",
    }
}

/// Returns true if this tool typically produces block-level output
fn is_block_tool(name: &str, result: Option<&(String, bool)>) -> bool {
    let normalized = normalize_tool_name(name);
    // Tools that always produce block output
    match normalized.as_str() {
        "bash" | "shell" | "apply_patch" => return true,
        _ => {}
    }
    // Otherwise, check result length
    if let Some((result_text, _)) = result {
        result_text.lines().count() > BLOCK_RESULT_THRESHOLD
    } else {
        false
    }
}

/// Result of rendering a tool call: the lines plus whether its output can be
/// collapsed/expanded by clicking the block.
pub struct ToolCallRender {
    pub lines: Vec<Line<'static>>,
    pub collapsible: bool,
}

/// Render a single tool call as lines (inline or block style).
///
/// `expanded` only affects block-style calls with output beyond the collapsed
/// preview. Collapsed output keeps the historical preview behavior; expanded
/// output shows every captured result line.
pub fn render_tool_call(
    id: &str,
    name: &str,
    arguments: &str,
    state: ToolState,
    tool_results: &HashMap<String, (String, bool)>,
    show_tool_details: bool,
    expanded: bool,
    theme: &Theme,
) -> ToolCallRender {
    if matches!(state, ToolState::Completed) && !show_tool_details {
        return ToolCallRender {
            lines: Vec::new(),
            collapsible: false,
        };
    }

    let result = tool_results.get(id);
    let block_mode = is_block_tool(name, result);
    let normalized = normalize_tool_name(name);

    let glyph = tool_glyph(name);
    let is_denied =
        result.is_some_and(|(result_text, is_error)| *is_error && is_denied_result(result_text));

    let (state_icon, icon_style, name_style) = styles_for_state(state, is_denied, theme);

    let mut lines = Vec::new();

    if block_mode {
        let bg = theme.background_panel;
        let preview_limit = if normalized == "bash" || normalized == "shell" {
            10usize
        } else {
            6usize
        };
        let is_error_result = result.is_some_and(|(_, is_error)| *is_error);
        let total_output_lines = result.map_or(0, |(result_text, _)| result_text.lines().count());
        let collapsed_limit = if is_error_result {
            if show_tool_details {
                3
            } else {
                1
            }
        } else {
            preview_limit
        };
        let collapsible = if is_error_result {
            total_output_lines > collapsed_limit
        } else {
            show_tool_details && total_output_lines > preview_limit
        };
        let hidden_lines = total_output_lines.saturating_sub(collapsed_limit);

        let mut main_spans = vec![
            block_prefix(theme, bg),
            Span::styled(format!("{} ", state_icon), icon_style.bg(bg)),
        ];

        if normalized == "bash" || normalized == "shell" {
            main_spans.push(Span::styled("$ ", icon_style.bg(bg)));
            match shell_command_text(arguments) {
                Some(command) => main_spans.push(Span::styled(command, name_style.bg(bg))),
                None => main_spans.push(Span::styled(name.to_string(), name_style.bg(bg))),
            }
        } else {
            main_spans.push(Span::styled(format!("{} ", glyph), icon_style.bg(bg)));
            main_spans.push(Span::styled(name.to_string(), name_style.bg(bg)));
            if let Some(argument_preview) = tool_argument_preview(&normalized, arguments) {
                main_spans.push(Span::styled(
                    format!("  {}", argument_preview),
                    Style::default().fg(theme.text_muted).bg(bg),
                ));
            }
        }

        if is_denied {
            main_spans.push(Span::styled(
                "  denied",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD)
                    .bg(bg),
            ));
        }

        if collapsible {
            main_spans.push(Span::styled(
                format!("  {}", if expanded { "▾" } else { "▸" }),
                Style::default().fg(theme.text_muted).bg(bg),
            ));
        }

        lines.push(Line::from(main_spans));

        if let Some((result_text, is_error)) = result {
            if *is_error {
                let mut iter = result_text.lines();
                if let Some(first_line) = iter.next() {
                    lines.push(block_content_line(
                        format!("Error: {}", format_preview_line(first_line, 96)),
                        Style::default().fg(theme.error),
                        theme,
                        bg,
                    ));
                }

                if expanded {
                    for line in iter {
                        lines.push(block_content_line(
                            format_preview_line(line, 96),
                            Style::default().fg(theme.error),
                            theme,
                            bg,
                        ));
                    }
                } else if show_tool_details {
                    for line in iter.take(2) {
                        lines.push(block_content_line(
                            format_preview_line(line, 96),
                            Style::default().fg(theme.error),
                            theme,
                            bg,
                        ));
                    }
                }
            } else if show_tool_details {
                let output_lines = result_text.lines().collect::<Vec<_>>();
                let line_count = output_lines.len();

                lines.push(block_content_line(
                    format!("({} lines of output)", line_count),
                    Style::default().fg(theme.text_muted),
                    theme,
                    bg,
                ));

                let visible = if expanded {
                    line_count
                } else {
                    preview_limit.min(line_count)
                };
                for line in output_lines.iter().take(visible) {
                    lines.push(block_content_line(
                        format_preview_line(line, 96),
                        Style::default().fg(theme.text),
                        theme,
                        bg,
                    ));
                }
            }
        }

        if collapsible {
            let affordance = if expanded {
                "▾ click to collapse".to_string()
            } else {
                format!("▸ {} more lines — click to expand", hidden_lines)
            };
            lines.push(block_content_line(
                affordance,
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                theme,
                bg,
            ));
        }

        return ToolCallRender { lines, collapsible };
    }

    // Inline mode
    let mut main_spans = vec![
        Span::styled(format!("{} ", state_icon), icon_style),
        Span::styled(format!("{} ", glyph), Style::default().fg(theme.tool_icon)),
        Span::styled(name.to_string(), name_style),
    ];

    // Inline result summary for completed non-block tools
    if let Some((result_text, is_error)) = result {
        if *is_error {
            let first_line = result_text.lines().next().unwrap_or(result_text).trim();
            main_spans.push(Span::styled(
                format!(" — {}", format_preview_line(first_line, 96)),
                Style::default().fg(theme.error),
            ));
            if is_denied {
                main_spans.push(Span::styled(
                    " (denied)",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        } else {
            let line_count = result_text.lines().count();
            if line_count <= 1 {
                let summary = result_text.trim();
                if !summary.is_empty() && summary.len() <= 80 {
                    main_spans.push(Span::styled(
                        format!(" — {}", summary),
                        Style::default().fg(theme.text_muted),
                    ));
                }
            }
        }
    }

    lines.push(Line::from(main_spans));

    if show_tool_details {
        if let Some(argument_preview) = tool_argument_preview(&normalized, arguments) {
            lines.push(Line::from(Span::styled(
                format!("    {}", argument_preview),
                Style::default().fg(theme.text_muted),
            )));
        }
    }

    ToolCallRender {
        lines,
        collapsible: false,
    }
}

/// Extract the raw shell command text from a bash/shell call's arguments,
/// without any display prefix.
fn shell_command_text(arguments: &str) -> Option<String> {
    let raw = arguments.trim();
    let parsed = serde_json::from_str::<Value>(raw).ok();
    parsed
        .as_ref()
        .and_then(extract_shell_command)
        .or_else(|| (!raw.is_empty()).then_some(raw.to_string()))
}

fn block_prefix(theme: &Theme, background: ratatui::style::Color) -> Span<'static> {
    Span::styled(
        "│ ",
        Style::default().fg(theme.border_subtle).bg(background),
    )
}

fn block_content_line(
    content: impl Into<String>,
    style: Style,
    theme: &Theme,
    background: ratatui::style::Color,
) -> Line<'static> {
    Line::from(vec![
        block_prefix(theme, background),
        Span::styled(format!("  {}", content.into()), style.bg(background)),
    ])
}

fn styles_for_state(
    state: ToolState,
    is_denied: bool,
    theme: &Theme,
) -> (&'static str, Style, Style) {
    match state {
        ToolState::Pending => (
            "◯",
            Style::default().fg(theme.warning),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        ToolState::Running => (
            "◐",
            Style::default().fg(theme.warning),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        ToolState::Completed => (
            "●",
            Style::default().fg(theme.success),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        ToolState::Failed => {
            let mut name_style = Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD);
            if is_denied {
                name_style = name_style.add_modifier(Modifier::CROSSED_OUT);
            }
            ("✗", Style::default().fg(theme.error), name_style)
        }
    }
}

fn normalize_tool_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace('-', "_")
}

fn tool_argument_preview(normalized_name: &str, arguments: &str) -> Option<String> {
    let raw = arguments.trim();
    let parsed = serde_json::from_str::<Value>(raw).ok();

    if normalized_name == "bash" || normalized_name == "shell" {
        let command = parsed
            .as_ref()
            .and_then(extract_shell_command)
            .or_else(|| (!raw.is_empty()).then_some(raw.to_string()))?;
        return Some(format!("$ {}", command.trim()));
    }

    if matches!(normalized_name, "read" | "readfile" | "read_file") {
        if let Some(path) = parsed.as_ref().and_then(extract_path) {
            return Some(format!("→ {}", path));
        }
    }

    if matches!(
        normalized_name,
        "write" | "writefile" | "write_file" | "edit" | "editfile" | "edit_file"
    ) {
        if let Some(path) = parsed.as_ref().and_then(extract_path) {
            return Some(format!("← {}", path));
        }
    }

    if raw.is_empty() {
        return None;
    }

    let first = raw.lines().next().unwrap_or(raw).trim();
    if first.is_empty() {
        None
    } else {
        Some(format_preview_line(first, 84))
    }
}

fn extract_shell_command(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    for key in ["command", "cmd", "script", "input", "text"] {
        if let Some(command) = object.get(key).and_then(|v| v.as_str()) {
            let trimmed = command.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_path(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    for key in [
        "path",
        "file",
        "filename",
        "filepath",
        "target",
        "destination",
        "to",
        "from",
    ] {
        if let Some(path) = object.get(key).and_then(|v| v.as_str()) {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn is_denied_result(result_text: &str) -> bool {
    let lower = result_text.to_ascii_lowercase();
    lower.contains("permission denied")
        || lower.contains("denied")
        || lower.contains("not permitted")
        || lower.contains("forbidden")
}

fn format_preview_line(line: &str, max_chars: usize) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let truncated: String = trimmed.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn result_map(id: &str, text: &str, is_error: bool) -> HashMap<String, (String, bool)> {
        let mut map = HashMap::new();
        map.insert(id.to_string(), (text.to_string(), is_error));
        map
    }

    fn body(render: &ToolCallRender) -> String {
        render
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn rendered(
        id: &str,
        results: &HashMap<String, (String, bool)>,
        show_tool_details: bool,
        expanded: bool,
    ) -> ToolCallRender {
        render_tool_call(
            id,
            "bash",
            "{\"command\":\"ls\"}",
            ToolState::Completed,
            results,
            show_tool_details,
            expanded,
            &Theme::default(),
        )
    }

    #[test]
    fn long_output_is_collapsible_and_expands_to_every_line() {
        let id = "call-long";
        let output = (1..=25)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let results = result_map(id, &output, false);

        let collapsed = rendered(id, &results, true, false);
        assert!(collapsed.collapsible);
        let collapsed_body = body(&collapsed);
        assert!(collapsed_body.contains("(25 lines of output)"));
        assert!(collapsed_body.contains("▸ 15 more lines — click to expand"));
        assert!(!collapsed_body.contains("line 11"));

        let expanded = rendered(id, &results, true, true);
        assert!(expanded.collapsible);
        let expanded_body = body(&expanded);
        assert!(expanded_body.contains("line 25"));
        assert!(expanded_body.contains("▾ click to collapse"));
        assert!(!expanded_body.contains("more lines"));
    }

    #[test]
    fn output_within_preview_is_not_collapsible() {
        let id = "call-short";
        let output = (1..=4)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let results = result_map(id, &output, false);

        let render = rendered(id, &results, true, false);
        assert!(!render.collapsible);
        assert!(!body(&render).contains("click to expand"));
    }

    #[test]
    fn completed_call_renders_nothing_when_details_are_hidden() {
        let id = "call-hidden";
        let results = result_map(id, "one\ntwo\nthree\nfour", false);

        let render = rendered(id, &results, false, false);
        assert!(render.lines.is_empty());
        assert!(!render.collapsible);
    }

    #[test]
    fn error_output_expands_beyond_collapsed_preview() {
        let id = "call-error";
        let output = (1..=6)
            .map(|i| format!("error {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let results = result_map(id, &output, true);

        let collapsed = render_tool_call(
            id,
            "bash",
            "{\"command\":\"false\"}",
            ToolState::Failed,
            &results,
            true,
            false,
            &Theme::default(),
        );
        assert!(collapsed.collapsible);
        let collapsed_body = body(&collapsed);
        assert!(!collapsed_body.contains("error 6"));
        assert!(collapsed_body.contains("click to expand"));

        let expanded = render_tool_call(
            id,
            "bash",
            "{\"command\":\"false\"}",
            ToolState::Failed,
            &results,
            true,
            true,
            &Theme::default(),
        );
        assert!(body(&expanded).contains("error 6"));
    }
}
