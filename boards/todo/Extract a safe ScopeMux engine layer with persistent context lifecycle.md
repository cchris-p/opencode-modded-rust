---
id: "SCOPE-016"
title: "Extract a safe ScopeMux engine layer with persistent context lifecycle"
priority: "P1"
type: "refactor"
area: "SCOPE"
spec: "wiki/scopemux-integration-plan.md"
status: "todo"
created: "2026-11-06"
updated: "2026-11-06"
predecessor: "SCOPE-018"
---

# Extract a safe ScopeMux engine layer with persistent context lifecycle

## Summary

Split the native adapter into a policy-free engine layer that safely owns a long-lived
`ProjectContext`. The provider becomes a thin adapter; the engine owns open/update/remove,
incremental rebuild, and query calls, and never holds a raw pointer across an `await`.

This is step 2 of `SCOPE-015` (after `SCOPE-018`).

## Scope

- Split `crates/opencode-scopemux/src/lib.rs` into at least:
  - `ffi`: raw `extern "C"` declarations and `#[repr(C)]` mirrors only.
  - `engine`: a safe `WorkspaceEngine` over one `ProjectContext`.
  - `provider`: the `RetrievalProvider` adapter and slice mapping.
- Engine API (shape may refine during implementation):
  - `open(root)`, `close`/`Drop`
  - `add_file(path)`, `update_file(path)`, `remove_file(path)`
  - `rebuild` / `rebuild_incremental`
  - `query` entry points used by search/delta/anchors/observability/duplicates/impact
  - counters: last rebuilt file count, last recomputed file count, incremental flag,
    InfoBlock reused/re-derived counts
- Use the existing core incremental surface: `project_update_file`,
  `project_context_remove_file`, `project_context_rebuild_ir`,
  `project_context_last_recompute...` where available. Do not invent a second index.
- Concurrency contract: one engine is owned by one worker/task; no `*mut ProjectContext`
  escapes the engine or crosses an `await`. Document the `Send`/`Sync` story explicitly.
- Keep current provider behavior byte-compatible until `SCOPE-017` changes lifecycle.

## Acceptance criteria

- No raw FFI pointer is exposed outside `ffi`/`engine`.
- The engine can be created once, updated after an edit, and queried without recreating
  the context; a proof test shows the updated symbol is visible.
- `project_update_file` + incremental rebuild is used; the incremental counters show the
  dirty set is smaller than a full rebuild for a one-file change.
- Provider output is unchanged for all existing tests.
- The engine contains no model activation, prompt, task-state, or fallback policy.
- Native and non-native builds compile; `opencode-scopemux` default tests stay green.

## Non-goals

- No watcher and no background supervisor (that is `SCOPE-017`).
- No provider/tool wiring change beyond making the provider use the engine.
- No new dependency on `opencode-tool` or `opencode-session`.
- No new grammars or `scopemux-core` pin change unless the engine needs an already-pinned
  call.

## Touchpoints

- `crates/opencode-scopemux/src/lib.rs` (split into `ffi`, `engine`, `provider`)
- `crates/opencode-scopemux/build.rs` only if module layout/build inputs require it
- `crates/opencode-scopemux/src/engine.rs` (new)
- `wiki/scopemux-integration-plan.md` (engine boundary note)

## Verification

```bash
cargo clean -p opencode-scopemux
SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
  cargo test -p opencode-scopemux --features native
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-scopemux
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux --all-targets
```

Add proof tests for: persistent context reuse, edit -> update -> query visibility, and
incremental-vs-full rebuild counters on a one-file change.

## Risks

- FFI lifetime and thread affinity: a long-lived pointer is the main hazard. Prove the
  single-owner worker contract before `SCOPE-017`.
- Upstream API gaps: if `scopemux-core` lacks a needed call, record it as a prerequisite
  instead of adding a parallel index.

## Related Items

- `SCOPE-015` program epic.
- `SCOPE-018` shared contracts/policy (predecessor).
- `SCOPE-017` supervisor/watcher (successor).
- `SCOPE-019` provider/tool integration.
- `SCOPE-001` incremental reconciliation design.
