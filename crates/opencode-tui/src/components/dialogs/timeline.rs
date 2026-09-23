use std::cell::Cell;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::theme::Theme;

/// Default page size used before the dialog has rendered once and measured its
/// own inner height.
const DEFAULT_PAGE_STEP: usize = 10;

#[derive(Clone, Debug)]
pub struct TimelineEntry {
    pub message_id: String,
    pub role: String,
    pub preview: String,
    pub timestamp: String,
}

pub struct TimelineDialog {
    entries: Vec<TimelineEntry>,
    state: ListState,
    open: bool,
    /// Number of entries a page-up/page-down press moves, derived from the last
    /// rendered viewport so paging matches what the user can see.
    page_step: Cell<usize>,
}

impl TimelineDialog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            state: ListState::default(),
            open: false,
            page_step: Cell::new(DEFAULT_PAGE_STEP),
        }
    }

    pub fn open(&mut self, entries: Vec<TimelineEntry>) {
        self.entries = entries;
        // Select the oldest prompt first so `Enter` immediately jumps to the top
        // of the session; `End` reaches the newest.
        self.state.select(if self.entries.is_empty() {
            None
        } else {
            Some(0)
        });
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn move_up(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 {
                self.state.select(Some(i - 1));
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.entries.len().saturating_sub(1) {
                self.state.select(Some(i + 1));
            }
        }
    }

    /// Move the selection by `delta` entries, clamped to the list bounds. Used
    /// for page-up/page-down so a long timeline is reachable without holding an
    /// arrow key.
    pub fn move_by(&mut self, delta: isize) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let current = self.state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, len as isize - 1) as usize;
        self.state.select(Some(next));
    }

    pub fn select_first(&mut self) {
        if !self.entries.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if let Some(last) = self.entries.len().checked_sub(1) {
            self.state.select(Some(last));
        }
    }

    pub fn page_up(&mut self) {
        let step = self.page_step.get().max(1) as isize;
        self.move_by(-step);
    }

    pub fn page_down(&mut self) {
        let step = self.page_step.get().max(1) as isize;
        self.move_by(step);
    }

    pub fn selected_entry(&self) -> Option<&TimelineEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    pub fn selected_message_id(&self) -> Option<&str> {
        self.selected_entry().map(|e| e.message_id.as_str())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }
        let dialog_width = 70u16.min(area.width.saturating_sub(4));
        let dialog_height = 20u16.min(area.height.saturating_sub(4));
        let dialog_area = centered_rect(dialog_width, dialog_height, area);
        frame.render_widget(Clear, dialog_area);
        let title = if self.entries.is_empty() {
            " Timeline ".to_string()
        } else {
            let position = self.state.selected().map_or(0, |i| i + 1);
            format!(" Timeline  {}/{} ", position, self.entries.len())
        };
        let block = Block::default()
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));
        let inner = super::dialog_inner(block.inner(dialog_area));
        self.page_step
            .set(usize::from(inner.height.saturating_sub(1)).max(1));
        frame.render_widget(block, dialog_area);
        if self.entries.is_empty() {
            let empty = ratatui::widgets::Paragraph::new("No messages in timeline")
                .style(Style::default().fg(theme.text_muted));
            frame.render_widget(empty, inner);
            return;
        }
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let role_icon = match entry.role.as_str() {
                    "user" => "[U]",
                    "assistant" => "[A]",
                    _ => "[S]",
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", role_icon),
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled(&entry.preview, Style::default().fg(theme.text)),
                    Span::styled(
                        format!("  {}", entry.timestamp),
                        Style::default().fg(theme.text_muted),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(theme.background_element)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, inner, &mut self.state.clone());
    }
}

impl Default for TimelineDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(n: usize) -> Vec<TimelineEntry> {
        (0..n)
            .map(|i| TimelineEntry {
                message_id: format!("m{i}"),
                role: "user".to_string(),
                preview: format!("prompt-{i}"),
                timestamp: "00:00:00".to_string(),
            })
            .collect()
    }

    #[test]
    fn open_selects_oldest_prompt() {
        let mut dialog = TimelineDialog::new();
        dialog.open(entries(5));

        assert_eq!(dialog.len(), 5);
        assert_eq!(dialog.selected_index(), Some(0));
    }

    #[test]
    fn select_first_and_last_clamp_to_bounds() {
        let mut dialog = TimelineDialog::new();
        dialog.open(entries(5));

        dialog.select_last();
        assert_eq!(dialog.selected_index(), Some(4));
        dialog.move_down();
        assert_eq!(dialog.selected_index(), Some(4));

        dialog.select_first();
        assert_eq!(dialog.selected_index(), Some(0));
        dialog.move_up();
        assert_eq!(dialog.selected_index(), Some(0));
    }

    #[test]
    fn move_by_clamps_within_the_list() {
        let mut dialog = TimelineDialog::new();
        dialog.open(entries(30));

        dialog.move_by(12);
        assert_eq!(dialog.selected_index(), Some(12));
        dialog.move_by(1_000);
        assert_eq!(dialog.selected_index(), Some(29));
        dialog.move_by(-1_000);
        assert_eq!(dialog.selected_index(), Some(0));
    }

    #[test]
    fn page_moves_use_measured_step() {
        let mut dialog = TimelineDialog::new();
        dialog.open(entries(50));
        dialog.page_step.set(10);

        dialog.page_down();
        assert_eq!(dialog.selected_index(), Some(10));
        dialog.page_down();
        assert_eq!(dialog.selected_index(), Some(20));
        dialog.page_up();
        assert_eq!(dialog.selected_index(), Some(10));

        dialog.select_last();
        dialog.page_down();
        assert_eq!(dialog.selected_index(), Some(49));
    }

    #[test]
    fn navigation_on_empty_timeline_is_safe() {
        let mut dialog = TimelineDialog::new();
        dialog.open(Vec::new());

        assert!(dialog.is_empty());
        assert_eq!(dialog.selected_index(), None);
        dialog.select_first();
        dialog.select_last();
        dialog.move_up();
        dialog.move_down();
        dialog.move_by(5);
        dialog.page_down();
        assert_eq!(dialog.selected_index(), None);
    }

    fn render_to_string(dialog: &TimelineDialog) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();
        let theme = crate::theme::Theme::default();
        terminal
            .draw(|frame| dialog.render(frame, frame.size(), &theme))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer.get(x, y).symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn title_shows_position_out_of_total() {
        let mut dialog = TimelineDialog::new();
        dialog.open(entries(5));
        assert!(render_to_string(&dialog).contains("Timeline  1/5"));

        dialog.select_last();
        assert!(render_to_string(&dialog).contains("Timeline  5/5"));

        dialog.page_step.set(1);
        dialog.page_up();
        assert!(render_to_string(&dialog).contains("Timeline  4/5"));
    }

    #[test]
    fn empty_timeline_title_has_no_position() {
        let mut dialog = TimelineDialog::new();
        dialog.open(Vec::new());
        let rendered = render_to_string(&dialog);
        assert!(rendered.contains("Timeline"));
        assert!(!rendered.contains("/0"));
    }
}
