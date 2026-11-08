# Plan: Fix BUG-018 prompt input cursor out of place

## Target board item

- Path: `boards/qa/prompt-input-cursor-renders-out-of-place.md`
- ID: `BUG-018` (P1, bug, area BUG, status `qa`)
- Related: `BUG-013` (cursor always visible, done), `PHASE-001` V1 daily-driver hardening
- Current lane: `qa`, intentionally held for observation after PR #47
- This plan reopens it because the misalignment is still observed while typing.

## Requested outcome

Reopen `BUG-018`, fix the still-observed cursor misalignment in the prompt input, verify it in a
real terminal and with unit tests, and return the item to `qa` with evidence.

## Confirmed current state

- Default branch is `development`; `BUG-018` fix commit `36ee852` ("fix(tui): align prompt cursor at
  line edge") is merged via PR #47.
- That fix changed only one line: cursor `max_col` from `input_width - 1` to `input_width`.
- All prompt cursor placement lives in `crates/opencode-tui/src/components/prompt.rs`:
  - `Prompt::render` builds `cursor_visual_position` at line 234 and calls `frame.set_cursor` at
    line 325.
  - `input_cursor_visual_position` (line 1163) computes row/col with a **greedy per-`char` wrap**.
  - `visual_line_count` (line 1217) computes input height with the same greedy per-`char` wrap.
  - `input_display_lines` (line 1022) uses `visual_line_count` for the box height.
  - `prompt_input_width` (line 1156) returns `area.width - 3` (1 left border + pad 1 + pad 1).

## Root cause (evidence-backed)

Rendering and cursor math use two different wrap algorithms:

1. Rendering: `Paragraph::new(self.input).wrap(Wrap { trim: false })` (line 291/306).
   ratatui 0.27 wraps with `WordWrapper` in the private `ratatui-0.27.0/src/widgets/reflow.rs`:
   - iterates **grapheme clusters** (`unicode_segmentation::UnicodeSegmentation::graphemes`),
   - wraps on **word boundaries** with whitespace handling (not greedy per column),
   - **ignores graphemes wider than the line width** (`if symbol_width > self.max_line_width { continue; }`).
2. Cursor/height: `input_cursor_visual_position` and `visual_line_count`:
   - iterate **`char`s** with `UnicodeWidthChar::width`,
   - do greedy column fill (`if col > 0 && col + ch_width > width { row += 1; col = 0 }`).

This produces exactly the reported intermittent offsets:

- Word vs char wrap: for text with spaces, ratatui moves a whole word to the next visual row while the
  cursor math keeps it on the current row, so the cursor is drawn on the wrong row (appears ahead/behind
  depending on where the mismatch lands).
- Char vs grapheme: combining marks, ZWJ emoji, and flags have different per-char vs grapheme width, so
  `col` drifts by one or more cells.
- Wide chars wider than the line width are dropped by ratatui but counted by the cursor math.
- The `col > 0` guard lets a wide char that exceeds width sit at `col 0`, producing `col > width`.

The PR #47 band-aid is a second, independent defect:

- `max_col = input_width` makes the cursor target `content_x + input_width`, which is the right padding
  cell (`content_x = frame_x + 1 border + 1 pad_left`), i.e. one cell past the content area. At an
  exact full-line end this shows the cursor "one character ahead".
- The previous `input_width - 1` clamp showed it "one behind". Neither is correct: at a wrap boundary
  the cursor belongs at the start of the next visual row, not on the pad cell and not on the last char.

`input_display_lines` inherits the same greedy wrap, so the box height and scroll offset can also
disagree with what ratatui actually renders.

## Fix approach

### Recommended: derive cursor geometry from the same renderer (no second wrap algorithm)

Use ratatui itself as the single source of truth by rendering offscreen into a scratch `Buffer` with an
invisible one-cell sentinel, then reading the sentinel cell. This makes cursor geometry exactly match
`Wrap { trim: false }` for graphemes, word boundaries, and wide chars.

- Add a private helper in `prompt.rs`, e.g.:
  `fn prompt_cursor_geometry(input: &str, cursor_position: usize, width: u16) -> (u16 /*row*/, u16 /*col*/, u16 /*line_count*/)`.
- Implementation sketch:
  - `const CURSOR_SENTINEL: char = '\u{E000}';` (private-use, width 1, not expected in prompts).
  - Build `prefix = &input[..floor_char_boundary(cursor_position)]`.
  - Render `Paragraph::new(prefix + sentinel).wrap(Wrap { trim: false })` into
    `Buffer::empty(Rect { x: 0, y: 0, width, height: PROMPT_MAX_INPUT_LINES + 2 })` using
    `Widget::render(area, &mut buf)` (import `ratatui::buffer::Buffer` and `ratatui::widgets::Widget`).
  - Scan `buf.content` for `CURSOR_SENTINEL`; its `(x, y)` is the cursor `(col, row)`.
  - Render `input + sentinel` the same way into a tall buffer and use the sentinel row + 1 as total
    wrapped line count (replaces `visual_line_count` for the prompt).
  - Fall back to the old greedy computation only if the sentinel is not found (defensive).
- Rewire `Prompt::render`:
  - `let (cursor_row_abs, cursor_col, line_count) = prompt_cursor_geometry(&self.input, self.cursor_position, input_width);`
  - `content_lines = line_count.clamp(PROMPT_MIN_INPUT_LINES, PROMPT_MAX_INPUT_LINES)` (and still bounded
    by `max_content_lines` from `area.height`).
  - Include the cursor row when sizing so a cursor at an exact full-line boundary gets its phantom next
    row: `content_lines = content_lines.max(cursor_row_abs + 1)` before clamping.
  - `input_scroll = cursor_row_abs.saturating_sub(content_lines - 1)`; leave `Paragraph.scroll` as is.
  - Remove the `max_col` band-aid; set cursor `x = content_x + cursor_col`, `y = content_y + (cursor_row_abs - input_scroll)`.
  - Keep `.wrap(Wrap { trim: false })` on the visible paragraph so visuals are unchanged.
- Keep `visual_line_count` for other callers if any, otherwise remove it and update tests.

Why this over re-implementing the wrapper: `ratatui::widgets::reflow` is private in 0.27, so
`WordWrapper` cannot be reused directly. The scratch-buffer trick reuses the real renderer with no
duplicated wrap logic and no drift if wrapping behavior changes. Cost is one small offscreen buffer
render per frame, which is negligible for a prompt.

### Fallback (only if the offscreen render proves unreliable)

Port the ratatui 0.27 `WordWrapper` semantics (grapheme + word boundary + `trim=false`) into a local
`wrap_prompt_input(input, width) -> Vec<String>` and use it for both the visible `Line`s (drop
`Wrap` on the visible paragraph) and cursor/height. This removes drift by construction but duplicates
upstream behavior and changes the rendering path.

### Boundary semantics to settle in tests

- Cursor between two graphemes that straddle a wrap boundary (e.g. `"abcdef"`, width 3, cursor 3)
  must be row 1, col 0.
- Cursor at end of input that exactly fills the last line (e.g. `"abc"`, width 3, cursor 3) must be
  row 1, col 0 via the phantom row, not col 3 on the pad cell and not col 2.
- Confirm the phantom row does not flicker: typing the next char keeps the same box height.
- If the box is already at `PROMPT_MAX_INPUT_LINES`, document the clamp behavior explicitly.

### Known-limitation note to record

`visual_line_count`/cursor math currently treats each `char` as one cell. The chosen fix makes the
cursor follow ratatui, but if any other prompt-height logic still uses the greedy wrap it must be
migrated in the same change so height and cursor cannot disagree.

## Files expected to change

- `crates/opencode-tui/src/components/prompt.rs` (cursor geometry, height, `render`, tests).
- `crates/opencode-tui/Cargo.toml` only if the fallback port is chosen (`unicode-segmentation = "1"`,
  already present transitively via ratatui 0.27).
- `boards/qa/prompt-input-cursor-renders-out-of-place.md` (Dev Notes + lane, per workflow).

## Implementation steps

1. Reproduce first: run `ort-build` then `ort`; type text with spaces that wraps, paste multi-line
   text, and type CJK/emoji; note exact ahead/behind offsets. Capture terminal width and the exact
   input string for each repro so a regression test can encode it.
2. Move `BUG-018` from `boards/qa/` to `boards/doing/` and set `status: "doing"` (keep markdown
   structure/frontmatter). Follow the `board-item-to-branch` skill.
3. Branch from local `development` (`git pull --ff-only` on `development` first): create
   `bug/BUG-018-prompt-cursor-position`.
4. Implement `prompt_cursor_geometry` in `prompt.rs`; rewire `Prompt::render`; remove the `max_col`
   band-aid; migrate height to the same geometry.
5. Add tests (see below).
6. Run focused tests plus fmt/clippy/check/build.
7. `ort-build` then `ort`; re-run the repro corpus from step 1 and confirm exact alignment; resize the
   terminal narrow and wide; confirm after streaming output and after overlays.
8. Update the board item Dev Notes with branch, what changed, decisions, and verification results;
   move it to `boards/qa/` with `status: "qa"` and leave the branch checked out for human verification.
9. Commit only BUG-018-related files with a concise `fix(tui): ...` message aligned to repo
   conventions. Do not create a PR unless asked.

## Tests to add

- Extend/replace `cursor_visual_position_follows_wrapped_input` and
  `cursor_visual_position_handles_newlines_and_wide_chars` to cover:
  - word-boundary wrap with spaces (char-greedy vs ratatui divergence case),
  - grapheme clusters (ZWJ emoji, flag, combining accent),
  - CJK wide chars at line edges,
  - exact full-line boundary (cursor after last visible cell),
  - multi-line draft with `\n`.
- Add a `TestBackend` render test that renders the prompt and asserts the terminal cursor equals the
  expected cell for the same corpus (this is the strongest guard because it uses the real widget).
- Keep `utf8_backspace_delete_and_cursor_are_char_safe` green.
- Note existing unrelated failures observed during BUG-018 (prompt autocomplete isolation
  `tab_autocomplete_uses_first_candidate` returning `test` instead of `team`, poisoned env lock) and
  the pre-existing fmt drift in `crates/opencode-provider/src/anthropic.rs` and
  `crates/opencode-tui/src/context/keybind.rs`; do not fix these opportunistically.

## Verification commands

- `cargo fmt --check -p opencode-tui` (expect pre-existing drift elsewhere; confirm no new drift)
- `cargo test -p opencode-tui cursor_visual_position -- --test-threads=1`
- `cargo test -p opencode-tui -- --test-threads=1` (triage existing failures vs new)
- `cargo check -p opencode-tui`
- `cargo clippy -p opencode-tui --all-targets`
- `cargo build -p opencode-tui`
- `ort-build` then `ort` for real-terminal QA (required; earlier attempts could not run in
  non-interactive shells, so this must be done from an interactive shell or recorded as blocked)

## Acceptance criteria (from board item)

- Cursor drawn exactly at the insertion point whenever the prompt input is focused.
- Alignment holds after typing, deleting, pasting, Home/End and word movement.
- Alignment holds across wrapped lines, multi-line drafts, wide/zero-width Unicode, narrow and wide
  terminals.
- Alignment holds after transcript changes and terminal resize.
- No constant ahead/behind offset in any case above.

## Risks / rollback

- Offscreen render changes cursor geometry only; visible wrap stays `Wrap { trim: false }`, so visual
  regression risk is low. Rollback is reverting the single `prompt.rs` change.
- If the sentinel scan fails on some backend, the defensive fallback keeps today's behavior rather than
  panicking.
- Perf: one extra scratch buffer per frame; negligible, but confirm no visible typing latency.

## Open questions

- None blocking. Board workflow to follow is `board-item-to-branch` (committed branch, no PR) unless a
  PR is requested.
