use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::api::ApiClient;
use crate::context::{KeybindRegistry, SessionContext};
use crate::event::EventBus;
use crate::router::Router;
use crate::theme::Theme;

const DEFAULT_TIPS_HIDDEN: bool = true;

#[derive(Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub models: Vec<ModelInfo>,
}

#[derive(Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub supports_vision: bool,
    pub supports_tools: bool,
}

#[derive(Clone)]
pub struct McpServerStatus {
    pub name: String,
    pub status: McpConnectionStatus,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum McpConnectionStatus {
    Connected,
    Disconnected,
    Failed,
    NeedsAuth,
    NeedsClientRegistration,
    Disabled,
}

#[derive(Clone)]
pub struct LspStatus {
    pub id: String,
    pub root: String,
    pub status: LspConnectionStatus,
}

#[derive(Clone, Debug)]
pub enum LspConnectionStatus {
    Connected,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarMode {
    Auto,
    Show,
    Hide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageDensity {
    Compact,
    Cozy,
}

impl MessageDensity {
    pub fn from_str_lossy(s: &str) -> Self {
        if s.eq_ignore_ascii_case("cozy") {
            Self::Cozy
        } else {
            Self::Compact
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Cozy => "cozy",
        }
    }
}

pub struct AppContext {
    pub theme: RwLock<Theme>,
    pub theme_name: RwLock<String>,
    pub router: RwLock<Router>,
    pub keybind: RwLock<KeybindRegistry>,
    pub session: RwLock<SessionContext>,
    pub providers: RwLock<Vec<ProviderInfo>>,
    pub mcp_servers: RwLock<Vec<McpServerStatus>>,
    pub lsp_status: RwLock<Vec<LspStatus>>,
    pub event_bus: EventBus,
    pub current_agent: RwLock<String>,
    pub current_model: RwLock<Option<String>>,
    pub current_provider: RwLock<Option<String>>,
    pub current_variant: RwLock<Option<String>>,
    pub directory: RwLock<String>,
    pub show_sidebar: RwLock<bool>,
    pub show_header: RwLock<bool>,
    pub show_scrollbar: RwLock<bool>,
    pub tips_hidden: RwLock<bool>,
    pub prompt_hidden: RwLock<bool>,
    pub sidebar_mode: RwLock<SidebarMode>,
    pub animations_enabled: RwLock<bool>,
    pub pending_permissions: RwLock<usize>,
    pub show_timestamps: RwLock<bool>,
    pub show_thinking: RwLock<bool>,
    pub show_tool_calls: RwLock<bool>,
    pub show_tool_details: RwLock<bool>,
    pub message_density: RwLock<MessageDensity>,
    pub semantic_highlight: RwLock<bool>,
    pub has_connected_provider: RwLock<bool>,
    /// FEAT-048: whether experimental background subagents are enabled
    /// (reference `capabilities.experimentalBackgroundSubagents`).
    pub experimental_background_subagents: RwLock<bool>,
    ui_kv: RwLock<UiKv>,
    pub api_client: RwLock<Option<Arc<ApiClient>>>,
}

impl AppContext {
    pub fn new() -> Self {
        Self::new_with_config(&opencode_config::Config::default())
    }

    /// FEAT-053: build the context, seeding display toggles from config.
    ///
    /// Precedence for every display flag is persisted `kv.json` override >
    /// config startup default > built-in default, so an explicit in-session
    /// toggle is never silently reverted on restart.
    pub fn new_with_config(config: &opencode_config::Config) -> Self {
        Self::from_ui_kv(UiKv::load(), config.tui.as_ref())
    }

    fn from_ui_kv(ui_kv: UiKv, tui: Option<&opencode_config::TuiConfig>) -> Self {
        let default_theme_name = format!("opencode@{}", detect_terminal_theme_mode());
        let default_theme = Theme::by_name(&default_theme_name).unwrap_or_else(Theme::dark);
        let message_density = seed_string(
            &ui_kv,
            "message_density",
            tui.and_then(|t| t.message_density.as_deref()),
            "compact",
        );
        Self {
            theme: RwLock::new(default_theme),
            theme_name: RwLock::new(default_theme_name),
            router: RwLock::new(Router::new()),
            keybind: RwLock::new(KeybindRegistry::new()),
            session: RwLock::new(SessionContext::new()),
            providers: RwLock::new(Vec::new()),
            mcp_servers: RwLock::new(Vec::new()),
            lsp_status: RwLock::new(Vec::new()),
            event_bus: EventBus::new(),
            current_agent: RwLock::new("build".to_string()),
            current_model: RwLock::new(None),
            current_provider: RwLock::new(None),
            current_variant: RwLock::new(None),
            directory: RwLock::new(String::new()),
            show_sidebar: RwLock::new(false),
            show_header: RwLock::new(seed_bool(
                &ui_kv,
                "header_visible",
                tui.and_then(|t| t.header),
                true,
            )),
            show_scrollbar: RwLock::new(seed_bool(
                &ui_kv,
                "scrollbar_visible",
                tui.and_then(|t| t.scrollbar),
                false,
            )),
            tips_hidden: RwLock::new(seed_bool(
                &ui_kv,
                "tips_hidden",
                tui.and_then(|t| t.tips_hidden),
                DEFAULT_TIPS_HIDDEN,
            )),
            prompt_hidden: RwLock::new(seed_bool(
                &ui_kv,
                "prompt_hidden",
                tui.and_then(|t| t.prompt_hidden),
                false,
            )),
            sidebar_mode: RwLock::new(SidebarMode::Auto),
            animations_enabled: RwLock::new(true),
            pending_permissions: RwLock::new(0),
            show_timestamps: RwLock::new(seed_timestamps(&ui_kv, tui.and_then(|t| t.timestamps))),
            show_thinking: RwLock::new(seed_bool(
                &ui_kv,
                "thinking_visibility",
                tui.and_then(|t| t.thinking),
                true,
            )),
            show_tool_calls: RwLock::new(seed_bool(
                &ui_kv,
                "tool_calls_visibility",
                tui.and_then(|t| t.tool_calls),
                true,
            )),
            show_tool_details: RwLock::new(seed_bool(
                &ui_kv,
                "tool_details_visibility",
                tui.and_then(|t| t.tool_details),
                true,
            )),
            message_density: RwLock::new(MessageDensity::from_str_lossy(&message_density)),
            semantic_highlight: RwLock::new(seed_bool(
                &ui_kv,
                "semantic_highlight",
                tui.and_then(|t| t.semantic_highlight),
                true,
            )),
            has_connected_provider: RwLock::new(false),
            experimental_background_subagents: RwLock::new(false),
            ui_kv: RwLock::new(ui_kv),
            api_client: RwLock::new(None),
        }
    }

    pub fn navigate(&self, route: crate::router::Route) {
        self.router.write().navigate(route);
    }

    pub fn current_route(&self) -> crate::router::Route {
        self.router.read().current().clone()
    }

    pub fn toggle_sidebar(&self) {
        let mut sidebar = self.show_sidebar.write();
        *sidebar = !*sidebar;
    }

    pub fn toggle_header(&self) {
        let mut show = self.show_header.write();
        *show = !*show;
        self.ui_kv.write().set_bool("header_visible", *show);
    }

    pub fn toggle_scrollbar(&self) {
        let mut show = self.show_scrollbar.write();
        *show = !*show;
        self.ui_kv.write().set_bool("scrollbar_visible", *show);
    }

    pub fn toggle_tips_hidden(&self) {
        let mut hidden = self.tips_hidden.write();
        *hidden = !*hidden;
        self.ui_kv.write().set_bool("tips_hidden", *hidden);
    }

    pub fn toggle_prompt_hidden(&self) {
        let mut hidden = self.prompt_hidden.write();
        *hidden = !*hidden;
        self.ui_kv.write().set_bool("prompt_hidden", *hidden);
    }

    pub fn set_model(&self, model: String, provider: String) {
        self.set_model_selection(model, Some(provider));
    }

    pub fn set_model_selection(&self, model: String, provider: Option<String>) {
        *self.current_model.write() = Some(model);
        *self.current_provider.write() = provider;
    }

    pub fn set_model_variant(&self, variant: Option<String>) {
        *self.current_variant.write() = variant;
    }

    pub fn current_model_variant(&self) -> Option<String> {
        self.current_variant.read().clone()
    }

    pub fn set_agent(&self, agent: String) {
        *self.current_agent.write() = agent;
    }

    pub fn toggle_animations(&self) {
        let mut enabled = self.animations_enabled.write();
        *enabled = !*enabled;
    }

    pub fn set_pending_permissions(&self, count: usize) {
        *self.pending_permissions.write() = count;
    }

    pub fn set_has_connected_provider(&self, connected: bool) {
        *self.has_connected_provider.write() = connected;
    }

    pub fn toggle_timestamps(&self) {
        let mut show = self.show_timestamps.write();
        *show = !*show;
        self.ui_kv.write().set_timestamps(*show);
    }

    pub fn toggle_thinking(&self) {
        let mut show = self.show_thinking.write();
        *show = !*show;
        self.ui_kv.write().set_bool("thinking_visibility", *show);
    }

    pub fn toggle_tool_details(&self) {
        let mut show = self.show_tool_details.write();
        *show = !*show;
        self.ui_kv
            .write()
            .set_bool("tool_details_visibility", *show);
    }

    pub fn toggle_tool_calls(&self) {
        let mut show = self.show_tool_calls.write();
        *show = !*show;
        self.ui_kv.write().set_bool("tool_calls_visibility", *show);
    }

    pub fn toggle_message_density(&self) {
        let mut density = self.message_density.write();
        *density = match *density {
            MessageDensity::Compact => MessageDensity::Cozy,
            MessageDensity::Cozy => MessageDensity::Compact,
        };
        self.ui_kv
            .write()
            .set_string("message_density", density.as_str());
    }

    pub fn toggle_semantic_highlight(&self) {
        let mut enabled = self.semantic_highlight.write();
        *enabled = !*enabled;
        self.ui_kv.write().set_bool("semantic_highlight", *enabled);
    }

    pub fn toggle_theme_mode(&self) -> bool {
        let current = normalize_theme_name(&self.current_theme_name());
        let Some((base, variant)) = split_theme_variant(&current) else {
            return false;
        };
        let next = if variant == "dark" { "light" } else { "dark" };
        self.set_theme_by_name(&format!("{base}@{next}"))
    }

    pub fn set_theme_by_name(&self, name: &str) -> bool {
        if let Some(theme) = Theme::by_name(name) {
            *self.theme.write() = theme;
            *self.theme_name.write() = normalize_theme_name(name);
            return true;
        }
        false
    }

    pub fn current_theme_name(&self) -> String {
        self.theme_name.read().clone()
    }

    pub fn available_theme_names(&self) -> Vec<String> {
        let mut names = Theme::builtin_theme_names()
            .into_iter()
            .flat_map(|name| [format!("{name}@dark"), format!("{name}@light")])
            .collect::<Vec<_>>();
        names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        names
    }

    pub fn set_api_client(&self, client: Arc<ApiClient>) {
        *self.api_client.write() = Some(client);
    }

    pub fn get_api_client(&self) -> Option<Arc<ApiClient>> {
        self.api_client.read().clone()
    }

    /// FEAT-048: capability gate for background subagents.
    pub fn set_experimental_background_subagents(&self, enabled: bool) {
        *self.experimental_background_subagents.write() = enabled;
    }

    pub fn experimental_background_subagents(&self) -> bool {
        *self.experimental_background_subagents.read()
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_theme_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return format!("opencode@{}", detect_terminal_theme_mode());
    }

    if let Some((base, variant)) = split_theme_variant(trimmed) {
        return format!("{base}@{variant}");
    }

    if trimmed.eq_ignore_ascii_case("dark") {
        return "opencode@dark".to_string();
    }
    if trimmed.eq_ignore_ascii_case("light") {
        return "opencode@light".to_string();
    }

    format!("{trimmed}@dark")
}

fn detect_terminal_theme_mode() -> &'static str {
    if let Ok(mode) = std::env::var("OPENCODE_THEME_MODE") {
        if mode.eq_ignore_ascii_case("light") {
            return "light";
        }
        if mode.eq_ignore_ascii_case("dark") {
            return "dark";
        }
    }

    // Common terminal convention: COLORFGBG="fg;bg", where bg in 0..=6 is dark
    // and 7..=15 is light.
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(last) = colorfgbg.split(';').next_back() {
            if let Ok(code) = last.parse::<u8>() {
                return if code <= 6 { "dark" } else { "light" };
            }
        }
    }

    "dark"
}

fn split_theme_variant(name: &str) -> Option<(&str, &str)> {
    let (base, variant) = name.rsplit_once('@').or_else(|| name.rsplit_once(':'))?;
    if base.is_empty() || !matches!(variant, "dark" | "light") {
        return None;
    }
    Some((base, variant))
}

/// FEAT-053 precedence: persisted `kv.json` override > config startup default >
/// built-in default.
fn seed_bool(kv: &UiKv, key: &str, config_value: Option<bool>, builtin: bool) -> bool {
    kv.get_bool_opt(key).or(config_value).unwrap_or(builtin)
}

fn seed_string(kv: &UiKv, key: &str, config_value: Option<&str>, builtin: &str) -> String {
    kv.get_string_opt(key)
        .or_else(|| config_value.map(|value| value.to_string()))
        .unwrap_or_else(|| builtin.to_string())
}

fn seed_timestamps(kv: &UiKv, config_value: Option<bool>) -> bool {
    kv.get_timestamps_opt().or(config_value).unwrap_or(false)
}

#[derive(Default)]
struct UiKv {
    path: Option<PathBuf>,
    values: HashMap<String, Value>,
}

impl UiKv {
    fn load() -> Self {
        let Some(path) = ui_kv_path() else {
            return Self::default();
        };

        let values = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<HashMap<String, Value>>(&content).ok())
            .unwrap_or_default();

        Self {
            path: Some(path),
            values,
        }
    }

    fn get_bool_opt(&self, key: &str) -> Option<bool> {
        match self.values.get(key) {
            Some(Value::Bool(flag)) => Some(*flag),
            _ => None,
        }
    }

    fn get_timestamps_opt(&self) -> Option<bool> {
        match self.values.get("timestamps") {
            Some(Value::String(value)) if value.eq_ignore_ascii_case("show") => Some(true),
            Some(Value::String(value)) if value.eq_ignore_ascii_case("hide") => Some(false),
            Some(Value::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    fn set_timestamps(&mut self, show: bool) {
        let value = if show { "show" } else { "hide" };
        self.values
            .insert("timestamps".to_string(), Value::String(value.to_string()));
        self.persist();
    }

    fn set_bool(&mut self, key: &str, value: bool) {
        self.values.insert(key.to_string(), Value::Bool(value));
        self.persist();
    }

    fn get_string_opt(&self, key: &str) -> Option<String> {
        match self.values.get(key) {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn set_string(&mut self, key: &str, value: &str) {
        self.values
            .insert(key.to_string(), Value::String(value.to_string()));
        self.persist();
    }

    fn persist(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                tracing::warn!(%err, "failed to create TUI kv directory");
                return;
            }
        }

        let payload = match serde_json::to_string_pretty(&self.values) {
            Ok(payload) => payload,
            Err(err) => {
                tracing::warn!(%err, "failed to encode TUI kv state");
                return;
            }
        };

        if let Err(err) = fs::write(path, payload) {
            tracing::warn!(%err, "failed to persist TUI kv state");
        }
    }
}

fn ui_kv_path() -> Option<PathBuf> {
    dirs::state_dir()
        .map(|dir| dir.join("opencode").join("kv.json"))
        .or_else(|| {
            dirs::home_dir().map(|home| {
                home.join(".local")
                    .join("state")
                    .join("opencode")
                    .join("kv.json")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::{AppContext, MessageDensity, UiKv, DEFAULT_TIPS_HIDDEN};
    use serde_json::json;

    fn tui_with<F: FnOnce(&mut opencode_config::TuiConfig)>(f: F) -> opencode_config::TuiConfig {
        let mut tui = opencode_config::TuiConfig::default();
        f(&mut tui);
        tui
    }

    #[test]
    fn config_seeds_display_flags_when_kv_is_absent() {
        let tui = tui_with(|tui| {
            tui.tool_calls = Some(false);
            tui.thinking = Some(false);
            tui.message_density = Some("cozy".to_string());
        });

        let context = AppContext::from_ui_kv(UiKv::default(), Some(&tui));

        assert!(!*context.show_tool_calls.read());
        assert!(!*context.show_thinking.read());
        assert_eq!(*context.message_density.read(), MessageDensity::Cozy);
    }

    #[test]
    fn persisted_kv_override_beats_config_startup_default() {
        let mut kv = UiKv::default();
        kv.values
            .insert("tool_calls_visibility".to_string(), json!(true));
        let tui = tui_with(|tui| tui.tool_calls = Some(false));

        let context = AppContext::from_ui_kv(kv, Some(&tui));

        assert!(*context.show_tool_calls.read());
    }

    #[test]
    fn builtin_defaults_apply_when_kv_and_config_are_absent() {
        let context = AppContext::from_ui_kv(UiKv::default(), None);

        assert!(*context.show_tool_calls.read());
        assert!(!*context.show_scrollbar.read());
        assert_eq!(*context.message_density.read(), MessageDensity::Compact);
    }

    #[test]
    fn tips_default_to_hidden_when_unset() {
        assert!(DEFAULT_TIPS_HIDDEN);
        let kv = UiKv::default();
        assert_eq!(kv.get_bool_opt("tips_hidden"), None);
    }

    #[test]
    fn prompt_defaults_to_visible_when_unset() {
        let context = AppContext::from_ui_kv(UiKv::default(), None);
        assert!(!*context.prompt_hidden.read());
    }

    #[test]
    fn prompt_hidden_seeds_from_config_when_kv_is_absent() {
        let tui = tui_with(|tui| tui.prompt_hidden = Some(true));
        let context = AppContext::from_ui_kv(UiKv::default(), Some(&tui));
        assert!(*context.prompt_hidden.read());
    }

    #[test]
    fn persisted_prompt_hidden_beats_config_startup_default() {
        let mut kv = UiKv::default();
        kv.values.insert("prompt_hidden".to_string(), json!(true));
        let tui = tui_with(|tui| tui.prompt_hidden = Some(false));

        let context = AppContext::from_ui_kv(kv, Some(&tui));

        assert!(*context.prompt_hidden.read());
    }

    #[test]
    fn toggle_prompt_hidden_flips_and_persists_to_ui_kv() {
        let context = AppContext::from_ui_kv(UiKv::default(), None);
        assert!(!*context.prompt_hidden.read());

        context.toggle_prompt_hidden();
        assert!(*context.prompt_hidden.read());
        assert_eq!(
            context.ui_kv.read().get_bool_opt("prompt_hidden"),
            Some(true)
        );

        context.toggle_prompt_hidden();
        assert!(!*context.prompt_hidden.read());
        assert_eq!(
            context.ui_kv.read().get_bool_opt("prompt_hidden"),
            Some(false)
        );
    }

    #[test]
    fn timestamps_opt_decodes_string_and_bool_encodings() {
        let mut kv = UiKv::default();
        assert_eq!(kv.get_timestamps_opt(), None);
        kv.values
            .insert("timestamps".to_string(), serde_json::json!("show"));
        assert_eq!(kv.get_timestamps_opt(), Some(true));
        kv.values
            .insert("timestamps".to_string(), serde_json::json!("hide"));
        assert_eq!(kv.get_timestamps_opt(), Some(false));
        kv.values
            .insert("timestamps".to_string(), serde_json::json!(true));
        assert_eq!(kv.get_timestamps_opt(), Some(true));
    }
}
