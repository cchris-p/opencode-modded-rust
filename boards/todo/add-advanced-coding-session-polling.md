---
id: "FEAT-007"
title: "Add advanced coding-session polling"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "wiki/advanced-coding-session-polling.md"
status: "doing"
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

## Resolved Decisions

These resolve the item's original TBDs and are authoritative in `wiki/advanced-coding-session-polling.md`.

- **First surface:** an agent tool (id `wait_for_state`) registered in `crates/opencode-tool`. Runtime and TUI surfaces are deferred.
- **First observables:** local and remote git refs and branch state only.
- **Wait model:** the tool call itself is the bounded wait. No durable runtime poll object and no background poller in the first slice.
- **Wake model:** no auto-wake; the calling session resumes when the tool returns.
- **Result:** compact, evidence-backed JSON with `status`, `condition`, `observed`, `evidence`, and `elapsed_ms`; no transcript import.
- **Background sessions:** the tool is callable from any session, including background sessions. Observing a background session as a target is deferred.
- **Cross-repository polling:** deferred.

## Deferred Work

Split into follow-up cards, each linked from the spec:

- `FEAT-057` Implement the git ref/branch polling agent tool (first slice).
- `FEAT-058` Add PR and CI check observables to polling.
- `FEAT-059` Add board lane and background-session observables to polling.
- `FEAT-060` Add a runtime poll registry and TUI surface for outstanding polls.

## Related Items

- `PHASE-003` (phase parent)
- `FEAT-002` Keep sessions running after TUI exit
- `FEAT-003` Add compact fork context for session branching
- `FEAT-004` Add in-session send-to-fork commands
- `START-005` Define V1 runtime loop
- `START-016` Define structured task state for V1
- `START-025` Add retrieval-provider boundary for task context assembly

## Notes

- The design prefers observable external state over transcript-derived guesses.
- Implementation work now lives in the `FEAT-057` through `FEAT-060` follow-up cards rather than in this planning item.
