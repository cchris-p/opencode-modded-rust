---
id: "CLI-006"
title: "Add CLI status visibility for tasks and background sessions"
priority: "P3"
type: "feature"
area: "CLI"
spec: ""
status: "done"
created: "2026-09-16"
updated: "2026-09-22"
---

# Add CLI status visibility for tasks and background sessions

## Status - 2026-09-22

Reactivated at user request as a co-prerequisite gate for the remaining `CLI-*` stories. Cline-like
functionality (`CLI-001`) plus CLI status visibility must exist before other CLI stories are refined
further.

Prerequisite already in place: `GATE-001` (done) exposes `status` (`idle|busy|queued|retry|...`) plus
`position`/`depth` from `GET /session/status`, which is this card's data source. Do not synthesize
`queued` client-side.

Canonical behavior reference: `wiki/cli-surface.md`.

Handoff: `handoffs/archive/2026-09-22-cli-task-surface-and-status-handoff.md` (`H-006`, complete).

## Summary

Add a CLI-visible status surface for tasks/sessions so command-line workflows can see what is running, waiting, completed, errored, or otherwise actionable without opening the TUI.

## Why this exists

`CLI-001` focuses on Cline-style task send conventions: starting work, sending follow-up messages, and attaching files from the shell. Status visibility is related but separable. It should not bloat the send-task implementation, and it should respect the attach/detach lifecycle decisions tracked elsewhere.

## Product Decisions

- Visible states: `idle`, `busy`, `queued` (with depth/position), `retry`, and `error`; completed is
  derived from assistant message completion. `queued` must come from the `GATE-001` run status, not be
  synthesized client-side.
- Output formats: human-readable plain text by default, plus `--json` for scripts. Rich/TUI styling is
  out of scope.
- Status is read from the existing session/status model (`GET /session/status` and session list data);
  no separate status database.

## Scope

- Define which task/session states are visible from the CLI, such as running, idle, awaiting approval, complete, failed, or detached where supported.
- Add a command or subcommand that lists recent or active tasks/sessions with compact status information.
- Add a command or subcommand that views one task/session status in more detail.
- Expose plain-text output by default and `--json` for scripts; do not add rich/TUI formatting.
- Reuse the existing session/task state model rather than inventing a separate status database.
- Make status useful for sessions that are not currently visible in the TUI when the runtime can observe them.

## Non-goals

- Sending prompts or follow-up chat messages; that is `CLI-001`.
- Defining detach behavior for TUI-launched servers; that is `CLI-004`.
- Deciding same-workspace automatic attach/reuse; that is `CLI-005`.
- Building a full process supervisor or background job manager.
- Cross-machine orchestration.

## Done when

- A user can list relevant tasks/sessions from the CLI with their current status.
- A user can inspect one task/session from the CLI without opening the TUI.
- Status output clearly distinguishes running work from completed or blocked work where the runtime exposes that state.
- The command behavior is documented separately from task send/follow-up behavior.

## Recommended verification

- Start or identify at least one active session and confirm the CLI status command shows it as active/running or equivalent.
- Confirm completed sessions do not look active.
- Confirm the output remains useful in plain terminal usage and, if implemented, JSON output parses cleanly.

## Related Items

- `CLI-001` Copy Cline-style CLI task send conventions
- `FEAT-007` Add advanced coding-session polling
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers
- `CLI-005` Decide whether same-workspace server attach or reuse should exist
- `CLI-007` Add default task target selection for CLI sends
- `CLI-008` Queue CLI task sends while TUI session is open

## Notes

- Split out from the original broad `CLI-001` Cline-workflow placeholder on 2026-09-16.

## Implementation Notes - 2026-09-22

- Branch: `feature/CLI-006-cli-status` stacked on PR #80 (`feature/CLI-001-task-commands`) because it extends the same `task` subcommand surface.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/81.
- Implemented `opencode task status [--server <URL>] [--session <SESSION_ID>] [--json]` in `crates/opencode-cli/src/main.rs`.
- Status reads come from `GET /session/status`; queued position/depth is displayed only from that server response.
- Default output lists root sessions for the selected/provided server; `--session` narrows to one session; `--json` emits parseable session/status objects.
- Verification passed: `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo run -p opencode-cli -- task status --help`.

## Merge Closeout - 2026-09-22

- PR #81 merged into `development` (merge commit `22e114d`), after #80.
- Branch `feature/CLI-006-cli-status` deleted locally and remotely.
- Remains in `qa` pending a recorded post-merge QA report (`H-006` QA notes) or explicit user completion.

## QA Verification - 2026-09-23 (PARTIAL / FAIL on status labeling)

Headless QA on `development` (`54aa9c3`) against a live `opencode serve`.

PASS:

- `opencode task status --server <url>` lists root sessions with `Session`/`Status`/`Queue`/`Title`.
- `--session <id>` narrows to one session.
- `--json` emits a parseable array with `id`, `title`, `directory`, `workspaceIdentity`, and the raw
  `status` object.
- `queued` is read from `GET /session/status` (not synthesized): while a second prompt was queued the
  CLI printed `queued  1/1`, matching the raw server `{status: queued, position: 1, depth: 1}`.
- Unreachable target fails clearly with exit code 1.

FAIL / gap: idle sessions are labeled `active`. `print_task_statuses` prints the lifecycle status when
the run status is idle (`crates/opencode-cli/src/main.rs:2940-2942`), so a session whose turn has
completed still shows `active`. This contradicts the product decision on this card
(`idle|busy|queued|retry|error`, with completed derived from assistant message completion) and fails the
recommended check "confirm completed sessions do not look active." The server already exposes
`idle: true` and lifecycle `completed`; the CLI just does not surface them.

Verdict: listing/JSON/queue all verified; status labeling needs a fix before this card can close.
Recommend keeping in `qa` and logging the idle/active/complete labeling gap as a bug item.

Remediation: `BUG-036` fixes the idle labeling in PR #82
(https://github.com/cchris-p/opencode-modded-rust/pull/82). Re-QA this card after that PR merges.

## QA Re-Verification and Merge Closeout - 2026-09-23

- `BUG-036` (PR #82, merge commit `570eff8`) fixed the idle labeling and merged into `development`.
- Re-QA on the fixed binary: `task status` now shows `idle` for an idle lifecycle-active session,
  `busy` during a live turn, and `queued 1/1` from the server run status; lifecycle `completed`/
  `archived`/`compacting` labels are preserved; `--json` output is unchanged.
- Branch `bug/BUG-036-cli-task-surface-defects` deleted locally and remotely.
- Card moved from `qa` to `done`.