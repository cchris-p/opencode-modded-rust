---
id: "FEAT-008"
title: "Disable the TUI sidebar by default"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
created: "2026-09-09"
---

# Disable the TUI sidebar by default

## Summary

Start the TUI with the sidebar hidden by default, using a hardcoded default value so the sidebar is off at startup for the narrow daily-driver workflow while remaining reachable on demand.

## Why this exists

The personal daily-driver workflow does not need the sidebar (LSP/MCP/todo/pending state panels) occupying horizontal space at startup. The sidebar should be hidden unless the user explicitly opens it, rather than shown and then dismissed every session.

## Scope

- Change the hardcoded initial value so the sidebar starts hidden: `show_sidebar: RwLock<bool>` in `crates/opencode-tui/src/context/app_context.rs` currently initializes to `true`.
- Keep the existing in-session toggle (`sidebar_toggle`, default Ctrl+S) so the sidebar can still be shown and hidden on demand for the rest of the session.
- Leave `sidebar_mode` (`SidebarMode::Auto`) behavior untouched.
- Make no config, persistence, or parity changes in this item.

## Non-goals

- Wiring the existing `tui.sidebar` config option or `ui_kv` persistence for sidebar visibility.
- Replacing the hardcode with the reference OpenCode sidebar-visibility mechanism. That is tracked separately as `FEAT-009`.
- Redesigning the sidebar contents or layout.

## Done when

- A freshly started TUI session renders with the sidebar hidden by default.
- The user can still toggle the sidebar on and off during a session with the existing keybind.
- No existing sidebar toggle/session behavior regresses.

## Recommended verification

- Build with `ort-build` and launch with `ort`.
- Confirm the TUI starts without the sidebar visible.
- Press Ctrl+S (or the configured `sidebar_toggle`) and confirm the sidebar appears, then toggles back off.
- Run the TUI crate checks/tests affected by the default flip: `cargo check -p opencode-tui` and relevant `cargo test -p opencode-tui`.

## Related Items

- `FEAT-009` Clone sidebar visibility handling from OpenCode to replace the hardcoded default.

## Dev Notes

- Flipped the startup default in `crates/opencode-tui/src/context/app_context.rs:143` from `RwLock::new(true)` to `RwLock::new(false)`.
- No other code changes needed: the in-session Ctrl+S `sidebar_toggle` (`app.rs:549`), mouse open/close buttons (`session.rs:936-948`), and `SidebarMode::Auto` behavior are all driven off the same `show_sidebar` value and are unchanged.
- Verification: `cargo check -p opencode-tui` passes; `cargo test -p opencode-tui` passes (21 tests). No tests asserted the previous default.

## Notes

- `tui.sidebar: Option<bool>` already exists in the config schema but is not consumed by the startup default; this card intentionally does not wire it.