# Harness Substrate Future-Proofing

## Purpose

Define the substrate that keeps `opencode-modded-rust` useful as models, providers,
and harness fashions change. This document is the explanatory companion to
`invariants/future-proofing.md`; the invariant is normative, this document explains
the reasoning and the target architecture.

The question this document answers is not "what feature should we build next?" It
is:

> If an unknown future change arrives — a stronger model, a cheaper model, a new
> decision engine, a new orchestration primitive, a vendor outage — how cheaply can
> this harness adapt without a rewrite?

"Future-proof" here means **low adaptation cost**, not accurate prediction.

## Definitions

### Flywheel

A **flywheel** is a self-reinforcing loop in which each cycle's output becomes the
next cycle's input, so the system accelerates without proportional new effort.

The relevant flywheel for agent harnesses is a **data flywheel**:

> a harness makes harder tasks usable -> more users attempt those tasks -> more
> task and trajectory data reaches the model labs -> models learn the capability
> natively -> the harness's model-compensating scaffolding becomes redundant ->
> the harness must move to a new frontier to stay useful.

This is why "the harness is dead" headlines appear even though harness work keeps
growing. The flywheel does not delete the harness. It deletes the part of the
harness that duplicates a capability the model has absorbed.

### Absorption

**Absorption** is a model lab turning a harness capability into native model
behavior. Examples: a model learning to explore a codebase without a custom
exploration prompt, or to check its own work without a verification shim.

Absorption is not a failure of the harness. It is the harness succeeding at making
the task common enough for the model to learn.

### Stable core

The **stable core** is the small set of runtime contracts that should change
rarely:

- session and task state,
- task lifecycle and transitions,
- tool execution,
- permission authority,
- durable storage,
- provider abstraction,
- the event bus.

The core owns **authority** and **durability**. It must not contain model-specific
prompt logic, Jev-specific code, or vendor-specific decision rules.

### Volatile policy

The **volatile policy** layer is everything expected to change often:

- prompts and skills,
- shims and guardrails,
- decision classes and their thresholds,
- capability profiles,
- model routing,
- evaluation task packs,
- UI copy that describes model behavior.

Policy is versioned, canaried, and rolled back independently of the binary. Policy
churn is expected; it must not force core churn.

### Decision boundary

The **decision boundary** is the interface between the stable core and a decision
provider:

- the runtime emits a typed request (decision class, state, options, confidence
  requirement, budget, sensitivity),
- a policy provider returns a typed result (choice, confidence, engine, cost,
  rationale),
- the runtime enforces the result under its own authority.

The core never imports a specific engine. Jev, Laya, deterministic rules, and
future engines are interchangeable providers behind this boundary.

### Capability profile

A **capability profile** maps a model (or model family) to the policy that applies
to it:

- which shims are enabled,
- which decision classes are delegated to System 1,
- which prompts and tool descriptions are used,
- which cost and latency budgets apply.

A new model should require a new profile and a new evaluation run, not a loop
rewrite.

### Shim

A **shim** is any deterministic or prompt-level addition that compensates for a
model or runtime gap. Examples: a nudge to check work, a tool-call repair pass, a
model-specific formatting instruction.

Shims are allowed. Permanent, entangled, unmeasured shims are not.

### Deletion-ready

**Deletion-ready** means a shim or policy can be removed quickly and safely:

- isolated behind a flag or interface,
- has one owner,
- has a firing or usage metric,
- has no cross-cutting dependencies,
- has a documented retirement criterion and removal path.

Deletion-readiness is the property that makes absorption cheap. An entangled
feature is one whose removal requires rewriting the loop; it therefore survives
past its usefulness and accumulates change-management cost.

### Change management

**Change management** is the cost of changing something without breaking its
dependents: compatibility checks, migrations, testing, communication, rollback.

The lecture behind this document observes that exposing fewer harness internals
reduces change-management cost (Hyrum's Law: once users depend on an internal, it
is a de facto public contract). The lesson is not "never change the harness." It
is:

> keep the exposed and contractual surface small, put churn behind it, and make
> policy independently versioned and reversible.

## The two-layer rule

The substrate is split deliberately:

```text
+---------------------------------------------------------------+
| Volatile policy                                               |
| prompts | skills | shims | decision registry | profiles       |
| model routing | evaluation packs | thresholds | UI copy       |
+---------------------------------------------------------------+
| Decision boundary                                             |
| typed request -> provider -> typed result -> runtime enforces |
+---------------------------------------------------------------+
| Stable core                                                   |
| task state | lifecycle | tools | permissions | storage | bus   |
+---------------------------------------------------------------+
```

Rules:

1. The core may call the decision boundary; it may not import a provider.
2. Policy may depend on the core's contracts; the core may not depend on policy
   internals.
3. Adding a decision class, shim, model profile, or prompt variant is a policy
   change, not a core change.
4. Removing a policy item must not require a core change.

## Decision versus generation

A harness makes two kinds of calls:

- **decision**: choose among known options, with a confidence and an audit trail;
- **generation**: produce text, code, or a plan.

Vanilla harnesses often mix both in one expensive prompt. This substrate separates
them:

- System 1 (decision provider) answers typed questions cheaply and returns
  calibrated confidence;
- System 2 (the model) is invoked for generation and hard reasoning, ideally only
  when System 1 is uncertain or the task requires synthesis.

The split is not about replacing the model. It is about spending generative tokens
only where generation is actually required.

## Authority model

The authority rule is absolute:

> **Classifiers propose; the runtime disposes.**

A decision provider may advise on risk, intent, severity, sufficiency, or routing.
It may not:

- grant or deny a permission by itself,
- change a task disposition,
- merge or close a board item,
- weaken an invariant,
- bypass a deterministic rule or a human gate.

Deterministic rulesets, invariants, and human decisions remain authoritative. This
is what lets the decision layer expand without becoming a safety hole.

## Capability profiles

Each supported model gets a profile. A profile is data, not code:

```yaml
model: deepseek/deepseek-flash
profile: deepseek-flash-2026-10
shims:
  work-check-nudge: off
  tool-call-repair: on
decisions:
  failure-classification: system1
  review-triage: system1
  permission-risk: advisory
  compaction-trigger: model
budgets:
  max_decision_cost_usd_per_task: 0.02
  max_escalation_rate: 0.15
```

The profile is selected at session start and recorded in the decision log. When a
new model lands, the workflow is:

1. add a profile,
2. run the evaluation pack,
3. enable the shims and decisions the model still needs,
4. disable the ones it has absorbed,
5. record the result.

This is the mechanism for **riding** the flywheel instead of being erased by it.

## Shim registry

Every shim is registered with:

- name and owner,
- trigger condition,
- firing counter,
- cost per firing (tokens, latency, or compute),
- kill switch,
- retirement criterion,
- related board item.

The runtime logs firings. A periodic report lists shims with zero or falling
firings. A zero-firing shim opens a removal card automatically or through the
review cadence. A shim that cannot be removed without a core change is a design
defect, not a success.

## Policy versioning and change management

Policy versions are separate from the binary version:

- prompts, skills, shims, thresholds, and profiles have a version;
- a session records the policy version it used;
- canary a policy change on a subset of sessions or tasks;
- replay a decision log against the new policy before promotion;
- roll back without rebuilding the binary.

This is how the harness iterates quickly without turning every prompt tweak into a
release event.

## Data contract

The owned data asset is the combination of:

- decision log entries,
- task and session outcomes,
- review verdicts,
- board transitions,
- transcript/classification labels where applicable,
- human corrections.

Every record carries provenance (engine, policy version, model, prompt version) and
a sensitivity tier. The schema, retention, access, and egress rules are defined in
`wiki/decision-log-egress.md`.

The data contract exists for three reasons:

1. **Replay and audit** — explain and reproduce behavior.
2. **Evaluation** — build held-out and out-of-distribution sets.
3. **Ownership** — create a private dataset the labs do not train on.

The third reason is the compounding advantage. A generic harness can copy a
feature; it cannot copy your private outcomes.

## Workspace intelligence and retrieval

ScopeMux is the first implementation of the **Workspace Intelligence Protocol (WIP)**
defined in `wiki/workspace-intelligence.md`. It is not a separate moat; it is the
concrete implementation of the context/data pillar for code workspaces.

- It is optional and evidence-only. The harness must pass the eight tests without it.
- It is activated by a capability profile, not a model-ID branch. The Qwen-via-Ollama
  path is the initial experiment profile.
- System-1 routing may choose generic vs WIP, the rung, and the budget within the
  profile; the profile is a hard bound and the runtime disposes.
- Its capabilities form a ladder: parser/index, map, task projection/reconciliation,
  map-backed tools, decision integration. Each rung is independently enableable,
  measurable, and removable.
- Its highest-value use case is a small model or a spawned subagent that cannot afford a
  file dump and needs the smallest faithful slice.
- It composes with `opencode-lsp`; it does not replace it.
- Its plan nodes are projections of the authoritative task record.
- Its retrieval provenance and outcomes feed the decision log and the owned data flywheel.

The WIP contract is versioned and capability-negotiated, with an in-process transport
first and an out-of-process transport later. It is LSP-like in contract discipline, not in
scope.

## Degradation ladder

The harness must complete work without a single hosted dependency:

1. preferred decision engine (for example Jev),
2. local engine (for example Laya),
3. deterministic rules,
4. baseline model behavior.

The ladder is selected by policy and budget. A `402`, quota, timeout, or outage
degrades rather than crashes. A session that cannot afford a decision records that
fact and continues under the next rung.

## Efficiency and trust as first-class axes

The lecture's non-absorbable list is infrastructure, trust, and efficiency. The
substrate therefore treats them as evaluation axes, not tie-breakers:

- **efficiency**: tokens, cost, cache hit rate, p50/p95 turn latency, startup,
  peak RSS, binary and disk footprint;
- **trust**: permission blast radius, secret scope, spend caps, audit coverage,
  classifier authority;
- **capability**: task success, review quality, recovery, decision accuracy.

Raw model latency remains secondary. Harness-induced cost and trust are not.

## The eight future-proofing tests

A structural change passes future-proofing only if it satisfies these tests.

1. **Swap test** — replace the decision engine with another engine or a
   deterministic stub without touching the loop.
2. **Delete test** — remove any single shim, decision class, or policy item
   without breaking the runtime.
3. **Add test** — add a new decision class or shim through the registry/profile
   without a core change.
4. **Model-upgrade test** — adopt a new model by adding a profile and running the
   evaluation pack; no loop rewrite.
5. **Offline test** — complete a real task with no hosted decision engine.
6. **Replay test** — replay a decision log and reproduce the same decisions under
   the recorded policy version.
7. **Cost-cap test** — hit a spend or credit ceiling and degrade gracefully.
8. **Audit test** — explain every runtime decision from the log, including engine,
   confidence, policy version, and downstream outcome.

If a change fails a test, it is a core change and needs explicit justification, not
a quiet exception.

## Operating cadence

- **On every model release:** add or update a capability profile; run the
  evaluation pack; record absorbed shims.
- **Quarterly (or on a major model shift):** absorption review. Re-run the
  harness-vs-model comparison on current models; retire shims that no longer fire
  or no longer matter.
- **Per policy change:** canary, replay, rollback plan.
- **Per decision class:** owner, evaluation evidence, cost budget, authority tier.
- **Per release:** efficiency scoreboard and trust review.

## Metrics

- **Absorption lag** — time from model release to profile update, and time from a
  shim becoming obsolete to its removal.
- **Decision density** — share of harness decisions served by System 1.
- **Cost per successful task** — total tokens, credits, and compute.
- **Context tokens saved** — WIP vs generic retrieval, per task and per subagent.
- **Map freshness and fallback rate** — how often WIP is stale or unavailable.
- **Escalation rate** — share of System 1 decisions escalated to the model.
- **Replay fidelity** — share of logged decisions reproduced exactly.
- **Offline completion** — task success with the hosted engine unavailable.
- **Shim inventory age** — age and firing rate of every shim.
- **Exposed-surface change rate** — how often user-visible internals change.

## Relationship to Jev and Laya

Jev here is the TypeSafe System One decision API, not the separate
`jev-ultrafast` browser agent. It is the first major tenant of this substrate, not
the substrate itself. The substrate is designed so Jev can be:

- the preferred hosted decision engine,
- replaced by a fine-tuned local Laya,
- demoted to an optional engine for a subset of classes,
- removed entirely without rewriting the loop.

The implementation plan lives in `wiki/decision-layer.md`; the post-Jev phases live
in `wiki/post-jev-roadmap.md`.

## Non-goals

- Predicting the next model or vendor.
- Freezing the core so nothing changes.
- Chasing parity for parity's sake.
- Building a permanent benchmark leaderboard.
- Exposing harness internals as user contracts.
- Letting classifiers acquire authority.
- Optimizing a fixed benchmark without held-out validation.

## Board

- `FUTURE-PROOFING-001` — guidance epic (stays open; action items spawn from it).
- `FUTURE-PROOFING-013` — Workspace Intelligence Protocol program (epic).
- `DECISION-001` — System-1 decision layer (Jev/Laya) epic.
