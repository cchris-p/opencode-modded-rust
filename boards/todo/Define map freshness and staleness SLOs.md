---
id: "SCOPE-009"
title: "Define map freshness and staleness SLOs"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
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
