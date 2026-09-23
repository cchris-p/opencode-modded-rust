---
id: "H-006"
title: "CLI-001/CLI-006 CLI task surface and status - Handoff"
status: "in_progress"
created: "2026-09-22"
updated: "2026-09-22"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["CLI-001", "CLI-006"]
---

# CLI-001/CLI-006 CLI Task Surface and Status - Handoff

## Objective

Deliver the Cline-like headless CLI task surface: `opencode task new|send|view` on the canonical
server/session runtime (`CLI-001`) and CLI status visibility (`CLI-006`). Together with the already
implemented detached-server/detach behavior (`CLI-004`, done) and target selection (`CLI-007`, merged),
these make the product usable without a TUI.

Canonical behavior reference: `wiki/cli-surface.md`.
Binding invariants: `invariants/cli-task-targeting.md`, `invariants/message-queuing.md`,
`invariants/runtime-lifecycle.md`.

## Included Board Items

- `CLI-001` Copy Cline-style CLI task send conventions.
- `CLI-006` Add CLI status visibility for tasks and background sessions.

## Why This Composition

Both cards build the same `opencode task` command surface against the existing canonical
`POST /session/{id}/prompt` runtime and `GET /session/status`. They are the two prerequisite gates for
every other CLI story. They are independently reviewable (send/view vs status), so they stay as separate
PRs, but they belong in one handoff because `CLI-001` defines the `task` subcommand surface that
`CLI-006` extends.

## Prerequisites Already In Place

- `GATE-001` (done): per-session FIFO queue, `started`/`queued` responses, `Queued { position, depth }`
  status, and the `QUEUED` badge.
- `CLI-007` (merged, PR #37, in `qa`): `opencode task target list|select|show|clear`; `CLI-001` consumes
  the selected target.
- `CLI-004` (done): explicit `/detach` leaves a TUI-launched server alive for headless clients.

## Dependency Order

1. `CLI-001` - defines the `task new|send|view` subcommands and consumes `CLI-007`'s target.
2. `CLI-006` - uses the same command surface and `GET /session/status`; may land after `CLI-001` or, if
   `CLI-001` is delayed, independently as a `task status` surface.

Inferred dependency: `CLI-006`'s output advertises queued state, which only exercises meaningfully after
`CLI-001` can submit work; hence `CLI-001` first. Rationale: reading status is useful alone, but its QA
is strongest once sends exist.

## PR Plan

| PR | Board Items | Branch | Why this grouping | Merge rule |
|----|-------------|--------|-------------------|------------|
| 1 | `CLI-001` | `feature/CLI-001-task-commands` | The core Cline-like send/view surface. | Merge after QA; prerequisite for `CLI-006`'s strongest QA. |
| 2 | `CLI-006` | `feature/CLI-006-cli-status` | Separable status surface; shares only the `task` subcommand. | May merge after PR 1, or independently if PR 1 slips. |

Rationale: one PR per item preserves traceability and independent QA; the two are not tightly coupled
enough to require a single PR.

## Merge Target

All implementation PRs target `development`. The purpose is to land dev work there so QA and testing
happen on `development`. This handoff does not define deployment to `main`.

## Merge Strategy

- PR 1 (`CLI-001`) merges first once its QA passes.
- PR 2 (`CLI-006`) merges after PR 1, or independently if PR 1 is delayed; no hard code dependency.
- Batch merge is acceptable only if both PRs are QA-ready at the same time and reviewers accept the
  combined scope; otherwise merge separately.
- Do not merge solely because tests pass; wait for explicit user merge direction.

## QA Notes

After PR 1 merges to `development`:

- `opencode task new "..."` creates a session and submits through `/session/{id}/prompt`.
- `opencode task new < prompt.md` preserves stdin prompt text.
- `opencode task send "..."` uses the selected/current target; `--server`/`--session` override the target.
- `opencode task view` prints the selected session without opening the TUI.
- file paths in prompt text are readable by the agent through normal tools.
- `--stream` reports `queued` (with position) then follows once active.
- normal `ort` launch/exit lifecycle is unchanged.

After PR 2 merges to `development`:

- status shows at least one busy session and a queued session with position/depth;
- completed sessions do not look active; `--json` output parses.

Cross-PR: run status reads must come from `GET /session/status`; do not synthesize `queued` client-side.

## Branch Cleanup

After each PR merges into `development`:

- delete the local branch: `git branch -d <branch>`
- delete the remote branch: `git push origin --delete <branch>`

Only the branches created by this handoff. Do not delete unrelated branches. If PR 2 remains open while
PR 1 merges, clean up PR 1's branch only.

## Execution Sequence

1. Pull latest `development`.
2. Branch `feature/CLI-001-task-commands`; implement `task new|send|view` on the canonical prompt path.
3. Update `README.md` CLI overview and `docs/opencode-cli.md` for the new commands.
4. Open PR 1 targeting `development`, referencing `CLI-001` and `wiki/cli-surface.md`; move `CLI-001` to `qa`.
5. On user merge direction: merge PR 1 into `development`, delete its local and remote branches.
6. Branch `feature/CLI-006-cli-status`; implement `task status` from `GET /session/status`; open PR 2.
7. On user merge direction: merge PR 2, delete its branches; move `CLI-006` to `done` after QA passes.
8. Compose the follow-up handoff for the deferred dependents below once these gates land.

## Deferred Dependents (not covered here)

These are not implementation-ready for this handoff and are intentionally excluded:

- `CLI-002` Route `opencode run` through the canonical session runtime - depends on `CLI-001`/`CLI-006`
  and embeds a routing decision (coordinate with `FEAT-011`). Compose after this handoff.
- `CLI-009` CLI and direct-run question parity - blocked by `GATE-002` and `CLI-002`; absorbed the former
  `CLI-011` scope.
- `CLI-010` CLI subagent surface parity - blocked by `GATE-004` (open) and `CLI-001`/`CLI-006`.
- `CLI-005` Same-workspace attach/reuse decision - human gate (`attention`); requires explicit 110%
  confirmation before any implementation.
- `CLI-007` Default task target selection - prerequisite input, already merged (PR #37, in `qa`); its QA
  can proceed independently of this handoff.

## Readiness Assessment

`CLI-001` and `CLI-006` are implementation-ready: scope, product decisions, done-when, verification,
binding invariants, and prerequisites are explicit, and both are unblocked. Each deferred item has an open
decision or an open gate and must be composed later.

## Execution Notes

### 2026-09-22 - PR 1 / `CLI-001`

- Branch: `feature/CLI-001-task-commands`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/80.
- Implemented `opencode task new|send|view` against explicit/selected task targets and the canonical `POST /session/{id}/prompt` path.
- `task new` creates a target-server session and stores it as the default only after server acknowledgement.
- `task send` uses explicit `--server`/`--session` overrides or the selected default target.
- `task view` prints the selected target server's transcript without opening the TUI; `--json` emits the message array.
- Verification passed: `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo run -p opencode-cli -- task --help`; `cargo run -p opencode-cli -- task new --help`; `cargo run -p opencode-cli -- task send --help`; `cargo run -p opencode-cli -- task view --help`.

### 2026-09-22 - PR 2 / `CLI-006`

- Branch: `feature/CLI-006-cli-status`, stacked on PR #80 because it extends the same `task` command surface.
- Implemented `opencode task status [--server <URL>] [--session <SESSION_ID>] [--json]` from `GET /session/status`.
- Plain output lists server sessions by default and includes queued position/depth when the server reports it; `--session` narrows to one session.
- Verification passed: `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo run -p opencode-cli -- task status --help`.
