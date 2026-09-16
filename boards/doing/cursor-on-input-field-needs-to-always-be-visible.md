---
id: "BUG-013"
title: "Cursor on the input field needs to always be visible"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
created: "2026-09-16"
---

# Cursor on the input field needs to always be visible

## Summary

The TUI input-field cursor is not always visible. While typing or editing a prompt, the user can lose visual track of where input will land, which makes the main daily-driver interaction unreliable.

## Why this exists

The prompt input is the primary control surface for the product. Its cursor must remain visible whenever the input field has focus, including after typing, editing, resizing, scrolling transcript content, receiving assistant output, or opening and closing normal TUI overlays.

## Scope

- Identify why the input cursor becomes hidden or drawn outside the visible input-field area.
- Keep the cursor visible whenever the prompt/input field is focused and accepting text.
- Preserve existing prompt editing behavior, keybindings, selection behavior, and input-field layout.
- Ensure fixes work in both narrow and wide terminal sizes.

## Non-goals

- Redesigning the prompt composer UI.
- Changing session transcript scrolling semantics except where necessary to keep the input cursor visible.
- Changing keyboard shortcuts unrelated to prompt editing or focus.

## Done when

- The input-field cursor remains visible while entering and editing a prompt.
- The cursor remains visible after the transcript changes because assistant text, tool output, or status lines are rendered.
- The cursor remains visible after terminal resize and after opening/closing standard TUI overlays.
- The input field does not overlap or push the cursor behind borders, status lines, footer hints, or other layout elements.

## Recommended verification

- Run `ort-build`, then launch `ort` in a real terminal.
- Type a prompt long enough to wrap and confirm the cursor stays visible at the insertion point.
- Move within the prompt text using normal editing keys and confirm the cursor remains visible.
- Send a prompt, wait for streaming output, then type another prompt and confirm the cursor is visible throughout.
- Resize the terminal smaller and larger while the prompt has text and confirm the cursor stays visible.
- Open and close common overlays such as help, session list, command palette, or provider/model selection if available, then confirm the input cursor is visible when focus returns to the prompt.

## Related Items

- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input key handling surface.
- `BUG-002` Remove duplicate session actions and hotkey hints from session UI - related TUI session chrome/hint surface.
- `PHASE-001` V1 daily-driver hardening - this blocks trust in the primary daily-driver input loop.

## Dev Notes

- Updated `Prompt::render` to place the terminal cursor at the prompt input insertion point whenever the prompt is focused.
- Added wrapped-input cursor coordinate calculation that handles newlines and wide Unicode characters.
- Added prompt paragraph scrolling so drafts longer than the visible prompt area keep the logical cursor row in view.
- Kept prompt editing, keybindings, and visual layout unchanged outside cursor placement/scrolling.

## Verification

- `cargo fmt --check -p opencode-tui`
- `cargo test -p opencode-tui cursor_visual_position -- --test-threads=1`
- `cargo check -p opencode-tui`
- `cargo clippy -p opencode-tui --all-targets` completed with existing workspace warnings; one new warning in the changed prompt code was fixed.
- `cargo build -p opencode-tui`
- `ort-build` could not be run from this non-interactive shell because the launcher command was not on `PATH`.
