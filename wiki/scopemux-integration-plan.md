# ScopeMux Integration Plan

## Purpose

Define the minimum architecture boundary the Rust runtime should preserve so `ScopeMux` can improve retrieval and context quality later without expanding V1 scope now.

## Current Position

`ScopeMux` is not a V1 requirement.

V1 still needs to ship as a useful personal daily-driver with explicit task state, runtime-owned lifecycle stages, focused context construction, verification, and fresh-context review.

The current Rust runtime already assembles context directly inside session and prompt code. That is acceptable for V1 as long as the system does not hard-code assumptions that prevent a later retrieval layer from being inserted cleanly.

## Planning Constraints

- `ScopeMux` must remain optional for V1.
- The runtime stays authoritative for task state, lifecycle transitions, verification, review, and completion.
- Context construction must continue working with generic repository facts even when no structural graph is available.
- Future `ScopeMux` support should improve retrieval quality, not become a hidden dependency for basic operation.

## What Stays Generic In V1

V1 should keep these responsibilities in the generic runtime layer:

- selecting or accepting one bounded task
- recording authoritative task objective, criteria, stage, and verification expectations
- deciding which context role is being built, such as implementation or review
- reading repository files, docs, and basic workspace facts
- applying token budgets and stage-specific context limits
- deciding when verification and review must run
- recording verification and review outcomes in structured task state

These behaviors are core runtime responsibilities whether retrieval is simple file-based discovery or a future structural system.

## What Future ScopeMux Should Own

Once mature enough, `ScopeMux` should own retrieval-quality improvements rather than core workflow control.

Candidate `ScopeMux` responsibilities:

- symbol and structure-aware repository retrieval
- heuristic relationship discovery between files, symbols, and task-relevant subsystems
- ranked retrieval candidates for a given task and stage
- optional graph-backed expansion from one anchor file or symbol to nearby relevant context
- confidence-scored structural links that the runtime can accept, trim, or ignore

`ScopeMux` should not own:

- task lifecycle stages
- completion decisions
- verification gates
- review authority
- session continuity

## How ScopeMux Can Help This Project

`ScopeMux` is not only a future idea: the concrete engine is `scopemux-core` (`$HOME/apps/scopemux-core`), a native Tree-sitter parsing and context-compression library. Its current API already covers most of the retrieval-quality responsibilities listed above, which is why preserving the boundary now is worthwhile.

The mapping below grounds each ability in a real `scopemux-core` surface and the runtime need it can improve.

| `scopemux-core` capability | What it can improve here | Where it plugs in |
| --- | --- | --- |
| `ParserContext` (Tree-sitter parse, AST/CST) | structured reads of files instead of raw text | retrieval provider request/response; tool-input enrichment |
| `ProjectContext` multi-file parse and project-wide symbol table | repository-wide symbol awareness beyond single-file grep | retrieval provider for a task and stage |
| Cross-file reference resolution (`reference_resolver.h`) | following calls, types, and imports/includes across files | relationship expansion from a seed file or symbol |
| Project IR: symbols, resolved references, call graph, dependencies | discovering the task-relevant subsystem; sharper review context | ranked candidates plus provenance |
| Canonical `InfoBlock` registry (tiers 0-4) | selecting cheap-to-expensive context instead of uniform file dumps | stage-specific context budgets |
| Tiered context selection (focus, exclude, summary-only, tier range) | fitting context to a token budget without losing structure | prompt context assembly |
| Search index over query text, anchor symbol, and relationships | ranked retrieval for a task objective or review | retrieval response ordering |
| Prompt assembly API | a deterministic, token-aware context package | the boundary output consumed by prompt construction |
| `ContextEngine` relevance metrics, compression levels, token budgets | trimming and compressing context under real limits | token-budget enforcement during assembly |

Concretely, this can give the product:

- better retrieval quality on multi-file and unfamiliar codebases
- structure-aware review context rather than recency-only file selection
- deterministic token budgeting on the same boundary the generic provider uses
- an upgrade path where the runtime keeps lifecycle and verification authority while `ScopeMux` only improves inputs

Honest limits to design around:

- `scopemux-core` does not parse Rust today (`scopemux-core/README.md` lists C, C++, Python, JavaScript, and TypeScript), so it cannot help with this product's own repository or Rust workspaces; those must fall back to the generic provider.
- the shipped Python extension exposes only `ParserContext` and `ContextEngine` (`scopemux-core/core/src/bindings/module.c`); `ProjectContext`, tiered context, search, and prompt assembly are C-only, so integration needs FFI or a compiled helper.
- `scopemux-core` is development-oriented and not packaged for standard distribution, so it must be pinned and built reproducibly rather than assumed present.
- its structural relationships are heuristic, so the confidence rules below still apply.

This near-term integration is tracked by `SCOPE-002` ("Integrate scopemux-core behind the retrieval-provider boundary"). It stays non-blocking for V1: the generic provider remains the default, and the `ScopeMux` provider is added as one peer behind the boundary.

The larger idealized target is `SCOPE-001` ("Idealized scopemux map integration") and `wiki/scopemux-map-integration.md`, which extend this plan with current/target state, delta views, observability blocks, duplication detection, and incremental reconciliation. Those phases must not disturb the boundary or guardrails defined here.

## Required Abstraction Boundary

The runtime should preserve one explicit retrieval boundary between:

- task and stage intent, which the runtime owns
- retrieval results, which a generic file-based path or future `ScopeMux` path may produce

The boundary should look conceptually like this:

1. The runtime defines a retrieval request from task state.
2. A retrieval provider returns ranked context candidates plus provenance.
3. The runtime assembles the final stage-specific prompt context from those results.

The key design rule is that prompt construction must consume retrieval outputs, not embed `ScopeMux`-specific logic directly throughout runtime orchestration.

## Minimum Deferred Contract

Future `ScopeMux` integration should be able to plug into a narrow contract that includes:

- task objective
- task stage
- repository root or workspace target
- optional seed files, symbols, or changed files
- context role, such as implementing or reviewing
- token or item budget

The retrieval response should return:

- ordered candidate files, symbols, or snippets
- provenance for why each candidate was returned
- confidence per candidate or relationship cluster
- enough metadata for the runtime to decide what to include in final context

This contract stays intentionally small so V1 can implement a simple generic provider first.

## Confidence Model

`ScopeMux` structural relationships will likely include heuristics, so the runtime should treat them as evidence rather than truth.

Rules for confidence handling:

- exact repository facts outrank heuristic relationships
- directly referenced files and symbols outrank inferred neighbors
- low-confidence structural links may enrich context, but should not override explicit task scope
- review context should use a stricter inclusion threshold than implementation context
- the runtime should retain provenance so bad retrieval can be diagnosed later

This keeps retrieval mistakes from silently reshaping the task.

## Phase Guidance

### V1

- keep retrieval generic and local
- preserve the retrieval-provider boundary
- do not block shipping on `ScopeMux`

### V2

- allow an early `ScopeMux` provider if it materially improves repository discovery and context quality
- compare generic retrieval and `ScopeMux` retrieval using the evaluation strategy

### V3+

- expand `ScopeMux` use only if real usage shows better task outcomes, cleaner review context, or fewer retrieval-related failures

## Follow-Up Work

This plan requires explicit follow-up items:

- `START-025` Add retrieval-provider boundary for task context assembly — introduce the narrow abstraction that lets V1 keep generic retrieval while leaving a clean insertion point for future `ScopeMux` support.
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary — connect the concrete `scopemux-core` engine as one provider behind that boundary (Phase 0, near-term).
- `SCOPE-001` Idealized scopemux map integration — the target map model and phases (`SCOPE-003`, `SCOPE-004`) that extend this plan without changing its guardrails.

## Non-Goals

- shipping `ScopeMux` in V1
- building a full repository graph now
- replacing normal file reads for straightforward small tasks
- making `ScopeMux` the owner of task state or runtime progression

## Bottom Line

The Rust runtime should ship V1 without `ScopeMux`, but it should not hard-wire context construction so tightly that future structural retrieval becomes invasive. Preserve a small retrieval-provider boundary now, keep runtime authority over task flow, and let later `ScopeMux` adoption compete on retrieval quality rather than on architectural control.
