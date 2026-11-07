---
id: "SCOPE-009"
title: "Define map freshness and staleness SLOs"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
updated: "2026-11-06"
predecessor: "SCOPE-017"
attention: "Deferred until SCOPE-017 (supervisor/watcher) and SCOPE-019 (integration) provide real freshness measurement; see H-020."
---

# Define map freshness and staleness SLOs

## Summary

Define how fresh the workspace map must be, how staleness is detected and surfaced, and
what the runtime does when the map is stale.

A correct stale map is worse than no map: it can anchor a task to code that no longer
exists. Freshness is part of the capability contract, not an implementation detail.

## Why this exists

`SCOPE-001` requires incremental reconciliation and a durable map store. `SCOPE-004`
consumes delta and anchors. Neither defines an SLO or a runtime response to staleness.
Without one, the map silently degrades.

## Scope

- Define freshness states: fresh, lagging, stale, unknown.
- Define measurement: last parsed revision, content hashes, watcher lag, dirty set size,
  re-anchor result.
- Define SLOs per operation:
  - locate/search after an edit,
  - delta after a change,
  - plan-node reconciliation,
  - review slice.
- Surface staleness in provenance and the retrieval response.
- Define runtime behavior:
  - fresh: use the map,
  - lagging: use with a warning,
  - stale: fall back to generic,
  - unknown: generic.
- Add a stale/conflict signal for plan nodes.
- Add tests for edit -> reparse -> re-anchor latency and stale fallback.
- Record freshness metrics in the retrieval data.

## Acceptance criteria

- Freshness states and SLOs are documented.
- A retrieval response carries a freshness state.
- A stale map falls back to generic retrieval with a clear reason.
- Plan nodes whose anchors change are marked stale/conflict, never silently dropped.
- Freshness is measured on a real workspace.
- The SLO is recorded and reviewed.

## Non-goals

- A perfect real-time index.
- Parsing every language.
- Blocking the task while the map rebuilds.
- Replacing the generic fallback.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-017`
- `SCOPE-001` idealized map integration
- `SCOPE-003` plan-node reconciliation
- `SCOPE-004` stage-aware slices
- `wiki/workspace-intelligence.md`
- `wiki/scopemux-map-integration.md`

## Program dependency (2026-11-06)

- Freshness measurement requires the persistent map from `SCOPE-017` (supervisor/watcher)
  and the provider/tool integration in `SCOPE-019`; see `H-020`.
- `SCOPE-018` supplies the shared `MapFreshness` vocabulary and provenance fields.
- Do not implement measurement against the current per-call provider: it has no persistent
  revision, dirty set, watcher lag, or re-anchor result to measure.
- Resume after `SCOPE-019`; this card is the follow-on to `SCOPE-006` in the `H-018` tail.
