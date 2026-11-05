---
id: "SCOPE-012"
title: "Consolidate ScopeMux and LSP responsibilities and remove duplicate point-query paths"
priority: "P1"
type: "refactor"
area: "SCOPE"
spec: "wiki/scopemux-lsp-boundary.md"
status: "doing"
created: "2026-11-05"
---

# Consolidate ScopeMux and LSP responsibilities and remove duplicate point-query paths

## Summary

Codify and enforce the ScopeMux/LSP ownership boundary: LSP owns exact point semantics and
diagnostics; ScopeMux owns syntax, the map, and token-budgeted context. Remove any
duplicate point-query or diagnostic responsibility from ScopeMux and prevent it from being
reintroduced.

## Why this exists

The two capabilities overlap on definitions, references, symbols, implementations, type
definitions, and call hierarchy. Without an enforced boundary, ScopeMux will keep growing a
compiler-resolution responsibility it cannot match, and the product will expose two
competing ways to answer the same question.

## Scope

- Record the ownership matrix and precedence rule in the WIP docs.
- Audit the retrieval provider, tool surface, and any map-backed tools for duplicate
  point-query or diagnostic behavior.
- Keep map-backed tooling limited to locate/search/slice/delta/review-style queries.
- Ensure LSP is the only owner of go-to-definition, find-references, hover, document
  symbols, implementations, type definitions, call hierarchy, and diagnostics.
- Add an architecture guard or test that fails if a map-backed path reimplements an LSP
  point query.
- Update `wiki/workspace-intelligence.md`, `wiki/scopemux-integration-plan.md`, and
  `wiki/scopemux-map-integration.md` to point at the boundary.
- Preserve the heuristic resolver as the fallback, not as a compiler-parity project.

## Acceptance criteria

- The ownership matrix and precedence rule are documented and referenced from the WIP
  program docs.
- No ScopeMux-backed tool claims a point semantic query that LSP owns.
- No ScopeMux path synthesizes diagnostics.
- The map still works with LSP absent, using heuristic edges.
- An architecture guard/test prevents a future map-backed duplicate of an LSP point query.
- WIP non-goals explicitly state that ScopeMux is not a second LSP client and does not add
  a type system.
- Runtime authority is unchanged.

## Non-goals

- Implementing the LSP semantic-fact interface (`LSP-002`).
- Ingesting type facts into the map (`SCOPE-013`).
- Removing ScopeMux's parser or heuristic resolver.
- Deleting the existing LSP tool or debug commands.

## Dev Notes

- This card is deliberately about scope discipline, not new retrieval features.
- The enforcement point should be cheap: a unit/architecture test plus a tool-surface
  assertion is enough.
- Do not weaken the existing fallback guarantees to satisfy the guard.

## Related Items

- `LSP-001` Standardize LSP runtime wiring and configuration.
- `LSP-002` Add a capability-negotiated LSP semantic-fact interface.
- `SCOPE-013` Ingest LSP-derived exact type facts into the ScopeMux map.
- `FUTURE-PROOFING-013` Workspace Intelligence Protocol program.
- `invariants/workspace-intelligence.md`
- `invariants/retrieval.md`
- `wiki/scopemux-lsp-boundary.md`
