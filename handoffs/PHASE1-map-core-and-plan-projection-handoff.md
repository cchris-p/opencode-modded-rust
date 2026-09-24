---
id: "H-011"
title: "Phase 1/2 map core and plan projection - Handoff"
status: "open"
created: "2026-09-24"
updated: "2026-09-24"
owner: ""
target: "$HOME/apps/scopemux-core, $HOME/apps/scopemux-notes"
blocked_reason: ""
needs_human: ""
items: ["WI-033", "WI-032", "WI-036", "WI-031", "SCOPE-003"]
---

# Phase 1/2 Map Core And Plan Projection - Handoff

## Objective

Extend `scopemux-core` with the fields and API the map needs (`WI-033`, `WI-032`, `WI-036`, `WI-031`) and project the product's authoritative task state into plan nodes behind the `START-025` boundary (`SCOPE-003`). Phase 0 (`START-025`, `SCOPE-002`) is complete: the product consumes `scopemux-core` through `opencode-scopemux::ScopemuxProvider`.

## Sequencing (dependencies are explicit)

1. **`WI-033` InfoBlock origin, lifecycle, provenance, confidence** (`scopemux-core`). Foundational: add `origin` (`parsed`/`planned`), `lifecycle`, `provenance`, `confidence` to `ProjectInfoBlock`, plus the `sym:`/`file:`/`plan:` id scheme, and origin/lifecycle filters on search and tiered context. Everything else depends on this.
2. **`WI-032` target-state (plan) InfoBlocks** (`scopemux-core`). Plan-node kinds, fields, lifecycle (`planned`…`documented`, `stale`/`conflict`/`abandoned`), registry integration, reconciliation against parsed state, and create/update/query/reconcile APIs.
3. **`WI-031` durable map store** (`scopemux-core`). Split derived vs durable storage and persist plan nodes without wiping them on re-index. Can proceed in parallel with `WI-032` once `WI-033` fields exist.
4. **`WI-036` delta view and query API** (`scopemux-core`). Delta request/response and the map query operations (`resolve`, `node`, `expand`, `neighbors`, `delta`, `duplicates`, `observability`, `change_impact`), all carrying provenance, confidence, and token estimates.
5. **`SCOPE-003`** (`opencode-modded-rust`). Project the durable task record into plan nodes and reconcile through the FFI provider, keeping the task record authoritative.

`WI-018` (incremental index/watcher) and `WI-030` (Rust grammar) are Phase 1 enablers and can be scheduled independently; `WI-030` is what makes the map useful on this product's own Rust repo.

## Product-side contract (`SCOPE-003`)

- Projection contract: map task-record fields (`objective`, `completion_criteria`, `stage`, `workspace_target`, `artifacts`, `reopen_reason`) to plan-node attributes.
- Reconciliation handshake: plan-node lifecycle transitions and `stale`/`conflict` are reported to the runtime as evidence; the runtime remains the only writer of stage and completion.
- Extend `opencode-scopemux` FFI bindings and `RetrievalRequest`/`RetrievalResponse` (or a new delta request/response) to carry origin/lifecycle and delta entries.
- The runtime still completes tasks with no plan nodes present.

## Cross-cutting constraints

- `scopemux-core` is pinned in the product via `scripts/fetch-scopemux-core.sh`. **`WI-033` changes the `ProjectInfoBlock` layout**, so the FFI structs in `crates/opencode-scopemux/src/lib.rs` and the pinned revision must be updated in lockstep; the product's native build and tests must be re-run.
- Plan nodes are projection-only; the task record stays authoritative (`invariants/task-state.md`).
- Structural links stay confidence-labeled (`invariants/retrieval.md`); context stays role-specific and budgeted (`invariants/context.md`).

## Verification

- `scopemux-core`: per-language + interfile C tests via `scripts/docker_test.sh`; new tests for origin/lifecycle filters, plan-node lifecycle/reconciliation, and delta output.
- Product: default `cargo test`/clippy; native `cargo test -p opencode-scopemux --features native`; a task projected into plan nodes reconciles and surfaces `stale`/`conflict` without advancing stage; the task completes when scopemux is absent.
- Record the updated pinned revision after each upstream merge.

## Risks

- `WI-033`'s field additions are a breaking `ProjectInfoBlock` layout change for the FFI provider; land it with the product binding update in the same coordinated change.
- `WI-032`/`WI-036` are large; keep each card independently shippable and avoid making plan nodes a dependency for basic retrieval.
- `WI-030` (Rust grammar) is a prerequisite for the map to be useful on this repo; it is separate and sizeable.

## Exit criteria

- The registry distinguishes parsed and planned blocks, and plan nodes carry lifecycle, provenance, and confidence.
- Delta and query APIs return machine-readable results for the product to consume behind the `START-025` boundary.
- `SCOPE-003` projects the task record into plan nodes, reconciles them, and never overrides task authority.
- Pin, FFI bindings, and verification are updated in lockstep.
