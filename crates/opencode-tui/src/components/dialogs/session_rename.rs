use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

use super::text_input::DialogTextInput;

pub struct SessionRenameDialog {
    open: bool,
    session_id: Option<String>,
    input: DialogTextInput,
}

impl SessionRenameDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            session_id: None,
            input: DialogTextInput::new(),
        }
    }

    pub fn open(&mut self, session_id: String, title: String) {
        self.open = true;
        self.session_id = Some(session_id);
        self.input.set(title);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.session_id = None;
        self.input.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn handle_input(&mut self, c: char) {
        self.input.insert_char(c);
    }

    pub fn handle_backspace(&mut self) {
        self.input.backspace();
    }

    pub fn handle_delete(&mut self) {
        self.input.delete();
    }

    pub fn move_left(&mut self) {
        self.input.move_left();
    }

    pub fn move_right(&mut self) {
        self.input.move_right();
    }

    pub fn move_word_left(&mut self) {
        self.input.move_word_left();
    }

    pub fn move_word_right(&mut self) {
        self.input.move_word_right();
    }

    pub fn move_home(&mut self) {
        self.input.move_home();
    }

    pub fn move_end(&mut self) {
        self.input.move_end();
    }

    pub fn confirm(&mut self) -> Option<(String, String)> {
        let session_id = self.session_id.clone()?;
        let title = self.input.value().trim().to_string();
        if title.is_empty() {
            return None;
        }
        self.close();
        Some((session_id, title))
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let dialog_area = centered_rect(70, 8, area);
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Rename Session ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));
        let inner = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let (before, after) = self.input.split_at_cursor();
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme.primary)),
                Span::styled(before.to_string(), Style::default().fg(theme.text)),
                Span::styled("▏", Style::default().fg(theme.primary)),
                Span::styled(after.to_string(), Style::default().fg(theme.text)),
            ])),
            layout[0],
        );

        frame.render_widget(
            Paragraph::new("←/→ move  Enter save  Esc cancel")
                .style(Style::default().fg(theme.text_muted)),
            layout[2],
        );
    }
}

impl Default for SessionRenameDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}
