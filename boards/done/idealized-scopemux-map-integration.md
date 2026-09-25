---
id: "SCOPE-001"
title: "Idealized scopemux map integration"
priority: "P2"
type: "research"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "done"
created: "2026-09-21"
---

# Idealized scopemux map integration

## Summary

Define the idealized way `scopemux` helps `scopemux-code`: a live **map** of the workspace that converts a codebase into layered representations an agent can concisely reference and jump into, so tasks complete with less context.

The map is more than the parser and more than an index. It holds the **current state** (parsed code) and the **target state** (planned nodes that become real code), and it exposes the **delta** between them. Target-state nodes are InfoBlocks that help with system understanding, observability when debugging, and DRY by surfacing refactor opportunities. The map is kept current by incremental reconciliation rather than constant full re-indexing.

This card is the design and vision. The full design lives in `wiki/scopemux-map-integration.md`. Implementation is split into `SCOPE-002`, `SCOPE-003`, and `SCOPE-004`. Integration work includes the enabling `scopemux` development it depends on: the upstream work tracked in `$HOME/apps/scopemux-notes` is in-scope for this program, not a separate owner's prerequisite (`invariants/integration-scope.md`).

## Why this exists

`scopemux-core` today parses and produces structural IR, InfoBlocks, tiered context, search, and prompt assembly. That is enough to describe what exists, but it cannot represent where a task is going, which is where most of the agent context savings are. `SCOPE-002` (the near-term provider integration) only connects the existing capabilities behind `START-025`; this card captures the larger target so that near-term work does not foreclose it.

The concrete pain driving the target is that the map must currently be re-indexed on every change, and that planned work has no representation in the map at all.

## Core Model

Use `scopemux` as a mapper, not merely an index. Four operations and five layers:

| Operation | Question it answers |
| --- | --- |
| **Locate** | Which nodes matter for this task and stage? |
| **Slice** | What is the cheapest faithful representation of those nodes? |
| **Project** | What is the delta between current and target state? |
| **Reconcile** | How do we keep the map correct after changes? |

| Layer | Content |
| --- | --- |
| **L0 Parse** | CST/AST, byte and line ranges |
| **L1 Structural IR** | symbols, references, call graph, import/dependency graph, scopes |
| **L2 InfoBlocks** | addressable units, tiers 0-4, token estimates, dispositions |
| **L3 Task / plan** | projected target-state nodes, lifecycle, anchors, rationale |
| **L4 Views** | compositions, tiered contexts, delta views, review and debug slices |

L3 is what turns a parser-plus-index into a map.

## Target State (Plan Nodes)

Target state is represented by additional InfoBlocks layered into the same registry as current-state nodes:

- a new symbol, file, module, or test to add
- a modification to an existing symbol (signature, behavior, contract)
- a removal or consolidation
- a refactor opportunity to resolve
- an observability point to add

Each plan node carries a stable id, a kind and desired shape, anchors to current-state nodes, rationale tied to the task's completion criteria, provenance, and a lifecycle state:

```
planned -> in_progress -> implemented -> verified -> documented
             \-> stale / conflict / abandoned
```

Plan nodes share the registry so graph traversal, tiers, search, and duplication analysis work across current and target state, but they are always distinguished by origin and lifecycle. A plan node whose anchors change is marked `stale` or `conflict`, never silently dropped.

## Authority And Guardrails

Plan nodes are a projection of the runtime's authoritative task record, not a second source of truth. Per `invariants/task-state.md`, the durable task record owns objective, completion criteria, stage, verification, and review. `scopemux` may materialize and reconcile nodes but never advances a stage, declares completion, or overrides the task record. Per `invariants/retrieval.md`, structural links are confidence-labeled and never treated as guaranteed truth. Per `invariants/context.md`, all map output is role-specific and token-budgeted.

## Delta, Observability, And DRY

- **Delta view.** `current (+) target = { add, change, remove, reuse }`, returned with anchors, projected shape, token cost, provenance, and confidence. This is the representation an agent actually needs during work.
- **Observability InfoBlocks.** Log points, metrics, assertions, invariant checks, expected failure modes, and error annotations attached to real or planned symbols, so a debugging request resolves to a focused slice instead of a file dump.
- **Refactor opportunities.** Structural similarity across InfoBlocks and graph roles produces `refactor_opportunity` nodes clustering near-duplicates with confidence and estimated cost, enabling "reuse this instead" as evidence-backed work.

## Incremental Reconciliation

Split the map into a **derived store** (parse, IR, InfoBlocks, graph, search index; disposable, content-hashed) and a **durable map store** (plan nodes, lifecycle, provenance, observability tags, anchors). Re-index refreshes the derived store and reconciles the durable store; it never wipes plan state. Invalidation is dependency-scoped: watch, debounce, hash-gate, re-parse only changed files, walk reverse edges to build a dirty set, recompute only dirty artifacts, and re-anchor plan nodes. Correctness criterion: incremental equals a full derived rebuild with the durable store reconciled.

## Runtime Integration

All map use crosses the `START-025` retrieval-provider boundary. Each runtime stage maps to a role and a representation slice:

| Runtime stage | Map role | Representation |
| --- | --- | --- |
| `selected` | locate | seed nodes from objective; candidate anchors |
| `context_prepared` | slice | delta plus anchors, dependencies, observability at a budget |
| `implementing` | slice / project | delta entries with desired shapes; reuse candidates |
| `verifying` | locate | completion-criteria anchors and covering tests |
| `reviewing` | project | delta, changed symbols, observability, refactor flags |
| `repairing` | locate / project | reopen reason mapped to affected anchors and observability |
| `completed` | reconcile | mark plan nodes implemented/verified; refresh derived state |

The generic provider remains the default and fallback; unsupported language is an explicit provenance signal. Rust grammar and Rust reference resolution are phase-one enabling work because this product's own repository is Rust.

## Prerequisites

Every prerequisite is tracked in this repo or in `$HOME/apps/scopemux-notes`. No phase starts until its prerequisites are met. Under `invariants/integration-scope.md`, these upstream items are in-scope deliverables of the integration program rather than external blockers; they are developed in their own repository.

| Prerequisite | Home | Gates | Status |
| --- | --- | --- | --- |
| `START-025` retrieval-provider boundary | this repo | `SCOPE-002` implementation | done (PR #104) |
| `FIX-001` C++ resolver registration/declaration | scopemux-notes | trustworthy C API core for `SCOPE-002` | done (`scopemux-core` PR #7) |
| `FIX-003` Python interpreter range alignment | scopemux-notes | pinned/reproducible build for `SCOPE-002` | done (`scopemux-core` PR #7) |
| `FIX-004` Python API surface correction | scopemux-notes | honest C-only boundary for `SCOPE-002` | done (`scopemux-core` PR #7) |
| `FIX-002` InfoBlock terminology disambiguation | scopemux-notes | trustworthy context model for Phase 2/3 | done (`scopemux-core` PR #7) |
| `FIX-006` planning-doc alignment | scopemux-notes | docs cannot be mistaken for implementation | done (`scopemux-notes` `9691e45`) |
| `FIX-005` duplicate `parser_free` declaration | scopemux-notes | clean parser public surface | done (`scopemux-core` PR #7) |
| `TESTS-006` C example AST mismatch | scopemux-notes | clean `run_c_tests.sh` baseline | done (`scopemux-core` PR #8) |
| `WI-030` Rust grammar and reference resolution | scopemux-notes | Phase 1 usefulness on this repo | todo |
| `WI-018` incremental index and watcher | scopemux-notes | map stays current | todo |
| `WI-031` durable map store | scopemux-notes | plan persistence | todo |
| `WI-032` target-state (plan) InfoBlocks | scopemux-notes | `SCOPE-003` | todo |
| `WI-033` InfoBlock origin/lifecycle/provenance | scopemux-notes | `WI-032`, `WI-034` | todo |
| `WI-034` observability InfoBlocks | scopemux-notes | `SCOPE-004` | todo |
| `WI-035` duplication/refactor detection | scopemux-notes | `SCOPE-004` | todo |
| `WI-036` delta view and map query API | scopemux-notes | `SCOPE-003`, `SCOPE-004` | todo |

- The core invariants that constrain the upstream fixes are `$HOME/apps/scopemux-notes/invariants/`.
- `FIX-001`-`FIX-006` are correctness prerequisites with no standalone feature value; they are composed by `$HOME/apps/scopemux-notes/handoffs/FIX-001-006-invariant-conflict-fixes-handoff.md` (`H-001`).
- `SCOPE-002` was gated by `START-025`; that boundary now exists (PR #104). The `FIX-*` items are done and merged (`scopemux-core` PR #7), and `TESTS-006` cleared the C test baseline (PR #8), so `SCOPE-002` is unblocked.

## Phasing

- **Phase 0 - Boundary.** `START-025` and `SCOPE-002`. No plan nodes required.
- **Phase 1 - Map core.** Rust grammar/resolution, incremental reconciliation, InfoBlock origin/lifecycle/provenance, delta view, agent map API. Upstream work in `$HOME/apps/scopemux-notes` (`WI-018`, `WI-030`-`WI-036`), with `FIX-001`-`FIX-006` clearing core conflicts first.
- **Phase 2 - Plan projection.** Project the task record into plan nodes and reconcile them (`SCOPE-003`).
- **Phase 3 - Consumption.** Stage-aware retrieval of delta, observability, and refactor nodes (`SCOPE-004`).
- **Later.** Refactor automation, richer observability and trace IR, cross-repo maps.

## Non-goals

- Making `scopemux` a V1 hard dependency.
- Porting or rewriting `scopemux-core` parsers.
- Letting `scopemux` own task state, lifecycle, verification, review, or completion.
- Treating heuristic links as semantic truth.
- Replacing direct file reads for small, obvious tasks.

## Open Questions

- Must plan nodes come only from task-record projection, or may they be authored directly and reconciled back into task criteria?
- What durable serialization and location should the map store use?
- How aggressively may a plan node transition to `implemented` from observed code?
- Does duplication detection always run, or only on request?
- How much observability context fits a typical local-model token budget?

## Done when

- The idealized design is captured in `wiki/scopemux-map-integration.md` and indexed from `wiki/README.md`.
- Near-term and phase implementation is represented by explicit cards (`SCOPE-002`, `SCOPE-003`, `SCOPE-004`) rather than implied intent.
- Enabling upstream work is represented by explicit cards in `$HOME/apps/scopemux-notes`.
- The design is consistent with `invariants/task-state.md`, `invariants/retrieval.md`, and `invariants/context.md`.

## Related Items

- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary (Phase 0).
- `SCOPE-003` Project authoritative task state into scopemux plan nodes (Phase 2).
- `SCOPE-004` Consume map delta, observability, and refactor nodes in context assembly (Phase 3).
- `START-025` Add retrieval-provider boundary for task context assembly (prerequisite).
- `START-007` Plan ScopeMux integration.
- `PHASE-003` V2 reliability.

## Notes

- This is a design item. Do not start `SCOPE-002` implementation before `START-025` exists.
- Upstream `scopemux-notes` items: `WI-018` (incremental index/watcher, refined), `WI-030` (Rust grammar), `WI-031` (durable map store), `WI-032` (plan InfoBlocks), `WI-033` (InfoBlock origin/lifecycle), `WI-034` (observability blocks), `WI-035` (duplication/refactor detection), `WI-036` (delta and map query API).
- Core conflict fixes: `FIX-001`-`FIX-006`, composed by `handoffs/FIX-001-006-invariant-conflict-fixes-handoff.md` (`H-001`); core invariants at `$HOME/apps/scopemux-notes/invariants/`.
- Cross-repo material is referenced by path only; binding rules are restated in this repo per `invariants/documentation-boundary.md`.
- Closed out 2026-09-23 (design-only). Done-when verified: design captured in `wiki/scopemux-map-integration.md` and indexed from `wiki/README.md`; phases represented by `SCOPE-002`/`SCOPE-003`/`SCOPE-004`; upstream `FIX-001`-`FIX-006` and `WI-018`, `WI-030`-`WI-036` exist as cards in `$HOME/apps/scopemux-notes`; design consistent with `invariants/task-state.md`, `invariants/retrieval.md`, and `invariants/context.md`. No implementation is claimed here; execution remains gated on the prerequisites table, which is in-scope integration work per `invariants/integration-scope.md`.
