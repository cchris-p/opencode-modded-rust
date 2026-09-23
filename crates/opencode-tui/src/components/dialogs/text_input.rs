use crate::components::prompt::{
    next_char_boundary, next_word_boundary, prev_char_boundary, prev_word_boundary,
};

/// Small cursor-aware single-line edit buffer for dialog text fields.
///
/// Mirrors the caret behavior of the main prompt (`Backspace`/`Delete` around the caret,
/// `Left`/`Right` by character, `Alt+Left`/`Alt+Right` by word, `Home`/`End` to the boundaries)
/// without pulling in the prompt's multi-line and history machinery.
#[derive(Debug, Default, Clone)]
pub struct DialogTextInput {
    value: String,
    cursor: usize,
}

impl DialogTextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
        }
    }

    pub fn set(&mut self, value: String) {
        self.cursor = value.len();
        self.value = value;
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor.min(self.value.len())
    }

    pub fn split_at_cursor(&self) -> (&str, &str) {
        self.value.split_at(self.cursor())
    }

    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor(), c);
        self.cursor = self.cursor() + c.len_utf8();
    }

    /// Insert `text` at the caret, flattening line breaks and tabs into single
    /// spaces so a pasted multi-line blob becomes a valid single-line value.
    pub fn insert_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let mut normalized = String::with_capacity(text.len());
        let mut pending_cr = false;
        for c in text.chars() {
            match c {
                '\r' => {
                    normalized.push(' ');
                    pending_cr = true;
                }
                '\n' => {
                    if !pending_cr {
                        normalized.push(' ');
                    }
                    pending_cr = false;
                }
                '\t' => {
                    normalized.push(' ');
                    pending_cr = false;
                }
                _ => {
                    normalized.push(c);
                    pending_cr = false;
                }
            }
        }

        let cursor = self.cursor();
        self.value.insert_str(cursor, &normalized);
        self.cursor = cursor + normalized.len();
    }

    pub fn backspace(&mut self) {
        if let Some(prev) = prev_char_boundary(&self.value, self.cursor()) {
            self.value.replace_range(prev..self.cursor(), "");
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if let Some(next) = next_char_boundary(&self.value, self.cursor()) {
            self.value.replace_range(self.cursor()..next, "");
        }
    }

    pub fn move_left(&mut self) {
        if let Some(prev) = prev_char_boundary(&self.value, self.cursor()) {
            self.cursor = prev;
        }
    }

    pub fn move_right(&mut self) {
        if let Some(next) = next_char_boundary(&self.value, self.cursor()) {
            self.cursor = next;
        }
    }

    pub fn move_word_left(&mut self) {
        self.cursor = prev_word_boundary(&self.value, self.cursor());
    }

    pub fn move_word_right(&mut self) {
        self.cursor = next_word_boundary(&self.value, self.cursor());
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_honors_caret() {
        let mut input = DialogTextInput::new();
        input.set("abcd".to_string());
        input.move_home();
        input.insert_char('X');
        assert_eq!(input.value(), "Xabcd");
        assert_eq!(input.split_at_cursor(), ("X", "abcd"));
    }

    #[test]
    fn backspace_and_delete_around_caret() {
        let mut input = DialogTextInput::new();
        input.set("abcd".to_string());
        input.move_left();
        input.move_left();
        input.backspace();
        assert_eq!(input.value(), "acd");
        input.delete();
        assert_eq!(input.value(), "ad");
    }

    #[test]
    fn caret_movement_by_character() {
        let mut input = DialogTextInput::new();
        input.set("abc".to_string());
        input.move_home();
        assert_eq!(input.cursor(), 0);
        input.move_right();
        assert_eq!(input.cursor(), 1);
        input.move_end();
        assert_eq!(input.cursor(), 3);
        input.move_left();
        assert_eq!(input.cursor(), 2);
    }

    #[test]
    fn word_movement_skips_words_and_punctuation() {
        let mut input = DialogTextInput::new();
        input.set("alpha beta-gamma".to_string());
        input.move_home();
        input.move_word_right();
        assert_eq!(input.split_at_cursor(), ("alpha", " beta-gamma"));
        input.move_word_right();
        assert_eq!(input.split_at_cursor(), ("alpha beta", "-gamma"));
        input.move_word_left();
        assert_eq!(input.split_at_cursor(), ("alpha ", "beta-gamma"));
        input.move_word_left();
        assert_eq!(input.split_at_cursor(), ("", "alpha beta-gamma"));
    }

    #[test]
    fn insert_str_flattens_newlines_and_tabs() {
        let mut input = DialogTextInput::new();
        input.insert_str("line one\r\nline two\nline three\ttail");
        assert_eq!(input.value(), "line one line two line three tail");
        assert_eq!(input.cursor(), input.value().len());
    }

    #[test]
    fn insert_str_honors_caret_and_multibyte() {
        let mut input = DialogTextInput::new();
        input.set("你a好".to_string());
        input.move_home();
        input.move_right();
        input.insert_str("X\r\nY");
        assert_eq!(input.value(), "你X Ya好");
        assert_eq!(input.cursor(), "你X Y".len());
    }

    #[test]
    fn multibyte_editing_is_boundary_safe() {
        let mut input = DialogTextInput::new();
        input.set("你好世界".to_string());
        input.move_end();
        input.backspace();
        assert_eq!(input.value(), "你好世");
        input.move_home();
        input.insert_char('A');
        assert_eq!(input.value(), "A你好世");
        input.move_right();
        input.delete();
        assert_eq!(input.value(), "A你世");
    }
}
