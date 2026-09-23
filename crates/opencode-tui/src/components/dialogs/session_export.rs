use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

use super::text_input::DialogTextInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFocus {
    Filename,
    Thinking,
    ToolDetails,
    Metadata,
}

impl ExportFocus {
    const COUNT: usize = 4;

    fn index(self) -> usize {
        match self {
            ExportFocus::Filename => 0,
            ExportFocus::Thinking => 1,
            ExportFocus::ToolDetails => 2,
            ExportFocus::Metadata => 3,
        }
    }

    fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => ExportFocus::Filename,
            1 => ExportFocus::Thinking,
            2 => ExportFocus::ToolDetails,
            _ => ExportFocus::Metadata,
        }
    }
}

pub struct SessionExportDialog {
    open: bool,
    session_id: Option<String>,
    filename: DialogTextInput,
    pub include_thinking: bool,
    pub include_tool_details: bool,
    pub include_metadata: bool,
    focus: ExportFocus,
}

impl SessionExportDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            session_id: None,
            filename: DialogTextInput::new(),
            include_thinking: false,
            include_tool_details: true,
            include_metadata: false,
            focus: ExportFocus::Filename,
        }
    }

    pub fn open(&mut self, session_id: String, default_filename: String) {
        self.open = true;
        self.session_id = Some(session_id);
        self.filename.set(default_filename);
        self.focus = ExportFocus::Filename;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.session_id = None;
        self.filename.clear();
        self.focus = ExportFocus::Filename;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn handle_input(&mut self, c: char) {
        if c == ' ' && self.focus != ExportFocus::Filename {
            self.toggle_focused_option();
            return;
        }
        self.focus = ExportFocus::Filename;
        self.filename.insert_char(c);
    }

    pub fn handle_backspace(&mut self) {
        self.focus = ExportFocus::Filename;
        self.filename.backspace();
    }

    pub fn handle_delete(&mut self) {
        self.focus = ExportFocus::Filename;
        self.filename.delete();
    }

    pub fn move_left(&mut self, alt: bool) {
        if self.focus == ExportFocus::Filename {
            if alt {
                self.filename.move_word_left();
            } else {
                self.filename.move_left();
            }
        } else {
            self.focus_prev();
        }
    }

    pub fn move_right(&mut self, alt: bool) {
        if self.focus == ExportFocus::Filename {
            if alt {
                self.filename.move_word_right();
            } else {
                self.filename.move_right();
            }
        } else {
            self.focus_next();
        }
    }

    pub fn move_home(&mut self) {
        self.focus = ExportFocus::Filename;
        self.filename.move_home();
    }

    pub fn move_end(&mut self) {
        self.focus = ExportFocus::Filename;
        self.filename.move_end();
    }

    pub fn focus_next(&mut self) {
        self.focus = ExportFocus::from_index(self.focus.index() + 1);
    }

    pub fn focus_prev(&mut self) {
        self.focus = ExportFocus::from_index(self.focus.index() + ExportFocus::COUNT - 1);
    }

    pub fn is_filename_focused(&self) -> bool {
        self.focus == ExportFocus::Filename
    }

    fn toggle_focused_option(&mut self) {
        match self.focus {
            ExportFocus::Filename => {}
            ExportFocus::Thinking => self.include_thinking = !self.include_thinking,
            ExportFocus::ToolDetails => self.include_tool_details = !self.include_tool_details,
            ExportFocus::Metadata => self.include_metadata = !self.include_metadata,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn filename(&self) -> &str {
        self.filename.value()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let dialog_area = centered_rect(80, 14, area);
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Export Session ",
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
                Constraint::Length(1), // "Output filename:"
                Constraint::Length(1), // filename input
                Constraint::Length(1), // spacer
                Constraint::Length(1), // "Options:"
                Constraint::Length(1), // thinking
                Constraint::Length(1), // tool details
                Constraint::Length(1), // metadata
                Constraint::Length(1), // spacer
                Constraint::Length(1), // hint
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new("Output filename:").style(Style::default().fg(theme.text)),
            layout[0],
        );

        let filename_line = if self.is_filename_focused() {
            let (before, after) = self.filename.split_at_cursor();
            Line::from(vec![
                Span::styled("> ", Style::default().fg(theme.primary)),
                Span::styled(before.to_string(), Style::default().fg(theme.text)),
                Span::styled("▏", Style::default().fg(theme.primary)),
                Span::styled(after.to_string(), Style::default().fg(theme.text)),
            ])
        } else {
            Line::from(vec![
                Span::styled("  ", Style::default().fg(theme.primary)),
                Span::styled(
                    self.filename.value().to_string(),
                    Style::default().fg(theme.text),
                ),
            ])
        };
        frame.render_widget(Paragraph::new(filename_line), layout[1]);

        frame.render_widget(
            Paragraph::new("Options:").style(Style::default().fg(theme.text_muted)),
            layout[3],
        );

        let check = |v: bool| if v { "[x]" } else { "[ ]" };
        let marker = |focused: bool| if focused { ">" } else { " " };
        let option_style = |focused: bool| {
            if focused {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            }
        };

        frame.render_widget(
            Paragraph::new(format!(
                " {} {} Include thinking blocks",
                marker(self.focus == ExportFocus::Thinking),
                check(self.include_thinking)
            ))
            .style(option_style(self.focus == ExportFocus::Thinking)),
            layout[4],
        );
        frame.render_widget(
            Paragraph::new(format!(
                " {} {} Include tool call details",
                marker(self.focus == ExportFocus::ToolDetails),
                check(self.include_tool_details)
            ))
            .style(option_style(self.focus == ExportFocus::ToolDetails)),
            layout[5],
        );
        frame.render_widget(
            Paragraph::new(format!(
                " {} {} Include assistant metadata",
                marker(self.focus == ExportFocus::Metadata),
                check(self.include_metadata)
            ))
            .style(option_style(self.focus == ExportFocus::Metadata)),
            layout[6],
        );

        frame.render_widget(
            Paragraph::new(
                "Tab/Shift+Tab focus  Space toggle  Enter export  Ctrl+C copy  Esc cancel",
            )
            .style(Style::default().fg(theme.text_muted)),
            layout[8],
        );
    }
}

impl Default for SessionExportDialog {
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

    fn dialog_with(filename: &str) -> SessionExportDialog {
        let mut dialog = SessionExportDialog::new();
        dialog.open("session-1".to_string(), filename.to_string());
        dialog
    }

    #[test]
    fn digits_type_into_filename() {
        let mut dialog = dialog_with("run-");
        dialog.handle_input('1');
        dialog.handle_input('2');
        dialog.handle_input('3');
        assert_eq!(dialog.filename(), "run-123");
    }

    #[test]
    fn caret_edits_in_the_middle_of_filename() {
        let mut dialog = dialog_with("run-1.md");
        dialog.move_home();
        dialog.move_right(false);
        dialog.move_right(false);
        dialog.move_right(false);
        dialog.handle_input('X');
        assert_eq!(dialog.filename(), "runX-1.md");
        dialog.handle_backspace();
        assert_eq!(dialog.filename(), "run-1.md");
        dialog.handle_delete();
        assert_eq!(dialog.filename(), "run1.md");
    }

    #[test]
    fn alt_arrows_skip_words_in_filename() {
        let mut dialog = dialog_with("alpha beta gamma");
        dialog.move_home();
        dialog.move_right(true);
        assert_eq!(dialog.filename(), "alpha beta gamma");
        // Caret is after "alpha"; a typed char lands there.
        dialog.handle_input('!');
        assert_eq!(dialog.filename(), "alpha! beta gamma");
    }

    #[test]
    fn space_toggles_focused_option_but_types_in_filename() {
        let mut dialog = dialog_with("out.md");
        dialog.handle_input(' ');
        assert_eq!(dialog.filename(), "out.md ");

        dialog.focus_next();
        assert!(!dialog.is_filename_focused());
        dialog.handle_input(' ');
        assert!(dialog.include_thinking);

        dialog.focus_next();
        dialog.handle_input(' ');
        assert!(!dialog.include_tool_details);

        dialog.focus_next();
        dialog.handle_input(' ');
        assert!(dialog.include_metadata);
    }

    #[test]
    fn left_right_move_focus_when_option_focused() {
        let mut dialog = dialog_with("out.md");
        dialog.focus_next();
        assert!(!dialog.is_filename_focused());
        dialog.move_right(false);
        assert!(!dialog.is_filename_focused());
        dialog.move_left(false);
        assert!(!dialog.is_filename_focused());
        dialog.move_left(false);
        assert!(dialog.is_filename_focused());
    }

    #[test]
    fn typing_char_returns_focus_to_filename() {
        let mut dialog = dialog_with("out.md");
        dialog.focus_next();
        dialog.focus_next();
        dialog.handle_input('7');
        assert!(dialog.is_filename_focused());
        assert_eq!(dialog.filename(), "out.md7");
    }
}
