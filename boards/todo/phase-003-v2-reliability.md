---
id: "PHASE-003"
title: "V2 reliability"
priority: "P2"
type: "epic"
area: "PHASE"
spec: "wiki/v2.md"
status: "todo"
created: "2026-09-10"
---

# V2 reliability

## Summary

Extend the hardened V1 daily-driver into dependable multi-step work: stronger task decomposition, better context assembly and state transitions, and more dependable reviewer/repair loops, without broadening product scope.

## Why this exists

Per `wiki/v2.md`, V2 improves reliability on larger multi-step work while preserving the V1 architecture. Once the V1 loop and tool behavior are trustworthy, the next limit is reliability across longer task chains and better context/retrieval quality.

## Scope

- `START-025` Add retrieval-provider boundary for task context assembly.
- `FEAT-007` Advanced coding-session polling.
- Stronger decomposition, repair-after-failure, and planning/execution/review handoff behavior (see `wiki/v1-runtime-loop.md`, `wiki/v2.md`).
- Early `ScopeMux` integration if mature enough to improve retrieval quality.

## Non-goals

- Full OpenCode parity (`START-008`).
- Desktop app support, broad MCP ecosystem completeness (per `wiki/v2.md`).

## Done when

- Multi-step work inside one repository session is reliably decomposed, executed, verified, and repaired on failure.
- Context assembly and state transitions are dependable enough for larger task chains.

## Related Items

- `START-025` Add retrieval-provider boundary for task context assembly
- `FEAT-007` Add advanced coding-session polling
- `FEAT-003` Add compact fork context for session branching
- `FEAT-004` Add in-session send-to-fork commands
- `PHASE-001` V1 daily-driver hardening (prerequisite)

## Notes

- Grouping/tracking parent; child cards remain in their own lanes.
- Keep V2 work scoped to reliability gains, per `wiki/v2.md` non-goals.
