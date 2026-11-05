# ScopeMux and LSP: Responsibility Boundary

> **Status:** decision note (2026-11-05).
>
> This document records the comparison and decisions from the ScopeMux-vs-LSP design
> discussion. It refines the composition rules in `wiki/workspace-intelligence.md` and
> `wiki/scopemux-integration-plan.md`; it does not replace the Workspace Intelligence
> Protocol (WIP) contract. The binding truths remain in
> `invariants/workspace-intelligence.md`.

## Purpose

Define how the product's two code-intelligence capabilities divide responsibility:

- **`opencode-lsp`** — the Language Server Protocol client.
- **ScopeMux / WIP** — the workspace map and context producer.

The goal is to stop ScopeMux from growing into a compiler frontend, stop LSP from being
asked to do context management, and keep one owner per query class.

## One-line model

**LSP is a semantic oracle; ScopeMux is a context producer.** LSP answers exact language
questions at a position. ScopeMux answers which code matters for a task, how much of it to
include, and what the task is trying to change.

## What each capability does

| | `opencode-lsp` (vanilla semantics) | ScopeMux / WIP |
|---|---|---|
| Problem solved | "I need an exact language fact at this position." | "I need the smallest faithful context for this task and stage." |
| Unit | Document + position/query | Project, task, stage, token budget |
| Scope | Per-language, document-centric | Cross-language, project- and task-centric |
| State | Mostly stateless requests + pushed diagnostics | Persistent map with current and target state |
| Source of truth | Compiler-grade language server (`rust-analyzer`, `clangd`, `pyright`, …) | Tree-sitter CST + heuristic IR (`scopemux-core`) |
| Output | Definitions, references, hover/types, symbols, implementations, type definitions, call hierarchy, rename edits, diagnostics | Ranked candidates, InfoBlocks, token-budgeted slices, current→target delta, observability, duplication/refactor signals, provenance + confidence |
| Cost | Server startup, memory, version drift, per-language availability | Index build/freshness, heuristic error, budget policy |
| Authority | Facts consumed by tools/model; diagnostics feed the edit loop | Evidence only; the runtime disposes |

### Why "both parse ASTs" is not a shared interface

Both sides parse source, but LSP exposes *query results*, not a syntax tree. The protocol
does not provide:

- a project-wide CST/AST,
- arbitrary subtree ranges for slicing,
- InfoBlocks or token estimates,
- a map of current vs target state,
- incremental reconciliation semantics.

ScopeMux needs all of those. LSP needs none of them. Parsing is a shared substrate, not a
replaceable layer.

## Can LSP replace ScopeMux parsing?

**No.** Replacing parsing with LSP would require redesigning ScopeMux into an
editor-query layer and would lose the map, budgeted slices, cross-language uniformity, and
offline fallback. The current provider makes the dependency explicit: every retrieval run
does `project_parse_all_files` → `project_resolve_references` →
`project_context_rebuild_info_blocks` (`crates/opencode-scopemux/src/lib.rs:433-444`).

LSP cannot supply the first or third step. It also cannot guarantee coverage of every
supported language, and it requires a running server per language.

## What is actually redundant

The redundancy is in **semantic resolution**, not parsing. `scopemux-core` already models
reference classes including `REF_TYPE`, `REF_INHERITANCE`, `REF_IMPLEMENTATION`,
`REF_TEMPLATE`, and language-specific resolvers
(`core/include/scopemux/reference_resolver.h:28-46`), but its resolution is heuristic and
has known Rust gaps: trait-object dispatch, macro expansion, and generic binding
(`wiki/scopemux-map-integration.md:168`). That is exactly where LSP is stronger.

### Decision: remove these redundancies

1. **Point queries.** Do not build `goToDefinition`, `findReferences`, `hover`, or
   `documentSymbol` as ScopeMux-backed tools. LSP owns point semantic queries. ScopeMux
   owns ranked locate/search/slice.
2. **Full per-language semantic resolvers.** Do not chase compiler parity inside
   ScopeMux. Keep its resolver as the heuristic fallback; delegate exact edges to LSP when
   a server is available.
3. **Diagnostics.** No ScopeMux duplicate. LSP owns post-edit diagnostics; the map must
   not synthesize them.
4. **Competing symbol surfaces.** Keep one model-facing story: LSP for precise point
   lookup, ScopeMux for ranked candidate sets. Do not expose two "find symbol" paths.

### What remains duplicated, and why it is acceptable

Both ScopeMux and the LSP server parse the same file. That duplication is accepted because
they produce different artifacts: a map and slices versus exact semantic answers. It is
acceptable only if LSP is started lazily, for anchors, and only when configured. It becomes
a problem if the product tries to build a whole-repo typed graph by issuing LSP queries per
symbol.

## LSP-derived type facts

Type support does **not** mean ScopeMux grows a type checker. It means the map may ingest
optional, exact type facts from LSP and label them as such.

- **Do not** add type checking, inference, or a compiler frontend to ScopeMux.
- **Do** allow LSP-derived type definitions, type references, implementations, inheritance,
  and call hierarchy to enter the map as exact, provenance-labeled edges. The graph schema
  already has the relevant reference classes; the gap is exactness, not representation.

### Why this helps context management

- Disambiguates same-name symbols and overloaded methods.
- Makes interface→implementation and trait-impl impact analysis accurate.
- Improves delta and review context when a change crosses type boundaries.
- Reduces false "related code" candidates.
- Improves duplicate detection where names differ but roles/types match.

### What it does not do

Type facts do not change token budgeting. They improve precision, not the compression
mechanism. They must remain an optional enrichment with `confidence = exact`, not a new
invariant or a hard dependency.

### Cost constraint

Type facts are cheap at a few anchors and expensive project-wide. The rule is:

1. ScopeMux locates the candidate set from its own map.
2. LSP resolves exact semantic edges only for the anchors that matter.
3. The result is cached and invalidated by file hash/document version.
4. Without LSP, heuristic edges remain and the task still completes.

## Composition and precedence

| Concern | Owner |
|---|---|
| CST/AST, ranges, InfoBlocks, token estimates, slicing | ScopeMux parser |
| Project map, delta, plan nodes, observability, duplication, reconciliation | ScopeMux |
| Exact definition/reference/type/implementation/call hierarchy | LSP, optional |
| Diagnostics | LSP, optional |
| Semantic edge confidence | LSP = exact; ScopeMux = heuristic |
| Budgets, stage, inclusion, disposition | Runtime |

Precedence rules:

- Exact LSP facts outrank ScopeMux heuristic edges.
- ScopeMux may consume LSP facts as optional enrichment; it must never require LSP.
- ScopeMux must not override explicit task scope with low-confidence links
  (`invariants/retrieval.md`).
- The runtime remains the only authority over stage, verification, review, and completion.

## Current ORT state (2026-11-05)

`LSP-001` wired the capability end-to-end:

- `crates/opencode-lsp/src/lib.rs` implements the client and registry, including
  definition, references, hover, document/workspace symbols, implementations, type
  definitions, rename, diagnostics, and call hierarchy. The registry is config-driven and
  lazily spawns per `(workspace root, server)` with extension gating, broken-server
  suppression, `Content-Length` framing, env propagation, shutdown, and status.
- `opencode-tool`'s `lsp` feature is now a default feature, so released binaries expose
  the real tool instead of placeholder output.
- The server configures the shared registry from the `lsp` block per workspace and
  attaches it to `SessionPrompt` and `ToolContext`; `write`, `edit`, and `apply_patch`
  collect bounded post-edit diagnostics when a server is available.
- The server `/lsp` endpoint returns actual connected servers and roots (plus failed
  servers), and the TUI renders the same data.
- With no `lsp` config, no server is spawned and behavior is unchanged.

The remaining boundary layer is `LSP-002`: a narrow, capability-negotiated semantic-fact
interface for consumers such as `SCOPE-013`.

## Standardization decisions

Standardize the contract and lifecycle, not a specific server list:

1. Adopt vanilla's `lsp` config shape and activation semantics: server entries with
   `command`, `extensions`, `env`, `initialization`, and `disabled`; disabled unless
   configured.
2. Implement the registry lifecycle for real: lazy per-`(root, server)` spawn, extension
   gating, root detection, broken-server suppression, shutdown, and a real `/lsp` status
   endpoint.
3. Enable the build path: make the LSP feature default (or compile `opencode-lsp` in by
   default), and attach the registry to `ToolContext` and `SessionPrompt`.
4. Keep diagnostics bounded: current-file diagnostics after edits, capped project-wide like
   vanilla's 5-file limit, with timeouts and no-op safeguards.
5. Define precedence in the retrieval boundary: LSP = exact point oracle; ScopeMux =
   project/task context. ScopeMux may consume LSP as optional enrichment; never the
   reverse.
6. Keep LSP opt-in per workspace and measure before forcing it on. Vanilla's own guidance
   is that language servers are not always a net positive.

## Non-goals

- Replacing ScopeMux parsing with LSP.
- Adding a type system or compiler frontend to ScopeMux.
- Making LSP a hard dependency of the map.
- Making ScopeMux a second LSP client.
- Letting either capability own task state, lifecycle, verification, review, or completion.

## Related work

- `LSP-001` Standardize LSP runtime wiring and configuration.
- `LSP-002` Add a capability-negotiated LSP semantic-fact interface.
- `SCOPE-012` Consolidate ScopeMux and LSP responsibilities and remove duplicate
  point-query paths.
- `SCOPE-013` Ingest LSP-derived exact type facts into the ScopeMux map.
- `SCOPE-014` Measure the context value of LSP-derived semantic facts.
- Handoff: `handoffs/SCOPE-LSP-responsibility-boundary-handoff.md`.
- Program: `FUTURE-PROOFING-013` Workspace Intelligence Protocol program.

## Related documents

- `wiki/workspace-intelligence.md`
- `wiki/scopemux-integration-plan.md`
- `wiki/scopemux-map-integration.md`
- `invariants/workspace-intelligence.md`
- `invariants/retrieval.md`
- `docs/opencode-lsp.md`
