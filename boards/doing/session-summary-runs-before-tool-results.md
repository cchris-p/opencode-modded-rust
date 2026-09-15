---
id: "BUG-012"
title: "Session summary runs before tool results and breaks OpenAI-compatible continuation"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
created: "2026-09-15"
---

# Session summary runs before tool results and breaks OpenAI-compatible continuation

## Summary

After an assistant response emits tool calls, the live session prompt loop may run first-step post-processing before executing the tools. That can send a follow-up provider request while the message history contains assistant `tool_calls` without matching tool-result messages, which OpenAI-compatible chat providers reject.

## Reported behavior

An exported transcript captured a session where the assistant requested `grep` and `bash` tool calls, then subsequent `continue` prompts failed with:

`Provider error: API error: 400 Bad Request: {"error":{"message":"An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'. (insufficient tool messages following tool_calls message)"...}}`

Archived evidence: [`docs/archive/session-tool-call-summary-order-2026-09-15.md`](../../docs/archive/session-tool-call-summary-order-2026-09-15.md)

## Investigation - 2026-09-15

Root cause candidate:

- `crates/opencode-session/src/prompt.rs` finalizes the streamed assistant message with tool-call parts.
- Before the `has_tool_calls` branch executes those tools, the loop runs first-step title/session summary work.
- Title generation uses only first user text, but session summary converts the whole current session into model context.
- During that interval the session history is invalid for OpenAI-compatible chat continuation: assistant tool calls exist without corresponding tool results.

## Proposed fix

- Defer first-step title/session summary post-processing until after a non-tool assistant step, so no auxiliary provider request is made while tool calls are unresolved.
- Add focused regression coverage around the ordering guard.

## Related Items

- `BUG-004` Coding sessions run as bare chat
- `BUG-005` OpenAI-compatible chat providers reject requests once tools are attached
- `BUG-006` DeepSeek tool loop reasoning passback and split toolcall
- `BUG-008` batch tool is advertised but unusable on the session path

## Implementation Notes

- Added `SessionPrompt::should_run_first_step_postprocessing` and gated the first-step title/session summary block on `!has_tool_calls`.
- This prevents session summary generation from making an auxiliary provider request while the current assistant step has unresolved tool calls.
- Added a focused unit test for the guard.

## Verification

- `cargo test -p opencode-session first_step_postprocessing_waits_for_tool_free_step` — passed.
- `cargo test -p opencode-session` — 143 passed, 2 failed in `instruction::tests::{test_find_up_stops_at_stop_dir,test_find_up_walks_parents}`; failures are outside the changed prompt-loop code.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/34

## Merge status

- PR #34 opened against `development`; direct merge requested by user.
