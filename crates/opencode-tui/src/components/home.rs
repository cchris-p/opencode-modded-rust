use std::sync::Arc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::branding::{APP_SHORT_NAME, APP_VERSION_DATE};
use crate::components::Prompt;
use crate::context::{AppContext, McpConnectionStatus};
use crate::ui::workspace_location_label;

const HOME_TIPS: &[&str] = &[
    "Press {highlight}Tab{/highlight} to cycle agents",
    "Press {highlight}Shift+Tab{/highlight} to cycle agents backward",
    "Press {highlight}Ctrl+P{/highlight} to open command palette",
    "Type {highlight}/help{/highlight} to browse all commands",
    "Use {highlight}/themes{/highlight} to switch visual themes",
    "Use {highlight}/sessions{/highlight} to resume older work",
    "Use {highlight}/timeline{/highlight} to jump to any message",
    "Use {highlight}/status{/highlight} to inspect runtime status",
    "Use {highlight}/mcps{/highlight} to inspect MCP connections",
    "Use {highlight}/editor{/highlight} to write prompts externally",
    "Use {highlight}/compact{/highlight} when context gets too long",
    "Use {highlight}/new{/highlight} to start a clean session",
    "Use {highlight}/copy{/highlight} to copy current session summary",
    "Use {highlight}/fork{/highlight} to branch from a message",
    "Use {highlight}/rename{/highlight} to rename this session",
    "Use {highlight}/share{/highlight} to generate share links",
    "Use {highlight}/unshare{/highlight} to revoke share links",
    "Use {highlight}/timestamps{/highlight} to show or hide times",
    "Use {highlight}/thinking{/highlight} to toggle reasoning blocks",
    "Use {highlight}/density{/highlight} for cozy or compact layout",
    "Use {highlight}/highlight{/highlight} to toggle semantic styling",
    "Use {highlight}/sidebar{/highlight} to toggle sidebar visibility",
    "Use {highlight}/header{/highlight} to toggle session header",
    "Use {highlight}/scrollbar{/highlight} to toggle scrollbar",
    "Use {highlight}/tips.toggle{/highlight} to hide or show tips",
    "Use {highlight}/stash{/highlight} to save current draft",
    "Use {highlight}/export{/highlight} to export chat transcript",
    "Use {highlight}Esc{/highlight} twice to interrupt running tasks",
    "Use {highlight}Alt+Up{/highlight} and {highlight}Alt+Down{/highlight} for prompt history",
    "Use {highlight}Ctrl+V{/highlight} to paste into the prompt",
    "Use {highlight}Ctrl+Shift+C{/highlight} to copy selected text",
    "Use {highlight}@path{/highlight} to reference files in prompt",
    "Use {highlight}/connect{/highlight} to add a new provider",
    "Use {highlight}/settings{/highlight} to reconfigure the active provider",
    "Use {highlight}/models{/highlight} to switch active model",
    "Use {highlight}/agents{/highlight} to switch active agent",
    "Use {highlight}/skills{/highlight} to inspect installed skills",
];
const TIP_ROTATE_SECONDS: i64 = 12;
const HOME_MAX_CONTENT_WIDTH: u16 = 75;
const HOME_OUTER_H_PADDING: u16 = 2;
const HOME_OUTER_V_PADDING: u16 = 1;

pub struct HomeView {
    context: Arc<AppContext>,
}

impl HomeView {
    pub fn new(context: Arc<AppContext>) -> Self {
        Self { context }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) -> Option<Rect> {
        let prompt = Prompt::new(self.context.clone())
            .with_placeholder("Ask anything... \"Fix a TODO in the codebase\"");
        self.render_with_prompt(frame, area, &prompt)
    }

    pub fn render_with_prompt(
        &self,
        frame: &mut Frame,
        area: Rect,
        prompt: &Prompt,
    ) -> Option<Rect> {
        let area = Rect {
            x: area.x.saturating_add(HOME_OUTER_H_PADDING),
            y: area.y.saturating_add(HOME_OUTER_V_PADDING),
            width: area
                .width
                .saturating_sub(HOME_OUTER_H_PADDING.saturating_mul(2)),
            height: area
                .height
                .saturating_sub(HOME_OUTER_V_PADDING.saturating_mul(2)),
        };
        if area.width == 0 || area.height == 0 {
            return None;
        }
        let prompt_hidden = *self.context.prompt_hidden.read();
        let prompt_height = if prompt_hidden { 0 } else { 8 };
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(prompt_height),
                Constraint::Length(4),
                Constraint::Length(3),
            ])
            .split(area);

        let prompt_width = layout[1].width.min(HOME_MAX_CONTENT_WIDTH);
        let left_pad = (layout[1].width.saturating_sub(prompt_width)) / 2;
        let prompt_area = Rect {
            x: layout[1].x + left_pad,
            y: layout[1].y,
            width: prompt_width,
            height: layout[1].height,
        };

        if !prompt_hidden && prompt_area.height > 0 && prompt_area.width > 0 {
            prompt.render(frame, prompt_area);
        }

        if self.should_show_tips() {
            self.render_tips(frame, layout[2]);
        }

        self.render_footer(frame, layout[3]);

        if prompt_hidden {
            None
        } else {
            Some(prompt_area)
        }
    }

    fn render_tips(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = self.context.theme.read();
        let tip_width = area.width.min(HOME_MAX_CONTENT_WIDTH);
        let left_pad = (area.width.saturating_sub(tip_width)) / 2;
        let tip_area = Rect {
            x: area.x + left_pad,
            y: area.y,
            width: tip_width,
            height: area.height,
        };
        if tip_area.height == 0 || tip_area.width == 0 {
            return;
        }

        let top_padding = 3u16.min(tip_area.height.saturating_sub(1));
        let tip_render_area = Rect {
            x: tip_area.x,
            y: tip_area.y.saturating_add(top_padding),
            width: tip_area.width,
            height: tip_area.height.saturating_sub(top_padding).max(1),
        };

        let slot = chrono::Utc::now()
            .timestamp()
            .div_euclid(TIP_ROTATE_SECONDS);
        let tip_idx = slot.rem_euclid(HOME_TIPS.len() as i64) as usize;
        let tip = HOME_TIPS
            .get(tip_idx)
            .copied()
            .unwrap_or("Use /help to open command guide");
        let mut spans = vec![Span::styled("● Tip ", Style::default().fg(theme.warning))];
        spans.extend(parse_tip_highlights(tip, &theme));
        let paragraph = Paragraph::new(Line::from(spans));

        frame.render_widget(paragraph, tip_render_area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let theme = self.context.theme.read();
        let directory = self.context.directory.read();
        let git_branch = self.context.git_branch.read();
        let mcp_servers = self.context.mcp_servers.read();

        let horizontal_padding = 2u16.min(area.width / 2);
        let vertical_padding = if area.height >= 3 { 1 } else { 0 };
        let content_area = Rect {
            x: area.x.saturating_add(horizontal_padding),
            y: area.y.saturating_add(vertical_padding),
            width: area
                .width
                .saturating_sub(horizontal_padding.saturating_mul(2)),
            height: area
                .height
                .saturating_sub(vertical_padding.saturating_mul(2)),
        };
        if content_area.width == 0 || content_area.height == 0 {
            return;
        }

        let connected_count = mcp_servers
            .iter()
            .filter(|s| matches!(s.status, McpConnectionStatus::Connected))
            .count();

        let mut spans: Vec<Span> = Vec::new();

        // Left: workspace path and checked-out branch
        let dir_text = workspace_location_label(&directory, git_branch.as_deref());
        spans.push(Span::styled(
            dir_text.clone(),
            Style::default().fg(theme.text_muted),
        ));

        // Middle: MCP status (only when servers exist)
        let mcp_text = if !mcp_servers.is_empty() {
            let has_errors = mcp_servers.iter().any(|s| {
                matches!(
                    s.status,
                    McpConnectionStatus::Failed | McpConnectionStatus::NeedsClientRegistration
                )
            });
            let dot_color = if has_errors {
                theme.error
            } else if connected_count > 0 {
                theme.success
            } else {
                theme.text_muted
            };
            let label = format!("{} MCP", connected_count);
            Some((dot_color, label))
        } else {
            None
        };

        // Right: branding + date version
        let version_text = format!("{} {}", APP_SHORT_NAME, APP_VERSION_DATE);

        // Calculate padding
        let left_len = dir_text.len();
        let mid_len = mcp_text.as_ref().map(|(_, l)| l.len() + 4).unwrap_or(0);
        let right_len = version_text.len();
        let total_content = left_len + mid_len + right_len;
        let available = content_area.width as usize;

        if let Some((dot_color, ref label)) = mcp_text {
            let left_padding = if available > total_content {
                (available - total_content) / 2
            } else if available > right_len + left_len + 1 {
                1
            } else {
                0
            };
            spans.push(Span::raw(" ".repeat(left_padding)));
            spans.push(Span::styled(
                "⊙ ".to_string(),
                Style::default().fg(dot_color),
            ));
            spans.push(Span::styled(
                label.clone(),
                Style::default().fg(theme.text_muted),
            ));
        }

        // Right-align version
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        let right_padding = available.saturating_sub(used + right_len);
        spans.push(Span::raw(" ".repeat(right_padding)));
        spans.push(Span::styled(
            version_text,
            Style::default().fg(theme.text_muted),
        ));

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, content_area);
    }

    fn should_show_tips(&self) -> bool {
        tips_visible(*self.context.tips_hidden.read())
    }
}

fn tips_visible(tips_hidden: bool) -> bool {
    !tips_hidden
}

fn parse_tip_highlights(tip: &str, theme: &crate::theme::Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = tip;
    const START: &str = "{highlight}";
    const END: &str = "{/highlight}";

    loop {
        let Some(start_idx) = remaining.find(START) else {
            if !remaining.is_empty() {
                spans.push(Span::styled(
                    remaining.to_string(),
                    Style::default().fg(theme.text_muted),
                ));
            }
            break;
        };

        let (plain, after_start_marker) = remaining.split_at(start_idx);
        if !plain.is_empty() {
            spans.push(Span::styled(
                plain.to_string(),
                Style::default().fg(theme.text_muted),
            ));
        }

        let highlighted_tail = &after_start_marker[START.len()..];
        let Some(end_idx) = highlighted_tail.find(END) else {
            spans.push(Span::styled(
                START.to_string(),
                Style::default().fg(theme.text_muted),
            ));
            if !highlighted_tail.is_empty() {
                spans.push(Span::styled(
                    highlighted_tail.to_string(),
                    Style::default().fg(theme.text_muted),
                ));
            }
            break;
        };

        let (highlighted, after_end_marker) = highlighted_tail.split_at(end_idx);
        if !highlighted.is_empty() {
            spans.push(Span::styled(
                highlighted.to_string(),
                Style::default().fg(theme.text),
            ));
        }
        remaining = &after_end_marker[END.len()..];
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::{tips_visible, HomeView};
    use crate::components::Prompt;
    use crate::context::AppContext;
    use ratatui::layout::Rect;
    use std::sync::Arc;

    #[test]
    fn tips_are_only_visible_when_not_hidden() {
        assert!(!tips_visible(true));
        assert!(tips_visible(false));
    }

    fn draw_home(prompt_hidden: bool) -> (Option<Rect>, String) {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let context = Arc::new(AppContext::new());
        *context.prompt_hidden.write() = prompt_hidden;
        let mut prompt = Prompt::new(context.clone());
        prompt.set_input("pending draft".to_string());
        let home = HomeView::new(context.clone());

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        let mut prompt_area = None;
        terminal
            .draw(|frame| {
                prompt_area = home.render_with_prompt(frame, frame.size(), &prompt);
            })
            .expect("draw");
        let draft = prompt.get_input().to_string();
        (prompt_area, draft)
    }

    #[test]
    fn hidden_prompt_reserves_zero_rows_on_home() {
        let (prompt_area, draft) = draw_home(true);
        assert!(
            prompt_area.is_none(),
            "hidden home prompt must not reserve a render area"
        );
        assert_eq!(draft, "pending draft");
    }

    #[test]
    fn visible_prompt_reserves_rows_on_home() {
        let (prompt_area, draft) = draw_home(false);
        assert!(
            prompt_area.is_some(),
            "visible home prompt must reserve a render area"
        );
        assert_eq!(draft, "pending draft");
    }
}
