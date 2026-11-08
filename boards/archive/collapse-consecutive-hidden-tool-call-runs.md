---
id: "FEAT-063"
title: "Collapse consecutive tool-call runs into one count when tool calls are hidden"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "archived"
created: "2026-09-24"
updated: "2026-09-24"
---

# Collapse consecutive tool-call runs into one count when tool calls are hidden

## Archived

Archived 2026-09-24 at user request; kept for later, not scheduled. The behavior is a refinement of FEAT-028 (`hide-tool-calls-with-count-toggle`).

## Summary

When tool calls are hidden (`show_tool_calls == false`), a multi-step agent turn that issues one tool call per step renders a vertical stack of separate `1 tool call` summary lines instead of a single line reporting the total. The hidden representation should coalesce the whole consecutive run of tool calls into one summary line that shows how many calls occurred (for example `● 5 tool calls`), and expanding that summary should still reveal each individual tool call as it renders today.

## Why this exists

Feature FEAT-028 added a hide-tool-calls toggle that replaces a contiguous run of tool calls with a count line. The grouping is done per assistant message: the render loop scans `msg.parts` and only absorbs immediately adjacent `MessagePart::ToolCall` parts into one run (`crates/opencode-tui/src/components/session.rs:878-951`, run-end scan at `session.rs:881-886`, summary at `session.rs:890-912`).

The agent loop does not put all calls of a turn in one message. Each LLM step produces its own assistant message holding that step's tool calls (`crates/opencode-session/src/prompt.rs:1416-1430`), and tool execution appends the results as a separate assistant message (`execute_tool_calls` pushes a new `SessionMessage` of `PartType::ToolResult` parts at `prompt.rs:1854-1878`). Sessions confirm this shape: messages are typically `['reasoning','toolCall']` followed by `['toolResult']`, repeating once per step.

Because a run never crosses a message boundary, a turn that calls one tool per step (the common DeepSeek pattern) produces one `1 tool call` line per step, directly adjacent with no separating blank line (consecutive assistant messages get no spacing at `session.rs:684-689`). The result is a stack of `1 tool call` lines instead of the single count the toggle was meant to produce.

## Scope

- In the session render path, coalesce a maximal sequence of consecutive tool-call messages into one hidden summary line with the total count, rather than one line per message.
  - A "tool-call message" is an assistant message whose visible content, under the current `show_tool_calls` / `show_thinking` flags, is only tool-call summaries (individual calls) — i.e. an assistant message containing `ToolCall` parts with no visible text/file/reasoning.
  - Intervening result-only assistant messages render nothing while hidden (`session.rs:984`) and must not break a run.
  - Hidden reasoning blocks must not break a run; visible reasoning (`show_thinking == true`) should break it, consistent with run grouping following what is actually visible.
  - Visible assistant text must break the run, so an answer interleaved between tool groups keeps each group separate.
- Preserve state aggregation across the merged run: running wins while any call is in flight, failed/denied wins once any call errored or was denied, and the singular/plural label reflects the merged total (`1 tool call` vs `N tool calls`; see `render_tool_run_summary` at `crates/opencode-tui/src/components/session_tool.rs:20-52`).
- Keep the summary clickable. The run id / hit region must be stable across re-renders and keyed to the start of the run (currently `{msg_id}:tools:{run_start}` at `session.rs:887-912`, stored in `expanded_tool_calls`).
- Expanding the merged summary (and toggling `/tool-calls` back on) must show every individual call in the run, rendered by the existing per-call path (`render_tool_call_part` at `session.rs:1386-1408`).
- Keep `show_tool_details` behavior independent, exactly as today.
- Render-time only: do not change stored messages or the part model.

## Non-goals

- Changing how the agent loop stores tool calls or results.
- Changing the `/tool-calls`, `/tools`, `/toggle-tools` command surface, its ui key (`tool_calls_visibility`), or the optional keybind.
- Changing `show_tool_details` / `tool_details_visibility` semantics.
- Per-tool filtering or a tool-call management UI.

## Done when

- With tool calls hidden, a multi-step agent turn that issues one call per step shows a single `N tool calls` summary line for the whole consecutive run, not N stacked `1 tool call` lines.
- The run breaks only at visible non-tool content (assistant text; visible reasoning when thinking is shown).
- The merged summary shows running / failed / denied state and the correct singular/plural total.
- Clicking the merged summary, or toggling tool calls back on, reveals each individual tool call.
- `show_tool_calls` on and `show_tool_details` behavior are otherwise unchanged.
- Unit tests cover cross-message run merging, run breaking on visible text, and state aggregation across the merged run.

## Recommended verification

- `cargo test -p opencode-tui session`
- `ort-build` then `ort`; run a session that issues several tool calls in sequence (one per step) and toggle `/tool-calls` off. Confirm a single count line replaces the stack, that a turn with an interleaved final answer keeps the answer and does not merge across it, and that expanding the summary lists each call.
- Open a historical session of the same shape and confirm the collapsed view matches.

## Constraints / risks

- Cross-message grouping does not fit the current single-pass, per-message loop cleanly; the run must be detected over the message list (a pre-pass or a carried-over "open run" accumulator) before per-message rendering.
- The per-message `tool_results` map (`session.rs:718-728`) only sees results stored in the same message. Since `execute_tool_calls` stores results in a separate assistant message (`prompt.rs:1854-1878`), confirm during implementation whether expanding an individual call currently resolves its output and state; if it does not, build a session-wide `tool_results` map (or otherwise associate results across messages) so expanded calls show output. Do not regress the running-call indicator.
- Expansion state is keyed by run id; a merged run that starts at an earlier message must use a deterministic id anchored to the first message of the run so clicks survive re-syncs.

## Notes

- FEAT-028: `boards/done/hide-tool-calls-with-count-toggle.md` (PR #72).
- Grouping logic to extend: `crates/opencode-tui/src/components/session.rs:878-951`.
- Summary rendering: `crates/opencode-tui/src/components/session_tool.rs:20-52`.
- Message loop and spacing: `crates/opencode-tui/src/components/session.rs:681-689`, `:713-728`.
- Agent loop step/message shape: `crates/opencode-session/src/prompt.rs:1416-1430`, `:1854-1878`.
- Evidence: recent sessions store patterns like `['reasoning','toolCall']` then `['toolResult']` per step (for example `ses_8f8bd514430d428da7a8aa50ba09e0f5`, `ses_ab9f3317b0c94d74b4fc7cbe505ff937`), which the current per-message grouping renders as stacked `1 tool call` lines.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count
- `FEAT-001` Improve historical chat transcripts workflow
