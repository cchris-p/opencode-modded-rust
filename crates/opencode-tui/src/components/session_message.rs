use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratatui::{
    style::Color,
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::markdown::MarkdownRenderer;
use crate::context::{Message, MessagePart};
use crate::theme::Theme;

/// Render a user message with shared left gutter shape.
pub fn render_user_message(
    msg: &Message,
    theme: &Theme,
    show_timestamps: bool,
    agent: Option<&str>,
    queued: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let border_char = "  ";
    let border_style = Style::default().fg(user_border_color_for_agent(agent, theme));

    if msg.parts.is_empty() {
        for line_text in msg.content.lines() {
            lines.push(Line::from(vec![
                Span::styled(border_char, border_style),
                Span::styled(line_text.to_string(), Style::default().fg(theme.text)),
            ]));
        }
    } else {
        for part in &msg.parts {
            match part {
                MessagePart::Text { text } => {
                    let md_renderer = MarkdownRenderer::new(theme.clone());
                    let md_lines = md_renderer.to_lines(text);
                    for md_line in md_lines {
                        let mut spans = vec![Span::styled(border_char, border_style)];
                        spans.extend(md_line.spans);
                        lines.push(Line::from(spans));
                    }
                }
                MessagePart::File { path, mime } => {
                    lines.push(Line::from(vec![
                        Span::styled(border_char, border_style),
                        Span::styled(
                            mime_badge(mime),
                            Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(path.clone(), Style::default().fg(theme.text)),
                    ]));
                }
                MessagePart::Image { url } => {
                    lines.push(Line::from(vec![
                        Span::styled(border_char, border_style),
                        Span::styled("[image] ", Style::default().fg(theme.info)),
                        Span::styled(url.clone(), Style::default().fg(theme.text_muted)),
                    ]));
                }
                _ => {}
            }
        }
    }

    if queued {
        // Vanilla replaces the timestamp row with a ` QUEUED ` badge whenever
        // the message is queued, independent of the timestamps setting.
        let badge_bg = user_border_color_for_agent(agent, theme);
        let badge_style = Style::default()
            .bg(badge_bg)
            .fg(theme.selected_foreground(Some(badge_bg)))
            .add_modifier(Modifier::BOLD);
        if !lines.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(border_char, border_style),
                Span::styled(" QUEUED ", badge_style),
            ]));
        }
    } else if show_timestamps {
        let ts = msg.created_at.format("%H:%M").to_string();
        if !lines.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(border_char, border_style),
                Span::styled(ts, Style::default().fg(theme.text_muted)),
            ]));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ratatui::style::Modifier;

    use super::render_user_message;
    use crate::context::{Message, MessageRole, TokenUsage};
    use crate::theme::Theme;

    #[test]
    fn user_message_uses_padding_without_selectable_gutter() {
        let theme = Theme::default();
        let message = Message {
            id: "msg-1".to_string(),
            role: MessageRole::User,
            content: "I have enough".to_string(),
            created_at: Utc::now(),
            agent: None,
            model: None,
            mode: None,
            finish: None,
            error: None,
            completed_at: None,
            cost: 0.0,
            tokens: TokenUsage::default(),
            parts: Vec::new(),
        };

        let lines = render_user_message(&message, &theme, false, None, false);

        let first_span = lines
            .first()
            .and_then(|line| line.spans.first())
            .expect("rendered message should have a leading padding span");
        assert_eq!(first_span.content.as_ref(), "  ");
        assert!(!first_span.content.contains('┃'));
    }

    #[test]
    fn queued_user_message_renders_badge_instead_of_timestamp() {
        let theme = Theme::default();
        let message = Message {
            id: "msg-1".to_string(),
            role: MessageRole::User,
            content: "queued work".to_string(),
            created_at: Utc::now(),
            agent: None,
            model: None,
            mode: None,
            finish: None,
            error: None,
            completed_at: None,
            cost: 0.0,
            tokens: TokenUsage::default(),
            parts: Vec::new(),
        };

        let lines = render_user_message(&message, &theme, true, None, true);
        let badge = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .find(|span| span.content.contains("QUEUED"))
            .expect("queued message should render a QUEUED badge");
        assert_eq!(badge.content.as_ref(), " QUEUED ");
        assert!(badge.style.add_modifier.contains(Modifier::BOLD));
        assert!(badge.style.bg.is_some());

        let timestamp_rows = lines.iter().filter(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains(':') && span.content.len() == 5)
        });
        assert_eq!(
            timestamp_rows.count(),
            0,
            "badge must replace the timestamp row"
        );
    }

    #[test]
    fn queued_badge_shows_even_without_timestamps() {
        let theme = Theme::default();
        let message = Message {
            id: "msg-1".to_string(),
            role: MessageRole::User,
            content: "queued work".to_string(),
            created_at: Utc::now(),
            agent: None,
            model: None,
            mode: None,
            finish: None,
            error: None,
            completed_at: None,
            cost: 0.0,
            tokens: TokenUsage::default(),
            parts: Vec::new(),
        };

        let lines = render_user_message(&message, &theme, false, None, true);
        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content == " QUEUED "));
    }
}

fn mime_badge(mime: &str) -> String {
    let short = if let Some(sub) = mime.strip_prefix("image/") {
        sub.to_uppercase()
    } else if let Some(sub) = mime.strip_prefix("text/") {
        sub.to_uppercase()
    } else if let Some(sub) = mime.strip_prefix("application/") {
        sub.to_uppercase()
    } else {
        mime.to_uppercase()
    };
    format!("[{}]", short)
}

fn user_border_color_for_agent(agent: Option<&str>, theme: &Theme) -> Color {
    let Some(agent) = agent else {
        return theme.primary;
    };
    if theme.agent_colors.is_empty() {
        return theme.primary;
    }
    let mut hasher = DefaultHasher::new();
    agent.hash(&mut hasher);
    let idx = (hasher.finish() as usize) % theme.agent_colors.len();
    theme.agent_colors[idx]
}
