---
id: "SCOPE-003"
title: "Project authoritative task state into scopemux plan nodes"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "todo"
created: "2026-09-23"
predecessor: "SCOPE-002"
---

# Project authoritative task state into scopemux plan nodes

## Summary

Bridge the runtime's authoritative task record to `scopemux` target-state nodes so a task's intended change is represented in the map as plan InfoBlocks, and reconciled as the code changes.

This is Phase 2 of `SCOPE-001`. It does not give `scopemux` any authority over task state.

## Why this exists

`SCOPE-001` defines target state as additional InfoBlock nodes that describe code that does not exist yet. Without a defined contract, plan nodes would either be authored ad hoc or would duplicate task intent, which would break `invariants/task-state.md` ("structured task state is authoritative for bounded work execution").

## Scope

- Define the projection contract from the durable task record to plan nodes: which fields (`objective`, `completion_criteria`, `stage`, `workspace_target`, `artifacts`, `reopen_reason`) map to which plan-node attributes.
- Implement the projection so plan nodes carry anchors, desired shape, rationale tied to completion criteria, provenance, and lifecycle.
- Implement the reconciliation handshake: observed code changes and plan-node lifecycle transitions are reported back to the runtime as evidence, and the runtime remains the only writer of task stage and completion.
- Surface plan-node conflicts (`stale`, `conflict`) to the runtime as explicit signals so a task can be re-planned instead of silently diverging.
- Keep the durable task record in `crates/opencode-types` / `crates/opencode-storage` / `crates/opencode-session` as the sole authority; `scopemux` holds only projections.

## Non-goals

- Moving task, stage, verification, review, or completion authority into `scopemux`.
- Persisting task state only as plan nodes.
- Auto-advancing a task stage from map evidence without a runtime-recorded transition.

## Done when

- A task record produces plan nodes with anchors, rationale, provenance, and lifecycle.
- Plan-node lifecycle changes are reported to the runtime without advancing task stage.
- `stale`/`conflict` plan nodes surface as explicit re-plan signals.
- The runtime still completes tasks with no plan nodes present (generic path unaffected).
- The task record round-trips unchanged; no field authority is transferred.

## Recommended verification

- Create a task, project it, mutate the anchored code, and confirm plan nodes reconcile and conflicts surface.
- Confirm the durable task record wins on any disagreement with plan-node state.
- Confirm a task completes when `scopemux` is absent.

## Dependencies and design (2026-09-24)

Gated on upstream `scopemux-core` work, sequenced by `H-011`: `WI-033` (origin/lifecycle/provenance/confidence on `ProjectInfoBlock`), `WI-032` (target-state plan InfoBlocks + reconciliation), then `WI-036` (delta/query API); `WI-031` persists plan nodes. Only then can this card project task state into real plan nodes.

Product-side shape:

- Extend `opencode-scopemux` FFI bindings and the retrieval contract to carry origin/lifecycle and delta entries; keep the `ProjectSearchResult`/`ProjectInfoBlock` layout in lockstep with the pinned `scopemux-core` revision.
- Map task-record fields (`objective`, `completion_criteria`, `stage`, `workspace_target`, `artifacts`, `reopen_reason`) to plan-node attributes.
- Reconciliation handshake: plan-node lifecycle transitions and `stale`/`conflict` are evidence returned to the runtime; the runtime remains the sole writer of stage and completion.

Do not start implementation until `WI-032`/`WI-033`/`WI-036` land; the runtime must keep completing tasks with no plan nodes present.

## Related Items

- `SCOPE-001` Idealized scopemux map integration.
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary - predecessor.
- `SCOPE-004` Consume map delta, observability, and refactor nodes in context assembly.
- `START-016` Define structured task state for V1.
- Upstream (in-scope integration work per `invariants/integration-scope.md`): `WI-032`, `WI-033`, `WI-036`, plus `FIX-002` (block-type disambiguation) and `FIX-006` (planning-doc alignment) in `$HOME/apps/scopemux-notes`.

## Notes

- Depends on `SCOPE-002` and the upstream plan-InfoBlock work (`WI-032`, `WI-033`), which the integration program owns and develops in `$HOME/apps/scopemux-notes`.
- Authority rule: if a plan node and the task record disagree, the task record wins.
