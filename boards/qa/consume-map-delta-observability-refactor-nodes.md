---
id: "SCOPE-004"
title: "Consume map delta, observability, and refactor nodes in context assembly"
priority: "P3"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "qa"
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

## Implementation (2026-09-26)

Worktree `scope-004`, branch `feature/SCOPE-004-stage-aware-map-slices`, base
`development`. The provider pin stays at `e93df08`; that revision already exposes
the `WI-036` delta and map query surface the slices consume.

Stage table (`crates/opencode-types/src/retrieval.rs`):

- Added `MapRole` (`Locate`/`Slice`/`Project`/`Reconcile`), `RetrievalRepresentation`
  (`Search`/`Delta`/`Anchors`/`Observability`/`Duplicates`/`Impact`/`Review`/`Reconcile`),
  and `stage_retrieval(stage)` implementing the wiki stage table.
- `RetrievalRequest` now carries the stage-derived `role` and `representation`, plus the
  recorded `reopen_reason`; `RetrievalResponse` echoes the representation. New candidate
  kinds `Observability` and `RefactorOpportunity`.

Runtime boundary (`crates/opencode-session/src/retrieval.rs`):

- `build_request` derives role/representation from the task stage instead of hardcoding a
  role; sessions with no task use the `Selected` mapping.
- `apply_inclusion_threshold` is the runtime-owned threshold: `Review` keeps only
  exact/high-confidence evidence, implementation keeps heuristic enrichment. Plan
  reconciliation signals are never filtered.
- `prompt.rs` records `retrieval_representation` in message metadata next to
  `retrieval_provider`/`retrieval_candidates`.

Scopemux provider (`crates/opencode-scopemux/src/lib.rs`):

- Binds the `WI-036` FFI surface (`project_context_compute_delta`,
  `project_context_query_resolve/_duplicates/_observability/_change_impact`,
  `project_map_query_result_free`, `project_delta_result_free`).
- Dispatches per representation: `Search` (tiered search), `Delta`
  (`compute_delta`), `Anchors` (`query_resolve`), `Observability`
  (`query_observability` for seed and projected symbols), `Duplicates`
  (`query_duplicates`, whole parsed project), `Impact` (`query_change_impact`),
  `Review` (delta + duplicates + observability, deduped), `Reconcile` (signals only).
- Plan nodes are materialized before the query so target state is visible; every
  candidate carries provenance, confidence, origin/lifecycle, and token estimate.
- The generic provider stays the default and fallback: it returns seed/changed facts for
  map-less slices and no candidates for map-only slices (`Reconcile`, `Observability`,
  `Duplicates`), so every stage still completes.

Verification:

- `cargo test -p opencode-types -p opencode-retrieval -p opencode-session -p opencode-scopemux`
  (default): new stage-table, threshold, generic-fallback, and boundary tests pass.
  Two pre-existing `opencode-session` `instruction::tests::test_find_up_*` failures are
  unrelated macOS `/tmp` canonicalization issues (untouched code).
- `SCOPEMUX_CORE_DIR=.../third_party/scopemux-core cargo test -p opencode-scopemux
  --features native`: 14/14, including new delta, duplicates, observability, and composed
  review slice tests.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace`: clean.

Still open / follow-ups:

- Full prompt context assembly still consumes retrieval candidates only as recorded
  provenance; injecting the slices into the prompt at a token budget is a later slice.
- The recommended generic-vs-map-aware comparison and the repair-cycle
  `reopen_reason`-to-anchor check have not been run end to end on a live task.
- `WI-034`/`WI-035` parsed observability blocks and richer duplicate signals only become
  available when the pin advances past `e93df08`; at this pin `Observability` resolves
  observability plan nodes and `Duplicates` uses the `WI-036` heuristic clustering.
