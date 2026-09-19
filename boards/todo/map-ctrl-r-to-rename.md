---
id: "FEAT-025"
title: "Map Ctrl+R to rename"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-18"
---

# Map Ctrl+R to rename

## Summary

Map `Ctrl+R` in the TUI session view to the existing session rename action.

## Scope

- Add a `Ctrl+R` keybinding that opens the current rename flow for the active session.
- Keep the existing rename command and any current menu/action entry points working.
- Update visible hotkey/help text if the TUI displays rename shortcuts.

## Acceptance Criteria

- Pressing `Ctrl+R` from the normal session view starts renaming the active session.
- The binding does not fire while focus is inside text entry modes where `Ctrl+R` should be reserved or ignored.
- Existing rename behavior, validation, and persistence remain unchanged.

## Likely Touchpoints

- `crates/opencode-tui/src/app/app.rs`
- TUI keybinding/help components if shortcut hints are rendered outside `app.rs`

## Verification

- `cargo check -p opencode-tui`
- Manual smoke with `ort-build` then `ort`: open a session, press `Ctrl+R`, rename it, and confirm the displayed name updates.
