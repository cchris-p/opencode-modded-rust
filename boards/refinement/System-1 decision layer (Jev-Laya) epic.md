---
id: "DECISION-001"
title: "System-1 decision layer (Jev-Laya) epic"
priority: "P1"
type: "epic"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "refinement"
created: "2026-10-24"
---

# System-1 decision layer (Jev-Laya) epic

## Summary

Build the System-1 decision layer for `ort`: an engine-neutral boundary that lets the
harness ask typed, calibrated questions (Jev hosted, Laya local, deterministic rules),
log the answers, evaluate them per class, and act on them under runtime authority.

Jev is the preferred hosted engine and is already locked in. The economics are
confirmed. The open work is architecture, evaluation, rollout, and the path from Jev to
a fine-tuned local Laya.

## Why this exists

The harness currently mixes decision and generation in the same expensive calls. That
is unnecessary. A typed decision is cheaper, faster, auditable, and calibrated; it can
run at boundaries where a generative call is unaffordable.

The decision layer is also an **absorption-resistant** advantage. A generic harness can
copy a prompt; it cannot copy a dense, owned decision log, a calibrated local classifier,
or per-model decision policy. This epic builds that layer without turning Jev into a
hard dependency.

## Settled decisions

- Jev is the preferred hosted engine for the first implementation.
- Laya is the local and fine-tuning path.
- The engine is replaceable behind one boundary.
- Classifiers propose; the runtime disposes.
- No class starts above `advisory`.
- Shadow mode before promotion.
- Local-first for sensitive classes.
- No Jev-specific imports in the stable core.

## Architecture

1. **Decision boundary** — typed request/result with confidence, cost, latency,
   sensitivity, engine, and policy version.
2. **Decision registry** — data-driven classes with owner, schemas, authority tier,
   threshold, fallback, budget, and evaluation card.
3. **Engine client** — Jev and Laya adapters with retries, `402` handling, redaction,
   and local fallback.
4. **Shadow mode** — run beside the current path; no authority.
5. **Decision log** — append-only, replayable, provenance-bearing; policy in
   `wiki/decision-log-egress.md`.
6. **Evaluation harness** — accuracy, precision/recall, calibration, cost, latency,
   escalation, held-out and OOD splits.
7. **Capability profiles** — which classes are delegated for which model.

## Decision inventory (initial)

| Decision | Current path | System-1 role |
| --- | --- | --- |
| Permission allow/ask/deny | deterministic ruleset + plugin hook | advisory risk only |
| Agent/tool availability | agent ruleset + executor | intent/routing suggestion |
| Title generation | LLM `title` agent | not a decision; generation stays |
| Session summary | LLM `summary` agent | what to summarize and at what level |
| Compaction | LLM `compaction` agent | trigger and retention policy |
| Model/provider selection | launcher/config | cost/tier routing and escalation |
| Tool-call normalization | provider parsers | repair/classification gate |
| Loop continuation | stream/timeout/budget | continue/stop/escalate |
| Retrieval sufficiency | context assembly/ScopeMux | sufficient/insufficient, next source |
| Failure classification | logs/manual triage | transport vs model vs harness |
| Review severity | review agent/operator | blocking/non-blocking/noise |
| Board/task classification | boards workflow | lane/next-action suggestion |
| Memory write | session persistence | persist/recall/forget policy |
| Safety screening | permissions/provider policy | injection/sensitive-data advisory |

## Wave plan

### Wave 1 — low risk, stable boundaries

- `DECISION-007` failure classification
- `DECISION-008` review triage
- `DECISION-009` compaction trigger and retrieval sufficiency

### Wave 2 — reliability and cost

- `DECISION-010` tool-call normalization and repair
- `DECISION-011` model-tier routing and board/task classification

### Wave 3 — advisory, higher stakes

- `DECISION-012` permission and risk classification
- `DECISION-013` safety screening and memory write policy

## Substrate and rollout cards

- `DECISION-002` Inventory and baseline harness decision points
- `DECISION-003` Add the engine-neutral decision boundary and registry
- `DECISION-004` Implement the Jev-Laya engine client and degradation ladder
- `DECISION-005` Add shadow mode the decision log and replay
- `DECISION-006` Build the per-class decision evaluation harness
- `DECISION-014` Align Laya to Jev from the decision log and add the engine-swap playbook
- `DECISION-015` Post-Jev absorption audit and demotion plan

## Authority

- Deterministic rules, invariants, and human gates remain authoritative.
- A classifier cannot grant permission, change a disposition, merge, or weaken an
  invariant.
- Every class declares an authority tier; promotion above `advisory` requires evidence
  and an owner decision.

## Metrics

- decision density (% of decisions served by System 1)
- accuracy and calibration per class
- cost per decision and cost per successful task
- p50/p95 decision latency
- escalation rate
- offline completion with the hosted engine unavailable
- replay fidelity
- Laya-vs-Jev gap and cost saving after fine-tuning

## Non-goals

- Replacing generation with classification.
- Giving classifiers authority over permissions or dispositions.
- Making Jev a hard runtime dependency.
- Rebuilding the `transcript-pulls` annotation pipeline here.
- Committing secrets or private content to a decision log.
- Optimizing only the easy-to-label classes.

## Done when

- the boundary, registry, client, log, and evaluation harness exist;
- Wave 1 classes run in shadow and at least one is promoted under policy;
- cost or latency improvement is measured on a real task pack;
- the log is producing a usable fine-tune set;
- the swap test and offline test pass for at least one class;
- a post-Jev absorption audit is scheduled.

## Related

- `FUTURE-PROOFING-001` guidance epic
- `wiki/decision-layer.md`
- `wiki/harness-substrate.md`
- `wiki/decision-log-egress.md`
- `wiki/post-jev-roadmap.md`
- `invariants/decision-layer.md`
- `TP-026` / `TP-029` in `transcript-pulls` (benchmark and economics reference)
- `STD-089` Jev/Laya umbrella (fine-tuning route)