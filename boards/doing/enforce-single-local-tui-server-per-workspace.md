---
id: "FEAT-014"
title: "Enforce a single local TUI server per workspace and increment the server per additional ort run"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
created: "2026-09-09"
---

# Enforce a single local TUI server per workspace and increment the server per additional ort run

## Summary

Prevent stale local TUI servers from silently invalidating QA. Today `ort` reuses a recorded detached server even when that process is still running an older binary, so a user can rebuild, launch, and test against the pre-fix server and see no change at all. Enforce that only one local server exists per workspace at a time, and that each additional `ort` run starts a fresh server instance (incrementing the instance/port) rather than silently reusing a possibly-stale process.

## Why this exists

The stale-server failure was observed directly while QAing `BUG-004`:

- `ort` records the running detached server in `~/.local/state/opencode/tui-servers/*.json` (per `FEAT-002`).
- A rebuilt binary at `target/debug/opencode` (e.g. 14:03) coexisted with a server process started at 06:50 on the recorded base URL. `/proc/<pid>/exe` reported `(deleted)`, proving the running server still mapped the pre-fix inode.
- QA on that recorded server reproduced the original bare-chat symptom exactly, because the new `session_prompt` code never ran.
- The same trap is documented in the archived `BUG-003` notes; this card removes the class of failure rather than repeating the manual cleanup (kill server, delete `tui-servers/*.json`, relaunch).

## Scope

- Define and enforce a single-server-per-workspace invariant for the local `ort`/TUI launch path.
- On `ort` launch when a server is already recorded/running for this workspace:
  - decide deterministically whether the existing server is still valid (current binary / healthy), and
  - if it is stale, stop/replace it and start a fresh server instance instead of reusing it.
- Each additional `ort` run that results in a new server instance must allocate a distinct instance (increment the server, i.e. the port), so repeated runs never silently attach to an out-of-date process.
- The "Reusing local TUI server" path must only occur when the running server is provably the current build and healthy.
- Keep explicit `opencode attach <url>` working for intentional cross-process/remote attach.

## Non-goals

- Cross-machine or remote-server orchestration.
- Rewriting the whole session-continuation model from `FEAT-002`; background session execution after TUI exit is retained.
- Process supervision beyond the local launcher (no full daemon manager).

## Done when

- Rebuilding and relaunching with `ort-build`/`ort` always executes the new binary: QA can never silently hit a pre-fix server.
- At most one local server is active per workspace at a time.
- Each additional `ort` run that must start a new instance increments the server instance/port rather than reusing a stale recorded one.
- A stale recorded server is detected and replaced automatically (no manual `kill` + `rm ~/.local/state/opencode/tui-servers/*.json` step needed).
- `opencode attach <url>` behavior is unchanged.
- The launcher behavior is documented in this repo's own `AGENTS.md` launcher section or equivalent docs.

## Recommended verification

- Start `ort`, note the server instance/port, exit, rebuild a changed binary, relaunch `ort`, and confirm the fresh run uses a new/incremented instance running the new binary rather than the recorded old one.
- Verify the "look at the files" BUG-004 QA passes on the fresh instance without manual cleanup.
- Confirm the workspace server record (`tui-servers/*.json`) is updated consistently and never points at a `(deleted)` process.
- Verify `opencode attach <url>` still works.

## Related Items

- `FEAT-002` Keep sessions running after TUI exit (archived; superseded by this card)
- `BUG-003` Session stops completely after first prompt (documents the stale-server trap)
- `BUG-004` Coding sessions run as bare chat (this card exists to make that QA trustworthy)

## Implementation - 2026-09-09

### Decision

- Selected "always fresh + next port" semantics: each additional `ort` run stops the previously recorded server and starts a fresh instance on the next port, recording only the newest. Background sessions on the superseded server end when it is stopped.

### What changed

- `crates/opencode-cli/src/main.rs`:
  - `LocalTuiServerRecord` now records `port` and `pid` in addition to `base_url` (both `Option` with serde defaults, so legacy records still load).
  - `prepare_local_tui_server` no longer reuses a recorded server blindly. If a recorded server is reachable it is stopped (via pid) before a fresh instance is started; stale records with an unknown pid are surfaced with guidance instead of silently reused.
  - Port selection now increments: `next_local_server_port(base_port, previous)` returns the previous recorded port + 1 (falling back to parsing the port from a legacy `base_url`), so repeated `ort` runs walk 3000 -> 3001 -> ... and never silently attach to an out-of-date process.
  - `spawn_detached_tui_server` returns the child pid; the new record stores port + pid.
  - Added `terminate_local_tui_server` (SIGTERM on unix, taskkill on windows), `port_from_base_url`, and `next_local_server_port`.
- Legacy records that carry no pid and are still reachable are not auto-killed (no pid to signal); they are reported and left running, and the launcher proceeds to the next port.

### Tests

- `cargo test -p opencode-cli`: 3 unit tests for `port_from_base_url` and `next_local_server_port` (defaults, increment from recorded port, legacy base_url parse).
- `cargo check -p opencode-cli -p opencode-server` passes.

### Still open (this card)

- Live verification with `ort-build`/`ort`: confirm repeated runs increment the port and stop the prior server, and that `opencode attach <url>` is unaffected.
- Confirm no stale recorded server is ever reused for QA.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/27 (branch `feature/FEAT-014-single-local-tui-server`, base `development`)

## Notes

- This card supersedes `FEAT-002`'s reuse model; see that card's Archived note for the failure evidence.
- The launcher logic lives in the CLI TUI boot path (`prepare_local_tui_server` / `spawn_detached_tui_server` and the server record handling); identify the exact functions when starting implementation.
