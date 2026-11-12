use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::theme::Theme;

#[derive(Clone, Debug)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: u64,
}

pub struct ModelSelectDialog {
    models: Vec<Model>,
    filtered: Vec<usize>,
    query: String,
    state: ListState,
    open: bool,
    current_model: Option<String>,
}

impl ModelSelectDialog {
    pub fn new() -> Self {
        let models = vec![
            Model {
                id: "claude-sonnet-5".into(),
                name: "Claude Sonnet 5".into(),
                provider: "anthropic".into(),
                context_window: 200000,
            },
            Model {
                id: "claude-haiku-4-5".into(),
                name: "Claude Haiku 4.5".into(),
                provider: "anthropic".into(),
                context_window: 200000,
            },
            Model {
                id: "claude-opus-5".into(),
                name: "Claude Opus 5".into(),
                provider: "anthropic".into(),
                context_window: 200000,
            },
            Model {
                id: "gpt-5.3-codex".into(),
                name: "GPT-5.3 Codex".into(),
                provider: "openai".into(),
                context_window: 400000,
            },
            Model {
                id: "gpt-5-mini".into(),
                name: "GPT-5 Mini".into(),
                provider: "openai".into(),
                context_window: 400000,
            },
            Model {
                id: "o4-mini".into(),
                name: "o4 Mini".into(),
                provider: "openai".into(),
                context_window: 200000,
            },
            Model {
                id: "deepseek-v4-pro".into(),
                name: "DeepSeek V4 Pro".into(),
                provider: "deepseek".into(),
                context_window: 128000,
            },
            Model {
                id: "deepseek-flash".into(),
                name: "DeepSeek Flash".into(),
                provider: "deepseek".into(),
                context_window: 128000,
            },
        ];

        let filtered = (0..models.len()).collect();
        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            models,
            filtered,
            query: String::new(),
            state,
            open: false,
            current_model: None,
        }
    }

    pub fn set_models(&mut self, models: Vec<Model>) {
        self.models = models;
        self.filtered = (0..self.models.len()).collect();
    }

    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.filtered = (0..self.models.len()).collect();
        self.state.select(Some(0));
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_current_model(&mut self, current: Option<String>) {
        self.current_model = current.filter(|value| !value.trim().is_empty());
    }

    pub fn handle_input(&mut self, c: char) {
        self.query.push(c);
        self.filter();
    }

    pub fn handle_backspace(&mut self) {
        self.query.pop();
        self.filter();
    }

    pub fn move_up(&mut self) {
        if let Some(selected) = self.state.selected() {
            if selected > 0 {
                self.state.select(Some(selected - 1));
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(selected) = self.state.selected() {
            if selected < self.filtered.len().saturating_sub(1) {
                self.state.select(Some(selected + 1));
            }
        }
    }

    pub fn selected_model(&self) -> Option<&Model> {
        self.state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .and_then(|&idx| self.models.get(idx))
    }

    fn is_current_model(&self, model: &Model) -> bool {
        self.current_model
            .as_deref()
            .is_some_and(|current| current == format!("{}/{}", model.provider, model.id))
    }

    fn filter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.filtered = self
            .models
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.provider.to_lowercase().contains(&query_lower)
                    || m.id.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();

        if self.filtered.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let dialog_width = 50;
        let dialog_height = (self.filtered.len() + 4).min(15) as u16;
        let dialog_area = centered_rect(dialog_width, dialog_height, area);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Select Model ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));

        let inner_area = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);

        let search_line = Line::from(vec![
            Span::styled("> ", Style::default().fg(theme.primary)),
            Span::styled(&self.query, Style::default().fg(theme.text)),
            Span::styled("▏", Style::default().fg(theme.primary)),
        ]);

        frame.render_widget(
            Paragraph::new(search_line),
            Rect {
                x: inner_area.x,
                y: inner_area.y,
                width: inner_area.width,
                height: 1,
            },
        );

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .filter_map(|&idx| {
                self.models.get(idx).map(|m| {
                    let is_selected = self
                        .state
                        .selected()
                        .and_then(|s| self.filtered.get(s))
                        .map(|&i| i == idx)
                        .unwrap_or(false);

                    let style = if is_selected {
                        Style::default().fg(theme.text).bg(theme.background_element)
                    } else {
                        Style::default().fg(theme.text)
                    };

                    let mut spans = vec![
                        Span::styled(&m.name, style),
                        Span::styled("  ", style),
                        Span::styled(&m.provider, Style::default().fg(theme.text_muted)),
                    ];
                    if self.is_current_model(m) {
                        spans.push(Span::styled(
                            "  (current)",
                            Style::default().fg(theme.text_muted),
                        ));
                    }

                    ListItem::new(Line::from(spans))
                })
            })
            .collect();

        let list = List::new(items);
        let list_area = Rect {
            x: inner_area.x,
            y: inner_area.y + 2,
            width: inner_area.width,
            height: inner_area.height.saturating_sub(2),
        };

        frame.render_stateful_widget(list, list_area, &mut self.state.clone());
    }
}

impl Default for ModelSelectDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}

#[cfg(test)]
mod tests {
    use super::{Model, ModelSelectDialog};
    use crate::theme::Theme;

    fn sample_models() -> Vec<Model> {
        vec![
            Model {
                id: "alpha".into(),
                name: "Alpha".into(),
                provider: "zeta".into(),
                context_window: 0,
            },
            Model {
                id: "beta".into(),
                name: "Beta".into(),
                provider: "zeta".into(),
                context_window: 0,
            },
        ]
    }

    fn render_to_string(dialog: &ModelSelectDialog) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();
        let theme = Theme::default();
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

    fn line_with<'a>(rendered: &'a str, needle: &str) -> &'a str {
        rendered
            .lines()
            .find(|line| line.contains(needle))
            .unwrap_or("")
    }

    #[test]
    fn current_model_is_marked_in_the_list() {
        let mut dialog = ModelSelectDialog::new();
        dialog.set_models(sample_models());
        dialog.set_current_model(Some("zeta/beta".to_string()));
        dialog.open();

        let rendered = render_to_string(&dialog);
        assert!(line_with(&rendered, "Beta").contains("(current)"));
        assert!(!line_with(&rendered, "Alpha").contains("(current)"));
    }

    #[test]
    fn no_current_model_marks_nothing() {
        let mut dialog = ModelSelectDialog::new();
        dialog.set_models(sample_models());
        dialog.set_current_model(None);
        dialog.open();

        let rendered = render_to_string(&dialog);
        assert!(!rendered.contains("(current)"));
    }

    #[test]
    fn blank_current_model_is_treated_as_unset() {
        let mut dialog = ModelSelectDialog::new();
        dialog.set_models(sample_models());
        dialog.set_current_model(Some("   ".to_string()));
        dialog.open();

        let rendered = render_to_string(&dialog);
        assert!(!rendered.contains("(current)"));
    }
}
