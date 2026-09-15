---
id: "FEAT-016"
title: "Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "AGENTS.md"
status: "qa"
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