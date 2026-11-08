---
id: "SCOPE-019"
title: "Wire the persistent map into retrieval and tool search"
priority: "P1"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "done"
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

## Implementation — 2026-11-07 (H-020 step 4)

Branch `feature/SCOPE-019-map-integration`; PR
https://github.com/cchris-p/opencode-modded-rust/pull/167

Landed:

- `ScopemuxProvider` resolves one map per workspace root from the shared
  `WorkspaceMapRegistry`; no per-call context creation. All slices and plan
  materialization/reconciliation use the persistent map.
- `RetrievalResponse.freshness` carries the supervisor measurement; a stale scopemux
  response falls back to the generic provider in `opencode-session` (runtime policy).
- `RegistryWorkspaceSearch` implements the neutral `WorkspaceSearch` trait;
  `workspace_search_for` uses the same `model_scope_enabled` predicate and injects the
  capability into `ToolContext` only on the Qwen-only scope.
- New `code_search` tool: map-backed locate with file/symbol/`block_id`/confidence/
  provenance/freshness, bounded grep fallback, and a `prose` mode.
- Shared block-to-evidence mapping lives in `engine` so provider and capability agree.
- Docs updated (`docs/opencode-tool.md`, `wiki/scopemux-map-integration.md`).

Verification (exact):

```bash
$ cargo clean -p opencode-scopemux

$ SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
    cargo test -p opencode-scopemux --features native
test result: ok. 35 passed; 0 failed; 0 ignored

$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tool -p opencode-session -p opencode-retrieval
opencode-retrieval: 7 passed
opencode-session:   177 passed (+ 2, + 11)
opencode-tool:      75 passed (+ 2, + 11)
opencode-types:     9 passed

$ cargo test -p opencode-tool grep
test result: ok. 5 passed; 0 failed; 0 ignored

$ cargo fmt --all --check
exit 0

$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux -p opencode-tool -p opencode-session --all-targets
Finished; exit 0 (pre-existing warnings only; none in changed paths)
```

Required tests added: provider persistent-reuse (same map per root), map search
block-id/provenance, absent-capability -> grep, stale -> grep/generic, prose -> grep,
activation scope, and no per-call context creation.

`scripts/scopemux-qa-check.sh` (LAN Ollama, `OLLAMA_HOST=http://192.168.1.33:11434`,
model `qwen3:30b`):

```bash
$ scripts/scopemux-qa-check.sh --url http://127.0.0.1:4096 \
    --workspace ~/worktrees/scopemux-qa --file sample.rs \
    --log /tmp/scopemux-qa-server-019.log --expect scopemux
session:    ses_e62481f6a1a44d1c9ebc69dafc8f4aa2
file:       @sample.rs
expected:   scopemux
observed:   scopemux
candidates: 5
PASS
```

Server log: `retrieval provider selected provider=scopemux representation=Search
candidates=5`. This exercises the branch binary against the real LAN Ollama.

Status: PR merged into `development` as `ec50ffe` (2026-11-08).

## QA Verification — 2026-11-08 (post-merge, development @ ec50ffe)

```bash
$ cargo clean -p opencode-scopemux
$ SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
    cargo test -p opencode-scopemux --features native
test result: ok. 35 passed; 0 failed; 0 ignored

$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tool -p opencode-session -p opencode-retrieval
opencode-retrieval: 7 passed
opencode-session:   177 passed (+ 2, + 11)
opencode-tool:      75 passed (+ 2, + 11)
opencode-types:     9 passed

$ cargo test -p opencode-tool grep
test result: ok. 5 passed; 0 failed; 0 ignored

$ cargo fmt --all --check
exit 0

$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux -p opencode-tool -p opencode-session --all-targets
Finished; exit 0 (pre-existing warnings only; none in changed paths)
```

Post-merge QA check with the binary built from merged `development` and the LAN Ollama
(`OLLAMA_HOST=http://192.168.1.33:11434`, `qwen3:30b`):

```bash
$ scripts/scopemux-qa-check.sh --url http://127.0.0.1:4096 \
    --workspace ~/worktrees/scopemux-qa --file sample.rs \
    --log /tmp/scopemux-qa-server-019-postmerge.log --expect scopemux
session:    ses_27d9a24df6724776a3c5bab8bab4a413
file:       @sample.rs
expected:   scopemux
observed:   scopemux
candidates: 5
PASS
```

Server log: `retrieval provider selected provider=scopemux representation=Search
candidates=5`. QA result: pass.

## Related Items

- `SCOPE-015` program epic.
- `SCOPE-017` supervisor (predecessor).
- `SCOPE-018` shared contracts/policy.
- `SCOPE-006` code-search tool (unblocked).
- `SCOPE-009` freshness/SLOs (consumer).
- `SCOPE-004` retrieval slices and prompt injection.
- `SCOPE-010` language coverage/fallback.