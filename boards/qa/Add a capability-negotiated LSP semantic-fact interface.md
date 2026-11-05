---
id: "LSP-002"
title: "Add a capability-negotiated LSP semantic-fact interface"
priority: "P1"
type: "feature"
area: "LSP"
spec: "wiki/scopemux-lsp-boundary.md"
status: "qa"
created: "2026-11-05"
updated: "2026-11-05"
---

# Add a capability-negotiated LSP semantic-fact interface

## Summary

Add a narrow semantic-fact interface over `opencode-lsp` so other capabilities (notably
ScopeMux) can ask for exact definitions, references, type definitions, implementations,
inheritance, and call hierarchy without embedding `lsp_types` or LSP request logic.

The interface is capability-negotiated per server, cache-aware, and optional.

## Why this exists

The LSP client exposes many operations, but consumers should not call it ad hoc. ScopeMux
needs a small, stable fact contract with exactness and provenance, and servers differ in
which methods they support. Without this boundary, ScopeMux would either duplicate LSP
logic or depend directly on LSP types and availability.

## Scope

- Define a small semantic-fact request/response surface:
  - definition,
  - references,
  - type definition,
  - implementation,
  - inheritance/supertypes where the server supports it,
  - call hierarchy.
- Negotiate capabilities from the server's `ServerCapabilities`; never call an
  unsupported method.
- Return facts with `confidence = exact`, source server, and document version/file hash.
- Cache facts and invalidate them on document version or file hash change.
- Make unavailability explicit: no server, unsupported capability, timeout, or failure.
- Keep LSP-specific types out of core consumers; expose owned neutral structs.
- Support an in-process implementation first; do not add a new transport protocol.

## Acceptance criteria

- Consumers can request facts without importing `lsp_types`.
- Unsupported server capabilities return an explicit unsupported result, not an error that
  fails the caller.
- Cached facts are invalidated on document version/file hash change.
- The interface works with a fake LSP server in tests and against at least one real server.
- No server or a broken server degrades cleanly to "unavailable".
- The interface does not own task state, context budgets, or inclusion decisions.
- LSP request latency is bounded and does not block the session loop.

## Non-goals

- Project-wide eager fact indexing.
- Type checking or type inference.
- Making any consumer require LSP.
- Defining the map's type-edge schema (that belongs to `SCOPE-013`).
- Changing LSP tool behavior; this is an internal consumer contract.

## Dev Notes

- Depends on `LSP-001`; do not build a second registry or startup path.
- Keep the fact surface query-shaped (anchor in, facts out), not a bulk parser API.
- Cache key must include server id, workspace root, file version/hash, and capability.
- Use the same timeout/health rules as the LSP tool.

## QA Verification

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/150 (merged into `development`).
- `opencode_lsp::facts` exposes neutral request/response types; consumers do not import
  `lsp_types`.
- Capability negotiation: unsupported methods return `Unavailable { reason: Unsupported }`
  and are never called.
- Cache is keyed by `(server, path, anchor, kind, content hash)`; changing the file misses
  the cache and prunes older entries for that path.
- Request timeout is bounded (default 10s, configurable); a hung server returns `Timeout`.
  This also closes LSP-001's slow-server gate.
- `cargo test -p opencode-lsp`: 17/17, including fake-server exact/cache/invalidation,
  unsupported capability, and hung-server timeout.
- Real server: `cargo test -p opencode-lsp real_rust_analyzer_definition_smoke -- --ignored`
  passed against installed `rust-analyzer`.

## Related Items

- `LSP-001` Standardize LSP runtime wiring and configuration.
- `SCOPE-012` Consolidate ScopeMux and LSP responsibilities.
- `SCOPE-013` Ingest LSP-derived exact type facts into the ScopeMux map.
- `SCOPE-011` Out-of-process WIP transport (does not apply to this in-process LSP fact
  interface).
- `wiki/scopemux-lsp-boundary.md`
