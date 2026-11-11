---
id: "FEAT-074"
title: "Auto-hide the prompt input box after every sent message"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-11-11"
updated: "2026-11-11"
---

# Auto-hide the prompt input box after every sent message

## Summary

Hide the prompt input box automatically whenever the user submits a non-empty message, instead of
leaving it visible until the user presses `Ctrl+T` again. The operator's current workflow is: the
box is hidden, they press `Ctrl+T` to reveal it, type, send, and then want it to disappear again
without a second manual `Ctrl+T`.

"Hide" means the existing persisted `prompt_hidden` display state (FEAT-064), not a new input mode.
Submission, typing, paste, history, and cursor behavior stay unchanged; only the post-submit
visibility is set to hidden.

## Why this exists

Requested by the operator 2026-11-11: "I want to hide the text box whenever I send a message
(always). I have to manually make it appear (ctrl t) in order to see it but every time I send a
message I want it to disappear."

Today `Ctrl+T` (keybind `prompt_toggle`) flips `prompt_hidden`, but a send never changes it. With
`prompt_hidden = false`, the session prompt stays visible after send (the implicit empty-draft rule
only hides it when the viewport is not near the bottom). The operator therefore has to re-hide the
box manually after each message.

## Scope

- After a non-empty submit, set the persisted `prompt_hidden` state to `true`.
- Apply the same behavior to the Home (first-prompt) route and the Session route.
- Apply the same behavior to a submitted shell command (`$ ...`), since it is also a "send".
- Keep all existing input handling intact: the prompt model still receives keys and `Enter` still
  submits while hidden (FEAT-064 invariant).
- Reuse the persisted `prompt_hidden` ui key so the auto-hidden state and the `Ctrl+T` /
  `/prompt` / command-palette toggle read and write the same value.

## Non-goals

- Do not add a configuration flag or a new keybind; the request is "always".
- Do not change what `Ctrl+T`, `/prompt`, `/prompt.toggle`, or the command-palette entry do.
- Do not change the implicit empty-draft visibility rule, input semantics, submission, history,
  paste, newline, or clear behavior.
- Do not auto-show the prompt when the user starts typing; revealing remains an explicit `Ctrl+T`
  (this matches the operator's described workflow).
- Do not change how the prompt is rendered when visible.

## Implementation plan

1. `crates/opencode-tui/src/context/app_context.rs`
   - Add a `hide_prompt(&self)` helper beside `toggle_prompt_hidden` (`app_context.rs:263-267`).
     Set `*self.prompt_hidden.write() = true` and persist `ui_kv.set_bool("prompt_hidden", true)`.
2. `crates/opencode-tui/src/app/app.rs`
   - In `submit_prompt` (`app.rs:2666`), after the empty-input early return
     (`app.rs:2669-2671`) and before the `shell_mode` branch, call `self.context.hide_prompt()`.
     This covers both prompt and shell-command submits on Home and Session, and also hides when the
     session/send later fails, keeping behavior consistent with the input already being taken.
3. Rendering is already gated on `self.context.prompt_hidden` on both surfaces
   (`session.rs:286-287`, `home.rs:94-95`), so no render change is needed. Hidden means zero rows
   and no terminal cursor placement.

## Acceptance criteria

- Pressing `Enter` on a non-empty prompt hides the input box immediately after the turn is sent, on
  both Home and Session.
- Submitting a `$` shell command hides the input box the same way.
- Pressing `Enter` on an empty prompt does not change visibility.
- `Ctrl+T`, `/prompt`, `/prompt.toggle`, and the command-palette entry still toggle visibility and
  agree with the auto-hidden state.
- The auto-hidden state persists across a TUI restart via the existing `prompt_hidden` ui key.
- Typing while hidden still edits the draft and `Enter` still submits.

## Verification

- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-tui` — clean.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui --lib` — pass; add an `AppContext` test
  that `hide_prompt` sets `prompt_hidden` and persists it to the ui kv, and that it is safe when
  already hidden.
- Manual `ort-build` then `ort`: confirm the box is hidden, `Ctrl+T` to show, type and send, and
  confirm the box disappears again; confirm `Ctrl+T` restores it and the draft/persistence behaves
  as before.

## Dev Notes

Implemented on `feat/FEAT-074-auto-hide-prompt-after-send` (branch off `development`).

- `crates/opencode-tui/src/context/app_context.rs`: added `hide_prompt()`, which sets
  `prompt_hidden = true` and persists `ui_kv["prompt_hidden"] = true`; it is idempotent when already
  hidden and shares state with `toggle_prompt_hidden()`.
- `crates/opencode-tui/src/app/app.rs` (`submit_prompt`): after the empty-input early return and
  before the `shell_mode` branch, call `self.context.hide_prompt()`. This covers prompt and shell
  submits on Home and Session, and also hides when a later session/send call fails (the input has
  already been taken).
- Rendering required no change: `session.rs` and `home.rs` already gate on `context.prompt_hidden`,
  so a hidden box reserves zero rows and places no terminal cursor.
- Slash commands are unaffected: the slash popup dispatches `CommandAction`s directly
  (`app.rs:1506-1510`) and does not route through `submit_prompt`, so `/prompt` still shows the box.
- Added `hide_prompt_hides_and_persists_to_ui_kv` covering hide, idempotent hide, and that a later
  toggle still flips it back to visible.

Verification run:

- `cargo fmt --all` — applied.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-tui` — clean.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui --lib` — 190 passed, 0 failed.

QA: interactive `ort-build`/`ort` check (send hides the box; `Ctrl+T` reveals it) is pending on the
PR branch.

## Related Items

- `FEAT-064` Hide the prompt input box with a Ctrl+T display toggle while keeping input active —
  provides the `prompt_hidden` state, persistence, `Ctrl+T`/`/prompt`/palette toggle, and the
  render gate this card reuses; this card only adds the post-send state transition.
- `FEAT-036` Persist and recall typed input-box messages after send or clear (hold) — adjacent
  lifecycle work on what happens to input after send; out of scope here.

## Decisions

- Auto-hide reuses the existing persisted `prompt_hidden` flag rather than introducing a separate
  transient flag, so the auto-hidden state and the manual toggle cannot disagree.
- Hiding happens on every non-empty submit, including shell commands, and regardless of whether
  the subsequent network call succeeds.
