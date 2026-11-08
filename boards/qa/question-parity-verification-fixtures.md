---
id: "FEAT-044"
title: "Question parity verification fixtures"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: ""
created: "2026-09-21"
---

# Question parity verification fixtures

## Summary

Child of `GATE-002` (parity gap 7). Add focused tests/fixtures for question behavior across schema,
tool output, ownership, lifecycle, and TUI state so the gate can be verified without manual-only
checks.

## Parent

`GATE-002` question-tool full parity (`boards/qa/gate-question-tool-full-parity.md`), gap 7.

## Problem

There are currently no tests for the question feature in any crate. Verification today is manual
smoke only (`boards/qa/gate-question-tool-full-parity.md:118-120`, `:157`).

## Scope / deliverables

Focused tests or smoke fixtures for:

- Schema parsing: required `header`, required option `description`, `custom` default true, `multiple`
  default false.
- Model output formatting: single answer, multi-select join, custom text, unanswered.
- Tool contract: the `question` tool uses the callback and never stdin; missing callback errors.
- Ownership: wrong-session reply/reject is rejected; correct-session reply succeeds.
- Lifecycle: reply/reject/drop cleanup; rejection unblocks the waiter.
- TUI state transitions: single select, multi select, custom answer, review/confirm, reject.

## Non-goals

- New product behavior; this card only adds verification coverage for the other children.

## Acceptance criteria

- [x] Each fixture above exists and passes in CI.
- [x] The suite fails if the tool regresses to stdin or if ownership checks are removed.
- [x] A short side-by-side parity note against the frozen reference commit is recorded in the gate.

## Verification

- `cargo fmt --all`
- `cargo test -p opencode-tool -p opencode-server -p opencode-tui`

## Dev Notes - 2026-09-22

- Consolidated coverage across the child cards:
  - Schema, model output, callback contract, and no-stdin behavior:
    `cargo test -p opencode-tool question` (`crates/opencode-tool/src/question.rs`).
  - Session ownership, abort cleanup, guard drop, and resolution-to-error mapping:
    `cargo test -p opencode-server question` (`crates/opencode-server/src/routes.rs`).
  - Permission tool-availability: `cargo test -p opencode-permission` and
    `cargo test -p opencode-server agentic`.
  - TUI state transitions for single/multi/custom/review/cancel:
    `cargo test -p opencode-tui --lib` (`crates/opencode-tui/src/components/question.rs`).
- Removed the final manual-only gap by extracting `question_resolution_result` so rejection vs.
  dropped-waiter behavior is asserted directly.
- Recorded the side-by-side parity note against the frozen reference commit
  `f54ce313b99a6661d7758ad042f7a6e05c8e0972` in `GATE-002` "Side-by-Side Parity Note".
- Committed directly to `development` per maintainer direction.

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-038` question schema and tool contract parity.
- `FEAT-039` session-scoped question API parity.
- `FEAT-040` question runtime event and lifecycle parity.
- `FEAT-041` TUI question prompt UX parity.