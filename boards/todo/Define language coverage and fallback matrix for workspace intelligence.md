---
id: "SCOPE-010"
title: "Define language coverage and fallback matrix for workspace intelligence"
priority: "P2"
type: "research"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Define language coverage and fallback matrix for workspace intelligence

## Summary

Define which languages and file types workspace intelligence supports, what quality is
expected for each, and what the fallback is for everything else.

This prevents silent mis-parsing and sets honest expectations for Rust, the supported
grammars, markdown/prose, and unsupported languages.

## Why this exists

`scopemux-core` supports C, C++, Python, JavaScript, TypeScript, and Rust. Rust resolution
is not at C/C++ parity (trait dispatch, macros, generics). Markdown/prose is not code.
Unsupported languages must fall back explicitly, not silently return an empty map.

## Scope

- Define a coverage matrix:
  - language,
  - parse support,
  - symbol/reference support,
  - known gaps,
  - confidence expectations,
  - fallback.
- Mark Rust as first-class for this product, with its known resolution gaps.
- Mark markdown/prose as generic search, not code-grammar search.
- Define unsupported-language behavior: explicit provenance signal + generic fallback.
- Define mixed-language workspaces: per-file language detection and partial coverage.
- Add a provenance field for "unsupported language" vs "no matches."
- Add tests for supported, partially supported, and unsupported workspaces.
- Document the matrix in `wiki/workspace-intelligence.md`.

## Acceptance criteria

- A coverage matrix exists for every supported language.
- Rust coverage and gaps are explicit.
- Markdown/prose uses the correct search mode.
- Unsupported language produces an explicit fallback signal, not a false empty result.
- Mixed-language workspaces degrade per file, not all-or-nothing.
- Tests cover the matrix.

## Non-goals

- Adding new language grammars in this card.
- Reaching C/C++ parity for Rust.
- Parsing prose as code.
- A universal language server.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-017`
- `SCOPE-006` map-backed locate tool with grep fallback
- `wiki/workspace-intelligence.md`
- `wiki/scopemux-integration-plan.md`
- `invariants/retrieval.md`
