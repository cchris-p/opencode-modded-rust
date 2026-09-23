use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::context::TodoStatus;
use crate::theme::Theme;

pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
}

impl TodoItem {
    pub fn new(content: &str, status: TodoStatus) -> Self {
        Self {
            content: content.to_string(),
            status,
        }
    }

    pub fn status_icon_and_color(&self, theme: &Theme) -> (&'static str, Color) {
        match &self.status {
            TodoStatus::Pending => ("○", theme.text_muted),
            TodoStatus::InProgress => ("◐", theme.warning),
            TodoStatus::Completed => ("●", theme.success),
            TodoStatus::Cancelled => ("○", theme.text_muted),
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let (icon, color) = self.status_icon_and_color(theme);

        let line = Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(&self.content, Style::default().fg(theme.text)),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_colors_track_theme_tokens() {
        let mut theme = Theme::dark();
        theme.text_muted = Color::Rgb(1, 2, 3);
        theme.warning = Color::Rgb(4, 5, 6);
        theme.success = Color::Rgb(7, 8, 9);

        let pending = TodoItem::new("a", TodoStatus::Pending);
        let in_progress = TodoItem::new("b", TodoStatus::InProgress);
        let completed = TodoItem::new("c", TodoStatus::Completed);
        let cancelled = TodoItem::new("d", TodoStatus::Cancelled);

        assert_eq!(pending.status_icon_and_color(&theme).1, theme.text_muted);
        assert_eq!(in_progress.status_icon_and_color(&theme).1, theme.warning);
        assert_eq!(completed.status_icon_and_color(&theme).1, theme.success);
        assert_eq!(cancelled.status_icon_and_color(&theme).1, theme.text_muted);
    }
}
