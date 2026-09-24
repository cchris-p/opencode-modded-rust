---
id: "SCOPE-002"
title: "Integrate scopemux-core behind the retrieval-provider boundary"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-integration-plan.md"
status: "todo"
created: "2026-09-21"
---

# Integrate scopemux-core behind the retrieval-provider boundary

## Summary

Connect the external `scopemux-core` native engine to this product as one concrete retrieval provider behind the boundary defined in `START-025`, so task context assembly can draw on structural, symbol-aware, budgeted context instead of only direct file reads.

This is Phase 0 of `SCOPE-001` ("Idealized scopemux map integration"). It is the near-term integration step that `START-007` deliberately deferred. It does not change runtime authority over task state, lifecycle, verification, or review, and it does not require the target-state/plan-node map layer described in `wiki/scopemux-map-integration.md`.

## Why this exists

`START-007` planned ScopeMux integration and `START-025` reserves the runtime boundary, but there is still no code path that consumes `scopemux-core` at all. `scopemux-core` (`$HOME/apps/scopemux-core`) already ships the capabilities the plan anticipated:

- multi-file `ProjectContext` with a project-wide symbol table and cross-file reference resolution (`scopemux-core/core/include/scopemux/project_context.h`, `reference_resolver.h`)
- a project IR snapshot: symbols, resolved references, call-graph edges, and include/import dependency edges
- a canonical `InfoBlock` registry across tiers 0-4, with tiered context selection, an indexed search API, and a token-aware prompt assembly API (`project_context.h`)
- a `ContextEngine` with relevance metrics (recency, cursor proximity, semantic similarity, reference count, user focus), compression levels, and token-budget distribution (`context_engine.h`)

Those map directly onto the retrieval responsibilities the plan assigned to a future structural provider. The work now is to wire one of them through the `START-025` boundary with honest limits and a fallback to the generic provider.

## Current understanding (verify before relying on it)

- `scopemux-core` exposes a C API through headers under `scopemux-core/core/include/scopemux/` and builds with CMake (`scopemux-core/CMakeLists.txt`).
- The shipped Python extension registers `ParserContext`, `ASTNode`, `CSTNode`, `ContextEngine`, `InfoBlock`, module functions (`detect_language`, `parse_c_file_to_cst`), and a test processor (`scopemux-core/core/src/bindings/module.c` and siblings); `ProjectContext`, tiered context, search, and prompt assembly are **not** exposed to Python today. Any integration that needs the project IR must use the C API (FFI or a compiled helper), not the Python module. (`FIX-004` corrects the upstream `README.md`, which currently claims only `ParserContext` and `ContextEngine`.)
- Supported grammars today are C, C++, Python, JavaScript, and TypeScript (`scopemux-core/README.md`). **Rust is not a supported grammar**, which matters because this product and typical downstream Rust workspaces cannot be parsed by `scopemux-core` yet.
- `scopemux-core` is explicitly development-oriented and not packaged for standard distribution (`scopemux-core/README.md` "Limitations"), so the integration must pin and build it rather than assume a system install.
- This runtime currently builds context directly inside session and prompt code, which is what `START-025` exists to abstract.

## Scope

- Choose and document one integration mechanism for `scopemux-core`: Rust FFI against the C API, an out-of-process helper binary built from `scopemux-core`, or a Python sidecar (only if its reduced surface is sufficient).
- Make `scopemux-core` buildable/linkable from this repo's build in a pinned, reproducible way, or package the chosen helper path.
- Implement a retrieval provider that satisfies the `START-025` request/response contract and returns ranked candidates with provenance (file, symbol, block id, relationship, score).
- Keep the generic repository-local provider as the default and fallback; the runtime must work when `scopemux-core` is absent or fails.
- Feed retrieval outputs into prompt construction through the boundary rather than embedding `scopemux`-specific logic across orchestration.
- Record how confidence/provenance from `scopemux-core` (match flags, resolution status, tier) is interpreted by the runtime.

## Non-goals

- Making `scopemux-core` a V1 hard dependency or a build requirement for the daily-driver path.
- Porting `scopemux-core` to Rust or rewriting its parsers.
- Adding Rust grammar support to `scopemux-core` as part of this product-side provider card (it is upstream program work; tracked as `WI-030`).
- Target-state/plan nodes, delta views, observability blocks, or duplication analysis (those belong to `SCOPE-001` later phases).
- Replacing direct file reads for small, obvious tasks.
- Giving `scopemux-core` authority over task lifecycle, completion, verification, or review.
- Changing the `START-025` boundary contract beyond what integration proves necessary.

## Open Questions

- Which integration mechanism is correct: FFI to the C API, a compiled helper binary, or a Python sidecar? This determines build complexity, packaging, and which capabilities are reachable.
- Should `scopemux-core` be consumed as a pinned git submodule/commit, or vendored as a built artifact?
- How should the runtime behave for workspaces in languages `scopemux-core` cannot parse (notably Rust): silent fallback to the generic provider, or an explicit "structural retrieval unavailable" signal in provenance?
- Does the product need the full `ProjectContext` IR, or is `ParserContext` + `ContextEngine` (the Python-exposed subset) sufficient for the first provider version?

## Done when

- One integration mechanism is chosen and documented with rationale and its limits.
- `scopemux-core` is pinned and reproducible in this repo's build, or the chosen helper is packaged and runnable.
- A retrieval provider behind the `START-025` boundary can call `scopemux-core` and return ranked candidates with provenance.
- The runtime still assembles context and completes tasks with the generic provider when `scopemux-core` is unavailable or unsupported for the workspace language.
- No `scopemux`-specific logic is embedded outside the retrieval provider and boundary.
- Verification compares generic retrieval and `scopemux` retrieval on at least one real multi-language workspace and records the outcome.

## Recommended verification

- Build `scopemux-core` per `scopemux-core/scripts/build_all_and_pybind.sh` and confirm the C artifacts and headers available to link are what the integration assumes.
- Confirm the boundary call path by tracing one session where the provider returns candidates and prompt construction consumes them.
- Run the same task with `scopemux-core` present and absent and confirm identical task lifecycle behavior, only differing context quality.
- Check a Rust workspace (unsupported grammar) and confirm the fallback behavior matches the chosen policy.
- Confirm the pinned version is recorded and reproducible from a clean checkout.

## Related Items

- `SCOPE-001` Idealized scopemux map integration - the vision this card is the first phase of.
- `START-025` Add retrieval-provider boundary for task context assembly - prerequisite; this card plugs into that boundary.
- `START-007` Plan ScopeMux integration - defined the deferred contract and responsibilities this card now implements.
- `PHASE-003` V2 reliability - parent phase where early ScopeMux integration was scheduled.
- `START-016` Define structured task state for V1 - source of task/stage intent the retrieval request is built from.
- `PHASE-001` / `START-005` - V1 runtime loop that must remain generic and ScopeMux-free.
- Upstream work in `$HOME/apps/scopemux-notes` (in-scope for ScopeMux integration, not an external prerequisite): `FIX-001` (C++ resolver registration and declaration), `FIX-003` (Python interpreter range), `FIX-004` (Python API surface docs). These make the C API and build contract reliable before integration; the full core conflict set is `FIX-001`-`FIX-006` composed by `H-001`.

## Notes

- Depends on `START-025`; do not start implementation before the boundary exists.
- Integration scope: the ScopeMux integration program includes the `scopemux-core`/`scopemux-notes` development it depends on (`invariants/integration-scope.md`); upstream `FIX-*`/`WI-*` items are in-scope program deliverables, not external prerequisites.
- Keep the integration one provider among peers, not a special case threaded through the runtime.
- Treat `scopemux-core` maturity gaps (no Rust grammar, dev-oriented build, C-only project API) as first-class design inputs, not as footnotes.
- The wiki companions are `wiki/scopemux-integration-plan.md` (boundary and guardrails) and `wiki/scopemux-map-integration.md` (idealized target).
