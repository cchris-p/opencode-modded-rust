---
id: "FEAT-064"
title: "Hide the prompt input box with a Ctrl+T display toggle while keeping input active"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-09-25"
---

# Hide the prompt input box with a Ctrl+T display toggle while keeping input active

## Summary

Add a persisted display toggle that visually hides the prompt input box, bound to `Ctrl+T` (keybind
name `prompt_toggle`) and also reachable from the command palette and a slash command `/prompt`
(alias `/prompt.toggle`). When hidden, the box collapses to zero height so the transcript gets the
reclaimed rows, but keyboard input, editing, cursor movement, paste, and `Enter` submit must all keep
working exactly as if the box were visible. Hiding is a rendering decision only, never an
input-disable.

## Why this exists

Requested by the operator 2026-09-25: they want to reclaim vertical space in the session view without
losing the ability to type and send prompts. Today the only way the prompt area changes size is the
implicit empty-draft rule, and there is no explicit, persistent user control over prompt visibility.

Current behavior and evidence:

- The session layout already computes an implicit visibility rule, not a user toggle:
  `let show_prompt = !prompt_empty || near_bottom;` at
  `crates/opencode-tui/src/components/session.rs:221`. When the draft is empty and the viewport is not
  near the bottom, the prompt is not rendered (`session.rs:277-279`). Typing makes `prompt_empty`
  false, so it reappears. There is no way to keep it hidden while typing.
- Prompt rendering is the same `Prompt` component on both the home and session surfaces:
  `crates/opencode-tui/src/components/session.rs:78` and `home.rs:69-107`.
- `Prompt::render` unconditionally calls `frame.set_cursor(...)` (`crates/opencode-tui/src/components/prompt.rs:421-423`),
  so a hidden box must be skipped entirely rather than rendered into a zero-height area.
- The persisted display-toggle pattern to copy: `tips_hidden` state (`app_context.rs:111`), its
  seeded default (`app_context.rs:180-185`), `toggle_tips_hidden` (`app_context.rs:247-251`),
  command `/tips.toggle`/`/tips` (`command.rs:408-415`), palette entry
  (`command_palette.rs:111`), and dispatch (`app.rs:1960-1962`).
- Keybind dispatch and registration: `sidebar_toggle` is matched at `app.rs:695-698` and registered
  in `crates/opencode-tui/src/context/keybind.rs:198`. `Ctrl+T` is currently unused as a direct
  binding; the existing `Char('t')` case at `app.rs:492` is a leader (`Ctrl+X`) sub-key for
  `SwitchTheme`.
- Config startup defaults for display toggles: `TuiConfig` fields at
  `crates/opencode-config/src/schema.rs:349-368`.

## Scope

- Add a `prompt_hidden: RwLock<bool>` field to `AppContext` (default `false`), seeded with the
  existing precedence pattern (persisted `kv.json` key > `TuiConfig` startup default > built-in
  default), and a `toggle_prompt_hidden` method that flips it and persists the ui key.
- Wire the toggle to `Ctrl+T` via a new registered keybind `prompt_toggle` and dispatch it in the
  main key handler without stealing the leader sub-key.
- Add a slash command `/prompt` (alias `/prompt.toggle`) and a command-palette entry that flip the
  same state, following the `/tips.toggle` pattern, with a title/description that reflects the
  current visible state.
- Gate prompt rendering on the session route using the new flag, so a hidden prompt reserves zero
  rows and the messages area grows into the freed space. Skip `Prompt::render` entirely when hidden.
- Keep input live while hidden: key events must still route to the `Prompt` (typing, word motions,
  history, paste, newline, clear) and `Enter` must still submit. Do not gate input handling on the
  visibility flag.

## Implementation plan

1. **State + persistence** — `crates/opencode-tui/src/context/app_context.rs`:
   - Add `pub prompt_hidden: RwLock<bool>` beside `tips_hidden` (`app_context.rs:111`).
   - Seed it in the constructor with `seed_bool(&ui_kv, "prompt_hidden", tui.and_then(|t| t.prompt_hidden), false)`,
     mirroring the `tips_hidden` seed (`app_context.rs:180-185`).
   - Add `toggle_prompt_hidden` next to `toggle_tips_hidden` (`app_context.rs:247-251`) that flips
     the lock and persists via `self.ui_kv.write().set_bool("prompt_hidden", *hidden)`.
2. **Config default** — `crates/opencode-config/src/schema.rs`:
   - Add `pub prompt_hidden: Option<bool>` to `TuiConfig` (`schema.rs:337-369`) with the same
     `#[serde(skip_serializing_if = "Option::is_none")]` shape as `tips_hidden` (`schema.rs:368`),
     plus its `merge_option_replace` line in the `DeepMerge` impl for `TuiConfig`.
3. **Keybind** — `crates/opencode-tui/src/context/keybind.rs`:
   - In `register_defaults`, add `self.register("prompt_toggle", Keybind::ctrl(KeyCode::Char('t')));`
     near `sidebar_toggle` (`keybind.rs:198`). `Ctrl+T` is free as a direct binding; the existing
     `Char('t')` at `app.rs:492` is a leader (`Ctrl+X`) sub-key and is unaffected.
   - User-remapping is out of scope: the registry currently only uses built-in defaults and
     `KeybindsConfig` is not applied to it, so no config wiring is required here.
4. **Dispatch** — `crates/opencode-tui/src/app/app.rs`:
   - In the main key handler, add an `if self.matches_keybind("prompt_toggle", *key) { self.context.toggle_prompt_hidden(); return Ok(()); }`
     branch beside `sidebar_toggle` (`app.rs:695-698`). Place it before input handling so it always
     toggles regardless of draft content.
5. **Command + palette**:
   - `crates/opencode-tui/src/command.rs`: add `CommandAction::TogglePrompt` to the Display group
     (`command.rs:70-87`) and register the slash command after `/tips.toggle` (`command.rs:408-415`)
     with `name: "/prompt"`, `aliases: vec!["/prompt.toggle"]`, title `"Hide prompt"`, and action
     `CommandAction::TogglePrompt`.
   - `crates/opencode-tui/src/components/dialogs/command_palette.rs`: add a `Command` entry in the
     `"View"` category (near `ToggleTips`, `command_palette.rs:110-115`) with
     `keybind: Some("ctrl+t".to_string())`.
   - `crates/opencode-tui/src/app/app.rs`: handle `CommandAction::TogglePrompt` in
     `execute_command_action` by calling `self.context.toggle_prompt_hidden()` (near
     `ToggleTips`, `app.rs:1960-1962`); add a `prompt_hidden: bool` parameter to
     `sync_command_palette_labels` (`app.rs:4323-4340`) and flip the title between `"Hide prompt"` /
     `"Show prompt"`, threading `*self.context.prompt_hidden.read()` from the caller at
     `app.rs:1961` and the callers at `app.rs:674`/`app.rs:1980`.
6. **Render gate** — `crates/opencode-tui/src/components/session.rs`:
   - Read `let prompt_hidden = *self.context.prompt_hidden.read();` in `render_main` and combine it
     with the existing rule: `let show_prompt = !prompt_hidden && (!prompt_empty || near_bottom);`
     (`session.rs:221`). The existing zero-height branch (`session.rs:228-237`) and the
     `layout[3].height > 0` guard (`session.rs:277-279`) already skip `prompt.render`, which is what
     keeps `set_cursor` (`prompt.rs:421-423`) from running in a hidden area.
   - Confirm the messages-area constraint grows into the freed rows; the existing `!show_prompt`
     layout branch already sizes messages with `Constraint::Min(0)`, so no new layout code is needed.
7. **Input path** — no change. Do not add a `prompt_hidden` check to the `Route::Home | Route::Session`
   key match (`app.rs:747-757`); `Enter` submit and `prompt.handle_key` must stay unconditional so a
   hidden box still accepts and sends text.

## Non-goals

- Changing the existing implicit empty-draft hide rule (`show_prompt = !prompt_empty || near_bottom`)
  for the visible case. The new toggle is an additional layer, not a replacement.
- Changing submit, clear, newline, paste, history, or cursor semantics.
- Adding a persistent "input hidden" indicator, badge, or hint line.
- Hiding the prompt on the home/landing screen; the home prompt stays visible. Hiding the only
  affordance on an otherwise empty screen has no benefit and the toggle is scoped to the session
  surface.
- Making the toggle keybind itself a display-only behavior change; this is a layout toggle, not an
  input-mode switch.

## Done when

- `Ctrl+T` toggles the prompt box hidden/shown on a session, and the choice survives a TUI restart.
- The slash command and command-palette entry apply and reflect the same state as `Ctrl+T`.
- With the box hidden, typing characters, moving the cursor, using `Alt+Up`/`Alt+Down` history,
  pasting, `Ctrl+J` newline, and `Enter` submit all behave exactly as with the box visible.
- When hidden, the prompt reserves zero rows and the transcript viewport expands into the space; no
  terminal cursor is placed in the hidden area.
- Default is visible for a user with no persisted value and no config key set.

## Constraints

- Keep the hidden state as a pure rendering gate; never disable or unmount the `Prompt` input model.
  The `Prompt` instance continues to own the draft and cursor.
- Do not render the prompt into a zero-height `Rect`; skip the call so `set_cursor` is never reached.
- Preserve the flat board lane convention and the existing frontmatter shape.

## Recommended verification

- `cargo check -p opencode-tui -p opencode-config`.
- Unit-test the toggle default and persistence through `ui_kv`, mirroring
  `context::app_context::tests::tips_default_to_hidden_when_unset`, and add a session-layout test
  asserting zero prompt rows when `prompt_hidden` is true and a non-empty draft.
- Add an input test that inserts text and submits while `prompt_hidden` is true, asserting the
  submission happens (guards against gating input on visibility).
- `ort-build` then `ort`: press `Ctrl+T` and confirm the box disappears and the transcript grows;
  type and submit a prompt blind; toggle back and confirm the draft/cursor state is intact; restart
  and confirm the hidden state persisted; confirm `/prompt.toggle` and the palette entry agree with
  the key.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count — the same
  display-toggle, persistence, command, and palette pattern.
- `FEAT-052` Default home screen tips to hidden — nearest example of a persisted visibility flag with
  a `/`-command and palette entry.
- `FEAT-053` Make TUI display-toggle defaults configurable from opencode.json — defines the
  `TuiConfig` startup-default precedence this flag should join.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt — input-keybind conventions the
  hidden-input path must not regress.

## Decisions

- Keybind is fixed to `Ctrl+T` under the name `prompt_toggle`; remapping is deferred because
  `KeybindsConfig` is not yet applied to the runtime registry.
- Slash command is `/prompt` (alias `/prompt.toggle`), matching the short display-toggle names
  `/sidebar`, `/header`, `/scrollbar`.
- Scope is the session prompt surface only; the home prompt stays visible.

## Dev Notes

Implemented on `feature/FEAT-064-hide-prompt-ctrl-t` (branch off `development`).

- `AppContext.prompt_hidden: RwLock<bool>` added beside `tips_hidden`, seeded with the
  `kv.json > TuiConfig.prompt_hidden > false` precedence, plus `toggle_prompt_hidden()` persisting the
  `prompt_hidden` ui key.
- `TuiConfig.prompt_hidden: Option<bool>` added with its `DeepMerge` line.
- `prompt_toggle` registered as `Ctrl+T`; dispatched in the main key handler beside `sidebar_toggle`
  and before input handling, so it toggles regardless of draft content. The `Ctrl+X` leader sub-key
  `Char('t')` (`SwitchTheme`) is untouched.
- `CommandAction::TogglePrompt` added with slash command `/prompt` (alias `/prompt.toggle`) and a
  `View`-category palette entry (`ctrl+t`). `sync_visibility_labels` now takes `prompt_hidden` and
  flips the palette title between `Hide prompt` / `Show prompt`.
- `render_main` gates the prompt with `!prompt_hidden && (!prompt_empty || near_bottom)`; the existing
  zero-height layout branch and `show_prompt && layout[3].height > 0` guard skip `Prompt::render`
  entirely, so no cursor is placed and the transcript grows into the freed rows.
- Input path intentionally unchanged: no `prompt_hidden` check was added to the Home/Session route key
  match, so typing, paste, history, newline, clear, and `Enter` submit stay live while hidden.

Verification run:

- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-tui -p opencode-config` — clean.
- `cargo test -p opencode-tui --lib` — 164 passed (adds prompt default/config-seed/kv-override/toggle
  persistence tests, session-layout tests asserting zero prompt rows while hidden and reserved rows
  while visible with a non-empty draft, and a `/prompt` alias-resolution test).
- `cargo test -p opencode-config --lib` — 64 passed.
- `cargo fmt --all` applied.

Notes:

- A full-app "submit while hidden" integration test was not added because the `App` has no headless
  test harness; input independence is instead guaranteed by leaving the route key match unmodified.
  The session-layout tests also assert the draft survives a hidden render.
- Only `prompt_hidden` was added to `TuiConfig`'s `DeepMerge` per this card; the earlier FEAT-053
  display flags still lack merge lines (pre-existing, out of scope).
- `prompt.rs` tests `tab_autocomplete_uses_first_candidate` and
  `utf8_backspace_delete_and_cursor_are_char_safe` are flaky (frecency tie ordering); they pass on
  rerun and are unrelated to this change.

## QA Report

QA: FEAT-064 — Ctrl+T prompt visibility toggle with input kept live.

- commit: `ddefbfd` (feature branch tip), merged as `6d3d9bc` (PR #115, base `development`).
- Method: agent-driven QA. This is a TUI-only rendering change with no server API surface, so the
  real render path was driven deterministically with ratatui `TestBackend` instead of the interactive
  TUI, per the product QA policy.
- Commands:
  - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace` — clean.
  - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui --lib` — 166 passed (one rerun after a
    known-flaky prompt.rs pair, see Notes).
  - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-config --lib` — 64 passed.
  - `cargo fmt --all` applied.
- Observed signals:
  - `hidden_prompt_reserves_zero_rows_with_a_non_empty_draft`: rendering `SessionView` with
    `prompt_hidden = true` and draft `"pending draft"` returns `None` for the prompt area (zero rows)
    and leaves the draft intact.
  - `visible_prompt_reserves_rows_with_a_non_empty_draft`: same render with `prompt_hidden = false`
    returns `Some(area)` (rows reserved).
  - `toggle_prompt_hidden_flips_and_persists_to_ui_kv`: toggling flips the flag and writes
    `prompt_hidden` to the ui kv; config seed and kv-override precedence tests pass; `/prompt` and
    `/prompt.toggle` resolve to `CommandAction::TogglePrompt`.
- result: PASS.
- Env: local `development` checkout, Linux, `SCOPEMUX_SKIP_NATIVE_BUILD=1`.

Merged closeout: PR #115 merged into `development`; remote and local feature branches deleted; local
`development` fast-forwarded to the merge commit.