---
id: "BUG-002"
title: "Remove duplicate session actions and visible hotkey hints from the session UI"
priority: "P1"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "done"
created: "2026-08-31"
---

# Remove duplicate session actions and visible hotkey hints from the session UI

## Summary

The in-session command UI currently shows multiple `Switch Session` and `New Session` choices with hotkey-looking text such as `ctrl_n` and `ctrl_s`. Simplify that session-facing UI so each intent appears once and the hotkey text is removed.

## Reported behavior

- The user sees multiple `Switch Session` and `New Session` entries.
- The UI shows text that looks like hotkeys, including `ctrl_n` and `ctrl_s`, to the right of those entries.
- On macOS, trying those hotkeys does nothing.

## Required solution direction

- Do not preserve or expand hotkey support for this flow.
- Remove the duplicate choices.
- Remove the visible hotkey text from the UI.
- Do not add a new hotkey customization feature.

## Scope

- Fix the in-session slash-command suggestions first; current evidence points at `crates/opencode-tui/src/command.rs` and `crates/opencode-tui/src/components/slash_command.rs`.
- Remove any duplicate `New Session` and `Switch Session` entries from that session-facing list.
- Remove the visible hotkey label text from the affected session-facing list.
- Do not broaden this item into restoring or redesigning keyboard shortcut behavior.
- Only touch `crates/opencode-tui/src/components/dialogs/command_palette.rs` if the same duplicated session actions or visible hotkey hints are still part of the reproduced bug surface after checking the slash-command path.

## Done when

- The relevant session UI shows only one `Switch Session` action and one `New Session` action where appropriate.
- The hotkey text is no longer shown for those actions.
- The resulting UI remains clear on macOS without depending on non-working keyboard shortcuts.

## Verification

- In a session, open the slash-command suggestions and confirm the session actions list shows only one `New Session` entry and one `Switch Session` entry.
- Confirm the same list no longer shows `ctrl_n`, `ctrl_s`, or similar hotkey hint text beside those actions.
- Confirm selecting the remaining entries still performs the expected session action.

## Notes

- The user does not want hotkeys for this area and does not want the product to support creating them.
- Treat this as a product-surface simplification, not as a hotkey-debugging task.
- Current code evidence: the slash-command registry still assigns `ctrl_n` and `ctrl_s` in `crates/opencode-tui/src/command.rs`, and the slash-command popup renders keybind text in `crates/opencode-tui/src/components/slash_command.rs`.

## Dev Notes

- Root cause: `CommandRegistry::search()` iterated alias-backed registry entries directly, so queries such as `resume` could surface the same session action more than once.
- Implementation: deduplicated slash-command search results by canonical command name and removed the slash-command keybind hints for `/new` and `/sessions`.
- Verification: `cargo test -p opencode-tui command::tests -- --nocapture`

## Related Items

- `START-018` Complete TUI approval and question handling
- `FEAT-002` Keep sessions running after TUI exit