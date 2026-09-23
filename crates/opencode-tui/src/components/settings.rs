use std::collections::HashMap;
use std::sync::Arc;

use opencode_config::{Config as AppConfig, ProviderConfig};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::api::{
    ProviderAuthMethodInfo, ProviderAuthStatusInfo, ProviderOAuthStartInfo, ProviderSetupInfo,
};
use crate::components::Prompt;
use crate::context::{AppContext, ProviderInfo};

const SETTINGS_OUTER_H_PADDING: u16 = 2;
const SETTINGS_OUTER_V_PADDING: u16 = 1;
const V1_PROVIDER_IDS: &[&str] = &["ollama", "openai", "anthropic", "deepseek", "openrouter"];

pub struct SettingsView {
    selected_provider: usize,
    selected_model: usize,
    provider_setup: ProviderSetupInfo,
    openai_auth_methods: Vec<ProviderAuthMethodInfo>,
    auth_method_selection: Option<usize>,
    input_mode: Option<SettingsInputMode>,
    input_value: String,
    oauth_prompt: Option<ProviderOAuthStartInfo>,
    oauth_method: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsInputMode {
    ApiKey,
    OAuthCode,
    OllamaBaseUrl,
}

impl SettingsView {
    pub fn new() -> Self {
        Self {
            selected_provider: 0,
            selected_model: 0,
            provider_setup: ProviderSetupInfo::default(),
            openai_auth_methods: Vec::new(),
            auth_method_selection: None,
            input_mode: None,
            input_value: String::new(),
            oauth_prompt: None,
            oauth_method: None,
        }
    }

    pub fn set_provider_setup(&mut self, setup: ProviderSetupInfo) {
        self.provider_setup = setup;
    }

    pub fn set_openai_auth_methods(&mut self, methods: Vec<ProviderAuthMethodInfo>) {
        self.openai_auth_methods = methods;
        if let Some(selection) = self.auth_method_selection {
            if self.openai_auth_methods.is_empty() {
                self.auth_method_selection = None;
            } else {
                self.auth_method_selection =
                    Some(selection.min(self.openai_auth_methods.len() - 1));
            }
        }
    }

    pub fn openai_auth_methods(&self) -> &[ProviderAuthMethodInfo] {
        &self.openai_auth_methods
    }

    pub fn begin_auth_method_select(&mut self) {
        self.auth_method_selection = (!self.openai_auth_methods.is_empty()).then_some(0);
    }

    pub fn auth_method_selection(&self) -> Option<usize> {
        self.auth_method_selection
    }

    pub fn cancel_auth_method_select(&mut self) {
        self.auth_method_selection = None;
    }

    pub fn move_auth_method_up(&mut self) {
        if let Some(selection) = self.auth_method_selection {
            if selection > 0 {
                self.auth_method_selection = Some(selection - 1);
            }
        }
    }

    pub fn move_auth_method_down(&mut self) {
        if let Some(selection) = self.auth_method_selection {
            if selection + 1 < self.openai_auth_methods.len() {
                self.auth_method_selection = Some(selection + 1);
            }
        }
    }

    pub fn selected_auth_method(&self) -> Option<ProviderAuthMethodInfo> {
        let selection = self.auth_method_selection?;
        self.openai_auth_methods.get(selection).cloned()
    }

    pub fn begin_api_key_input(&mut self) {
        self.auth_method_selection = None;
        self.input_mode = Some(SettingsInputMode::ApiKey);
        self.input_value.clear();
        self.oauth_prompt = None;
        self.oauth_method = None;
    }

    pub fn begin_oauth_input(&mut self, method: usize, prompt: ProviderOAuthStartInfo) {
        self.auth_method_selection = None;
        self.input_mode = Some(SettingsInputMode::OAuthCode);
        self.input_value.clear();
        self.oauth_prompt = Some(prompt);
        self.oauth_method = Some(method);
    }

    pub fn begin_ollama_base_url_input(&mut self) {
        self.auth_method_selection = None;
        self.input_mode = Some(SettingsInputMode::OllamaBaseUrl);
        self.input_value = self.provider_setup.ollama_base_url.clone();
        self.oauth_prompt = None;
        self.oauth_method = None;
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = None;
        self.input_value.clear();
        self.oauth_prompt = None;
        self.oauth_method = None;
    }

    pub fn input_mode(&self) -> Option<SettingsInputMode> {
        self.input_mode
    }

    pub fn oauth_method(&self) -> Option<usize> {
        self.oauth_method
    }

    pub fn oauth_requires_code(&self) -> bool {
        match self
            .oauth_prompt
            .as_ref()
            .map(|prompt| prompt.method_type.as_str())
        {
            Some(method) => !method.eq_ignore_ascii_case("auto"),
            None => true,
        }
    }

    pub fn input_value(&self) -> String {
        self.input_value.trim().to_string()
    }

    pub fn handle_input(&mut self, c: char) {
        self.input_value.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.input_value.pop();
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

    pub fn selected_provider_ref(&self, context: &Arc<AppContext>) -> Option<String> {
        let providers = filtered_providers(context);
        providers
            .get(self.selected_provider)
            .map(|provider| provider.id.clone())
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
                Constraint::Length(8),
                Constraint::Min(10),
                Constraint::Length(9),
                Constraint::Length(7),
                Constraint::Length(4),
            ])
            .split(area);

        let effective_auth = self
            .provider_setup
            .effective_provider
            .as_ref()
            .and_then(|provider_id| self.provider_setup.auth.get(provider_id));

        let summary = vec![
            Line::from(Span::styled(
                "Settings > Provider",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Authoritative path: ",
                    Style::default().fg(theme.text_muted),
                ),
                Span::styled(
                    if self.provider_setup.authoritative_path.is_empty() {
                        "Settings > Provider".to_string()
                    } else {
                        self.provider_setup.authoritative_path.clone()
                    },
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Effective provider: ",
                    Style::default().fg(theme.text_muted),
                ),
                Span::styled(
                    self.provider_setup
                        .effective_provider
                        .clone()
                        .or(current_provider.clone())
                        .unwrap_or_else(|| "not selected".to_string()),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled("Effective model: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    self.provider_setup
                        .effective_model
                        .clone()
                        .or(current_model.clone())
                        .unwrap_or_else(|| "not selected".to_string()),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled("Selection source: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    self.provider_setup.selection_source.clone(),
                    Style::default().fg(theme.warning),
                ),
            ]),
            Line::from(vec![
                Span::styled("Auth state: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    effective_auth
                        .map(format_auth_summary)
                        .unwrap_or_else(|| "Not configured".to_string()),
                    Style::default().fg(
                        if effective_auth
                            .map(|status| status.configured)
                            .unwrap_or(false)
                        {
                            theme.success
                        } else {
                            theme.warning
                        },
                    ),
                ),
            ]),
            Line::from(vec![
                Span::styled("Ollama host: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    format!(
                        "{} ({})",
                        self.provider_setup.ollama_base_url,
                        self.provider_setup.ollama_base_url_source
                    ),
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

        let auth_panel = Paragraph::new(self.auth_lines(&providers, &theme))
            .block(
                Block::default()
                    .title(" Auth ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(auth_panel, layout[2]);

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
                    " to save the highlighted provider/model path.",
                    Style::default().fg(theme.text_muted),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press a for API key, l for login, x to clear auth, u for Ollama host, r to refresh.",
                Style::default().fg(theme.warning),
            )),
            Line::from(Span::styled(
                "Config and environment overrides remain supported, but this screen is the authoritative setup path.",
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
        frame.render_widget(notes, layout[3]);

        prompt.render(frame, layout[4]);
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

impl SettingsView {
    fn auth_lines(
        &self,
        providers: &[ProviderInfo],
        theme: &crate::theme::Theme,
    ) -> Vec<Line<'static>> {
        let selected_provider_id = providers
            .get(self.selected_provider)
            .map(|provider| provider.id.as_str());
        let selected_status =
            selected_provider_id.and_then(|provider_id| self.provider_setup.auth.get(provider_id));

        if selected_provider_id != Some("openai") {
            let mut lines = vec![Line::from(vec![
                Span::styled("Status: ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    selected_status
                        .map(format_auth_summary)
                        .unwrap_or_else(|| "Not configured".to_string()),
                    Style::default().fg(
                        if selected_status
                            .map(|status| status.configured)
                            .unwrap_or(false)
                        {
                            theme.success
                        } else {
                            theme.warning
                        },
                    ),
                ),
            ])];
            lines.push(Line::from(""));
            if selected_provider_id == Some("ollama") {
                match self.input_mode {
                    Some(SettingsInputMode::OllamaBaseUrl) => {
                        lines.push(Line::from(Span::styled(
                            "Enter Ollama host or base URL:",
                            Style::default().fg(theme.text),
                        )));
                        lines.push(Line::from(Span::styled(
                            format!("> {}", self.input_value),
                            Style::default().fg(theme.primary),
                        )));
                    }
                    _ => lines.push(Line::from(Span::styled(
                        format!(
                            "u Edit Ollama host ({})",
                            self.provider_setup.ollama_base_url_source
                        ),
                        Style::default().fg(theme.text),
                    ))),
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "Advanced auth overrides can still come from config or environment.",
                    Style::default().fg(theme.text_muted),
                )));
            }
            return lines;
        }

        let (status_text, status_color) = match self
            .provider_setup
            .auth
            .get("openai")
            .as_ref()
            .and_then(|status| status.auth_type.as_deref())
        {
            Some("api") => ("API key saved", theme.success),
            Some("oauth") => ("Login token saved", theme.success),
            Some("wellknown") => ("Auth saved", theme.success),
            Some(_) => ("Configured", theme.success),
            None if self
                .provider_setup
                .auth
                .get("openai")
                .as_ref()
                .map(|status| status.configured)
                .unwrap_or(false) =>
            {
                ("Configured", theme.success)
            }
            None => ("Not configured", theme.warning),
        };

        let mut lines = vec![Line::from(vec![
            Span::styled("Status: ", Style::default().fg(theme.text_muted)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ])];

        if let Some(selection) = self.auth_method_selection {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Choose ChatGPT/Codex auth method:",
                Style::default().fg(theme.text),
            )));
            for (index, method) in self.openai_auth_methods.iter().enumerate() {
                let style = if index == selection {
                    Style::default().fg(theme.text).bg(theme.background_element)
                } else {
                    Style::default().fg(theme.text)
                };
                let kind = if method.is_api() { "api" } else { "oauth" };
                lines.push(Line::from(Span::styled(
                    format!(
                        "{} {} ({})",
                        if index == selection { "▸" } else { " " },
                        method.name,
                        kind
                    ),
                    style,
                )));
            }
            lines.push(Line::from(Span::styled(
                "Enter to choose   Esc to cancel",
                Style::default().fg(theme.warning),
            )));
            return lines;
        }

        match self.input_mode {
            Some(SettingsInputMode::ApiKey) => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Enter OpenAI API key:",
                    Style::default().fg(theme.text),
                )));
                lines.push(Line::from(Span::styled(
                    format!("> {}", "*".repeat(self.input_value.chars().count())),
                    Style::default().fg(theme.primary),
                )));
            }
            Some(SettingsInputMode::OAuthCode) => {
                lines.push(Line::from(""));
                if let Some(prompt) = &self.oauth_prompt {
                    if !prompt.url.trim().is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled("Open: ", Style::default().fg(theme.text_muted)),
                            Span::styled(prompt.url.clone(), Style::default().fg(theme.primary)),
                        ]));
                    }
                    if !prompt.instructions.trim().is_empty() {
                        lines.push(Line::from(Span::styled(
                            prompt.instructions.clone(),
                            Style::default().fg(theme.text),
                        )));
                    }
                }
                if self.oauth_requires_code() {
                    lines.push(Line::from(Span::styled(
                        format!("> {}", self.input_value),
                        Style::default().fg(theme.primary),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "Complete the flow, then press Enter to finish (no code required).",
                        Style::default().fg(theme.warning),
                    )));
                }
            }
            Some(SettingsInputMode::OllamaBaseUrl) => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Enter Ollama host or base URL:",
                    Style::default().fg(theme.text),
                )));
                lines.push(Line::from(Span::styled(
                    format!("> {}", self.input_value),
                    Style::default().fg(theme.primary),
                )));
            }
            None => {
                lines.push(Line::from(""));
                let shortcut = if self.openai_auth_methods.len() > 1 {
                    "a API key   l Login (choose method)   x Clear"
                } else {
                    "a API key   l Login   x Clear"
                };
                lines.push(Line::from(Span::styled(
                    shortcut,
                    Style::default().fg(theme.text),
                )));
                if let Some(source) = self
                    .provider_setup
                    .auth
                    .get("openai")
                    .and_then(|status| status.source.as_ref())
                {
                    lines.push(Line::from(Span::styled(
                        format!("Credential source: {source}"),
                        Style::default().fg(theme.text_muted),
                    )));
                }
            }
        }

        lines
    }
}

impl Default for SettingsView {
    fn default() -> Self {
        Self::new()
    }
}

fn format_auth_summary(status: &ProviderAuthStatusInfo) -> String {
    let base = match status.auth_type.as_deref() {
        Some("api") => "API key configured",
        Some("oauth") => "OAuth login configured",
        Some("wellknown") => "Managed auth configured",
        Some("local") => "Local provider ready",
        Some(_) if status.configured => "Configured",
        _ => "Not configured",
    };
    match status.source.as_deref() {
        Some(source) if !source.is_empty() => format!("{base} via {source}"),
        _ => base.to_string(),
    }
}

pub fn provider_selection_patch(model_ref: String) -> AppConfig {
    AppConfig {
        model: Some(model_ref),
        ..Default::default()
    }
}

pub fn ollama_base_url_patch(base_url: String) -> AppConfig {
    let mut providers = HashMap::new();
    providers.insert(
        "ollama".to_string(),
        ProviderConfig {
            base_url: Some(base_url),
            ..Default::default()
        },
    );
    AppConfig {
        provider: Some(providers),
        ..Default::default()
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
        .filter(|provider| !opencode_provider::is_provider_temporarily_hidden(&provider.id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method(index: usize, method_type: &str, name: &str) -> ProviderAuthMethodInfo {
        ProviderAuthMethodInfo {
            index,
            method_type: method_type.to_string(),
            name: name.to_string(),
            description: method_type.to_string(),
        }
    }

    fn codex_methods() -> Vec<ProviderAuthMethodInfo> {
        vec![
            method(0, "oauth", "ChatGPT Pro/Plus (browser)"),
            method(1, "oauth", "ChatGPT Pro/Plus (headless)"),
            method(2, "api", "Manually enter API Key"),
        ]
    }

    #[test]
    fn openai_auth_methods_are_not_collapsed_to_first() {
        let mut view = SettingsView::new();
        view.set_openai_auth_methods(codex_methods());
        assert_eq!(view.openai_auth_methods().len(), 3);
        assert_eq!(view.openai_auth_methods()[0].method_type, "oauth");
        assert!(view.openai_auth_methods()[1].is_oauth());
        assert!(view.openai_auth_methods()[2].is_api());
    }

    #[test]
    fn auth_method_selection_navigates_all_methods() {
        let mut view = SettingsView::new();
        view.set_openai_auth_methods(codex_methods());
        view.begin_auth_method_select();
        assert_eq!(view.auth_method_selection(), Some(0));

        view.move_auth_method_down();
        let selected = view.selected_auth_method().expect("method selected");
        assert_eq!(selected.index, 1);
        assert_eq!(selected.name, "ChatGPT Pro/Plus (headless)");

        view.move_auth_method_down();
        view.move_auth_method_down();
        let selected = view.selected_auth_method().expect("method selected");
        assert_eq!(selected.index, 2);
        assert_eq!(selected.name, "Manually enter API Key");

        view.cancel_auth_method_select();
        assert_eq!(view.auth_method_selection(), None);
    }

    #[test]
    fn auto_oauth_does_not_require_a_code_value() {
        let mut view = SettingsView::new();
        view.begin_oauth_input(
            1,
            ProviderOAuthStartInfo {
                url: "https://auth.openai.com/codex/device".to_string(),
                method_type: "auto".to_string(),
                instructions: "Enter code: ABCD".to_string(),
            },
        );
        assert!(!view.oauth_requires_code());
    }

    #[test]
    fn code_oauth_requires_a_code_value() {
        let mut view = SettingsView::new();
        view.begin_oauth_input(
            0,
            ProviderOAuthStartInfo {
                url: "https://example.test".to_string(),
                method_type: "code".to_string(),
                instructions: "Paste code".to_string(),
            },
        );
        assert!(view.oauth_requires_code());
    }

    #[test]
    fn provider_auth_method_payload_deserializes_type_and_index() {
        let payload = r#"[
            {"index":0,"type":"oauth","name":"ChatGPT Pro/Plus (browser)","description":"oauth"},
            {"index":1,"type":"oauth","name":"ChatGPT Pro/Plus (headless)","description":"oauth"},
            {"index":2,"type":"api","name":"Manually enter API Key","description":"api"}
        ]"#;
        let parsed: Vec<ProviderAuthMethodInfo> =
            serde_json::from_str(payload).expect("payload deserializes");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[1].index, 1);
        assert!(parsed[1].is_oauth());
        assert!(parsed[2].is_api());
    }
}
