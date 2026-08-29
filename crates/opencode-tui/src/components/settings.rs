use std::sync::Arc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::components::Prompt;
use crate::context::{AppContext, ProviderInfo};

const SETTINGS_OUTER_H_PADDING: u16 = 2;
const SETTINGS_OUTER_V_PADDING: u16 = 1;
const V1_PROVIDER_IDS: &[&str] = &["openai", "anthropic", "deepseek", "openrouter"];

pub struct SettingsView {
    selected_provider: usize,
    selected_model: usize,
}

impl SettingsView {
    pub fn new() -> Self {
        Self {
            selected_provider: 0,
            selected_model: 0,
        }
    }

    pub fn sync_from_context(&mut self, context: &Arc<AppContext>) {
        let providers = filtered_providers(context);
        if providers.is_empty() {
            self.selected_provider = 0;
            self.selected_model = 0;
            return;
        }

        let current_provider = context.current_provider.read().clone();
        let current_model = context.current_model.read().clone();

        if let Some(provider_id) = current_provider.as_deref() {
            if let Some(index) = providers
                .iter()
                .position(|provider| provider.id == provider_id)
            {
                self.selected_provider = index;
            }
        }

        self.selected_provider = self
            .selected_provider
            .min(providers.len().saturating_sub(1));

        if let Some(model_ref) = current_model.as_deref() {
            if let Some(provider) = providers.get(self.selected_provider) {
                if let Some(index) = provider
                    .models
                    .iter()
                    .position(|model| model.id == model_ref)
                {
                    self.selected_model = index;
                } else {
                    self.selected_model = 0;
                }
            }
        } else {
            self.selected_model = 0;
        }

        self.clamp_model_selection(&providers);
    }

    pub fn move_up(&mut self, context: &Arc<AppContext>) {
        if self.selected_provider > 0 {
            self.selected_provider -= 1;
        }
        let providers = filtered_providers(context);
        self.clamp_model_selection(&providers);
    }

    pub fn move_down(&mut self, context: &Arc<AppContext>) {
        let providers = filtered_providers(context);
        if self.selected_provider + 1 < providers.len() {
            self.selected_provider += 1;
        }
        self.clamp_model_selection(&providers);
    }

    pub fn move_left(&mut self, context: &Arc<AppContext>) {
        let providers = filtered_providers(context);
        if providers.is_empty() {
            return;
        }
        if self.selected_model > 0 {
            self.selected_model -= 1;
        }
        self.clamp_model_selection(&providers);
    }

    pub fn move_right(&mut self, context: &Arc<AppContext>) {
        let providers = filtered_providers(context);
        let Some(provider) = providers.get(self.selected_provider) else {
            return;
        };
        if self.selected_model + 1 < provider.models.len() {
            self.selected_model += 1;
        }
        self.clamp_model_selection(&providers);
    }

    pub fn selected_model_ref(&self, context: &Arc<AppContext>) -> Option<(String, String)> {
        let providers = filtered_providers(context);
        let provider = providers.get(self.selected_provider)?;
        let model = provider.models.get(self.selected_model)?;
        Some((model.id.clone(), provider.id.clone()))
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        prompt: &Prompt,
        context: &Arc<AppContext>,
    ) {
        let area = Rect {
            x: area.x.saturating_add(SETTINGS_OUTER_H_PADDING),
            y: area.y.saturating_add(SETTINGS_OUTER_V_PADDING),
            width: area
                .width
                .saturating_sub(SETTINGS_OUTER_H_PADDING.saturating_mul(2)),
            height: area
                .height
                .saturating_sub(SETTINGS_OUTER_V_PADDING.saturating_mul(2)),
        };
        if area.width == 0 || area.height == 0 {
            return;
        }

        let theme = context.theme.read().clone();
        let providers = filtered_providers(context);
        let current_provider = context.current_provider.read().clone();
        let current_model = context.current_model.read().clone();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(10),
                Constraint::Length(6),
                Constraint::Length(4),
            ])
            .split(area);

        let summary = vec![
            Line::from(Span::styled(
                "Settings > Provider",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Current provider: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    current_provider
                        .clone()
                        .unwrap_or_else(|| "not selected".to_string()),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled("Current model: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    current_model
                        .clone()
                        .unwrap_or_else(|| "not selected".to_string()),
                    Style::default().fg(theme.text),
                ),
            ]),
        ];
        frame.render_widget(Paragraph::new(summary), layout[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(layout[1]);

        let provider_items = providers
            .iter()
            .enumerate()
            .map(|(index, provider)| {
                let is_selected = index == self.selected_provider;
                let is_active = current_provider.as_deref() == Some(provider.id.as_str());
                let prefix = if is_active { "● " } else { "  " };
                let style = if is_selected {
                    Style::default().fg(theme.text).bg(theme.background_element)
                } else if is_active {
                    Style::default().fg(theme.success)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", prefix, provider.name),
                    style,
                )))
            })
            .collect::<Vec<_>>();

        let provider_block = Block::default()
            .title(" Providers ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));
        frame.render_widget(List::new(provider_items).block(provider_block), body[0]);

        let model_items = providers
            .get(self.selected_provider)
            .map(|provider| {
                provider
                    .models
                    .iter()
                    .enumerate()
                    .map(|(index, model)| {
                        let is_selected = index == self.selected_model;
                        let is_active = current_model.as_deref() == Some(model.id.as_str());
                        let style = if is_selected {
                            Style::default().fg(theme.text).bg(theme.background_element)
                        } else if is_active {
                            Style::default().fg(theme.success)
                        } else {
                            Style::default().fg(theme.text)
                        };
                        let label = if is_active {
                            format!("{}  active", model.name)
                        } else {
                            model.name.clone()
                        };
                        ListItem::new(Line::from(Span::styled(label, style)))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                vec![ListItem::new(Line::from(Span::styled(
                    "No in-scope providers available",
                    Style::default().fg(theme.text_muted),
                )))]
            });

        let models_title = providers
            .get(self.selected_provider)
            .map(|provider| format!(" Models ({}) ", provider.id))
            .unwrap_or_else(|| " Models ".to_string());
        let model_block = Block::default()
            .title(models_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));
        frame.render_widget(List::new(model_items).block(model_block), body[1]);

        let notes = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Use ", Style::default().fg(theme.text_muted)),
                Span::styled("Up/Down", Style::default().fg(theme.text)),
                Span::styled(" for providers and ", Style::default().fg(theme.text_muted)),
                Span::styled("Left/Right", Style::default().fg(theme.text)),
                Span::styled(" for models.", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(theme.text_muted)),
                Span::styled("Enter", Style::default().fg(theme.text)),
                Span::styled(
                    " to apply the highlighted model for this session.",
                    Style::default().fg(theme.text_muted),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Credential entry and OpenAI login are deferred to START-015.",
                Style::default().fg(theme.warning),
            )),
            Line::from(Span::styled(
                "This screen intentionally reuses session-local model selection instead of first-run onboarding.",
                Style::default().fg(theme.text_muted),
            )),
        ])
        .block(
            Block::default()
                .title(" Notes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(notes, layout[2]);

        prompt.render(frame, layout[3]);
    }

    fn clamp_model_selection(&mut self, providers: &[ProviderInfo]) {
        if providers.is_empty() {
            self.selected_provider = 0;
            self.selected_model = 0;
            return;
        }

        self.selected_provider = self
            .selected_provider
            .min(providers.len().saturating_sub(1));
        let model_count = providers[self.selected_provider].models.len();
        if model_count == 0 {
            self.selected_model = 0;
        } else {
            self.selected_model = self.selected_model.min(model_count.saturating_sub(1));
        }
    }
}

impl Default for SettingsView {
    fn default() -> Self {
        Self::new()
    }
}

fn filtered_providers(context: &Arc<AppContext>) -> Vec<ProviderInfo> {
    let providers = context.providers.read();
    V1_PROVIDER_IDS
        .iter()
        .filter_map(|provider_id| {
            providers
                .iter()
                .find(|provider| provider.id == *provider_id)
        })
        .cloned()
        .collect()
}
