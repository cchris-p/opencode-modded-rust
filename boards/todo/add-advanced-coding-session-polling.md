---
id: "FEAT-005"
title: "Add advanced coding-session polling"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "wiki/advanced-coding-session-polling.md"
status: "todo"
created: "2026-09-08"
---

# Add advanced coding-session polling

## Summary

Define a low-context polling capability for coding sessions so one session or agent can wait for another coding session's external state to become true without carrying the full conversation or repeatedly asking the model to inspect everything manually.

## What this means right now

Advanced polling means runtime-supported waiting for concrete coding-task state, such as git state, pushed commits, PR state, branch availability, CI/check results, board movement, or another agent/session publishing a result.

The key idea is a little-to-no-context wait: the waiting session should be able to ask for a specific observable condition and get resumed, notified, or given a compact result when that condition becomes true.

## Why this exists

Multi-agent coding work often needs one session to pause until another session finishes a bounded external action. Today that waiting tends to consume context, rely on manual refreshes, or require a model to repeatedly poll broad repository state. This feature should make waiting explicit, cheap, and state-based.

## Scope

- Define the polling request shape at a high level.
- Define what can be observed initially, such as git branch/commit status, pushed remote refs, PR/check status, board lane movement, and session result availability.
- Define how a polling result should be represented without importing full source-session context.
- Define whether polling belongs to the runtime, a tool boundary, the TUI, or a combination of those surfaces.
- Support background sessions explicitly, including sessions that are not the currently visible TUI session.
- Keep this as a planning/refinement item until exact polling surfaces and state contracts are agreed.

## Non-goals

- Building a general distributed job scheduler.
- Replacing explicit session/task state.
- Streaming another session's full transcript into the waiting session.
- Making the model decide unbounded polling strategy without a runtime contract.
- Solving all multi-agent orchestration in the first implementation.

## Done when

- The product has a clearly documented advanced polling contract for coding-session waits.
- Background coding sessions are explicitly covered by the design.
- The design supports low-context waiting on another agent/session state becoming present.
- Initial observable state categories are listed with TBDs where implementation details are unresolved.
- Follow-up implementation cards can be split from this item without redefining the core concept.

## TBD

- Exact first polling surface: TUI command, runtime API, tool, skill, or all of these.
- Exact first observable condition set.
- Whether polling results should wake a paused session automatically or only notify the user.
- How polling relates to durable structured task state.
- How polling identifies another coding session safely.
- Whether cross-repository polling is in scope for V1 or later.

## Related Items

- `FEAT-002` Keep sessions running after TUI exit
- `FEAT-003` Add compact fork context for session branching
- `FEAT-004` Add in-session send-to-fork commands
- `START-005` Define V1 runtime loop
- `START-016` Define structured task state for V1
- `START-025` Add retrieval-provider boundary for task context assembly

## Notes

- Keep this item high level until the intended polling surfaces are clarified.
- The implementation should prefer observable external state over transcript-derived guesses.
