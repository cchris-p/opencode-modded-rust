use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::context::{AppContext, SessionStatus};
use crate::file_index::FileIndex;
use crate::theme::Theme;

use super::spinner::{KnightRiderSpinner, SpinnerMode, TaskKind};

const MAX_HISTORY_ENTRIES: usize = 200;
const MAX_STASH_ENTRIES: usize = 50;
const MAX_FRECENCY_ENTRIES: usize = 1000;
const PROMPT_MIN_INPUT_LINES: u16 = 1;
const PROMPT_MAX_INPUT_LINES: u16 = 6;
const SHELL_PLACEHOLDER: &str = "Run a command... \"ls -la\"";
const INTERRUPT_CONFIRM_WINDOW_SECS: u64 = 5;
const FILE_INDEX_MAX_DEPTH: usize = 8;
const FILE_SUGGESTION_LIMIT: usize = 20;
const PROMPT_BLOCK_PAD_LEFT: u16 = 1;
const PROMPT_BLOCK_PAD_RIGHT: u16 = 1;
const PROMPT_BLOCK_PAD_TOP: u16 = 1;
const PROMPT_BLOCK_PAD_BOTTOM: u16 = 1;
const PROMPT_LINE_H_INSET: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    Normal,
    Shell,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptStashEntry {
    pub input: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct FrecencyEntry {
    frequency: u64,
    last_used: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct HistoryStore {
    entries: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct FrecencyStore {
    entries: HashMap<String, FrecencyEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct StashStore {
    entries: Vec<PromptStashEntry>,
}

pub struct Prompt {
    context: Arc<AppContext>,
    input: String,
    cursor_position: usize,
    focused: bool,
    placeholder: String,
    history: Vec<String>,
    history_index: Option<usize>,
    history_draft: Option<String>,
    frecency: HashMap<String, FrecencyEntry>,
    stash: Vec<PromptStashEntry>,
    suggestions: Vec<String>,
    suggestion_index: Option<usize>,
    known_commands: Vec<String>,
    known_agents: Vec<String>,
    known_skills: Vec<String>,
    history_path: PathBuf,
    frecency_path: PathBuf,
    stash_path: PathBuf,
    file_index: FileIndex,
    spinner: KnightRiderSpinner,
    mode: PromptMode,
    interrupt_press_count: u8,
    last_interrupt_time: Option<Instant>,
}

impl Prompt {
    pub fn new(context: Arc<AppContext>) -> Self {
        let state_dir = prompt_state_dir();
        let history_path = state_dir.join("prompt-history.json");
        let frecency_path = state_dir.join("prompt-frecency.json");
        let stash_path = state_dir.join("prompt-stash.json");

        let history = load_history(&history_path);
        let frecency = load_frecency(&frecency_path);
        let stash = load_stash(&stash_path);

        let spinner_color = {
            let theme = context.theme.read();
            let agent = context.current_agent.read();
            prompt_agent_color(&theme, agent.as_str())
        };

        let mut spinner = KnightRiderSpinner::with_color(spinner_color);
        spinner.set_mode(spinner_mode_from_env());

        let mut prompt = Self {
            context,
            input: String::new(),
            cursor_position: 0,
            focused: true,
            placeholder: "Ask anything...".to_string(),
            history,
            history_index: None,
            history_draft: None,
            frecency,
            stash,
            suggestions: Vec::new(),
            suggestion_index: None,
            known_commands: vec![
                "/help".to_string(),
                "/model".to_string(),
                "/agent".to_string(),
                "/status".to_string(),
                "/session".to_string(),
                "/sessions".to_string(),
                "/mcp".to_string(),
                "/mcps".to_string(),
                "/skill".to_string(),
                "/export".to_string(),
                "/stash".to_string(),
                "/new".to_string(),
                "/clear".to_string(),
                "/share".to_string(),
                "/unshare".to_string(),
                "/rename".to_string(),
                "/fork".to_string(),
                "/compact".to_string(),
                "/timeline".to_string(),
                "/undo".to_string(),
                "/redo".to_string(),
                "/copy".to_string(),
                "/themes".to_string(),
                "/timestamps".to_string(),
                "/tips.toggle".to_string(),
                "/tips".to_string(),
                "/thinking".to_string(),
                "/density".to_string(),
                "/highlight".to_string(),
                "/sidebar".to_string(),
                "/command".to_string(),
                "/connect".to_string(),
                "/editor".to_string(),
                "/exit".to_string(),
                "/detach".to_string(),
                "/quit".to_string(),
            ],
            known_agents: vec![
                "build".to_string(),
                "plan".to_string(),
                "explore".to_string(),
                "compaction".to_string(),
                "title".to_string(),
            ],
            known_skills: Vec::new(),
            history_path,
            frecency_path,
            stash_path,
            file_index: FileIndex::default(),
            spinner,
            mode: PromptMode::Normal,
            interrupt_press_count: 0,
            last_interrupt_time: None,
        };
        prompt.recompute_suggestions();
        prompt
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    pub fn set_agent_suggestions(&mut self, agents: Vec<String>) {
        if agents.is_empty() {
            return;
        }
        self.known_agents = dedup_sort(agents);
        self.recompute_suggestions();
    }

    pub fn set_skill_suggestions(&mut self, skills: Vec<String>) {
        self.known_skills = dedup_sort(skills);
        self.recompute_suggestions();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let theme = self.context.theme.read();
        let agent = self.context.current_agent.read();
        let model = self.context.current_model.read();
        let variant = self.context.current_model_variant();
        let animations_enabled = *self.context.animations_enabled.read();

        let highlight_color = prompt_agent_color(&theme, agent.as_str());
        let active_color = if matches!(self.mode, PromptMode::Shell) {
            theme.primary
        } else {
            highlight_color
        };
        let placeholder = if matches!(self.mode, PromptMode::Shell) {
            SHELL_PLACEHOLDER
        } else {
            self.placeholder.as_str()
        };

        let max_content_lines = area
            .height
            .saturating_sub(3)
            .saturating_sub(PROMPT_BLOCK_PAD_TOP)
            .saturating_sub(PROMPT_BLOCK_PAD_BOTTOM)
            .max(PROMPT_MIN_INPUT_LINES);
        let input_width = prompt_input_width(area.width);
        let wrapped_input = wrap_prompt_input(&self.input, input_width);
        let cursor_visual_position = wrapped_input.cursor_visual_position(
            self.input.len(),
            self.cursor_position.min(self.input.len()),
            input_width,
        );
        let needed_lines = wrapped_input
            .lines
            .len()
            .max(cursor_visual_position.row.saturating_add(1));
        let content_lines = u16::try_from(needed_lines)
            .unwrap_or(u16::MAX)
            .max(PROMPT_MIN_INPUT_LINES)
            .min(max_content_lines);
        let input_scroll = cursor_visual_position
            .row
            .saturating_sub(usize::from(content_lines.saturating_sub(1)));
        let input_lines = content_lines
            .saturating_add(PROMPT_BLOCK_PAD_TOP)
            .saturating_add(PROMPT_BLOCK_PAD_BOTTOM);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(input_lines),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let border_set = ratatui::symbols::border::Set {
            top_left: " ",
            top_right: " ",
            bottom_left: " ",
            bottom_right: " ",
            vertical_left: "┃",
            vertical_right: " ",
            horizontal_top: " ",
            horizontal_bottom: " ",
        };

        let paragraph = if self.input.is_empty() {
            Paragraph::new(Line::from(Span::styled(
                placeholder,
                Style::default().fg(theme.text_muted),
            )))
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_set(border_set)
                    .border_style(Style::default().fg(active_color))
                    .padding(Padding::new(
                        PROMPT_BLOCK_PAD_LEFT,
                        PROMPT_BLOCK_PAD_RIGHT,
                        PROMPT_BLOCK_PAD_TOP,
                        PROMPT_BLOCK_PAD_BOTTOM,
                    ))
                    .style(Style::default().bg(theme.background_element)),
            )
            .style(Style::default().fg(if self.focused {
                theme.text
            } else {
                theme.text_muted
            }))
        } else {
            let mut display_lines: Vec<Line> = wrapped_input
                .lines
                .iter()
                .map(|line| Line::from(line.clone()))
                .collect();
            while display_lines.len() < needed_lines {
                display_lines.push(Line::from(""));
            }
            Paragraph::new(display_lines)
                .block(
                    Block::default()
                        .borders(Borders::LEFT)
                        .border_set(border_set)
                        .border_style(Style::default().fg(active_color))
                        .padding(Padding::new(
                            PROMPT_BLOCK_PAD_LEFT,
                            PROMPT_BLOCK_PAD_RIGHT,
                            PROMPT_BLOCK_PAD_TOP,
                            PROMPT_BLOCK_PAD_BOTTOM,
                        ))
                        .style(Style::default().bg(theme.background_element)),
                )
                .scroll((u16::try_from(input_scroll).unwrap_or(u16::MAX), 0))
                .style(Style::default().fg(if self.focused {
                    theme.text
                } else {
                    theme.text_muted
                }))
        };

        frame.render_widget(paragraph, chunks[0]);
        if self.focused {
            let cursor_col = cursor_visual_position.col;
            let visible_row = cursor_visual_position.row.saturating_sub(input_scroll);
            let cursor_row = u16::try_from(visible_row).unwrap_or(u16::MAX);
            let content_x = chunks[0]
                .x
                .saturating_add(1)
                .saturating_add(PROMPT_BLOCK_PAD_LEFT);
            let content_y = chunks[0].y.saturating_add(PROMPT_BLOCK_PAD_TOP);
            frame.set_cursor(
                content_x.saturating_add(cursor_col),
                content_y.saturating_add(cursor_row.min(content_lines.saturating_sub(1))),
            );
        }

        let mut info_parts = vec![
            Span::styled(
                if matches!(self.mode, PromptMode::Shell) {
                    "shell"
                } else {
                    agent.as_str()
                },
                Style::default().fg(active_color).bold(),
            ),
            Span::raw("  "),
        ];

        if let Some(m) = model.as_ref() {
            let provider = self.context.current_provider.read();
            if let Some(ref p) = *provider {
                info_parts.push(Span::styled(m.clone(), Style::default().fg(theme.text)));
                info_parts.push(Span::styled(
                    format!(" {p}"),
                    Style::default().fg(theme.text_muted),
                ));
            } else {
                info_parts.push(Span::styled(m.clone(), Style::default().fg(theme.text)));
            }
            if let Some(ref v) = variant {
                info_parts.push(Span::styled(" · ", Style::default().fg(theme.text_muted)));
                info_parts.push(Span::styled(
                    v.clone(),
                    Style::default().fg(theme.warning).bold(),
                ));
            }
        }

        render_prompt_continuation_row(frame, chunks[1], active_color, theme.background_element);
        let info_row = row_content_area(chunks[1], PROMPT_LINE_H_INSET);
        let info_line = Line::from(info_parts);
        let info_paragraph =
            Paragraph::new(info_line).style(Style::default().bg(theme.background_element));
        frame.render_widget(info_paragraph, info_row);

        let spinner_row = inset_horizontal(chunks[2], PROMPT_LINE_H_INSET);
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(theme.background)),
            spinner_row,
        );
        let spinner_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(9), Constraint::Min(0)])
            .split(spinner_row);

        self.spinner.render(
            frame,
            spinner_chunks[0],
            animations_enabled,
            theme.background,
        );

        let status_line = Paragraph::new(self.render_status_line(&theme))
            .style(Style::default().bg(theme.background));
        frame.render_widget(
            status_line,
            inset_horizontal(chunks[3], PROMPT_LINE_H_INSET),
        );
    }

    pub fn tick_spinner(&mut self, delta_ms: u64) -> bool {
        let spinner_changed = self.spinner.advance(delta_ms);
        let interrupt_changed = self.maybe_reset_interrupt_confirmation();
        spinner_changed || interrupt_changed
    }

    pub fn set_spinner_active(&mut self, active: bool) {
        self.spinner.set_active(active);
        if !active {
            self.reset_interrupt_confirmation();
        }
    }

    pub fn spinner_active(&self) -> bool {
        self.spinner.is_active()
    }

    pub fn set_spinner_task_kind(&mut self, task_kind: TaskKind) {
        self.spinner.set_task_kind(task_kind);
    }

    pub fn spinner_task_kind(&self) -> TaskKind {
        self.spinner.task_kind()
    }

    pub fn set_spinner_color(&mut self, color: Color) {
        self.spinner.set_color(color);
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    match c {
                        'b' | 'B' => {
                            self.cursor_position =
                                prev_word_boundary(&self.input, self.cursor_position);
                            return false;
                        }
                        'f' | 'F' => {
                            self.cursor_position =
                                next_word_boundary(&self.input, self.cursor_position);
                            return false;
                        }
                        _ => {}
                    }
                }
                if c == '!'
                    && key.modifiers.is_empty()
                    && matches!(self.mode, PromptMode::Normal)
                    && self.cursor_position == 0
                    && self.input.is_empty()
                {
                    self.mode = PromptMode::Shell;
                    return false;
                }
                self.input.insert(self.cursor_position, c);
                self.cursor_position += c.len_utf8();
                self.reset_history_cursor();
                self.recompute_suggestions();
            }
            KeyCode::Backspace => {
                if matches!(self.mode, PromptMode::Shell) && self.cursor_position == 0 {
                    self.mode = PromptMode::Normal;
                    return false;
                }
                if let Some(prev) = prev_char_boundary(&self.input, self.cursor_position) {
                    self.input.replace_range(prev..self.cursor_position, "");
                    self.cursor_position = prev;
                    self.reset_history_cursor();
                    self.recompute_suggestions();
                }
            }
            KeyCode::Delete => {
                if let Some(next) = next_char_boundary(&self.input, self.cursor_position) {
                    self.input.replace_range(self.cursor_position..next, "");
                    self.reset_history_cursor();
                    self.recompute_suggestions();
                }
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    self.cursor_position = prev_word_boundary(&self.input, self.cursor_position);
                } else if let Some(prev) = prev_char_boundary(&self.input, self.cursor_position) {
                    self.cursor_position = prev;
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::ALT) {
                    self.cursor_position = next_word_boundary(&self.input, self.cursor_position);
                } else if let Some(next) = next_char_boundary(&self.input, self.cursor_position) {
                    self.cursor_position = next;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.apply_autocomplete_previous();
                } else {
                    self.apply_autocomplete_next();
                }
            }
            KeyCode::BackTab => {
                self.apply_autocomplete_previous();
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    return true;
                }
            }
            KeyCode::Esc => {
                if matches!(self.mode, PromptMode::Shell) {
                    self.mode = PromptMode::Normal;
                }
            }
            KeyCode::Up => {
                if self.cursor_position == 0 {
                    self.history_previous();
                } else {
                    self.move_cursor_vertical(true);
                }
            }
            KeyCode::Down => {
                if self.cursor_position >= self.input.len() {
                    self.history_next();
                } else {
                    self.move_cursor_vertical(false);
                }
            }
            _ => {}
        }

        false
    }

    pub fn history_previous_entry(&mut self) {
        self.history_previous();
    }

    pub fn history_next_entry(&mut self) {
        self.history_next();
    }

    pub fn autocomplete_next(&mut self) {
        self.apply_autocomplete_next();
    }

    pub fn autocomplete_previous(&mut self) {
        self.apply_autocomplete_previous();
    }

    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    pub fn take_input(&mut self) -> String {
        let input = std::mem::take(&mut self.input);
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            self.push_history(input.clone());
            self.bump_frecency(trimmed);
            if let Some(first) = trimmed.split_whitespace().next() {
                if first.starts_with('/') || first.starts_with('@') {
                    self.bump_frecency(first);
                }
            }
            store_history(&self.history_path, &self.history);
            store_frecency(&self.frecency_path, &self.frecency);
        }
        self.cursor_position = 0;
        self.history_index = None;
        self.history_draft = None;
        self.suggestions.clear();
        self.suggestion_index = None;
        self.mode = PromptMode::Normal;
        self.reset_interrupt_confirmation();
        input
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
        self.history_index = None;
        self.history_draft = None;
        self.suggestions.clear();
        self.suggestion_index = None;
        self.mode = PromptMode::Normal;
        self.reset_interrupt_confirmation();
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn set_input(&mut self, input: String) {
        self.input = input;
        self.cursor_position = self.input.len();
        self.history_index = None;
        self.history_draft = None;
        self.reset_interrupt_confirmation();
        self.recompute_suggestions();
    }

    pub fn mode(&self) -> PromptMode {
        self.mode
    }

    pub fn is_shell_mode(&self) -> bool {
        matches!(self.mode, PromptMode::Shell)
    }

    pub fn exit_shell_mode(&mut self) {
        self.mode = PromptMode::Normal;
    }

    pub fn register_interrupt_keypress(&mut self) -> bool {
        if self.interrupt_confirmation_active() {
            self.reset_interrupt_confirmation();
            return true;
        }
        self.interrupt_press_count = 1;
        self.last_interrupt_time = Some(Instant::now());
        false
    }

    pub fn clear_interrupt_confirmation(&mut self) {
        self.reset_interrupt_confirmation();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.input.insert_str(self.cursor_position, text);
        self.cursor_position = self.cursor_position.saturating_add(text.len());
        self.reset_history_cursor();
        self.recompute_suggestions();
    }

    pub fn desired_height(&self, width: u16) -> u16 {
        self.prompt_content_lines(width)
            .saturating_add(PROMPT_BLOCK_PAD_TOP)
            .saturating_add(PROMPT_BLOCK_PAD_BOTTOM)
            .saturating_add(3)
    }

    pub fn stash_current(&mut self) -> bool {
        if self.input.trim().is_empty() {
            return false;
        }

        self.stash.push(PromptStashEntry {
            input: self.input.clone(),
            created_at: chrono::Utc::now().timestamp_millis(),
        });
        if self.stash.len() > MAX_STASH_ENTRIES {
            let overflow = self.stash.len().saturating_sub(MAX_STASH_ENTRIES);
            self.stash.drain(0..overflow);
        }
        store_stash(&self.stash_path, &self.stash);
        self.clear();
        true
    }

    pub fn stash_entries(&self) -> &[PromptStashEntry] {
        &self.stash
    }

    pub fn pop_stash(&mut self) -> Option<PromptStashEntry> {
        let entry = self.stash.pop();
        if entry.is_some() {
            store_stash(&self.stash_path, &self.stash);
        }
        entry
    }

    pub fn remove_stash(&mut self, index: usize) -> bool {
        if index >= self.stash.len() {
            return false;
        }
        self.stash.remove(index);
        store_stash(&self.stash_path, &self.stash);
        true
    }

    pub fn load_stash(&mut self, index: usize) -> bool {
        let Some(entry) = self.stash.get(index) else {
            return false;
        };
        self.set_input(entry.input.clone());
        true
    }

    fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        match self.history_index {
            Some(idx) => {
                if idx > 0 {
                    self.history_index = Some(idx - 1);
                }
            }
            None => {
                self.history_draft = Some(self.input.clone());
                self.history_index = Some(self.history.len() - 1);
            }
        }

        if let Some(idx) = self.history_index {
            self.input = self.history[idx].clone();
            self.cursor_position = 0;
            self.recompute_suggestions();
        }
    }

    fn history_next(&mut self) {
        let Some(idx) = self.history_index else {
            return;
        };

        if idx + 1 < self.history.len() {
            let next = idx + 1;
            self.history_index = Some(next);
            self.input = self.history[next].clone();
            self.cursor_position = self.input.len();
        } else {
            self.history_index = None;
            self.input = self.history_draft.take().unwrap_or_default();
            self.cursor_position = self.input.len();
        }
        self.recompute_suggestions();
    }

    fn reset_history_cursor(&mut self) {
        self.history_index = None;
        self.history_draft = None;
    }

    fn move_cursor_vertical(&mut self, up: bool) {
        let cursor = self.cursor_position.min(self.input.len());
        let (line_start, line_end) = line_bounds(&self.input, cursor);
        let column = self.input[line_start..cursor].chars().count();

        if up {
            if line_start == 0 {
                self.cursor_position = 0;
                return;
            }
            let prev_end = line_start - 1;
            let (prev_start, _) = line_bounds(&self.input, prev_end);
            let target = byte_offset_for_column(&self.input[prev_start..prev_end], column);
            self.cursor_position = prev_start + target;
        } else {
            if line_end >= self.input.len() {
                self.cursor_position = self.input.len();
                return;
            }
            let next_start = line_end + 1;
            let (_, next_end) = line_bounds(&self.input, next_start);
            let target = byte_offset_for_column(&self.input[next_start..next_end], column);
            self.cursor_position = next_start + target;
        }
    }

    fn push_history(&mut self, entry: String) {
        if self
            .history
            .last()
            .is_some_and(|existing| existing == &entry)
        {
            return;
        }
        self.history.push(entry);
        if self.history.len() > MAX_HISTORY_ENTRIES {
            let overflow = self.history.len().saturating_sub(MAX_HISTORY_ENTRIES);
            self.history.drain(0..overflow);
        }
    }

    fn bump_frecency(&mut self, key: &str) {
        let key = key.trim();
        if key.is_empty() {
            return;
        }
        let now = chrono::Utc::now().timestamp_millis();
        let entry = self.frecency.entry(key.to_string()).or_default();
        entry.frequency = entry.frequency.saturating_add(1);
        entry.last_used = now;

        if self.frecency.len() > MAX_FRECENCY_ENTRIES {
            let mut items = self.frecency.iter().collect::<Vec<_>>();
            items.sort_by(|(_, a), (_, b)| b.last_used.cmp(&a.last_used));
            let keep = items
                .into_iter()
                .take(MAX_FRECENCY_ENTRIES)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<HashMap<_, _>>();
            self.frecency = keep;
        }
    }

    fn frecency_score(&self, key: &str) -> f64 {
        let Some(entry) = self.frecency.get(key) else {
            return 0.0;
        };
        let now = chrono::Utc::now().timestamp_millis();
        let days_since = (now - entry.last_used).max(0) as f64 / 86_400_000.0;
        let weight = 1.0 / (1.0 + days_since);
        entry.frequency as f64 * weight
    }

    fn selected_suggestion(&self) -> Option<&str> {
        self.suggestion_index
            .and_then(|idx| self.suggestions.get(idx))
            .map(String::as_str)
    }

    fn refresh_file_index_if_needed(&mut self) {
        let directory = self.context.directory.read().clone();
        let root = if directory.trim().is_empty() {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            PathBuf::from(directory)
        };
        self.file_index.refresh(&root, FILE_INDEX_MAX_DEPTH);
    }

    fn push_candidate(
        scored: &mut Vec<(String, i32)>,
        dedup: &mut HashMap<String, ()>,
        token: &str,
        token_lower: &str,
        item: String,
        pre_scored: Option<i32>,
    ) {
        if item.eq_ignore_ascii_case(token) {
            return;
        }
        if dedup.insert(item.to_lowercase(), ()).is_some() {
            return;
        }

        let score = pre_scored
            .or_else(|| crate::command::fuzzy_match(token, &item))
            .or_else(|| {
                if item.to_lowercase().starts_with(token_lower) {
                    Some(1)
                } else {
                    None
                }
            });
        if let Some(score) = score {
            scored.push((item, score));
        }
    }

    fn recompute_suggestions(&mut self) {
        let Some((_, _, token)) = self.current_token() else {
            self.suggestions.clear();
            self.suggestion_index = None;
            return;
        };

        if token.is_empty() {
            self.suggestions.clear();
            self.suggestion_index = None;
            return;
        }

        let token_lower = token.to_lowercase();
        let mut dedup = HashMap::<String, ()>::new();
        let mut scored: Vec<(String, i32)> = Vec::new();
        if token.starts_with('/') {
            for item in self.known_commands.iter().cloned() {
                Self::push_candidate(
                    &mut scored,
                    &mut dedup,
                    token.as_str(),
                    token_lower.as_str(),
                    item,
                    None,
                );
            }
            for item in self
                .known_skills
                .iter()
                .map(|skill| format!("/{}", skill.trim()))
            {
                Self::push_candidate(
                    &mut scored,
                    &mut dedup,
                    token.as_str(),
                    token_lower.as_str(),
                    item,
                    None,
                );
            }
        } else if token.starts_with('@') {
            // Agent completions
            for item in self
                .known_agents
                .iter()
                .map(|agent| format!("@{}", agent.trim()))
            {
                Self::push_candidate(
                    &mut scored,
                    &mut dedup,
                    token.as_str(),
                    token_lower.as_str(),
                    item,
                    None,
                );
            }
            // File path completions: strip @ prefix and optional #line range
            let after_at = &token[1..];
            let (file_part, _line_range) = extract_line_range(after_at);
            let range_suffix = after_at
                .strip_prefix(file_part)
                .filter(|suffix| suffix.starts_with('#'))
                .unwrap_or("");
            self.refresh_file_index_if_needed();
            let path_score_boost = if file_part.trim().is_empty() { 0 } else { 1024 };
            for (path, path_score) in self.file_index.search(file_part, FILE_SUGGESTION_LIMIT) {
                let candidate = format!("@{}{}", path, range_suffix);
                let path_score = i32::try_from(path_score).unwrap_or(i32::MAX);
                let score = path_score.saturating_add(path_score_boost);
                Self::push_candidate(
                    &mut scored,
                    &mut dedup,
                    token.as_str(),
                    token_lower.as_str(),
                    candidate,
                    Some(score),
                );
            }
        } else {
            for item in self.history.iter().rev().cloned() {
                let history_score = legacy_subsequence_score(token.as_str(), item.as_str());
                Self::push_candidate(
                    &mut scored,
                    &mut dedup,
                    token.as_str(),
                    token_lower.as_str(),
                    item,
                    history_score,
                );
            }
        }

        scored.sort_by(|a, b| {
            let frecency_cmp = self
                .frecency_score(&b.0)
                .partial_cmp(&self.frecency_score(&a.0))
                .unwrap_or(std::cmp::Ordering::Equal);
            b.1.cmp(&a.1)
                .then(frecency_cmp)
                .then_with(|| a.0.len().cmp(&b.0.len()))
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });

        self.suggestions = scored.into_iter().map(|(item, _)| item).collect();
        self.suggestion_index = if self.suggestions.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    fn apply_autocomplete_next(&mut self) {
        if self.suggestions.is_empty() {
            self.recompute_suggestions();
            if self.suggestions.is_empty() {
                return;
            }
        }

        self.suggestion_index = Some(self.suggestion_index.unwrap_or(0));
        self.apply_selected_suggestion();
    }

    fn apply_autocomplete_previous(&mut self) {
        if self.suggestions.is_empty() {
            self.recompute_suggestions();
            if self.suggestions.is_empty() {
                return;
            }
        }

        self.suggestion_index = Some(
            self.suggestion_index
                .map(|current| {
                    if current == 0 {
                        self.suggestions.len() - 1
                    } else {
                        current - 1
                    }
                })
                .unwrap_or(self.suggestions.len() - 1),
        );
        self.apply_selected_suggestion();
    }

    fn apply_selected_suggestion(&mut self) {
        let Some((start, end, _)) = self.current_token() else {
            return;
        };
        let Some(suggestion) = self.selected_suggestion().map(ToString::to_string) else {
            return;
        };

        self.input.replace_range(start..end, &suggestion);
        self.cursor_position = start + suggestion.len();
        if (suggestion.starts_with('/') || suggestion.starts_with('@'))
            && self.cursor_position < self.input.len()
            && !self
                .input
                .as_bytes()
                .get(self.cursor_position)
                .copied()
                .is_some_and(|b| b.is_ascii_whitespace())
        {
            self.input.insert(self.cursor_position, ' ');
            self.cursor_position += 1;
        }
        self.recompute_suggestions();
    }

    fn current_token(&self) -> Option<(usize, usize, String)> {
        if self.input.is_empty() || self.cursor_position > self.input.len() {
            return None;
        }

        let bytes = self.input.as_bytes();
        let mut start = self.cursor_position;
        while start > 0 && !bytes[start - 1].is_ascii_whitespace() {
            start -= 1;
        }

        let mut end = self.cursor_position;
        while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }

        let token = self.input[start..self.cursor_position].to_string();
        Some((start, end, token))
    }

    fn prompt_content_lines(&self, width: u16) -> u16 {
        let input_width = prompt_input_width(width);
        let wrapped = wrap_prompt_input(&self.input, input_width);
        let cursor = wrapped.cursor_visual_position(
            self.input.len(),
            self.cursor_position.min(self.input.len()),
            input_width,
        );
        let needed = wrapped.lines.len().max(cursor.row.saturating_add(1));
        u16::try_from(needed)
            .unwrap_or(u16::MAX)
            .clamp(PROMPT_MIN_INPUT_LINES, PROMPT_MAX_INPUT_LINES)
    }

    fn render_status_line(&self, theme: &Theme) -> Line<'static> {
        if let Some(status) = self.current_session_status() {
            return self.status_line_for_session(status, theme);
        }
        self.hint_line(theme)
    }

    fn current_session_status(&self) -> Option<SessionStatus> {
        let session_id = match self.context.current_route() {
            crate::router::Route::Session { session_id } => session_id,
            _ => return None,
        };
        let session_ctx = self.context.session.read();
        Some(session_ctx.status(&session_id).clone())
    }

    fn status_line_for_session(&self, status: SessionStatus, theme: &Theme) -> Line<'static> {
        if matches!(self.mode, PromptMode::Shell) {
            return Line::from(vec![
                Span::styled("esc", Style::default().fg(theme.text)),
                Span::styled(" exit shell mode", Style::default().fg(theme.text_muted)),
            ]);
        }
        let interrupt = self.context.keybind.read().print("session_interrupt");
        match status {
            SessionStatus::Retrying {
                message,
                attempt,
                next,
            } => {
                let now = chrono::Utc::now().timestamp_millis();
                let secs = ((next - now) / 1000).max(0);
                let truncated = truncate_for_status(&message, 72);
                Line::from(vec![
                    Span::styled(
                        format!("retrying in {}s (#{}) ", secs, attempt),
                        Style::default().fg(theme.warning),
                    ),
                    Span::styled(truncated, Style::default().fg(theme.text_muted)),
                    Span::raw("  "),
                    Span::styled(interrupt, Style::default().fg(theme.text)),
                    Span::styled(" interrupt", Style::default().fg(theme.text_muted)),
                ])
            }
            SessionStatus::Running => {
                let mut spans = vec![
                    Span::styled("thinking", Style::default().fg(theme.text_muted)),
                    Span::raw("  "),
                ];
                if self.interrupt_confirmation_active() {
                    spans.push(Span::styled(interrupt, Style::default().fg(theme.warning)));
                    spans.push(Span::styled(
                        " again to interrupt",
                        Style::default().fg(theme.warning),
                    ));
                } else {
                    spans.push(Span::styled(interrupt, Style::default().fg(theme.text)));
                    spans.push(Span::styled(
                        " interrupt",
                        Style::default().fg(theme.text_muted),
                    ));
                }
                Line::from(spans)
            }
            SessionStatus::Idle => self.hint_line(theme),
        }
    }

    fn interrupt_confirmation_active(&self) -> bool {
        if self.interrupt_press_count == 0 {
            return false;
        }
        self.last_interrupt_time
            .is_some_and(|t| t.elapsed() < Duration::from_secs(INTERRUPT_CONFIRM_WINDOW_SECS))
    }

    fn maybe_reset_interrupt_confirmation(&mut self) -> bool {
        if !self.interrupt_confirmation_active() {
            return self.reset_interrupt_confirmation();
        }
        false
    }

    fn reset_interrupt_confirmation(&mut self) -> bool {
        let changed = self.interrupt_press_count != 0 || self.last_interrupt_time.is_some();
        self.interrupt_press_count = 0;
        self.last_interrupt_time = None;
        changed
    }

    fn hint_line(&self, theme: &Theme) -> Line<'static> {
        let keybind = self.context.keybind.read();
        let variant_cycle = keybind.print("variant_cycle");
        let agent_cycle = keybind.print("agent_cycle");
        let command_list = keybind.print("command_list");
        drop(keybind);

        let mut spans = Vec::new();
        if self.context.current_model_variant().is_some() {
            spans.push(Span::styled(variant_cycle, Style::default().fg(theme.text)));
            spans.push(Span::styled(
                " variants",
                Style::default().fg(theme.text_muted),
            ));
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(agent_cycle, Style::default().fg(theme.text)));
        spans.push(Span::styled(
            " agents",
            Style::default().fg(theme.text_muted),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(command_list, Style::default().fg(theme.text)));
        spans.push(Span::styled(
            " commands",
            Style::default().fg(theme.text_muted),
        ));
        Line::from(spans)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorVisualPosition {
    row: usize,
    col: u16,
}

fn prompt_input_width(width: u16) -> u16 {
    let reserved = 1u16
        .saturating_add(PROMPT_BLOCK_PAD_LEFT)
        .saturating_add(PROMPT_BLOCK_PAD_RIGHT);
    width.saturating_sub(reserved).max(1)
}

fn prompt_grapheme_is_whitespace(grapheme: &str) -> bool {
    grapheme == "\u{200b}" || (grapheme != "\u{00a0}" && grapheme.chars().all(char::is_whitespace))
}

fn prompt_grapheme_width(grapheme: &str) -> usize {
    UnicodeWidthStr::width(grapheme)
}

fn prompt_text_width(text: &str) -> usize {
    text.graphemes(true).map(prompt_grapheme_width).sum()
}

fn saturating_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[derive(Debug, Clone, Default)]
struct WrappedPromptInput {
    lines: Vec<String>,
    positions: HashMap<usize, (usize, u16)>,
}

impl WrappedPromptInput {
    fn cursor_visual_position(
        &self,
        input_len: usize,
        cursor_position: usize,
        width: u16,
    ) -> CursorVisualPosition {
        let width = usize::from(width.max(1));
        let cursor_position = cursor_position.min(input_len);
        let mut position = self.positions.get(&cursor_position).copied();
        if position.is_none() && cursor_position < input_len {
            position = self
                .positions
                .iter()
                .filter(|(offset, _)| **offset <= cursor_position)
                .max_by_key(|(offset, _)| **offset)
                .map(|(_, value)| *value);
        }

        if let Some((line, col)) = position {
            let col = usize::from(col);
            if col >= width {
                return CursorVisualPosition {
                    row: line.saturating_add(1),
                    col: 0,
                };
            }
            return CursorVisualPosition {
                row: line,
                col: saturating_u16(col),
            };
        }

        let last_line = self.lines.len().saturating_sub(1);
        let last_width = self
            .lines
            .get(last_line)
            .map_or(0, |line| prompt_text_width(line));
        if last_width >= width {
            CursorVisualPosition {
                row: self.lines.len(),
                col: 0,
            }
        } else {
            CursorVisualPosition {
                row: last_line,
                col: saturating_u16(last_width),
            }
        }
    }
}

fn append_pending_prompt_whitespace(
    current: &mut String,
    current_width: &mut usize,
    pending_ws: &mut Vec<(usize, String)>,
    pending_ws_width: &mut usize,
    positions: &mut HashMap<usize, (usize, u16)>,
    line_index: usize,
) {
    for (offset, grapheme) in pending_ws.drain(..) {
        positions.insert(offset, (line_index, saturating_u16(*current_width)));
        *current_width = current_width.saturating_add(prompt_grapheme_width(&grapheme));
        current.push_str(&grapheme);
    }
    *pending_ws_width = 0;
}

fn wrap_prompt_input(input: &str, width: u16) -> WrappedPromptInput {
    let width = usize::from(width.max(1));
    let mut lines: Vec<String> = Vec::new();
    let mut positions: HashMap<usize, (usize, u16)> = HashMap::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut pending_ws: Vec<(usize, String)> = Vec::new();
    let mut pending_ws_width = 0usize;

    let graphemes: Vec<(usize, &str)> = input.grapheme_indices(true).collect();
    let mut index = 0usize;
    while index < graphemes.len() {
        let (byte_offset, grapheme) = graphemes[index];
        if grapheme == "\n" {
            append_pending_prompt_whitespace(
                &mut current,
                &mut current_width,
                &mut pending_ws,
                &mut pending_ws_width,
                &mut positions,
                lines.len(),
            );
            positions.insert(byte_offset, (lines.len(), saturating_u16(current_width)));
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            index += 1;
            continue;
        }
        if prompt_grapheme_is_whitespace(grapheme) {
            pending_ws_width = pending_ws_width.saturating_add(prompt_grapheme_width(grapheme));
            pending_ws.push((byte_offset, grapheme.to_string()));
            index += 1;
            continue;
        }

        let word_start = index;
        let mut word_end = index;
        let mut word_width = 0usize;
        while word_end < graphemes.len() {
            let (_, candidate) = graphemes[word_end];
            if candidate == "\n" || prompt_grapheme_is_whitespace(candidate) {
                break;
            }
            word_width = word_width.saturating_add(prompt_grapheme_width(candidate));
            word_end += 1;
        }

        if !current.is_empty() && current_width + pending_ws_width + word_width > width {
            for (offset, _) in &pending_ws {
                positions.insert(*offset, (lines.len(), saturating_u16(current_width)));
            }
            pending_ws.clear();
            pending_ws_width = 0;
            lines.push(std::mem::take(&mut current));
            current_width = 0;
        } else {
            for (offset, whitespace) in pending_ws.drain(..) {
                positions.insert(offset, (lines.len(), saturating_u16(current_width)));
                current_width = current_width.saturating_add(prompt_grapheme_width(&whitespace));
                current.push_str(&whitespace);
            }
            pending_ws_width = 0;
        }

        for (offset, grapheme) in graphemes[word_start..word_end].iter().copied() {
            let grapheme_width = prompt_grapheme_width(grapheme);
            if current_width > 0 && current_width + grapheme_width > width {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            positions.insert(offset, (lines.len(), saturating_u16(current_width)));
            current.push_str(grapheme);
            current_width = current_width.saturating_add(grapheme_width);
        }

        index = word_end;
    }

    append_pending_prompt_whitespace(
        &mut current,
        &mut current_width,
        &mut pending_ws,
        &mut pending_ws_width,
        &mut positions,
        lines.len(),
    );
    lines.push(std::mem::take(&mut current));
    if lines.is_empty() {
        lines.push(String::new());
    }

    WrappedPromptInput { lines, positions }
}

fn truncate_for_status(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out = String::with_capacity(max_chars + 1);
    for ch in input.chars().take(max_chars.saturating_sub(1)) {
        out.push(ch);
    }
    out.push('…');
    out
}

fn dedup_sort(mut items: Vec<String>) -> Vec<String> {
    items.retain(|item| !item.trim().is_empty());
    items.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    items.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    items
}

fn inset_horizontal(area: Rect, padding: u16) -> Rect {
    if area.width <= padding.saturating_mul(2) {
        return area;
    }
    Rect {
        x: area.x.saturating_add(padding),
        y: area.y,
        width: area.width.saturating_sub(padding.saturating_mul(2)),
        height: area.height,
    }
}

fn row_content_area(area: Rect, horizontal_padding: u16) -> Rect {
    if area.width <= 1 {
        return area;
    }
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y,
        width: area.width.saturating_sub(1),
        height: area.height,
    };
    inset_horizontal(inner, horizontal_padding)
}

fn render_prompt_continuation_row(
    frame: &mut Frame,
    area: Rect,
    border_color: Color,
    background: Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut spans = vec![Span::styled(
        "┃",
        Style::default().fg(border_color).bg(background),
    )];
    let trailing = usize::from(area.width).saturating_sub(1);
    if trailing > 0 {
        spans.push(Span::styled(
            " ".repeat(trailing),
            Style::default().bg(background),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn legacy_subsequence_score(query: &str, target: &str) -> Option<i32> {
    let query_lower = query.trim().to_lowercase();
    if query_lower.is_empty() {
        return Some(0);
    }
    let target_lower = target.to_lowercase();
    let mut score = 0i32;
    let mut query_idx = 0usize;
    let query_chars: Vec<char> = query_lower.chars().collect();
    for (idx, ch) in target_lower.chars().enumerate() {
        if query_idx < query_chars.len() && ch == query_chars[query_idx] {
            score += if idx == 0 || query_idx == 0 { 10 } else { 5 };
            query_idx += 1;
        }
    }
    if query_idx == query_chars.len() {
        Some(score)
    } else {
        None
    }
}

fn prompt_agent_color(theme: &Theme, agent_name: &str) -> Color {
    if theme.agent_colors.is_empty() {
        return theme.primary;
    }

    let mut hasher = DefaultHasher::new();
    agent_name.hash(&mut hasher);
    let idx = (hasher.finish() as usize) % theme.agent_colors.len();
    theme.agent_colors[idx]
}

fn spinner_mode_from_env() -> SpinnerMode {
    match std::env::var("OPENCODE_TUI_SPINNER")
        .ok()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "knight" | "knight-rider" | "scan" | "scanner" => SpinnerMode::KnightRider,
        _ => SpinnerMode::Braille,
    }
}

fn prev_char_boundary(input: &str, cursor_position: usize) -> Option<usize> {
    if cursor_position == 0 || cursor_position > input.len() {
        return None;
    }
    input[..cursor_position]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
}

fn next_char_boundary(input: &str, cursor_position: usize) -> Option<usize> {
    if cursor_position >= input.len() {
        return None;
    }
    let suffix = &input[cursor_position..];
    suffix
        .chars()
        .next()
        .map(|ch| cursor_position + ch.len_utf8())
}

fn prev_word_boundary(input: &str, cursor_position: usize) -> usize {
    let mut position = cursor_position.min(input.len());
    while let Some((prev, ch)) = prev_char(input, position) {
        if is_word_char(ch) {
            break;
        }
        position = prev;
    }
    while let Some((prev, ch)) = prev_char(input, position) {
        if !is_word_char(ch) {
            break;
        }
        position = prev;
    }
    position
}

fn next_word_boundary(input: &str, cursor_position: usize) -> usize {
    let mut position = cursor_position.min(input.len());
    while let Some((next, ch)) = next_char(input, position) {
        if is_word_char(ch) {
            break;
        }
        position = next;
    }
    while let Some((next, ch)) = next_char(input, position) {
        if !is_word_char(ch) {
            break;
        }
        position = next;
    }
    position
}

fn prev_char(input: &str, cursor_position: usize) -> Option<(usize, char)> {
    if cursor_position == 0 || cursor_position > input.len() {
        return None;
    }
    input[..cursor_position].char_indices().last()
}

fn next_char(input: &str, cursor_position: usize) -> Option<(usize, char)> {
    if cursor_position >= input.len() {
        return None;
    }
    let ch = input[cursor_position..].chars().next()?;
    Some((cursor_position + ch.len_utf8(), ch))
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn line_bounds(input: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(input.len());
    let start = input[..offset].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let end = input[offset..]
        .find('\n')
        .map(|idx| offset + idx)
        .unwrap_or(input.len());
    (start, end)
}

fn byte_offset_for_column(line: &str, column: usize) -> usize {
    line.char_indices()
        .nth(column)
        .map(|(idx, _)| idx)
        .unwrap_or(line.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn with_isolated_prompt<T>(f: impl FnOnce(Prompt) -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let state_dir =
            std::env::temp_dir().join(format!("opencode-tui-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&state_dir).expect("create state dir");
        let previous = std::env::var("OPENCODE_STATE_DIR").ok();
        std::env::set_var("OPENCODE_STATE_DIR", &state_dir);

        let context = Arc::new(AppContext::new());
        let prompt = Prompt::new(context);
        let result = f(prompt);

        if let Some(prev) = previous {
            std::env::set_var("OPENCODE_STATE_DIR", prev);
        } else {
            std::env::remove_var("OPENCODE_STATE_DIR");
        }
        let _ = std::fs::remove_dir_all(state_dir);
        result
    }

    #[test]
    fn plain_q_types_into_prompt() {
        with_isolated_prompt(|mut prompt| {
            prompt.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "q");
        });
    }

    #[test]
    fn tab_autocomplete_uses_first_candidate() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("team".to_string());
            let _ = prompt.take_input();
            prompt.set_input("test".to_string());
            let _ = prompt.take_input();
            prompt.set_input("te".to_string());

            prompt.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "team");
        });
    }

    #[test]
    fn tab_autocomplete_keeps_line_range_for_file_candidates() {
        with_isolated_prompt(|mut prompt| {
            let root =
                std::env::temp_dir().join(format!("opencode-tui-files-{}", uuid::Uuid::new_v4()));
            let src_dir = root.join("src");
            std::fs::create_dir_all(&src_dir).expect("create src dir");
            std::fs::write(src_dir.join("main.rs"), "fn main() {}\n").expect("write file");
            *prompt.context.directory.write() = root.to_string_lossy().to_string();

            prompt.set_input("@src/main#12-20".to_string());
            prompt.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "@src/main.rs#12-20");

            let _ = std::fs::remove_dir_all(root);
        });
    }

    #[test]
    fn bare_up_recalls_history_only_at_cursor_start() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("alpha".to_string());
            let _ = prompt.take_input();
            prompt.set_input("beta".to_string());
            let _ = prompt.take_input();
            prompt.set_input("draft".to_string());

            // Mid-draft Up snaps the cursor to the start without recalling.
            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "draft");
            assert_eq!(prompt.cursor_position(), 0);

            // A second Up now recalls the previous entry, cursor landing at start.
            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "beta");
            assert_eq!(prompt.cursor_position(), 0);
        });
    }

    #[test]
    fn bare_down_recalls_history_only_at_cursor_end() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("alpha".to_string());
            let _ = prompt.take_input();
            prompt.set_input("beta".to_string());
            let _ = prompt.take_input();
            prompt.set_input("draft".to_string());

            // Recall "beta"; entering history places the cursor at the start.
            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "beta");
            assert_eq!(prompt.cursor_position(), 0);

            // From the start, Down first snaps to the end without recalling...
            prompt.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "beta");
            assert_eq!(prompt.cursor_position(), "beta".len());

            // ...then the next Down recalls the preserved draft.
            prompt.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "draft");
            assert_eq!(prompt.cursor_position(), "draft".len());
        });
    }

    #[test]
    fn bare_arrows_navigate_multiline_draft_without_recall() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("old".to_string());
            let _ = prompt.take_input();
            prompt.set_input("ab\ncd".to_string());
            assert_eq!(prompt.cursor_position(), "ab\ncd".len());

            // Up walks the draft lines and never touches history.
            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "ab\ncd");
            assert_eq!(prompt.cursor_position(), "ab".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "ab\ncd");
            assert_eq!(prompt.cursor_position(), 0);

            // Only at the very start does Up recall.
            prompt.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "old");
            assert_eq!(prompt.cursor_position(), 0);

            // Down from the recalled entry's start navigates, then recalls the draft.
            prompt.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "old");
            assert_eq!(prompt.cursor_position(), "old".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "ab\ncd");
            assert_eq!(prompt.cursor_position(), "ab\ncd".len());
        });
    }

    #[test]
    fn explicit_history_navigation_is_ungated() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("alpha".to_string());
            let _ = prompt.take_input();
            prompt.set_input("draft".to_string());
            prompt.cursor_position = 2;

            prompt.history_previous_entry();
            assert_eq!(prompt.get_input(), "alpha");
        });
    }

    #[test]
    fn history_navigation_preserves_draft() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("alpha".to_string());
            let _ = prompt.take_input();
            prompt.set_input("beta".to_string());
            let _ = prompt.take_input();
            prompt.set_input("draft".to_string());

            prompt.history_previous_entry();
            assert_eq!(prompt.get_input(), "beta");

            prompt.history_previous_entry();
            assert_eq!(prompt.get_input(), "alpha");

            prompt.history_next_entry();
            assert_eq!(prompt.get_input(), "beta");

            prompt.history_next_entry();
            assert_eq!(prompt.get_input(), "draft");
        });
    }

    #[test]
    fn utf8_backspace_delete_and_cursor_are_char_safe() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("你好".to_string());

            prompt.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
            prompt.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "好");

            prompt.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
            prompt.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::empty()));
            assert_eq!(prompt.get_input(), "");
        });
    }

    #[test]
    fn alt_left_and_right_move_by_words() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("hello, world_again 你好".to_string());

            prompt.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "hello, world_again ".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "hello, ".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), 0);

            prompt.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "hello".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "hello, world_again".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "hello, world_again 你好".len());
        });
    }

    #[test]
    fn alt_b_and_f_move_by_words() {
        with_isolated_prompt(|mut prompt| {
            prompt.set_input("alpha beta".to_string());

            prompt.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "alpha ".len());

            prompt.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT));
            assert_eq!(prompt.cursor_position(), "alpha beta".len());
        });
    }

    fn cursor_position(input: &str, cursor: usize, width: u16) -> CursorVisualPosition {
        wrap_prompt_input(input, width).cursor_visual_position(input.len(), cursor, width)
    }

    #[test]
    fn cursor_visual_position_follows_wrapped_input() {
        assert_eq!(
            cursor_position("abcdef", 5, 3),
            CursorVisualPosition { row: 1, col: 2 }
        );
        assert_eq!(
            cursor_position("abcdef", 3, 3),
            CursorVisualPosition { row: 1, col: 0 }
        );
    }

    #[test]
    fn cursor_visual_position_uses_word_boundaries_like_renderer() {
        let wrapped = wrap_prompt_input("ab cdefgh", 6);
        assert_eq!(wrapped.lines, vec!["ab".to_string(), "cdefgh".to_string()]);
        assert_eq!(
            wrapped.cursor_visual_position("ab cdefgh".len(), 7, 6),
            CursorVisualPosition { row: 1, col: 4 }
        );

        let wrapped = wrap_prompt_input("hello world", 6);
        assert_eq!(
            wrapped.lines,
            vec!["hello".to_string(), "world".to_string()]
        );
        assert_eq!(
            wrapped.cursor_visual_position("hello world".len(), "hello world".len(), 6),
            CursorVisualPosition { row: 1, col: 5 }
        );
    }

    #[test]
    fn cursor_visual_position_handles_newlines_and_wide_chars() {
        assert_eq!(
            cursor_position("ab\ncd", 5, 10),
            CursorVisualPosition { row: 1, col: 2 }
        );

        let wrapped = wrap_prompt_input("你a好", 3);
        assert_eq!(wrapped.lines, vec!["你a".to_string(), "好".to_string()]);
        assert_eq!(
            wrapped.cursor_visual_position("你a好".len(), "你a".len(), 3),
            CursorVisualPosition { row: 1, col: 0 }
        );
        assert_eq!(
            wrapped.cursor_visual_position("你a好".len(), "你a好".len(), 3),
            CursorVisualPosition { row: 1, col: 2 }
        );
    }

    #[test]
    fn cursor_visual_position_handles_full_line_boundary() {
        let wrapped = wrap_prompt_input("abc", 3);
        assert_eq!(wrapped.lines, vec!["abc".to_string()]);
        assert_eq!(
            wrapped.cursor_visual_position(3, 2, 3),
            CursorVisualPosition { row: 0, col: 2 }
        );
        assert_eq!(
            wrapped.cursor_visual_position(3, 3, 3),
            CursorVisualPosition { row: 1, col: 0 }
        );
    }

    #[test]
    fn cursor_visual_position_handles_grapheme_clusters() {
        let combining = "e\u{301}x";
        let wrapped = wrap_prompt_input(combining, 3);
        assert_eq!(wrapped.lines, vec![combining.to_string()]);
        assert_eq!(
            wrapped.cursor_visual_position(combining.len(), "e\u{301}".len(), 3),
            CursorVisualPosition { row: 0, col: 1 }
        );
        assert_eq!(
            wrapped.cursor_visual_position(combining.len(), combining.len(), 3),
            CursorVisualPosition { row: 0, col: 2 }
        );
        assert_eq!(
            wrapped.cursor_visual_position(combining.len(), 1, 3),
            CursorVisualPosition { row: 0, col: 0 }
        );
    }

    #[test]
    fn cursor_visual_position_never_reaches_full_width() {
        // The render path no longer clamps the cursor column to `input_width`
        // because `cursor_visual_position` can never return a column at or past
        // the content width; a column that would reach the edge is reported on
        // the next (phantom) row instead. This guards that invariant.
        let inputs = [
            "",
            " ",
            "abc",
            "ab cdefgh",
            "hello world",
            "你a好",
            "e\u{301}x",
            "ab\ncd",
            "abcdefghij",
        ];
        for width in 1..=6u16 {
            for input in inputs {
                let wrapped = wrap_prompt_input(input, width);
                for cursor in 0..=input.len() {
                    let pos = wrapped.cursor_visual_position(input.len(), cursor, width);
                    assert!(
                        pos.col < width,
                        "col {} >= width {} for {input:?} at byte {cursor}",
                        pos.col,
                        width
                    );
                }
            }
        }
    }

    #[test]
    fn rendered_cursor_matches_insertion_point() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // (input, cursor byte offset, terminal width)
        let cases: &[(&str, usize, u16)] = &[
            ("ab cdefgh", 7, 9),
            ("abc", 2, 6),
            ("abc", 3, 6),
            ("hello world", 6, 9),
            ("你a好", 3, 6),
            ("你a好", 7, 6),
            ("ab\ncd", 3, 8),
            ("e\u{301}x", 3, 6),
        ];

        with_isolated_prompt(|mut prompt| {
            for &(input, cursor, width) in cases {
                let input_width = prompt_input_width(width);
                let wrapped = wrap_prompt_input(input, input_width);
                let pos = wrapped.cursor_visual_position(input.len(), cursor, input_width);

                // Render layout offsets: content_x = left border (1) + pad_left (1);
                // content_y = pad_top (1). These short inputs never scroll.
                let expected = (2 + pos.col, 1 + u16::try_from(pos.row).unwrap());

                prompt.set_input(input.to_string());
                prompt.cursor_position = cursor;

                let mut terminal = Terminal::new(TestBackend::new(width, 12)).expect("terminal");
                terminal
                    .draw(|frame| prompt.render(frame, frame.size()))
                    .expect("draw");

                assert_eq!(
                    terminal.get_cursor().expect("cursor"),
                    expected,
                    "cursor cell for {input:?} at byte {cursor} width {width}"
                );

                // Non-tautological check: when the cursor sits before a real
                // grapheme, the cell under the cursor must be that grapheme.
                if cursor < input.len() {
                    let grapheme = input[cursor..].graphemes(true).next().unwrap();
                    let cell = terminal.backend().buffer().get(expected.0, expected.1);
                    assert_eq!(
                        cell.symbol(),
                        grapheme,
                        "cursor cell content for {input:?} at byte {cursor} width {width}"
                    );
                }
            }
        });
    }

    #[test]
    fn rendered_cursor_tracks_scroll_for_long_drafts() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let input = "one two three four five six seven eight nine ten";
        let width = 9u16;
        let height = 8u16;
        let cursor = input.len();

        let input_width = prompt_input_width(width);
        let wrapped = wrap_prompt_input(input, input_width);
        let pos = wrapped.cursor_visual_position(input.len(), cursor, input_width);
        // Render content height for this area: height - 3 chrome rows - 2 padding.
        let content_lines = height - 3 - PROMPT_BLOCK_PAD_TOP - PROMPT_BLOCK_PAD_BOTTOM;
        let scroll = pos.row.saturating_sub(usize::from(content_lines - 1));
        let visible_row = pos.row - scroll;
        let expected = (2 + pos.col, 1 + u16::try_from(visible_row).unwrap());

        with_isolated_prompt(|mut prompt| {
            prompt.set_input(input.to_string());
            prompt.cursor_position = cursor;

            let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
            terminal
                .draw(|frame| prompt.render(frame, frame.size()))
                .expect("draw");

            assert!(
                pos.row >= usize::from(content_lines),
                "draft must overflow the visible content height to exercise scroll"
            );
            assert_eq!(terminal.get_cursor().expect("cursor"), expected);
            assert!(
                expected.1 < height,
                "cursor must stay inside the prompt area"
            );
        });
    }
}

fn prompt_state_dir() -> PathBuf {
    let base = std::env::var("OPENCODE_STATE_DIR")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::state_dir().map(|d| d.join("opencode")))
        .unwrap_or_else(|| std::env::temp_dir().join("opencode"));
    let path = base.join("tui");
    let _ = std::fs::create_dir_all(&path);
    path
}

fn load_history(path: &PathBuf) -> Vec<String> {
    let store = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<HistoryStore>(&s).ok())
        .unwrap_or_default();
    let mut entries = store.entries;
    if entries.len() > MAX_HISTORY_ENTRIES {
        let overflow = entries.len() - MAX_HISTORY_ENTRIES;
        entries.drain(0..overflow);
    }
    entries
}

fn load_frecency(path: &PathBuf) -> HashMap<String, FrecencyEntry> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<FrecencyStore>(&s).ok())
        .map(|store| store.entries)
        .unwrap_or_default()
}

fn load_stash(path: &PathBuf) -> Vec<PromptStashEntry> {
    let store = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<StashStore>(&s).ok())
        .unwrap_or_default();
    let mut entries = store.entries;
    if entries.len() > MAX_STASH_ENTRIES {
        let overflow = entries.len() - MAX_STASH_ENTRIES;
        entries.drain(0..overflow);
    }
    entries
}

fn store_history(path: &PathBuf, entries: &[String]) {
    let payload = HistoryStore {
        entries: entries.to_vec(),
    };
    if let Ok(json) = serde_json::to_string(&payload) {
        let _ = std::fs::write(path, json);
    }
}

fn store_frecency(path: &PathBuf, entries: &HashMap<String, FrecencyEntry>) {
    let payload = FrecencyStore {
        entries: entries.clone(),
    };
    if let Ok(json) = serde_json::to_string(&payload) {
        let _ = std::fs::write(path, json);
    }
}

fn store_stash(path: &PathBuf, entries: &[PromptStashEntry]) {
    let payload = StashStore {
        entries: entries.to_vec(),
    };
    if let Ok(json) = serde_json::to_string(&payload) {
        let _ = std::fs::write(path, json);
    }
}

/// Extract a `#line` or `#line-line` range suffix from a file path reference.
/// Returns the base path and an optional (start, optional_end) line range.
fn extract_line_range(input: &str) -> (&str, Option<(usize, Option<usize>)>) {
    if let Some(hash_idx) = input.rfind('#') {
        let base = &input[..hash_idx];
        let range_str = &input[hash_idx + 1..];
        if let Some(dash_idx) = range_str.find('-') {
            let start = range_str[..dash_idx].parse().ok();
            let end = range_str[dash_idx + 1..].parse().ok();
            if let Some(s) = start {
                return (base, Some((s, end)));
            }
        } else if let Ok(line) = range_str.parse::<usize>() {
            return (base, Some((line, None)));
        }
    }
    (input, None)
}
