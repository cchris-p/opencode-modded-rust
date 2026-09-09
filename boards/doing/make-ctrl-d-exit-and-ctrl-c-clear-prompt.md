---
id: "FEAT-006"
title: "Make Ctrl+D exit the TUI and Ctrl+C clear the prompt"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
created: "2026-09-09"
---

# Make Ctrl+D exit the TUI and Ctrl+C clear the prompt

## Summary

Prevent `Ctrl+C` from quitting the TUI session. `Ctrl+C` should instead clear the prompt input field. `Ctrl+D` becomes the primary exit key.

This is intended as a natural, durable TUI behavior that should not regress.

## Why this exists

The current TUI treats plain `Ctrl+C` (with no active text selection) as quit (`AppState::Exiting`). A user who reaches for `Ctrl+C` out of habit loses the running session instead of just clearing their draft input. In a terminal, `Ctrl+D` is the conventional "exit/EOF" gesture, so moving quit onto `Ctrl+D` and repurposing `Ctrl+C` to clear the prompt matches user expectations and avoids accidental quits.

## Current Rust behavior

- Global key handler in `opencode-tui/src/app/app.rs`: `Ctrl+C` copies an active selection, otherwise it exits the TUI.
- `Ctrl+D` is only bound inside the session-list dialog as the session delete (arm + confirm) shortcut.
- Prompt clearing is bound to `Ctrl+U` (`input_clear`) and also reachable via the `/clear`-style command action.

## Scope

- Change the top-level (no open dialog, no active selection) `Ctrl+C` handling to clear the prompt instead of quitting.
- Keep the existing behavior where `Ctrl+C` with an active text selection copies that selection.
- Make `Ctrl+D` quit the TUI at the top level.
- Keep the session-list dialog's `Ctrl+D` = delete shortcut working (dialog swallows keys first, so quit stays a main-screen behavior).
- Keep `q` and `Esc` behaviors unchanged.
- Update the help dialog and command palette exit hint so the documented exit key matches the new default.
- Update the default `app_exit` keybind registration from `Ctrl+C` to `Ctrl+D`.

## Non-goals

- Changing the session delete shortcut inside the session-list dialog.
- Changing interrupt semantics (`Esc` double-press) for running sessions.
- Reordering how dialogs consume keys before the global handler.
- Making keybindings user-configurable.

## Done when

- Pressing `Ctrl+C` in the main prompt view no longer quits the TUI and instead clears any prompt text.
- Pressing `Ctrl+C` with an active text selection still copies the selection.
- Pressing `Ctrl+D` in the main prompt view exits the TUI.
- Pressing `Ctrl+D` inside the session-list dialog still deletes the selected session.
- Help text lists `Ctrl+D` (not `Ctrl+C`) as the exit key and documents `Ctrl+C` as clearing the prompt.

## Recommended verification

- Run `cargo fmt --check` and `cargo clippy` on the TUI crate.
- Manually verify in the TUI: type draft text, press `Ctrl+C` and confirm the prompt clears without exiting; press `Ctrl+D` and confirm the TUI exits.
- Open the session list, select a session, press `Ctrl+D` twice and confirm deletion still works (unchanged).

## Related Items

- `FEAT-002` Keep sessions running after TUI exit - exit must stay intentional so background sessions can continue.
- `BUG-002` Remove duplicate session actions and hotkey hints from session UI - same UI/help text surface for key hints.

## Dev Notes

- `app.rs` global key handler: top-level `Ctrl+C` now clears the prompt input (via `prompt.clear()`) instead of setting `AppState::Exiting`. Copy-on-selection behavior for `Ctrl+C` is preserved. A new top-level `Ctrl+D` handler sets `AppState::Exiting`.
- Dialog-scoped shortcuts are unchanged because dialogs consume keys before the global handler: session-list `Ctrl+D` delete, alert/session-export `Ctrl+C` copy, etc.
- `keybind.rs` default `app_exit` binding updated from `Ctrl+C` to `Ctrl+D` to match the new top-level behavior.
- Help dialog now lists `Ctrl+D/q Exit TUI` and `Ctrl+C/U Clear prompt`; command palette Exit hint now shows `ctrl+d`.

## Verification

- `cargo fmt --check -p opencode-tui`
- `cargo check -p opencode-tui`
- `cargo clippy -p opencode-tui --all-targets` (no new warnings from this change)

## PR

- Pending

