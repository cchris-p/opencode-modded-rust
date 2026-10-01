---
id: "BUG-055"
title: "Detach reattach command drops the active session id"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/cli-surface.md"
status: "qa"
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

## Second defect (printed command not runnable)

Carrying the session id was not sufficient. The hint hardcoded both an `opencode attach <url>` line
and an `ort --attach <url>` line with prose labels:

```
Reattach command: opencode attach http://127.0.0.1:3191 --session ses_...
ort reattach command: ort --attach http://127.0.0.1:3191 --session ses_...
```

Two problems in the launching shell:

- The interactive shell defines `opencode() { opencode-rust-tui "$@"; }`
  (`~/standards/opencode-config:181`), and `opencode-rust-tui` injects the `tui` subcommand. So
  `opencode attach <url>` becomes `opencode tui attach <url>` and fails with
  `unexpected argument '<url>' found`.
- The prose label sits on the same line as the command, so copying the whole line pastes the label
  as arguments (`unexpected argument 'command:' found`).

`ort --attach <url> --session <id>` is the working form in that shell; a bare `opencode` install
would use the `attach <url>` subcommand instead.

## Proposed fix

- Make `TuiExit::Detach` carry `session_id: Option<String>`, populated from the current route at
  detach time.
- Print the reattach hint with `--session <id>` when a session is active, falling back to the
  session-less command otherwise.
- Choose the launcher the same way the exit resume hint does (`ort` when `OPENCODE_RUST_REPO` is
  set, else `opencode`) and emit the matching command form (`ort --attach <url>` vs
  `opencode attach <url>`) on its own indented line so the label is never pasted as arguments.

This keeps reattachment explicit (no implicit server discovery), consistent with `CLI-003`/`CLI-005`.

## Scope

- In: detach reattach hint and the `TuiExit::Detach` payload.
- Out: automatic server discovery/reuse (`CLI-005` NO-GO), and changing normal exit behavior
  (`FEAT-033` resume hint stays as-is).

## Done when

- `/detach` from a session prints a `--session <id>` reattach command that actually runs in the
  launching shell (launcher-aware `ort`/`opencode` form).
- Running that command reopens the detached session.
- Detach from a non-session view (no active route) still prints a session-less command.
- Unit tests cover the launcher forms with and without a session id.

## Verification

- `cargo check -p opencode-cli -p opencode-tui`
- `cargo test -p opencode-cli hint`
- Manual: `ort`, `/detach`, run printed reattach command, confirm the prior session reopens.

## Dev Notes

- `TuiExit::Detach` now carries `session_id` (`crates/opencode-tui/src/lib.rs`).
- `App::run` populates `session_id` from `context.current_route()` for both detach and exit
  (`crates/opencode-tui/src/app/app.rs`).
- Added `reattach_hint_lines(base_url, session_id, launcher)` and call it with `resume_launcher()`
  in the detach branch (`crates/opencode-cli/src/main.rs`). It emits `ort --attach <url>` for the
  `ort` launcher and `opencode attach <url>` otherwise, on its own indented line.
- Added unit tests `reattach_hint_uses_ort_attach_flag_with_session`,
  `reattach_hint_uses_opencode_attach_subcommand_with_session`, and
  `reattach_hint_omits_session_id_when_absent`.
- Verified with `cargo check -p opencode-cli -p opencode-tui` and `cargo test -p opencode-cli hint`
  (5 passed).

## Related items

- `CLI-004` Plan explicit detach command behavior for TUI-launched servers (done)
- `CLI-005` Decide whether same-workspace server attach or reuse should exist (done, NO-GO)
- `FEAT-033` Print a resume command on normal TUI exit (done)

## Landing

- Landed by direct commit on `development` (no PR), per user direction.

## QA Status

- Committed directly to `development`; awaiting local re-verification of the printed reattach
  command.