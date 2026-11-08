use std::cell::{Cell, RefCell};

use ratatui::prelude::Stylize;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::theme::Theme;

use super::dialogs::DialogTextInput;

/// Rendered row count for one content line, matching the wrapping the popup
/// paragraph uses. Empty lines occupy exactly one row.
fn line_wrapped_rows(line: &Line, width: u16) -> usize {
    if width == 0 {
        return 1;
    }
    Paragraph::new(vec![line.clone()])
        .wrap(Wrap { trim: false })
        .line_count(width)
        .max(1)
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuestionType {
    Text,
    MultipleChoice,
    SingleChoice,
    Review,
}

#[derive(Clone, Debug)]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct QuestionRequest {
    pub id: String,
    pub question: String,
    pub question_type: QuestionType,
    pub options: Vec<QuestionOption>,
    pub custom: bool,
}

pub struct QuestionPrompt {
    current_question: Option<QuestionRequest>,
    pub is_open: bool,
    selected_index: usize,
    selected_options: Vec<bool>,
    custom_selected: bool,
    text_input: DialogTextInput,
    text_mode: bool,
    last_rendered_area: Cell<Option<Rect>>,
    option_rows: RefCell<Vec<u16>>,
}

fn is_single_select(question_type: &QuestionType) -> bool {
    matches!(
        question_type,
        QuestionType::SingleChoice | QuestionType::Review
    )
}

impl QuestionPrompt {
    pub fn new() -> Self {
        Self {
            current_question: None,
            is_open: false,
            selected_index: 0,
            selected_options: Vec::new(),
            custom_selected: false,
            text_input: DialogTextInput::new(),
            text_mode: false,
            last_rendered_area: Cell::new(None),
            option_rows: RefCell::new(Vec::new()),
        }
    }

    pub fn ask(&mut self, question: QuestionRequest) {
        let option_count = question.options.len();
        self.current_question = Some(question);
        self.is_open = true;
        self.selected_index = 0;
        self.selected_options = vec![false; option_count];
        self.custom_selected = false;
        self.text_input.clear();
        self.text_mode = false;
        self.option_rows.borrow_mut().clear();
    }

    pub fn current(&self) -> Option<&QuestionRequest> {
        self.current_question.as_ref()
    }

    pub fn close(&mut self) {
        self.current_question = None;
        self.is_open = false;
        self.selected_index = 0;
        self.selected_options.clear();
        self.custom_selected = false;
        self.text_input.clear();
        self.text_mode = false;
        self.option_rows.borrow_mut().clear();
    }

    /// True when a selectable "Type your own answer" row is appended.
    fn has_custom_row(&self) -> bool {
        match &self.current_question {
            Some(q) => {
                q.custom
                    && !q.options.is_empty()
                    && q.question_type != QuestionType::Text
                    && q.question_type != QuestionType::Review
            }
            None => false,
        }
    }

    fn option_row_count(&self) -> usize {
        match &self.current_question {
            Some(q) => q.options.len() + usize::from(self.has_custom_row()),
            None => 0,
        }
    }

    fn on_custom_row(&self) -> bool {
        let Some(q) = &self.current_question else {
            return false;
        };
        self.has_custom_row() && self.selected_index == q.options.len()
    }

    /// Switch the prompt into custom-answer text mode, keeping the custom-row
    /// marker consistent regardless of how the row was reached. For
    /// single-select choices a custom answer replaces the option selection.
    fn enter_custom_text_mode(&mut self) {
        let single_select = self
            .current_question
            .as_ref()
            .is_some_and(|q| is_single_select(&q.question_type));
        self.text_mode = true;
        self.custom_selected = true;
        if single_select {
            for opt in self.selected_options.iter_mut() {
                *opt = false;
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.option_row_count();
        if count > 0 && self.selected_index + 1 < count {
            self.selected_index += 1;
        }
    }

    /// Select an option row (or the custom row) by index. Single-select types
    /// replace the current selection; multi-select toggles.
    fn select_index(&mut self, idx: usize) {
        let Some(q) = &self.current_question else {
            return;
        };
        if q.question_type == QuestionType::Text {
            return;
        }
        let option_len = q.options.len();
        let has_custom = self.has_custom_row();

        if idx < option_len {
            if is_single_select(&q.question_type) {
                for opt in self.selected_options.iter_mut() {
                    *opt = false;
                }
                self.selected_options[idx] = true;
                self.custom_selected = false;
            } else {
                self.selected_options[idx] = !self.selected_options[idx];
            }
            self.selected_index = idx;
        } else if has_custom && idx == option_len {
            self.selected_index = idx;
            self.custom_selected = true;
            if is_single_select(&q.question_type) {
                for opt in self.selected_options.iter_mut() {
                    *opt = false;
                }
            }
        }
    }

    pub fn toggle_selected(&mut self) {
        let Some(q) = &self.current_question else {
            return;
        };
        if q.question_type == QuestionType::Text {
            return;
        }
        if self.on_custom_row() {
            self.custom_selected = !self.custom_selected;
            if is_single_select(&q.question_type) && self.custom_selected {
                for opt in self.selected_options.iter_mut() {
                    *opt = false;
                }
            }
            return;
        }
        if self.selected_index >= self.selected_options.len() {
            return;
        }
        if is_single_select(&q.question_type) {
            for opt in self.selected_options.iter_mut() {
                *opt = false;
            }
            self.selected_options[self.selected_index] = true;
            self.custom_selected = false;
        } else {
            self.selected_options[self.selected_index] =
                !self.selected_options[self.selected_index];
        }
    }

    pub fn type_char(&mut self, c: char) {
        if self.text_mode {
            self.text_input.insert_char(c);
            return;
        }
        let Some(q) = &self.current_question else {
            return;
        };
        if q.question_type == QuestionType::Text {
            self.text_input.insert_char(c);
            return;
        }
        if let Some(digit) = c.to_digit(10) {
            if digit >= 1 {
                self.select_index((digit - 1) as usize);
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.is_text_input_active() {
            self.text_input.backspace();
        }
    }

    pub fn delete(&mut self) {
        if self.is_text_input_active() {
            self.text_input.delete();
        }
    }

    pub fn move_left(&mut self) {
        if self.is_text_input_active() {
            self.text_input.move_left();
        }
    }

    pub fn move_right(&mut self) {
        if self.is_text_input_active() {
            self.text_input.move_right();
        }
    }

    pub fn move_word_left(&mut self) {
        if self.is_text_input_active() {
            self.text_input.move_word_left();
        }
    }

    pub fn move_word_right(&mut self) {
        if self.is_text_input_active() {
            self.text_input.move_word_right();
        }
    }

    pub fn move_home(&mut self) {
        if self.is_text_input_active() {
            self.text_input.move_home();
        }
    }

    pub fn move_end(&mut self) {
        if self.is_text_input_active() {
            self.text_input.move_end();
        }
    }

    /// Insert pasted or programmatic text into the active input. Pasting into a
    /// choice question enters custom-answer mode so the text has somewhere to go.
    pub fn insert_text(&mut self, text: &str) {
        let is_text = self
            .current_question
            .as_ref()
            .is_some_and(|q| q.question_type == QuestionType::Text);
        if !is_text {
            if !self.has_custom_row() {
                return;
            }
            if !self.text_mode {
                self.enter_custom_text_mode();
            }
        }
        self.text_input.insert_str(text);
    }

    /// True while keystrokes should be captured as typed answer text.
    pub fn is_text_input_active(&self) -> bool {
        self.text_mode
            || self
                .current_question
                .as_ref()
                .is_some_and(|q| q.question_type == QuestionType::Text)
    }

    /// Route a space keypress to the active input, otherwise toggle selection.
    pub fn space(&mut self) {
        if self.is_text_input_active() {
            self.text_input.insert_char(' ');
        } else {
            self.toggle_selected();
        }
    }

    /// Cancel an in-progress custom-answer edit without closing the prompt.
    /// Returns `true` when an active custom edit was cancelled.
    pub fn cancel_text_input(&mut self) -> bool {
        if self.text_mode {
            self.text_mode = false;
            self.custom_selected = false;
            self.text_input.clear();
            true
        } else {
            false
        }
    }

    /// Confirm the current prompt. Returns the resolved request and answer IDs
    /// when the answer is submit-ready. Returns `None` (without closing) when the
    /// user has not selected an answer yet, or when selecting the custom row
    /// switches the prompt into text-entry mode.
    pub fn confirm(&mut self) -> Option<(QuestionRequest, Vec<String>)> {
        let q = self.current_question.as_ref()?;
        let is_text = q.question_type == QuestionType::Text;

        // Selecting the custom row enters text-entry mode first.
        if !self.text_mode && !is_text && self.on_custom_row() {
            self.enter_custom_text_mode();
            return None;
        }

        let mut answers: Vec<String> = Vec::new();
        if is_text || self.text_mode {
            let text = self.text_input.value().trim().to_string();
            if text.is_empty() {
                return None;
            }
            // A custom answer on a multi-select choice is additive; a custom
            // answer on a single-select choice replaces the selection.
            if self.text_mode && !is_text && !is_single_select(&q.question_type) {
                answers.extend(
                    q.options
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| self.selected_options.get(*i).copied().unwrap_or(false))
                        .map(|(_, opt)| opt.id.clone()),
                );
            }
            answers.push(text);
        } else {
            answers.extend(
                q.options
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| self.selected_options.get(*i).copied().unwrap_or(false))
                    .map(|(_, opt)| opt.id.clone()),
            );
            if self.custom_selected {
                // Custom row selected but no text typed yet.
                self.text_mode = true;
                return None;
            }
            if answers.is_empty() {
                return None;
            }
        }

        let request = self.current_question.take()?;
        self.is_open = false;
        self.selected_index = 0;
        self.selected_options.clear();
        self.custom_selected = false;
        self.text_input.clear();
        self.text_mode = false;
        Some((request, answers))
    }

    pub fn handle_click(&mut self, col: u16, row: u16) {
        if !self.is_open {
            return;
        }
        let Some(area) = self.last_rendered_area.get() else {
            return;
        };
        if row < area.y || row >= area.y + area.height || col < area.x || col >= area.x + area.width
        {
            return;
        }
        let option_len = self
            .current_question
            .as_ref()
            .map(|q| q.options.len())
            .unwrap_or(0);
        let idx = {
            let rows = self.option_rows.borrow();
            rows.iter().position(|y| *y == row)
        };
        if let Some(idx) = idx {
            if self.has_custom_row() && idx == option_len {
                self.selected_index = idx;
                self.custom_selected = true;
                self.text_mode = true;
            } else {
                self.selected_index = idx;
                self.toggle_selected();
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.is_open {
            return;
        }

        let question = match &self.current_question {
            Some(q) => q,
            None => return,
        };

        // The prompt box is the single place the question is shown; the border
        // title labels it, so no separate "Question:" line is repeated here.
        let mut content: Vec<Line> = Vec::new();

        for line in question.question.split('\n') {
            content.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(theme.text),
            )));
        }
        content.push(Line::from(""));

        // Content-line indices for each selectable row, resolved to absolute
        // screen rows after the popup rect is known.
        let mut row_indices: Vec<usize> = Vec::new();

        if question.question_type == QuestionType::Text || question.options.is_empty() {
            let editing = question.question_type == QuestionType::Text || self.text_mode;
            if editing {
                content.push(self.input_line(theme));
                content.push(Line::from(""));
                content.push(Line::from(Span::styled(
                    "Type your answer and press Enter",
                    Style::default().fg(theme.text_muted),
                )));
            }
        } else {
            for (i, opt) in question.options.iter().enumerate() {
                let is_selected = self.selected_options.get(i).copied().unwrap_or(false);
                let is_highlighted = i == self.selected_index && !self.text_mode;
                let marker = if is_selected { "[x]" } else { "[ ]" };
                let key = if i < 9 { (b'1' + i as u8) as char } else { ' ' };
                let label_style = if is_highlighted {
                    Style::default().fg(theme.primary).bold()
                } else {
                    Style::default().fg(theme.text)
                };
                row_indices.push(content.len());
                content.push(Line::from(vec![
                    Span::styled(
                        format!("{} ({}) ", marker, key),
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled(&opt.label, label_style),
                ]));
                if !opt.description.is_empty() {
                    content.push(Line::from(Span::styled(
                        format!("      {}", opt.description),
                        Style::default().fg(theme.text_muted),
                    )));
                }
            }

            if self.has_custom_row() {
                let is_highlighted =
                    self.selected_index == question.options.len() && !self.text_mode;
                let marker = if self.custom_selected { "[x]" } else { "[ ]" };
                let label_style = if is_highlighted {
                    Style::default().fg(theme.primary).bold()
                } else {
                    Style::default().fg(theme.text)
                };
                row_indices.push(content.len());
                content.push(Line::from(vec![
                    Span::styled(
                        format!("{} (\u{270e}) ", marker),
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled("Type your own answer", label_style),
                ]));
            }

            content.push(Line::from(""));
            if self.text_mode {
                content.push(self.input_line(theme));
                content.push(Line::from(Span::styled(
                    "Enter to submit, Esc to cancel",
                    Style::default().fg(theme.text_muted),
                )));
            } else {
                content.push(Line::from(Span::styled(
                    "Up/Down to navigate, Space to toggle, Enter to confirm",
                    Style::default().fg(theme.text_muted),
                )));
            }
        }

        // Two-thirds of the area, clamped to sensible bounds and centered.
        let max_width = area.width.saturating_sub(2).max(1);
        let target_width = ((area.width as u32) * 2 / 3) as u16;
        let width = target_width.clamp(24.min(max_width), max_width);
        let inner_width = width.saturating_sub(2);

        // Row count each content line occupies once wrapped, so the popup height
        // and clickable option rows match what actually renders.
        let line_rows: Vec<usize> = content
            .iter()
            .map(|line| line_wrapped_rows(line, inner_width))
            .collect();

        // Reserve one blank row above and below the box so it does not touch the
        // transcript or the bottom edge.
        let max_height = area.height.saturating_sub(2).max(1);

        // No scrolling: when the wrapped content is taller than the box can be,
        // statically drop leading content lines so the input line and hint stay
        // visible.
        let mut start = 0usize;
        let mut visible_rows: usize = line_rows.iter().sum();
        while visible_rows + 2 > max_height as usize && start < content.len() {
            visible_rows -= line_rows[start];
            start += 1;
        }
        if start > 0 {
            content.drain(0..start);
            row_indices = row_indices
                .iter()
                .filter_map(|&index| index.checked_sub(start))
                .collect();
        }
        let visible_line_rows = &line_rows[start..];

        let height = u16::try_from(visible_rows)
            .unwrap_or(u16::MAX)
            .saturating_add(2)
            .min(max_height);
        let inner_height = height.saturating_sub(2);

        // Render inline near the bottom of the area.
        let popup_area = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height + 1),
            width,
            height,
        );

        // Track rendered area and absolute option rows for click handling.
        self.last_rendered_area.set(Some(popup_area));

        // Offset of each visible content line in wrapped rows, so clickable
        // option rows map to the rows actually drawn.
        let mut offsets = vec![0usize; content.len()];
        for i in 1..content.len() {
            offsets[i] = offsets[i - 1] + visible_line_rows[i - 1];
        }
        let visible_top = popup_area.y.saturating_add(1);
        let absolute_rows: Vec<u16> = row_indices
            .iter()
            .map(|&index| match offsets.get(index) {
                Some(&row) if row < inner_height as usize => visible_top.saturating_add(row as u16),
                _ => u16::MAX,
            })
            .collect();
        *self.option_rows.borrow_mut() = absolute_rows;

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Question ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.primary)),
            )
            .style(Style::default().bg(theme.background_panel))
            .wrap(Wrap { trim: false });

        // Clear the popup area first so transcript glyphs never bleed through
        // the box background.
        frame.render_widget(ratatui::widgets::Clear, popup_area);
        frame.render_widget(paragraph, popup_area);
    }

    /// The `> {before}▏{after}` input line with the caret drawn at the cursor.
    fn input_line(&self, theme: &Theme) -> Line<'static> {
        let text_style = Style::default().fg(theme.text);
        let caret_style = Style::default().fg(theme.primary);
        if self.text_input.value().is_empty() {
            return Line::from(vec![
                Span::styled("> ", text_style),
                Span::styled("\u{258f}", caret_style),
                Span::styled("Type your answer...", Style::default().fg(theme.text_muted)),
            ]);
        }
        let (before, after) = self.text_input.split_at_cursor();
        Line::from(vec![
            Span::styled("> ", text_style),
            Span::styled(before.to_string(), text_style),
            Span::styled("\u{258f}", caret_style),
            Span::styled(after.to_string(), text_style),
        ])
    }
}

impl Default for QuestionPrompt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(question_type: QuestionType, custom: bool) -> QuestionRequest {
        QuestionRequest {
            id: "request-1".to_string(),
            question: "Choose one".to_string(),
            question_type,
            options: vec![
                QuestionOption {
                    id: "a".to_string(),
                    label: "A".to_string(),
                    description: "Option A".to_string(),
                },
                QuestionOption {
                    id: "b".to_string(),
                    label: "B".to_string(),
                    description: "Option B".to_string(),
                },
            ],
            custom,
        }
    }

    fn option_prompt(question_type: QuestionType) -> QuestionPrompt {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(question_type, false));
        prompt
    }

    #[test]
    fn confirm_ignores_single_choice_without_selection() {
        let mut prompt = option_prompt(QuestionType::SingleChoice);

        assert!(prompt.confirm().is_none());
        assert!(prompt.is_open);
        assert!(prompt.current().is_some());
    }

    #[test]
    fn confirm_ignores_multiple_choice_without_selection() {
        let mut prompt = option_prompt(QuestionType::MultipleChoice);

        assert!(prompt.confirm().is_none());
        assert!(prompt.is_open);
        assert!(prompt.current().is_some());
    }

    #[test]
    fn confirm_submits_selected_option() {
        let mut prompt = option_prompt(QuestionType::SingleChoice);
        prompt.toggle_selected();

        let (_request, answers) = prompt.confirm().expect("selected answer submits");

        assert_eq!(answers, vec!["a".to_string()]);
        assert!(!prompt.is_open);
    }

    #[test]
    fn digit_shortcut_selects_option() {
        let mut prompt = option_prompt(QuestionType::SingleChoice);

        prompt.type_char('2');
        let (_request, answers) = prompt.confirm().expect("digit-selected answer submits");

        assert_eq!(answers, vec!["b".to_string()]);
    }

    #[test]
    fn digit_shortcut_is_ignored_while_typing_custom_answer() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));

        // Digit '3' selects the appended custom row (options are 1 and 2).
        prompt.type_char('3');
        assert!(prompt.confirm().is_none());

        prompt.type_char('3');
        prompt.type_char(',');
        prompt.type_char('5');

        let (_request, answers) = prompt.confirm().expect("custom answer submits");
        assert_eq!(answers, vec!["3,5".to_string()]);
    }

    #[test]
    fn multiple_choice_preserves_selected_labels() {
        let mut prompt = option_prompt(QuestionType::MultipleChoice);

        prompt.type_char('1');
        prompt.type_char('2');
        let (_request, answers) = prompt.confirm().expect("multi answer submits");

        assert_eq!(answers, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn text_question_returns_typed_answer() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(QuestionRequest {
            id: "request-text".to_string(),
            question: "Anything else?".to_string(),
            question_type: QuestionType::Text,
            options: Vec::new(),
            custom: true,
        });

        for c in "hello".chars() {
            prompt.type_char(c);
        }
        let (_request, answers) = prompt.confirm().expect("text answer submits");

        assert_eq!(answers, vec!["hello".to_string()]);
    }

    #[test]
    fn text_question_ignores_empty_input() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(QuestionRequest {
            id: "request-text".to_string(),
            question: "Anything else?".to_string(),
            question_type: QuestionType::Text,
            options: Vec::new(),
            custom: true,
        });

        assert!(prompt.confirm().is_none());
        assert!(prompt.is_open);
    }

    #[test]
    fn review_prompt_selects_action() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(QuestionRequest {
            id: "review".to_string(),
            question: "Review your answers".to_string(),
            question_type: QuestionType::Review,
            options: vec![
                QuestionOption {
                    id: "Submit answers".to_string(),
                    label: "Submit answers".to_string(),
                    description: String::new(),
                },
                QuestionOption {
                    id: "Go back".to_string(),
                    label: "Go back".to_string(),
                    description: String::new(),
                },
            ],
            custom: false,
        });

        prompt.toggle_selected();
        let (_request, answers) = prompt.confirm().expect("review action submits");
        assert_eq!(answers, vec!["Submit answers".to_string()]);
    }

    #[test]
    fn cancel_text_input_returns_to_options() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));

        prompt.type_char('3');
        prompt.confirm();
        assert!(prompt.text_mode);
        prompt.type_char('x');
        prompt.cancel_text_input();

        assert!(!prompt.text_mode);
        assert!(prompt.text_input.value().is_empty());
    }

    #[test]
    fn custom_answer_is_caret_editable() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));

        prompt.type_char('3');
        assert!(prompt.confirm().is_none());
        assert!(prompt.text_mode);

        for c in "abc".chars() {
            prompt.type_char(c);
        }
        prompt.move_left();
        prompt.move_left();
        prompt.type_char('X');
        let (_request, answers) = prompt.confirm().expect("custom answer submits");
        assert_eq!(answers, vec!["aXbc".to_string()]);
    }

    #[test]
    fn delete_removes_character_at_caret() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));
        prompt.type_char('3');
        prompt.confirm();
        for c in "abc".chars() {
            prompt.type_char(c);
        }
        prompt.move_home();
        prompt.move_right();
        prompt.delete();
        let (_request, answers) = prompt.confirm().expect("custom answer submits");
        assert_eq!(answers, vec!["ac".to_string()]);
    }

    #[test]
    fn word_movement_skips_by_word() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));
        prompt.type_char('3');
        prompt.confirm();
        prompt.insert_text("alpha beta gamma");
        prompt.move_home();
        prompt.move_word_right();
        prompt.type_char('|');
        let (_request, answers) = prompt.confirm().expect("custom answer submits");
        assert_eq!(answers, vec!["alpha| beta gamma".to_string()]);
    }

    #[test]
    fn insert_text_flattens_newlines() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(QuestionRequest {
            id: "request-text".to_string(),
            question: "Anything else?".to_string(),
            question_type: QuestionType::Text,
            options: Vec::new(),
            custom: true,
        });
        prompt.insert_text("line one\r\nline two\nline three\ttail");
        let (_request, answers) = prompt.confirm().expect("text answer submits");
        assert_eq!(
            answers,
            vec!["line one line two line three tail".to_string()]
        );
    }

    #[test]
    fn insert_text_enters_custom_mode_and_marks_row() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));
        prompt.insert_text("pasted");
        assert!(prompt.text_mode);
        assert!(prompt.custom_selected);
        let (_request, answers) = prompt.confirm().expect("pasted answer submits");
        assert_eq!(answers, vec!["pasted".to_string()]);
    }

    #[test]
    fn custom_row_marker_set_when_confirm_enters_text_mode() {
        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, true));
        prompt.move_down();
        prompt.move_down();
        assert!(!prompt.custom_selected);
        assert!(prompt.confirm().is_none());
        assert!(prompt.text_mode);
        assert!(prompt.custom_selected);
    }

    #[test]
    fn short_terminal_keeps_input_and_hint_visible() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut prompt = QuestionPrompt::new();
        prompt.ask(QuestionRequest {
            id: "long".to_string(),
            question: "A question long enough to wrap across several rows in a narrow popup"
                .to_string(),
            question_type: QuestionType::SingleChoice,
            options: (0..4)
                .map(|i| QuestionOption {
                    id: format!("opt{i}"),
                    label: format!("Option {i}"),
                    description: "A description that is also long enough to wrap around"
                        .to_string(),
                })
                .collect(),
            custom: true,
        });
        prompt.type_char('5');
        prompt.confirm();
        assert!(prompt.text_mode);
        prompt.insert_text("the typed answer");

        let theme = Theme::default();
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");
        terminal
            .draw(|frame| prompt.render(frame, frame.size(), &theme))
            .expect("draw");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer.get(x, y).symbol());
            }
            text.push('\n');
        }
        assert!(text.contains("Enter to submit"), "hint visible in:\n{text}");
        assert!(text.contains('\u{258f}'), "caret visible in:\n{text}");
    }

    #[test]
    fn wide_terminal_uses_two_thirds_width_and_centers() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut prompt = QuestionPrompt::new();
        prompt.ask(option(QuestionType::SingleChoice, false));

        let theme = Theme::default();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
        terminal
            .draw(|frame| prompt.render(frame, frame.size(), &theme))
            .expect("draw");

        let buffer = terminal.backend().buffer();
        let mut corner = None;
        'outer: for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                if buffer.get(x, y).symbol() == "\u{250c}" {
                    corner = Some((x, y));
                    break 'outer;
                }
            }
        }
        let (x, y) = corner.expect("box top-left corner");
        let expected_width = (120u32 * 2 / 3) as u16;
        let expected_x = (120 - expected_width) / 2;
        assert_eq!(x, expected_x, "box is horizontally centered");
        assert_eq!(
            buffer.get(x + expected_width - 1, y).symbol(),
            "\u{2510}",
            "box is two-thirds of the terminal width"
        );
    }
}
