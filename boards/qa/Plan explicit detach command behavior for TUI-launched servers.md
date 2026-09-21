---
id: "FEAT-017"
title: "Plan explicit detach command behavior for TUI-launched servers"
priority: "P2"
type: "feature"
area: "FEAT"
status: "qa"
created: "2026-09-16"
---

# Plan explicit detach command behavior for TUI-launched servers

## Summary

Add an explicit `/detach` command/action for `ort`/TUI-launched local servers. Detach exits the
TUI but intentionally leaves the server launched for that TUI alive so the user can explicitly
reattach later.

## Context

`FEAT-016` keeps the immediate launcher contract strict: every normal `ort` launch starts a fresh
server, no persisted server record is read or written, and exiting the TUI terminates the server
started for that TUI launch. A separate detach command would intentionally leave the launched
server alive after the TUI exits.

## Scope

- Add `/detach` as a slash-command and command-palette action. Do not add a default global
  keybinding initially.
- `/detach` exits the TUI immediately and leaves the local server started for that TUI launch alive.
- `/detach` is allowed while assistant/tool work is running; active work continues on the server.
- Normal exits remain distinct from detach: `Ctrl-D`, Esc, and `/exit` terminate the launched server
  by default, preserving `FEAT-016` behavior.
- After detach returns to the terminal, print the server URL, workspace, and an explicit
  `opencode attach <url>` command so reattachment remains user-directed.
- Do not write a persisted record for detach in this item. Any record for later discovery belongs to
  `FEAT-018` and requires separate confirmation.

## Non-goals

- Reintroducing automatic reuse before the behavior is explicitly approved.
- Changing `FEAT-016`'s no-persisted-record cleanup path.
- Same-workspace auto-attach or automatic server reuse; that is tracked separately in `FEAT-018`.
- Adding a default detach keybinding.
- Persisting a server/process record for future launch discovery.

## Done when

- `/detach` is available from the existing TUI command surfaces.
- Running `/detach` closes the TUI and leaves the TUI-launched local server alive.
- Detach is allowed while work is running, and the work continues on the server.
- Normal `Ctrl-D`, Esc, and `/exit` still terminate the TUI-launched server.
- The terminal prints the detached server URL, workspace, and `opencode attach <url>` command.
- No detach-time persisted record is written.

## Implementation Notes

- The TUI likely needs a distinct detach result/signal rather than treating detach as ordinary exit,
  so the CLI can skip the `LocalTuiServer` cleanup guard only for `/detach`.
- The current base URL is already passed into the TUI via `OPENCODE_TUI_BASE_URL`; the detach output
  can use that URL and the current workspace path.
- `opencode attach <url>` remains the explicit reattach mechanism for this item.

## Dev Notes

- Added `/detach` to the TUI command registry and prompt slash-command suggestions with no default
  keybinding.
- Added a distinct `TuiExit::Detach` path so `/detach` exits the TUI without being treated as a normal
  cleanup-triggering exit.
- Updated CLI local-server cleanup so only detach disables the `LocalTuiServer` kill-on-drop guard and
  prints the server URL, workspace, and `opencode attach <url>` command after returning to the terminal.
- Verified with `cargo fmt` and `cargo check -p opencode-cli -p opencode-tui`.

## Related Items

- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/35

## QA Status

- Merged into `development` via PR #35 on 2026-09-16; retained in `qa` for post-merge verification.
- 2026-09-21 follow-up: clarified detach terminal output to print `Reattach command: opencode attach <url>`.
- 2026-09-21 follow-up: added `opencode tui --attach <url>` so the local `ort --attach <url>` launcher shape is supported, and updated detach output to print both `opencode attach <url>` and `ort --attach <url>` reattach commands.
