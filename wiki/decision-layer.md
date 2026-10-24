# System-1 Decision Layer (Jev/Laya)

## Purpose

Define how `opencode-modded-rust` uses a System-1 decision engine (Jev, Laya, or a
future equivalent) without letting the engine become the harness.

This document is the implementation plan. `wiki/harness-substrate.md` defines the
substrate; `invariants/decision-layer.md` defines the normative rules;
`wiki/decision-log-egress.md` defines the data policy.

## Scope note

"Jev" here means the TypeSafe System One decision API
(`POST https://api.typesafe.ai/v1/systemone`, model `jev-latest` or a pinned
version). It is **not** the separate `jev-ultrafast` browser agent, which remains
not planned for integration. Jev is not a CLI; the engine client uses HTTP, not a
PATH executable.

## Settled decisions

- **Jev is the preferred hosted decision engine.** It is locked in for the first
  implementation. The economics are already confirmed: the `transcript-pulls`
  corpus ran roughly 84,188 decisions for about $1.73, with strong calibration on
  the transcript-classification benchmark.
- **Laya is the local and fine-tuning path.** Zero-shot Laya is not competitive on
  the transcript benchmark. Its value is a fine-tuned checkpoint aligned to Jev,
  which can remove per-request cost and data egress later.
- **The engine is replaceable.** Jev, Laya, deterministic rules, and future engines
  implement the same decision boundary.
- **Classifiers are advisory.** The runtime disposes. Permission, disposition,
  merge, and invariant authority never move to a classifier.
- **Shadow first.** A decision class is logged and evaluated before it is allowed
  to act.
- **Local-first for sensitive classes.** State that contains secrets, private
  research, or personal data does not leave the machine for a hosted engine.

## Decision versus generation

A decision is a choice among known options with a confidence and an audit trail. A
generation is produced text, code, or a plan.

Jev/Laya answer typed questions; they do not generate prose. That is the point:
the cheap, calibrated, auditable layer selects and gates, while the model generates
only where generation is required.

Examples:

- **Decision:** is this failure a transport error, a model error, or a harness
  error?
- **Generation:** write the repair patch.
- **Decision:** is this review comment blocking, non-blocking, or noise?
- **Generation:** write the review comment.
- **Decision:** should this turn compact now, and which spans must survive?
- **Generation:** write the compacted summary.

## Current decision inventory

The first inventory pass should confirm and extend this list. The current code
already contains decision points that are either deterministic or generative:

| Decision point | Current implementation | System-1 candidate |
| --- | --- | --- |
| Permission allow/ask/deny | `crates/opencode-permission/src/engine.rs`, `ruleset.rs` | Advisory risk classification only; rules remain authoritative |
| Agent/tool availability | `crates/opencode-agent/src/agent.rs`, `executor.rs` | Intent/agent routing; tool risk |
| Title generation | builtin `title` agent | Not a decision; generation stays with the model |
| Session summary | builtin `summary` agent | What to summarize and at what granularity |
| Compaction | builtin `compaction` agent | Trigger and retention policy |
| Model/provider selection | launcher profiles and config | Cost/tier routing and escalation |
| Tool-call normalization | provider parsers | Repair/classification gate |
| Loop continuation | stream/timeout/budget logic | Continue/stop/escalate classification |
| Retrieval sufficiency | context assembly and ScopeMux | Sufficient/insufficient and next source |
| Failure classification | logs and manual triage | Transport vs model vs harness |
| Review severity | review agent and operator | Blocking/non-blocking/noise |
| Board/task classification | boards workflow | Lane/next-action suggestion |
| Memory write | session persistence | What to persist or recall |
| Safety screening | permissions and provider policy | Jailbreak, injection, sensitive data |

The inventory card must record the exact code location, current cost, current
latency, current accuracy or error rate, and the authority tier of each decision.

## Architecture

```text
runtime event
    |
    v
DecisionRequest { class, state, options, confidence_floor, budget, sensitivity }
    |
    v
decision boundary
    |-- Jev (hosted)
    |-- Laya (local)
    |-- deterministic rules
    |-- stub (tests)
    |
    v
DecisionResult { choice, confidence, engine, cost, latency, rationale }
    |
    v
runtime enforces under its own authority
    |
    v
append-only decision log
```

Components:

1. **Decision boundary** — typed request/result, timeouts, confidence, cost,
   sensitivity, engine metadata.
2. **Decision registry** — data-driven list of classes: owner, input schema, output
   schema, authority tier, threshold, fallback, budget, evaluation card.
3. **Engine client** — Jev and Laya adapters with retries, `402` handling, local
   fallback, and redaction.
4. **Shadow mode** — run System 1 alongside the current path; log agreement; no
   authority.
5. **Decision log** — append-only, replayable, provenance-bearing; see
   `wiki/decision-log-egress.md`.
6. **Evaluation harness** — per-class accuracy, precision/recall, calibration, cost,
   latency, escalation rate, and held-out/out-of-distribution behavior.
7. **Capability profiles** — which classes are delegated for which model.

## Workspace-intelligence inputs

The Workspace Intelligence Protocol (`wiki/workspace-intelligence.md`) is a decision
**input**, not a decision maker. It supplies structured evidence; the runtime and the
decision layer decide.

| Decision class | Map input |
| --- | --- |
| Retrieval sufficiency | candidate coverage, anchors, confidence, token estimate |
| Compaction trigger | delta size, token estimates, spans that must survive |
| Review triage | delta, observability, duplicate/refactor signals |
| Model/tier routing | task complexity signal (advisory only) |

The direction is one-way: **map -> decision -> runtime**. The map never advances a task
stage or widens its own authority. The runtime still owns inclusion thresholds, token
budgets, and stage. Tracked by `DECISION-016`.

## Decision classes and waves

### Wave 1 — low risk, stable boundaries

These survive the V1 task-stage rewrite and can ship first:

- **Failure classification** — transport vs model vs harness vs environment.
  Feeds retry, diagnostics, and escalation.
- **Review triage** — blocking, non-blocking, noise, or approve. Reduces review
  spam and operator load.
- **Compaction trigger and retention** — whether to compact now and which spans
  must survive. Reduces token cost.
- **Retrieval sufficiency** — whether the current context is enough or which source
  to read next. Reduces redundant reads.

### Wave 2 — reliability and cost

- **Tool-call normalization and repair** — classify malformed or leaked tool calls
  before they waste a turn. This is a local-model reliability lever.
- **Model/tier routing and escalation** — choose cheap vs strong model; escalate on
  low confidence or task complexity.
- **Board/task classification** — suggest lane, type, or next action; never mutate
  a disposition.

### Wave 3 — advisory, higher stakes

- **Permission and risk classification** — advisory only. Deterministic rules and
  human approval remain authoritative.
- **Safety screening** — jailbreak, prompt injection, sensitive data. Prefer local
  Laya for sensitive state.
- **Memory write policy** — what to persist, recall, or forget.

## Authority tiers

Every decision class declares one:

- **advisory** — logs a recommendation; the runtime ignores it unless a human or
  deterministic rule acts.
- **gated** — may block an action pending a deterministic check or human approval.
- **delegated** — may select among safe, reversible options within a declared
  budget.
- **forbidden** — cannot be delegated (permissions, dispositions, merges,
  invariants, secret access).

No class starts above **advisory**. Promotion requires evaluation evidence and an
explicit owner decision.

## Evaluation

Each class needs an evaluation pack before it can act:

- hand-labelled or human-reviewed ground truth,
- per-class accuracy and precision/recall,
- calibration (ECE or reliability curve),
- cost per decision,
- p50/p95 latency,
- escalation rate,
- held-out and out-of-distribution splits,
- regression against the current behavior.

The `transcript-pulls` benchmark is the shape to reuse, not the final dataset. Its
result is evidence that a hosted decision engine can be cheap and well calibrated;
it is not evidence that Jev is accurate on permissions or tool repair.

## Cost and billing

- Per-session and per-task decision budgets.
- Credit metering visible in the decision log.
- A `402`, quota, or insufficient-credit response is a **stop-and-resume** gate,
  not a crash and not a permanent failure.
- Resume skips completed decisions and continues from the last recorded position.
- Local Laya is the cost-free fallback when a budget is exhausted.

## Rollout

1. **Inventory and baseline** — measure the current path.
2. **Boundary and registry** — add the interface with a deterministic stub.
3. **Shadow** — run Jev/Laya beside the current path; no authority.
4. **Evaluate** — compare accuracy, calibration, cost, latency, escalation.
5. **Promote a wave** — enable one class at a time behind policy.
6. **Observe** — log outcomes and human corrections.
7. **Expand or demote** — add classes, or remove ones the model has absorbed.

## Fine-tuning and engine swap

The decision log is the training set for Laya alignment:

1. collect `(state, question, Jev answer, confidence, outcome, correction)`;
2. split train/validation/test by task and time, not randomly, to avoid leakage;
3. fine-tune Laya on the train split;
4. compare against Jev on held-out and out-of-distribution data;
5. swap per class when the accuracy gap is acceptable and the cost saving is real;
6. keep Jev as the preferred engine for classes where Laya is not yet competitive.

The swap is a policy change. It must pass the swap test in
`wiki/harness-substrate.md`.

## Post-Jev phases

The phases after the first Jev implementation are in
`wiki/post-jev-roadmap.md`: shadow, promotion, ownership, expansion/absorption,
and the next frontier. The short version:

- Jev proves the decision layer.
- The log enables Laya.
- Laya makes the layer cheaper and local.
- Capability profiles let models absorb classes without a rewrite.
- The same substrate supports events, coordination, and durable cloud runs.

## Non-goals

- Replacing generation with classification.
- Giving classifiers authority over permissions or dispositions.
- Making Jev a hard runtime dependency.
- Rebuilding the `transcript-pulls` annotation pipeline here.
- Committing secrets, private transcripts, or personal data to a decision log.
- Optimizing only for the classes that are easy to label.

## Board

- `DECISION-001` — System-1 decision layer (Jev-Laya) epic
- `DECISION-002` — inventory and baseline harness decision points
- `DECISION-003` — engine-neutral decision boundary and registry
- `DECISION-004` — Jev-Laya engine client and degradation ladder
- `DECISION-005` — shadow mode, decision log, and replay
- `DECISION-006` — per-class decision evaluation harness
- `DECISION-007` — Wave 1: failure classification
- `DECISION-008` — Wave 1: review triage
- `DECISION-009` — Wave 1: compaction trigger and retrieval sufficiency
- `DECISION-010` — Wave 2: tool-call normalization and repair
- `DECISION-011` — Wave 2: model-tier routing and board-task classification
- `DECISION-012` — Wave 3: permission and risk classification (advisory)
- `DECISION-013` — Wave 3: safety screening and memory write policy
- `DECISION-014` — Laya alignment and engine-swap playbook
- `DECISION-015` — post-Jev absorption audit and demotion plan
- `DECISION-016` — wire workspace-intelligence signals into retrieval, compaction, and
  review-triage decisions

## Related

- `wiki/harness-substrate.md`
- `wiki/decision-log-egress.md`
- `wiki/post-jev-roadmap.md`
- `wiki/workspace-intelligence.md`
- `invariants/decision-layer.md`
- `invariants/future-proofing.md`
- `invariants/workspace-intelligence.md`
- `boards/refinement/System-1 decision layer (Jev-Laya) epic.md`
