---
id: "SCOPE-004"
title: "Consume map delta, observability, and refactor nodes in context assembly"
priority: "P3"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "todo"
created: "2026-09-23"
predecessor: "SCOPE-003"
---

# Consume map delta, observability, and refactor nodes in context assembly

## Summary

Make retrieval stage-aware so each runtime stage asks the `scopemux` map for the representation it actually needs: delta entries, observability blocks, or refactor opportunities, at a token budget, with provenance and confidence.

This is Phase 3 of `SCOPE-001`. It consumes the retrieval-provider boundary from `START-025` and the map capabilities built in Phase 1/2.

## Why this exists

Connecting `scopemux` (`SCOPE-002`) and projecting target state (`SCOPE-003`) only pays off if the runtime requests the right slice for each stage. Without stage-aware roles, the map would still return roughly the same retrieval for planning, implementation, verification, review, and repair, wasting the target-state and observability work.

## Scope

- Map each runtime stage (`selected`, `context_prepared`, `implementing`, `verifying`, `reviewing`, `repairing`, `completed`) to a map role and representation slice as defined in `wiki/scopemux-map-integration.md`.
- Assemble implementation context from the delta view plus anchors, dependencies, and observability.
- Assemble review context from delta, changed symbols, observability, and refactor/duplicate flags, using a stricter inclusion threshold than implementation.
- Assemble repair context from the recorded `reopen_reason` mapped to affected anchors, prior plan nodes, and observability.
- Interpret provenance and confidence from map results, keeping exact facts above heuristic links.
- Keep the generic provider as the default and fallback for every stage.

## Non-goals

- Giving the map authority over stage transitions, verification, review, or completion.
- Removing the generic provider path.
- Building refactor automation beyond surfacing `refactor_opportunity` nodes.

## Done when

- Each stage issues a distinct map request and consumes a distinct representation slice.
- Review and repair contexts demonstrably use map provenance and confidence rather than raw file recency.
- A debugging/review path can resolve a symbol to its observability blocks and covering tests.
- Every stage still completes with the generic provider when `scopemux` is unavailable.
- The runtime remains authoritative for inclusion thresholds and token budgets.

## Recommended verification

- Run the same task with generic retrieval and map-aware retrieval and compare context size and task outcome at each stage.
- Trigger a repair cycle and confirm the reopen reason drives map anchoring.
- Confirm an unsupported-language workspace falls back cleanly.

## Related Items

- `SCOPE-001` Idealized scopemux map integration.
- `SCOPE-003` Project authoritative task state into scopemux plan nodes - predecessor.
- `START-025` Add retrieval-provider boundary for task context assembly.
- Upstream (in-scope integration work per `invariants/integration-scope.md`): `WI-034` (observability blocks), `WI-035` (refactor detection), `WI-036` (delta and map query API), plus `FIX-002` (context model disambiguation) in `$HOME/apps/scopemux-notes`.

## Notes

- Depends on `SCOPE-003` and upstream `WI-034`-`WI-036`, which the integration program owns and develops in `$HOME/apps/scopemux-notes`.
- Review must use a stricter confidence threshold than implementation per `invariants/retrieval.md`.
