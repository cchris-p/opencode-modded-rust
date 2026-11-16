use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::theme::Theme;
use opencode_util::update::{short_rev, AvailableUpdate, Channel};

pub struct UpdateDialog {
    current_version: String,
    current_rev: Option<String>,
    channel: Channel,
    available: Option<AvailableUpdate>,
    checking: bool,
    installing: bool,
    open: bool,
    status: Option<String>,
}

impl UpdateDialog {
    pub fn new() -> Self {
        Self {
            current_version: String::new(),
            current_rev: None,
            channel: Channel::Stable,
            available: None,
            checking: false,
            installing: false,
            open: false,
            status: None,
        }
    }

    pub fn open(
        &mut self,
        current_version: &str,
        current_rev: Option<&str>,
        channel: Channel,
        available: Option<AvailableUpdate>,
        checking: bool,
    ) {
        self.current_version = current_version.to_string();
        self.current_rev = current_rev.map(str::to_string);
        self.channel = channel;
        self.available = available;
        self.checking = checking;
        self.installing = false;
        self.status = None;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_available(&mut self, available: Option<AvailableUpdate>) {
        self.available = available;
        self.checking = false;
    }

    pub fn set_checking(&mut self, checking: bool) {
        self.checking = checking;
        if checking {
            self.available = None;
            self.status = None;
        }
    }

    pub fn set_installing(&mut self, installing: bool) {
        self.installing = installing;
        if installing {
            self.status = Some("Installing...".to_string());
        }
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.installing = false;
        self.status = status;
    }

    pub fn available(&self) -> Option<&AvailableUpdate> {
        self.available.as_ref()
    }

    pub fn can_install(&self) -> bool {
        self.available.is_some() && !self.installing
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let width = 68.min(area.width.saturating_sub(4)).max(20);
        let height = 11.min(area.height.saturating_sub(2)).max(8);
        let dialog_area = super::centered_rect(width, height, area);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Update ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.warning))
            .style(Style::default().bg(theme.background_panel));
        let inner = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);

        let channel = self.channel.as_str();
        let current = match self.current_rev.as_deref() {
            Some(rev) => format!(
                "Current:   {} ({}) - {}",
                self.current_version,
                short_rev(rev),
                channel
            ),
            None => format!(
                "Current:   {} - {} (not managed by ort-install)",
                self.current_version, channel
            ),
        };
        let available = match self.available.as_ref() {
            Some(update) => format!(
                "Available: {} ({}) - {}",
                update.version,
                short_rev(&update.rev),
                update.channel.as_str()
            ),
            None if self.checking => "Available: checking...".to_string(),
            None => "Available: none (up to date)".to_string(),
        };

        let mut lines = vec![
            Line::from(Span::styled(current, Style::default().fg(theme.text))),
            Line::from(Span::styled(
                available,
                Style::default().fg(if self.available.is_some() {
                    theme.warning
                } else {
                    theme.text_muted
                }),
            )),
        ];
        if let Some(status) = self.status.as_deref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                status,
                Style::default().fg(if self.installing {
                    theme.info
                } else {
                    theme.text_muted
                }),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .style(Style::default().fg(theme.text))
            .wrap(Wrap { trim: true });
        frame.render_widget(
            paragraph,
            Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: inner.height.saturating_sub(4),
            },
        );

        let mut hints = Vec::new();
        if self.can_install() {
            hints.push(("[Enter] ", "Install"));
        }
        if self.checking {
            hints.push(("", "Checking..."));
        } else {
            hints.push(("[r] ", "Check again"));
        }
        if self.available.is_some() {
            hints.push(("[d] ", "Dismiss"));
        }
        hints.push(("[Esc] ", "Close"));

        let mut spans = Vec::new();
        for (key, label) in hints {
            if !spans.is_empty() {
                spans.push(Span::raw("  "));
            }
            if !key.is_empty() {
                spans.push(Span::styled(key, Style::default().fg(theme.text_muted)));
            }
            spans.push(Span::styled(label, Style::default().fg(theme.text)));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).centered(),
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(2),
                width: inner.width,
                height: 1,
            },
        );
    }
}

impl Default for UpdateDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(rev: &str) -> AvailableUpdate {
        AvailableUpdate {
            version: "v0.1.5".to_string(),
            tag: "v0.1.5".to_string(),
            rev: rev.to_string(),
            channel: Channel::Stable,
            dismissed: false,
        }
    }

    #[test]
    fn install_requires_an_available_update() {
        let mut dialog = UpdateDialog::new();
        dialog.open("0.1.4", Some("cc9befa1"), Channel::Stable, None, false);
        assert!(!dialog.can_install());

        dialog.set_available(Some(update("bbbb2222")));
        assert!(dialog.can_install());

        dialog.set_installing(true);
        assert!(!dialog.can_install());
    }
}
