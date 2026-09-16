---
id: "FEAT-012"
title: "CLI/AgentExecutor tool-loop parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-09-09"
---

# CLI/AgentExecutor tool-loop parity

## Summary

The `opencode run` / `github run` CLI path (`opencode-agent::AgentExecutor`) attaches the system prompt as a conversation `Role::System` message but never attaches tools to the request, and its streaming entrypoint (`execute_streaming`) issues a single model pass without running the tool-execution loop.

Evidence:

- `AgentExecutor::execute` / `execute_subsession` / `execute_streaming` all build `ChatRequest::new(...)`, which defaults `tools: None` (`crates/opencode-agent/src/executor.rs:164, 226, 274`; `crates/opencode-provider/src/message.rs:212-226`).
- The tool-looping `execute()` (with `ensure_tool_allowed` at `executor.rs:514-526`) has no production caller other than the subagent callback; the CLI drives `execute_streaming` only (`main.rs:1685, 5210`), which never executes tools.
- `Ask` permission hard-denies with no ask UI on this path (`executor.rs:514-526`); no `with_ask` callback is ever wired.

## Why this exists

`BUG-004` scopes the parity foundation to the live TUI/server session path and defers the CLI/`AgentExecutor` engine. The `run`/`github run` entrypoints are used for QA (`BUG-003`) and automation, and currently cannot behave agentically.

## Scope

- Attach the resolved, permission-filtered tool set to `AgentExecutor` requests.
- Run a real agentic loop on the CLI path: execute tool calls, feed results back, continue until the model stops, bounded by `max_steps`.
- Decide whether CLI tool execution routes through the existing `execute()` loop or through the session/server path (`FEAT-011` decision applies); do not permanently keep a tool-less streaming stub as the CLI's only behavior.
- Provide or reuse an ask/approval path instead of hard-denying `Ask` tools.

## Non-goals

- Full TUI feature parity in the CLI.
- Provider/transport changes (tracked by `FEAT-010`).

## Done when

- `opencode run "<prompt that requires a file read>"` issues and executes read/glob/grep/bash tool calls in a directory, not just a plain-chat reply.
- Ask-gated tools are not silently hard-denied without any user path.

## Recommended verification

- `cargo check -p opencode-agent -p opencode-cli`
- Live: `./target/debug/opencode run "List the files in this workspace and summarize them"` (workspace has a real provider configured).

## Related Items

- `PHASE-002` (phase parent)
- `BUG-004` Coding sessions run as bare chat
- `FEAT-011` Consolidate v1/v2 session prompt loops (decision affects CLI routing)
- `BUG-003` Session stops completely after first prompt (uses `opencode run` in QA)

## Notes

- Implement after `BUG-004` so the tool-set resolution logic is shared rather than duplicated.

## Dev Notes - 2026-09-16

- Implemented the interim `AgentExecutor` path rather than routing `opencode run` through the session/server runtime. The canonical QA path should remain `opencode serve` plus `/session/{id}/prompt` until `FEAT-011` or a follow-up routes `run` through the same session runtime.
- `AgentExecutor` now attaches permission-filtered tool definitions to provider requests when the active model supports tools, applies agent model params, and drives a bounded tool loop from `execute_streaming` instead of making a single tool-less model pass.
- Conversation conversion now preserves assistant `tool_use` parts and `tool_result` messages so executed tool output is fed back to the provider on the next pass.
- Streamed tool-call argument deltas are accumulated before JSON parsing, avoiding partial-JSON tool arguments.
- Ask-gated tools still return an explicit permission error on this direct executor path; they are no longer silently absent from the request, but this remains less capable than the TUI/session ask UI.
- Verification: `cargo test -p opencode-agent`; `cargo check -p opencode-agent -p opencode-cli`; `cargo build -p opencode-cli`.
- Live smoke attempted with `./target/debug/opencode run "List the files in this workspace and summarize them"` and `./target/debug/opencode run "Say hello in one sentence."`; provider completion is currently blocked by `401 Unauthorized: {"error":{"message":"User not found.","code":401}}` from the configured provider.

## Merge Notes - 2026-09-16

- PR #36 merged into `development`: https://github.com/cchris-p/opencode-modded-rust/pull/36
- Board item intentionally remains in `qa` for later post-merge verification rather than moving to `done`.
- Post-merge QA should run from `development` after configuring a valid provider credential, then retry the live `opencode run` smoke that was blocked by provider auth during implementation.
