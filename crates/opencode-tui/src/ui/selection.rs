use unicode_width::UnicodeWidthChar;

/// Number of rows from the top/bottom transcript edge that trigger auto-scroll.
pub const AUTO_SCROLL_EDGE_BAND: u16 = 3;

/// Terminal text selection — tracks a rectangular region in screen coordinates
/// and provides hit-testing + text extraction.
///
/// Selection follows standard terminal behavior:
/// - First row: from start column to end of line
/// - Middle rows: entire line
/// - Last row: from start of line to end column
/// - Single row: from start column to end column

pub struct Selection {
    /// Anchor point (where mouse-down happened).
    anchor: Option<(u16, u16)>,
    /// Current drag endpoint.
    cursor: Option<(u16, u16)>,
    /// True while the mouse button is held down.
    dragging: bool,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            anchor: None,
            cursor: None,
            dragging: false,
        }
    }

    /// Begin a new selection at (row, col).
    pub fn start(&mut self, row: u16, col: u16) {
        self.anchor = Some((row, col));
        self.cursor = Some((row, col));
        self.dragging = true;
    }

    /// Update the drag endpoint.
    pub fn update(&mut self, row: u16, col: u16) {
        if self.dragging {
            self.cursor = Some((row, col));
        }
    }

    /// Mouse button released — keep the selection visible but stop tracking.
    pub fn finalize(&mut self) {
        self.dragging = false;
    }

    /// Dismiss the selection entirely.
    pub fn clear(&mut self) {
        self.anchor = None;
        self.cursor = None;
        self.dragging = false;
    }

    /// True if there is a visible selection (dragging or finalized).
    pub fn is_active(&self) -> bool {
        self.anchor.is_some() && self.cursor.is_some()
    }

    /// True if the user is currently dragging.
    pub fn is_selecting(&self) -> bool {
        self.dragging
    }

    /// Returns the normalized range: (top-left, bottom-right) in reading order.
    fn range(&self) -> Option<((u16, u16), (u16, u16))> {
        match (self.anchor, self.cursor) {
            (Some(a), Some(b)) => {
                if a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) {
                    Some((a, b))
                } else {
                    Some((b, a))
                }
            }
            _ => None,
        }
    }

    /// Test whether a specific cell is inside the selection.
    pub fn is_selected(&self, row: u16, col: u16) -> bool {
        let ((r0, c0), (r1, c1)) = match self.range() {
            Some(r) => r,
            None => return false,
        };

        if row < r0 || row > r1 {
            return false;
        }

        if r0 == r1 {
            // Single-line selection
            return col >= c0 && col <= c1;
        }

        if row == r0 {
            // First row: from start col to end of line
            return col >= c0;
        }

        if row == r1 {
            // Last row: from start of line to end col
            return col <= c1;
        }

        // Middle rows: entire line
        true
    }

    /// Extract the selected text using a callback that returns the full line
    /// content for a given row number.
    pub fn get_selected_text<F>(&self, get_line: F) -> String
    where
        F: Fn(u16) -> Option<String>,
    {
        let ((r0, c0), (r1, c1)) = match self.range() {
            Some(r) => r,
            None => return String::new(),
        };

        let mut result = String::new();

        for row in r0..=r1 {
            let line = match get_line(row) {
                Some(l) => l,
                None => continue,
            };

            let selected = if r0 == r1 {
                // Single line: clip both ends
                slice_by_columns(&line, c0 as usize, c1.saturating_add(1) as usize)
            } else if row == r0 {
                // First row: from col to end
                slice_from_column(&line, c0 as usize)
            } else if row == r1 {
                // Last row: from start to col
                slice_to_column(&line, c1.saturating_add(1) as usize)
            } else {
                // Middle row: full line
                line
            };

            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(selected.trim_end());
        }

        result
    }
}

fn slice_by_columns(line: &str, start_col: usize, end_col_exclusive: usize) -> String {
    if start_col >= end_col_exclusive {
        return String::new();
    }
    let start = byte_index_for_column_start(line, start_col);
    let end = byte_index_for_column_end(line, end_col_exclusive);
    if start >= end || start >= line.len() {
        return String::new();
    }
    line[start..end].to_string()
}

fn slice_from_column(line: &str, start_col: usize) -> String {
    let start = byte_index_for_column_start(line, start_col);
    if start >= line.len() {
        return String::new();
    }
    line[start..].to_string()
}

fn slice_to_column(line: &str, end_col_exclusive: usize) -> String {
    let end = byte_index_for_column_end(line, end_col_exclusive);
    if end == 0 {
        return String::new();
    }
    line[..end].to_string()
}

fn byte_index_for_column_start(line: &str, target_col: usize) -> usize {
    if target_col == 0 {
        return 0;
    }

    let mut col = 0usize;
    for (idx, ch) in line.char_indices() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if target_col <= col {
            return idx;
        }
        if target_col < col + width {
            // Selection starts inside a glyph cell: snap to glyph start.
            return idx;
        }
        col += width;
    }
    line.len()
}

fn byte_index_for_column_end(line: &str, target_col: usize) -> usize {
    if target_col == 0 {
        return 0;
    }

    let mut col = 0usize;
    for (idx, ch) in line.char_indices() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        let end_idx = idx + ch.len_utf8();
        if target_col <= col {
            return idx;
        }
        if target_col < col + width {
            // Selection ends inside a glyph cell: include the whole glyph.
            return end_idx;
        }
        if target_col == col + width {
            return end_idx;
        }
        col += width;
    }
    line.len()
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

/// A selection anchored in transcript content-line coordinates rather than
/// screen coordinates. Content line index 0 is the first rendered transcript
/// line, so the anchor stays on the original line while the transcript scrolls
/// underneath it.
#[derive(Clone, Copy, Debug)]
pub struct ContentSelection {
    anchor_line: usize,
    anchor_col: u16,
    cursor_line: usize,
    cursor_col: u16,
    scrolled: bool,
}

impl ContentSelection {
    pub fn new(anchor_line: usize, anchor_col: u16) -> Self {
        Self {
            anchor_line,
            anchor_col,
            cursor_line: anchor_line,
            cursor_col: anchor_col,
            scrolled: false,
        }
    }

    /// Update the drag endpoint. Content lines are absolute transcript rows.
    pub fn update_cursor(&mut self, line: usize, col: u16) {
        self.cursor_line = line;
        self.cursor_col = col;
    }

    /// Record that auto-scroll has moved the transcript since the drag began.
    pub fn mark_scrolled(&mut self) {
        self.scrolled = true;
    }

    pub fn scrolled(&self) -> bool {
        self.scrolled
    }

    pub fn anchor_line(&self) -> usize {
        self.anchor_line
    }

    pub fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    /// Inclusive content-line range spanned by the selection, in reading order.
    pub fn content_range(&self) -> (usize, usize) {
        if self.anchor_line <= self.cursor_line {
            (self.anchor_line, self.cursor_line)
        } else {
            (self.cursor_line, self.anchor_line)
        }
    }

    fn range(&self) -> ((usize, u16), (usize, u16)) {
        let a = (self.anchor_line, self.anchor_col);
        let b = (self.cursor_line, self.cursor_col);
        if a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) {
            (a, b)
        } else {
            (b, a)
        }
    }

    /// Test whether a content cell is inside the selection.
    pub fn is_selected(&self, line: usize, col: u16) -> bool {
        let ((r0, c0), (r1, c1)) = self.range();

        if line < r0 || line > r1 {
            return false;
        }

        if r0 == r1 {
            return col >= c0 && col <= c1;
        }

        if line == r0 {
            return col >= c0;
        }

        if line == r1 {
            return col <= c1;
        }

        true
    }

    /// Extract the selected text using a callback that returns the rendered
    /// line for an absolute transcript line index.
    pub fn get_selected_text<F>(&self, get_line: F) -> String
    where
        F: Fn(usize) -> Option<String>,
    {
        let ((r0, c0), (r1, c1)) = self.range();
        let mut result = String::new();

        for row in r0..=r1 {
            let line = match get_line(row) {
                Some(l) => l,
                None => continue,
            };

            let selected = if r0 == r1 {
                slice_by_columns(&line, c0 as usize, c1.saturating_add(1) as usize)
            } else if row == r0 {
                slice_from_column(&line, c0 as usize)
            } else if row == r1 {
                slice_to_column(&line, c1.saturating_add(1) as usize)
            } else {
                line
            };

            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(selected.trim_end());
        }

        result
    }
}

/// Drives edge auto-scroll while a selection drag is held near the transcript
/// top or bottom. Uses the vanilla opentui speed bands and a fractional
/// lines-per-second accumulator so the rate is time based, not per-event.
#[derive(Clone, Copy, Debug, Default)]
pub struct AutoScroll {
    direction: i8,
    lines_per_second: u32,
    accumulator: f32,
}

impl AutoScroll {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.direction != 0
    }

    pub fn direction(&self) -> i8 {
        self.direction
    }

    pub fn lines_per_second(&self) -> u32 {
        self.lines_per_second
    }

    pub fn stop(&mut self) {
        self.direction = 0;
        self.lines_per_second = 0;
        self.accumulator = 0.0;
    }

    /// Recompute the edge band from the pointer row. Returns true when the
    /// pointer is inside the scrollable area and within an edge band.
    pub fn update_pointer(&mut self, row: u16, area_top: u16, area_height: u16) -> bool {
        if area_height == 0 {
            self.stop();
            return false;
        }

        let bottom = area_top.saturating_add(area_height - 1);
        if row < area_top || row > bottom {
            self.stop();
            return false;
        }

        let dist_top = row - area_top;
        let dist_bottom = bottom - row;
        let (direction, distance) = if dist_top <= dist_bottom {
            (-1i8, dist_top)
        } else {
            (1i8, dist_bottom)
        };

        let speed = match distance {
            0 | 1 => 72,
            2 => 36,
            AUTO_SCROLL_EDGE_BAND => 6,
            _ => {
                self.stop();
                return false;
            }
        };

        if self.direction != direction {
            self.accumulator = 0.0;
        }
        self.direction = direction;
        self.lines_per_second = speed;
        true
    }

    /// Advance the accumulator by `elapsed_ms` and return the direction plus
    /// the whole number of lines that should be scrolled this step.
    pub fn step(&mut self, elapsed_ms: u64) -> (i8, usize) {
        if self.direction == 0 || self.lines_per_second == 0 {
            return (0, 0);
        }

        self.accumulator += self.lines_per_second as f32 * elapsed_ms as f32 / 1000.0;
        let whole = self.accumulator.floor();
        if whole < 1.0 {
            return (self.direction, 0);
        }
        let lines = whole as usize;
        self.accumulator -= whole;
        (self.direction, lines)
    }
}

/// Compute the next transcript scroll offset for one auto-scroll step.
/// Returns `None` when the transcript is already at the requested limit.
pub fn next_scroll_offset(
    direction: i8,
    lines: usize,
    offset: usize,
    max_offset: usize,
) -> Option<usize> {
    match direction {
        d if d < 0 => {
            if offset == 0 {
                None
            } else {
                Some(offset.saturating_sub(lines))
            }
        }
        d if d > 0 => {
            if offset >= max_offset {
                None
            } else {
                Some(offset.saturating_add(lines).min(max_offset))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{next_scroll_offset, AutoScroll, ContentSelection, Selection};

    #[test]
    fn utf8_glyph_boundary_selection_is_safe() {
        let line = "  ││ Provider error";
        let mut selection = Selection::new();
        selection.start(0, 2);
        selection.update(0, 4);
        selection.finalize();

        let selected = selection.get_selected_text(|row| {
            if row == 0 {
                Some(line.to_string())
            } else {
                None
            }
        });

        assert_eq!(selected, "││");
    }

    #[test]
    fn unicode_wide_char_selection_is_safe() {
        let line = "A你B";
        let mut selection = Selection::new();
        // Select only the wide glyph cell range.
        selection.start(0, 1);
        selection.update(0, 2);
        selection.finalize();

        let selected = selection.get_selected_text(|row| {
            if row == 0 {
                Some(line.to_string())
            } else {
                None
            }
        });

        assert_eq!(selected, "你");
    }

    fn line_map(range: std::ops::RangeInclusive<usize>) -> impl Fn(usize) -> Option<String> {
        let start = *range.start();
        let lines: Vec<String> = range.map(|idx| format!("line {idx}")).collect();
        move |row| lines.get(row.saturating_sub(start)).cloned()
    }

    #[test]
    fn content_selection_spans_multiple_viewports() {
        // Anchor on content line 5, then scroll so the pointer ends on line 20.
        let mut selection = ContentSelection::new(5, 0);
        selection.update_cursor(10, 4);
        selection.mark_scrolled();
        selection.update_cursor(20, 6);
        selection.mark_scrolled();

        assert!(selection.scrolled());
        assert_eq!(selection.content_range(), (5, 20));

        let text = selection.get_selected_text(line_map(0..=25));
        let expected = (5..=20)
            .map(|idx| format!("line {idx}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(text, expected);
    }

    #[test]
    fn content_selection_anchor_stays_put_as_content_scrolls_up() {
        // Dragging upward from line 20 to line 8 keeps the anchor at 20.
        let mut selection = ContentSelection::new(20, 0);
        selection.update_cursor(12, 0);
        selection.mark_scrolled();
        selection.update_cursor(8, 3);

        assert_eq!(selection.anchor_line(), 20);
        assert_eq!(selection.cursor_line(), 8);
        assert_eq!(selection.content_range(), (8, 20));
        assert!(selection.is_selected(10, 0));
        assert!(!selection.is_selected(21, 0));
    }

    #[test]
    fn auto_scroll_uses_edge_speed_bands() {
        let mut scroll = AutoScroll::new();

        assert!(scroll.update_pointer(3, 2, 20));
        assert_eq!(scroll.direction(), -1);
        assert_eq!(scroll.lines_per_second(), 72);

        assert!(scroll.update_pointer(4, 2, 20));
        assert_eq!(scroll.lines_per_second(), 36);

        assert!(scroll.update_pointer(5, 2, 20));
        assert_eq!(scroll.lines_per_second(), 6);

        // Symmetric at the bottom edge (area rows 2..=21).
        assert!(scroll.update_pointer(21, 2, 20));
        assert_eq!(scroll.direction(), 1);
        assert_eq!(scroll.lines_per_second(), 72);
        assert!(scroll.update_pointer(19, 2, 20));
        assert_eq!(scroll.lines_per_second(), 36);
        assert!(scroll.update_pointer(18, 2, 20));
        assert_eq!(scroll.lines_per_second(), 6);

        // Interior and outside rows stop auto-scroll.
        assert!(!scroll.update_pointer(11, 2, 20));
        assert!(!scroll.is_active());
        assert!(!scroll.update_pointer(1, 2, 20));
        assert!(!scroll.is_active());
        assert!(!scroll.update_pointer(25, 2, 20));
        assert!(!scroll.is_active());
    }

    #[test]
    fn auto_scroll_step_accumulates_fractionally() {
        let mut scroll = AutoScroll::new();
        scroll.update_pointer(3, 2, 20); // fast: 72 lines/s

        // One 16ms frame at 72 lines/s is ~1.15 lines.
        assert_eq!(scroll.step(16), (-1, 1));
        // Remaining fraction plus another frame yields one more line.
        assert_eq!(scroll.step(16), (-1, 1));
    }

    #[test]
    fn auto_scroll_step_is_zero_below_one_line() {
        let mut scroll = AutoScroll::new();
        scroll.update_pointer(5, 2, 20); // slow: 6 lines/s

        assert_eq!(scroll.step(16), (-1, 0));
        assert_eq!(scroll.step(16), (-1, 0));
    }

    #[test]
    fn next_scroll_offset_clamps_at_limits() {
        // Up at the top limit stops.
        assert_eq!(next_scroll_offset(-1, 3, 0, 100), None);
        // Down at the bottom limit stops.
        assert_eq!(next_scroll_offset(1, 3, 100, 100), None);
        // Up clamps to zero.
        assert_eq!(next_scroll_offset(-1, 10, 4, 100), Some(0));
        // Down clamps to the max offset.
        assert_eq!(next_scroll_offset(1, 10, 96, 100), Some(100));
        // No direction makes no change.
        assert_eq!(next_scroll_offset(0, 5, 50, 100), None);
    }
}
