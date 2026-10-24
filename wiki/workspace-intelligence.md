# Workspace Intelligence Protocol (WIP)

## Purpose

Define the long-term role of ScopeMux in `opencode-modded-rust` as a **Workspace
Intelligence Protocol (WIP)**: a stable, versioned, capability-negotiated way for the
harness to ask a workspace-intelligence provider for structured context, map state, and
task projections.

This document locks in the strategic choice and the terminology. It is the companion to
`invariants/workspace-intelligence.md`; the invariant is normative, this document explains
the reasoning and the target architecture.

## Decisions locked

- **Option B: Workspace-intelligence layer.** ScopeMux is not just a retrieval
  optimization and not a general LSP replacement. It is the owned context/data capability
  that builds a persistent map of a workspace's current and target state and serves
  token-budgeted slices to the harness.
- **Term: Workspace Intelligence Protocol (WIP).** The contract between the harness and a
  workspace-intelligence provider is called WIP. ScopeMux is the first implementation.
- **LSP analogy is useful for the contract, not the scope.** WIP should borrow LSP's
  discipline: client/server boundary, capability negotiation, versioning, lifecycle,
  health, multiple implementations. WIP is not an LSP and must not duplicate
  `opencode-lsp`.
- **ScopeMux is part of the project substrate.** It is not a side experiment or a
  separate product. `opencode-modded-rust` is the main utilizer and the reference
  implementation.
- **Small models and subagents are the primary beneficiary.** The map exists to make a
  small local model and a spawned subagent efficient: return the smallest faithful slice,
  not a file dump.
- **Qwen-only activation is an initial experiment, not the permanent policy.** Long-term
  activation is capability-profile-driven (see `FUTURE-PROOFING-015`).
- **Keep/expand/retire criteria are open.** Options are recorded below; the decision is
  pending.

## Terms

| Term | Meaning |
|---|---|
| `scopemux-core` | The native C engine: Tree-sitter parsing, project IR, InfoBlocks, search, prompt assembly. |
| ScopeMux provider | `opencode-scopemux`, the Rust FFI adapter behind the retrieval boundary. |
| ScopeMux map | The current/target-state model: locate, slice, project, reconcile. |
| ScopeMux program | The integration plus upstream `scopemux-core`/`scopemux-notes` work. |
| WIP | The versioned contract between the harness and a workspace-intelligence provider. In this document WIP always means Workspace Intelligence Protocol, never work-in-progress. |
| Workspace-intelligence provider | Any implementation of WIP. ScopeMux is the first. |

## What WIP is

WIP answers four questions for the harness:

| Operation | Question | Typical consumer |
|---|---|---|
| **Locate** | Which nodes matter for this task and stage? | retrieval, subagents, review |
| **Slice** | What is the cheapest faithful representation? | context assembly, compaction |
| **Project** | What is the delta between current and target state? | planning, implementation, review, repair |
| **Reconcile** | How do we keep the map current after changes? | watcher, incremental indexer |

It is layered:

| Layer | Content |
|---|---|
| L0 Parse | CST/AST, byte and line ranges |
| L1 Structural IR | symbols, resolved references, call graph, import/dependency graph |
| L2 InfoBlocks | addressable units, tiers, token estimates, dispositions |
| L3 Task/plan | projected target-state nodes, lifecycle, anchors, rationale |
| L4 Views | delta, review, debug, observability, refactor slices |

The map is more than an index. An index answers "what exists"; a map also represents
"what is intended" and "what changed," which is where the context savings come from.

## What WIP is not

- It is not an LSP. It does not implement hover, completion, diagnostics, formatting, or
  rename. `opencode-lsp` remains the language-semantics client.
- It is not a second task store. Plan nodes are projections of the authoritative task
  record; the task record wins.
- It is not a runtime authority. It cannot advance a stage, declare completion, verify,
  review, or change a disposition.
- It is not a general platform for every harness. It is the project substrate for this
  product, with a contract clean enough that another implementation or consumer is
  possible.
- It is not mandatory for V1 or for the eight future-proofing tests.

## The LSP analogy

Useful parts:

- client/server boundary,
- capability negotiation,
- versioned protocol,
- lifecycle and health,
- multiple implementations,
- testability,
- the harness does not embed provider internals.

Misleading parts:

| LSP | WIP |
|---|---|
| Per-language | Cross-language |
| Document-centric | Project- and task-centric |
| Editor features | Context economics and task projection |
| Mostly stateless request/response | Persistent map with reconciliation |
| Mature standard | Bespoke, still forming |
| Does not fit a token budget | Explicitly token-budgeted and stage-aware |

The right sentence is: **WIP is an LSP-like protocol for workspace intelligence, not an
LSP.** Compose with `opencode-lsp`; do not replace it.

## Why ScopeMux matters

The expensive resource for a coding agent is context, not compute.

- A small local model cannot hold a large repository in its window.
- A subagent has an even smaller effective budget and a narrower task.
- A frontier model can explore a codebase, but it still pays tokens for every file it
  reads, and it does not own the project's current/target state.

A map that returns the smallest faithful slice is a direct efficiency and reliability
lever. That is the primary case: **small models and subagents**.

## Placement in the future-proofing plan

ScopeMux is not a separate moat. It is the concrete implementation of the **context/data**
pillar, with a measured contribution to **efficiency**, and it is a **data source** for the
decision layer.

| Pillar | Role |
|---|---|
| Runtime / lifecycle | None. No stage, verification, review, or completion authority. |
| Trust | Indirect evidence only (observability, duplicate/risk signals). |
| Efficiency | Token reduction through structure-aware, budgeted slices. Measured. |
| Context / data | **Primary.** Persistent current/target map with provenance. |
| Decisions | **Input only.** Supplies evidence; the decision layer decides. |
| Orchestration | None. Events/coordination are a separate frontier. |

## Is it necessary?

| Goal | Necessary? |
|---|---|
| Pass the eight future-proofing tests | No. WIP must itself pass them. |
| Ship V1 | No. It remains optional. |
| Local-model daily driver on large/multi-file repos | **Likely yes for competitiveness.** |
| Subagent efficiency | **Yes, high value.** |
| Frontier-model-only thin harness | No. Models absorb codebase exploration. |
| Owned context/data moat | **Yes, if** the map includes task projection, provenance, and outcomes. |
| General platform | No. Out of scope. |

ScopeMux is a **strategic bet with an exit ramp**, not an invariant. It must earn its
place through measured token savings and task outcomes, and it must remain removable.

## Authority and fallback

- Plan nodes are a projection of the authoritative task record.
- The task record wins on any disagreement.
- Map output is evidence, not truth; heuristic links are confidence-labeled.
- The runtime owns inclusion thresholds, token budgets, role filtering, and stage.
- The generic repository-local provider is the default and fallback.
- Unsupported language, absent provider, provider failure, or budget exhaustion degrades
  to generic behavior; the task still completes.
- WIP never advances a stage, verifies, reviews, or declares completion.

## Capability ladder

Each rung is independently enableable and measurable. Higher rungs are not prerequisites
for lower ones.

| Rung | Capability | Fallback |
|---|---|---|
| W0 | Parser/index: symbols, references, search | generic grep/read |
| W1 | Map: delta, observability, refactor opportunities | generic retrieval |
| W2 | Task projection: plan nodes, reconciliation signals | task record only |
| W3 | Map-backed tools: locate/search over the node registry | bounded grep |
| W4 | Decision integration: retrieval sufficiency, compaction, review triage | model/rules |

The product should be able to ship W0 and W1 without W2/W3/W4, and disable any rung
independently.

## Protocol

WIP is versioned and capability-negotiated.

Minimum contract:

- protocol version,
- provider id and implementation version,
- supported capabilities (locate, slice, project, reconcile, observability, duplicates,
  search),
- supported languages,
- map schema version,
- health/status,
- request/response schemas for each capability,
- fallback and error semantics,
- provenance and confidence fields.

Transport:

- in-process Rust trait is the first implementation,
- an out-of-process transport (stdio/JSON-RPC or equivalent) is the target for
  testability, isolation, and reuse (`SCOPE-011`),
- both must implement the same contract; the harness uses the in-process path by default
  and falls back.

Compatibility:

- a provider advertises capabilities; the harness only asks for what is advertised,
- a missing capability degrades to the next rung, not an error,
- schema versions are checked; incompatible providers are rejected with a clear reason,
- the contract is tested with a stub provider and the generic provider.

## Capability profiles and activation

Activation is policy, not a model-ID branch in code.

- A capability profile declares whether a workspace-intelligence provider is enabled for a
  model, workspace, or task class.
- The Qwen-via-Ollama ScopeMux path is the **initial experiment profile**.
- New models and subagents get a profile; they do not get a code change.
- A profile records the enabled rungs, token budgets, and fallback.
- The profile is recorded in the session and the decision log.

This replaces the permanent `ollama + qwen*` rule in `invariants/providers.md` with a
profile-driven rule (`FUTURE-PROOFING-015`).

## System-1 runtime governance

A static profile is the bound; a System-1 decision can choose within it. The decision
layer may route, per task or subagent:

- `generic` — skip WIP,
- `wip:w0`..`wip:w3` — use a specific rung,
- `escalate` — switch model or ask,
- and a follow-up **context sufficiency** decision (enough / one more slice / different
  source / fall back).

Why: a one-line edit does not need a map; an unfamiliar multi-file change does; a subagent
has a different budget from its parent; a stale map should fall back to generic. A cheap,
calibrated decision can make that call without a model turn.

Guardrails:

- the capability profile is a hard bound; the classifier cannot enable a rung, provider,
  or budget outside it,
- low confidence falls back to the profile default,
- the runtime enforces; the classifier only proposes,
- no authority over stage, verification, review, completion, or disposition,
- local-first egress; no sensitive content to a hosted engine,
- shadow first; promotion only after evaluation.

This is an evaluation hypothesis, not a settled behavior. Tracked by `DECISION-017`.

## Subagents and small models

Subagents are the clearest use case:

- each subagent has a narrow objective and a small budget,
- it should receive a scoped slice, not the parent's context,
- the map can anchor the subagent to the relevant nodes and return a delta,
- the subagent's retrieval result and outcome become training/eval data.

Requirements:

- a subagent request carries its own objective, stage, seed nodes, and budget,
- the map slice is scoped to the subagent's task, not the parent session,
- the parent remains authoritative for the task and the subagent's result is evidence,
- per-subagent token and cost budgets are enforced,
- a subagent with no map support falls back to generic retrieval.

Tracked by `SCOPE-007`.

## Evaluation and exit criteria

Compare three arms on real tasks:

1. generic only,
2. static WIP profile,
3. System-1 routing within the profile bounds (`DECISION-017`).

Record:

- tokens saved per task and per subagent,
- context precision/recall,
- task success,
- review quality,
- p50/p95 latency,
- map freshness/staleness,
- fallback rate,
- cost per successful task,
- routing accuracy against the arm that would have been best.

Evaluation must use held-out and out-of-distribution tasks, not only the repo the map was
tuned on.

### Keep/expand/retire decision (2026-10-24)

**Selected: B + D — tiered, time-boxed.**

- **B (tiered):** keep WIP as an optional profile for local models and subagents if it
  helps there; expand to other models only when it beats generic on both cost and outcome.
- **D (time-boxed):** first review when 20 real tasks and 10 subagent tasks are recorded,
  or after 6 weeks, whichever comes first. Fixed evaluation budget.
- **Expand** only with >= 10% token reduction or a measurable task/review improvement,
  no task-success regression, and no worse cost per successful task.
- **Keep tiered** if it helps local models/subagents but not frontier models.
- **Retire or keep experimental** if there is no measurable benefit after the review.
- A null result for System-1 routing does not block the tiered keep; it only means static
  profiles are sufficient for routing.

The decision is recorded in `FUTURE-PROOFING-017`; the review trigger is tracked there.

## Data and provenance

Record for every retrieval:

- provider and WIP version,
- capability and representation,
- map schema version,
- candidates, confidence, and provenance,
- plan signals,
- fallback reason,
- downstream task/review outcome.

This feeds the decision log and the owned data flywheel. It is subject to
`wiki/decision-log-egress.md`; code content is sensitive and local-first.

Tracked by `SCOPE-008`.

## Packaging and schema versioning

- The native provider must be consumable as a pinned artifact, not a per-machine C build.
- Reuse the `INFRA-006`/`INFRA-007` artifact path.
- Version the native provider and the map schema independently.
- A missing or incompatible artifact degrades to generic retrieval.
- The source build remains a contributor path.

Tracked by `FUTURE-PROOFING-016`.

## Decision-layer integration

WIP supplies evidence; the decision layer decides:

| Decision class | WIP input / output |
|---|---|
| WIP routing | choose generic vs WIP, rung, budget, and sufficiency within profile bounds |
| Retrieval sufficiency | candidate coverage, anchors, confidence, token estimate |
| Compaction trigger | delta size, token estimates, which spans must survive |
| Review triage | delta, observability, duplicate/refactor signals |
| Model/tier routing | task complexity signal (advisory only) |

One-way rule: map -> decision -> runtime. The map never advances state, and the decision
layer never lets the map widen its own authority. A WIP-routing decision can narrow the
profile, never widen it.

Tracked by `DECISION-016` and `DECISION-017`.

## Gaps this closes

1. ScopeMux was absent from the substrate docs.
2. Activation was hardcoded to a model id.
3. No capability ladder; all-or-nothing.
4. No evaluation/exit criteria.
5. No data contract for retrieval provenance.
6. Packaging was not part of the substrate.
7. No versioned protocol or capability negotiation.
8. No decision-layer interaction.
9. Terminology was overloaded.
10. The "grep replacement" card undersold the role.

## Non-goals

- Replacing `opencode-lsp`.
- Owning task state, lifecycle, verification, review, or completion.
- Making ScopeMux a V1 hard dependency.
- A general-purpose platform for every harness.
- Treating heuristic links as truth.
- Sending private code to a hosted engine.
- Building a second core loop.

## Board

- `FUTURE-PROOFING-013` — Workspace Intelligence Protocol program (epic)
- `SCOPE-006` — map-backed locate tool with grep fallback (one WIP capability)
- `FUTURE-PROOFING-014` — WIP contract, capability negotiation, and versioning
- `FUTURE-PROOFING-015` — capability-profile activation
- `FUTURE-PROOFING-016` — artifact packaging and map schema versioning
- `FUTURE-PROOFING-017` — evaluation and keep/expand/retire criteria
- `DECISION-016` — map signals into retrieval, compaction, and review decisions
- `DECISION-017` — System-1 routing of WIP provider/rung/budget at runtime
- `SCOPE-007` — subagent-scoped workspace intelligence
- `SCOPE-008` — retrieval provenance and outcomes into the data flywheel
- `SCOPE-009` — map freshness/staleness SLOs
- `SCOPE-010` — language coverage and fallback matrix
- `SCOPE-011` — out-of-process WIP transport

## Open decisions

- Whether WIP transport should be stdio/JSON-RPC or another protocol.
- Whether a profile may enable WIP for frontier models, or only local/subagent paths in
  the first phase.
- Whether System-1 WIP routing is advisory or gated after evaluation. It cannot widen a
  profile either way.
