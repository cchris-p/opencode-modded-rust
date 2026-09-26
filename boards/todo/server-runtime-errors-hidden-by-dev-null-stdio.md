---
id: "BUG-045"
title: "Server runtime stderr and panics are discarded, hiding why a run failed"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-26"
---

# Server runtime stderr and panics are discarded, hiding why a run failed

## Summary

The TUI spawns the local server with stdout/stderr pointed at `/dev/null`. Any panic, `tracing` error,
or diagnostic the server writes is thrown away, so when a run stops without a terminal record there is
nothing recorded anywhere to explain why. This blocked confirming the mechanism of `BUG-043` (panic in
the detached drain task vs. a permanently parked await) and will block every future run-failure
investigation until fixed.

## Evidence

- Live Rust server `pid 99477` for session `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26):
  `lsof -a -p 99477 -d 0,1,2` shows fd 1 and fd 2 both `CHR 3,2 ... /dev/null`.
- The only durable runtime artifact is the TUI-side trace
  `~/Library/Application Support/opencode/traces/tui.log`, which records TUI SAMPLE/SYNC lines only,
  not server errors.
- The server logs run failures via `tracing::error!` (for example
  `crates/opencode-server/src/routes.rs:4025`, `"session prompt failed"`) and a panicking detached
  task only reaches the default panic handler on stderr — both currently discarded.
- Related context: `BUG-043` could not distinguish panic from a parked await because of this gap.

## Why this exists

Failure investigation depends on having at least one durable artifact for the failing stage. With
server stdio nulled and no panic hook writing anywhere, a wedged run produces no evidence at all — the
investigator is reduced to OS-thread sampling, which cannot see parked async tasks.

## Scope

- Route the server's stdout/stderr to a durable sink (per-run or rolling log file) instead of
  `/dev/null`, without breaking TUI display.
- Install a panic hook that records panic message, location, and backtrace to the same durable sink,
  including panics in detached tasks.
- Ensure server-side run failures (`tracing` errors and prompt errors) are written to that sink and
  are discoverable per session id.
- Make the sink path discoverable from the product (for example `opencode debug paths` and/or
  `opencode session inspect`).

## Non-goals

- Fixing the run-terminal-state defect itself; that is `BUG-043`.
- Replacing the existing TUI trace (`tui.log`); this adds the missing server-side sink.
- Full distributed tracing; a durable local file is sufficient.

## Done when

- A forced panic in the server (or a detached run task) leaves a durable record with message,
  location, and backtrace.
- A run that fails server-side leaves a durable, session-scoped error record.
- The log sink location is reported by a product command.

## Recommended verification

- Unit/integration test that triggers a panic and asserts the sink contains it.
- Manual: reproduce a failing run and confirm the sink has a session-scoped error.
- `cargo test -p opencode-server -p opencode-cli`; `cargo check --workspace`.

## Related Items

- `BUG-043` A session run can end without a terminal state - blocked on this for root-cause
  confirmation.
- `session-diagnostics-and-stack-capture` (done) - existing diagnostics/stacks work this extends.
- `BUG-046` / `BUG-047` - other `BUG-043` split-outs.

## Notes

- Captured evidence: `lsof -a -p 99477 -d 0,1,2` (2026-09-26).
- Relevant files: `crates/opencode-cli/src/main.rs` (server spawn/stdio), `crates/opencode-server/`.
