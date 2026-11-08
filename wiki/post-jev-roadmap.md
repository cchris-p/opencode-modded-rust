# Post-Jev Roadmap

## Purpose

Describe what happens after the first Jev implementation lands, so the decision
layer becomes a durable substrate rather than a one-off integration.

This is a roadmap, not a commitment to dates. It defines phases, exit criteria,
metrics, risks, and the conditions under which a decision is reversed.

## Assumptions

- Jev is the preferred hosted System-1 engine for the first implementation.
- The economics are confirmed; the remaining question is architecture and
  operation, not whether Jev is worth trying.
- Laya is the local and fine-tuning path.
- Classifiers are advisory; the runtime disposes.
- The decision log is the owned data asset.
- The base harness must not be rewritten for each new engine, model, or decision
  class.

## Phase 0 — Substrate

Deliver the substrate before or alongside the first integration:

- stable-core and volatile-policy boundaries,
- engine-neutral decision boundary and registry,
- capability profiles and shim registry,
- policy versioning and rollback,
- decision-log schema and egress policy,
- per-class evaluation harness,
- the eight future-proofing tests.

Exit criteria:

- a deterministic stub can answer every registered class,
- adding a class is a registry change,
- removing a class does not require a core change,
- the log records enough to replay and audit.

If Phase 0 is skipped, Jev will be woven into the interim chat loop and will have
to be integrated again when the V1 task-stage runtime lands.

## Phase 1 — Shadow

Run Jev beside the current path:

- no authority,
- log agreement and disagreement,
- record cost, latency, confidence, and escalation,
- compare against the current heuristic or model behavior.

Exit criteria:

- a stable evaluation pack exists for each Wave 1 class,
- disagreement is understood, not just counted,
- cost per decision and p95 latency are within budget,
- the decision log is producing usable records.

## Phase 2 — Promotion

Promote one class at a time, lowest risk first:

- failure classification,
- review triage,
- compaction trigger,
- retrieval sufficiency.

Each promotion is a policy change with:

- a capability profile entry,
- an authority tier,
- a confidence threshold,
- a fallback,
- a rollback plan,
- a budget.

Exit criteria:

- measured cost or latency improvement on a real task pack,
- no regression in task success or review quality,
- escalation rate is understood,
- a human can explain any decision from the log.

## Phase 3 — Ownership

Turn the log into an owned asset:

- freeze a training export with task/time splits,
- fine-tune Laya against Jev,
- evaluate on held-out and out-of-distribution data,
- swap per class where the gap is acceptable,
- keep Jev for classes where Laya is not yet competitive.

Exit criteria:

- the swap test passes for at least one class,
- the local path completes a real task with the hosted engine unavailable,
- the cost saving is measured, not assumed,
- data egress is reduced for at least one sensitive class.

## Context/data track — Workspace Intelligence

In parallel with the decision phases, the context/data pillar matures through the
Workspace Intelligence Protocol (`wiki/workspace-intelligence.md`):

- ScopeMux provides a persistent map of current and target state;
- small models and subagents get scoped, token-budgeted slices;
- retrieval provenance and map outcomes join the decision log;
- activation moves from the Qwen-only experiment to capability profiles;
- System-1 routing may choose generic vs WIP, the rung, budget, and sufficiency within
  those profile bounds (`DECISION-017`);
- the keep/expand/retire decision is made from measured token and task outcomes (B+D:
  tiered, time-boxed).

This track is not a prerequisite for Jev, but the two reinforce each other: the map
supplies evidence for retrieval/compaction/review decisions, and the decision layer
decides when and how much map context to use. It remains optional and must pass the eight
future-proofing tests.

## Phase 4 — Expansion and absorption

Expand the decision layer and let the flywheel work:

- add Wave 2 classes (tool repair, routing, task classification),
- add Wave 3 advisory classes (permission risk, safety, memory),
- run the absorption review on each model release,
- disable or delete classes the model has absorbed,
- update capability profiles.

Exit criteria:

- the decision registry is the source of truth for what runs,
- zero-firing or obsolete classes are retired,
- the core loop has not grown to accommodate each class,
- decision density and cost per successful task are tracked over time.

## Phase 5 — Next frontier

Use the same substrate for the next non-absorbable layer:

- event subscriptions and wake conditions (git, PR, CI, board, session, time),
- coordinator/worker agent fleets,
- durable cloud or off-laptop runs,
- software-factory orchestration across the development lifecycle.

The substrate supplies:

- the event bus,
- the task graph,
- the budget and capability profiles,
- the decision boundary,
- the audit log.

The point is to add the next frontier without a rewrite. If a new frontier
requires a new core loop, the substrate failed a future-proofing test.

## Continuous loops

- **Absorption review** — on every model release and quarterly. Re-run the
  harness-vs-model comparison; update profiles; retire shims.
- **Shim retirement** — zero or falling firing counters open removal cards.
- **Policy review** — canary, replay, rollback for every policy change.
- **Decision-class review** — owner, evidence, authority tier, budget, and
  retirement criterion for every class.
- **Cost review** — tokens, credits, cache, latency, and spend caps.
- **Trust review** — permission blast radius, secret scope, audit coverage.

## Metrics by phase

| Phase | Primary metrics |
| --- | --- |
| 0 | eight tests pass; class add/remove is policy-only |
| 1 | agreement, cost/decision, p95 latency, log completeness |
| 2 | cost per successful task, task success, review quality, escalation rate |
| 3 | Laya-vs-Jev accuracy gap, cost saving, offline completion, egress reduction |
| 4 | decision density, absorption lag, shim inventory age, core churn |
| 5 | time-to-add a frontier primitive, core changes per primitive |

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Decision sprawl | Registry, owner, budget, and a maximum active class count |
| Authority creep | `invariants/decision-layer.md`; advisory-first; no classifier authority |
| Vendor lock-in | Engine-neutral boundary; Laya fallback; swap test |
| Data egress | Sensitivity tiers; local-first; `wiki/decision-log-egress.md` |
| Overfitting | Held-out and out-of-distribution splits; multiple engines |
| Double integration | Build the boundary against the target V1 loop, not the interim chat loop |
| Change-management debt | Small core; policy versioning; deletion-ready shims |
| Cost surprise | Per-task budgets; `402` stop-and-resume; local fallback |
| Classifier error | Confidence thresholds; escalation; deterministic rules remain authoritative |

## What would make us reverse a decision

- Jev accuracy on a class is not better than the deterministic or model baseline.
- The cost or latency of a class exceeds its measured benefit.
- A hosted engine cannot meet the egress policy for a class that matters.
- Laya fine-tuning closes the gap faster than expected.
- A model release absorbs a class, making the gate redundant.
- The substrate cannot pass the eight tests without a core rewrite.

Reversing a decision is a success of the process, not a failure. It means the
harness is tracking reality.

## Success

The post-Jev state is successful when:

- Jev is a policy provider, not a dependency,
- the log is a useful, governed dataset,
- Laya can take over classes where it is competitive,
- adding or removing a decision class does not touch the core,
- the harness degrades without the hosted engine,
- the next frontier can be added on the same substrate,
- efficiency and trust are measured, not asserted.

## Board

- `DECISION-001` — System-1 decision layer epic.
- `DECISION-015` — post-Jev absorption audit and demotion plan.
- `FUTURE-PROOFING-001` — guidance epic.
- `FUTURE-PROOFING-013` — Workspace Intelligence Protocol program.
- `FUTURE-PROOFING-017` — ScopeMux evaluation and keep/expand/retire criteria.
