---
id: "SCOPE-017"
title: "Add a workspace map supervisor with watcher-driven incremental updates"
priority: "P1"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "doing"
created: "2026-11-06"
updated: "2026-11-07"
predecessor: "SCOPE-016"
---

# Add a workspace map supervisor with watcher-driven incremental updates

## Summary

Add the single owner of workspace-map lifecycle: a supervisor/registry that opens one map
per canonical workspace root, subscribes to `opencode-watcher`, coalesces dirty paths, and
drives incremental engine updates. This is what makes the map persistent and current across
retrieval calls and tool calls.

This is step 3 of `SCOPE-015`.

## Scope

- `WorkspaceMapRegistry` / `MapSupervisor`:
  - open-or-get a map by canonical workspace root,
  - reference counting or explicit acquire/release,
  - idle eviction/TTL and shutdown,
  - one serialized writer/worker per map; queries do not create contexts.
- Watcher ingestion:
  - subscribe to `opencode_watcher::FileWatcher` events,
  - debounce and coalesce add/change/unlink into a dirty set,
  - apply the shared ignore/exclusion policy from `SCOPE-018`,
  - route updates through the engine (`update_file`/`remove_file` + incremental rebuild).
- Freshness counters:
  - last parsed revision/mtime, dirty-set size, watcher lag, last rebuild duration,
  - re-anchor result for projected plan nodes,
  - expose values for `SCOPE-009`; do not decide fallback here.
- Lifecycle and failure policy:
  - map creation failure is reported as unavailable, not silent,
  - a failed update keeps the previous map and marks it degraded/stale for the runtime,
  - no task-state writes.

## Acceptance criteria

- One map instance per workspace root; repeated retrieval/tool calls reuse it.
- A file edit produces updated nodes without a full reparse/rebuild of the workspace.
- Add, change, and unlink events are handled; deletes do not leave stale nodes.
- Watcher and indexer use the same ignore policy as grep (`target/`, `node_modules/`,
  `.git/`, build/vendor dirs), so excluded files never enter the map.
- Eviction/shutdown releases the context and stops the watcher; no cross-workspace leakage.
- Freshness counters are recorded and readable; no fallback decision is made here.
- Map state is evidence-only; task stage/verification/review/completion are untouched.

## Non-goals

- No provider activation or prompt policy.
- No tool surface (that is `SCOPE-019`).
- No cross-process/out-of-process map.
- No full real-time guarantee; bounded debounce and coalescing are expected.

## Touchpoints

- new supervisor module (for example `crates/opencode-scopemux/src/supervisor.rs` or a
  small dedicated crate if the tool must depend on it without FFI)
- `crates/opencode-watcher/src/lib.rs` (event consumption; avoid map-specific logic here)
- `crates/opencode-scopemux/src/engine.rs`
- `crates/opencode-types/src/retrieval.rs` (freshness value type from `SCOPE-018`)
- `wiki/scopemux-map-integration.md`

## Verification

```bash
cargo clean -p opencode-scopemux
SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
  cargo test -p opencode-scopemux --features native
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-watcher -p opencode-scopemux
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux -p opencode-watcher --all-targets
```

Required tests: registry reuse, edit -> incremental update, unlink -> removal, ignore
policy agreement with grep, eviction/shutdown, and no cross-workspace leakage.

## Risks

- Watcher storms and debounce correctness; start with a bounded dirty set and coalescing.
- FFI thread affinity; the supervisor must serialize engine access.
- Stale map after a failed update must be visible, never silent.

## Related Items

- `SCOPE-015` program epic.
- `SCOPE-016` engine (predecessor).
- `SCOPE-018` shared ignore/freshness contracts.
- `SCOPE-019` integration and tool capability.
- `SCOPE-009` freshness/staleness SLOs (consumer).