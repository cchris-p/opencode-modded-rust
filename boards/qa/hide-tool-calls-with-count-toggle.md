---
id: "FEAT-028"
title: "Add a hide-tool-calls toggle that compacts runs to a tool-call count"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-09-21"
---

# Add a hide-tool-calls toggle that compacts runs to a tool-call count

## Summary

Add a display toggle command, modeled on `/thinking`, that hides tool-call rows in the session transcript and replaces each run of tool calls with a single compact summary line that shows how many tool calls occurred (for example `● 3 tool calls`). The toggle persists like the other display toggles and can be flipped from the command palette and slash command.

## Why this exists

Long agent turns produce many tool-call blocks (reads, greps, edits, bash runs) between the things a user actually wants to read. The transcript becomes a wall of tool output that buries the reasoning and the final answer. Thinking already solved this shape of problem: `/thinking` (aliases `/toggle-thinking`, `CommandAction::ToggleThinking` at `crates/opencode-tui/src/command.rs:413-422`) collapses reasoning into a single header, and reasoning rows render as `▶ Thinking (N lines)` when hidden (`crates/opencode-tui/src/components/session_text.rs:50`). Tool calls need the same treatment: one toggle that swaps the detail for a count so the turn stays legible and scannable.

This is distinct from the existing `show_tool_details` state (`crates/opencode-tui/src/context/app_context.rs:115`, `toggle_tool_details` at `app_context.rs:240-246`, ui key `tool_details_visibility`). That flag only removes arguments and output; individual tool-call rows are still rendered, and a completed call with `show_tool_details == false` becomes an empty `Vec<Line>` (`crates/opencode-tui/src/components/session_tool.rs:67-69`) rather than a counted summary. There is currently no user-facing command that collapses tool calls to a count, and `/thinking` has no tool equivalent.

## Scope

- Add a persisted display toggle for tool-call visibility, sibling to `show_thinking`:
  - New `RwLock<bool>` on `AppContext` and a `toggle_tool_calls` method that flips it and stores the value through `ui_kv` (mirror `show_thinking` / `toggle_thinking` at `crates/opencode-tui/src/context/app_context.rs:114`, `:151`, `:234-238`). Proposed ui key: `tool_calls_visibility`, default `true`.
  - Register a slash command analogous to `/thinking` (`crates/opencode-tui/src/command.rs:413`): proposed name `/tool-calls`, aliases `/tools` and `/toggle-tools`, category `Display`. Add a matching `CommandAction` variant next to `ToggleToolDetails` (`command.rs:73`) and handle it in the app command dispatch (`crates/opencode-tui/src/app/app.rs:1781-1785`).
  - Add the palette entry and dynamic title flipping ("Show tool calls" / "Hide tool calls") in `crates/opencode-tui/src/components/dialogs/command_palette.rs:117-135`, `:294`, `:330-333`.
  - Support an optional keybind key `tool_calls` in the keybind config schema, parallel to `tool_details` (`crates/opencode-config/src/schema.rs:205`) and `display_thinking` (`schema.rs:317`), wire it into the key handling at `crates/opencode-tui/src/app/app.rs:571-576`. Default: unbound.
- When hidden, collapse each contiguous run of tool calls inside an assistant message into one summary line instead of rendering each call, in the message assembly loop at `crates/opencode-tui/src/components/session.rs:740-781`.
  - Summary text must include the count, e.g. `3 tool calls`, with singular handling (`1 tool call`).
  - Preserve the existing state glyph/color semantics: show a running indicator while a call in the run is active, and keep the run visibly failed/denied when any call in it errored or was denied (do not let hidden failures silently disappear).
  - Keep a click/hit-target on the summary so it can be expanded like the thinking header, following the existing `thinking_toggle_hits` pattern (`session.rs:30-42`, `:687-733`, `:922`).
- Do not change what the model receives or what is stored; this is a render-time-only compaction.
- When visible, tool calls render exactly as they do today, so `show_tool_details` behavior is unchanged.

## Non-goals

- Replacing or changing the semantics of the existing `show_tool_details` / `tool_details_visibility` toggle.
- Hiding reasoning, text, or final assistant output.
- Changing session export contents or the `include_tool_details` export options (`crates/opencode-tui/src/app/app.rs:48`, `crates/opencode-tui/src/components/dialogs/session_export.rs`).
- Per-tool-type filtering, search, or a tool-call management UI.
- Changing server-side session storage or the message/part model.

## Done when

- A `/tool-calls` command exists (with the aliases above) in the slash command registry and command palette, shown under the Display/View category like `/thinking`.
- Running it hides tool-call rows and renders one summary line per contiguous run that states the number of tool calls.
- Running it again restores the current per-call rendering, and the toggled state survives a TUI restart via the ui key.
- A run containing an active call still shows an in-progress indicator, and a run containing a failed/denied call still surfaces that state while hidden.
- The optional `tool_calls` keybind, when configured, performs the same toggle as the slash command.
- `show_tool_details` continues to control arguments/output independently of the new visibility toggle.

## Recommended verification

- Run `ort-build`, then `ort`; start a session that produces several tool calls in one assistant turn.
- Confirm default rendering is unchanged (all tool calls visible with current detail behavior).
- Run `/tool-calls` and confirm each run collapses to a single `N tool calls` line, with correct singular/plural.
- Confirm the active-call indicator and failed/denied state remain visible while collapsed.
- Toggle back and confirm the full per-call view returns.
- Toggle off, exit and relaunch the TUI, and confirm the collapsed state was persisted.
- Configure the `tool_calls` keybind and confirm it toggles the same state.
- Toggle `show_tool_details` while tool calls are hidden and visible; confirm the two controls do not interfere.
- Add unit tests for the run-count summary, singular/plural, and error/active state surfacing in the render path.

## Product decisions

- Model the command, persistence, and palette behavior directly on `/thinking` so the two display toggles are consistent.
- The hidden representation is a count, not an empty block: `N tool calls` (singular `1 tool call`).
- Compaction is per contiguous run within a message, so a turn interleaving text and tool calls stays readable in order.
- Failures and active calls remain visible while collapsed; only successful detail is compacted away.
- Reuse the existing display-toggle code path rather than introducing a new settings surface.

## Related Items

- `FEAT-001` Improve historical chat transcripts workflow
- `FEAT-008` Disable TUI sidebar by default
- `PHASE-001` V1 daily-driver hardening

## Notes

- The existing toggle scaffolding to copy is: `CommandAction::ToggleThinking` (`crates/opencode-tui/src/command.rs:72`), the `/thinking` registration (`command.rs:413-422`), the handler branch (`crates/opencode-tui/src/app/app.rs:1781-1783`), and `AppContext::toggle_thinking` (`crates/opencode-tui/src/context/app_context.rs:234-238`).
- `render_tool_call` (`crates/opencode-tui/src/components/session_tool.rs:58`) currently decides between inline and block layouts per call; the collapse decision belongs one level up, where runs are assembled in `session.rs`, so a single summary can replace a whole run.
- Do not conflate this with `tool_details_visibility`: that keeps the call header and drops the body, while this drops the individual headers into a count.

## Implementation Notes

- Added persisted `tool_calls_visibility` state with `/tool-calls`, `/tools`, and `/toggle-tools` display commands plus command-palette labels.
- Added optional `tool_calls` keybind schema support and TUI key handling; no default keybind is registered.
- Collapsed hidden contiguous tool-call runs at render time into one `N tool calls` summary line, preserving running, failed, and denied state visibility; clicking a summary expands that run inline.
- Left `show_tool_details` behavior independent: when tool calls are visible, per-call rendering still uses the existing details toggle.
- Verification run: `cargo check -p opencode-tui -p opencode-config`; `cargo test -p opencode-tui tool_run_summary -- --nocapture`.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/72

## Merge Closeout - 2026-09-22

- PR #72 merged into `development` at `15201ae0edcf175c0cb4caf4567eabf79375ec91`.
- Remote PR branch `feature/feat-028-hide-tool-calls` was deleted by `gh pr merge --delete-branch`; the local PR branch is no longer present in the worktree used for implementation.
- Code/task completeness checked against this card before merge; item remains in `qa` pending post-merge QA report or explicit completion direction.
