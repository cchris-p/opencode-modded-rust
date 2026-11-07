---
id: "SCOPE-015"
title: "Persistent workspace map foundation"
priority: "P1"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-integration-plan.md"
status: "todo"
created: "2026-11-06"
updated: "2026-11-06"
predecessor: "SCOPE-014"
---

# Persistent workspace map foundation

## Summary

Build the persistent, per-workspace ScopeMux map that `SCOPE-006` and `SCOPE-009`
require. Today every retrieval creates a fresh `ProjectContext`, registers only the
request's seeds, and drops it, so there is no always-on registry and no freshness
measurement.

This is the tracking epic for the prerequisite program. The implementation order is
`SCOPE-018` -> `SCOPE-016` -> `SCOPE-017` -> `SCOPE-019`, followed by resuming `H-018`
at `SCOPE-006` and `SCOPE-009`.

## Why this exists

`SCOPE-006` was verified blocked (2026-11-06): a map-backed code-search tool cannot answer
"where is X" without a filesystem walk because the product provider has no persistent
whole-workspace node registry and no watcher. `SCOPE-009` cannot measure freshness for the
same reason. Building either on the current stateless provider would violate their
acceptance criteria.

The architecture decision is locked here so implementation does not re-litigate it.

## Locked architecture

| Concern | Owner | Must not own |
|---|---|---|
| Raw FFI, safe context wrapper, incremental parse/rebuild | `opencode-scopemux` engine layer | provider selection, prompt policy, tool formatting |
| Per-workspace map lifecycle, eviction, watcher subscription, dirty-set scheduling | one map supervisor/registry | task state, inclusion thresholds, model activation |
| File events, ignore matching, debounce | `opencode-watcher` | parsing, freshness policy, retrieval decisions |
| Query slices (search/delta/anchors/impact/review) | provider adapter over the supervisor | creating contexts per call, watching files |
| Freshness measurement (last revision, lag, dirty size, re-anchor result) | engine/supervisor -> shared value type | deciding "stale means fall back" |
| Freshness policy (use/warn/generic fallback) | `opencode-session` / tool facade | engine or watcher |
| Model/provider activation | one shared predicate; runtime injects capability | duplicated in tool and provider |
| Prompt budget, inclusion threshold, role filtering | `opencode-session` | map engine |
| Tool surface and output formatting | `opencode-tool` | FFI, model string parsing, ignore policy |
| Evidence/provenance types | `opencode-types` | per-consumer duplicate result structs |

Dependency direction:

```
opencode-types          (shared evidence, freshness, fallback enums)
      ^
opencode-scopemux       (ffi -> engine -> supervisor -> provider adapter)
      ^                         ^
opencode-session        (activation, inclusion, prompt policy, wiring)
      ^
opencode-tool           (tool surface; backend injected via ToolContext)

opencode-watcher        (generic file events only)  -> supervisor subscribes
```

## Shared rules (DRY)

- One ignore/exclusion policy consumed by watcher, grep, and indexing.
- One language/prose predicate shared with the `SCOPE-010` matrix.
- One activation predicate; the runtime injects the capability into the tool.
- One evidence type carrying file, symbol, block id, confidence, provenance.
- One freshness vocabulary and one fallback-reason enum.
- One path-normalization helper and one token budget/truncation utility.

DRY applies to decisions and vocabulary, not to forcing grep and map search to share an
algorithm. Their mechanisms stay separate; their policy and evidence shape do not.

## Program order

| # | Card | Work |
|---|---|---|
| 1 | `SCOPE-018` | Shared contracts and policy: block id, freshness, ignore/language, activation, capability trait |
| 2 | `SCOPE-016` | Safe engine layer and persistent context lifecycle |
| 3 | `SCOPE-017` | Map supervisor plus watcher-driven incremental updates |
| 4 | `SCOPE-019` | Wire into retrieval provider and tool search capability |
| 5 | `SCOPE-006` | Resume the map-backed code-search tool (unblocked by `SCOPE-019`) |
| 6 | `SCOPE-009` | Resume freshness/staleness SLOs (unblocked by `SCOPE-017`/`019`) |

## Acceptance criteria

- A persistent map exists per workspace and survives across retrieval calls.
- Edits update nodes through incremental rebuild; a full rebuild is not required per call.
- Freshness (last revision, dirty set, watcher lag, re-anchor result) is measured.
- The provider and tool query the map without creating a context per call.
- The generic provider remains the default/fallback and task authority is unchanged.
- `SCOPE-006` and `SCOPE-009` can be implemented without violating their acceptance criteria.

## Non-goals

- No `FUTURE-PROOFING-*` or `DECISION-*` work.
- No `SCOPE-007`, `SCOPE-008`, or `SCOPE-011`.
- No capability profiles, artifact packaging, out-of-process transport, or data flywheel.
- No new language grammars.
- No multi-workspace distributed map or cross-process cache.
- No task stage/verification/review/completion authority in the map.

## Related Items

- `SCOPE-006` map-backed code search with grep fallback (blocked, unblocked by this program).
- `SCOPE-009` map freshness and staleness SLOs (blocked on persistence).
- `SCOPE-001` idealized map integration and incremental reconciliation design.
- `SCOPE-002` retrieval-provider boundary.
- `SCOPE-010` language coverage matrix.
- `H-018` remaining ScopeMux program (paused at `SCOPE-006`).
- `H-020` persistent workspace map program handoff.
