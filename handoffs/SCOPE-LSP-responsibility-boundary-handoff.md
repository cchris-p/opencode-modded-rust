---
id: "H-016"
title: "SCOPE/LSP responsibility boundary and semantic-fact enrichment - Handoff"
status: "complete"
created: "2026-11-05"
updated: "2026-11-05"
owner: ""
target: "opencode-modded-rust; scopemux-core only if exact-fact ingestion needs a schema change"
blocked_reason: ""
needs_human: ""
items: ["LSP-001", "LSP-002", "SCOPE-012", "SCOPE-013", "SCOPE-014"]
---

# SCOPE/LSP Responsibility Boundary And Semantic-Fact Enrichment - Handoff

## Included Board Items

| Item | Title | Role |
|---|---|---|
| `LSP-001` | Standardize LSP runtime wiring and configuration | Make LSP real and config-driven |
| `LSP-002` | Add a capability-negotiated LSP semantic-fact interface | Stable fact boundary over the LSP client |
| `SCOPE-012` | Consolidate ScopeMux and LSP responsibilities and remove duplicate point-query paths | Ownership guard and scope discipline |
| `SCOPE-013` | Ingest LSP-derived exact type facts into the ScopeMux map | Optional exact-edge enrichment |
| `SCOPE-014` | Measure the context value of LSP-derived semantic facts | Keep/expand/retire evidence gate |

## Why This Composition

These items implement one decision documented in `wiki/scopemux-lsp-boundary.md`: **LSP is
the exact semantic oracle; ScopeMux is the context producer.** They are tightly coupled
because the type-fact enrichment is only safe after the ownership boundary exists and only
useful after LSP is actually wired.

The set splits into two reviewable units:

1. **Capability enablement** — `LSP-001` and `LSP-002` make LSP usable and expose a narrow
   fact interface.
2. **Scope discipline and enrichment** — `SCOPE-012` enforces one owner per query class;
   `SCOPE-013` consumes optional facts; `SCOPE-014` decides whether to keep them.

## Decisions Locked

- ScopeMux parsing stays. LSP is not a parser API and cannot supply the CST, InfoBlocks,
  token estimates, or incremental map reconciliation.
- The redundancy to remove is semantic resolution, not parsing. Do not build a compiler
  frontend inside ScopeMux.
- LSP owns point semantic queries and diagnostics. ScopeMux owns locate/search/slice,
  current/target map, delta, observability, duplication, and reconciliation.
- Type support means optional LSP-derived exact facts in the map, not a type checker or
  inference engine.
- LSP-derived facts are `confidence = exact`, optional, cached, and never required. The
  map must still work with no server.
- The runtime keeps authority over stage, verification, review, completion, and inclusion.

## Current State At Creation

- `crates/opencode-lsp/src/lib.rs` implements the client and registry.
- `crates/opencode-tool/src/lsp_tool.rs` exposes the tool, but the `lsp` feature is not
  default and emits placeholder output without it.
- `write.rs`, `edit/tool.rs`, and `apply_patch.rs` collect diagnostics only when a registry
  is attached; no runtime path constructs one outside tests.
- The server `/lsp` endpoint is a stub returning an empty list
  (`crates/opencode-server/src/routes.rs:8160`).
- ScopeMux already models reference classes including type, inheritance, implementation,
  and template references, with heuristic resolution and known Rust gaps.

## Dependency Order

1. **`LSP-001`** — no new item depends on a broken runtime.
2. **`SCOPE-012`** — can start in parallel with `LSP-001`; it is docs and guard work and
   does not need the runtime. It must land before `SCOPE-013`.
3. **`LSP-002`** — depends on `LSP-001`; uses the real registry and config.
4. **`SCOPE-013`** — depends on `LSP-002` for the fact interface and `SCOPE-012` for the
   ownership guard.
5. **`SCOPE-014`** — depends on `SCOPE-013`; it evaluates the implemented enrichment.

Parallel-safe: `LSP-001` and `SCOPE-012` touch different areas and may run concurrently.
Everything after them is sequential because each layer builds on the previous contract.

## PR Plan

Use one PR per item, targeting `development`:

| Pass | Item | PR scope | Base gate |
|---|---|---|---|
| 1 | `LSP-001` | LSP config, registry lifecycle, tool/session attachment, status, bounded diagnostics | Clean baseline |
| 1 (parallel) | `SCOPE-012` | Ownership docs + architecture guard; no code-path behavior change | Clean baseline |
| 2 | `LSP-002` | Semantic-fact interface, capability negotiation, cache/versioning, tests | `LSP-001` merged |
| 3 | `SCOPE-013` | LSP fact ingestion into map, anchor bounds, provenance, fallback | `LSP-002` + `SCOPE-012` merged |
| 4 | `SCOPE-014` | Evaluation harness, measured results, keep/expand/retire decision | `SCOPE-013` merged |

If a pass discovers that the pinned `scopemux-core` schema must change, treat that as an
in-scope upstream deliverable per `invariants/integration-scope.md` and update the pin and
FFI bindings in lockstep.

## Implementation Steps

### Pass 1 — `LSP-001` Standardize LSP runtime

- Adopt vanilla's `lsp` config shape and activation semantics.
- Implement lazy per-`(root, server)` spawn, extension gating, root detection,
  broken-server suppression, shutdown, and health.
- Attach the registry to `ToolContext` and `SessionPrompt`.
- Make the LSP build path default so released binaries do not emit placeholders.
- Replace the `/lsp` stub with real connected-server status; mirror it in the TUI.
- Keep post-edit diagnostics bounded (current file plus capped project files) and
  best-effort.
- Keep LSP opt-in per workspace; no task may require it.

**Gate:** a configured server starts for a matching file; an absent/broken server does not
fail the task; the tool returns real results in a default build.

### Pass 1 (parallel) — `SCOPE-012` Consolidate responsibilities

- Document the ownership matrix and precedence rule in the WIP docs.
- Audit the retrieval provider, tool surface, and map-backed tools for duplicate
  point-query or diagnostic behavior.
- Keep map-backed tooling to locate/search/slice/delta/review queries.
- Add a cheap architecture guard or test that fails if a map-backed path reimplements an
  LSP point query.
- Preserve the heuristic resolver as fallback, not as a compiler-parity project.

**Gate:** docs and guard are in place; no behavior regressions; map works without LSP.

### Pass 2 — `LSP-002` Semantic-fact interface

- Define neutral request/response structs for definition, references, type definition,
  implementation, inheritance, and call hierarchy.
- Negotiate against `ServerCapabilities`; never call unsupported methods.
- Return exact facts with server provenance and document version/file hash.
- Cache and invalidate on version/hash change.
- Surface unavailability explicitly (no server, unsupported, timeout, failure).
- Do not leak `lsp_types` into core consumers.

**Gate:** the interface works against a fake server and at least one real server; missing
capabilities degrade cleanly; no consumer is forced to require LSP.

### Pass 3 — `SCOPE-013` Ingest LSP-derived type facts

- Consume facts at map anchors only; do not query every symbol.
- Map exact facts into existing reference classes where possible.
- Label exact edges with provenance; keep heuristic edges when unavailable.
- Cache/invalidate by document version/file hash.
- Bound LSP calls per retrieval request and record the fallback reason.
- Keep the task result identical when LSP is absent.
- Do not add a type checker, inference, or a new language frontend.

**Gate:** exact and heuristic edges are distinguishable; no-server runs complete; LSP calls
are bounded; native/default builds compile.

### Pass 4 — `SCOPE-014` Measure the value

- Compare heuristic ScopeMux only, ScopeMux + exact LSP facts, and generic retrieval on a
  held-out task set.
- Record tokens, precision/recall, task success, review quality, p50/p95 latency, fallback
  rate, cache hit rate, and cost per successful task.
- Prove the no-server path still completes.
- Record keep/expand/retire with a retirement criterion.

**Gate:** evidence is recorded on the card; a null or negative result is valid; no
enrichment is kept without measured benefit.

## Verification

- `cargo fmt`, `cargo check`, `cargo clippy --workspace --all-targets`.
- Default `cargo test`.
- LSP tests with a fake server plus a real-server smoke test where available.
- `cargo test -p opencode-scopemux --features native` if the map path changes.
- Manual checks: LSP off, LSP configured with a working server, LSP configured with a
  broken server, and an unsupported-language workspace.
- Evaluation evidence for `SCOPE-014` is attached to the card, not only described in chat.

## Risks And Constraints

- **Two parsers run.** ScopeMux and the LSP server both parse the file. Keep LSP lazy and
  anchor-driven; do not attempt whole-repo eager fact indexing.
- **Server variance.** Not every server supports every method; the interface must
  capability-negotiate.
- **Latency.** LSP must not block the session loop; bound timeouts and fail open.
- **Scope creep.** Do not let `SCOPE-013` become a type system, and do not let `LSP-002`
  become a second retrieval provider.
- **FFI drift.** If exact-fact ingestion changes the pinned `scopemux-core` schema, update
  bindings and pin together.
- **Optionality.** Every new path must preserve the no-LSP and no-ScopeMux fallbacks.

## Rollback / Decommission

- `LSP-001`/`LSP-002` can be disabled by config; disabling LSP restores today's behavior.
- `SCOPE-013` must be removable without changing the core retrieval loop; removing exact
  facts leaves heuristic behavior.
- `SCOPE-014` may record "retire" and the ingestion path can be removed without touching
  task authority or the parser.

## Exit Criteria

- LSP is configurable, actually starts, surfaces status, and feeds bounded diagnostics.
- A semantic-fact interface exists and degrades cleanly when a server or capability is
  absent.
- The ScopeMux/LSP ownership boundary is documented and guarded; no duplicate point-query
  or diagnostic path remains.
- The map can optionally consume exact LSP-derived type facts without requiring LSP.
- The enrichment has measured evidence and a recorded keep/expand/retire decision.
- No new task-state, lifecycle, verification, or review authority moves into either
  capability.

## Completion (2026-11-05)

All covered PRs are merged into `development`:

| Item | PR | Outcome |
|---|---|---|
| `LSP-001` | #149 | Config-driven lazy LSP registry, default `lsp` feature, real `/lsp` + TUI status, bounded request timeout |
| `SCOPE-012` | #148 | Ownership matrix + docs, architecture guard, tool-surface guard |
| `LSP-002` | #150 | Neutral capability-negotiated semantic-fact interface with cache and explicit unavailability |
| `SCOPE-013` | #152 | Optional exact-fact ingestion at bounded map anchors; no C schema change |
| `SCOPE-014` | #154 | Three-arm evaluation; **retired default activation**, opt-in via `OPENCODE_SCOPEMUX_EXACT_FACTS=1` |

Cards are in `qa` with evidence recorded. No card is `done`; a QA pass or explicit owner
direction is still required.

Key decisions:
- LSP stays optional and config-gated; no task requires it.
- ScopeMux never depends on `opencode-lsp` at runtime; the neutral contract lives in
  `opencode-retrieval`.
- Exact-fact enrichment is retired from default behavior after measuring no task-success
  gain and a ~70% p95 latency increase. The interface and adapter remain for a larger
  evaluation. See `docs/research/SCOPE-014-exact-facts-evaluation.md`.

Known pre-existing issue (not introduced here; reproduced on base `development`): the pinned
native `scopemux-core` artifact fails `native_delta_slice_returns_projected_entries` and
segfaults in `native_provider_reports_plan_reconciliation_evidence`. The rest of the native
suite passes. The `opencode-tui` test `tab_autocomplete_uses_first_candidate` also fails on
base. Investigate these before relying on a fully green `cargo test --workspace`.

Next-agent notes:
- Re-run QA with `scopemux-self-qa` and move cards to `done` only with recorded QA.
- If expanding exact facts, start from the evaluation doc's "what would justify expanding"
  and use a >=20-task held-out set before changing the default.
- `LSP-001` manual `/lsp`/TUI checks still need a human or QA session with a configured
  server.

## Next Agent Instructions

1. Start with `LSP-001` and `SCOPE-012` in parallel worktrees if practical.
2. Do not start `LSP-002` until `LSP-001` is merged; do not start `SCOPE-013` until
   `LSP-002` and `SCOPE-012` are merged.
3. Read `wiki/scopemux-lsp-boundary.md`, `invariants/workspace-intelligence.md`, and
   `invariants/retrieval.md` before changing the boundary.
4. Read each card in full before implementation; the cards carry the acceptance criteria.
5. Keep PRs scoped to one card. If a card needs a schema change, update the pin, FFI, and
   tests in the same PR.
6. Move each card to `qa` on merge; `SCOPE-014` is the evidence gate before any card is
   treated as settled.
