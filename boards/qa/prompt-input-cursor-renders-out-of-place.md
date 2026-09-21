---
id: "BUG-018"
title: "Prompt input cursor renders out of place in the TUI"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-09-19"
---

# Prompt input cursor renders out of place in the TUI

## Summary

The terminal cursor on the prompt input box is sometimes drawn at the wrong character position. The rendered cursor can appear one character ahead of the true insertion point, or up to roughly two characters behind it. The cursor is visible (see `BUG-013`), but its position cannot be trusted, which makes text entry and editing unreliable.

## Reported behavior

- While typing or editing a prompt in the TUI, the visible cursor is not always aligned with where the next character will be inserted.
- Observed offsets vary:
  - Sometimes the cursor renders one character ahead of the real insertion point.
  - Sometimes it renders about two characters behind the real insertion point.
- The mismatch is intermittent and appears during normal typing/editing, not only at wrap boundaries.

## Why this exists

The prompt input is the primary daily-driver interaction surface. If the cursor is visible but misaligned, the user cannot tell where input will land, so editing requires guesswork and errors are easy to make.

## Hypotheses to investigate

- Width/column math for the cursor position disagrees with the width math used to render the text (for example, inconsistent handling of grapheme clusters, combining marks, or east-asian/wide characters).
- Prompt padding, prompt prefix (`> `), or border/scroll offset is applied when rendering text but not when computing the cursor column (or vice versa), producing a constant offset.
- The cursor column is computed from a byte index or a character index instead of display width in one of the code paths.
- Scrolling of a wrapped/multi-line draft causes the cursor row/column to be computed against a stale scroll offset.
- Cursor placement is computed before the paragraph layout is finalized, so it uses the previous frame's layout.

## Scope

- Align the rendered cursor position with the actual insertion point for the prompt input.
- Handle multiple characters, grapheme clusters, wide/zero-width Unicode, wrapped lines, and multi-line drafts consistently between rendering and cursor placement.
- Preserve existing prompt editing behavior, keybindings, selection behavior, and layout.
- Ensure correct behavior in both narrow and wide terminal sizes.

## Non-goals

- Redesigning the prompt composer UI.
- Changing session transcript scrolling semantics except where necessary for correct cursor placement.
- Changing keyboard shortcuts unrelated to prompt editing or focus.

## Done when

- The cursor is drawn exactly at the insertion point whenever the prompt input is focused.
- The cursor stays aligned after typing, deleting, pasting, navigating with normal editing keys, and moving by word.
- Alignment holds across wrapped lines, multi-line drafts, wide/zero-width Unicode input, and both narrow and wide terminals.
- Alignment holds after the transcript changes (assistant text, tool output, status lines) and after terminal resize.
- No constant ahead/behind offset is observable in any of the above cases.

## Recommended verification

- Run `ort-build`, then launch `ort` in a real terminal.
- Type prompts containing ASCII, emoji, accented/combining characters, and CJK text; confirm the cursor stays at the insertion point.
- Type a prompt long enough to wrap, then navigate within it with arrow keys, Home/End, and word movement; confirm alignment after each move.
- Paste long and multi-line text and confirm the cursor lands at the true insertion point.
- Delete characters at the start, middle, and end of the draft and confirm alignment.
- Resize the terminal smaller and larger while the prompt has text and confirm alignment is preserved.
- Send a prompt, wait for streaming output, then type again and confirm alignment throughout.
- Add/extend a `cursor_visual_position`-style unit test to cover the offset cases above.

## Related Items

- `BUG-013` Cursor on the input field needs to always be visible - established that the cursor must be drawn; this item covers its correctness when drawn.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input key handling surface.
- `PHASE-001` V1 daily-driver hardening - misaligned cursor blocks trust in the primary input loop.
- `FEAT-034` Render prompt cursor as a software cursor - archived fallback strategy only; do not pursue before isolating the current off-by-one behavior.

## Dev Notes

- 2026-09-19: Fixed cursor placement at the prompt content-width boundary. The visual cursor column can legitimately equal `input_width` when the insertion point is immediately after the last visible cell on a full line; rendering now allows that column instead of clamping it back to `input_width - 1`.
- Added/extended cursor visual-position coverage for the exact full-width boundary case.
- 2026-09-21: QA reported the cursor still sometimes feels off by one character. Primary focus remains proving and fixing the concrete off-by-one in the current cursor/layout implementation, not replacing the cursor strategy preemptively.
- 2026-09-21: Created archived follow-up `FEAT-034` for the software-cursor fallback. That option should only be revived if the remaining behavior proves structural after direct off-by-one investigation.
- 2026-09-21: Reopened. Confirmed root cause: the cursor/height math and the rendered paragraph used two different wrapping algorithms. Rendering used ratatui's `Wrap { trim: false }` (grapheme-aware, word-boundary wrapper in `ratatui-0.27/src/widgets/reflow.rs`), while `input_cursor_visual_position`/`visual_line_count` used a greedy per-`char` column fill. Spaces, grapheme clusters, and wide characters made the two layouts disagree, producing the intermittent one-ahead/two-behind offsets. The PR #47 `max_col = input_width` change also drew the cursor on the right pad cell at an exact full-line edge.
- 2026-09-21: Replaced the parallel wrap math with a single `wrap_prompt_input` (grapheme-aware, word-boundary, `trim=false`-style) that produces the visual lines and records the (line, column) of every grapheme. `Prompt::render` now feeds those pre-wrapped `Line`s to `Paragraph` (no `Wrap`) and derives cursor row/column, box height, and scroll from the same structure, so cursor and text can no longer disagree. A cursor at an exact full-line boundary lands on the next (phantom) row at column 0. Removed the `max_col` band-aid plus the old `input_cursor_visual_position`/`visual_line_count`.
- 2026-09-21: Added `unicode-segmentation = "1"` to `crates/opencode-tui/Cargo.toml` for grapheme iteration (already present transitively via ratatui 0.27).
- 2026-09-21: `desired_height`/`prompt_content_lines` now include the cursor's phantom row so the layout reserves that row before the next character wraps.
- 2026-09-21: Final cleanup. Removed the vestigial `cursor_col.min(input_width)` clamp in `Prompt::render` that survived Fix B. It could never trigger: `WrappedPromptInput::cursor_visual_position` only returns a column strictly less than the content width (a column that would reach the edge is reported as `row + 1, col 0`), and `prompt_input_width` is always `>= 1`. The clamp was leftover from the PR #47 band-aid, not a live guard, so removing it is behavior-preserving.
- 2026-09-21: Replaced the tautological render test (which re-rendered `wrap_prompt_input`'s own lines through a bare `Paragraph` and therefore could not catch a padding/offset/scroll error in the real widget) with tests that drive the actual `Prompt::render` path through a `TestBackend` and assert the set-cursor cell: `rendered_cursor_matches_insertion_point` (ASCII, space-wrap, CJK, combining cluster, newline, exact-full-line; also asserts the cell under the cursor is the next grapheme), `rendered_cursor_tracks_scroll_for_long_drafts` (scrolled overflow draft), and `cursor_visual_position_never_reaches_full_width` (guards the invariant that made the clamp removable).
- 2026-09-21: Known residual, intentionally out of scope: tabs and other control characters pasted into the prompt compute as display width 0 (`prompt_grapheme_width` via `unicode-width`), so a real terminal that expands a tab to the next tab stop can still show a mismatch. This is a pre-existing ratatui/terminal surface issue, not the wrap-vs-cursor split this item fixed; noted here for a follow-up if tabbed paste is ever reported.

## Verification

- `cargo test -p opencode-tui cursor_visual_position` passed.
- `cargo test -p opencode-tui components::prompt::tests::utf8_backspace_delete_and_cursor_are_char_safe -- --exact` passed.
- `cargo test -p opencode-tui` failed on existing prompt autocomplete isolation: `components::prompt::tests::tab_autocomplete_uses_first_candidate` returns `test` instead of `team`; the following env-lock test then reports a poisoned lock.
- `cargo fmt --check` failed on pre-existing formatting drift in `crates/opencode-provider/src/anthropic.rs` and `crates/opencode-tui/src/context/keybind.rs`, outside this change.
- `ort-build` could not be run from this shell because the command was not found.
- `cargo build` was attempted as a fallback in the original QA checkout and timed out after 120 seconds; rustc processes were terminated by timeout signal, not source diagnostics.
- 2026-09-21: `cargo test -p opencode-tui cursor_visual_position -- --test-threads=1` passed (5 cursor-geometry tests).
- 2026-09-21: `cargo test -p opencode-tui -- --test-threads=1 --skip tab_autocomplete_uses_first_candidate` passed (34 tests).
- 2026-09-21: `cargo test -p opencode-tui -- --test-threads=1` still fails only on the pre-existing `tab_autocomplete_uses_first_candidate` (`test` vs `team`) and the resulting env-lock poison. Confirmed the same failure on clean `development` HEAD with these changes stashed, so it is unrelated to this item.
- 2026-09-21: `cargo check -p opencode-tui`, `cargo build -p opencode-tui`, and `cargo clippy -p opencode-tui --all-targets` completed. No new clippy warnings in the changed code; pre-existing workspace/TUI warnings remain.
- 2026-09-21: `rustfmt --edition 2021 --check crates/opencode-tui/src/components/prompt.rs` passed. Whole-workspace `cargo fmt` still flags only pre-existing drift in `anthropic.rs` and `keybind.rs`; an incidental reformat of `keybind.rs` was reverted.
- 2026-09-21: Added a ratatui `TestBackend` test that renders the pre-wrapped lines and asserts the cursor cell points at the next grapheme, plus word-boundary, wide-char, newline, full-line-boundary, and grapheme-cluster cursor tests.
- 2026-09-21: `ort-build`/`ort` are not on `PATH` in this shell, so real-terminal QA could not be run here. Needs interactive verification: type spaced/CJK/emoji prompts that wrap, paste multi-line text, move with Home/End/word keys, delete at start/middle/end, resize, and type again after streaming output.
- 2026-09-21: `rustfmt --edition 2021 --check crates/opencode-tui/src/components/prompt.rs` passed.
- 2026-09-21: `cargo test -p opencode-tui --lib cursor -- --test-threads=1` passed (9 tests, including the three new render/invariant tests).
- 2026-09-21: `cargo test -p opencode-tui -- --test-threads=1 --skip tab_autocomplete_uses_first_candidate` passed (37 tests).
- 2026-09-21: `cargo clippy -p opencode-tui --all-targets` produced no new warnings in `prompt.rs`; pre-existing crate/workspace warnings remain.
- 2026-09-21: `ort-build`/`ort` still unavailable in this shell, so real-terminal QA remains outstanding before this item can leave `qa`.

## Fix reconciliation (2026-09-21)

Two earlier fixes landed on this item before the final cleanup. They are sequential, not conflicting; the second supersedes the first.

- Fix A (PR #47 / `36ee852` "fix(tui): align prompt cursor at line edge"): narrow band-aid that let the visual cursor column equal `input_width` at an exact full-line boundary instead of clamping it back to `input_width - 1`.
- Fix B (`1207cd3` "fix(tui): align prompt cursor with wrapped layout", merged to `development` as `fc86b5b`): replaced the parallel wrap math (removed `input_cursor_visual_position` / `visual_line_count`) with a single `wrap_prompt_input` shared by rendering, cursor placement, box height, and scroll.

Relationship:

- Not conflicting. Fix B was built on top of Fix A (`36ee852` is an ancestor of `fc86b5b`), so there was no revert and no competing edit to merge.
- Fix B supersedes Fix A's treatment of the full-line boundary. Fix A drew the cursor on the pad cell at `col == input_width`; Fix B places it on the next (phantom) `row + 1` at `col == 0`. Only one can apply, and the live code is Fix B's.
- Residual redundancy: Fix A's named `max_col` variable was deleted, but the clamp survived inline as `cursor_col.min(input_width)` in `Prompt::render`. That clamp was a no-op, and this final change removes it.

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/47
- https://github.com/cchris-p/opencode-modded-rust/pull/52 (final follow-up; base `development` @ `a9e5d9f`)
- Fix B rework was merged directly (no PR) as merge commit `fc86b5b` on `development`.

## Completion

- 2026-09-19: PR #47 merged into `development`; branch cleanup completed. Kept in `qa` for observation and revisit if the cursor misalignment is seen again.
- 2026-09-21: Reopened and reworked on branch `bug/BUG-018-prompt-cursor-position`. Moved back to `qa` awaiting interactive/real-terminal verification; branch left checked out.
- 2026-09-21: Branch merged into `development` (merge `fc86b5b`) and deleted. Still in `qa` pending interactive/real-terminal verification on `development`; the automated verification in Dev Notes already passes.
- 2026-09-21: Final follow-up on branch `bug/BUG-018-cursor-core-fix` (based on `development` @ `a9e5d9f`): removed the surviving clamp and replaced the tautological render test with real-render cursor tests. Opened as PR #52; still in `qa` pending interactive/real-terminal verification.
