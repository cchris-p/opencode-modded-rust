---
id: "FEAT-030"
title: "Make terminal tool output expandable on click in the session transcript"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-09-21"
---

# Make terminal tool output expandable on click in the session transcript

## Summary

Make terminal (`bash`/`shell`) tool-call blocks in the session transcript interactive: a command block renders as a visually distinct header plus a short preview, and clicking the block (its header or its `… (N more lines)` affordance) expands it to show the full captured output. Clicking again collapses it back to the preview. Expansion is per tool call, so several command blocks in one turn can be opened independently.

## Why this exists

Terminal commands are the part of a transcript users most often want to inspect in full, but today the TUI only ever shows a truncated preview and gives no way to reveal the rest. In block mode (`render_tool_call`, `crates/opencode-tui/src/components/session_tool.rs:58`), a `bash`/`shell` call renders its header, then `(N lines of output)`, then at most `preview_limit` lines (10 for bash, `session_tool.rs:136-140`), then a passive `… (M more lines)` line (`session_tool.rs:158-162`). There is no interaction on those rows.

The only existing control is the global `show_tool_details` toggle (`crates/opencode-tui/src/context/app_context.rs:115`, `toggle_tool_details` at `app_context.rs:240-246`). That is a display-wide switch, and even when it is on the output is still capped at the preview limit, so it cannot reveal the remainder of a long command. Meanwhile reasoning already solved this exact shape of problem: reasoning parts render collapsed and expand on click via `expanded_reasoning` (`crates/opencode-tui/src/components/session.rs:41`), `ThinkingToggleHit` (`session.rs:30-33`), hit regions recorded at `session.rs:741-752`, and `handle_click` toggling at `session.rs:925-953` (dispatched from `crates/opencode-tui/src/app/app.rs:708`). Terminal blocks need the same click-to-expand affordance, plus clearer visual structure so a command block reads as an inspectable object rather than a passive log excerpt.

This is distinct from `FEAT-028` (hide tool calls / collapse to a count). FEAT-028 removes or counts runs of calls; this item keeps each call visible and controls how much of a single call's output is shown. The two must coexist without interfering.

## Scope

- Add per-tool-call expansion state, keyed by the tool call id, in `SessionView`, mirroring `expanded_reasoning` (`crates/opencode-tui/src/components/session.rs:41`, initialized `:59`, retained to visible ids `:887-888`). Session-local, not persisted.
- Give block-style tool calls a click hit region, reusing the `ThinkingToggleHit` pattern (`session.rs:30-33`): a new hit type recording `line_index` plus tool call id, a `tool_toggle_hits: Vec<...>` list cleared at the start of each render (parallel to `thinking_toggle_hits.clear()` at `session.rs:556`), populated in the tool-call render loop (`session.rs:759-801`). To do this cleanly, `render_tool_call` must report which output lines belong to the call, or expose an expandable/`collapsible` signal like `render_reasoning_part` does (`session.rs:719-753`).
- Extend `render_tool_call` (`session_tool.rs:58`) to accept the per-call expanded flag and render the full output when expanded, instead of capping at `preview_limit` (`session_tool.rs:149-162`). Collapsed behavior stays the current preview; expanded shows all `result_text` lines.
- Visually enhance the block presentation:
  - Make the header line read as a command block (state glyph + `$ command` clearly grouped, using existing theme tokens such as `theme.tool_icon`, `theme.background_panel`, `theme.border_subtle`).
  - Show an explicit expand/collapse indicator on the collapsible row (for example `▸`/`▾` or `▼`/`▲`), and make it change state when expanded.
  - Make the `(N lines of output)` / `… (M more lines)` affordance clearly actionable (for example `▸ N lines hidden — click to expand` collapsed, `▾ click to collapse` expanded).
- Make the toggle clickable from the block header and from the more-lines affordance, following `handle_click` (`session.rs:925-953`) and the left-button dispatch at `app.rs:699-710`. A successful hit toggles the id's expanded state and returns `true` so the click is not also treated as a text selection start.
- Preserve existing semantics and layout: the completed-and-`show_tool_details`-off early return (`session_tool.rs:67-69`), the error branch (`session_tool.rs:112-135`), inline (non-block) tools, non-bash block tools, selection, scrolling, and thinking toggles.
- Allow full error output to be expanded too (error branch currently shows one line plus up to two more when details are on, `session_tool.rs:112-135`).

## Non-goals

- Server-side or storage changes; this is render- and interaction-only, using output already held in `tool_results`.
- Persisting expansion state across TUI restarts or storing it in `ui_kv`.
- A global "expand all / collapse all" command, keybind, or slash command.
- Full terminal emulation, ANSI color rendering, or syntax highlighting of command output.
- Changing what `show_tool_details` / `tool_details_visibility` means, or its default.
- Changing `FEAT-028`'s run-count compaction behavior.
- Changing the inline rendering path or non-block tools beyond whatever is needed for type/signature consistency.

## Done when

- A completed `bash`/`shell` block renders a distinct header and a collapsed preview, with a visible expand affordance.
- Clicking the block header or the more-lines affordance expands it to show the complete captured output; clicking again returns to the preview.
- Multiple terminal calls in one turn expand and collapse independently, and their state survives normal re-renders (streaming, resize, scroll) while the call remains in the transcript.
- The header/affordance visually distinguishes collapsed from expanded state.
- A failed/error terminal call can be expanded to show its full error output.
- With `show_tool_details` off, no output body or expand affordance is shown for a completed call (existing behavior preserved); disabling it while a call is expanded degrades cleanly.
- Existing reasoning click-to-expand, text selection, mouse scrolling, and sidebar interaction are unaffected.

## Recommended verification

- Run `cargo fmt --check -p opencode-tui`, `cargo check -p opencode-tui`, and `cargo test -p opencode-tui`.
- Run `ort-build`, then `ort`; start a session that runs a command producing more than 10 lines of output (for example a build, test run, or `ls` of a large tree).
- Confirm the collapsed block shows the preview plus the expand affordance, and that clicking it expands to the full output.
- Confirm clicking again collapses, and that the indicator flips state.
- Start a turn with two or more long-running commands and confirm they expand independently.
- Run a failing command and confirm the error can be fully expanded.
- Toggle `show_tool_details` off and on and confirm the block body and affordance behave consistently and do not break the header.
- Confirm clicking a command block does not start a text selection, and that clicking reasoning still toggles reasoning.
- Confirm scrolling, resize, and session switching do not leave stale hit regions toggling the wrong block.
- Add unit tests for the expanded-vs-collapsed line output and for the hit-region/toggle mapping in the render path.

## Product decisions

- Reuse the reasoning expand/collapse mechanism rather than inventing a new interaction, so both collapsible transcripts behave the same.
- Expansion state is session-local and in-memory, matching `expanded_reasoning`; it is not persisted to `ui_kv`.
- Collapsed is the default, preserving the current preview-oriented transcript density.
- Expansion is per individual tool call, not per run of calls.
- The affordance is explicit text plus a state glyph, so the block is discoverable by mouse without a hover system.
- The block keeps its existing panel background and border vocabulary; the enhancement is emphasis and an indicator, not a new layout system.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count - same tool-call render surface (`session.rs:759-801`, `session_tool.rs`); the two features must not conflict.
- `FEAT-001` Improve historical chat transcripts workflow - both target transcript readability.
- `PHASE-001` V1 daily-driver hardening - terminal output inspection is part of trusting a daily-driver session.

## Notes

- The block decision point is `is_block_tool` (`crates/opencode-tui/src/components/session_tool.rs:42-55`): `bash`, `shell`, and `apply_patch` are always block tools; others become block tools when output exceeds `BLOCK_RESULT_THRESHOLD` (`session_tool.rs:20`).
- The current truncation is entirely inside `render_tool_call` (`session_tool.rs:136-162`); expansion belongs there, while hit-region bookkeeping and toggle state belong one level up in `session.rs`, mirroring how `render_reasoning_part` returns a `collapsible` signal that `session.rs` turns into hits (`session.rs:719-753`).
- `render_tool_call` currently returns `Vec<Line<'static>>` with no metadata. Either change its return type to include collapsible/line-span info, or compute collapsibility in `session.rs` from `is_block_tool` and the output length before calling it.
- The mouse path is already wired for session clicks (`app.rs:699-710`); the new hits only need to be consulted alongside `thinking_toggle_hits` in `handle_click`, and the click must be consumed so it does not also seed a selection (`app.rs:708-710`).
- `thinking_toggle_hits` is cleared at `session.rs:556` before the message loop; `tool_toggle_hits` should be cleared in the same pass and line indices must stay consistent with the `Paragraph` scroll used at `session.rs:901-907`.
- Keep the `expanded_reasoning.retain(...)` pruning pattern in mind (`session.rs:887-888`) so expansion state for calls no longer present is dropped; the analogous pruning applies to expanded tool calls.

## Dev Notes

- Implemented in PR #63 (`feature/FEAT-030-terminal-tool-output-expandable`).
- `render_tool_call` now returns `ToolCallRender { lines, collapsible }` and takes an `expanded` flag. The block header renders as `│ ● $ <command>` with a `▸`/`▾` indicator; collapsed preview limits are unchanged (10 shell / 6 other); the affordance row is `▸ N more lines — click to expand` / `▾ click to collapse` styled with `theme.info`; error output expands fully.
- `session.rs` adds `expanded_tool_calls`, `tool_toggle_hits`, and `ToolToggleHit`. Header and tail hit regions are recorded per collapsible call, keyed by tool call id; `handle_click` toggles tool calls before reasoning hits; state is pruned to visible ids each render and stays session-local.
- Verification: `cargo fmt --check -p opencode-tui` clean; `cargo check -p opencode-tui` clean; `cargo test -p opencode-tui` 55 passed / 0 failed, including 4 new `session_tool` tests (long-output collapse + expand, short-output non-collapsible, hidden-details early return, error expansion).
- Manual TUI verification (`ort-build` / `ort`) was not performed in this pass.

## QA Notes

- Pending local verification on the PR branch: collapsed preview plus affordance, expand/collapse toggling and indicator flip, independent expansion of multiple calls in one turn, error expansion, and `show_tool_details` off/on behavior.

## Completion

- PR #63 merged into `development` (`fbbbc82`); feature branch `feature/FEAT-030-terminal-tool-output-expandable` deleted remotely and locally.
- QA evidence: `cargo fmt --check -p opencode-tui` clean; `cargo check -p opencode-tui` clean; `cargo test -p opencode-tui` 55 passed / 0 failed, including 4 new `session_tool` tests (long-output collapse + expand, short-output non-collapsible, hidden-details early return, error expansion).
- Manual TUI verification (`ort-build` / `ort`) was not run; the card was completed on automated QA plus explicit user direction to close out.