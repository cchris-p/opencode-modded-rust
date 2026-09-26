---
id: "BUG-037"
title: "Slash menu freezes keyboard input until terminal resize"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "done"
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
## Root Cause (2026-09-25)

Reproduced on `development` by running the rebuilt TUI locally and pressing `/`.

- The menu was rendered against the whole frame instead of the prompt rect: `draw()` passed
  `frame.size()` to `SlashCommandPopup::render`, and `render` computes its top as
  `area.y - (height + 1)`. With `area.y == 0` the menu was pinned to the top of the screen,
  far from the prompt at the bottom, so pressing `/` looked like it did nothing and the menu
  appeared to vanish.
- Key routing then sent every typed character into that top-anchored menu query, so the input
  line stayed empty while the user typed. The event loop was alive, but the UI read as frozen.
- The popup was counted in `has_open_dialog_layer()`, so opening it also painted the
  full-screen modal backdrop, dimming the prompt and reinforcing the disabled/frozen look.
- While the menu was open, `handle_dialog_key` swallowed every key, including `Ctrl-C` and
  `Ctrl-D`, removing the usual global escapes; `Ctrl-D` stopped quitting.
- Resizing forces a full redraw/relayout, which is why a resize could appear to restore the UI.

The behavior has existed since the popup was introduced (`e937c3c publish`); it is not caused
by the provider-list change.

## Dev Notes

- `crates/opencode-tui/src/components/session.rs`: `render`/`render_main` now return the
  rendered prompt `Rect` (`None` when the prompt is hidden) so overlays can anchor to the input.
- `crates/opencode-tui/src/components/home.rs`: `render`/`render_with_prompt` return the prompt
  `Rect`.
- `crates/opencode-tui/src/components/slash_command.rs`: `render` takes the prompt rect, clamps
  to the frame, and falls back below the prompt when there is no room above it.
- `crates/opencode-tui/src/app/app.rs`: `draw()` passes the prompt rect to the menu (falling back
  to the bottom edge when the prompt is hidden); the inline menu no longer triggers the
  full-screen modal backdrop (`has_modal_dialog_layer`); and `Ctrl-C`/`Ctrl-D` pass through while
  the menu is open, with `Ctrl-C` also closing it.

## Verification

- `cargo test -p opencode-tui` — 157 passed, including 3 new menu tests (bare `/` lists suggested
  commands, reopening resets query/selection, and the menu renders directly above the prompt).
- `cargo check --workspace` (with `SCOPEMUX_SKIP_NATIVE_BUILD=1`).
- `cargo clippy -p opencode-tui --all-targets` — no new warnings.
- Manual (Linux, rebuilt binary): `/` opens the menu directly above the prompt on both the home
  and in-session surfaces; `Esc` closes it and returns focus (typing lands in the input);
  `Ctrl-C` closes the menu and clears the prompt; `Ctrl-D` still exits while the menu is open.
  Prompt submission was not exercised locally (no model call); a manual pass on iTerm2 is pending.

## PR / Merge

- PR #114 (`bug/BUG-037-slash-menu-freeze`) merged into `development` via merge commit `c6fb3af`
  (2026-09-26); the remote and local feature branches were deleted.
- The PR also merged `origin/development` to resolve a conflict with BUG-042 in the slash-menu
  test module; the merged tree reran `cargo test -p opencode-tui` — 159 passed, 0 failed.
- Item remains in `qa` until a QA report is recorded or completion is explicitly directed.

## QA Report (agent, 2026-09-26)

Performed by the agent on the merged `development` tree; no human testing.

- Artifact: `development` @ `fbe3e7d` (contains BUG-037 merge `c6fb3af`).
  Binary: `$HOME/worktrees/opencode-modded-rust/.shared-target/debug/opencode`
  (built with `cargo build -p opencode-cli --no-default-features`, `SCOPEMUX_SKIP_NATIVE_BUILD=1`;
  the native feature is offline-only and untouched by this TUI change).
- Deterministic signal: `cargo test -p opencode-tui` on the merged tree — 159 passed, 0 failed,
  including the new `slash_command::tests::popup_renders_above_the_prompt_area` render-anchor test.
  (A first run showed 2 pre-existing `prompt` test flakes: an ordering-sensitive
  `tab_autocomplete_uses_first_candidate` mismatch, plus a poisoned env-mutex cascade in
  `utf8_backspace_delete_and_cursor_are_char_safe`. The flaky test passes in isolation and the
  full suite passes on re-run; both are unrelated to this change.)
- Interactive corroboration on the built binary (tmux 120x40, home surface):
  - `/` opens the menu directly above the prompt — top border at row 29, prompt input at row 41,
    nothing at row 1 (previously pinned to row 1).
  - Typing filters the menu (`h` -> `Help`).
  - `Esc` closes the menu and returns focus; typing `qa037` lands in the prompt input.
  - `Ctrl-C` closes the menu and clears the prompt.
  - `Ctrl-D` exits the TUI while the menu is open (pane returned to the shell).
- Result: PASS. iTerm2 itself was not available in this environment, but the defect (frame-level
  anchoring plus app-level key routing) is terminal-independent and was reproduced and fixed
  locally.
