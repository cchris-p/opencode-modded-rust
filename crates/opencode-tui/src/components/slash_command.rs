use std::collections::HashSet;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::api::SkillSummary;
use crate::command::{fuzzy_match, CommandAction, CommandRegistry};
use crate::theme::Theme;

pub struct SlashCommandPopup {
    pub registry: CommandRegistry,
    pub skills: Vec<SkillSummary>,
    pub query: String,
    pub filtered: Vec<String>,
    pub state: ListState,
    pub open: bool,
    pub selected_action: Option<CommandAction>,
}

impl SlashCommandPopup {
    pub fn new() -> Self {
        Self {
            registry: CommandRegistry::new(),
            skills: Vec::new(),
            query: String::new(),
            filtered: Vec::new(),
            state: ListState::default(),
            open: false,
            selected_action: None,
        }
    }

    pub fn open(&mut self) {
        self.query = String::new();
        self.refresh_filter();
        self.state.select(Some(0));
        self.open = true;
        self.selected_action = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.filtered.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn take_action(&mut self) -> Option<CommandAction> {
        self.selected_action.take()
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_skills(&mut self, skills: Vec<SkillSummary>) {
        self.skills = skills;
        if self.open {
            self.refresh_filter();
        }
    }

    fn is_skill(&self, name: &str) -> bool {
        self.registry.get(name).is_none() && self.skills.iter().any(|skill| skill.name == name)
    }

    fn registry_has_command(&self, skill_name: &str) -> bool {
        self.registry.get(skill_name).is_some()
            || self.registry.get(&format!("/{}", skill_name)).is_some()
    }

    fn refresh_filter(&mut self) {
        self.filtered.clear();
        if self.query.is_empty() {
            self.filtered = self
                .registry
                .suggested_commands()
                .iter()
                .map(|cmd| cmd.name.clone())
                .collect();
        } else {
            let mut seen: HashSet<String> = HashSet::new();
            for cmd in self.registry.search(&self.query) {
                if seen.insert(cmd.name.clone()) {
                    self.filtered.push(cmd.name.clone());
                }
            }

            let mut skill_matches: Vec<(String, i32)> = self
                .skills
                .iter()
                .filter(|skill| !self.registry_has_command(&skill.name))
                .filter_map(|skill| {
                    fuzzy_match(&self.query, &skill.name).map(|score| (skill.name.clone(), score))
                })
                .filter(|(name, _)| !seen.contains(name))
                .collect();
            skill_matches.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            for (name, _) in skill_matches {
                if seen.insert(name.clone()) {
                    self.filtered.push(name);
                }
            }
        }
        self.state.select(Some(0));
    }

    pub fn handle_input(&mut self, c: char) {
        self.query.push(c);
        self.refresh_filter();
    }

    pub fn handle_backspace(&mut self) -> bool {
        if self.query.pop().is_some() {
            self.refresh_filter();
            true
        } else {
            false
        }
    }

    pub fn move_up(&mut self) {
        if let Some(selected) = self.state.selected() {
            let new = selected.saturating_sub(1);
            self.state.select(Some(new));
        }
    }

    pub fn move_down(&mut self) {
        if let Some(selected) = self.state.selected() {
            let new = (selected + 1).min(self.filtered.len().saturating_sub(1));
            self.state.select(Some(new));
        }
    }

    pub fn select_current(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(name) = self.filtered.get(idx).cloned() {
                if let Some(cmd) = self.registry.get(&name) {
                    self.selected_action = Some(cmd.action.clone());
                    self.close();
                } else if self.is_skill(&name) {
                    self.selected_action = Some(CommandAction::InsertSkill(name));
                    self.close();
                }
            }
        }
    }

    /// Render the menu directly above `area`, which is expected to be the prompt/input
    /// rect it was opened from. Anchoring to the prompt keeps the menu where the user is
    /// looking; anchoring to the window instead pushes it to the top of the screen and
    /// makes a typed query look like lost input.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open || self.filtered.is_empty() {
            return;
        }

        let screen = frame.size();
        let width = 50.min(area.width.saturating_sub(4)).min(screen.width);
        if width < 4 || screen.height < 3 {
            return;
        }
        let height = (10.min(self.filtered.len()) as u16).saturating_add(2);

        let x =
            (area.x + area.width.saturating_sub(width) / 2).min(screen.width.saturating_sub(width));
        // Prefer the menu directly above the prompt; if there is no room above, place it
        // below the prompt so it never lands on top of the input it belongs to.
        let y = if area.y > height {
            area.y.saturating_sub(height + 1)
        } else {
            (area.y + area.height + 1).min(screen.height.saturating_sub(height))
        };

        let popup_area = Rect::new(x, y.max(1), width, height);

        let query_line = Line::from(vec![
            Span::raw("/"),
            Span::styled(&self.query, Style::default().fg(theme.primary)),
        ]);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let cmd = self.registry.get(name);
                let is_skill = cmd.is_none() && self.is_skill(name);
                let title = cmd.map(|c| c.title.as_str()).unwrap_or(name.as_str());
                let keybind = cmd.and_then(|c| c.keybind.clone());

                let is_selected = self.state.selected() == Some(idx);
                let style = if is_selected {
                    Style::default()
                        .fg(theme.primary)
                        .bg(theme.background_element)
                } else {
                    Style::default().fg(theme.text)
                };
                let muted = Style::default().fg(theme.text_muted);

                let mut spans = vec![Span::styled(title, style)];
                if let Some(kb) = keybind {
                    spans.push(Span::styled(format!("  ({})", kb), muted));
                }
                if is_skill {
                    spans.push(Span::styled("  \u{b7} skill", muted));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(query_line)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            )
            .highlight_style(Style::default().fg(theme.primary));

        frame.render_widget(list, popup_area);
    }
}

impl Default for SlashCommandPopup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SlashCommandPopup;
    use crate::api::SkillSummary;
    use crate::command::CommandAction;
    use crate::theme::Theme;

    fn skill(name: &str) -> SkillSummary {
        SkillSummary {
            name: name.to_string(),
            description: None,
        }
    }

    fn popup_with_skills() -> SlashCommandPopup {
        let mut popup = SlashCommandPopup::new();
        popup.set_skills(vec![skill("session-summary"), skill("review-pr")]);
        popup
    }

    #[test]
    fn empty_query_lists_no_skills() {
        let mut popup = popup_with_skills();
        popup.open();
        assert!(!popup.filtered.is_empty());
        assert!(popup.filtered.iter().all(|name| !popup.is_skill(name)));
    }

    #[test]
    fn filtering_lists_matching_skills() {
        let mut popup = popup_with_skills();
        popup.open();
        for c in "summary".chars() {
            popup.handle_input(c);
        }
        assert!(popup.filtered.iter().any(|name| name == "session-summary"));
    }

    #[test]
    fn selecting_skill_yields_insert_action() {
        let mut popup = popup_with_skills();
        popup.open();
        for c in "review".chars() {
            popup.handle_input(c);
        }
        let idx = popup
            .filtered
            .iter()
            .position(|name| name == "review-pr")
            .expect("skill should be listed");
        popup.state.select(Some(idx));
        popup.select_current();
        assert!(matches!(
            popup.take_action(),
            Some(CommandAction::InsertSkill(name)) if name == "review-pr"
        ));
        assert!(!popup.is_open());
    }

    #[test]
    fn bare_slash_lists_suggested_commands() {
        let mut popup = SlashCommandPopup::new();
        popup.open();
        assert!(
            !popup.filtered.is_empty(),
            "bare `/` must offer suggested commands, not an empty menu"
        );
        let expected = popup.registry.suggested_commands();
        assert_eq!(popup.filtered.len(), expected.len());
        assert!(popup.filtered.iter().all(|name| name.starts_with('/')));
    }

    #[test]
    fn reopening_resets_query_and_selection() {
        let mut popup = popup_with_skills();
        popup.open();
        popup.handle_input('z');
        popup.move_down();
        popup.close();

        popup.open();
        assert!(popup.is_open());
        assert_eq!(popup.query(), "");
        assert_eq!(popup.state.selected(), Some(0));
        assert!(popup.take_action().is_none());
    }

    #[test]
    fn popup_renders_above_the_prompt_area() {
        use ratatui::backend::TestBackend;
        use ratatui::layout::Rect;
        use ratatui::Terminal;

        let mut popup = SlashCommandPopup::new();
        popup.open();

        let mut terminal = Terminal::new(TestBackend::new(60, 24)).expect("terminal");
        let prompt_area = Rect::new(5, 18, 50, 3);
        terminal
            .draw(|frame| popup.render(frame, prompt_area, &Theme::default()))
            .expect("draw");

        let buffer = terminal.backend().buffer();
        let bottom_border_row =
            (0..24).find(|&y| (0..60).any(|x| buffer.get(x, y).symbol() == "└"));

        assert_eq!(
            bottom_border_row,
            Some(prompt_area.y - 2),
            "the menu must sit directly above the prompt, not at the top of the screen"
        );
        assert!(
            (0..60).all(|x| buffer.get(x, 0).symbol() == " "),
            "nothing should render on the first row when the prompt is near the bottom"
        );
    }
}
