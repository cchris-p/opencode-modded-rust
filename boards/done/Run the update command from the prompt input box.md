---
id: "FEAT-081"
title: "Run /update from the prompt input box"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "README.md"
status: "done"
created: "2026-11-16"
updated: "2026-11-17"
---

# Run /update from the prompt input box

## Summary

Pressing Enter after typing `/update` (or `/upgrade`) in the session prompt should open the
update dialog instead of sending the text to the model. Today `/update` only works while the
inline slash-completion popup is open and selected; dismissing the popup (or pasting the text)
falls through to `submit_prompt`, which sends the literal string to the model.

## Why this exists

Follow-up to FEAT-080. The feature advertises `/update` as a session command, but submission
paths that bypass the popup should still honor it. This makes the command reliable when
pasted, when the popup is dismissed with Esc, and when a user types the whole command and
submits.

## Scope

- Intercept a bare `/update` / `/upgrade` (case-insensitive, surrounding whitespace allowed)
  in `submit_prompt` before any session/network work, clear the prompt, and execute
  `CommandAction::CheckForUpdates`.
- Leave every other `/...` input unchanged (still sent as a normal prompt) and do not change
  shell mode.
- Do not treat `/update <args>` or text containing `/update` as the command.
- Add a unit test for the parser and update the README hotkey/command text.

## Non-goals

- Turning every registry slash command into an input-box command.
- Changing the popup, keybind, or update detection/install behavior from FEAT-080.

## Implementation notes

- `input_command_action` in `crates/opencode-tui/src/app/app.rs` resolves the first token
  through `CommandRegistry` and only returns `CheckForUpdates`.
- `submit_prompt` checks it before `take_input`, so a command is not sent to the server and
  does not create/enter a session.

## Verification (2026-11-16)

- `cargo test -p opencode-tui input_command_action` passes.
- `cargo fmt --all -- --check` clean.
- Implemented in PR https://github.com/cchris-p/opencode-modded-rust/pull/196 (same branch as
  FEAT-080).
- **Merged into `development` 2026-11-16 (PR #196, merge commit `29857cb`); in `qa`, awaiting
  owner QA.**

## Related Items

- `FEAT-080` Detect and suggest new ORT release updates.
- `CLI-017` Show the ORT version for `ort --version`.

## QA Pass - 2026-11-17

Owner accepted the fleet v0.1.5 install/update run as the QA pass. The input-box `/update` /
`/upgrade` path shipped in `v0.1.5` and is installed on all reachable active machines.

Status: done.
