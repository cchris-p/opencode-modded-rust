---
id: "FEAT-033"
title: "Print a resume command for the last session on TUI exit"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-21"
---

# Print a resume command for the last session on TUI exit

## Summary

When the TUI exits normally (`Ctrl+D`, `Esc`, `/exit`, `/quit`, `/q`), print a short, copy-pasteable command that reopens the session the user was just in, so they can jump straight back after leaving the terminal.

## Why this exists

Today a normal exit is silent about how to get back in. The user has to remember the session id or hunt through `opencode session list`, even though the launcher already knows the workspace and the runtime already persists the session to SQLite. The detach path already prints a reattach hint, but the far more common normal-exit path does not.

## Recommended method (best method)

Print a **session-scoped CLI command**, not a URL:

```
Resume this session:
  opencode --session <session-id>

Or open the most recent session in this workspace:
  opencode --continue
```

Why the command is better than a URL here:

- On a normal exit the launcher tears down the local server (`LocalTuiServer::terminate_on_drop` stays true; see `crates/opencode-cli/src/main.rs:927`). A printed `opencode attach <url>` would point at a dead server and fail.
- `--session <id>` re-launches a fresh server for the same workspace and reopens the exact session from the shared SQLite store, which is the behavior the user actually wants.
- `--continue` (`-c`) already resolves the most recent root session (`resolve_requested_session` at `crates/opencode-cli/src/main.rs:1030`) and is the zero-argument fallback, but it is ambiguous when several sessions exist, so it should be secondary.
- A URL is still the right hint for the **detach** path only; keep the existing `Reattach: opencode attach <url>` line for `TuiExit::Detach` unchanged.

## Scope

- On normal TUI exit, print the resume hint to stderr after the alternate screen is restored, mirroring the existing detach-hint style in `run_tui`.
- Include the exact workspace-resolved command the user should run, and the session id so the command is unambiguous.
- Make the CLI learn the session that was active at exit time. The launch-time `OPENCODE_TUI_SESSION` env var only covers the session selected at launch; if the user created or switched sessions during the run, the CLI must be told the final active session.
- Pick one mechanism for that handoff and use it consistently, for example:
  - extend `TuiExit` (`crates/opencode-tui/src/lib.rs:23`) so the normal exit carries `Option<String>` with the active session id, or
  - return a small exit struct, or
  - have the CLI query the most-recently-updated root session for the workspace from `SessionRepository` after exit.
- Keep `--continue` as the fallback hint when no specific session id is available.
- Keep the hint on stderr so it does not pollute piped stdout.

## Non-goals

- Auto-resuming a session without the user asking; this only prints a hint.
- Changing `Ctrl+D` / `Esc` / `/exit` exit semantics; that is `FEAT-006`.
- Changing detach behavior or the existing detach/reattach hint; that is `FEAT-017`.
- Changing `--continue` / `--session` resolution rules.
- Printing a live server URL for the normal-exit path.

## Done when

- Exiting the TUI normally prints a resume command that includes the session id.
- Running the printed command reopens the same session in the same workspace.
- The hint reflects the session that was actually active at exit, including when the user switched or created a session during the run.
- The detach path still prints its `opencode attach <url>` hint and is not regressed.
- The hint does not appear on stdout.

## Recommended verification

- `ort-build`, then `ort`; in a session send at least one message, exit with `Ctrl+D`, and confirm the printed `opencode --session <id>` command is present.
- Run the printed command and confirm the same transcript reopens.
- Repeat after switching to a different session mid-run and confirm the printed id matches the session that was active on exit.
- Repeat the detach flow and confirm the `opencode attach <url>` hint still prints.
- Confirm `opencode tui ... | cat` style stdout is not polluted by the hint.

## Related Items

- `FEAT-002` Keep sessions running after TUI exit (archived; superseded by `FEAT-014`)
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt
- `FEAT-014` Enforce a single local TUI server per workspace
- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist
- `FEAT-019` Add CLI status visibility for tasks and background sessions

## Notes

- The `ort` launcher wraps the built binary in the activated directory, so the printed command should use a stable executable name (`opencode`) rather than echoing the shell wrapper, and should note the workspace via `--session` id rather than a directory argument.
- If the CLI chooses the "query most-recent root session" route, make sure workspace scoping is correct so a hint never points at a session from another directory; that filtering work is tracked by `FEAT-023`.
