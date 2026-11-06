---
id: "SCOPE-010"
title: "Define language coverage and fallback matrix for workspace intelligence"
priority: "P2"
type: "research"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "done"
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
## Implementation (2026-11-05, H-018 step 6)

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/161
- Branch: `feature/SCOPE-010-language-coverage-matrix`
- Commit: `bb937d3`
- **Awaiting human merge approval (H-018 one-PR-at-a-time gate).** Card stays in `doing`
  until merge; move to `qa` after merge and promote only with post-merge QA.

Delivered:

- Coverage matrix in `wiki/workspace-intelligence.md` for C, C++, Python, JavaScript,
  TypeScript, Rust (first-class, known resolution gaps), markdown/prose (generic search),
  and unsupported languages.
- Mixed-language behavior: supported files are parsed per file; unsupported files are
  skipped and recorded as `skipped unsupported language: <path>` in `provider_notes`.
- Typed `RetrievalFallbackReason` on `RetrievalResponse.fallback_reason`:
  `unsupported_language`, `no_matches`, `provider_unavailable`.
- Session fallback distinguishes unsupported workspace from provider failure.
- No new grammars.

Verification:

```bash
$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-retrieval -p opencode-session
opencode_retrieval: test result: ok. 7 passed; 0 failed
opencode_session:   test result: ok. 176 passed; 0 failed
deepseek_fixture:   test result: ok. 2 passed; 0 failed
session_integration:test result: ok. 11 passed; 0 failed

$ cargo test -p opencode-scopemux --features native
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo fmt --all --check
(exit 0)

$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-types -p opencode-retrieval -p opencode-scopemux -p opencode-session --all-targets
(exit 0)
```
### Post-merge QA (2026-11-05)

- PR #161 merged into `development` as `bc4c8de`; branch deleted.
- Re-ran on merged `development`:
  - `cargo test -p opencode-scopemux --features native` -> 21 passed, 0 failed.
  - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-retrieval -p opencode-session`
    -> retrieval 7; session 176 + 2 + 11 passed, 0 failed.
  - `cargo fmt --all --check` -> exit 0.
- Coverage matrix, mixed-language provenance, and fallback reason all verified on the
  merged base. Promoting `qa` -> `done`.