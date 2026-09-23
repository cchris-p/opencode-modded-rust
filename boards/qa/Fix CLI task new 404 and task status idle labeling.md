---
id: "BUG-036"
title: "Fix CLI task new 404 and task status idle labeling"
priority: "P2"
type: "bug"
area: "CLI"
spec: "wiki/cli-surface.md"
status: "qa"
created: "2026-09-23"
---

# Fix CLI task new 404 and task status idle labeling

## Summary

Post-merge headless QA of the CLI task surface (`CLI-001`, `CLI-006`) found two defects:

1. `opencode task new` always fails with `404 Not Found`, so the CLI cannot create a task/session.
2. `opencode task status` labels idle sessions as `active`, so a session whose turn has finished looks
   the same as the generic active lifecycle state and `completed` sessions are not distinguishable.

Both are in `crates/opencode-cli/src/main.rs`. Evidence is recorded in
`boards/qa/copy-cline-style-cli-task-send-conventions.md` (CLI-001) and
`boards/qa/add-cli-status-visibility-for-tasks-and-background-sessions.md` (CLI-006).

## Defect 1 - `task new` 404

`create_task_session` posts to `/session/` with a trailing slash
(`crates/opencode-cli/src/main.rs:2788`). The server registers `POST /session` without a trailing slash
(`crates/opencode-server/src/routes.rs:68`), and `server_url` preserves the trailing slash, so the
request 404s.

Reproduction:

```
$ opencode task new --server http://127.0.0.1:<port> "Reply with exactly PONG"
Error: Request failed (404 Not Found):
```

Raw evidence: `POST /session/` -> `404`, `POST /session` -> `200`.

## Defect 2 - idle sessions labeled `active`

`print_task_statuses` prints the server-provided `status` string directly
(`crates/opencode-cli/src/main.rs:2938-2942`). When a run is idle, the server returns the session
lifecycle status (`"active"`) rather than `"idle"`, so `task status` never shows `idle`.

Reproduction: after a completed turn on a normal session, `task status` shows `active` even though the
raw endpoint returns `{status: "active", idle: true, busy: false}`.

## Scope

- Fix the session-create URL so `opencode task new` succeeds.
- Make `task status` surface `idle` when the run is idle and the lifecycle is `active`, while preserving
  `busy`, `queued`, `retry`, `completed`, `archived`, and `compacting`.
- Keep `--json` output unchanged except for any status label change that is consistent with the server
  contract; do not remove the raw `status` object.

## Non-goals

- Changing the server `/session/status` contract.
- Deriving `completed` from assistant message state for ordinary chat sessions; lifecycle `completed`
  (task-stage sessions) and idle turns are sufficient and avoid per-session message fetches.
- Reworking `task target`, `task send`, or `task view`, which already pass QA.

## Acceptance Criteria

- `opencode task new --server <url> "<prompt>"` creates a session and returns the session id plus status.
- `opencode task status` shows `idle` for an idle normal session, `busy` while a turn runs, and
  `queued` with position/depth when queued.
- Lifecycle `completed`/`archived`/`compacting` sessions continue to display those labels rather than
  `idle`.
- No regression in `task status --json`, `task target`, `task send`, or `task view`.

## Verification

- `cargo fmt --all` and `cargo check -p opencode-cli`.
- Headless: start `opencode serve`, run `task new`, then `task send`, `task view`, and
  `task status` (plain and `--json`), including a queued turn, and confirm labels are
  `idle`/`busy`/`queued` as expected.
- Confirm the empty/no-target and unreachable-target error paths still fail clearly.

## Related Items

- `CLI-001` Copy Cline-style CLI task send conventions - owns `task new/send/view`; QA blocked by Defect 1.
- `CLI-006` Add CLI status visibility for tasks and background sessions - owns `task status`; QA blocked by Defect 2.
- `CLI-007` Add default task target selection for CLI sends - passed QA; its selected target is consumed here.
- `GATE-001` session prompt queue and `Queued { position, depth }` run status - source of the status data.

## Implementation Notes - 2026-09-23

- Branch: `bug/BUG-036-cli-task-surface-defects`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/82.
- Defect 1: `create_task_session` now posts to `/session` (no trailing slash) in
  `crates/opencode-cli/src/main.rs:2788`.
- Defect 2: `print_task_statuses` now prints `idle` when the run is idle and the lifecycle is `active`,
  while preserving `busy`/`queued`/`retry` and lifecycle `completed`/`archived`/`compacting`
  (`crates/opencode-cli/src/main.rs:2938`).
- Verification passed: `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo test -p opencode-cli`
  (5 passed). Headless re-QA against a live `opencode serve`: `task new` succeeds and persists the
  selected session; `task status` shows `idle`, `busy`, and `queued 1/1`; `--json` output unchanged.
