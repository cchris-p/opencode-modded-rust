---
id: "BUG-014"
title: "Enable Alt key word skipping in prompt input"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
created: "2026-09-16"
---

# Enable Alt key word skipping in prompt input

## Summary

The TUI prompt input should support standard terminal word navigation with Alt/Option plus left/right arrow keys. Pressing Alt+Left should move the input cursor to the previous word boundary, and pressing Alt+Right should move it to the next word boundary.

## Why this exists

The prompt input is the product's primary editing surface. Users expect it to behave like a normal terminal input field, including word-level cursor movement for quickly editing longer prompts. Missing Alt word navigation makes prompt editing slower and inconsistent with default terminal behavior.

## Scope

- Add prompt-input handling for Alt/Option+Left and Alt/Option+Right word navigation.
- Match common terminal input-field behavior for skipping to previous and next word boundaries.
- Keep the cursor visible after word navigation, including when the prompt wraps or scrolls horizontally/vertically.
- Preserve existing character-level movement, prompt editing shortcuts, selection behavior, and global/dialog key handling.
- Support the key event forms emitted by common terminals for Alt/Option arrow combinations where practical.

## Non-goals

- Adding full readline/emacs/vi editing parity.
- Making keybindings user-configurable.
- Changing transcript scrolling or session navigation shortcuts unrelated to prompt editing.
- Redesigning the prompt composer UI.

## Done when

- Alt+Left in the focused prompt moves the cursor to the previous word boundary.
- Alt+Right in the focused prompt moves the cursor to the next word boundary.
- Word navigation works in empty prompts, single-word prompts, multi-word prompts, prompts with punctuation, and wrapped long prompts without panics or cursor misplacement.
- The cursor remains visible after Alt word navigation.
- Existing prompt editing shortcuts continue to work.

## Recommended verification

- Run `cargo fmt --check -p opencode-tui`.
- Run relevant prompt/input key handling tests, or add focused coverage if none exists.
- Run `cargo check -p opencode-tui`.
- Run `ort-build`, then launch `ort` in a real terminal.
- Type a long multi-word prompt and verify Alt+Left and Alt+Right jump by words while the cursor remains visible.
- Verify normal Left/Right movement and existing clear/exit shortcuts still behave as expected.

## Related Items

- `BUG-013` Cursor on the input field needs to always be visible - same prompt cursor placement and visibility surface.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input key handling surface.
- `PHASE-001` V1 daily-driver hardening - prompt editing ergonomics are part of daily-driver readiness.

## Notes

- Created on 2026-09-16 from the request to enable default terminal/input-field Alt word skipping behavior.

## Dev Notes

- Added prompt-level word navigation for Alt+Left and Alt+Right.
- Added Meta-B and Meta-F handling for terminals that emit Alt/Option word navigation as escaped `b`/`f` key events.
- Replaced the app-level prompt key filter with a helper that continues blocking broad Ctrl/Alt shortcuts while allowing only the prompt-owned Alt word navigation keys through.
- Word boundaries treat alphanumeric and underscore characters as word characters; punctuation and whitespace act as separators.

## Verification

- `cargo fmt --check -p opencode-tui`
- `cargo test -p opencode-tui move_by_words -- --test-threads=1`
- `cargo check -p opencode-tui`
- `cargo clippy -p opencode-tui --all-targets` completed with existing workspace warnings and no errors.
- `cargo test -p opencode-tui -- --test-threads=1`
- `ort-build` could not be run from this non-interactive shell because the launcher command was not on `PATH`.
