---
id: "BUG-022"
title: "BUG: /thinking toggle shows a line count instead of the actual reasoning in real time"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "qa"
created: "2026-09-21"
---

# BUG: /thinking toggle shows a line count instead of the actual reasoning in real time

## Summary

The `/thinking` command (alias `/toggle-thinking`) is not useful in practice. Toggling it does not
surface the model's reasoning text; when thinking is shown, each reasoning block renders collapsed as
`▶ Thinking (N lines)` and only the line count is visible. The actual reasoning content never appears
while the model is thinking — it only becomes readable after a manual mouse click on each individual
block. The user's expectation is that showing thinking shows the thinking, live, as it streams.

## Reported behavior

- Running `/thinking` changes visibility but the user only sees the number of thinking lines.
- The real reasoning text is not displayed in real time as it streams in.
- The command therefore feels like it does nothing useful.

## Why this exists

Reasoning is the highest-signal part of a long agent turn. If the only display toggle for thinking
renders a count until the user clicks every block, the toggle fails its purpose and the daily-driver
workflow loses the ability to watch the model reason. This is a display correctness bug, not a
missing feature: the toggle claims to "Show/hide thinking blocks" (`command.rs:417`) but does not
reveal their content.

## Code evidence

- The command is registered as `Show/hide thinking blocks` and dispatched to a global flag:
  `crates/opencode-tui/src/command.rs:413-422` → `crates/opencode-tui/src/app/app.rs:1776-1778`.
- `AppContext::toggle_thinking` only flips the persisted `show_thinking` flag (ui key
  `thinking_visibility`); it never touches reasoning expansion:
  `crates/opencode-tui/src/context/app_context.rs:114`, `:151`, `:234-238`.
- The flag gates whether reasoning renders at all, and collapse is decided per part:
  `crates/opencode-tui/src/components/session.rs:520` (`show_thinking`),
  `:687-733` (reasoning render loop),
  `:697-704` (`collapsed = !self.expanded_reasoning.contains(&reasoning_id)`).
- `expanded_reasoning` starts empty and is per-part keyed `{msg.id}:{part_idx}`, so nothing is
  expanded by default: `crates/opencode-tui/src/components/session.rs:41`, `:59`, `:697`.
- When collapsed, `render_reasoning_part` returns early with only the count line, before the
  preview/content loop:
  `crates/opencode-tui/src/components/session_text.rs:41-56`.
  Because of that early return, `THINKING_PREVIEW_LINES` (`session.rs:24`) and the `preview_lines`
  parameter are effectively dead in the default collapsed state — collapsed means count only, not a
  preview.
- The only way to reveal content is a mouse click on the block header; there is no keyboard or
  command path to expand, and it is per-block:
  `crates/opencode-tui/src/components/session.rs:886-931` (`handle_click`).
- The reasoning id is stable during streaming, so an expanded block *would* update live; nothing
  auto-expands the in-progress part and the toggle does not set expansion.

Net effect: the toggle's two reachable states are "hidden" and "collapsed to a line count". Actual
reasoning text is never shown by the toggle itself.

## Suspected root cause

The feature conflates visibility with expansion. `show_thinking` (global show/hide) and
`expanded_reasoning` (per-part collapse) are independent, and the default for `expanded_reasoning`
is collapsed. The command only flips the former, so the user-visible result of "showing thinking" is
the collapsed count line rather than content.

## Scope

- Make `/thinking` reveal actual reasoning content, not just a count, when thinking is shown.
- Ensure reasoning that is currently streaming is readable in real time (the in-progress part should
  not require a click to be seen).
- Keep hiding behavior intact: toggling thinking off still hides reasoning.
- Keep per-block manual collapse/expand as an optional affordance, but it must not be the only way to
  read reasoning.
- Decide and document the intended default (expanded-when-shown vs. expanded-only-while-streaming).
- Persist any new state consistently with the existing `thinking_visibility` ui key.

## Non-goals

- Changing what the model receives or what is stored; this is a render-time display fix.
- Changing the `/thinking` command name, aliases, or category.
- Reintroducing the `THINKING_PREVIEW_LINES` preview behavior unless it is chosen as the intended
  default (it is currently unreachable when collapsed).
- Redesigning the reasoning block styling or theme.
- Tool-call compaction (`FEAT-028`); that is a separate display toggle.

## Done when

- Running `/thinking` to show thinking displays the actual reasoning text, not only `N lines`.
- Reasoning that arrives during an active turn is visible as it streams without a per-block click.
- Running `/thinking` to hide thinking still removes reasoning from the transcript.
- Manual collapse/expand, if kept, still works and does not fight the global toggle.
- The chosen default behavior is stated in the item Notes and reflected in the code.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, with a test covering the
  shown-state rendering path (content present, not just a count).

## Recommended verification

- `ort-build`, then `ort`; send a prompt that produces multi-line reasoning on the default model.
- Confirm that with thinking shown, the reasoning content is visible while it streams, not a count.
- Run `/thinking` to hide and confirm reasoning disappears; run it again and confirm content returns.
- If manual collapse/expand is retained, click a block and confirm it toggles without breaking the
  global visibility state.
- Restart the TUI and confirm the persisted visibility state matches the last toggle.
- Add a render test asserting the shown state emits reasoning text lines, not only the `N lines`
  header.

## Product decisions

- The point of `/thinking` is to read reasoning; a state that only shows a count does not satisfy it.
- Default should favor readability of live reasoning over a compact count, unless the user explicitly
  collapses a block.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count - same "collapse to
  a count" display pattern; the two toggles should have consistent, correct semantics.
- `FEAT-001` Improve historical chat transcripts workflow - reasoning visibility in transcripts.
- `PHASE-001` V1 daily-driver hardening.

## Notes

- Relevant files: `crates/opencode-tui/src/command.rs`,
  `crates/opencode-tui/src/app/app.rs`,
  `crates/opencode-tui/src/context/app_context.rs`,
  `crates/opencode-tui/src/components/session.rs`,
  `crates/opencode-tui/src/components/session_text.rs`.
- `crates/opencode-tui/src/components/thinking.rs` (`ThinkingBlock`) is a separate rendering path
  (collapsed defaults to true as well) and may need the same treatment for consistency; confirm which
  path is actually used by the session transcript before changing it.
