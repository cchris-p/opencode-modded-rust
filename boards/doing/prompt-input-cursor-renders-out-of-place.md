---
id: "BUG-018"
title: "Prompt input cursor renders out of place in the TUI"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
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

## Dev Notes

- 2026-09-19: Fixed cursor placement at the prompt content-width boundary. The visual cursor column can legitimately equal `input_width` when the insertion point is immediately after the last visible cell on a full line; rendering now allows that column instead of clamping it back to `input_width - 1`.
- Added/extended cursor visual-position coverage for the exact full-width boundary case.

## Verification

- `cargo test -p opencode-tui cursor_visual_position` passed.
- `cargo test -p opencode-tui components::prompt::tests::utf8_backspace_delete_and_cursor_are_char_safe -- --exact` passed.
- `cargo test -p opencode-tui` failed on existing prompt autocomplete isolation: `components::prompt::tests::tab_autocomplete_uses_first_candidate` returns `test` instead of `team`; the following env-lock test then reports a poisoned lock.
- `cargo fmt --check` failed on pre-existing formatting drift in `crates/opencode-provider/src/anthropic.rs` and `crates/opencode-tui/src/context/keybind.rs`, outside this change.

## PR

- (empty)

## Completion

- (empty)
