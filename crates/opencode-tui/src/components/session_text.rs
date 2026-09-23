use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::markdown::MarkdownRenderer;
use crate::theme::Theme;

/// Render a text part using markdown rendering
pub fn render_text_part(text: &str, theme: &Theme, marker_color: Color) -> Vec<Line<'static>> {
    let renderer = MarkdownRenderer::new(theme.clone());
    let mut with_marker = Vec::new();
    for line in renderer.to_lines(text) {
        let mut spans = vec![Span::styled("  ", Style::default().fg(marker_color))];
        spans.extend(line.spans);
        with_marker.push(Line::from(spans));
    }
    with_marker
}

/// Render a reasoning/thinking part with muted styling and collapsible header.
pub struct ReasoningRender {
    pub lines: Vec<Line<'static>>,
    pub collapsible: bool,
}

#[cfg(test)]
mod tests {
    use super::{render_reasoning_part, render_text_part};
    use crate::theme::Theme;

    fn text_of(lines: &[ratatui::text::Line<'static>]) -> String {
        lines
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

    #[test]
    fn text_part_uses_padding_without_selectable_marker() {
        let theme = Theme::default();

        let lines = render_text_part("I have enough", &theme, theme.primary);

        let first_span = lines
            .first()
            .and_then(|line| line.spans.first())
            .expect("rendered text should have a leading padding span");
        assert_eq!(first_span.content.as_ref(), "  ");
        assert!(!first_span.content.contains('▸'));
    }

    #[test]
    fn shown_reasoning_emits_content_not_only_a_count() {
        let theme = Theme::default();

        let rendered = render_reasoning_part("First line\nSecond line", &theme, false);

        let body = text_of(&rendered.lines);
        assert!(rendered.collapsible);
        assert!(body.contains("First line"));
        assert!(body.contains("Second line"));
        assert!(!body.contains("lines)"));
    }

    #[test]
    fn collapsed_reasoning_shows_only_the_count_header() {
        let theme = Theme::default();

        let rendered = render_reasoning_part("First line\nSecond line", &theme, true);

        let body = text_of(&rendered.lines);
        assert!(body.contains("Thinking (2 lines)"));
        assert!(!body.contains("First line"));
    }
}

/// Render a reasoning part.
///
/// BUG-022: collapse is now an explicit, per-block user action. When
/// `collapsed` is false (the default whenever `/thinking` is on) the actual
/// reasoning content is emitted, including the in-progress streamed part.
/// `collapsed` renders only the `▶ Thinking (N lines)` header.
pub fn render_reasoning_part(text: &str, theme: &Theme, collapsed: bool) -> ReasoningRender {
    let cleaned = text.replace("[REDACTED]", "").trim().to_string();
    if cleaned.is_empty() {
        return ReasoningRender {
            lines: Vec::new(),
            collapsible: false,
        };
    }

    let mut lines = Vec::new();
    let renderer = MarkdownRenderer::new(theme.clone()).with_concealed(true);
    let content_lines = renderer.to_lines(&cleaned);
    let total_content_lines = content_lines.len();
    let collapsible = !content_lines.is_empty();

    if collapsed {
        lines.push(Line::from(Span::styled(
            format!("▶ Thinking ({} lines)", total_content_lines),
            Style::default()
                .fg(theme.text_muted)
                .add_modifier(Modifier::ITALIC),
        )));
        return ReasoningRender { lines, collapsible };
    }

    lines.push(Line::from(Span::styled(
        if collapsible {
            "▼ Thinking"
        } else {
            "Thinking"
        },
        Style::default()
            .fg(theme.text_muted)
            .add_modifier(Modifier::ITALIC),
    )));

    // Render reasoning with concealed style and muted color.
    for line in content_lines {
        let mut spans = vec![Span::styled("  ", Style::default().fg(theme.text_muted))];
        spans.extend(
            line.spans
                .into_iter()
                .map(|span| Span::styled(span.content, span.style.fg(theme.text_muted))),
        );
        lines.push(Line::from(spans));
    }

    if collapsible {
        lines.push(Line::from(Span::styled(
            "  [click to collapse]",
            Style::default().fg(theme.text_muted),
        )));
    }

    ReasoningRender { lines, collapsible }
}
