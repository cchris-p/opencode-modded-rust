# ScopeMux Map Integration

## Purpose

Define the idealized way `scopemux-core` should serve `scopemux-code`: not as a search index bolted onto the runtime, but as a live **map** of the workspace that lets an agent understand a task, change it, and verify it using as little context as possible.

This document is the target for `SCOPE-001` and its follow-on cards. It extends, and does not replace, `wiki/scopemux-integration-plan.md`, which still defines the retrieval-provider boundary and the V1 guardrails. Where this document describes capabilities that do not exist in `scopemux-core` yet, it names the upstream work explicitly rather than assuming it.

## The Mapper Model

An index answers a query about what already exists. A map does three things an index does not:

1. **Navigation.** It knows where things are and how to jump to them, so an agent can move from a symptom or objective to the exact node.
2. **Projection.** It can hold places that are *planned but not yet built*, and show the difference between the current codebase and the intended one.
3. **Reconciliation.** It stays current as both code and intent change, without a full rebuild.

The expensive resource for a coding agent is context, not compute. The map exists to return the smallest high-signal representation for the current task stage, always with provenance, plus a concrete path to jump into the real code.

Four operations frame the whole integration:

| Operation | Question it answers | Runtime consumer |
| --- | --- | --- |
| **Locate** | Which nodes matter for this task and stage? | retrieval provider |
| **Slice** | What is the cheapest faithful representation of those nodes? | context assembly |
| **Project** | What is the delta between current state and target state? | planning, review, repair |
| **Reconcile** | How do we keep the map correct after changes? | watcher / incremental indexer |

## Representation Layers

The map is layered so each consumer pays only for the resolution it needs.

| Layer | Content | Primary consumer |
| --- | --- | --- |
| **L0 Parse** | CST/AST, byte and line ranges | parser, structural queries |
| **L1 Structural IR** | symbols, resolved references, call graph, import/dependency graph, scopes | retrieval, impact analysis |
| **L2 InfoBlocks** | addressable semantic units with tiers 0-4, token estimates, dispositions | context assembly |
| **L3 Task / plan** | projected target-state nodes, lifecycle, anchors, rationale | planning, delta, review |
| **L4 Views** | compositions, tiered contexts, delta views, review and debug slices | the runtime |

L3 is the layer that turns a parser-plus-index into a map. Without it, `scopemux` can describe the codebase but cannot represent where a task is going.

## Current State And Target State

### Current state

Current state is derived from parsed source. It is recomputable, content-hashable, and disposable. It is what `scopemux-core` produces today through `ProjectContext` and the `ProjectInfoBlockRegistry`.

### Target state

Target state is represented by additional nodes - **plan InfoBlocks** - layered into the same registry as current-state nodes. A plan node is a projected semantic unit that describes code that does not exist yet, or an intended change to code that does:

- a new symbol, file, module, or test to add
- a modification to an existing symbol (signature, behavior, contract)
- a removal or consolidation
- a refactor opportunity to resolve
- an observability point to add (see below)

Each plan node carries:

- a stable id (`plan:<task_id>:<slug>`)
- a kind and desired shape (signature, structure, expected symbols)
- **anchors** to current-state nodes it will touch (`sym:...`, `file:...`, block ids)
- rationale and references to the task's completion criteria
- provenance (which task record and stage produced it)
- a lifecycle state

Plan nodes live in the same InfoBlock registry so graph traversal, tier selection, search, and duplication analysis work uniformly across current and target state. They are distinguished by an explicit origin and lifecycle, never mixed silently with parsed nodes.

### Lifecycle

Plan nodes move through a lifecycle that mirrors real delivery:

```
planned -> in_progress -> implemented -> verified -> documented
             \-> stale / conflict / abandoned
```

Transitions are detected from observable evidence where possible (a projected symbol appearing in the parsed layer, a matching change landing), and otherwise set explicitly. A plan node whose anchors change or vanish is marked `stale` or `conflict` rather than deleted; that signal is surfaced to the runtime so a task can be re-planned instead of silently diverging.

### Authority

Plan nodes are a **projection of the runtime's authoritative task record**, not a second source of truth. Per `invariants/task-state.md`, the durable task record owns objective, completion criteria, stage, verification, and review. `scopemux` may materialize, annotate, and reconcile those nodes, but it must never advance a stage, declare completion, or override the task record. If a plan node and the task record disagree, the task record wins.

## Delta View

The representation an agent actually needs during work is the delta between current and target state:

```
current (parsed)  (+)  target (plan)  ->  { add, change, remove, reuse }
```

The delta view returns, per entry: affected anchors, the projected shape, token cost, provenance, and confidence. This is what turns "understand this codebase" into "here are the three edits and the one refactor this task implies", which is where the context savings come from.

## Observability Information Blocks

System understanding is not only structural. The map should carry **observability InfoBlocks**: log points, metrics, assertions, invariant checks, expected failure modes, and error annotations attached to real or planned symbols.

Two sources feed them:

- a plan node can declare the observability it expects to add (`kind = observability`)
- an implemented symbol can be tagged with the observability it actually carries

During debugging, the map resolves a failing symbol or error to its observability blocks, recent change provenance, and covering tests, so the agent gets a focused debugging slice instead of a file dump. This reuses the `Error Annotation IR` and `Execution Trace IR` concepts already catalogued in the `scopemux-notes` IR inventory, but scoped to what a task needs.

## DRY And Refactor Opportunities

Because current and planned nodes share one registry and one graph, the map can detect structural duplication directly:

- normalized AST-subtree and signature similarity across InfoBlocks
- shared call-graph or dataflow role between otherwise separate symbols
- planned nodes that would reintroduce duplication already present elsewhere

The result is `refactor_opportunity` nodes that cluster near-duplicates with a confidence score and an estimated duplication or token cost. These follow `invariants/retrieval.md`: they are heuristic, confidence-labeled, and never treated as guaranteed truth. They let the runtime offer "reuse this instead" or "resolve this duplication" as an explicit, evidence-backed task rather than a vague suggestion.

## Incremental Reconciliation

The central problem today is that the map must be re-indexed whenever anything changes. The fix is to split the map into two stores and invalidate by dependency, not by rebuild.

### Two stores

- **Derived store** - parse results, IR, InfoBlocks, graph edges, search index. Cheap to rebuild, content-hashed, and safe to discard.
- **Durable map store** - plan nodes, lifecycle, provenance, observability tags, refactor decisions, and anchors. Must survive re-index and restart.

Re-index refreshes the derived store and **reconciles** the durable store. It must never wipe plan state.

### Scoped invalidation

1. Watch the filesystem; debounce and batch events.
2. Gate on content hash (for example xxh3) plus mtime, so no-op writes are ignored.
3. Re-parse only changed files and recompute only their symbols and InfoBlocks.
4. Walk reverse edges (callers, importers, dependents) to build a **dirty set**.
5. Recompute only the dirty InfoBlocks, graph edges, index entries, and derived search results.
6. Re-anchor plan nodes in the dirty set; mark any whose anchors changed as `stale` or `conflict`.

The correctness criterion is strict: an incrementally updated map must be equivalent to a full rebuild of the derived store, with the durable store reconciled rather than regenerated.

## Runtime Integration

All map use crosses the `START-025` retrieval-provider boundary. The runtime asks; the map answers; prompt construction consumes the answer. No `scopemux`-specific logic leaks into orchestration, lifecycle, verification, or review.

### Stage-aware retrieval

Each runtime stage maps to a retrieval role and a representation slice:

| Runtime stage | Map role | Representation |
| --- | --- | --- |
| `selected` | locate | seed nodes from objective; candidate anchors |
| `context_prepared` | slice | delta plus anchors, dependencies, and observability at a token budget |
| `implementing` | slice / project | delta entries with desired shapes; reuse candidates |
| `verifying` | locate | completion-criteria anchors and covering tests |
| `reviewing` | project | delta, changed symbols, observability, refactor and duplicate flags |
| `repairing` | locate / project | reopen reason mapped to affected anchors, prior plan nodes, and observability |
| `completed` | reconcile | mark plan nodes `implemented`/`verified`; refresh derived state |

Every result carries file/symbol/block provenance, confidence, and a token estimate. The runtime keeps authority over budgets, inclusion thresholds, and role filtering; review context uses a stricter inclusion threshold than implementation context.

### Fallback

The generic repository-local provider stays the default and the fallback. When `scopemux` is absent, fails, or cannot parse the workspace language, context assembly still completes. Unsupported language is an explicit provenance signal, not a silent downgrade.

## Language Coverage

The product's own repository is Rust, and `scopemux-core` does not parse Rust today. A map that cannot map its own workspace is not usable as the primary path, so Rust grammar and Rust reference resolution are phase-one enabling work, not an afterthought. Until that lands, the Rust workspace exercises only the generic provider.

## Confidence And Provenance

- Exact repository facts outrank heuristic relationships.
- Directly referenced nodes outrank inferred neighbors.
- Every projected value (similarity, dependency, duplication, observability relevance) is labeled with confidence.
- Provenance is retained through incremental updates so bad retrieval can be diagnosed later.
- Low-confidence links may enrich context but never override explicit task scope.

## Phasing

- **Prerequisites.** Upstream core conflict fixes (`FIX-001`-`FIX-006`, handoff `H-001`) and enabling items (`WI-018`, `WI-030`-`WI-036`) in `$HOME/apps/scopemux-notes`; the product boundary `START-025` gates `SCOPE-002`.
- **Phase 0 - Boundary.** Land `START-025` and the near-term provider integration (`SCOPE-002`). No plan nodes required.
- **Phase 1 - Map core.** Rust grammar and resolution, incremental reconciliation, InfoBlock origin/lifecycle/provenance, delta view, agent map API. This is the minimum that makes the map useful on this product's own repo.
- **Phase 2 - Plan projection.** Project the authoritative task record into plan nodes and reconcile them (`SCOPE-003`).
- **Phase 3 - Consumption.** Stage-aware retrieval of delta, observability, and refactor nodes (`SCOPE-004`).
- **Later.** Refactor automation, richer observability and trace IR, cross-repo maps.

Each phase is independently useful and must not make the previous phase a hard dependency for basic operation.

## Non-Goals

- Making `scopemux` a V1 hard dependency or a build requirement for the daily driver.
- Porting `scopemux-core` to Rust or rewriting its parsers.
- Letting `scopemux` own task state, lifecycle, verification, review, or completion.
- Treating heuristic links as semantic truth.
- Replacing direct file reads for small, obvious tasks.

## Open Questions

- Should plan nodes be authored only by projection from the task record, or may a human/agent author them directly and have them reconciled back into task criteria?
- What is the durable serialization for the map store (FlatBuffers vs JSON vs a local database), and where does it live relative to the runtime's own storage?
- How aggressively should a plan node transition to `implemented` from observed code, versus requiring an explicit confirmation?
- Does duplication detection run always, or only when a stage requests refactor analysis?
- How much observability context can be paid for in a typical local-model token budget?

## Related Board Items

- `SCOPE-001` Idealized scopemux map integration (this document).
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary (Phase 0).
- `SCOPE-003` Project authoritative task state into scopemux plan nodes (Phase 2).
- `SCOPE-004` Consume map delta, observability, and refactor nodes in context assembly (Phase 3).
- `START-025` Add retrieval-provider boundary for task context assembly (prerequisite).
- `START-007` Plan ScopeMux integration (deferred contract this document extends).

## Cross-Repo References

Referenced by path, not by inheritance. Upstream scope and IR context live in `$HOME/apps/scopemux-notes` (`wiki/concepts.md`, `wiki/indexing-pipeline.md`, `wiki/supported-ir-structures.md`, `features/mapping-dependency-relationships.md`, `features/watcher-and-incremental-indexer.md`, `features/documentation-suite-feature-archived.md`), the core invariants in `$HOME/apps/scopemux-notes/invariants/`, and the conflict-fix program in `$HOME/apps/scopemux-notes/handoffs/FIX-001-006-invariant-conflict-fixes-handoff.md` (`H-001`). The engine is `$HOME/apps/scopemux-core` (`core/include/scopemux/`). Any rule that must bind `scopemux-code` is restated here or in `invariants/`.
