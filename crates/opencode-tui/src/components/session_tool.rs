use std::collections::HashMap;

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use serde_json::Value;

use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolState {
    Pending,
    Running,
    Completed,
    Failed,
}

pub fn render_tool_run_summary(
    count: usize,
    state: ToolState,
    is_denied: bool,
    expanded: bool,
    theme: &Theme,
) -> Line<'static> {
    let bg = theme.background_panel;
    let (state_icon, icon_style, name_style) = styles_for_state(state, is_denied, theme);
    let label = if count == 1 {
        "1 tool call".to_string()
    } else {
        format!("{} tool calls", count)
    };
    let mut spans = vec![
        block_prefix(theme, bg),
        Span::styled(format!("{} ", state_icon), icon_style.bg(bg)),
        Span::styled("● ", icon_style.bg(bg)),
        Span::styled(label, name_style.bg(bg)),
    ];
    if is_denied {
        spans.push(Span::styled(
            "  denied",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD)
                .bg(bg),
        ));
    }
    spans.push(Span::styled(
        format!("  {}", if expanded { "▾" } else { "▸" }),
        Style::default().fg(theme.text_muted).bg(bg),
    ));
    Line::from(spans)
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

/// Returns true if this tool renders as a block.
///
/// FEAT-055: this is now the single inline-vs-block authority (the dead
/// `ToolRenderMode` classification was folded in). Bash/shell plus the
/// edit-style and todo tools always render blocks; everything else escalates
/// to block mode once its output exceeds the preview threshold.
fn is_block_tool(name: &str, result: Option<&(String, bool)>) -> bool {
    match normalize_tool_name(name).as_str() {
        "bash" | "shell" | "write" | "writefile" | "write_file" | "edit" | "editfile"
        | "edit_file" | "multiedit" | "apply_patch" | "applypatch" | "task" | "subagent"
        | "todowrite" | "todo_write" | "question" | "glob" | "grep" | "search" | "ripgrep"
        | "list" | "ls" | "listdir" | "list_dir" => return true,
        _ => {}
    }
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
        let parsed = serde_json::from_str::<Value>(arguments.trim()).ok();
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
            let parsed = serde_json::from_str::<Value>(arguments.trim()).ok();
            if let Some(argument_preview) =
                tool_argument_preview(&normalized, parsed.as_ref(), arguments)
            {
                main_spans.push(Span::styled(
                    format!("  {}", argument_preview),
                    Style::default().fg(theme.text_muted).bg(bg),
                ));
            }
            if matches!(
                normalized.as_str(),
                "glob" | "grep" | "search" | "ripgrep" | "list" | "ls"
            ) {
                if let Some((result_text, _)) = result {
                    let count = result_text
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count();
                    main_spans.push(Span::styled(
                        format!("  ({} matches)", count),
                        Style::default().fg(theme.text_muted).bg(bg),
                    ));
                }
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

        // Purpose-built content derived from the call arguments, ported from the
        // previously-unused per-tool views.
        if show_tool_details {
            lines.extend(tool_detail_lines(&normalized, parsed.as_ref(), theme, bg));
        }

        if let Some((result_text, is_error)) = result {
            if *is_error {
                let mut iter = result_text.lines();
                if let Some(first_line) = iter.next() {
                    lines.push(block_content_line(
                        format!("Error: {}", first_line.trim()),
                        Style::default().fg(theme.error),
                        theme,
                        bg,
                    ));
                }

                if expanded {
                    for line in iter {
                        lines.push(block_content_line(
                            line.to_string(),
                            Style::default().fg(theme.error),
                            theme,
                            bg,
                        ));
                    }
                } else if show_tool_details {
                    for line in iter.take(2) {
                        lines.push(block_content_line(
                            line.to_string(),
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
                        line.to_string(),
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
        let parsed = serde_json::from_str::<Value>(arguments.trim()).ok();
        if let Some(argument_preview) =
            tool_argument_preview(&normalized, parsed.as_ref(), arguments)
        {
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
        "  ",
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

fn tool_argument_preview(
    normalized_name: &str,
    parsed: Option<&Value>,
    arguments: &str,
) -> Option<String> {
    let raw = arguments.trim();

    if normalized_name == "bash" || normalized_name == "shell" {
        let command = parsed
            .and_then(extract_shell_command)
            .or_else(|| (!raw.is_empty()).then_some(raw.to_string()))?;
        return Some(format!("$ {}", command.trim()));
    }

    if matches!(normalized_name, "read" | "readfile" | "read_file") {
        if let Some(path) = parsed.and_then(extract_path) {
            return Some(match read_line_range(parsed) {
                Some((start, end)) => format!("→ {} (lines {}-{})", path, start, end),
                None => format!("→ {}", path),
            });
        }
    }

    if matches!(
        normalized_name,
        "write" | "writefile" | "write_file" | "edit" | "editfile" | "edit_file" | "multiedit"
    ) {
        if let Some(path) = parsed.and_then(extract_path) {
            return Some(format!("← {}", path));
        }
    }

    if matches!(normalized_name, "glob" | "grep" | "search" | "ripgrep") {
        if let Some(pattern) = parsed
            .and_then(|value| value.get("pattern"))
            .and_then(Value::as_str)
        {
            return Some(
                match parsed
                    .and_then(|value| value.get("path"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                {
                    Some(path) => format!("{} in {}", pattern, path),
                    None => pattern.to_string(),
                },
            );
        }
    }

    if matches!(normalized_name, "list" | "ls" | "listdir" | "list_dir") {
        if let Some(path) = parsed.and_then(extract_path) {
            return Some(path);
        }
    }

    if matches!(normalized_name, "webfetch" | "web_fetch" | "fetch") {
        if let Some(url) = parsed
            .and_then(|value| value.get("url"))
            .and_then(Value::as_str)
        {
            return Some(url.to_string());
        }
    }

    if matches!(normalized_name, "websearch" | "web_search") {
        if let Some(query) = parsed
            .and_then(|value| value.get("query"))
            .and_then(Value::as_str)
        {
            return Some(query.to_string());
        }
    }

    if normalized_name == "skill" {
        if let Some(skill) = parsed
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
        {
            return Some(skill.to_string());
        }
    }

    if matches!(normalized_name, "task" | "subagent") {
        if let Some(description) = parsed
            .and_then(|value| value.get("description").or_else(|| value.get("prompt")))
            .and_then(Value::as_str)
        {
            return Some(collapse_whitespace(description));
        }
    }

    if raw.is_empty() {
        return None;
    }

    let first = raw.lines().next().unwrap_or(raw).trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

/// Purpose-built content lines for tools whose dead views showed more than the
/// generic argument preview (FEAT-055).
fn tool_detail_lines(
    normalized: &str,
    parsed: Option<&Value>,
    theme: &Theme,
    bg: ratatui::style::Color,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    match normalized {
        "write" | "writefile" | "write_file" => {
            if let Some(content) = parsed
                .and_then(|value| value.get("content"))
                .and_then(Value::as_str)
            {
                for line in content.lines().take(6) {
                    out.push(block_content_line(
                        format!("+ {}", line),
                        Style::default().fg(theme.success),
                        theme,
                        bg,
                    ));
                }
            }
        }
        "edit" | "editfile" | "edit_file" | "multiedit" => {
            if let Some(old) = parsed
                .and_then(|value| value.get("old_string"))
                .and_then(Value::as_str)
            {
                for line in old.lines().take(4) {
                    out.push(block_content_line(
                        format!("- {}", line),
                        Style::default().fg(theme.error),
                        theme,
                        bg,
                    ));
                }
            }
            if let Some(new) = parsed
                .and_then(|value| value.get("new_string"))
                .and_then(Value::as_str)
            {
                for line in new.lines().take(4) {
                    out.push(block_content_line(
                        format!("+ {}", line),
                        Style::default().fg(theme.success),
                        theme,
                        bg,
                    ));
                }
            }
        }
        "apply_patch" | "applypatch" => {
            if let Some(patch) = parsed
                .and_then(|value| value.get("patchText").or_else(|| value.get("patch")))
                .and_then(Value::as_str)
            {
                let mut files: Vec<&str> = patch
                    .lines()
                    .filter_map(|line| {
                        line.strip_prefix("+++ b/")
                            .or_else(|| line.strip_prefix("+++ "))
                            .map(str::trim)
                    })
                    .filter(|file| !file.is_empty() && *file != "/dev/null")
                    .collect();
                files.dedup();
                if !files.is_empty() {
                    out.push(block_content_line(
                        format!("({} files)", files.len()),
                        Style::default().fg(theme.text_muted),
                        theme,
                        bg,
                    ));
                    for file in files.iter().take(8) {
                        out.push(block_content_line(
                            (*file).to_string(),
                            Style::default().fg(theme.info),
                            theme,
                            bg,
                        ));
                    }
                }
            }
        }
        "todowrite" | "todo_write" => {
            if let Some(todos) = parsed
                .and_then(|value| value.get("todos"))
                .and_then(Value::as_array)
            {
                for item in todos {
                    let content = item
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    if content.is_empty() {
                        continue;
                    }
                    let status = item
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending");
                    let (icon, color) = match status {
                        "completed" => ("●", theme.success),
                        "in_progress" => ("◐", theme.warning),
                        _ => ("○", theme.text_muted),
                    };
                    out.push(Line::from(vec![
                        block_prefix(theme, bg),
                        Span::styled(format!("  {} ", icon), Style::default().fg(color).bg(bg)),
                        Span::styled(content.to_string(), Style::default().fg(theme.text).bg(bg)),
                    ]));
                }
            }
        }
        _ => {}
    }
    out
}

fn read_line_range(parsed: Option<&Value>) -> Option<(usize, usize)> {
    let value = parsed?;
    let offset = value.get("offset").and_then(Value::as_u64);
    let limit = value.get("limit").and_then(Value::as_u64);
    match (offset, limit) {
        (None, None) => None,
        (Some(offset), Some(limit)) => {
            let start = offset.max(1);
            let end = start.saturating_add(limit.saturating_sub(1));
            Some((start as usize, end as usize))
        }
        (Some(offset), None) => {
            let start = offset.max(1);
            Some((start as usize, start as usize))
        }
        (None, Some(limit)) => Some((1, limit.max(1) as usize)),
    }
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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

pub(crate) fn is_denied_result(result_text: &str) -> bool {
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

    #[test]
    fn todowrite_renders_status_icons_and_content() {
        let args = r#"{"todos":[{"content":"first thing","status":"completed"},{"content":"second thing","status":"in_progress"}]}"#;
        let render = render_tool_call(
            "call-todo",
            "todowrite",
            args,
            ToolState::Running,
            &HashMap::new(),
            true,
            false,
            &Theme::default(),
        );
        let body = body(&render);
        assert!(body.contains("first thing"));
        assert!(body.contains("second thing"));
        assert!(body.contains('●'));
        assert!(body.contains('◐'));
    }

    #[test]
    fn edit_renders_old_and_new_lines() {
        let args =
            r#"{"file_path":"/tmp/a.rs","old_string":"let x = 1;","new_string":"let x = 2;"}"#;
        let render = render_tool_call(
            "call-edit",
            "edit",
            args,
            ToolState::Completed,
            &HashMap::new(),
            true,
            false,
            &Theme::default(),
        );
        let body = body(&render);
        assert!(body.contains("- let x = 1;"));
        assert!(body.contains("+ let x = 2;"));
    }

    #[test]
    fn glob_reports_match_count_and_pattern() {
        let args = r#"{"pattern":"*.rs"}"#;
        let results = result_map("call-glob", "a.rs\nb.rs\nc.rs", false);
        let render = render_tool_call(
            "call-glob",
            "glob",
            args,
            ToolState::Completed,
            &results,
            true,
            false,
            &Theme::default(),
        );
        let body = body(&render);
        assert!(body.contains("*.rs"));
        assert!(body.contains("(3 matches)"));
    }

    #[test]
    fn long_output_lines_are_not_truncated_at_96_columns() {
        let id = "call-wide";
        let long = "x".repeat(140);
        let results = result_map(id, &format!("{long}\n{long}\n{long}\n{long}"), false);
        let render = render_tool_call(
            id,
            "bash",
            "{\"command\":\"echo\"}",
            ToolState::Completed,
            &results,
            true,
            true,
            &Theme::default(),
        );
        assert!(body(&render).contains(&long));
    }
}
