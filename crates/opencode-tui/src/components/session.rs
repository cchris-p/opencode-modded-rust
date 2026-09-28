use chrono::Utc;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::message_palette;
use super::sidebar::SidebarState;
use crate::components::{Prompt, Sidebar};
use crate::context::{AppContext, Message, MessagePart, MessageRole, SidebarMode};

const SIDEBAR_WIDTH: u16 = 42;
const HEADER_NARROW_THRESHOLD: u16 = 80;
const MOUSE_SCROLL_LINES: usize = 3;
const MESSAGE_BLOCK_RIGHT_PADDING: usize = 1;
const SIDEBAR_CLOSE_BUTTON_WIDTH: u16 = 3;
const SIDEBAR_OPEN_BUTTON_WIDTH: u16 = 3;

/// Test-only counter used to prove that per-frame layout is bounded by the
/// render window instead of growing with the session history.
#[cfg(test)]
static LAYOUT_RENDERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct ThinkingToggleHit {
    line_index: usize,
    reasoning_id: String,
}

struct ToolToggleHit {
    line_index: usize,
    tool_id: String,
}

/// Cached per-message layout height. Only the line *count* is retained, not the
/// rendered lines, so a long session does not keep a second copy of the whole
/// transcript in memory. `sig` covers every input that can change the height.
#[derive(Clone, Copy)]
struct CachedMessageLayout {
    sig: u64,
    height: usize,
}

/// The rendered lines for a single message plus toggle hit indices that are
/// relative to the start of `lines`.
#[derive(Default)]
struct RenderedBody {
    lines: Vec<Line<'static>>,
    thinking_hits: Vec<ThinkingToggleHit>,
    tool_hits: Vec<ToolToggleHit>,
}

/// Everything a single message needs to be laid out, borrowed for one frame.
struct MessageRenderCtx<'a> {
    messages: &'a [Message],
    last_assistant_idx: Option<usize>,
    pending_assistant_idx: Option<usize>,
    fallback_model: Option<&'a str>,
    theme: &'a crate::theme::Theme,
    user_bg: Color,
    assistant_bg: Color,
    thinking_bg: Color,
    assistant_border: Color,
    thinking_border: Color,
    show_thinking: bool,
    show_timestamps: bool,
    show_tool_calls: bool,
    show_tool_details: bool,
    semantic_hl: bool,
    collapsed_reasoning: &'a HashSet<String>,
    expanded_tool_calls: &'a HashSet<String>,
    keybind: &'a crate::context::KeybindRegistry,
    include_background_subagents: bool,
    content_width: usize,
}

pub struct SessionView {
    context: Arc<AppContext>,
    session_id: String,
    scroll_offset: usize,
    rendered_line_count: usize,
    measured_line_count: usize,
    messages_viewport_height: usize,
    collapsed_reasoning: HashSet<String>,
    thinking_toggle_hits: Vec<ThinkingToggleHit>,
    expanded_tool_calls: HashSet<String>,
    tool_toggle_hits: Vec<ToolToggleHit>,
    last_messages_area: Option<Rect>,
    line_to_message: Vec<Option<String>>,
    layout_cache: HashMap<String, CachedMessageLayout>,
    revert_layout: Option<(u64, usize)>,
    sidebar_state: SidebarState,
    sidebar_close_button_area: Option<Rect>,
    sidebar_open_button_area: Option<Rect>,
}

impl SessionView {
    pub fn new(context: Arc<AppContext>, session_id: String) -> Self {
        Self {
            context,
            session_id,
            scroll_offset: 0,
            rendered_line_count: 0,
            measured_line_count: 0,
            messages_viewport_height: 0,
            collapsed_reasoning: HashSet::new(),
            thinking_toggle_hits: Vec::new(),
            expanded_tool_calls: HashSet::new(),
            tool_toggle_hits: Vec::new(),
            last_messages_area: None,
            line_to_message: Vec::new(),
            layout_cache: HashMap::new(),
            revert_layout: None,
            sidebar_state: SidebarState::default(),
            sidebar_close_button_area: None,
            sidebar_open_button_area: None,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, prompt: &Prompt) -> Option<Rect> {
        let show_sidebar = *self.context.show_sidebar.read();
        let sidebar_mode = self.context.sidebar_mode.read().clone();
        // Determine effective sidebar visibility (overlay on all widths)
        let effective_show = match sidebar_mode {
            SidebarMode::Auto => show_sidebar,
            SidebarMode::Show => show_sidebar,
            SidebarMode::Hide => false,
        };

        // Always render main content full-width; sidebar floats above it.
        if effective_show {
            self.sidebar_open_button_area = None;
            let prompt_area = self.render_main(frame, area, prompt);
            self.render_sidebar_overlay(frame, area);
            prompt_area
        } else {
            self.sidebar_state.reset_hidden();
            self.sidebar_close_button_area = None;
            let prompt_area = self.render_main(frame, area, prompt);
            self.render_sidebar_open_button(frame, area);
            prompt_area
        }
    }

    fn render_sidebar_overlay(&mut self, frame: &mut Frame, area: Rect) {
        let theme = self.context.theme.read();
        let sidebar = Sidebar::new(self.context.clone(), self.session_id.clone());

        // Overlay on the right portion of the screen
        let overlay_width = SIDEBAR_WIDTH.min(area.width);
        let sidebar_area = Rect {
            x: area.x + area.width.saturating_sub(overlay_width),
            y: area.y,
            width: overlay_width,
            height: area.height,
        };

        let overlay_tint = tint_sidebar_overlay(theme.background_menu, theme.primary);

        // Render a subtle underlay just for the sidebar area.
        let underlay = Block::default().style(Style::default().bg(overlay_tint));
        frame.render_widget(underlay, sidebar_area);

        self.sidebar_close_button_area = Some(Rect {
            x: sidebar_area.x.saturating_add(
                sidebar_area
                    .width
                    .saturating_sub(SIDEBAR_CLOSE_BUTTON_WIDTH),
            ),
            y: sidebar_area.y,
            width: SIDEBAR_CLOSE_BUTTON_WIDTH.min(sidebar_area.width),
            height: 1,
        });

        sidebar.render(frame, sidebar_area, &mut self.sidebar_state, true);
        if let Some(close_area) = self.sidebar_close_button_area {
            let close = Paragraph::new("✕")
                .style(Style::default().fg(theme.text).bg(overlay_tint))
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(close, close_area);
        }
    }

    fn render_sidebar_open_button(&mut self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            self.sidebar_open_button_area = None;
            return;
        }
        let theme = self.context.theme.read();
        let button = Rect {
            x: area
                .x
                .saturating_add(area.width.saturating_sub(SIDEBAR_OPEN_BUTTON_WIDTH)),
            y: area
                .y
                .saturating_add(1)
                .min(area.y + area.height.saturating_sub(1)),
            width: SIDEBAR_OPEN_BUTTON_WIDTH.min(area.width),
            height: 1,
        };
        self.sidebar_open_button_area = Some(button);
        let glyph = Paragraph::new("☰")
            .style(
                Style::default()
                    .fg(theme.primary)
                    .bg(theme.background_element)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(glyph, button);
    }

    fn render_main(&mut self, frame: &mut Frame, area: Rect, prompt: &Prompt) -> Option<Rect> {
        // Apply breathing boundary padding
        let area = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };
        if area.width == 0 || area.height == 0 {
            return None;
        }

        let show_header = {
            let user_pref = *self.context.show_header.read();
            if !user_pref {
                false
            } else {
                true
            }
        };
        let header_height = if show_header {
            if area.width < HEADER_NARROW_THRESHOLD {
                3u16
            } else {
                2u16
            }
        } else {
            0u16
        };
        // Child sessions get a subagent footer (label, position, Parent/Prev/Next).
        // Root sessions keep the general footer disabled, matching current behavior.
        let is_subagent_session = {
            let session_ctx = self.context.session.read();
            session_ctx
                .sessions
                .get(&self.session_id)
                .is_some_and(|session| session.parent_id.is_some())
        };
        let session_footer_height = if is_subagent_session { 1u16 } else { 0u16 };
        let desired_prompt_height = prompt.desired_height(area.width).max(3);
        let total_height = area.height;
        let available_after_header = total_height.saturating_sub(header_height);
        let available_after_header_footer =
            available_after_header.saturating_sub(session_footer_height);
        let prompt_empty = prompt.get_input().trim().is_empty();
        // BUG-054: measure the current transcript before sizing the messages
        // pane. `desired_messages_height` below reads `self.measured_line_count`;
        // if that value is a frame stale, the pane is one line shorter than the
        // content while the transcript is shorter than the pane, and the
        // one-line scroll offset shifts every visible queued row on each stream
        // event. Measuring first keeps the pane height, the virtual total, and
        // the viewport consistent within one frame. `rendered_line_count` is
        // still updated by the paint pass so follow/scroll behavior is
        // unchanged.
        self.render_messages(frame, area, true);
        let viewport_height = if self.messages_viewport_height == 0 {
            usize::from(available_after_header_footer)
        } else {
            self.messages_viewport_height
        };
        let near_bottom =
            self.scroll_offset.saturating_add(viewport_height) >= self.rendered_line_count;
        let prompt_hidden = *self.context.prompt_hidden.read();
        let show_prompt = !prompt_hidden && (!prompt_empty || near_bottom);
        let prompt_height = if show_prompt {
            desired_prompt_height.min(available_after_header_footer)
        } else {
            0
        };

        let layout = if !show_prompt {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height),
                    Constraint::Min(0),
                    Constraint::Length(session_footer_height.min(available_after_header)),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ])
                .split(area)
        } else if available_after_header_footer <= prompt_height {
            // Small terminal fallback: keep stable bottom input behavior.
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height),
                    Constraint::Min(0),
                    Constraint::Length(session_footer_height.min(available_after_header)),
                    Constraint::Length(prompt_height.min(available_after_header_footer)),
                    Constraint::Min(0),
                ])
                .split(area)
        } else {
            // Follow content: prompt sits directly below rendered messages.
            let max_messages_height = available_after_header_footer.saturating_sub(prompt_height);
            let desired_messages_height = (self.measured_line_count as u16)
                .max(1)
                .min(max_messages_height);

            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height),
                    Constraint::Length(desired_messages_height),
                    Constraint::Length(session_footer_height.min(available_after_header)),
                    Constraint::Length(prompt_height),
                    Constraint::Min(0),
                ])
                .split(area)
        };

        if show_header && layout[0].height > 0 {
            self.render_header(frame, layout[0]);
        }
        self.render_messages(frame, layout[1], false);
        if layout[2].height > 0 {
            self.render_session_footer(frame, layout[2]);
        }
        if show_prompt && layout[3].height > 0 {
            prompt.render(frame, layout[3]);
            Some(layout[3])
        } else {
            None
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let theme = self.context.theme.read();
        let session_ctx = self.context.session.read();
        let is_narrow = area.width < HEADER_NARROW_THRESHOLD;

        let title = session_ctx
            .sessions
            .get(&self.session_id)
            .map(|s| s.title.as_str())
            .unwrap_or("New Session");

        let messages = session_ctx
            .messages
            .get(&self.session_id)
            .cloned()
            .unwrap_or_default();

        // Find the last assistant message with output tokens > 0 for token display
        let last_assistant = messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, MessageRole::Assistant) && m.tokens.output > 0);

        // TS parity: cost only sums assistant messages.
        let total_cost: f64 = messages
            .iter()
            .filter(|m| matches!(m.role, MessageRole::Assistant))
            .map(|m| m.cost)
            .sum();
        let mut context_and_cost = None;
        if let Some(assistant_msg) = last_assistant {
            let t = &assistant_msg.tokens;
            let total_tokens = t.input + t.output + t.reasoning + t.cache_read + t.cache_write;
            if total_tokens > 0 {
                let model_context_limit = {
                    let providers = self.context.providers.read();
                    let current_model = self.context.current_model.read();
                    assistant_msg
                        .model
                        .as_ref()
                        .or(current_model.as_ref())
                        .and_then(|model_id| {
                            providers.iter().find_map(|p| {
                                p.models
                                    .iter()
                                    .find(|m| {
                                        m.id == *model_id
                                            || m.id
                                                .rsplit_once('/')
                                                .map(|(_, suffix)| suffix == model_id)
                                                .unwrap_or(false)
                                    })
                                    .map(|m| m.context_window)
                            })
                        })
                        .unwrap_or(0)
                };

                let mut context_text = format_number(total_tokens);
                if model_context_limit > 0 {
                    let pct =
                        ((total_tokens as f64 / model_context_limit as f64) * 100.0).round() as u64;
                    context_text.push_str(&format!(" {}%", pct));
                }

                let cost_text = format!("${:.2}", total_cost);
                context_and_cost = Some(format!("{} ({})", context_text, cost_text));
            }
        }

        let content = if is_narrow {
            let mut lines = vec![Line::from(vec![Span::styled(
                format!(" # {}", title),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )])];
            if let Some(info) = context_and_cost {
                lines.push(Line::from(Span::styled(
                    format!("   {}", info),
                    Style::default().fg(theme.text_muted),
                )));
            }
            lines
        } else {
            let title_span = Span::styled(
                format!(" # {}", title),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            );

            let mut title_line_spans = vec![title_span];
            if let Some(right_text) = context_and_cost {
                let right_text_len = right_text.len();
                let available = area.width as usize;
                let title_display_len = title.len() + 3; // " # " prefix
                if available > title_display_len + right_text_len + 2 {
                    let padding = available.saturating_sub(title_display_len + right_text_len + 2);
                    title_line_spans.push(Span::raw(" ".repeat(padding)));
                    title_line_spans.push(Span::styled(
                        right_text,
                        Style::default().fg(theme.text_muted),
                    ));
                    title_line_spans.push(Span::raw(" "));
                }
            }
            vec![Line::from(title_line_spans)]
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(theme.border)),
            )
            .style(Style::default().bg(theme.background_panel));

        frame.render_widget(paragraph, area);
    }

    /// Footer for child (subagent) sessions: agent label, sibling position, and
    /// Parent/Prev/Next affordances. Returns false for root sessions so the
    /// caller can fall back to the general footer.
    fn render_subagent_footer(&self, frame: &mut Frame, area: Rect) -> bool {
        let theme = self.context.theme.read().clone();
        let session_ctx = self.context.session.read();
        let Some(session) = session_ctx.sessions.get(&self.session_id) else {
            return false;
        };
        let Some(parent_id) = session.parent_id.clone() else {
            return false;
        };

        // Siblings sorted by creation time ascending, matching the reference
        // `SubagentFooter` position calculation.
        let mut siblings: Vec<&crate::context::Session> = session_ctx
            .sessions
            .values()
            .filter(|candidate| candidate.parent_id.as_deref() == Some(parent_id.as_str()))
            .collect();
        siblings.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let total = siblings.len();
        let index = siblings
            .iter()
            .position(|candidate| candidate.id == self.session_id)
            .map(|position| position + 1)
            .unwrap_or(1);

        let label = subagent_label(&session.title);
        let keybind = self.context.keybind.read();

        let mut left = vec![Span::styled(
            label,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        )];
        if total > 0 {
            left.push(Span::styled(
                format!(" ({} of {})", index, total),
                Style::default().fg(theme.text_muted),
            ));
        }

        let right = vec![
            Span::styled("Parent ", Style::default().fg(theme.text)),
            Span::styled(
                keybind.print("session_parent"),
                Style::default().fg(theme.text_muted),
            ),
            Span::raw("  "),
            Span::styled("Prev ", Style::default().fg(theme.text)),
            Span::styled(
                keybind.print("session_child_cycle_reverse"),
                Style::default().fg(theme.text_muted),
            ),
            Span::raw("  "),
            Span::styled("Next ", Style::default().fg(theme.text)),
            Span::styled(
                keybind.print("session_child_cycle"),
                Style::default().fg(theme.text_muted),
            ),
        ];

        let left_len: usize = left.iter().map(|span| span.content.len()).sum();
        let right_len: usize = right.iter().map(|span| span.content.len()).sum();
        let available = area.width as usize;
        let mut spans = left;
        if available > left_len + right_len + 1 {
            spans.push(Span::raw(" ".repeat(available - left_len - right_len)));
        } else {
            spans.push(Span::raw(" "));
        }
        spans.extend(right);

        let paragraph =
            Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.background_panel));
        frame.render_widget(paragraph, area);
        true
    }

    fn render_session_footer(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        if self.render_subagent_footer(frame, area) {
            return;
        }

        let theme = self.context.theme.read();
        let directory = self.context.directory.read().clone();
        let mcp_servers = self.context.mcp_servers.read();
        let lsp_status = self.context.lsp_status.read();
        let permission_count = *self.context.pending_permissions.read();
        let has_connected_provider = *self.context.has_connected_provider.read();

        let connected_lsp = lsp_status
            .iter()
            .filter(|s| matches!(s.status, crate::context::LspConnectionStatus::Connected))
            .count();
        let connected_mcp = mcp_servers
            .iter()
            .filter(|s| matches!(s.status, crate::context::McpConnectionStatus::Connected))
            .count();
        let has_mcp_failures = mcp_servers
            .iter()
            .any(|s| matches!(s.status, crate::context::McpConnectionStatus::Failed));
        let has_mcp_registration_needed = mcp_servers.iter().any(|s| {
            matches!(
                s.status,
                crate::context::McpConnectionStatus::NeedsClientRegistration
            )
        });
        let has_mcp_issues = has_mcp_failures || has_mcp_registration_needed;
        let show_connect_hint =
            !has_connected_provider && Utc::now().timestamp().rem_euclid(15) >= 10;

        let mut right_spans = Vec::new();
        if show_connect_hint {
            right_spans.push(Span::styled(
                "Get started ",
                Style::default().fg(theme.text_muted),
            ));
            right_spans.push(Span::styled("/connect", Style::default().fg(theme.primary)));
        } else {
            if permission_count > 0 {
                right_spans.push(Span::styled(
                    format!(
                        "△ {} Permission{}",
                        permission_count,
                        if permission_count == 1 { "" } else { "s" }
                    ),
                    Style::default().fg(theme.warning),
                ));
                right_spans.push(Span::raw("  "));
            }

            right_spans.push(Span::styled(
                format!("• {} LSP", connected_lsp),
                Style::default().fg(if connected_lsp > 0 {
                    theme.success
                } else {
                    theme.text_muted
                }),
            ));
            right_spans.push(Span::raw("  "));

            if connected_mcp > 0 || has_mcp_issues {
                let mcp_color = if has_mcp_failures {
                    theme.error
                } else if has_mcp_registration_needed {
                    theme.warning
                } else {
                    theme.success
                };
                right_spans.push(Span::styled(
                    format!("⊙ {} MCP", connected_mcp),
                    Style::default().fg(mcp_color),
                ));
                right_spans.push(Span::raw("  "));
            }

            right_spans.push(Span::styled(
                "/status",
                Style::default().fg(theme.text_muted),
            ));
        }

        let right_text_len: usize = right_spans.iter().map(|s| s.content.len()).sum();
        let dir_len = directory.len();
        let available = area.width as usize;
        let mut line_spans = vec![Span::styled(
            directory,
            Style::default().fg(theme.text_muted),
        )];
        if available > dir_len + right_text_len + 1 {
            line_spans.push(Span::raw(" ".repeat(available - dir_len - right_text_len)));
        } else {
            line_spans.push(Span::raw(" "));
        }
        line_spans.extend(right_spans);

        let paragraph =
            Paragraph::new(Line::from(line_spans)).style(Style::default().bg(theme.background));
        frame.render_widget(paragraph, area);
    }

    /// Render the messages pane.
    ///
    /// When `measure_only` is set, only the per-message layout cache and
    /// `self.measured_line_count` are updated; nothing is painted and the
    /// viewport/scroll state is left untouched. `render` calls this once in
    /// measure-only mode so the pane can be sized from the current transcript
    /// instead of the previous frame's total (BUG-054).
    fn render_messages(&mut self, frame: &mut Frame, area: Rect, measure_only: bool) {
        if area.height == 0 || area.width == 0 {
            self.last_messages_area = None;
            self.rendered_line_count = 0;
            self.measured_line_count = 0;
            self.messages_viewport_height = 0;
            self.thinking_toggle_hits.clear();
            self.tool_toggle_hits.clear();
            self.line_to_message.clear();
            return;
        }

        let was_near_bottom = self.is_near_bottom(2);
        let theme = self.context.theme.read();
        let user_bg = message_palette::user_message_bg(&theme);
        let assistant_bg = message_palette::assistant_message_bg(&theme);
        let thinking_bg = message_palette::thinking_message_bg(&theme);
        let assistant_border = message_palette::assistant_border_color(&theme);
        let thinking_border = message_palette::thinking_border_color(&theme);
        let show_scrollbar = *self.context.show_scrollbar.read() && area.width > 3;
        let messages_area = if show_scrollbar {
            Rect {
                x: area.x,
                y: area.y,
                width: area.width.saturating_sub(1),
                height: area.height,
            }
        } else {
            area
        };
        let scrollbar_area = show_scrollbar.then_some(Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y,
            width: 1,
            height: area.height,
        });
        if !measure_only {
            self.last_messages_area = Some(messages_area);
        }
        let content_width = usize::from(messages_area.width.saturating_sub(1));
        let show_thinking = *self.context.show_thinking.read();
        let show_timestamps = *self.context.show_timestamps.read();
        let show_tool_calls = *self.context.show_tool_calls.read();
        let show_tool_details = *self.context.show_tool_details.read();
        let semantic_hl = *self.context.semantic_highlight.read();
        let fallback_model = self.context.current_model.read().clone();
        let include_background_subagents = self.context.experimental_background_subagents();
        let keybind = self.context.keybind.read();

        let session_ctx = self.context.session.read();
        let empty_messages: Vec<Message> = Vec::new();
        let messages = session_ctx
            .messages
            .get(&self.session_id)
            .unwrap_or(&empty_messages);
        let revert_info = session_ctx.revert.get(&self.session_id).cloned();

        let last_assistant_idx = messages
            .iter()
            .rposition(|m| matches!(m.role, MessageRole::Assistant));
        let completed_assistant_idx = messages
            .iter()
            .rposition(|m| matches!(m.role, MessageRole::Assistant) && m.completed_at.is_some());
        let pending_assistant_idx = messages
            .iter()
            .enumerate()
            .filter(|(idx, m)| {
                matches!(m.role, MessageRole::Assistant)
                    && m.completed_at.is_none()
                    && completed_assistant_idx.map_or(true, |completed| *idx > completed)
            })
            .map(|(idx, _)| idx)
            .last();

        let mut globals_hasher = DefaultHasher::new();
        content_width.hash(&mut globals_hasher);
        show_thinking.hash(&mut globals_hasher);
        show_timestamps.hash(&mut globals_hasher);
        show_tool_calls.hash(&mut globals_hasher);
        show_tool_details.hash(&mut globals_hasher);
        semantic_hl.hash(&mut globals_hasher);
        include_background_subagents.hash(&mut globals_hasher);
        fallback_model.hash(&mut globals_hasher);
        last_assistant_idx.hash(&mut globals_hasher);
        pending_assistant_idx.hash(&mut globals_hasher);
        revert_info.is_some().hash(&mut globals_hasher);
        toggle_state_hash(&self.collapsed_reasoning, &self.expanded_tool_calls)
            .hash(&mut globals_hasher);
        keybind
            .leader_chord("session_child_first")
            .hash(&mut globals_hasher);
        let globals_hash = globals_hasher.finish();

        let ctx = MessageRenderCtx {
            messages,
            last_assistant_idx,
            pending_assistant_idx,
            fallback_model: fallback_model.as_deref(),
            theme: &theme,
            user_bg,
            assistant_bg,
            thinking_bg,
            assistant_border,
            thinking_border,
            show_thinking,
            show_timestamps,
            show_tool_calls,
            show_tool_details,
            semantic_hl,
            collapsed_reasoning: &self.collapsed_reasoning,
            expanded_tool_calls: &self.expanded_tool_calls,
            keybind: &keybind,
            include_background_subagents,
            content_width,
        };

        // Revert card: rendered only when its inputs change, otherwise reused.
        if revert_info.is_none() {
            self.revert_layout = None;
        }
        let mut revert_height = 0usize;
        if let Some(revert) = revert_info.as_ref() {
            let sig = hash_revert(revert, content_width);
            revert_height = match self.revert_layout {
                Some((cached_sig, cached_height)) if cached_sig == sig => cached_height,
                _ => {
                    let card = super::revert_card::render_revert_card(revert, &theme);
                    let painted = paint_block_lines(
                        card,
                        theme.background_panel,
                        theme.warning,
                        content_width,
                    );
                    let height = painted.len();
                    self.revert_layout = Some((sig, height));
                    height
                }
            };
        }

        // Pass 1: ensure every message has a valid cached height. Only messages
        // whose rendering inputs changed are laid out here; the rest reuse the
        // cached line count. This is what keeps per-frame layout bounded.
        let mut heights: Vec<usize> = Vec::with_capacity(messages.len());
        for (idx, msg) in messages.iter().enumerate() {
            let sig = message_sig(&ctx, msg, idx, globals_hash);
            let height = match self.layout_cache.get(&msg.id) {
                Some(cached) if cached.sig == sig => cached.height,
                _ => {
                    let body = render_message_body(&ctx, msg, idx);
                    let height = body.lines.len();
                    self.layout_cache
                        .insert(msg.id.clone(), CachedMessageLayout { sig, height });
                    height
                }
            };
            heights.push(height);
        }
        if self.layout_cache.len() > messages.len() {
            let live: HashSet<&str> = messages.iter().map(|m| m.id.as_str()).collect();
            self.layout_cache.retain(|id, _| live.contains(id.as_str()));
        }

        // Virtual whole-session line total so scroll, follow and scrollbar keep
        // whole-session semantics even though only a window is laid out.
        let mut total_lines = 0usize;
        if revert_info.is_some() {
            total_lines += revert_height;
            if !messages.is_empty() {
                total_lines += 1;
            }
        }
        for (idx, _msg) in messages.iter().enumerate() {
            total_lines += leading_spacing(messages, idx) + heights[idx];
        }
        if measure_only {
            self.measured_line_count = total_lines;
            return;
        }
        self.rendered_line_count = total_lines;
        self.messages_viewport_height = usize::from(messages_area.height);

        let max_scroll = self.max_scroll_offset();
        if was_near_bottom || self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        // Bounded render window around the viewport (plus overscan) so the
        // Paragraph only ever wraps window-relative lines.
        let viewport = usize::from(messages_area.height);
        let overscan = viewport.max(1);
        let window_start = self.scroll_offset.saturating_sub(overscan);
        let window_end = self
            .scroll_offset
            .saturating_add(viewport)
            .saturating_add(overscan);

        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut line_to_message: Vec<Option<String>> = Vec::new();
        let mut thinking_hits: Vec<ThinkingToggleHit> = Vec::new();
        let mut tool_hits: Vec<ToolToggleHit> = Vec::new();
        let mut window_start_line = 0usize;
        let mut started = false;
        let mut cursor = 0usize;

        if let Some(revert) = revert_info.as_ref() {
            if cursor < window_end && cursor + revert_height > window_start {
                if !started {
                    window_start_line = cursor;
                    started = true;
                }
                let card = super::revert_card::render_revert_card(revert, &theme);
                let painted =
                    paint_block_lines(card, theme.background_panel, theme.warning, content_width);
                append_non_message_lines(&mut lines, &mut line_to_message, painted);
            }
            cursor += revert_height;
            if !messages.is_empty() {
                if cursor < window_end && cursor + 1 > window_start {
                    if !started {
                        window_start_line = cursor;
                        started = true;
                    }
                    push_spacing_lines(&mut lines, &mut line_to_message, 1);
                }
                cursor += 1;
            }
        }

        for (idx, msg) in messages.iter().enumerate() {
            let spacing = leading_spacing(messages, idx);
            if spacing > 0 {
                if cursor < window_end && cursor + spacing > window_start {
                    if !started {
                        window_start_line = cursor;
                        started = true;
                    }
                    push_spacing_lines(&mut lines, &mut line_to_message, spacing);
                }
                cursor += spacing;
            }
            let body_end = cursor + heights[idx];
            if cursor < window_end && body_end > window_start {
                if !started {
                    window_start_line = cursor;
                    started = true;
                }
                let body = render_message_body(&ctx, msg, idx);
                let base = window_start_line + lines.len();
                for hit in &body.thinking_hits {
                    thinking_hits.push(ThinkingToggleHit {
                        line_index: base + hit.line_index,
                        reasoning_id: hit.reasoning_id.clone(),
                    });
                }
                for hit in &body.tool_hits {
                    tool_hits.push(ToolToggleHit {
                        line_index: base + hit.line_index,
                        tool_id: hit.tool_id.clone(),
                    });
                }
                append_message_lines(&mut lines, &mut line_to_message, &msg.id, body.lines);
            }
            cursor = body_end;
        }

        self.line_to_message = line_to_message;
        self.thinking_toggle_hits = thinking_hits;
        self.tool_toggle_hits = tool_hits;

        let window_scroll = self
            .scroll_offset
            .saturating_sub(window_start_line)
            .min(u16::MAX as usize) as u16;
        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(theme.border)),
            )
            .style(Style::default().bg(theme.background_panel))
            .scroll((window_scroll, 0));

        frame.render_widget(paragraph, messages_area);
        if let Some(scroll_area) = scrollbar_area {
            let mut scrollbar_state = ScrollbarState::new(self.rendered_line_count)
                .position(self.scroll_offset)
                .viewport_content_length(self.messages_viewport_height.max(1));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .track_style(Style::default().fg(theme.border_subtle))
                .thumb_symbol("█")
                .thumb_style(Style::default().fg(theme.primary));
            frame.render_stateful_widget(scrollbar, scroll_area, &mut scrollbar_state);
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16) -> bool {
        let Some(area) = self.last_messages_area else {
            return false;
        };

        let max_x = area.x.saturating_add(area.width);
        let max_y = area.y.saturating_add(area.height);
        if col < area.x || col >= max_x || row < area.y || row >= max_y {
            return false;
        }

        let line_index = self.scroll_offset + usize::from(row.saturating_sub(area.y));
        if line_index >= self.rendered_line_count {
            return false;
        }

        if let Some(tool_id) = self
            .tool_toggle_hits
            .iter()
            .find(|hit| hit.line_index == line_index)
            .map(|hit| hit.tool_id.clone())
        {
            if !self.expanded_tool_calls.insert(tool_id.clone()) {
                self.expanded_tool_calls.remove(&tool_id);
            }
            return true;
        }

        let Some(reasoning_id) = self
            .thinking_toggle_hits
            .iter()
            .find(|hit| hit.line_index == line_index)
            .map(|hit| hit.reasoning_id.clone())
        else {
            return false;
        };

        // Toggle the explicit collapse membership for this block.
        if !self.collapsed_reasoning.insert(reasoning_id.clone()) {
            self.collapsed_reasoning.remove(&reasoning_id);
        }
        true
    }

    pub fn handle_sidebar_click(&mut self, col: u16, row: u16) -> bool {
        if point_in_optional_rect(self.sidebar_open_button_area, col, row) {
            *self.context.show_sidebar.write() = true;
            self.sidebar_open_button_area = None;
            return true;
        }
        if point_in_optional_rect(self.sidebar_close_button_area, col, row) {
            *self.context.show_sidebar.write() = false;
            self.sidebar_close_button_area = None;
            return true;
        }
        self.sidebar_state.handle_click(col, row)
    }

    pub fn is_point_in_sidebar(&self, col: u16, row: u16) -> bool {
        self.sidebar_state.contains_sidebar_point(col, row)
    }

    pub fn scroll_sidebar_up_at(&mut self, col: u16, row: u16) -> bool {
        self.sidebar_state.scroll_up_at(col, row)
    }

    pub fn scroll_sidebar_down_at(&mut self, col: u16, row: u16) -> bool {
        self.sidebar_state.scroll_down_at(col, row)
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = self.max_scroll_offset();
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_up_by(&mut self, lines: usize) {
        let lines = lines.max(1);
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down_by(&mut self, lines: usize) {
        let lines = lines.max(1);
        let max_scroll = self.max_scroll_offset();
        self.scroll_offset = self.scroll_offset.saturating_add(lines).min(max_scroll);
    }

    pub fn scroll_up_mouse(&mut self) {
        self.scroll_up_by(MOUSE_SCROLL_LINES);
    }

    pub fn scroll_down_mouse(&mut self) {
        self.scroll_down_by(MOUSE_SCROLL_LINES);
    }

    pub fn scroll_page_up(&mut self) {
        let step = self.messages_viewport_height.saturating_sub(1).max(1);
        self.scroll_offset = self.scroll_offset.saturating_sub(step);
    }

    pub fn scroll_page_down(&mut self) {
        let step = self.messages_viewport_height.saturating_sub(1).max(1);
        let max_scroll = self.max_scroll_offset();
        self.scroll_offset = (self.scroll_offset + step).min(max_scroll);
    }

    pub fn scroll_to_message(&mut self, message_id: &str) {
        if let Some(first_line) = self.message_first_line(message_id) {
            self.scroll_offset = first_line.min(self.max_scroll_offset());
        }
    }

    /// Absolute first line of a message's body, computed from the cached
    /// per-message heights so that a jump to a message outside the current
    /// render window still re-anchors the window on the next frame.
    fn message_first_line(&self, message_id: &str) -> Option<usize> {
        let session_ctx = self.context.session.read();
        let messages = session_ctx.messages.get(&self.session_id)?;
        let idx = messages.iter().position(|m| m.id == message_id)?;

        let mut total = 0usize;
        if session_ctx.revert.contains_key(&self.session_id) {
            total += self.revert_layout.map(|(_, height)| height).unwrap_or(0);
            total += 1;
        }
        for (j, msg) in messages.iter().enumerate().take(idx) {
            total += leading_spacing(messages, j);
            total += self
                .layout_cache
                .get(&msg.id)
                .map(|cached| cached.height)
                .unwrap_or(0);
        }
        Some(total + leading_spacing(messages, idx))
    }

    fn max_scroll_offset(&self) -> usize {
        self.rendered_line_count
            .saturating_sub(self.messages_viewport_height)
    }

    fn is_near_bottom(&self, tolerance_lines: usize) -> bool {
        self.max_scroll_offset().saturating_sub(self.scroll_offset) <= tolerance_lines
    }
}

fn leading_spacing(messages: &[Message], idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    let prev_role = &messages[idx - 1].role;
    let role = &messages[idx].role;
    if *prev_role != *role || matches!(role, MessageRole::User) {
        1
    } else {
        0
    }
}

fn role_tag(role: &MessageRole) -> u8 {
    match role {
        MessageRole::User => 0,
        MessageRole::Assistant => 1,
        MessageRole::System => 2,
    }
}

/// Hashes cheap length/metadata fields. Two messages with equal length fields
/// are assumed to render identically for non-mutable (completed) messages.
fn hash_message_len_fields(msg: &Message, h: &mut DefaultHasher) {
    msg.content.len().hash(h);
    msg.parts.len().hash(h);
    for part in &msg.parts {
        match part {
            MessagePart::Text { text } => {
                0u8.hash(h);
                text.len().hash(h);
            }
            MessagePart::Reasoning { text } => {
                1u8.hash(h);
                text.len().hash(h);
            }
            MessagePart::File { path, mime } => {
                2u8.hash(h);
                path.len().hash(h);
                mime.len().hash(h);
            }
            MessagePart::Image { url } => {
                3u8.hash(h);
                url.len().hash(h);
            }
            MessagePart::ToolCall {
                id,
                name,
                arguments,
            } => {
                4u8.hash(h);
                id.len().hash(h);
                name.hash(h);
                arguments.len().hash(h);
            }
            MessagePart::ToolResult {
                id,
                result,
                is_error,
            } => {
                5u8.hash(h);
                id.len().hash(h);
                result.len().hash(h);
                is_error.hash(h);
            }
        }
    }
    msg.completed_at.is_some().hash(h);
    msg.finish.hash(h);
    msg.error.as_ref().map(|e| e.len()).hash(h);
    msg.agent.hash(h);
    msg.model.hash(h);
    msg.mode.hash(h);
    msg.tokens.input.hash(h);
    msg.tokens.output.hash(h);
    msg.tokens.reasoning.hash(h);
    msg.tokens.cache_read.hash(h);
    msg.tokens.cache_write.hash(h);
    msg.cost.to_bits().hash(h);
}

/// Full content hash, used for messages that can still mutate in place
/// (in-flight assistants and user messages without a completion timestamp).
fn hash_message_full_content(msg: &Message, h: &mut DefaultHasher) {
    msg.content.hash(h);
    for part in &msg.parts {
        match part {
            MessagePart::Text { text } => {
                0u8.hash(h);
                text.hash(h);
            }
            MessagePart::Reasoning { text } => {
                1u8.hash(h);
                text.hash(h);
            }
            MessagePart::File { path, mime } => {
                2u8.hash(h);
                path.hash(h);
                mime.hash(h);
            }
            MessagePart::Image { url } => {
                3u8.hash(h);
                url.hash(h);
            }
            MessagePart::ToolCall {
                id,
                name,
                arguments,
            } => {
                4u8.hash(h);
                id.hash(h);
                name.hash(h);
                arguments.hash(h);
            }
            MessagePart::ToolResult {
                id,
                result,
                is_error,
            } => {
                5u8.hash(h);
                id.hash(h);
                result.hash(h);
                is_error.hash(h);
            }
        }
    }
}

fn message_sig(ctx: &MessageRenderCtx, msg: &Message, idx: usize, globals_hash: u64) -> u64 {
    let mut h = DefaultHasher::new();
    globals_hash.hash(&mut h);
    msg.id.hash(&mut h);
    role_tag(&msg.role).hash(&mut h);
    hash_message_len_fields(msg, &mut h);

    let prev_role = if idx == 0 {
        None
    } else {
        Some(role_tag(&ctx.messages[idx - 1].role))
    };
    prev_role.hash(&mut h);

    let is_queued = matches!(msg.role, MessageRole::User)
        && ctx
            .pending_assistant_idx
            .is_some_and(|pending| idx > pending);
    is_queued.hash(&mut h);

    let is_active = matches!(msg.role, MessageRole::Assistant)
        && ctx.last_assistant_idx == Some(idx)
        && msg.finish.is_none()
        && msg.error.is_none();
    is_active.hash(&mut h);
    (ctx.last_assistant_idx == Some(idx)).hash(&mut h);

    if msg.completed_at.is_none() || ctx.last_assistant_idx == Some(idx) {
        hash_message_full_content(msg, &mut h);
    }
    h.finish()
}

/// Order-independent hash of the collapse/expand toggle sets.
fn toggle_state_hash(collapsed: &HashSet<String>, expanded: &HashSet<String>) -> u64 {
    let mut acc: u64 =
        (collapsed.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (expanded.len() as u64);
    for id in collapsed {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        acc ^= h.finish();
    }
    for id in expanded {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        acc ^= h.finish();
    }
    acc
}

fn hash_revert(revert: &crate::context::RevertInfo, content_width: usize) -> u64 {
    let mut h = DefaultHasher::new();
    revert.message_id.hash(&mut h);
    revert.part_id.hash(&mut h);
    revert.snapshot.hash(&mut h);
    revert.diff.hash(&mut h);
    content_width.hash(&mut h);
    h.finish()
}

fn render_message_body(ctx: &MessageRenderCtx, msg: &Message, idx: usize) -> RenderedBody {
    #[cfg(test)]
    LAYOUT_RENDERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let mut out = RenderedBody::default();
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut line_to_message: Vec<Option<String>> = Vec::new();
    let mut visible_tool_ids: HashSet<String> = HashSet::new();

    match msg.role {
        MessageRole::User => {
            let message_bg = ctx.user_bg;
            let message_border = user_border_color_for_agent(msg.agent.as_deref(), ctx.theme);
            let is_queued = ctx
                .pending_assistant_idx
                .is_some_and(|pending| idx > pending);
            let user_lines = super::session_message::render_user_message(
                msg,
                ctx.theme,
                ctx.show_timestamps,
                msg.agent.as_deref(),
                is_queued,
            );
            append_message_lines(
                &mut lines,
                &mut line_to_message,
                &msg.id,
                paint_block_lines(user_lines, message_bg, message_border, ctx.content_width),
            );
        }
        MessageRole::Assistant => {
            let message_bg = ctx.assistant_bg;
            let message_border = ctx.assistant_border;
            let message_thinking_bg = ctx.thinking_bg;
            let message_thinking_border = ctx.thinking_border;
            let mut tool_results: HashMap<String, (String, bool)> = HashMap::new();
            for part in &msg.parts {
                if let MessagePart::ToolResult {
                    id,
                    result,
                    is_error,
                } = part
                {
                    tool_results.insert(id.clone(), (result.clone(), *is_error));
                }
            }
            let is_active_assistant =
                ctx.last_assistant_idx == Some(idx) && msg.finish.is_none() && msg.error.is_none();
            let assistant_marker = assistant_marker_color(msg.agent.as_deref(), ctx.theme);
            let unresolved_tool_calls = msg
                .parts
                .iter()
                .filter_map(|part| match part {
                    MessagePart::ToolCall { id, .. } if !tool_results.contains_key(id) => {
                        Some(id.as_str())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let running_tool_call = if is_active_assistant {
                unresolved_tool_calls.first().copied()
            } else {
                None
            };

            if msg.parts.is_empty() {
                let mut text_lines = super::session_text::render_text_part(
                    &msg.content,
                    ctx.theme,
                    assistant_marker,
                );
                if ctx.semantic_hl {
                    text_lines = super::semantic_highlight::highlight_lines(text_lines, ctx.theme);
                }
                append_message_lines(
                    &mut lines,
                    &mut line_to_message,
                    &msg.id,
                    paint_block_lines(text_lines, message_bg, message_border, ctx.content_width),
                );
            } else {
                let mut prev_was_text = false;
                let mut prev_was_tool = false;
                let mut part_idx = 0;
                while part_idx < msg.parts.len() {
                    let part = &msg.parts[part_idx];
                    match part {
                        MessagePart::Text { text } => {
                            if prev_was_tool {
                                append_message_lines(
                                    &mut lines,
                                    &mut line_to_message,
                                    &msg.id,
                                    vec![Line::from("")],
                                );
                            }
                            let mut text_lines = super::session_text::render_text_part(
                                text,
                                ctx.theme,
                                assistant_marker,
                            );
                            if ctx.semantic_hl {
                                text_lines = super::semantic_highlight::highlight_lines(
                                    text_lines, ctx.theme,
                                );
                            }
                            append_message_lines(
                                &mut lines,
                                &mut line_to_message,
                                &msg.id,
                                paint_block_lines(
                                    text_lines,
                                    message_bg,
                                    message_border,
                                    ctx.content_width,
                                ),
                            );
                            prev_was_text = true;
                            prev_was_tool = false;
                        }
                        MessagePart::Reasoning { text } => {
                            if ctx.show_thinking {
                                if prev_was_text || prev_was_tool {
                                    append_message_lines(
                                        &mut lines,
                                        &mut line_to_message,
                                        &msg.id,
                                        vec![Line::from("")],
                                    );
                                }
                                let reasoning_id = format!("{}:{part_idx}", msg.id);
                                let collapsed = ctx.collapsed_reasoning.contains(&reasoning_id);
                                let start_line = lines.len();
                                let rendered = super::session_text::render_reasoning_part(
                                    text, ctx.theme, collapsed,
                                );
                                if !rendered.lines.is_empty() {
                                    let painted = paint_block_lines(
                                        rendered.lines,
                                        message_thinking_bg,
                                        message_thinking_border,
                                        ctx.content_width,
                                    );
                                    append_message_lines(
                                        &mut lines,
                                        &mut line_to_message,
                                        &msg.id,
                                        painted,
                                    );
                                    if rendered.collapsible {
                                        let end_line = lines.len().saturating_sub(1);
                                        out.thinking_hits.push(ThinkingToggleHit {
                                            line_index: start_line,
                                            reasoning_id: reasoning_id.clone(),
                                        });
                                        if end_line > start_line {
                                            out.thinking_hits.push(ThinkingToggleHit {
                                                line_index: end_line,
                                                reasoning_id,
                                            });
                                        }
                                    }
                                }
                                prev_was_text = false;
                                prev_was_tool = false;
                            }
                        }
                        MessagePart::ToolCall {
                            id,
                            name,
                            arguments,
                        } => {
                            if prev_was_text {
                                append_message_lines(
                                    &mut lines,
                                    &mut line_to_message,
                                    &msg.id,
                                    vec![Line::from("")],
                                );
                            }
                            if !ctx.show_tool_calls {
                                let run_start = part_idx;
                                let mut run_end = run_start;
                                while matches!(
                                    msg.parts.get(run_end),
                                    Some(MessagePart::ToolCall { .. })
                                ) {
                                    run_end += 1;
                                }
                                let run_id = format!("{}:tools:{run_start}", msg.id);
                                let expanded_run = ctx.expanded_tool_calls.contains(&run_id);
                                let summary = tool_run_summary(
                                    &msg.parts[run_start..run_end],
                                    &tool_results,
                                    running_tool_call,
                                );
                                let start_line = lines.len();
                                append_message_lines(
                                    &mut lines,
                                    &mut line_to_message,
                                    &msg.id,
                                    vec![super::session_tool::render_tool_run_summary(
                                        summary.count,
                                        summary.state,
                                        summary.denied,
                                        expanded_run,
                                        ctx.theme,
                                    )],
                                );
                                visible_tool_ids.insert(run_id.clone());
                                out.tool_hits.push(ToolToggleHit {
                                    line_index: start_line,
                                    tool_id: run_id,
                                });

                                if expanded_run {
                                    for run_part in &msg.parts[run_start..run_end] {
                                        if let MessagePart::ToolCall {
                                            id,
                                            name,
                                            arguments,
                                        } = run_part
                                        {
                                            let start_line = lines.len();
                                            let rendered = render_tool_call_part(
                                                id,
                                                name,
                                                arguments,
                                                &tool_results,
                                                running_tool_call,
                                                ctx.expanded_tool_calls.contains(id),
                                                ctx.show_tool_details,
                                                ctx.content_width,
                                                ctx.theme,
                                            );
                                            append_rendered_tool_call(
                                                rendered,
                                                id,
                                                &msg.id,
                                                start_line,
                                                message_bg,
                                                message_border,
                                                ctx.content_width,
                                                &mut visible_tool_ids,
                                                &mut out.tool_hits,
                                                &mut lines,
                                                &mut line_to_message,
                                            );
                                        }
                                    }
                                }

                                part_idx = run_end;
                            } else {
                                let start_line = lines.len();
                                let rendered = render_tool_call_part(
                                    id,
                                    name,
                                    arguments,
                                    &tool_results,
                                    running_tool_call,
                                    ctx.expanded_tool_calls.contains(id),
                                    ctx.show_tool_details,
                                    ctx.content_width,
                                    ctx.theme,
                                );
                                append_rendered_tool_call(
                                    rendered,
                                    id,
                                    &msg.id,
                                    start_line,
                                    message_bg,
                                    message_border,
                                    ctx.content_width,
                                    &mut visible_tool_ids,
                                    &mut out.tool_hits,
                                    &mut lines,
                                    &mut line_to_message,
                                );
                                part_idx += 1;
                            }
                            prev_was_text = false;
                            prev_was_tool = true;
                            continue;
                        }
                        MessagePart::ToolResult { .. } => {}
                        MessagePart::File { path, mime } => {
                            let file_line = Line::from(vec![
                                Span::styled("▸ ", Style::default().fg(assistant_marker)),
                                Span::styled("[file] ", Style::default().fg(ctx.theme.info)),
                                Span::styled(path.clone(), Style::default().fg(ctx.theme.text)),
                                Span::styled(
                                    format!(" ({})", mime),
                                    Style::default().fg(ctx.theme.text_muted),
                                ),
                            ]);
                            append_message_lines(
                                &mut lines,
                                &mut line_to_message,
                                &msg.id,
                                paint_block_lines(
                                    vec![file_line],
                                    message_bg,
                                    message_border,
                                    ctx.content_width,
                                ),
                            );
                        }
                        MessagePart::Image { url } => {
                            let image_line = Line::from(vec![
                                Span::styled("▸ ", Style::default().fg(assistant_marker)),
                                Span::styled("[image] ", Style::default().fg(ctx.theme.info)),
                                Span::styled(
                                    url.clone(),
                                    Style::default().fg(ctx.theme.text_muted),
                                ),
                            ]);
                            append_message_lines(
                                &mut lines,
                                &mut line_to_message,
                                &msg.id,
                                paint_block_lines(
                                    vec![image_line],
                                    message_bg,
                                    message_border,
                                    ctx.content_width,
                                ),
                            );
                        }
                    }
                    part_idx += 1;
                }
            }

            if let Some(footer) = assistant_footer(
                ctx.messages,
                idx,
                ctx.last_assistant_idx,
                msg,
                ctx.fallback_model,
                ctx.theme,
            ) {
                append_message_lines(
                    &mut lines,
                    &mut line_to_message,
                    &msg.id,
                    paint_block_lines(vec![footer], message_bg, message_border, ctx.content_width),
                );
            }

            let has_task_part = msg
                .parts
                .iter()
                .any(|part| matches!(part, MessagePart::ToolCall { name, .. } if name == "task"));
            if has_task_part {
                let hint = task_view_subagents_line(
                    ctx.theme,
                    ctx.keybind,
                    ctx.include_background_subagents,
                );
                append_message_lines(
                    &mut lines,
                    &mut line_to_message,
                    &msg.id,
                    paint_block_lines(
                        vec![Line::from(""), hint],
                        message_bg,
                        message_border,
                        ctx.content_width,
                    ),
                );
            }
        }
        MessageRole::System => {
            let system_lines: Vec<Line<'static>> = msg
                .content
                .lines()
                .map(|line_text| {
                    Line::from(Span::styled(
                        line_text.to_string(),
                        Style::default().fg(ctx.theme.text_muted),
                    ))
                })
                .collect();
            append_message_lines(&mut lines, &mut line_to_message, &msg.id, system_lines);
        }
    }

    out.lines = lines;
    out
}

fn push_spacing_lines(
    lines: &mut Vec<Line<'static>>,
    line_to_message: &mut Vec<Option<String>>,
    count: usize,
) {
    for _ in 0..count {
        lines.push(Line::from(""));
        line_to_message.push(None);
    }
}

fn append_message_lines(
    lines: &mut Vec<Line<'static>>,
    line_to_message: &mut Vec<Option<String>>,
    message_id: &str,
    new_lines: Vec<Line<'static>>,
) {
    if new_lines.is_empty() {
        return;
    }
    let marker = Some(message_id.to_string());
    for _ in 0..new_lines.len() {
        line_to_message.push(marker.clone());
    }
    lines.extend(new_lines);
}

fn append_non_message_lines(
    lines: &mut Vec<Line<'static>>,
    line_to_message: &mut Vec<Option<String>>,
    new_lines: Vec<Line<'static>>,
) {
    if new_lines.is_empty() {
        return;
    }
    for _ in 0..new_lines.len() {
        line_to_message.push(None);
    }
    lines.extend(new_lines);
}

struct ToolRunSummary {
    count: usize,
    state: super::session_tool::ToolState,
    denied: bool,
}

fn tool_run_summary(
    parts: &[MessagePart],
    tool_results: &HashMap<String, (String, bool)>,
    running_tool_call: Option<&str>,
) -> ToolRunSummary {
    let mut count = 0;
    let mut state = super::session_tool::ToolState::Completed;
    let mut denied = false;

    for part in parts {
        let MessagePart::ToolCall { id, .. } = part else {
            continue;
        };
        count += 1;
        let tool_state = tool_call_state(id, tool_results, running_tool_call);
        state = combine_tool_state(state, tool_state);
        if tool_results.get(id).is_some_and(|(result, is_error)| {
            *is_error && super::session_tool::is_denied_result(result)
        }) {
            denied = true;
        }
    }

    ToolRunSummary {
        count,
        state,
        denied,
    }
}

fn combine_tool_state(
    current: super::session_tool::ToolState,
    next: super::session_tool::ToolState,
) -> super::session_tool::ToolState {
    use super::session_tool::ToolState;
    match (current, next) {
        (ToolState::Failed, _) | (_, ToolState::Failed) => ToolState::Failed,
        (ToolState::Running, _) | (_, ToolState::Running) => ToolState::Running,
        (ToolState::Pending, _) | (_, ToolState::Pending) => ToolState::Pending,
        _ => ToolState::Completed,
    }
}

fn tool_call_state(
    id: &str,
    tool_results: &HashMap<String, (String, bool)>,
    running_tool_call: Option<&str>,
) -> super::session_tool::ToolState {
    if let Some((_, is_error)) = tool_results.get(id) {
        if *is_error {
            super::session_tool::ToolState::Failed
        } else {
            super::session_tool::ToolState::Completed
        }
    } else if running_tool_call == Some(id) {
        super::session_tool::ToolState::Running
    } else {
        super::session_tool::ToolState::Pending
    }
}

#[allow(clippy::too_many_arguments)]
fn render_tool_call_part(
    id: &str,
    name: &str,
    arguments: &str,
    tool_results: &HashMap<String, (String, bool)>,
    running_tool_call: Option<&str>,
    expanded: bool,
    show_tool_details: bool,
    width: usize,
    theme: &crate::theme::Theme,
) -> super::session_tool::ToolCallRender {
    super::session_tool::render_tool_call(
        id,
        name,
        arguments,
        tool_call_state(id, tool_results, running_tool_call),
        tool_results,
        show_tool_details,
        expanded,
        width,
        theme,
    )
}

fn append_rendered_tool_call(
    rendered: super::session_tool::ToolCallRender,
    id: &str,
    message_id: &str,
    start_line: usize,
    background: Color,
    border_color: Color,
    width: usize,
    visible_tool_ids: &mut HashSet<String>,
    tool_toggle_hits: &mut Vec<ToolToggleHit>,
    lines: &mut Vec<Line<'static>>,
    line_to_message: &mut Vec<Option<String>>,
) {
    if rendered.lines.is_empty() {
        return;
    }
    // FEAT-055: tool lines go through the same block pipeline as every other
    // message part so they get the gutter, background, padding, and wrapping.
    let painted = paint_block_lines(rendered.lines, background, border_color, width);
    if painted.is_empty() {
        return;
    }
    let end_line = start_line + painted.len() - 1;
    if rendered.collapsible {
        visible_tool_ids.insert(id.to_string());
        tool_toggle_hits.push(ToolToggleHit {
            line_index: start_line,
            tool_id: id.to_string(),
        });
        if end_line > start_line {
            tool_toggle_hits.push(ToolToggleHit {
                line_index: end_line,
                tool_id: id.to_string(),
            });
        }
    }
    append_message_lines(lines, line_to_message, message_id, painted);
}

fn paint_block_lines(
    lines: Vec<Line<'static>>,
    background: Color,
    border_color: Color,
    width: usize,
) -> Vec<Line<'static>> {
    let painted: Vec<Line<'static>> = lines
        .into_iter()
        .flat_map(|line| wrap_block_line(line, width))
        .map(|line| paint_block_line(line, background, border_color, width))
        .collect();

    if painted.is_empty() {
        return painted;
    }

    let gutter = painted
        .first()
        .and_then(|line| line.spans.first())
        .map(|span| span.content.to_string())
        .filter(|value| is_gutter_span(value.as_str()));

    let padding_line = if let Some(gutter) = gutter {
        paint_block_line(
            Line::from(vec![Span::raw(gutter)]),
            background,
            border_color,
            width,
        )
    } else {
        paint_block_line(Line::from(""), background, border_color, width)
    };

    let mut padded = Vec::with_capacity(painted.len() + 2);
    padded.push(padding_line.clone());
    padded.extend(painted);
    padded.push(padding_line);
    padded
}

fn paint_block_line(
    line: Line<'static>,
    background: Color,
    border_color: Color,
    width: usize,
) -> Line<'static> {
    let mut styled = Vec::with_capacity(line.spans.len() + 1);
    let mut rendered_width = 0usize;

    for (idx, span) in line.spans.into_iter().enumerate() {
        rendered_width += UnicodeWidthStr::width(span.content.as_ref());
        let style = if idx == 0 && is_gutter_span(span.content.as_ref()) {
            span.style.fg(border_color).bg(background)
        } else {
            span.style.bg(background)
        };
        styled.push(Span::styled(span.content, style));
    }

    if rendered_width < width {
        styled.push(Span::styled(
            " ".repeat(width - rendered_width),
            Style::default().bg(background),
        ));
    }

    Line::from(styled)
}

fn wrap_block_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width == 0 || line.spans.is_empty() {
        return vec![line];
    }

    let mut iter = line.spans.into_iter();
    let Some(gutter) = iter.next() else {
        return vec![Line::from("")];
    };
    if !is_gutter_span(gutter.content.as_ref()) {
        let mut all_spans = vec![gutter];
        all_spans.extend(iter);
        let wrapped = wrap_spans(all_spans, width);
        return wrapped.into_iter().map(Line::from).collect();
    }

    let body_spans: Vec<Span<'static>> = iter.collect();
    let gutter_width = UnicodeWidthStr::width(gutter.content.as_ref());
    if gutter_width >= width {
        return vec![Line::from(vec![gutter])];
    }

    let body_width = width
        .saturating_sub(gutter_width)
        .saturating_sub(MESSAGE_BLOCK_RIGHT_PADDING);
    if body_width == 0 {
        return vec![Line::from(vec![gutter])];
    }
    let wrapped_body = wrap_spans(body_spans, body_width);
    wrapped_body
        .into_iter()
        .map(|body| {
            let mut spans = Vec::with_capacity(body.len() + 1);
            spans.push(gutter.clone());
            spans.extend(body);
            Line::from(spans)
        })
        .collect()
}

fn is_gutter_span(content: &str) -> bool {
    let mut has_border = false;
    for ch in content.chars() {
        match ch {
            '│' | '┃' => has_border = true,
            ' ' => {}
            _ => return false,
        }
    }
    has_border
}

fn point_in_optional_rect(area: Option<Rect>, col: u16, row: u16) -> bool {
    let Some(area) = area else {
        return false;
    };
    let max_x = area.x.saturating_add(area.width);
    let max_y = area.y.saturating_add(area.height);
    col >= area.x && col < max_x && row >= area.y && row < max_y
}

fn tint_sidebar_overlay(background: Color, accent: Color) -> Color {
    match (background, accent) {
        (Color::Rgb(br, bg, bb), Color::Rgb(ar, ag, ab)) => {
            // 80% base + 20% accent: colored but still subtle.
            let blend = |b: u8, a: u8| -> u8 { ((u16::from(b) * 4 + u16::from(a)) / 5) as u8 };
            Color::Rgb(blend(br, ar), blend(bg, ag), blend(bb, ab))
        }
        _ => background,
    }
}

/// FEAT-055: word-aware wrapping with a hard-break fallback.
///
/// Breaks at whitespace so whole words move to the next line, preserving span
/// styles and explicit `\n`. A single token wider than `width` is hard-broken
/// so over-width paths/flags still make progress (parity with markdown
/// `Paragraph::wrap`).
fn wrap_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Vec<Span<'static>>> {
    if width == 0 {
        return vec![spans];
    }

    let mut tokens: Vec<(char, Style)> = Vec::new();
    for span in spans {
        for ch in span.content.chars() {
            tokens.push((ch, span.style));
        }
    }

    let mut lines: Vec<Vec<(char, Style)>> = Vec::new();
    let mut current: Vec<(char, Style)> = Vec::new();
    let mut current_width = 0usize;

    let mut index = 0;
    while index < tokens.len() {
        let (ch, _style) = tokens[index];
        if ch == '\n' {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            index += 1;
            continue;
        }

        if ch == ' ' {
            let mut end = index;
            let mut run_width = 0usize;
            while end < tokens.len() && tokens[end].0 == ' ' {
                run_width += UnicodeWidthChar::width(' ').unwrap_or(0);
                end += 1;
            }
            if current_width + run_width > width && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            } else {
                current.extend_from_slice(&tokens[index..end]);
                current_width += run_width;
            }
            index = end;
            continue;
        }

        let mut end = index;
        let mut run_width = 0usize;
        while end < tokens.len() && tokens[end].0 != ' ' && tokens[end].0 != '\n' {
            run_width += UnicodeWidthChar::width(tokens[end].0).unwrap_or(0);
            end += 1;
        }

        if run_width > width {
            // Hard-break a word that cannot fit on any line.
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            let mut cursor = index;
            while cursor < end {
                let char_width = UnicodeWidthChar::width(tokens[cursor].0).unwrap_or(0);
                if current_width + char_width > width && !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }
                current.push(tokens[cursor]);
                current_width += char_width;
                cursor += 1;
            }
        } else {
            if current_width + run_width > width && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            current.extend_from_slice(&tokens[index..end]);
            current_width += run_width;
        }
        index = end;
    }

    lines.push(current);

    lines
        .into_iter()
        .map(|line| {
            let mut merged: Vec<Span<'static>> = Vec::new();
            for (ch, style) in line {
                push_merged_span(&mut merged, ch, style);
            }
            merged
        })
        .collect()
}

fn push_merged_span(line: &mut Vec<Span<'static>>, ch: char, style: Style) {
    if let Some(last) = line.last_mut() {
        if last.style == style {
            last.content.to_mut().push(ch);
            return;
        }
    }

    line.push(Span::styled(ch.to_string(), style));
}

fn assistant_footer(
    messages: &[Message],
    idx: usize,
    last_assistant_idx: Option<usize>,
    message: &Message,
    fallback_model: Option<&str>,
    theme: &crate::theme::Theme,
) -> Option<Line<'static>> {
    if !matches!(message.role, MessageRole::Assistant) {
        return None;
    }

    let is_last_assistant = last_assistant_idx == Some(idx);
    let is_interrupted = is_assistant_interrupted(message);
    let is_final = is_assistant_final(message);

    if !is_last_assistant && !is_final && !is_interrupted {
        return None;
    }

    let mode = message.mode.as_deref().unwrap_or("default");
    let mut spans = vec![
        Span::styled(
            "▣ ",
            Style::default().fg(if is_interrupted {
                theme.text_muted
            } else {
                assistant_marker_color(message.agent.as_deref(), theme)
            }),
        ),
        Span::styled(titlecase(mode), Style::default().fg(theme.text)),
    ];

    if let Some(model) = message
        .model
        .as_deref()
        .or(fallback_model)
        .filter(|value| !value.trim().is_empty())
    {
        spans.push(Span::styled(" · ", Style::default().fg(theme.text_muted)));
        spans.push(Span::styled(
            model.to_string(),
            Style::default().fg(theme.text_muted),
        ));
    }

    if let Some(duration) = assistant_duration(messages, idx, message, is_final) {
        spans.push(Span::styled(" · ", Style::default().fg(theme.text_muted)));
        spans.push(Span::styled(
            duration,
            Style::default().fg(theme.text_muted),
        ));
    }

    if is_interrupted {
        spans.push(Span::styled(
            " · interrupted",
            Style::default().fg(theme.text_muted),
        ));
    }

    Some(Line::from(spans))
}

fn assistant_duration(
    messages: &[Message],
    idx: usize,
    message: &Message,
    is_final: bool,
) -> Option<String> {
    if !is_final {
        return None;
    }
    let user_start = messages[..idx]
        .iter()
        .rev()
        .find(|m| matches!(m.role, MessageRole::User))
        .map(|m| m.created_at.timestamp_millis())?;
    let end = message.completed_at?.timestamp_millis();
    if end <= user_start {
        return None;
    }

    let elapsed = (end - user_start) as u64;
    if elapsed < 1_000 {
        Some(format!("{elapsed}ms"))
    } else {
        Some(format!("{:.1}s", elapsed as f64 / 1_000.0))
    }
}

fn is_assistant_final(message: &Message) -> bool {
    matches!(
        message.finish.as_deref(),
        Some(reason) if reason != "tool-calls" && reason != "unknown"
    )
}

fn is_assistant_interrupted(message: &Message) -> bool {
    if message.finish.as_deref() == Some("abort") {
        return true;
    }
    message
        .error
        .as_deref()
        .map(|err| {
            let lower = err.to_ascii_lowercase();
            lower.contains("messageabortederror")
                || lower.contains("abortederror")
                || lower.contains("abort")
        })
        .unwrap_or(false)
}

fn assistant_marker_color(agent: Option<&str>, theme: &crate::theme::Theme) -> Color {
    let Some(agent) = agent else {
        return theme.primary;
    };
    let mut hasher = DefaultHasher::new();
    agent.hash(&mut hasher);
    theme.agent_color(hasher.finish() as usize)
}

fn user_border_color_for_agent(agent: Option<&str>, theme: &crate::theme::Theme) -> Color {
    let Some(agent) = agent else {
        return theme.primary;
    };
    let mut hasher = DefaultHasher::new();
    agent.hash(&mut hasher);
    theme.agent_color(hasher.finish() as usize)
}

fn titlecase(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Extract the agent label from a subagent child session title of the form
/// `"<description> (@<agent> subagent)"`, matching the reference
/// `SubagentFooter` regex `/@(\w+) subagent/`. Falls back to `"Subagent"`.
fn subagent_label(title: &str) -> String {
    if let Some(at) = title.rfind('@') {
        let rest = &title[at + 1..];
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '-')
            .collect();
        let after = rest[name.len()..].trim_start();
        if !name.is_empty() && after.starts_with("subagent") {
            return titlecase(&name);
        }
    }
    "Subagent".to_string()
}

/// The `ctrl+x down view subagents` hint rendered on assistant messages that
/// contain a `task` tool part, mirroring the reference `AssistantMessage`. When
/// experimental background subagents are enabled, the reference also surfaces
/// `ctrl+b background`.
fn task_view_subagents_line(
    theme: &crate::theme::Theme,
    keybind: &crate::context::KeybindRegistry,
    include_background: bool,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            keybind.leader_chord("session_child_first"),
            Style::default().fg(theme.text),
        ),
        Span::styled(" view subagents", Style::default().fg(theme.text_muted)),
    ];
    if include_background {
        spans.push(Span::styled(
            format!("  {}", keybind.print("session_background")),
            Style::default().fg(theme.text),
        ));
        spans.push(Span::styled(
            " background",
            Style::default().fg(theme.text_muted),
        ));
    }
    Line::from(spans)
}

fn format_number(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + (digits.len() / 3));
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_call(id: &str) -> MessagePart {
        MessagePart::ToolCall {
            id: id.to_string(),
            name: "read".to_string(),
            arguments: "{}".to_string(),
        }
    }

    #[test]
    fn subagent_label_reads_agent_name_from_title() {
        assert_eq!(
            subagent_label("Research the API (@explore subagent)"),
            "Explore"
        );
        assert_eq!(subagent_label("Do a thing (@general subagent)"), "General");
        assert_eq!(subagent_label("A normal session"), "Subagent");
        assert_eq!(subagent_label(""), "Subagent");
    }

    #[test]
    fn task_hint_uses_leader_chord_and_muted_suffix() {
        let theme = crate::theme::Theme::dark();
        let keybind = crate::context::KeybindRegistry::new();
        let line = task_view_subagents_line(&theme, &keybind, false);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(text, "ctrl+x down view subagents");
    }

    #[test]
    fn task_hint_includes_background_when_capability_enabled() {
        let theme = crate::theme::Theme::dark();
        let keybind = crate::context::KeybindRegistry::new();
        let line = task_view_subagents_line(&theme, &keybind, true);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(text, "ctrl+x down view subagents  ctrl+b background");
    }

    #[test]
    fn tool_run_summary_counts_calls_and_prefers_active_state() {
        let parts = vec![tool_call("one"), tool_call("two"), tool_call("three")];
        let mut results = HashMap::new();
        results.insert("one".to_string(), ("ok".to_string(), false));

        let summary = tool_run_summary(&parts, &results, Some("two"));

        assert_eq!(summary.count, 3);
        assert_eq!(
            summary.state,
            super::super::session_tool::ToolState::Running
        );
        assert!(!summary.denied);
    }

    #[test]
    fn tool_run_summary_surfaces_failures_and_denials() {
        let parts = vec![tool_call("one"), tool_call("two")];
        let mut results = HashMap::new();
        results.insert("one".to_string(), ("permission denied".to_string(), true));
        results.insert("two".to_string(), ("ok".to_string(), false));

        let summary = tool_run_summary(&parts, &results, None);

        assert_eq!(summary.count, 2);
        assert_eq!(summary.state, super::super::session_tool::ToolState::Failed);
        assert!(summary.denied);
    }

    #[test]
    fn tool_run_summary_renders_singular_and_plural_labels() {
        let theme = crate::theme::Theme::dark();
        let singular = super::super::session_tool::render_tool_run_summary(
            1,
            super::super::session_tool::ToolState::Completed,
            false,
            false,
            &theme,
        );
        let plural = super::super::session_tool::render_tool_run_summary(
            2,
            super::super::session_tool::ToolState::Completed,
            false,
            false,
            &theme,
        );

        let singular_text = singular
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let plural_text = plural
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(singular_text.contains("1 tool call"));
        assert!(plural_text.contains("2 tool calls"));
    }

    fn joined(lines: &[Vec<Span<'static>>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn wrap_spans_moves_whole_words_to_the_next_line() {
        let lines = wrap_spans(vec![Span::raw("hello world foo")], 11);
        assert_eq!(joined(&lines), vec!["hello world", "foo"]);
    }

    #[test]
    fn wrap_spans_hard_breaks_tokens_wider_than_the_line() {
        let lines = wrap_spans(vec![Span::raw("abcdefghij")], 4);
        assert_eq!(joined(&lines), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_spans_preserves_explicit_newlines() {
        let lines = wrap_spans(vec![Span::raw("one\ntwo")], 10);
        assert_eq!(joined(&lines), vec!["one", "two"]);
    }

    fn draw_session(prompt_hidden: bool) -> (Option<Rect>, String) {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        *context.prompt_hidden.write() = prompt_hidden;
        let mut prompt = Prompt::new(context.clone());
        prompt.set_input("pending draft".to_string());
        let mut session = SessionView::new(context.clone(), "test-session".to_string());

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        let mut prompt_area = None;
        terminal
            .draw(|frame| {
                prompt_area = session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        let draft = prompt.get_input().to_string();
        (prompt_area, draft)
    }

    #[test]
    fn hidden_prompt_reserves_zero_rows_with_a_non_empty_draft() {
        let (prompt_area, draft) = draw_session(true);
        assert!(
            prompt_area.is_none(),
            "hidden prompt must not reserve a render area"
        );
        assert_eq!(draft, "pending draft");
    }

    #[test]
    fn visible_prompt_reserves_rows_with_a_non_empty_draft() {
        let (prompt_area, draft) = draw_session(false);
        assert!(
            prompt_area.is_some(),
            "visible prompt must reserve a render area"
        );
        assert_eq!(draft, "pending draft");
    }

    fn message(id: &str, role: MessageRole, text: &str, completed: bool) -> Message {
        Message {
            id: id.to_string(),
            role,
            content: text.to_string(),
            created_at: chrono::Utc::now(),
            agent: None,
            model: None,
            mode: None,
            finish: None,
            error: None,
            completed_at: completed.then(chrono::Utc::now),
            cost: 0.0,
            tokens: Default::default(),
            parts: vec![MessagePart::Text {
                text: text.to_string(),
            }],
        }
    }

    fn row_containing(
        terminal: &ratatui::Terminal<ratatui::backend::TestBackend>,
        needle: &str,
    ) -> Vec<usize> {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .filter(|&y| {
                let mut row = String::new();
                for x in 0..buffer.area.width {
                    row.push_str(buffer.get(x, y).symbol());
                }
                row.contains(needle)
            })
            .map(usize::from)
            .collect()
    }

    #[test]
    fn continuation_prompt_renders_once_in_order_and_stays_put() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        let session_id = "test-session".to_string();
        // The interrupt-then-continue shape: a completed turn, the sent
        // continuation prompt, then the in-flight assistant reply.
        context.session.write().set_messages(
            &session_id,
            vec![
                message("m1", MessageRole::User, "start", true),
                message("m2", MessageRole::Assistant, "first answer", true),
                message("m3", MessageRole::User, "continuation prompt", true),
                message("m4", MessageRole::Assistant, "partial", false),
            ],
        );

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

        // Warm-up draws settle the follow-content layout (rendered line count).
        for _ in 0..2 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }

        let measure = |terminal: &ratatui::Terminal<TestBackend>| {
            let prompt_rows = row_containing(terminal, "continuation prompt");
            assert_eq!(
                prompt_rows.len(),
                1,
                "continuation prompt must render exactly once"
            );
            let answer_rows = row_containing(terminal, "first answer");
            assert!(
                answer_rows
                    .last()
                    .is_some_and(|answer| prompt_rows[0] > *answer),
                "continuation prompt must render after the answer"
            );
            prompt_rows[0]
        };

        let first_row = measure(&terminal);

        terminal
            .draw(|frame| {
                session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        let second_row = measure(&terminal);
        assert_eq!(
            first_row, second_row,
            "continuation prompt must not move across frames"
        );
    }

    #[test]
    fn queued_message_rows_do_not_jitter_while_streaming() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        let session_id = "queued-jitter-regression".to_string();
        let mut messages = vec![
            message("u1", MessageRole::User, "start prompt", true),
            message("a1", MessageRole::Assistant, "first answer", true),
            message("a2", MessageRole::Assistant, "partial", false),
            message("q1", MessageRole::User, "QUEUED-ONE", true),
            message("q2", MessageRole::User, "QUEUED-TWO", true),
            message("q3", MessageRole::User, "QUEUED-THREE", true),
        ];
        context
            .session
            .write()
            .set_messages(&session_id, messages.clone());

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        // A tall terminal keeps the transcript shorter than the pane (unclamped),
        // which is where the one-line pane-sizing lag used to shift queued rows.
        let mut terminal = Terminal::new(TestBackend::new(80, 60)).expect("terminal");
        for _ in 0..3 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }

        let rows = |terminal: &Terminal<TestBackend>| {
            (
                row_containing(terminal, "QUEUED-ONE"),
                row_containing(terminal, "QUEUED-TWO"),
                row_containing(terminal, "QUEUED-THREE"),
            )
        };

        let mut assistant_text = "partial".to_string();
        for step in 0..8 {
            assistant_text.push_str(&format!("\nline {step}"));
            messages[2].content = assistant_text.clone();
            messages[2].parts = vec![MessagePart::Text {
                text: assistant_text.clone(),
            }];
            context
                .session
                .write()
                .set_messages(&session_id, messages.clone());

            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
            let after_stream = rows(&terminal);

            // A redraw with no new content must not move the queued rows.
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
            let settled = rows(&terminal);

            assert_eq!(
                after_stream, settled,
                "queued rows must not move on a no-op redraw (step {step}): \
                 {after_stream:?} vs {settled:?}"
            );
            assert!(
                session.rendered_line_count <= session.messages_viewport_height,
                "test must stay in the unclamped regime (step {step})"
            );
        }
        assert_eq!(
            session.scroll_offset, 0,
            "an unclamped transcript must not scroll"
        );
    }

    fn long_session(session_id: &str, context: &Arc<AppContext>, count: usize) {
        let messages: Vec<Message> = (0..count)
            .map(|i| {
                let role = if i % 2 == 0 {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                };
                message(
                    &format!("m{i}"),
                    role,
                    &format!("message body number {i}"),
                    true,
                )
            })
            .collect();
        context.session.write().set_messages(session_id, messages);
    }

    #[test]
    fn per_frame_layout_is_bounded_by_the_window_not_history_size() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        use std::sync::atomic::Ordering;

        let context = Arc::new(AppContext::new());
        let session_id = "long-session".to_string();
        long_session(&session_id, &context, 2000);

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

        // Warm-up draws seed the per-message height cache (one full layout).
        for _ in 0..2 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }
        assert!(
            session.rendered_line_count > 2000,
            "scroll semantics must still use the whole-session line total"
        );

        LAYOUT_RENDERS.store(0, Ordering::Relaxed);
        terminal
            .draw(|frame| {
                session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        let laid_out = LAYOUT_RENDERS.load(Ordering::Relaxed);

        assert!(laid_out > 0, "the visible window must still be laid out");
        assert!(
            laid_out < 100,
            "per-frame layout must be bounded by the window, laid out {laid_out} of 2000 messages"
        );
    }

    #[test]
    fn timeline_jump_to_pre_cutoff_message_renders_it() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        let session_id = "jump-session".to_string();
        let mut messages = vec![message("m0", MessageRole::User, "OLDEST-MARKER", true)];
        for i in 1..200 {
            let role = if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            };
            messages.push(message(
                &format!("m{i}"),
                role,
                &format!("filler body number {i}"),
                true,
            ));
        }
        messages[50] = message("m50", MessageRole::Assistant, "MIDDLE-CUTOFF-MARKER", true);
        context.session.write().set_messages(&session_id, messages);

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

        for _ in 0..2 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }

        // Move to the newest message so older messages fall outside the window.
        session.scroll_to_message("m199");
        terminal
            .draw(|frame| {
                session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        assert!(
            row_containing(&terminal, "OLDEST-MARKER").is_empty(),
            "oldest message must be outside the render window before the jump"
        );
        assert!(
            row_containing(&terminal, "MIDDLE-CUTOFF-MARKER").is_empty(),
            "middle message must be outside the render window before the jump"
        );

        // A timeline jump to a mid-history message must re-anchor the window and
        // display that message, not merely move an abstract offset.
        session.scroll_to_message("m50");
        terminal
            .draw(|frame| {
                session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        assert!(
            !row_containing(&terminal, "MIDDLE-CUTOFF-MARKER").is_empty(),
            "timeline jump must render the pre-cutoff message"
        );

        // A jump to the very oldest message must also render it.
        session.scroll_to_message("m0");
        terminal
            .draw(|frame| {
                session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        assert!(
            !row_containing(&terminal, "OLDEST-MARKER").is_empty(),
            "timeline jump to the top must render the oldest message"
        );
    }

    #[test]
    fn follow_tail_keeps_newest_content_visible_under_windowing() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        let session_id = "follow-session".to_string();
        long_session(&session_id, &context, 80);

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");
        for _ in 0..3 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }

        // While pinned near the bottom, newly streamed content must stay visible.
        context.session.write().add_message(
            &session_id,
            message("m80", MessageRole::Assistant, "NEWEST-TAIL-MARKER", true),
        );
        for _ in 0..2 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }
        assert!(
            !row_containing(&terminal, "NEWEST-TAIL-MARKER").is_empty(),
            "follow must keep the newest content pinned at the bottom"
        );
    }

    #[test]
    fn scrolling_up_pages_in_older_content_under_windowing() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        let session_id = "scroll-session".to_string();
        let mut messages = vec![message("m0", MessageRole::User, "OLDEST-MARKER", true)];
        for i in 1..200 {
            let role = if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            };
            messages.push(message(
                &format!("m{i}"),
                role,
                &format!("filler body number {i}"),
                true,
            ));
        }
        context.session.write().set_messages(&session_id, messages);

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");
        for _ in 0..2 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }
        assert!(
            row_containing(&terminal, "OLDEST-MARKER").is_empty(),
            "oldest message starts outside the window at the bottom"
        );

        session.scroll_up_by(10_000);
        terminal
            .draw(|frame| {
                session.render(frame, frame.size(), &prompt);
            })
            .expect("draw");
        assert!(
            !row_containing(&terminal, "OLDEST-MARKER").is_empty(),
            "scrolling up must page older content into the window"
        );
    }

    #[test]
    fn windowed_toggle_click_hits_visible_block() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        *context.show_thinking.write() = true;
        let session_id = "click-session".to_string();
        let reasoning = "thinking line\n".repeat(40);
        let msg = Message {
            id: "m0".to_string(),
            role: MessageRole::Assistant,
            content: String::new(),
            created_at: chrono::Utc::now(),
            agent: None,
            model: None,
            mode: None,
            finish: Some("stop".to_string()),
            error: None,
            completed_at: Some(chrono::Utc::now()),
            cost: 0.0,
            tokens: Default::default(),
            parts: vec![
                MessagePart::Reasoning { text: reasoning },
                MessagePart::Text {
                    text: "answer".to_string(),
                },
            ],
        };
        context.session.write().set_messages(&session_id, vec![msg]);

        let prompt = Prompt::new(context.clone());
        let mut session = SessionView::new(context.clone(), session_id.clone());
        let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");
        for _ in 0..2 {
            terminal
                .draw(|frame| {
                    session.render(frame, frame.size(), &prompt);
                })
                .expect("draw");
        }

        let area = session.last_messages_area.expect("messages area");
        let top = session.scroll_offset;
        let bottom = top + usize::from(area.height);
        let hit = session
            .thinking_toggle_hits
            .iter()
            .find(|hit| hit.line_index >= top && hit.line_index < bottom)
            .expect("a visible reasoning toggle hit");
        let row = area.y + (hit.line_index - top) as u16;

        assert!(
            session.handle_click(area.x + 1, row),
            "click on a windowed toggle line must still toggle the block"
        );
    }
}
