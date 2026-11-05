---
id: "SCOPE-013"
title: "Ingest LSP-derived exact type facts into the ScopeMux map"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-lsp-boundary.md"
status: "todo"
created: "2026-11-05"
---

# Ingest LSP-derived exact type facts into the ScopeMux map

## Summary

Let the ScopeMux map consume optional, exact semantic facts from LSP — type definitions,
type references, implementations, inheritance, and relevant call-hierarchy edges — and
label them as exact with provenance, without adding a type system or requiring LSP.

## Why this exists

ScopeMux's cross-file resolution is heuristic and has known gaps (Rust trait-object
dispatch, macro expansion, generic binding). LSP has exact answers where a server exists.
Ingesting those facts at selected anchors improves candidate precision, impact analysis,
review context, and duplicate detection while keeping the map's offline and cross-language
fallbacks.

## Scope

- Consume `LSP-002` facts at map anchors; do not issue LSP queries for every symbol.
- Map exact facts into the existing reference classes where possible (type reference,
  inheritance, implementation, call); extend the schema only if a fact cannot be
  represented.
- Label LSP-derived edges `confidence = exact` with server/provenance metadata.
- Cache and invalidate by document version/file hash.
- Keep heuristic edges when LSP is unavailable, unsupported, or slow.
- Bound the number of LSP calls per retrieval request and record the fallback reason.
- Record provenance for evaluation and diagnosis.

## Acceptance criteria

- A map query can consume type definition/reference/implementation facts for a supported
  language and distinguish them from heuristic edges.
- The map returns the same task result when LSP is absent, with heuristic confidence where
  applicable.
- Unsupported servers and timeouts degrade cleanly; no task fails because LSP is missing.
- LSP calls are bounded and do not turn a retrieval request into a whole-repo query storm.
- Exact facts are invalidated when the file changes.
- No type checker, inference engine, or new language frontend is added.
- Native and default builds both compile; LSP remains optional.
- Provenance is sufficient for `SCOPE-014` to compare arms.

## Non-goals

- Project-wide eager typed indexing.
- Type checking or inference.
- Making LSP a hard dependency of ScopeMux or V1.
- Replacing the parser or InfoBlock representation.
- Using type facts to override explicit task scope.

## Dev Notes

- Depends on `LSP-002` for the fact interface and `SCOPE-012` for the ownership guard.
- Prefer a small set of anchor-driven queries: definition, implementation, and references
  for the nodes the map already selected.
- Do not change the runtime's inclusion thresholds; this card only improves evidence.
- Watch FFI/schema compatibility with the pinned `scopemux-core` revision; advance the pin
  in lockstep if the map schema changes.

## Related Items

- `LSP-001` Standardize LSP runtime wiring and configuration.
- `LSP-002` Add a capability-negotiated LSP semantic-fact interface.
- `SCOPE-012` Consolidate ScopeMux and LSP responsibilities.
- `SCOPE-014` Measure the context value of LSP-derived semantic facts.
- `SCOPE-010` Language coverage and fallback matrix.
- `wiki/scopemux-lsp-boundary.md`
- `invariants/retrieval.md`
