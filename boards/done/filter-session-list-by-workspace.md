---
id: "FEAT-023"
title: "Filter session list by workspace"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-09-18"
---

# Filter session list by workspace

## Summary

Make the TUI historical-session list and session picker workspace-scoped so a TUI launched from one project only lists sessions that belong to that project.

## Why this exists

The user's expected daily-driver workflow is project-local: if `ort` is run from `project_dir_a`, `project_dir_a` is the primary workspace for that TUI/server. The session picker and list should not mix in sessions from `project_dir_b`, even if those sessions were also created by `ort` on the same machine.

Without this boundary, unrelated work from other directories leaks into the current project workflow and makes it too easy to resume or inspect the wrong session.

## Status Note

This card was originally scoped to cover session **list** filtering and session **load/view** restrictions together. Investigation on 2026-09-21 (`docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md`) confirmed the list-filtering half was implemented and verified as part of `FEAT-022` / `REFACTOR-001`, while the load/view + CLI half was not. On 2026-09-23 the card was narrowed to the completed list-filtering scope, marked done, and the remaining load/view + CLI work was split into `FEAT-057` (hold).

## Scope (completed)

- Filter the TUI historical-session list, session picker, and list entry points to the current server workspace by default.
- Make cross-workspace sessions unavailable in the default project-local list, not merely sorted lower.
- Ensure legacy sessions with unknown workspace identity do not silently appear as current-workspace sessions.

## Non-goals

- Direct load/view-by-ID restrictions and CLI session scoping; those are split into `FEAT-057`.
- Reintroducing automatic attach/reuse of any previously started server.
- Hiding sessions from the global persistence store permanently.

## Done when

- If `ort` is launched from `project_dir_a`, the TUI session list only shows sessions associated with `project_dir_a`.
- Sessions created by `ort` from `project_dir_b` do not appear in `project_dir_a`'s default session list.
- Restarting `ort` in `project_dir_a` still shows sessions from `project_dir_a` and still excludes sessions from other directories.
- Legacy sessions with unknown workspace identity are excluded from the default workspace-scoped list.

## Implementation Notes

- Session model and storage gained a durable `workspace_identity` in `FEAT-022` (`crates/opencode-session/src/session.rs`, `crates/opencode-storage/src/`), which this card consumes.
- Server list path filters by workspace: `list_sessions` parses `workspace_identity` and delegates to `SessionManager::list_filtered` (`crates/opencode-server/src/routes.rs:340`, `crates/opencode-session/src/session.rs:1339`).
- Legacy/unknown sessions are excluded by the filter, with a focused test: `session::tests::test_list_filtered_by_workspace_identity_excludes_other_workspaces_and_legacy`.
- TUI threads the workspace through a single seam: `ApiClient::new(base_url, workspace_dir)` / `list_workspace_sessions_filtered` (`crates/opencode-tui/src/api.rs:399`, `:453`), called from `App::refresh_session_list_dialog` (`crates/opencode-tui/src/app/app.rs:3080`).
- HTTP-boundary coverage added by `REFACTOR-001`: `session_route_filters_by_workspace_identity_before_search_and_limit` in `crates/opencode-server/tests/skill_route.rs`.

## PR / Commit Links

- Commit `07c760a` — `fix(session): filter TUI session list by workspace` (2026-09-19).
- Commit `429c410` — `refactor(session): centralize workspace list filtering` (2026-09-19).
- PR #43 — REFACTOR-001 session workspace boundary coverage / refactor (`bbb5010`, merged 2026-09-19), contains both commits and the route-level coverage.
- PR #42 — `FEAT-022` persist session workspace identity (`cec1363`, commit `95e5d82`), the enabling metadata.

## Related Items

- `FEAT-022` Persist session workspace identity (done)
- `FEAT-057` Restrict cross-workspace session load/view and CLI session surfaces (hold; split from this card)
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `BUG-011` ort targets the rust repo workspace and inherits the vanilla openrouter default

## Merge Closeout

- List-filtering implementation shipped via PR #42/#43 and on `development`; the `FEAT-022` QA and `REFACTOR-001` QA reports record the verification.
- Card moved from `todo` to `done` on 2026-09-23 after confirming the user-visible behavior (only workspace-local sessions listed) and splitting the unimplemented load/view scope to `FEAT-057`.

## Notes

- Acceptance criterion from the user: if `project_dir_a` is the primary workspace because `ort` was run in `project_dir_a`, only sessions for that project should be viewable/loadable there.
- The "viewable/loadable" wording spans both this card and `FEAT-057`; only the list/view-list half is delivered here.
