use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::api::SkillSummary;
use crate::theme::Theme;

pub struct SkillListDialog {
    skills: Vec<SkillSummary>,
    filtered: Vec<usize>,
    query: String,
    state: ListState,
    open: bool,
}

impl SkillListDialog {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            skills: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            state,
            open: false,
        }
    }

    pub fn set_skills(&mut self, mut skills: Vec<SkillSummary>) {
        skills.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        });
        skills.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
        self.skills = skills;
        self.filter();
    }

    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.filter();
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

    pub fn selected_skill(&self) -> Option<&str> {
        let idx = self.state.selected().and_then(|s| self.filtered.get(s))?;
        self.skills.get(*idx).map(|skill| skill.name.as_str())
    }

    fn filter(&mut self) {
        let query = self.query.to_ascii_lowercase();
        self.filtered = self
            .skills
            .iter()
            .enumerate()
            .filter(|(_, skill)| {
                skill.name.to_ascii_lowercase().contains(&query)
                    || skill.description.to_ascii_lowercase().contains(&query)
            })
            .map(|(idx, _)| idx)
            .collect();
        self.state.select(if self.filtered.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let dialog_area = centered_rect(72, 18, area);
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Skills ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));
        let inner = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme.primary)),
                Span::styled(&self.query, Style::default().fg(theme.text)),
                Span::styled("▏", Style::default().fg(theme.primary)),
            ])),
            layout[0],
        );

        let items = if self.filtered.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "No skills available",
                Style::default().fg(theme.text_muted),
            )))]
        } else {
            self.filtered
                .iter()
                .filter_map(|idx| self.skills.get(*idx))
                .map(|skill| {
                    let mut lines = vec![Line::from(Span::styled(
                        format!("/{}", skill.name),
                        Style::default().fg(theme.text),
                    ))];
                    if !skill.description.trim().is_empty() {
                        lines.push(Line::from(Span::styled(
                            skill.description.clone(),
                            Style::default().fg(theme.text_muted),
                        )));
                    }
                    ListItem::new(lines)
                })
                .collect::<Vec<_>>()
        };

        frame.render_stateful_widget(
            List::new(items).highlight_style(
                Style::default()
                    .bg(theme.background_element)
                    .add_modifier(Modifier::BOLD),
            ),
            layout[1],
            &mut self.state.clone(),
        );

        frame.render_widget(
            Paragraph::new("Enter insert /skill  Esc close")
                .style(Style::default().fg(theme.text_muted)),
            layout[2],
        );
    }
}

impl Default for SkillListDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_name_and_description() {
        let mut dialog = SkillListDialog::new();
        dialog.set_skills(vec![
            SkillSummary {
                name: "reviewer".to_string(),
                description: "Review code changes".to_string(),
            },
            SkillSummary {
                name: "release".to_string(),
                description: "Prepare changelog".to_string(),
            },
        ]);

        dialog.open();
        for ch in "code".chars() {
            dialog.handle_input(ch);
        }

        assert_eq!(dialog.filtered.len(), 1);
        assert_eq!(dialog.selected_skill(), Some("reviewer"));
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}
