---
id: "CLI-009"
title: "CLI and direct-run question parity"
priority: "P2"
type: "feature"
area: "CLI"
spec: ""
status: "hold"
predecessors: "CLI-001, CLI-006, CLI-002, GATE-002"
created: "2026-09-21"
updated: "2026-09-22"
---

# CLI and direct-run question parity

## Summary

Child of `GATE-002` (parity gap 5). When the Rust CLI/direct-run task surface exists, handle pending
questions there without requiring the full TUI, reusing the shared prompt-state logic rather than
duplicating divergent behavior.

This card also absorbs the former `CLI-011` "CLI/AgentExecutor ask and approval parity" (split from
`CLI-002` on 2026-09-22 and folded back here the same day): the CLI ask/approval path is the
direct-run question path, so it needs one owner, not two. Because `CLI-002` routes `opencode run`
through the canonical session runtime, ask handling is expected to reuse the shared session question
logic rather than a separate `AgentExecutor` implementation.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 5.

## Blocked By

- `CLI-001` Copy Cline-style CLI task send conventions (prerequisite gate: canonical direct-run task surface).
- `CLI-006` Add CLI status visibility for tasks and background sessions (prerequisite gate).
- `CLI-002` Route `opencode run` through the canonical session runtime
  (`boards/hold/route-opencode-run-through-session-runtime.md`) is on hold until `GATE-002` passes. The CLI
  model loop/routing must land first so ask handling attaches to one canonical loop, otherwise there is no
  interactive CLI surface to route questions through.
- This card stays in `hold` until `CLI-001`, `CLI-006`, and `CLI-002` land.

## Gate Note - 2026-09-22

- `CLI-001`/`CLI-006` were reactivated on 2026-09-22, so the direct-run task surface premise is restored.
  Do not start this card until those gates land.

## Problem

- The Rust CLI has no interactive run queue/footer question surface; the `question` tool currently
  falls back to stdin (`crates/opencode-tool/src/question.rs:130-212`).
- Vanilla routes direct-run questions through a shared footer state machine
  (`packages/opencode/src/cli/cmd/run/footer.question.tsx`, `question.shared.ts`).

## Scope / deliverables

- Once a CLI/direct-run surface exists, route pending questions through the same session question
  callback the TUI uses (no stdin).
- Provide or reuse a real ask/approval path so `Ask`-gated tools on the CLI/run path are not silently
  hard-denied or errored without a user surface (the former `CLI-011` scope).
- Reuse shared prompt-state logic for single/multi/custom/reject where practical.
- Match vanilla's direct-run question display/behavior at the frozen reference commit.

## Non-goals

- TUI prompt UX (`FEAT-041`).
- Building the CLI tool loop itself (that is `CLI-002`).
- An `AgentExecutor`-specific ask implementation: `CLI-002` routes `opencode run` through the canonical
  session runtime, so this card reuses the shared question path instead.

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
- `CLI-001` Copy Cline-style CLI task send conventions - prerequisite gate.
- `CLI-006` Add CLI status visibility for tasks and background sessions - prerequisite gate.
- `CLI-002` Route `opencode run` through the canonical session runtime - predecessor; on hold.
- Former `CLI-011` "CLI/AgentExecutor ask and approval parity" - folded into this card on 2026-09-22.
- `FEAT-041` TUI question prompt UX parity - shared prompt logic.
