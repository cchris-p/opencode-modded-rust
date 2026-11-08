---
id: "FEAT-052"
title: "Default home screen tips to hidden"
priority: "P3"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-09-22"
---

# Default home screen tips to hidden

## Summary

Hide the rotating tips line on the home/landing ("first") screen by default. Tips stay available but
only appear after the user explicitly enables them with `/tips.toggle` (or `/tips`). This flips the
persisted `tips_hidden` default from `false` to `true` so the first screen opens with just the prompt
and footer.

## Why this exists

The home screen renders a rotating tip line (`crates/opencode-tui/src/components/home.rs:109`,
`render_tips` at `home.rs:116`) whenever `should_show_tips()` is true. `tips_hidden` defaults to
`false` (`crates/opencode-tui/src/context/app_context.rs:147`), so any returning user sees tips on
the first screen by default. The user wants that first screen quiet unless they opt in.

Current behavior:

- `should_show_tips` = `!is_first_time_user && !tips_hidden` (`crates/opencode-tui/src/components/home.rs:258-262`).
- Toggle and persistence: `AppContext::toggle_tips_hidden` (`crates/opencode-tui/src/context/app_context.rs:190-194`), ui key `tips_hidden`.
- Command `/tips.toggle` with alias `/tips` (`crates/opencode-tui/src/command.rs:404`), handled at `crates/opencode-tui/src/app/app.rs:1820`, palette entry at `crates/opencode-tui/src/components/dialogs/command_palette.rs:111`.

## Scope

- Change the default value for `tips_hidden` from `false` to `true` at `crates/opencode-tui/src/context/app_context.rs:147`.
- Make `should_show_tips` (`crates/opencode-tui/src/components/home.rs:258`) controlled solely by `tips_hidden`, dropping the `is_first_time_user` gate so an explicit opt-in always shows tips, including on an empty session list.
- Keep the `/tips.toggle` command, alias, palette entry, persistence, and rotation behavior unchanged.

## Non-goals

- Removing tips from the home screen.
- Reclaiming the reserved 4-line tips layout slot (`crates/opencode-tui/src/components/home.rs:93`) when tips are hidden.
- Changing tip content, order, rotation interval, or the `{highlight}` markup.
- Changing any other display toggle or ui key.

## Done when

- A fresh TUI launch with no persisted `tips_hidden` value shows no tip line on the home screen.
- `/tips.toggle` (or `/tips`) shows tips and persists the choice; running it again hides them.
- After opting in, tips show even when the session list is empty.
- A previously persisted `tips_hidden` value is respected.

## Recommended verification

- `cargo check -p opencode-tui -p opencode-config`.
- Add a unit test covering the default (hidden) and the explicit toggle (shown), and confirm persistence through `ui_kv`.
- `ort-build` then `ort`: confirm no tip line on a clean home screen; `/tips.toggle` shows a tip; restart and confirm the toggled state persisted.
- `grep` for `is_first_time_user` to confirm no other consumer relies on the removed gate.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count — same display-toggle, persistence, and command-palette pattern.

## Notes

- The change is a default + gate adjustment to existing tips scaffolding, not new surfaces.
- `tips_hidden` is only read by `should_show_tips` and the palette title flip, so flipping the default has no other side effects.

## Implementation Notes

- Flipped the `tips_hidden` default at `crates/opencode-tui/src/context/app_context.rs:147` to a new `DEFAULT_TIPS_HIDDEN = true` constant (declared at `app_context.rs:14`), so a clean launch opens the home screen with tips hidden.
- Simplified `should_show_tips` (`crates/opencode-tui/src/components/home.rs:258`) to depend only on `tips_hidden` via a pure `tips_visible` helper, dropping the `is_first_time_user` session gate so `/tips.toggle` opt-in always shows tips.
- Kept `/tips.toggle` (alias `/tips`), the command palette entry, persistence, and rotation behavior unchanged.
- Verification run: `cargo check -p opencode-tui -p opencode-config`; `cargo test -p opencode-tui tips` (2 passed: `components::home::tests::tips_are_only_visible_when_not_hidden`, `context::app_context::tests::tips_default_to_hidden_when_unset`).

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/75



## Merge Closeout - 2026-09-22

- PR #75 merged into `development` at `c9b5d3a8cfecbaefd8215be780d126f17cd3d791`.
- Remote PR branch `feature/FEAT-052-default-home-tips-to-hidden` deleted; local PR branch deleted with `git branch -d`.
- Code/task completeness checked against this card before merge; item remains in `qa` pending post-merge QA report or explicit completion direction.
