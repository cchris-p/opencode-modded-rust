---
id: "START-004"
title: "Assess current Rust state"
priority: "P1"
type: "research"
area: "START"
spec: ""
status: "doing"
created: "2026-08-28"
---

# Assess current Rust state

## Summary

Determine whether the existing Rust codebase is a strong foundation for the V1 product goal or whether parts of it should be treated as reference-only.

Produce the assessment as a dedicated wiki document and summarize the key conclusions on this board item.

## Why this exists

The repository already contains a substantial Rust implementation, but its readiness for the new product direction has not been evaluated against the current V1 goals.

## Questions to answer

- Which parts are already useful for a serious personal daily-driver on a narrow workflow?
- Which parts are incomplete, overly broad, or misaligned with the new architecture?
- Which subsystems are worth preserving for V1?
- Which subsystems should be deferred, replaced, or ignored?

## Scope

- Assess this repository as the product codebase of record.
- Use `$HOME/repos/opencode-modded` only as a reference point for comparison, planning context, or terminology.
- Judge fit against `wiki/v1.md`, not against upstream feature parity.

## Deliverables

- A wiki assessment document that records reusable subsystems, risky areas, gaps, and V1-fit conclusions.
- A concise summary on this board item linking to that wiki document.
- Follow-up board items for all notable gaps or risks discovered during the assessment.

## Done when

- A written assessment identifies reusable subsystems, risky areas, and gaps relative to `wiki/v1.md`.
- The assessment clearly distinguishes product-code findings in this repo from reference-only observations drawn from `$HOME/repos/opencode-modded`.
- Follow-up board items exist for all notable gaps.

## Verification

- The wiki assessment document exists in this repository and is linked from this board item.
- The assessment cites the code areas or subsystems it evaluates.
- New follow-up board items are specific enough to implement without re-running the assessment.

## Notes

- This is an alignment task, not a parity audit.
- Evaluate the current code against the new product goals rather than against upstream OpenCode.
- Cross-repo references are allowed for context, but this repository remains authoritative.

## Assessment Summary

- Assessment document: `wiki/current-rust-state.md`
- The current Rust repo is a strong V1 foundation in its TUI, session/storage, provider, and core tool layers.
- The biggest gap is architectural: the runtime still behaves like a broad chat-and-tool loop instead of a runtime-owned staged task workflow.
- Durable task state, verification/review enforcement, bounded-task context construction, and TUI approval handling still need explicit follow-up work.
- Broad parity-oriented surfaces should mostly be treated as reference-only or non-primary until the narrow V1 workflow is solid.

## Related Items

- `START-005` Define V1 runtime loop
- `START-007` Plan ScopeMux integration
- `START-016` Define structured task state for V1
- `START-017` Enforce verification and review stages in runtime
- `START-018` Complete TUI approval and question handling
- `START-019` Define local-model-first provider path
- `START-020` Constrain primary product surface to the V1 workflow

## Dev Notes

- Added `wiki/current-rust-state.md` as the main written assessment artifact.
- Compared the Rust repo against `wiki/v1.md`, current invariants, and the frozen TypeScript reference repo for context only.
- Captured the main conclusion that the codebase is worth preserving, but the runtime/task-state/verification architecture still needs focused V1 work.
- Created follow-up board items for the notable gaps that were not already covered by existing planning cards.
