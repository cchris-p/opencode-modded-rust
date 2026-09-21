---
id: "FEAT-023"
title: "Filter session list and load by workspace"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-18"
---

# Filter session list and load by workspace

## Summary

Make the session view/load surfaces workspace-scoped so a TUI launched from one project only shows and loads sessions that belong to that project.

## Why this exists

The user's expected daily-driver workflow is project-local: if `ort` is run from `project_dir_a`, `project_dir_a` is the primary workspace for that TUI/server. The session picker and load/view commands should not mix in sessions from `project_dir_b`, even if those sessions were also created by `ort` on the same machine.

Without this boundary, unrelated work from other directories leaks into the current project workflow and makes it too easy to resume or inspect the wrong session.

## Scope

- Filter the TUI historical-session list, session picker, and load/view entry points to the current server workspace by default.
- Ensure explicit CLI task/session list and view commands use the selected or current workspace scope unless the user provides an explicit cross-workspace override.
- Make cross-workspace sessions unavailable in the default project-local list, not merely sorted lower.
- Provide clear empty-state or legacy-state text when no sessions match the current workspace.
- Ensure direct load by session ID rejects or clearly warns when the target session belongs to a different workspace, unless an explicit override exists.
- Cover both fresh sessions and sessions resumed after restarting `ort` in the same directory.

## Non-goals

- Reintroducing automatic attach/reuse of any previously started server.
- Hiding sessions from the global persistence store permanently; this card is about workspace-scoped product surfaces.
- Designing a full global session browser, unless needed as an explicit override path.
- Inferring workspace identity for old sessions beyond the legacy handling defined by `FEAT-022`.

## Done when

- If `ort` is launched from `project_dir_a`, the TUI session list only shows sessions associated with `project_dir_a`.
- Sessions created by `ort` from `project_dir_b` do not appear in `project_dir_a`'s default session view/load list.
- Loading or viewing a session from another workspace is blocked or requires an explicit cross-workspace override with clear wording.
- Restarting `ort` in `project_dir_a` still shows sessions from `project_dir_a` and still excludes sessions from other directories.
- Legacy sessions with unknown workspace identity are handled explicitly and do not silently appear as `project_dir_a` sessions.
- Verification covers at least two real or fixture directories with sessions in each.

## Recommended verification

- Create `project_dir_a` and `project_dir_b` test workspaces.
- Run `ort` from `project_dir_a`, create a session, exit, then run `ort` from `project_dir_b` and create a second session.
- Run `ort` again from `project_dir_a` and confirm only the `project_dir_a` session appears in the session picker/list.
- Attempt to load the `project_dir_b` session from `project_dir_a` by ID and confirm it is blocked or requires an explicit override.
- Run `ort` again from `project_dir_b` and confirm only the `project_dir_b` session appears by default.

## Related Items

- `FEAT-022` Persist session workspace identity
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-007` Add default task target selection for CLI sends
- `CLI-006` Add CLI status visibility for tasks and background sessions
- `BUG-011` ort targets the rust repo workspace and inherits the vanilla openrouter default

## Notes

- Acceptance criterion from the user: if `project_dir_a` is the primary workspace because `ort` was run in `project_dir_a`, only sessions for that project should be viewable/loadable there, not sessions from other directories created via `ort`.
