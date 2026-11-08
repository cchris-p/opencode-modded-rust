---
id: "FEAT-062"
title: "Restrict cross-workspace session load/view and CLI session surfaces"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "hold"
created: "2026-09-23"
---

# Restrict cross-workspace session load/view and CLI session surfaces

## Summary

Enforce workspace scope on direct session load/view surfaces and the CLI, and define an explicit cross-workspace override. This is the unimplemented half split out of `FEAT-023`.

## Why this exists

`FEAT-023` filtered the TUI session **list** by workspace and shipped (PR #42 / PR #43). But a session's workspace boundary is still not enforced when a caller already knows the target session id. Direct load/view by id and the CLI session surfaces remain unscoped, so a session from `project_dir_b` can still be opened from `project_dir_a`.

This card also defines the explicit cross-workspace override that `FEAT-026` (cross-session transcript inspection) depends on.

## Current State / Evidence

- `get_session` loads by id with no workspace check: `crates/opencode-server/src/routes.rs:589`.
- CLI `session list` filters by `project_id`, not `workspace_identity`: `crates/opencode-cli/src/main.rs:3990`.
- CLI `session show <id>` loads any session by id without a workspace check: `crates/opencode-cli/src/main.rs:4040`.
- No explicit cross-workspace override flag/config exists anywhere in the load/view paths.
- The list filtering that this card builds on: `crates/opencode-server/src/routes.rs:340`, `crates/opencode-session/src/session.rs:1339`, `crates/opencode-tui/src/api.rs:453`.

## Scope

- Reject or clearly warn when a direct load/view by session id targets a session outside the current workspace, unless an explicit override is provided.
- Apply workspace scope by default to CLI `session list` and `session show`/load paths.
- Ensure resume/continue surfaces (`ort <id>`, `--continue`, most-recent-root selection) never silently open another workspace's session.
- Define one explicit cross-workspace override (flag and/or config) and surface it consistently across TUI and CLI.
- Handle legacy sessions with unknown workspace identity explicitly; do not silently treat them as current-workspace.
- Distinguish the empty/legacy/out-of-scope states with clear user-facing text.

## Non-goals

- Re-implementing the TUI list filtering already delivered by `FEAT-023`.
- Reintroducing automatic server reuse or attach behavior.
- Building a full global multi-workspace session browser beyond the explicit override.
- Migrating or guessing workspace identity for historical sessions beyond the legacy/unknown handling defined by `FEAT-022`.

## Done when

- Loading or viewing a session from another workspace by id is blocked or requires the explicit override, with clear wording.
- CLI `session list`, `session show`, and resume surfaces are workspace-scoped by default.
- The explicit cross-workspace override is available and documented, and `FEAT-026` can rely on it.
- Legacy sessions with unknown workspace identity do not masquerade as current-workspace sessions.
- Verification covers two real or fixture directories plus a legacy/unknown-workspace session.

## Recommended verification

- Create `project_dir_a` and `project_dir_b` test workspaces with one session each.
- From `project_dir_a`, attempt to load the `project_dir_b` session by id and confirm it is blocked or requires the override.
- Confirm the override succeeds with the intended explicit signal and fails closed without it.
- Run CLI `session list` and `session show` from each directory and confirm only same-workspace sessions resolve by default.
- Confirm a legacy session with `workspace_identity = None` is surfaced as legacy/unknown, not as current-workspace.

## Related Items

- `FEAT-023` Filter session list by workspace (done; delivered the list-filtering half)
- `FEAT-022` Persist session workspace identity (done; provides `workspace_identity`)
- `REFACTOR-001` Add session workspace boundary coverage before refactor (qa; added route/HTTP coverage and the TUI seam)
- `FEAT-026` Add cross-session transcript inspection (depends on the explicit cross-workspace override defined here)
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-007` Add default task target selection for CLI sends
- `BUG-011` ort targets the rust repo workspace and inherits the vanilla openrouter default

## Notes

- Split from `FEAT-023` on 2026-09-23. The prior investigation is recorded in `docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md`.
- Placed in `hold` per the 2026-09-23 decision to defer the load/view + CLI scope while marking the delivered list-filtering scope done.
- Keep the override explicit and narrow; the goal is preventing accidental cross-project resume, not enabling cross-project browsing.
