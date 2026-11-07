---
id: "SCOPE-019"
title: "Wire the persistent map into retrieval and tool search"
priority: "P1"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "doing"
created: "2026-11-06"
updated: "2026-11-07"
predecessor: "SCOPE-017"
---

# Wire the persistent map into retrieval and tool search

## Summary

Make the retrieval provider and the agent tool surface consume the persistent supervisor
map instead of creating a fresh context per call. This is the integration step that
unblocks `SCOPE-006` and provides the capability `SCOPE-009` will measure.

This is step 4 of `SCOPE-015`.

## Scope

- Provider adapter:
  - `ScopemuxProvider` holds a supervisor/map handle (or resolves one by workspace root),
  - query slices use the persistent map; no `project_context_create` per retrieval,
  - absent/stale/unavailable map returns the existing generic fallback and provenance,
  - plan materialization/reconciliation continues to work against the persistent map.
- Tool capability:
  - define/inject a `WorkspaceSearch` capability (from `SCOPE-018`) through `ToolContext`,
  - add map-backed code search: "where is X" over registered nodes in one call, returning
    file, symbol, block id, confidence, provenance, freshness,
  - keep bounded `grep` as the fallback with existing `spawn_blocking`, exclusions, scan
    limits, and result limits,
  - separate code search from markdown/prose search; prose uses grep,
  - activation follows the existing Qwen-only provider scope; every other model/path uses
    grep/generic.
- Evidence-only: no task stage, verification, review, or completion moved into the map.

## Acceptance criteria

- Retrieval and tool calls reuse one persistent map per workspace; no per-call context
  creation on the map-backed path.
- A code-search call answers "where is X" from the registered node set without a filesystem
  walk and returns provenance including block id.
- Unsupported language, absent map, stale map, or search failure falls back to bounded grep
  with explicit provenance.
- The tool surface distinguishes code search from prose search.
- Activation matches the existing `model_scope_enabled` rule and is decided once.
- `SCOPE-006` acceptance criteria can now be met and verified.
- Generic provider behavior and task authority are unchanged.

## Non-goals

- No new indexing algorithm (the supervisor owns it).
- No markdown/prose parsing as code.
- No prompt-budget redesign.
- No capability profiles or out-of-process transport.

## Touchpoints

- `crates/opencode-scopemux/src/lib.rs` / `provider.rs` / `supervisor.rs`
- `crates/opencode-session/src/retrieval.rs` (provider selection/fallback)
- `crates/opencode-tool/src/tool.rs` (`ToolContext` capability)
- `crates/opencode-tool/src/grep_tool.rs` (fallback + mode split)
- new code-search module under `crates/opencode-tool/src/`
- `crates/opencode-types/src/retrieval.rs` (evidence/freshness)
- `docs/opencode-tool.md`, `wiki/scopemux-map-integration.md`

## Verification

```bash
cargo clean -p opencode-scopemux
SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
  cargo test -p opencode-scopemux --features native
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tool -p opencode-session -p opencode-retrieval
cargo test -p opencode-tool grep
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux -p opencode-tool -p opencode-session --all-targets
scripts/scopemux-qa-check.sh --file <supported-file> --expect scopemux
```

Required tests: persistent provider reuse, map search provenance/block id, unsupported ->
grep fallback, stale -> generic fallback, activation scope, and no per-call context
creation.

## Risks

- Stale-map correctness: never present stale nodes as fresh; surface freshness and fall
  back when required.
- Tool coupling: keep the tool behind the capability trait, not FFI.
- Fallback parity: preserve grep bounds exactly (no regression to `BUG-040`).

## Related Items

- `SCOPE-015` program epic.
- `SCOPE-017` supervisor (predecessor).
- `SCOPE-018` shared contracts/policy.
- `SCOPE-006` code-search tool (unblocked).
- `SCOPE-009` freshness/SLOs (consumer).
- `SCOPE-004` retrieval slices and prompt injection.
- `SCOPE-010` language coverage/fallback.