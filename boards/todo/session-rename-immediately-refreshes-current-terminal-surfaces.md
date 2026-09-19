---
id: "BUG-015"
title: "Session rename immediately refreshes current terminal surfaces"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-18"
---

# Session rename immediately refreshes current terminal surfaces

## Summary

After renaming a session, the new title must be reflected immediately in every visible surface in the current terminal session, not only after closing/reopening a dialog, switching sessions, or restarting the TUI.

## Reported behavior

Renaming a session can leave stale titles visible in the current terminal session. A user may still see the old title in surfaces such as:

- The sessions list.
- The top/header area of the current chat.
- Any literal current-session reference that is already rendered in the TUI.

## Expected behavior

- A successful rename updates the in-memory TUI session state immediately.
- If the renamed session is the active session, the current chat header and terminal title update immediately.
- If the sessions list is open, the renamed row updates immediately without requiring the list to be closed and reopened.
- Any current-session label/reference in the active terminal view reads from the updated title or is explicitly refreshed after the rename response.

## Current code evidence

- `crates/opencode-tui/src/app/app.rs` handles the standalone session rename dialog around `session_rename_dialog.confirm()` and currently refreshes the session list and syncs from the server after `client.update_session_title` succeeds.
- `crates/opencode-tui/src/app/app.rs` handles rename-from-session-list around `session_list_dialog.confirm_rename()` and refreshes the list, then syncs only if the renamed session is active.
- `crates/opencode-tui/src/api.rs` exposes `ApiClient::update_session_title`, which returns the updated `SessionInfo`; the TUI should prefer applying that returned session to local state rather than depending only on a later list refresh or server sync.

## Scope

- Ensure both rename entry points apply the successful rename result to local TUI state immediately.
- Update the active session title when the renamed session is the current chat.
- Update the open sessions-list row immediately when rename originates from the list.
- Confirm whether the terminal window title is driven from the active session title, and update it when the active session is renamed.
- Keep the fix focused on refresh/state propagation; do not redesign session storage or title generation.

## Likely implementation notes

- Add or reuse one local TUI helper that accepts the updated `SessionInfo` returned by `ApiClient::update_session_title` and applies it to all relevant in-memory surfaces.
- Use the returned `SessionInfo` as the authoritative post-rename value to avoid a stale round trip where possible.
- Keep `refresh_session_list_dialog()` only if it is still needed to preserve sort/status/filter behavior, but do not rely on it as the only visible update.
- If `sync_session_from_server` remains necessary for other fields, call it after the immediate local title update so the UI is not visibly stale between operations.

## Non-goals

- Redesigning generated session summaries or title-generation timing.
- Changing the persistent session schema.
- Adding new rename commands, shortcuts, or command-palette actions.
- Changing session ordering semantics except where the existing sessions-list refresh already does so.

## Done when

- Renaming the active session immediately changes the top/current chat title.
- Renaming from the sessions list immediately changes the visible list row.
- Renaming a non-active session updates visible sessions-list state without requiring a full TUI restart.
- The terminal title/session title behavior remains correct when the active session is renamed.
- Both rename entry points share the same post-success state update behavior or are otherwise demonstrably equivalent.

## Recommended verification

- Run `ort-build`, then launch `ort`.
- Start or open a session, rename it from the in-session rename flow, and confirm the current chat title/header updates immediately.
- Open the sessions list, rename the current session, and confirm the list row and current chat title both update immediately.
- Rename a different session from the sessions list and confirm that row updates immediately.
- Close and reopen the sessions list and confirm the persisted renamed title still appears.
- If practical, add focused `opencode-tui` unit coverage around the local post-rename state update helper.

## Related Items

- `BUG-002` Remove duplicate session actions and visible hotkey hints from the session UI
- `FEAT-001` Improve historical chat transcripts workflow
- `FEAT-019` Add CLI status visibility for tasks and background sessions

## Refinement status

- Ready to implement. The symptom, expected behavior, affected entry points, likely code touchpoints, non-goals, and manual verification path are defined.
