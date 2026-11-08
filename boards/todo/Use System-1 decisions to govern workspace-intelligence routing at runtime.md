---
id: "DECISION-017"
title: "Use System-1 decisions to govern workspace-intelligence routing at runtime"
priority: "P2"
type: "feature"
area: "DECISION"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Use System-1 decisions to govern workspace-intelligence routing at runtime

## Summary

Evaluate a new decision class: use Jev/Laya to choose, per task or subagent, whether to
use the Workspace Intelligence Protocol (WIP) at all, which rung, how much budget, and
whether the returned slice is sufficient.

The capability profile sets the bounds. The decision layer proposes within them. The
runtime enforces. This is not authority; it is routing.

## Why this exists

Static profiles cannot know what a specific task needs:

- a one-line edit does not need a map,
- an unfamiliar multi-file change does,
- a subagent has a narrower budget than the parent,
- a stale map should fall back to generic,
- a cheap local model may benefit from a map while a frontier model does not need one.

A System-1 decision can make that call cheaply, with calibration and an audit trail,
instead of hardcoding heuristics or paying a model call to decide.

## Decision class

Working name: `workspace_intelligence.routing`.

Inputs:

- task objective and stage,
- model/provider and capability profile,
- subagent identity and budget,
- repository size/language coverage,
- seed files/symbols,
- map freshness/staleness,
- current context size and remaining budget,
- recent retrieval outcome.

Options:

- `generic` — skip WIP, use the generic provider,
- `wip:w0` — parser/index only,
- `wip:w1` — map/delta/observability,
- `wip:w2` — task projection/reconciliation,
- `wip:w3` — map-backed tools,
- `escalate` — switch to a stronger model or ask the user.

Optional second decision: `context_sufficiency` — is the returned slice enough, or should
the runtime request one more slice / a different source / fall back?

## Authority

- The capability profile is the hard bound: no classifier may enable a rung, provider, or
  budget outside it.
- The runtime enforces; the classifier only proposes.
- No classifier may advance a stage, verify, review, complete, or change a disposition.
- A low-confidence routing decision falls back to the profile default.
- The decision is logged with profile, engine, confidence, cost, and fallback reason.

## Guardrails

- No WIP egress for sensitive/secret content.
- No map access for a workspace/language outside the profile.
- No budget widening by the classifier.
- A stale map decision must allow generic fallback.
- The generic provider remains the default and fallback.
- Shadow first; promotion only after evaluation.

## Evaluation

- Compare three arms on the same task pack:
  1. generic only,
  2. static WIP profile,
  3. System-1 routing within the same profile bounds.
- Metrics:
  - tokens and cost per successful task,
  - task success and review quality,
  - context precision/recall,
  - p50/p95 latency,
  - fallback rate,
  - subagent token savings,
  - routing accuracy vs the arm that would have been best.
- Held-out and out-of-distribution tasks.
- A null result is valid: static profiles may be sufficient.

## Acceptance criteria

- The class is registered with an owner, schemas, authority tier, and budget.
- The profile is enforced as a hard bound.
- Shadow mode logs routing decisions without changing behavior.
- Evaluation compares the three arms.
- At least one arm improves cost or outcome without a regression, or a null result is
  recorded.
- No classifier can widen the profile or bypass fallback.
- The decision appears in the decision log and can be replayed.

## Non-goals

- Letting the classifier own WIP activation policy.
- Enabling WIP outside a profile.
- Replacing the profile system.
- A model call to decide on every turn.
- Routing around egress or permission rules.

## Related

- `DECISION-001`
- `DECISION-003` decision boundary
- `DECISION-006` evaluation harness
- `DECISION-016` map signals into decisions
- `FUTURE-PROOFING-013` WIP program
- `FUTURE-PROOFING-015` capability profiles
- `FUTURE-PROOFING-017` ScopeMux evaluation and B+D decision
- `SCOPE-007` subagent-scoped WIP
- `wiki/workspace-intelligence.md`
- `wiki/decision-layer.md`
- `invariants/workspace-intelligence.md`
- `invariants/decision-layer.md`
