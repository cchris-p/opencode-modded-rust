---
id: "BUG-045"
title: "Server runtime stderr and panics are discarded, hiding why a run failed"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
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

## Dev Notes - 2026-09-26

- `crates/opencode-cli/src/main.rs`:
  - Added `resolve_server_log_path` / `server_log_path`: default
    `dirs::data_local_dir()/opencode/traces/server.log`; override with `OPENCODE_SERVER_LOG`;
    `0`/`false`/`off`/empty disables the sink.
  - `spawn_detached_tui_server` now opens the server log and points the child's stdout/stderr at it
    instead of `/dev/null` (falls back to `/dev/null` only if the log cannot be created or disabled).
  - `install_server_panic_hook` (called at server start in `run_server_command`) installs a global
    panic hook writing `[PANIC] <message>` + source location + `Backtrace::force_capture()` to the
    sink, so detached-task panics are durable even when stdio is discarded.
  - Discoverability: `opencode debug paths` prints `server-log`; `opencode session inspect` prints
    `Server log: <path>`.
- This is the Phase 0 prerequisite for `BUG-043`; it does not change run behavior.

## Verification - 2026-09-26

- `cargo test -p opencode-cli` -> 10 passed (new: `server_log_path_respects_override_and_disable`,
  `format_panic_entry_includes_message_location_and_backtrace`,
  `panic_hook_writes_message_and_location_to_sink`).
- `cargo check --workspace` clean (`SCOPEMUX_SKIP_NATIVE_BUILD=1`); `cargo fmt --all` clean.
- Live discovery: `opencode debug paths` shows
  `server-log  /Users/lapis/Library/Application Support/opencode/traces/server.log`;
  `opencode session inspect ses_2c1168ee…` prints `Server log: …/traces/server.log`.
- Live capture mechanism: launched `opencode serve --port 3398` with stdout/stderr redirected to a
  file (what the TUI spawn now does); `GET /health` returned 200 and the file captured the server's
  stderr (`Warning: …`, `Server errors are logged to …`, `Starting OpenCode serve server …`).
- Not run here: the full `ort` spawn path (headless) and a real detached-task panic end-to-end; the
  panic hook is covered by the unit test and the redirect by the live serve test. Confirm during QA
  by reproducing a failing run and checking `…/traces/server.log` for a session-scoped error.

## Notes

- Captured evidence: `lsof -a -p 99477 -d 0,1,2` (2026-09-26).
- Relevant files: `crates/opencode-cli/src/main.rs` (server spawn/stdio), `crates/opencode-server/`.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/117
  (branch `bug/BUG-045-server-runtime-diagnostics`, base `development`, handoff H-012 Pass A).