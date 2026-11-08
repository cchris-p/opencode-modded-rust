---
id: "CLI-002"
title: "Route opencode run through the canonical session runtime"
priority: "P2"
type: "feature"
area: "CLI"
spec: "invariants/coding-session-behavior.md"
status: "hold"
predecessors: "CLI-001, CLI-006"
created: "2026-09-09"
updated: "2026-09-22"
---

# Route `opencode run` through the canonical session runtime

## Blocked By - 2026-09-22

- `CLI-001` Copy Cline-style CLI task send conventions (prerequisite gate: canonical non-TUI task surface).
- `CLI-006` Add CLI status visibility for tasks and background sessions (prerequisite gate).
- `FEAT-011` Consolidate v1 and v2 session prompt loops (this card's routing decision overlaps that
  consolidation).

Do not start until `CLI-001` and `CLI-006` land. Ask/approval parity for the CLI path was split out to
`CLI-009`.

## Summary

The `opencode run` / `github run` CLI path currently drives `opencode-agent::AgentExecutor`, a parallel
model loop separate from the canonical server/session runtime. `CLI-001` establishes the canonical non-TUI
path (`opencode task new`/`send`/`view` through `/session/{id}/prompt`). Once that exists, `opencode run`
should route through the same session runtime rather than maintaining a second CLI tool loop.

This card owns that routing/consolidation decision and the resulting `opencode run` tool-loop behavior. It
no longer owns the ask/approval question path; see `CLI-009`.

## Why this exists

`BUG-004` scoped the parity foundation to the live TUI/server session path and deferred the CLI engine. An
interim `AgentExecutor` tool loop was later added, but it duplicates the canonical session loop and drifts
from it. With `CLI-001` providing the canonical CLI surface, a second CLI agent loop is redundant work.

## Scope

- Decide whether `opencode run` / `github run` route through the canonical session/server prompt runtime, and
  remove the parallel `AgentExecutor` tool loop if so (coordinate with `FEAT-011`).
- Preserve `run`'s automation/QA use cases (`BUG-003`, `BUG-004`) on the canonical path.
- Do not permanently keep a second CLI model loop alongside `SessionPrompt`.

## Non-goals

- Full TUI feature parity in the CLI.
- Provider/transport changes (tracked by `FEAT-010`).
- Ask/approval question parity is owned by `CLI-009` (CLI and direct-run question parity).

## Done when

- `opencode run "<prompt that requires a file read>"` issues and executes read/glob/grep/bash tool calls
  through the canonical session runtime, not a parallel executor.
- Exactly one CLI model loop exists (shared with the TUI/server path, or explicitly justified).

## Recommended verification

- `cargo check -p opencode-cli -p opencode-session -p opencode-agent`
- Live: `./target/debug/opencode run "List the files in this workspace and summarize them"` with a real
  provider credential configured.

## Related Items

- `CLI-001` Copy Cline-style CLI task send conventions - prerequisite gate.
- `CLI-009` CLI and direct-run question parity - owns the CLI ask/approval path.
- `FEAT-011` Consolidate v1 and v2 session prompt loops.
- `PHASE-002` (phase parent)
- `BUG-003` Session stops completely after first prompt (uses `opencode run` in QA).
- `BUG-004` Coding sessions run as bare chat.

## Split Note - 2026-09-22

Split from the original `CLI-002` "CLI/AgentExecutor tool-loop parity" at user request:

- This card keeps the `opencode run` routing/consolidation decision and tool-loop behavior, now gated by
  `CLI-001`/`CLI-006`.
- `CLI-009` owns ask/approval (question) parity on the CLI path and is blocked by `GATE-002`.

## History

### Notes - 2026-09-09

- Implement after `BUG-004` so the tool-set resolution logic is shared rather than duplicated.

### Dev Notes - 2026-09-16

- Implemented the interim `AgentExecutor` path rather than routing `opencode run` through the session/server
  runtime. The canonical QA path remains `opencode serve` plus `/session/{id}/prompt` until `FEAT-011` or a
  follow-up routes `run` through the same session runtime.
- `AgentExecutor` now attaches permission-filtered tool definitions to provider requests when the active
  model supports tools, applies agent model params, and drives a bounded tool loop from `execute_streaming`
  instead of making a single tool-less model pass.
- Conversation conversion now preserves assistant `tool_use` parts and `tool_result` messages so executed
  tool output is fed back to the provider on the next pass.
- Streamed tool-call argument deltas are accumulated before JSON parsing, avoiding partial-JSON tool
  arguments.
- Ask-gated tools still return an explicit permission error on this direct executor path; they are no longer
  silently absent from the request. That gap is now owned by `CLI-009`.
- Verification: `cargo test -p opencode-agent`; `cargo check -p opencode-agent -p opencode-cli`; `cargo build -p opencode-cli`.
- Live smoke attempted with `./target/debug/opencode run "List the files in this workspace and summarize them"`
  and `./target/debug/opencode run "Say hello in one sentence."`; provider completion was blocked by
  `401 Unauthorized: {"error":{"message":"User not found.","code":401}}`.

### Merge Notes - 2026-09-16

- PR #36 merged into `development`: https://github.com/cchris-p/opencode-modded-rust/pull/36
- Board item was initially left in `qa`, then moved to `hold` because full completion required an
  ask/approval path that `GATE-002` now owns (via `CLI-009`). The merged loop is unverified in a real
  environment because the live smoke was blocked by provider auth.
