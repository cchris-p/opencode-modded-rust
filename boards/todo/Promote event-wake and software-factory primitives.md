---
id: "FUTURE-PROOFING-010"
title: "Promote event-wake and software-factory primitives"
priority: "P2"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/advanced-coding-session-polling.md"
status: "todo"
created: "2026-10-24"
---

# Promote event-wake and software-factory primitives

## Summary

Consolidate `FEAT-057`..`FEAT-060` (git refs, PR/CI checks, board lanes, background
sessions, runtime poll registry) into one **event-subscription primitive**: an agent can
wait on, or wake for, a named event source with a bounded budget and a compact result.

This is the third prediction in the harness lecture: primitives as composable building
blocks, instead of per-integration hacks.

## Why this exists

The board already has the pieces, but they are separate P3 cards. A single substrate is
needed for:

- coordinator/worker agent patterns,
- the software-factory loop (bug -> triage -> fix -> prove -> deploy -> observe),
- durable session wake-ups,
- the next frontier after decisions.

## Scope

- Define an event-source contract: source, condition, timeout, budget, result shape.
- Implement sources: git ref, PR, CI check, board lane, background session, time.
- Add a durable subscription registry that survives a session restart.
- Add a runtime surface for outstanding subscriptions.
- Add a policy for waking a paused session only with explicit user control unless a
  capability profile permits otherwise.
- Return compact evidence, not full transcripts.
- Document the software-factory mapping as an example composition, not a new product.

## Acceptance criteria

- One contract covers all sources.
- A subscription survives a restart and can be cancelled.
- Waiting is bounded and returns met/timeout/error with evidence.
- No auto-wake without policy.
- The existing FEAT cards are either absorbed or explicitly linked.
- A synthetic software-factory flow can be expressed with the primitive.

## Non-goals

- Building the full software factory.
- Cross-organization event sources in the first slice.
- Replacing the TUI.
- Auto-merging or auto-deploying without human gates.

## Related

- `FUTURE-PROOFING-001`
- `FEAT-057` implement git-ref polling agent tool
- `FEAT-058` add PR and CI check observables to polling
- `FEAT-059` add board lane and background-session observables to polling
- `FEAT-060` add a runtime poll registry and TUI surface
- `wiki/advanced-coding-session-polling.md`
- `wiki/post-jev-roadmap.md` (next frontier)
