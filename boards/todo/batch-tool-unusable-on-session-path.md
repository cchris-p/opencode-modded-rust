---
id: "BUG-008"
title: "batch tool unusable on the session path (schema mismatch + registry not wired)"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-10"
---

# batch tool unusable on the session path (schema mismatch + registry not wired)

## Summary

The `batch` tool is advertised to the model but cannot succeed in a live coding session. Two independent defects combine: (1) the advertised parameter schema uses `toolCalls` while the deserializer expects `tool_calls`, and (2) the session prompt loop builds the tool `ToolContext` without the tool registry, so batch execution always errors.

## Reported behavior

From session `ses_0ffc9b44f96944039f8621c4d980d11b` (exported `summarize-workspace-files.md`), the model made three `batch` attempts:

1. Called `batch` with `{"toolCalls":[...]}` (following the advertised schema) → `Error: Invalid arguments: Invalid parameters: missing field tool_calls`.
2. Repeated the same call → same error.
3. Switched to `{"tool_calls":[...]}` → `Error: Execution error: Tool registry not available. Batch execution requires registry access.`

The model then abandoned `batch` and read files individually. This wastes turns and makes a prominent tool look broken.

## Root cause (evidence)

1. **Schema/deserializer mismatch.**
   - `BatchTool::parameters()` advertises `"toolCalls"` (`crates/opencode-tool/src/batch.rs:56`, required `toolCalls` at `batch.rs:77`).
   - `BatchParams` fields are `tool_calls: Vec<ToolCall>` with no serde rename (`crates/opencode-tool/src/batch.rs:11-14`), so serde expects `tool_calls`.
2. **Registry not wired on the session path.**
   - The session prompt loop creates `ToolContext` with no `.with_registry(...)` (`crates/opencode-session/src/prompt.rs:1338-1348`).
   - `BatchTool::execute` requires `ctx.registry`, else returns `Tool registry not available` (`crates/opencode-tool/src/batch.rs:99-107`).
   - The CLI path does wire it (`crates/opencode-cli/src/main.rs:3824`), proving the intended wiring exists but was never applied on the server/session path.
3. **Related:** the same session tool context uses `.with_agent(String::new())` (`prompt.rs:1347`), so agent identity is empty for tools/permission logic that keys on it.

## Scope

- Fix the `batch` parameter schema/deserializer so the advertised keys match what the tool accepts (pick one canonical case and make schema, struct, and tests agree).
- Wire the `ToolRegistry` into the `ToolContext` on the session prompt path (server/TUI) so `batch` (and any tool needing `ctx.registry`) can execute.
- Set the resolved agent name on the session `ToolContext` instead of an empty string.
- Add a regression test that drives `batch` through the session prompt path (not just the CLI path) and asserts it executes contained tool calls.

## Non-goals

- Adding new batch/code-mode capabilities.
- Broad parity with the reference `code-mode` tool.

## Done when

- Calling `batch` with the schema-advertised parameter shape executes successfully in a live TUI/server session.
- `ToolContext` on the session path includes the registry and the resolved agent name.
- A session-path regression test covers batch execution.

## Recommended verification

- `cargo test -p opencode-tool -p opencode-session`
- Live: fresh server, prompt the agent to read several files at once and confirm `batch` succeeds instead of erroring.

## Related Items

- `BUG-004` Coding sessions run as bare chat
- `BUG-007` ls tool returns partial/misleading directory listings (same session-tool-context area)
- `PHASE-001` V1 daily-driver hardening

## Notes

- Surfaced during BUG-006 QA; see `summarize-workspace-files.md`.
- `batch` is Rust-introduced; the reference has no `batch` tool. Confirm whether `batch` should remain exposed to the model.
