---
id: "SCOPE-018"
title: "Centralize shared workspace-map contracts and policy"
priority: "P1"
type: "refactor"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-11-06"
updated: "2026-11-06"
predecessor: "SCOPE-014"
---

# Centralize shared workspace-map contracts and policy

## Summary

Establish the shared contracts and single-source policies the persistent map program needs,
so the engine, supervisor, provider, and tool do not grow competing result types, ignore
lists, language predicates, activation rules, or freshness vocabularies.

This is step 1 of `SCOPE-015` and must land first because `SCOPE-016`, `SCOPE-017`, and
`SCOPE-019` consume these contracts.

## Scope

- Shared evidence type:
  - add `block_id: Option<String>` (and any missing provenance fields) to
    `RetrievalCandidate` or introduce one map evidence type used by both retrieval and the
    code-search tool; do not create a tool-only result struct.
- Freshness contract:
  - add `MapFreshness` (`fresh`, `lagging`, `stale`, `unknown`) and the measurement fields
    the supervisor will fill (last revision, dirty-set size, watcher lag, re-anchor result).
  - keep the runtime fallback policy out of the type.
- Ignore/exclusion policy:
  - one source of truth consumed by `grep_tool.rs`, `opencode-watcher`, and the indexer.
  - default exclusions include at least `target/`, `node_modules/`, `.git/`, `.opencode/`,
    `dist/`, `build/`, `.venv/`, `venv/`, `__pycache__/`, `.cache/`.
- Language/prose policy:
  - one public `is_code_path`/language predicate shared with the `SCOPE-010` matrix; no
    second extension table.
- Activation policy:
  - one predicate for the Qwen-only map path (already `model_scope_enabled`); expose a
    capability handle or neutral `WorkspaceSearch` trait that the runtime injects into
    `ToolContext`; the tool must not parse model strings.
- Shared helpers:
  - one path-normalization helper (`file://`, relative vs absolute, canonical root),
  - one token budget/truncation utility for prompt and tool output.
- Documentation: update `wiki/workspace-intelligence.md` and
  `wiki/scopemux-integration-plan.md` with the ownership and DRY rules.

## Acceptance criteria

- There is one ignore policy source and tests prove grep, watcher, and indexer agree.
- There is one code/prose predicate and no duplicated extension lists.
- `RetrievalCandidate`/shared evidence carries `block_id` and existing consumers compile.
- Freshness and fallback vocabularies are shared types, not stringly duplicated.
- Activation is decided once; the tool receives a capability, not model strings.
- The generic provider path and task authority are unchanged.

## Non-goals

- No supervisor, engine, or watcher implementation (those consume this card).
- No tool behavior change beyond receiving the capability handle.
- No new grammars or language servers.
- No prompt-policy redesign.

## Touchpoints

- `crates/opencode-types/src/retrieval.rs`
- `crates/opencode-retrieval/src/lib.rs`
- `crates/opencode-watcher/src/lib.rs` (`WatcherConfig` / ignore policy)
- `crates/opencode-tool/src/grep_tool.rs` (`EXCLUDED_DIRS`)
- `crates/opencode-tool/src/tool.rs` (`ToolContext` capability field)
- `crates/opencode-scopemux/src/lib.rs` (`language_for_path`, `model_scope_enabled`)
- `wiki/workspace-intelligence.md`, `wiki/scopemux-integration-plan.md`

## Verification

```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-types -p opencode-retrieval -p opencode-tool -p opencode-watcher
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-scopemux -p opencode-session
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-types -p opencode-retrieval -p opencode-tool -p opencode-watcher --all-targets
```

Add tests for ignore-policy agreement, code/prose classification, shared freshness/fallback
serialization, and capability injection.

## Risks

- Over-DRY: grep scanning and map indexing share policy and evidence, not implementation.
- Type churn: keep `RetrievalCandidate` additions serde-compatible and backward-tolerant.
- Tool coupling: the tool must not depend on `opencode-scopemux` FFI directly.

## Related Items

- `SCOPE-015` program epic.
- `SCOPE-016` engine (consumer).
- `SCOPE-017` supervisor/watcher (consumer).
- `SCOPE-019` provider/tool integration (consumer).
- `SCOPE-006` code-search tool (consumer).
- `SCOPE-009` freshness SLOs (consumer).
- `SCOPE-010` language coverage matrix.
