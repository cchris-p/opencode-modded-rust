---
id: "FEAT-016"
title: "Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "AGENTS.md"
status: "done"
created: "2026-09-15"
---

# Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace

## Summary

Completely remove reuse of a previous/recorded local TUI server. Every `ort` launch must
start a fresh server bound to the workspace it was activated in. User directive
(2026-09-15): "Utterly decimate the reusing of the last server completely. This is
unacceptable behavior."

Today `ort` records a per-workspace detached server in
`~/.local/state/opencode/tui-servers/*.json` and `prepare_local_tui_server` inspects/rotates
that record (`FEAT-014`). Even the "stop previous + next port" design keeps reading and
writing reuse state, and a stale/other-workspace server can still be the thing that serves
the TUI.

## Why this exists

Server reuse has repeatedly produced untrustworthy QA and wrong-product behavior:

- Stale detached servers served pre-fix binaries (`BUG-003`, `BUG-004` QA, archived
  `FEAT-002` notes).
- The TUI showing the rust repo instead of the activated directory (`BUG-011`) is exactly
  the kind of cross-workspace/state bleed that reuse makes possible.
- `AGENTS.md` still documents a "Reusing local TUI server" path, contradicting the
  intended "fresh server every launch" behavior.

## Scope

- Remove the recorded-server reuse path: do not read `tui-servers/*.json` to select,
  attach to, or stop a "last" server for a launch.
- Every `ort` run spawns a fresh server for the activated workspace.
- Port selection must not depend on a previous record; choose a free port
  (scan/OS-assigned) rather than `previous_port + 1`.
- Ensure the workspace passed to the server is the activated directory (pairs with
  `BUG-011`).
- Delete/stop writing the per-workspace record file (and any launch-time cleanup of
  legacy records) so no reuse state remains.
- Keep explicit `opencode attach <url>` working for intentional cross-process attach.

## Non-goals

- Removing the detached-server architecture or background session execution after TUI
  exit, unless required to eliminate reuse.
- Remote/multi-machine orchestration.
- Process supervision beyond the local launcher.

## Done when

- No launch path reads a recorded server to decide reuse/attach.
- Repeated `ort` launches in the same directory each get a fresh server on a free port.
- `ort` launched in directory X never serves a server whose cwd/config is directory Y.
- `opencode attach <url>` is unchanged.
- `AGENTS.md` launcher docs no longer describe reuse, or the residual documented contract
  is explicitly approved.

## Recommended verification

- Rebuild (`ort-build`) and run `ort` twice in the same non-repo workspace: confirm two
  distinct fresh servers, both with the correct workspace/config.
- Confirm no new `~/.local/state/opencode/tui-servers/*.json` is written and none is read
  on launch.
- Launch in two different directories; confirm each server's cwd/config matches its own
  launch directory.
- Confirm `opencode attach <url>` still works.

## Related Items

- `FEAT-014` Enforce a single local TUI server per workspace and increment the server per additional ort run (superseded reuse/rotation model)
- `FEAT-002` Keep sessions running after TUI exit (archived; origin of the reuse design)
- `BUG-003` Session stops completely after first prompt (stale-server QA trap)
- `BUG-011` ort targets the rust repo workspace and inherits the vanilla openrouter default

## Notes

- `FEAT-014` already removed blind reuse (stop previous, next port) but kept recorded state;
  this card removes the recorded/reuse concept entirely.
- No repo code changed for this card yet; this card exists for user evaluation of scope.

## Implementation - 2026-09-15

### What changed

`crates/opencode-cli/src/main.rs`:

- Removed the recorded-server reuse path entirely: `LocalTuiServerRecord`,
  `local_tui_server_record_path`, `load_local_tui_server_record`,
  `store_local_tui_server_record`, `terminate_local_tui_server` (unix/windows),
  `port_from_base_url`, `next_local_server_port`, and the now-unused `server_is_ready`.
- `prepare_local_tui_server` no longer reads or writes
  `~/.local/state/opencode/tui-servers/*.json`. Every launch spawns a fresh detached server
  in the current working directory (the activated workspace).
- Added `find_available_port(host, base_port)`: binds to test availability and returns the
  first free port at or above the requested base (default 3000). Port choice depends only on
  current bindability, never on prior launch state.
- `spawn_detached_tui_server` no longer returns a pid.
- Startup message is now
  `Starting fresh local server for TUI at <url> (workspace <dir>)`.

### Behavior notes

- Repeated `ort` launches each get a fresh server; there is no rotation state and no "last
  server" concept. Legacy `tui-servers/*.json` files are now inert (nothing reads them).
- `opencode attach <url>` is unchanged for intentional re-attachment.
- Old detached servers are no longer stopped by the launcher; process accumulation is out of
  scope (no daemon manager), documented here rather than reintroducing reuse state.

### Verification

- `cargo test -p opencode-cli` -> 2 passed (`find_available_port_returns_base_when_free`,
  `find_available_port_skips_a_bound_port`).
- `cargo check -p opencode-cli -p opencode-config` clean (no new warnings).
- Live: launched `opencode-rust-tui` from a scratch workspace; output confirmed the fresh
  server URL and workspace path, the detached server's cwd matched the scratch workspace,
  `/config/providers` reported the deepseek product default, and no new
  `tui-servers/*.json` record was written.

### Still open (this card)

- User verification on the PR branch: two `ort` launches in the same workspace produce two
  fresh servers, each bound to that workspace.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/33 (base `development`)

## Merge status

- Merged into `development` on 2026-09-15 via PR #33 (merge commit `9ef8b47`); PR branch deleted.
- Item moved from `qa` to `hold`; **open decision** whether to keep or revert the reuse removal now that the
  triggering observation came from testing on the wrong machine (see handoff `H-002` and
  `FEAT-014`). Reverting restores `FEAT-014` stop-prior + next-port behavior.

## Hold - 2026-09-16

User QA showed repeated `ort` launches from the same workspace starting fresh servers on
successive ports:

- `http://127.0.0.1:3025`
- `http://127.0.0.1:3026`
- `http://127.0.0.1:3027`

This confirms reuse is not happening, but the desired port/process semantics need refinement
before closing the story. The user expected that if the prior server was exited or otherwise no
longer in use, `ort` should prefer the non-used/free port rather than monotonically advancing.

Questions for refinement:

- Should a fresh `ort` launch choose the lowest currently free port at or above the base port, or
  should it continue advancing to avoid recently used ports?
- Should exiting the TUI also terminate the local server by default, or should detached server
  lifetime remain independent of the TUI?
- If a prior detached server for the same workspace is still alive, should the next `ort` leave it
  running and choose another free port, terminate it first, or attach only by explicit
  `opencode attach <url>`?
- Should server records remain completely eliminated, or is a minimal active-process record needed
  only for cleanup/termination without enabling reuse?

Moved to `hold` pending this decision.

## Refinement - 2026-09-16

Resolved decisions:

- Port selection stays lowest-free: every fresh `ort` launch should choose the lowest currently
  bindable port at or above the base port. Freed ports should be reused by number; ports should
  only advance while lower ports are currently occupied by live servers.
- `ort` should terminate the local server it just launched when the TUI exits normally. This keeps
  `Ctrl-D`/exit from leaving the launch server around to occupy its port.
- No persisted process/server record should be reintroduced for FEAT-016. Cleanup should use only
  the in-memory child process handle owned by the launching CLI process.
- Automatic attach/reuse of an existing same-workspace server is explicitly not part of this card.
  It may be added later only after separate confirmation because it reverses the current no-reuse
  invariant.

Updated done criteria:

- Repeated `ort` launches in the same directory choose the lowest currently free port at or above
  the base port.
- Exiting the TUI terminates the local server started for that TUI launch.
- No `tui-servers/*.json` or equivalent persisted record is read or written for launch reuse or
  cleanup.
- Explicit `opencode attach <url>` remains the only supported attach path.

Follow-up created:

- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers.
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist.

## Reimplementation - 2026-09-16

`crates/opencode-cli/src/main.rs` now keeps the server process handle for the local server started
by `ort`. The handle is in-memory only and is never written to a persisted record. When the TUI
returns, the launcher kills and waits for that exact server process, so a normal TUI exit releases
the port for the next lowest-free launch.

`opencode attach <url>` remains unchanged and does not create a local server cleanup guard.

Verification:

- `cargo test -p opencode-cli` -> passed, 2 tests.

## QA Closeout - 2026-09-16

User verified after `ort-build` that `ort` starts a fresh local server for the activated workspace
and reuses the lowest free port after TUI exit:

- `/Users/cchrisleepyles/repos/opencode-modded-rust` -> `http://127.0.0.1:3030`
- `/Users/cchrisleepyles/apps/cnaqma-notes` -> `http://127.0.0.1:3030`
- `/Users/cchrisleepyles/apps/TSS` -> `http://127.0.0.1:3030`

Closed as complete. Follow-up detach command behavior is tracked in `FEAT-017`; possible
same-workspace attach/reuse is tracked separately in `FEAT-018` and requires explicit 110%
confirmation before implementation.
