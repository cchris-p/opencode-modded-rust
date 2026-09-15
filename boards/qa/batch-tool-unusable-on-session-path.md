---
id: "BUG-008"
title: "batch tool is advertised but unusable on the session path; remove it from the default tool set"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-09-10"
---

# batch tool is advertised but unusable on the session path; remove it from the default tool set

## Summary

The `batch` tool is advertised to the model but cannot succeed in a live coding session. It is also a Rust-introduced tool with no reference counterpart. Rather than fix it, this item **removes `batch` from the default tool set** so the model is no longer offered a tool that always fails.

## Reported behavior

From session `ses_0ffc9b44f96944039f8621c4d980d11b` (exported `summarize-workspace-files.md`), the model made three `batch` attempts:

1. Called `batch` with `{"toolCalls":[...]}` (following the advertised schema) → `Error: Invalid arguments: Invalid parameters: missing field tool_calls`.
2. Repeated the same call → same error.
3. Switched to `{"tool_calls":[...]}` → `Error: Execution error: Tool registry not available. Batch execution requires registry access.`

The model then abandoned `batch` and read files individually. This wastes turns and makes the tool surface look broken.

## Root cause (evidence, retained for history)

1. **Schema/deserializer mismatch.** `BatchTool::parameters()` advertises `"toolCalls"` (`crates/opencode-tool/src/batch.rs:56`, required at `batch.rs:77`), while `BatchParams` expects `tool_calls` (`batch.rs:11-14`).
2. **Registry not wired on the session path.** The session prompt loop builds `ToolContext` without `.with_registry(...)` (`crates/opencode-session/src/prompt.rs:1338-1348`); `BatchTool::execute` requires `ctx.registry` (`batch.rs:99-107`). Only the CLI path wires it (`crates/opencode-cli/src/main.rs:3824`).
3. `batch` is the only tool in the codebase that reads `ctx.registry`.

## Decision

- **Remove `batch` (2026-09-13).** `batch` is non-reference (reference has `code-mode`, not `batch`), it is the only consumer of `ToolContext.registry`, and the useful capability (running independent tool calls together) is already available through **parallel tool calls in a single assistant message**, which the prompt templates encourage. Removing it is smaller and safer than maintaining a divergent tool.
- A prior decision (2026-09-10) was to keep and fix `batch`; that is superseded by this removal decision.

## Scope

- Remove `batch` from the default tool registry and from the advertised tool list.
- Remove the `batch` tool module.
- Remove the now-dead `experimental.batch_tool` config flag.
- Keep the `ToolContext.registry` / `with_registry` plumbing and the existing CLI wiring intact (harmless; useful for future multi-tool orchestration).
- Do not touch unrelated `batch` references (`opencode-mcp` `register_batch`, prompt-template "batch your tool calls" text).

### Exact removal sites

- `crates/opencode-tool/src/registry.rs:248` — remove `registry.register(crate::batch::BatchTool).await;`
- `crates/opencode-tool/src/registry.rs:9` — remove `"batch"` from `FILTERED_FROM_SUGGESTIONS`.
- `crates/opencode-tool/src/lib.rs:3` — remove `pub mod batch;`.
- `crates/opencode-tool/src/batch.rs` — delete the file (no tests; referenced only by `registry.rs`).
- `crates/opencode-server/src/routes.rs:4402` — remove `"batch"` from `list_tool_ids()`.
- `crates/opencode-config/src/schema.rs:662` — remove `pub batch_tool: Option<bool>`.
- `crates/opencode-config/src/schema.rs:1169` — remove the `batch_tool` merge line.

## Non-goals

- Re-implementing batch/code-mode capabilities.
- Broad parity with the reference `code-mode` tool.
- Removing `ToolContext.registry` plumbing.

## Done when

- `batch` is no longer registered, advertised, or reachable from a session; the model cannot call it.
- No dangling references to the removed module remain (`registry.rs`, `lib.rs`, server tool list, config schema).
- `cargo check`/tests pass for the affected crates with no new warnings from the removal.

## Recommended verification

- `cargo check -p opencode-tool -p opencode-server -p opencode-config`
- `cargo test -p opencode-tool -p opencode-server`
- Live: fresh server, confirm `batch` is no longer offered and the agent uses parallel `read` calls instead.

## Related Items

- `BUG-004` Coding sessions run as bare chat
- `BUG-007` ls tool returns partial/misleading directory listings
- `PHASE-001` V1 daily-driver hardening

## Notes

- Surfaced during BUG-006 QA; see `summarize-workspace-files.md`.
- Decision 2026-09-13: remove `batch` (supersedes the 2026-09-10 keep-and-fix decision).

## Dev Notes

- Removed `batch` from the default registry (`crates/opencode-tool/src/registry.rs`): dropped `registry.register(crate::batch::BatchTool)` and removed `"batch"` from `FILTERED_FROM_SUGGESTIONS`.
- Removed `pub mod batch;` from `crates/opencode-tool/src/lib.rs` and deleted `crates/opencode-tool/src/batch.rs`.
- Removed `"batch"` from the advertised tool list in `crates/opencode-server/src/routes.rs` (`list_tool_ids`).
- Removed the dead `experimental.batch_tool` config field and its merge line in `crates/opencode-config/src/schema.rs`.
- Kept `ToolContext.registry` / `with_registry` and the CLI wiring (per decision; useful for future orchestration).
- No remaining references to `BatchTool`, `batch::`, `batch_tool`, or `"batch"` in `crates/` (verified by grep).

## Verification

- `cargo check -p opencode-tool -p opencode-server -p opencode-config` — clean.
- `cargo test -p opencode-tool` (22), `cargo test -p opencode-server --lib` (11), `cargo test -p opencode-config` (51 + 4) — all pass.
- Live server (`development` binary): `GET /experimental/tool/ids` returns 19 tools and `batch present: False`; the model is no longer offered `batch`.
- Note: the 4 new `ls` regression tests are on the `BUG-007` branch and not part of this branch's test count.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/32 (branch `bug/BUG-008-remove-batch-tool`, base `development`)
