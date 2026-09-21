---
id: "FEAT-051"
title: "Subagent parity verification fixtures"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Subagent parity verification fixtures

## Summary

Child of `GATE-004` (parity gap 7). Add focused fixtures that pin the subagent behaviors the gate
depends on, so the parity work in `FEAT-045` through `FEAT-050` is verifiable and regressions are
caught.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 7.

## Problem

- The only Rust subagent tests are the two task-tool unit tests built around the in-memory subsession
  path (`crates/opencode-tool/src/task.rs:247-378`); they will not survive the child-session
  migration unchanged.
- There are no fixtures for child session creation, `task_id` resume, depth limits, subagent
  permissions, TUI child navigation, background notification injection, or agent-role filtering.
- Parity claims in `GATE-004` need reproducible checks, not just manual smoke.

## Vanilla reference

Reference `f54ce313b99a` test coverage to mirror conceptually:

- `packages/opencode/test/tool/task.test.ts`
- `packages/opencode/test/agent/agent.test.ts`
- `packages/app/e2e/regression/subagent-child-navigation.spec.ts`
- `packages/opencode/test/cli/run/subagent-data.test.ts`

## Scope / deliverables

- Task tests: creates exactly one child session with `parent_id`; title format `"<desc> (@<agent>
  subagent)"`; `task_id` resumes the same session; unknown agent errors; depth limit enforced.
- Permission tests: subagent session ruleset derives parent denies plus default `todowrite`/`task`
  denies.
- TUI tests: child set computation and ordering; `ctrl+x down` selects the first child; left/right
  cycle; `up` returns to parent; enablement rules.
- Background tests: gate-off error; gate-on immediate return; completion and failure notification
  injection; cancel propagation.
- Agent-role tests: registry subagent filtering; `@agent-name` routing; unknown mention error.
- A documented side-by-side parity evidence procedure against the (re-pinned) reference.

## Acceptance criteria

- Each `GATE-004` acceptance criterion has at least one automated or explicitly scripted fixture.
- The fixtures fail before the corresponding child item lands and pass after.
- Fixtures are resilient to the in-memory → child-session migration (no assertions on synthetic
  `task_*` ids).
- A short parity evidence runbook is recorded in this card or a linked invariant.

## Verification

- `cargo test -p opencode-tool -p opencode-agent -p opencode-session -p opencode-server -p opencode-tui`
- Confirm every `GATE-004` acceptance bullet maps to a fixture in this card's coverage table.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-045` through `FEAT-050` - the work these fixtures verify.
