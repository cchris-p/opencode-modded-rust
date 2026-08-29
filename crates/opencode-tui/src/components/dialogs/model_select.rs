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
                id: "deepseek-v4-flash".into(),
                name: "DeepSeek V4 Flash".into(),
                provider: "deepseek".into(),
                context_window: 128000,
            },
            Model {
                id: "anthropic/claude-sonnet-5".into(),
                name: "Claude Sonnet 5 (OpenRouter)".into(),
                provider: "openrouter".into(),
                context_window: 200000,
            },
            Model {
                id: "openai/gpt-5-mini".into(),
                name: "GPT-5 Mini (OpenRouter)".into(),
                provider: "openrouter".into(),
                context_window: 400000,
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

                    ListItem::new(Line::from(vec![
                        Span::styled(&m.name, style),
                        Span::styled("  ", style),
                        Span::styled(&m.provider, Style::default().fg(theme.text_muted)),
                    ]))
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
