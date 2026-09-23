use std::cell::{Cell, RefCell};

use ratatui::prelude::Stylize;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

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
    text_input: String,
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
            text_input: String::new(),
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
            self.text_input.push(c);
            return;
        }
        let Some(q) = &self.current_question else {
            return;
        };
        if q.question_type == QuestionType::Text {
            self.text_input.push(c);
            return;
        }
        if let Some(digit) = c.to_digit(10) {
            if digit >= 1 {
                self.select_index((digit - 1) as usize);
            }
        }
    }

    pub fn backspace(&mut self) {
        let editing_text = self.text_mode
            || self
                .current_question
                .as_ref()
                .is_some_and(|q| q.question_type == QuestionType::Text);
        if editing_text {
            self.text_input.pop();
        }
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
            self.text_input.push(' ');
        } else {
            self.toggle_selected();
        }
    }

    /// Cancel an in-progress custom-answer edit without closing the prompt.
    /// Returns `true` when an active custom edit was cancelled.
    pub fn cancel_text_input(&mut self) -> bool {
        if self.text_mode {
            self.text_mode = false;
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
            self.text_mode = true;
            return None;
        }

        let mut answers: Vec<String> = Vec::new();
        if is_text || self.text_mode {
            let text = self.text_input.trim().to_string();
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

        let mut content = vec![
            Line::from(Span::styled(
                "Question:",
                Style::default().fg(theme.primary).bold(),
            )),
            Line::from(""),
        ];

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
                let input_display = if self.text_input.is_empty() {
                    "Type your answer...".to_string()
                } else {
                    format!("> {}", self.text_input)
                };
                let input_style = if self.text_input.is_empty() {
                    Style::default().fg(theme.text_muted)
                } else {
                    Style::default().fg(theme.text)
                };
                content.push(Line::from(Span::styled(input_display, input_style)));
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
                let input_display = if self.text_input.is_empty() {
                    "> Type your answer...".to_string()
                } else {
                    format!("> {}", self.text_input)
                };
                content.push(Line::from(Span::styled(
                    input_display,
                    Style::default().fg(theme.text),
                )));
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

        let height = (content.len() as u16 + 2).min(area.height.saturating_sub(2));
        let width = area.width.saturating_sub(2).min(80);

        // Render inline at the bottom of the area
        let popup_area = Rect::new(
            area.x + 1,
            area.y + area.height.saturating_sub(height + 1),
            width,
            height,
        );

        // Track rendered area and absolute option rows for click handling.
        self.last_rendered_area.set(Some(popup_area));
        let absolute_rows: Vec<u16> = row_indices
            .into_iter()
            .map(|index| popup_area.y + 1 + index as u16)
            .collect();
        *self.option_rows.borrow_mut() = absolute_rows;

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Question ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.primary)),
            )
            .style(Style::default().bg(theme.background_panel));

        frame.render_widget(paragraph, popup_area);
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
        assert!(prompt.text_input.is_empty());
    }
}
