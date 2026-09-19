---
id: "REFACTOR-001"
title: "Add session workspace boundary coverage before refactor"
priority: "P1"
type: "refactor"
area: "REFACTOR"
spec: ""
status: "todo"
created: "2026-09-19"
---

# Add session workspace boundary coverage before refactor

## Summary

Add focused coverage for workspace-scoped session listing at the API boundary before refactoring the duplicated workspace-filtering code paths discovered during `FEAT-022` follow-up testing.

## Why this exists

The `FEAT-022` follow-up fix exposed a non-DRY boundary: session workspace filtering required coordinated changes in session filtering, server query parsing, TUI API request construction, and TUI app code. If future changes must remember to edit multiple locations, workspace isolation can regress even when core session model tests still pass.

This item starts with coverage so the later refactor can safely simplify where workspace-scoped session-list behavior is defined.

## Scope

- Add route-level coverage for `GET /session?workspace_identity=<dir>` in `crates/opencode-server/tests/` or the nearest existing server test harness.
- Verify the route returns sessions for the requested workspace only.
- Verify the route excludes sessions from other workspaces.
- Verify the route excludes legacy sessions where `workspace_identity` is `None`.
- Verify search and limit behavior still compose with workspace filtering if the test setup can do so without excessive harness work.
- After coverage exists, refactor duplicated workspace query construction so callers do not manually rebuild the workspace filter contract in presentation code.

## Non-goals

- Reworking global or cross-workspace browsing behavior.
- Changing persistence semantics for legacy sessions.
- Completing all remaining `FEAT-023` session load/view restrictions.
- Large DTO consolidation unless it is necessary for the focused refactor.

## Done when

- Workspace-scoped session-list behavior is covered at the HTTP/API boundary, not only in `SessionManager` unit tests.
- The new coverage would fail if `/session` ignored `workspace_identity` or returned legacy/other-workspace sessions.
- TUI session-list code no longer manually threads workspace filtering from `app.rs`; the workspace-scoped API is centralized in a small reusable boundary.
- Existing `SessionManager` workspace filtering tests continue to pass.
- The dedicated no-history-directory `ort` manual scenario from `FEAT-022` still passes after the refactor.

## Recommended verification

- `cargo test -p opencode-session session::tests::test_list_filtered_by_workspace_identity_excludes_other_workspaces_and_legacy`
- Run the new server route test covering `/session?workspace_identity=<dir>`.
- `cargo check -p opencode-session -p opencode-server -p opencode-tui -p opencode-cli`
- Run `ort-build`, then launch `ort` from a dedicated directory with no history and confirm transcripts from other workspaces do not appear.

## Dev Notes

- Existing server integration tests in `crates/opencode-server/tests/skill_route.rs` already use `routes::router().with_state(Arc::new(ServerState::new()))` plus `tower::ServiceExt::oneshot`; use the same pattern for the `/session` route coverage.
- Seed test sessions through `state.sessions.lock().await` so the route test exercises `crates/opencode-server/src/routes.rs` query parsing and `SessionManager::list_filtered`, not storage.
- The duplicated TUI seam is currently `ApiClient::list_sessions_filtered(..., workspace_identity)` in `crates/opencode-tui/src/api.rs` and `App::refresh_session_list_dialog` passing `self.context.directory` from `crates/opencode-tui/src/app/app.rs`.
- Prefer the refactor target `ApiClient::new(base_url, workspace_dir)` or an equivalent workspace-aware constructor/method so presentation code does not rebuild session workspace query parameters.

## Related Items

- `FEAT-022` Persist session workspace identity
- `FEAT-023` Filter session list and load by workspace

## Notes

- Created from the divergence discovered while fixing `FEAT-022`: the first implementation persisted workspace identity, but the TUI still listed global transcripts because the UI list path did not pass a workspace filter.
