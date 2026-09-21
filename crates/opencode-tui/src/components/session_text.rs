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
    use super::render_text_part;
    use crate::theme::Theme;

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
}

pub fn render_reasoning_part(
    text: &str,
    theme: &Theme,
    collapsed: bool,
    preview_lines: usize,
) -> ReasoningRender {
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
    let collapsible = total_content_lines > preview_lines;

    if collapsible && collapsed {
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

    let visible_count = if collapsible && collapsed {
        preview_lines
    } else {
        total_content_lines
    };

    // Render reasoning with concealed style and muted color.
    for line in content_lines.into_iter().take(visible_count) {
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
