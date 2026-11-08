---
id: "RESEARCH-009"
title: "Plan a regularized recursive self-improvement loop for the ort harness and development workflow"
priority: "P2"
type: "research"
area: "RESEARCH"
spec: "wiki/agent-evaluation-strategy.md"
status: "todo"
created: "2026-10-15"
---

# Plan a regularized recursive self-improvement loop for the ort harness and development workflow

## Summary

Research and framework-plan whether `ort` should adopt a **regularized recursive
self-improvement (RSI) loop**: an automated cycle where agents evaluate, modify,
test, and iteratively improve (a) the `ort` **agent harness** and (b) the
**AI-assisted development workflow** built around it (boards, handoffs, QA gates,
agent roles, skills, verification), while keeping the backbone model frozen.

The expected outcome is a **practical architecture**, not a running optimizer: a
loop design, its admission/validation gates, and the guardrails that keep it from
overfitting fixed benchmarks, regressing prior gains, or optimizing without
control. Any concrete build is split into follow-up cards.

This is a framework-planning card in the evidence-gated research spirit of
`RESEARCH-001`/`RESEARCH-005`; a **null result** ("not worth automating the loop
yet") is a valid outcome.

## Why this exists

- `ort` already is an agent harness — prompts, control flow, tooling, memory, and
  context management around a frozen backbone. Harness quality, not only model
  quality, drives capability. Today the harness and the workflow around it are
  improved **manually** through boards, handoffs, QA, and fresh-context review.
- The manual loop is the right starting point (`invariants/verification.md`: the
  implementer is not the final authority; review runs in fresh context). The open
  question is whether a **partially automated** version of that loop improves the
  harness/workflow faster without losing the guards that make it trustworthy.
- The literature is maturing fast on this exact problem (agent-harness RSI), and
  it is unanimous that naive recursion **overfits and can regress**. The value of
  this card is to map those results onto `ort`'s existing evaluation and
  verification primitives and decide a safe integration shape.
- The evaluation substrate this depends on already exists as planned work:
  `START-006` (agent evaluation strategy) and its downstream `START-021`
  task pack, `START-022` review rubric, `START-023` structured results, and
  `START-024` rerun workflow — currently in `hold`. This card designs the loop
  that would sit on top of them; it does not unblock them.

## Research areas

1. **Recursive improvement** — automated feedback loops where agents evaluate,
   modify, test, and iteratively improve the harness/process (proposal →
   selection → persistence of accepted revisions across runs).
2. **Overfitting risks** — how repeated optimization against fixed benchmarks or
   evaluation criteria yields large in-distribution gains that shrink or vanish
   out-of-distribution, plus the specific failure modes others have named.
3. **Regularization** — techniques that constrain proposal and selection so the
   loop favors reusable mechanisms over benchmark-specific edits or noise.
4. **Agent-harness optimization** — the RRSI line and adjacent work: what is
   optimizable (prompts, control flow, tooling, memory, context management),
   what the admission rules are, and what is measurable.
5. **Practical integration** — how to wire an automated loop into the existing
   workflow (boards/handoffs/QA/agent roles) using automated testing, independent
   validation, performance metrics, and controlled iteration.

## Verified starting references (recorded 2026-10-15; re-verify before reuse)

- **RRSI: Regularized Recursive Self-Improvement of Agent Harnesses** —
  arXiv:2609.24972v3. The anchor paper. Overfitting is the central enemy: RSI
  "may overfit by memorizing the training tasks, showing large in-distribution
  gains that shrink or even vanish on out-of-distribution benchmarks."
  Regularizations: a proposer with a **temporally annealed budget** (limits how
  many edits a candidate can bundle) that **encourages unexplored trajectories**
  from evolution history; a selector with a **critic** (screens
  benchmark-specific proposals) and a **pruner** (removes too-small, too-expensive,
  or no-longer-useful changes). Reported: up to +6.0 on the evolved split and up
  to +4.3 across five OOD benchmarks, at **30% fewer policy tokens** than
  unregularized evolution. Code: `github.com/google-research/rrsi`.
- **Self-Evolving Agent Harness** — arXiv:2610.18813v1. Theory companion:
  formalizes RSI with a **continual PAC-learning** view, giving an expected
  generalization bound, a **retention bound conditional on historical regression
  budgets**, and a **high-probability admission rule using fresh, independent
  validation data**. Emphasizes continual validation as distributions/evaluators
  shift and independent safeguards for learning vs runtime privileges.
- **SAHOO: Safeguarded Alignment for High-Order Optimization Objectives in
  Recursive Self-Improvement** — arXiv:2603.06333v1. Drift control: a **Goal
  Drift Index**, **constraint-preservation checks** (syntactic correctness,
  non-hallucination), and **regression-risk quantification** to flag cycles that
  undo prior gains. Maps a capability–alignment frontier: cheap early cycles,
  rising alignment cost later.
- **VideoHarness-SRSI** — arXiv:2610.19424v1. Names three recurring failure
  modes — **validation overfitting, search-budget dilution, the early-experience
  trap** — all rooted in the same imperfect validation evidence driving both
  revision and search. Mitigations: **content-free feedback**, **instrumented
  warm-up**, and mechanisms developed in **isolated probe-race branches** before
  composition. Reports mean held-out +6.2% and a 37.9% smaller validation–test gap
  over its unregularized predecessor.
- **Self-Harness: Harnesses That Improve Themselves** — arXiv:2606.09498v3.
  Concrete three-stage loop: **Weakness Mining → Harness Proposal → Proposal
  Validation** (accept candidate edits only after regression testing); every
  model–benchmark combination improved held-in and held-out pass rates.
- **Recursive Harness Self-Improvement (RHI)** — arXiv:2607.15524v1. Harness as a
  prompt-level spec of the agent loop, refined by **pairwise feedback over its own
  revision history**; gains attributed to better context management, with up to
  60% lower inference cost.
- **HELIX: Model-Harness Co-evolution for RSI** — arXiv:2608.13951v1. A
  **source-traceable substrate** decomposing the agent system into typed ports,
  atoms, recipes, and runtime policies so every intervention is explicit and
  auditable, retaining trajectories, test outcomes, and provenance.
- **Recursive Agent Harnesses (RAH)** — arXiv:2606.13643v1. Harness recursion
  (parent harness spawns subagent harnesses) as a capability axis; relevant to
  `ort`'s own task/subagent surfaces.
- **ActiveSaddler** — arXiv:2610.00906v1. The **curriculum itself** should adapt
  as the harness evolves; models the evolving curriculum as a non-stationary
  bandit over reusable failure patterns.
- **HarnessRisk** — arXiv:2608.17597v1. Safety framing across harness lifecycle
  phases (configuration, capability extension, runtime, state persistence, action
  control, incident recovery); "Harness Configuration" is the most vulnerable
  phase. Relevant to permission/allow surfaces.

## Settled model (context, not in question)

- The **backbone model stays frozen**; RSI here modifies the harness/workflow, not
  weights (`HELIX`/`RHI`/`RRSI` all assume fixed-model harness evolution).
- The implementer is not the final authority and review runs in fresh context
  (`invariants/verification.md`); any automated loop must preserve this, not
  replace it.
- A task is not complete solely because code (or an accepted edit) was produced.
- Board/handoff/QA artifacts remain the system of record; board items, not a
  hidden optimizer, carry dispositions.
- Local artifacts are canonical; external evidence is advisory only
  (`invariants/research-department.md`).

## Scope

### Research questions

- **R1 — Target surface.** Which parts of the `ort` harness and dev workflow are
  safely auto-optimizable (prompts, skill text, agent-loop control parameters,
  tool schemas, context/memory policy, review checklists, QA task selection), and
  which must stay human-owned (invariants, permission/allow policy, dispositions,
  merge authority)?
- **R2 — Overfitting.** Given `ort` optimizes against task packs and QA
  fixtures, what concrete overfitting failure modes should be expected, and how
  are they measured (in-distribution vs held-out vs OOD task split; validation–test
  gap)?
- **R3 — Regularization.** Which of the surveyed techniques map cleanly onto
  `ort`'s primitives: edit-budget annealing, unexplored-trajectory exploration,
  critic/pruner, retention/regression budgets, fresh-independent-validation
  admission, constraint preservation, content-free feedback, isolated probe
  branches?
- **R4 — Admission and provenance.** What is the evidence-gated admission rule,
  and how are accepted revisions versioned, auditable, and rollback-able through
  the normal development workflow?
- **R5 — Cost and control.** What token/compute budget, iteration cadence, and
  human gates keep the loop bounded and interruptible?
- **R6 — Value.** Does a regularized loop beat the current manual improve loop on
  a real `ort` task (better harness behavior or workflow quality per unit of
  human effort), or is `ort` too heterogeneous/low-volume for RSI to pay off yet?

### Candidate architecture to evaluate (framework-planning deliverable)

A loop that reuses `ort`'s existing workflow stages, with explicit regularizers at
each step:

1. **Observe** — collect execution traces, QA evidence, board/handoff outcomes,
   and regression history (leverages `START-023` structured results).
2. **Diagnose / Mine** — weakness mining into **reusable failure patterns**
   (Self-Harness; ActiveSaddler uses these as curriculum arms).
3. **Propose** — generate candidate harness/workflow edits under a **temporally
   annealed edit budget**, encouraging **unexplored trajectories** (RRSI).
4. **Critic / Prune** — screen benchmark-specific or overfit proposals; prune
   edits that are too small, too expensive, or no longer useful (RRSI).
5. **Validate** — run **fresh, independent** validation task packs plus a
   regression suite; enforce a **retention bound** against a regression budget
   (Self-Evolving Agent Harness); use **content-free feedback** and, where useful,
   **isolated probe-race branches** (VideoHarness-SRSI).
6. **Admit** — high-probability admission rule on fresh data; constraint
   preservation and regression-risk checks (SAHOO); **human/fresh-context review**
   for anything touching invariants, permissions, or dispositions.
7. **Deploy & Roll back** — accepted revisions land through the normal board/PR
   workflow with versioned rollout and rollback.
8. **Learn** — record provenance and outcomes so the curriculum co-evolves with
   the harness (ActiveSaddler; HELIX-style source traceability).

Headline criterion: does a **regularized** loop change `ort` practice for the
better on a real task without losing the guards, or is the manual loop already
sufficient?

### Deliverable

- A framework-plan companion document
  (`docs/research/RESEARCH-009-recursive-self-improvement-framework.md`) covering
  the research map, the candidate loop, the regularization techniques mapped to
  `ort` primitives, the guardrail set, and the recommended integration shape.
- An explicit decision on whether/when to build the loop, surfaced for approval,
  plus any invariant implications (`wiki/agent-evaluation-strategy.md`,
  `invariants/verification.md`).
- Follow-up cards split out for any build, task-pack, or tooling work.

## Non-goals

- Building or running an automated optimizer in this card (research + framework
  plan only).
- Updating model weights, fine-tuning, or any model-side training (harness/workflow
  only; backbone stays frozen).
- Automating anything that is human-owned by invariant: dispositions, merge
  authority, permission/allow policy, or changing an invariant without approval.
- Unblocking or redefining `START-021`–`START-024`; this card consumes them as the
  evaluation substrate and records dependencies only.
- Making any external harness/model/provider a runtime dependency.
- Committing secrets or private workspace content.

## Done when

- Each research area (recursive improvement, overfitting risks, regularization,
  harness optimization, practical integration) is summarized with at least one
  **verified** primary source recorded with exact ID/revision.
- The overfitting failure modes (`validation overfitting`, `search-budget
  dilution`, `early-experience trap`, benchmark memorization) are named and each
  is mapped to a concrete `ort` measurement and guard.
- A candidate loop architecture exists with its **proposal, admission,
  validation, and rollback** rules, using independent/held-out validation and a
  regression budget.
- The target-level boundary (what is auto-optimizable vs human-owned) is stated.
- A clear **build / don't-build (yet)** recommendation is recorded and surfaced
  for approval; concrete work is split into follow-up cards.

## Recommended verification

- Read each cited paper from its own source, not from memory; record the exact
  arXiv ID/version and the specific claim being reused.
- Confirm the candidate loop does not weaken `invariants/verification.md`
  (implementer-is-not-authority, fresh-context review) or move human-owned
  decisions.
- Confirm validation data is genuinely held out from the evolve set and that a
  measurable validation–test gap is part of the design.
- Confirm no invariant/disposition/authority change is made by this card, and no
  private content or secret is committed.
- Cross-check the plan against `RESEARCH-005` (internal first-principles research
  step) so the loop does not skip derivation for narrative optimization.

## Related Items

- `START-006` Define agent evaluation strategy (done) — the evaluation framing
- `START-021` Define V1 evaluation task pack (hold) — evolve/validation/OOD splits
- `START-022` Define review quality rubric and fixtures (hold) — content-free review signal
- `START-023` Persist structured evaluation results (hold) — the observe step
- `START-024` Add lightweight evaluation rerun workflow (hold) — controlled iteration
- `START-025` Add retrieval-provider boundary for task context assembly (done)
- `EPIC-001` DeepSeek default-model reliability hardening (refinement) — reactive-fix history motivating a systematic loop
- `QA-002` Deterministic DeepSeek stream/loop fixture harness (qa) — regression fixture
- `RESEARCH-005` Vet the internal research process (first-principles derivation; done)
- `RESEARCH-007` Breadth survey: local qwen effectiveness (todo) — harness-vs-model attribution
- `RESEARCH-008` Evaluate integrating DeepSeek Harness (dsh) into ort (todo) — harness candidate
- `invariants/verification.md`, `invariants/research-department.md`, `wiki/agent-evaluation-strategy.md`, `wiki/future-versions.md`

## Notes

- Model is not the variable; this is a harness/workflow optimization question.
- The surveyed field converges on the same lesson: **unregularized recursion
  overfits**. Regularization and independent validation are load-bearing, not
  optional polish.
- Keep the human in the loop; a null result ("manual loop is sufficient for now")
  closes this card the same as a positive build recommendation.
