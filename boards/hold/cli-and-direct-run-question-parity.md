---
id: "FEAT-042"
title: "CLI and direct-run question parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "hold"
predecessors: "FEAT-012"
created: "2026-09-21"
---

# CLI and direct-run question parity

## Summary

Child of `GATE-002` (parity gap 5). When the Rust CLI/direct-run task surface exists, handle pending
questions there without requiring the full TUI, reusing the shared prompt-state logic rather than
duplicating divergent behavior.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 5.

## Blocked By

- `FEAT-012` CLI/AgentExecutor tool-loop parity (`boards/hold/cli-agentexecutor-tool-loop-parity.md`)
  is on hold until `GATE-002` passes. It must first attach tools and run a tool loop, otherwise there
  is no interactive CLI surface to route questions through.
- This card stays in `hold` until `FEAT-012` (or an equivalent direct-run surface) lands.

## Problem

- The Rust CLI has no interactive run queue/footer question surface; the `question` tool currently
  falls back to stdin (`crates/opencode-tool/src/question.rs:130-212`).
- Vanilla routes direct-run questions through a shared footer state machine
  (`packages/opencode/src/cli/cmd/run/footer.question.tsx`, `question.shared.ts`).

## Scope / deliverables

- Once a CLI/direct-run surface exists, route pending questions through the same session question
  callback the TUI uses (no stdin).
- Reuse shared prompt-state logic for single/multi/custom/reject where practical.
- Match vanilla's direct-run question display/behavior at the frozen reference commit.

## Non-goals

- TUI prompt UX (`FEAT-041`).
- Building the CLI tool loop itself (that is `FEAT-012`).

## Acceptance criteria

- [ ] The CLI/direct-run surface answers pending questions without reading stdin directly.
- [ ] Single, multi-select, custom, and reject flows work in the CLI surface.
- [ ] Behavior is verified against the frozen vanilla reference.

## Verification

- `cargo fmt --all`
- `cargo check -p opencode-cli`
- Manual smoke via the CLI direct-run surface once it exists.

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-012` CLI/AgentExecutor tool-loop parity - predecessor; on hold.
- `FEAT-041` TUI question prompt UX parity - shared prompt logic.
