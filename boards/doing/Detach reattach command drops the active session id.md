---
id: "BUG-053"
title: "Detach reattach command drops the active session id"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/cli-surface.md"
status: "doing"
created: "2026-09-30"
---

# Detach reattach command drops the active session id

## Summary

After `/detach`, the terminal prints an `opencode attach <url>` / `ort --attach <url>` reattach
command that carries no session id. Attaching then opens a fresh/blank session instead of returning
to the session the user was viewing. Reattaching should reopen the detached session.

## Reported behavior

```
$ ort
Starting fresh local server for TUI at http://127.0.0.1:3189 (workspace /Users/cchrisleepyles/apps/tss-notes)
Detached from TUI server.
Server: http://127.0.0.1:3189
Workspace: /Users/cchrisleepyles/apps/tss-notes
Reattach command: opencode attach http://127.0.0.1:3189
ort reattach command: ort --attach http://127.0.0.1:3189
```

Running the printed reattach command does not resume the detached session; it starts a new one.

## Root cause

- `TuiExit::Detach` carries no session id (`crates/opencode-tui/src/lib.rs:26`), unlike
  `TuiExit::Exit { session_id }`.
- The detach branch in the CLI prints reattach commands with no `--session`
  (`crates/opencode-cli/src/main.rs:967-977`).
- `run_tui` only navigates to a session when `OPENCODE_TUI_SESSION` is set, which comes from
  `--session` (`crates/opencode-cli/src/main.rs:166-174` in `app.rs`; `resolve_requested_session`).
  With no `--session`, attach lands on the home screen and the next prompt creates a new session.

The detach path already has the active route available (`context.current_route()`); it was simply
not threaded into the reattach hint.

## Proposed fix

- Make `TuiExit::Detach` carry `session_id: Option<String>`, populated from the current route at
  detach time.
- Print the reattach hint with `--session <id>` when a session is active, falling back to the
  session-less command otherwise.

This keeps reattachment explicit (no implicit server discovery), consistent with `CLI-003`/`CLI-005`.

## Scope

- In: detach reattach hint and the `TuiExit::Detach` payload.
- Out: automatic server discovery/reuse (`CLI-005` NO-GO), and changing normal exit behavior
  (`FEAT-033` resume hint stays as-is).

## Done when

- `/detach` from a session prints `opencode attach <url> --session <id>` and
  `ort --attach <url> --session <id>`.
- Running that command reopens the detached session.
- Detach from a non-session view (no active route) still prints the session-less command.
- Unit tests cover both hint variants.

## Verification

- `cargo check -p opencode-cli -p opencode-tui`
- `cargo test -p opencode-cli hint`
- Manual: `ort`, `/detach`, run printed reattach command, confirm the prior session reopens.

## Dev Notes

- `TuiExit::Detach` now carries `session_id` (`crates/opencode-tui/src/lib.rs`).
- `App::run` populates `session_id` from `context.current_route()` for both detach and exit
  (`crates/opencode-tui/src/app/app.rs`).
- Added `reattach_hint_lines(base_url, session_id)` and used it in the detach branch
  (`crates/opencode-cli/src/main.rs`).
- Added unit tests `reattach_hint_includes_session_id_when_available` and
  `reattach_hint_omits_session_id_when_absent`.
- Verified with `cargo check -p opencode-cli -p opencode-tui` and `cargo test -p opencode-cli hint`
  (4 passed).

## Related items

- `CLI-004` Plan explicit detach command behavior for TUI-launched servers (done)
- `CLI-005` Decide whether same-workspace server attach or reuse should exist (done, NO-GO)
- `FEAT-033` Print a resume command on normal TUI exit (done)