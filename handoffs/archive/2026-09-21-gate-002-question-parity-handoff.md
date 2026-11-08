---
id: "H-005"
title: "GATE-002 question tool full parity - Handoff"
status: "closed"
created: "2026-09-21"
updated: "2026-09-23"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["GATE-002", "FEAT-039", "FEAT-040", "FEAT-041", "FEAT-043", "FEAT-044", "CLI-009", "CLI-002"]
---

# GATE-002 Question Tool Full Parity - Handoff

## Objective

Finish `GATE-002`: the Rust product's question tool and question surfaces must match vanilla OpenCode's
core question behavior, with any deliberate UX/product deviations explicitly documented on the gate or
child cards before the gate is considered complete.

This handoff is for follow-up work after `FEAT-038` landed. Do not treat `GATE-002` as implemented yet.

## Binding Artifacts

- `GATE-002` (`boards/qa/gate-question-tool-full-parity.md`) is the umbrella gate and binding acceptance
  spec.
- `FEAT-038` (`boards/qa/question-schema-and-tool-contract-parity.md`) is merged and in QA; use it as the
  completed baseline, not as evidence that the whole gate passed.
- Frozen reference line per current `AGENTS.md`: `$HOME/repos/opencode-modded` `dev` at
  `f54ce313b99a6661d7758ad042f7a6e05c8e0972` unless re-pinned by a later gate. `GATE-002` now uses this
  pin (refreshed 2026-09-21 closeout), so side-by-side comparisons can proceed directly.

## Current State (2026-09-23, archived)

- `FEAT-038` is merged into `development` at merge commit `405a6e1ad6651e77897b08da02d3ed886a8be77f`
  (PR #65) and in `qa`.
- `FEAT-039` is merged into `development` at merge commit `cf47328d6e384a4f079d1225f79d7d2011bb20c5`
  (PR #67) and in `qa`.
- `FEAT-040` (`53b6e65`), `FEAT-041` (`9c24c7b`), and `FEAT-043`/`FEAT-044` (`b061656`) were committed
  directly to `development` and are in `qa`.
- `GATE-002` moved from `todo` to `qa` on 2026-09-23; it is awaiting the maintainer's local TUI smoke
  and `FEAT-039` post-merge QA confirmation before `done`.
- `CLI-009` remains in `hold`, deferred behind `CLI-002` (both listed on the `GATE-002` card). Their
  prerequisite `CLI-001`/`CLI-006` are now `done`, but this handoff never owned the CLI routing work.
- This handoff is archived; the remaining closeout state lives on `GATE-002`.

## Included Board Items

- `GATE-002` Gate: Question tool must match vanilla OpenCode exactly.
- `FEAT-039` Session-scoped question API parity.
- `FEAT-040` Question runtime event and lifecycle parity.
- `FEAT-041` TUI question prompt UX parity.
- `FEAT-043` Question permission integration parity.
- `FEAT-044` Question parity verification fixtures.
- `CLI-009` CLI and direct-run question parity, only after `CLI-001`/`CLI-006` and `CLI-002` create the
  needed CLI surface.

## Excluded / Not Owned Here

- `FEAT-038` implementation is not reopened here unless QA finds a regression.
- `CLI-002` itself is not implemented in this handoff; it is a prerequisite for `CLI-009`.
- Browser/web app question dock parity is out of scope for the Rust product unless that surface is added later.
- Persisting pending questions across server restart remains a documented non-goal unless a later board item
  changes it.

## Code Evidence At Handoff Time

- Server question routes are still global: `crates/opencode-server/src/routes.rs:4926-4930` exposes
  `/question`, `/question/{id}/reply`, and `/question/{id}/reject`.
- `list_questions` returns all pending requests without a session filter
  (`crates/opencode-server/src/routes.rs:4965-4969`).
- `reply_question` and `reject_question` remove by request ID only; they do not validate ownership against an
  addressed session (`crates/opencode-server/src/routes.rs:4977-5042`).
- The TUI client still calls global endpoints in `crates/opencode-tui/src/api.rs:840-895`.
- Ask currently broadcasts generic `session.updated` with `source: "question.request"`
  (`crates/opencode-server/src/routes.rs:2712-2719`).
- Reply/reject already broadcast explicit `question.replied` / `question.rejected` events as of the current
  checkout (`routes.rs:4992-5033`), but also still broadcast `session.updated`.
- Rejection still maps to a generic execution error string (`routes.rs:2721-2728`), not a distinct question
  rejection error type on the server callback path.
- TUI question rendering is still basic: `QuestionOption` has only `id` and `label`, uses letter shortcuts,
  and renders no option descriptions (`crates/opencode-tui/src/components/question.rs:21-25,110-130,213-236`).
- TUI flow still steps through multi-question requests sequentially and replies only at the end; there is no
  review/confirm screen (`crates/opencode-tui/src/app/app.rs:3068-3157`).
- TUI API structs carry `custom`, but the prompt component does not yet expose custom answers for choice
  questions.

## Recommended Implementation Sequence

### PR 1 - `FEAT-039` Session-Scoped Question API

Branch: `feature/FEAT-039-session-scoped-question-api`

1. Add session-scoped routes equivalent to vanilla semantics:
   `GET /session/{id}/question`, `POST /session/{id}/question/{requestID}/reply`, and
   `POST /session/{id}/question/{requestID}/reject`.
2. Enforce ownership: wrong-session reply/reject must return not found and must not resolve the original
   waiter.
3. Update `opencode-tui` API and app code to use session-scoped list/reply/reject.
4. Keep or deprecate global routes only if there is a concrete compatibility need; if kept, avoid making the
   TUI depend on them.
5. Add focused server tests for correct-session success, wrong-session rejection, and missing request handling.

Verification gate:

- `cargo fmt --all`
- `cargo check -p opencode-server -p opencode-tui`
- `cargo test -p opencode-server question`

### PR 2 - `FEAT-040` Runtime Events and Lifecycle

Branch: `feature/FEAT-040-question-runtime-lifecycle`

1. Add an explicit ask event (`question.asked` or the project-approved equivalent) carrying `sessionID` and
   `requestID`; keep event names aligned with the gate's chosen vanilla mapping.
2. Preserve the existing explicit reply/reject events, but audit payloads against vanilla/reference needs.
3. Replace generic rejected-question execution errors with a clear question-rejection error path that the model
   and transcript can distinguish from tool failure or dropped waiters.
4. Ensure pending state is removed after reply, reject, waiter drop/cancel, and session/runtime shutdown where
   feasible.
5. Add tests for reply cleanup, reject/unblock, drop cleanup, and session abort/shutdown behavior.

Verification gate:

- `cargo fmt --all`
- `cargo check -p opencode-server -p opencode-session`
- `cargo test -p opencode-server question`

### PR 3 - `FEAT-041` TUI Question Prompt UX

Branch: `feature/FEAT-041-tui-question-prompt-ux`

1. Add option descriptions to `QuestionOption` and render them under labels.
2. Support digit shortcuts (`1`..`9`) while preserving Rust TUI navigation conventions where sensible.
3. Add custom-answer affordance for choice questions when `custom` is true; typed input must return as an
   answer label/string in the same per-question array shape.
4. Implement the resolved deliberate deviation: sequential multi-question flow plus explicit review/confirm
   screen before final submission when there is more than one question.
5. Add submitting/error-recovery states and keep reject/dismiss from hanging the session.
6. Record any UX deviations from vanilla in `FEAT-041` and `GATE-002`.

Verification gate:

- `cargo fmt --all`
- `cargo check -p opencode-tui`
- `cargo test -p opencode-tui --lib -- --test-threads=1`
- Manual smoke with `ort-build` then `ort`: single select, multi select, custom answer, multi-question review,
  and reject.

### PR 4 - `FEAT-043` Permission Integration Decision

Branch: `feature/FEAT-043-question-permission-integration`

1. Confirm the existing ruleset/tool-list filtering remains the intended Rust behavior for `question`.
2. Add a focused test that `question` is offered to allowed agents and withheld where the ruleset denies it.
3. Document the deliberate deviation from vanilla's execution-time `question` permission assertion on the card
   and gate.

Verification gate:

- `cargo fmt --all`
- `cargo check -p opencode-permission -p opencode-server`
- `cargo test -p opencode-permission question` plus the focused agentic tool-resolution test.

### PR 5 - `FEAT-044` Verification Fixtures and Gate Closeout

Branch: `feature/FEAT-044-question-parity-fixtures`

1. Consolidate focused fixtures for schema parsing, model output formatting, callback/no-stdin routing,
   session ownership, lifecycle cleanup, and TUI state transitions.
2. Add or update a short side-by-side parity note against the current frozen reference commit in `GATE-002`.
3. Move completed child cards through `qa`/`done` only after user verification according to the normal board
   workflow.
4. Close `GATE-002` only when all child cards are done or explicitly resolved as documented deviations.

Verification gate:

- `cargo fmt --all`
- `cargo test -p opencode-tool -p opencode-server -p opencode-tui`
- Final manual smoke through the real TUI with the latest `ort-build` binary.

### Deferred - `CLI-009` CLI and Direct-Run Question Parity

Do not start `CLI-009` until `CLI-001`/`CLI-006` land the canonical CLI task/status surface and `CLI-002`
routes the CLI model loop, so pending questions can be exercised outside the TUI. Keep `CLI-009` in `hold`
until then. `CLI-001`/`CLI-006` are owned by `H-004`
(`handoffs/archive/2026-09-21-session-prompt-queue-gate-handoff.md`, which folded and archived the former `H-003`);
check that handoff for their status rather than duplicating their plan here.

## PR Workflow

- Create one branch and PR per phase, targeting `development`.
- Keep each board item in `qa` after its PR is ready for local verification.
- Do not merge based only on tests passing; wait for explicit user merge direction after local testing.
- Stage only files relevant to the active board item; preserve unrelated worktree changes.
- After each merge, update the child card with PR link, verification, closeout notes, and any documented
  deviation that affects `GATE-002`.

## Readiness Assessment

The remaining question-parity work is implementation-ready as separate PRs. The highest-value next step is
`FEAT-039`, because session-scoped ownership is foundational and reduces risk for the lifecycle and TUI work.
Before starting code, refresh `GATE-002`'s vanilla reference evidence to the current pinned reference commit in
`AGENTS.md` so future comparisons do not rely on the superseded `e62912b...` pin.

## Execution Notes

### 2026-09-21 - PR 1 / `FEAT-039`

- Branch: `feature/FEAT-039-session-scoped-question-api`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/67.
- Implemented session-scoped question list/reply/reject routes and wrong-session ownership checks.
- Updated `opencode-tui` API usage to call `/session/{id}/question` endpoints and carry the session ID through reply/reject.
- Refreshed `GATE-002`'s frozen reference commit from the superseded `e62912b...` pin to `f54ce313b99a6661d7758ad042f7a6e05c8e0972`.
- Verification passed: `cargo fmt --all`; `cargo check -p opencode-server -p opencode-tui`; `cargo test -p opencode-server question`.
- Completed with PR #67 merged into `development` at `cf47328d6e384a4f079d1225f79d7d2011bb20c5`; PR branch cleanup complete. `FEAT-039` remains in `qa` for post-merge verification.

### 2026-09-22 - Direct-to-`development` follow-up (FEAT-040/041/043/044)

Per maintainer direction the remaining children were implemented decoupled by layer and committed
directly to `development` (no PR branches):

- `53b6e65` `feat(question): runtime events, lifecycle cleanup, typed rejection (FEAT-040)` -
  explicit `question.asked` event, typed `QuestionRejected` error, drop guard, and abort/delete
  waiter rejection in `crates/opencode-server/src/routes.rs`.
- `9c24c7b` `feat(question): TUI prompt descriptions, digit keys, custom answers, review (FEAT-041)` -
  `crates/opencode-tui/src/components/question.rs` and `app.rs`; sequential multi-question flow with a
  review/confirm screen (documented deviation from vanilla's tabbed flow).
- `b061656` `test(question): permission gating and parity fixtures (FEAT-043, FEAT-044)` -
  permission ruleset and `resolve_tools` tests, plus the side-by-side parity note on `GATE-002`.

State after these commits:

- `FEAT-040`, `FEAT-041`, `FEAT-043`, `FEAT-044` are in `qa` awaiting maintainer local verification.
- `FEAT-039` remains in `qa`.
- `GATE-002` stays in `todo` with the full parity/deviation note; it closes only after the local TUI
  smoke and `FEAT-039` post-merge QA pass.
- `CLI-009`/`CLI-002` remain on `hold`; this handoff is now scoped to TUI/server question parity.
- Handoff is **not** archive-ready yet; archive only after `GATE-002` closes.

### 2026-09-23 - Handoff closeout and archive

- Reconciled this handoff to reality: the TUI/server question-parity children (`FEAT-038` through
  `FEAT-044`) are all merged into `development` and sitting in `qa`.
- Moved `GATE-002` from `todo` to `qa`; it now owns the remaining verification closeout (maintainer
  local TUI smoke plus `FEAT-039` post-merge QA).
- `CLI-009` and `CLI-002` stay on `hold` as deferred scope, documented on the `GATE-002` card.
- Handoff closed and archived per maintainer direction; no further work is planned from it.
