---
id: "BUG-037"
title: "Slash menu freezes keyboard input until terminal resize"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-22"
---

# Slash menu freezes keyboard input until terminal resize

## Summary

Opening the in-session slash-command menu by pressing `/` can wedge the TUI input state. The slash
menu disappears, the user is left on the prompt screen, and keyboard input no longer has any visible
effect. Resizing the terminal restores the UI/input path; otherwise the user has to kill the
`opencode-rust` process to recover.

This is reported as an always-reproducible regression in iTerm2. The user noted it did not happen
previously and suspects the most recent provider-list change may be the first affected build.

## Reported Behavior

- Pressing `/` appears to trigger the failure immediately.
- The slash menu disappears instead of remaining interactive or closing cleanly.
- The screen shows only the prompt area.
- All keyboard input appears frozen: typing, `/`, `Esc`, `Ctrl-C`, `Ctrl-D`, and navigation keys have
  no visible effect.
- Resizing the terminal restores the UI/input path.
- If the terminal is not resized, recovery requires killing the `opencode-rust` process and restarting
  the TUI.
- Repro rate: always.
- Terminal: iTerm2.

## Why This Matters

The slash menu is a primary command entry point. If opening it can make the TUI ignore all keyboard
input, the app becomes unusable in normal daily-driver flow and appears hard-locked even though a resize
can recover it.

## Initial Hypotheses

- The slash-command popup may leave the app in a stale focus/input mode after opening or immediately
  closing.
- A render invalidation or layout recomputation may be missing, since terminal resize repairs the state.
- The event loop may still be alive, but regular key events are not causing a redraw or are routed to a
  hidden/cleared component.
- Recent provider-list or command-list changes may have introduced an empty, duplicate, or invalid menu
  state that the slash popup does not handle correctly.

## Related Items

- `boards/done/remove-duplicate-session-actions-and-hotkey-hints-from-session-ui.md` touched the
  slash-command suggestion path and should be checked for interaction with this regression.
- The user suspected the most recent duplicate/provider-list work as a possible regression window; no
  exact matching board item was identified during card creation.

## Investigation Starting Points

- Slash command registry and rendering: `crates/opencode-tui/src/command.rs` and
  `crates/opencode-tui/src/components/slash_command.rs`.
- Prompt input focus and slash popup dispatch in `crates/opencode-tui/src/app/app.rs`.
- Redraw/layout invalidation paths that run on terminal resize versus ordinary key input.

## Open Questions

- Does this reproduce in a terminal other than iTerm2?
- Does it reproduce in a fresh session before any model/provider command is used?
- Does it reproduce when the slash menu has zero matches, many matches, or only provider-related matches?
- Which commit or board item first introduced the regression?

## Expected Behavior

- Pressing `/` opens the slash-command menu without freezing input.
- If the menu has no valid result or closes immediately, focus returns cleanly to the prompt.
- `Esc`, `Ctrl-C`, typing, and navigation keys remain responsive after opening or closing the menu.
- No terminal resize is required to restore input or redraw the prompt.

## Done When

- The root cause is identified and documented on this card.
- The slash-command menu can be opened repeatedly in iTerm2 without freezing keyboard input.
- Closing or cancelling the menu returns focus to the prompt reliably.
- Regression coverage exists for the stale focus/render state if practical.
- Manual verification includes opening `/`, cancelling with `Esc`, typing text, and submitting a prompt
  without resizing the terminal.
