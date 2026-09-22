---
id: "CLI-004"
title: "Plan explicit detach command behavior for TUI-launched servers"
priority: "P2"
type: "feature"
area: "CLI"
status: "hold"
predecessors: "CLI-001, CLI-006"
created: "2026-09-16"
updated: "2026-09-22"
---

# Plan explicit detach command behavior for TUI-launched servers

## Blocked By - 2026-09-22

- `CLI-001` Copy Cline-style CLI task send conventions (prerequisite gate).
- `CLI-006` Add CLI status visibility for tasks and background sessions (prerequisite gate).
- Code for this card is already merged into `development` (PR #35, PR #62). The gate defers further
  refinement and QA closeout; do not act on this card until `CLI-001`/`CLI-006` land.

## Summary

Add an explicit `/detach` command/action for `ort`/TUI-launched local servers. Detach exits the
TUI but intentionally leaves the server launched for that TUI alive so the user can explicitly
reattach later.

## Context

`CLI-003` keeps the immediate launcher contract strict: every normal `ort` launch starts a fresh
server, no persisted server record is read or written, and exiting the TUI terminates the server
started for that TUI launch. A separate detach command would intentionally leave the launched
server alive after the TUI exits.

## Scope

- Add `/detach` as a slash-command and command-palette action. Do not add a default global
  keybinding initially.
- `/detach` exits the TUI immediately and leaves the local server started for that TUI launch alive.
- `/detach` is allowed while assistant/tool work is running; active work continues on the server.
- Normal exits remain distinct from detach: `Ctrl-D`, Esc, and `/exit` terminate the launched server
  by default, preserving `CLI-003` behavior.
- After detach returns to the terminal, print the server URL, workspace, and an explicit
  `opencode attach <url>` command so reattachment remains user-directed.
- Do not write a persisted record for detach in this item. Any record for later discovery belongs to
  `CLI-005` and requires separate confirmation.

## Non-goals

- Reintroducing automatic reuse before the behavior is explicitly approved.
- Changing `CLI-003`'s no-persisted-record cleanup path.
- Same-workspace auto-attach or automatic server reuse; that is tracked separately in `CLI-005`.
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

- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-005` Decide whether same-workspace server attach or reuse should exist

## QA Closeout Checklist - 2026-09-22

This item is code-complete (PR #35 and PR #62 merged into `development`) and needs post-merge
verification, not a handoff. It has no handoff dependency.

- [ ] From `development`, run `ort`, then `/detach`; confirm the TUI exits and the launched server stays up.
- [ ] Confirm the terminal prints the server URL, workspace, and the `opencode attach <url>` reattach command.
- [ ] Confirm `ort --attach <url>` reattaches to the detached server.
- [ ] Run `/detach` while a turn is running; confirm the TUI exits and work continues server-side.
- [ ] Confirm `Ctrl-D`, Esc, and `/exit` still terminate the launched server.
- [ ] Confirm no detach-time process/server record is written.

On pass, move this card from `qa` to `done`.

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/35
- Follow-up attach option PR: https://github.com/cchris-p/opencode-modded-rust/pull/62

## QA Status

- Merged into `development` via PR #35 on 2026-09-16; retained in `qa` for post-merge verification.
- 2026-09-21 follow-up: clarified detach terminal output to print `Reattach command: opencode attach <url>`.
- 2026-09-21 follow-up: added `opencode tui --attach <url>` so the local `ort --attach <url>` launcher shape is supported, and updated detach output to print both `opencode attach <url>` and `ort --attach <url>` reattach commands.
- 2026-09-21 closeout: follow-up PR #62 merged into `development`; item remains in `qa` pending post-merge detach/reattach verification.
