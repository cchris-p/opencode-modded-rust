---
id: "SCOPE-003"
title: "Project authoritative task state into scopemux plan nodes"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "done"
created: "2026-09-23"
updated: "2026-11-05"
predecessor: "SCOPE-002"
---

# Project authoritative task state into scopemux plan nodes

## Summary

Bridge the runtime's authoritative task record to `scopemux` target-state nodes so a task's intended change is represented in the map as plan InfoBlocks, and reconciled as the code changes.

This is Phase 2 of `SCOPE-001`. It does not give `scopemux` any authority over task state.

## Why this exists

`SCOPE-001` defines target state as additional InfoBlock nodes that describe code that does not exist yet. Without a defined contract, plan nodes would either be authored ad hoc or would duplicate task intent, which would break `invariants/task-state.md` ("structured task state is authoritative for bounded work execution").

## Scope

- Define the projection contract from the durable task record to plan nodes: which fields (`objective`, `completion_criteria`, `stage`, `workspace_target`, `artifacts`, `reopen_reason`) map to which plan-node attributes.
- Implement the projection so plan nodes carry anchors, desired shape, rationale tied to completion criteria, provenance, and lifecycle.
- Implement the reconciliation handshake: observed code changes and plan-node lifecycle transitions are reported back to the runtime as evidence, and the runtime remains the only writer of task stage and completion.
- Surface plan-node conflicts (`stale`, `conflict`) to the runtime as explicit signals so a task can be re-planned instead of silently diverging.
- Keep the durable task record in `crates/opencode-types` / `crates/opencode-storage` / `crates/opencode-session` as the sole authority; `scopemux` holds only projections.

## Non-goals

- Moving task, stage, verification, review, or completion authority into `scopemux`.
- Persisting task state only as plan nodes.
- Auto-advancing a task stage from map evidence without a runtime-recorded transition.

## Done when

- A task record produces plan nodes with anchors, rationale, provenance, and lifecycle.
- Plan-node lifecycle changes are reported to the runtime without advancing task stage.
- `stale`/`conflict` plan nodes surface as explicit re-plan signals.
- The runtime still completes tasks with no plan nodes present (generic path unaffected).
- The task record round-trips unchanged; no field authority is transferred.

## Recommended verification

- Create a task, project it, mutate the anchored code, and confirm plan nodes reconcile and conflicts surface.
- Confirm the durable task record wins on any disagreement with plan-node state.
- Confirm a task completes when `scopemux` is absent.

## Dependencies and design (2026-09-24)

Gated on upstream `scopemux-core` work, sequenced by `H-011`: `WI-033` (origin/lifecycle/provenance/confidence on `ProjectInfoBlock`), `WI-032` (target-state plan InfoBlocks + reconciliation), then `WI-036` (delta/query API); `WI-031` persists plan nodes. Only then can this card project task state into real plan nodes.

Product-side shape:

- Extend `opencode-scopemux` FFI bindings and the retrieval contract to carry origin/lifecycle and delta entries; keep the `ProjectSearchResult`/`ProjectInfoBlock` layout in lockstep with the pinned `scopemux-core` revision.
- Map task-record fields (`objective`, `completion_criteria`, `stage`, `workspace_target`, `artifacts`, `reopen_reason`) to plan-node attributes.
- Reconciliation handshake: plan-node lifecycle transitions and `stale`/`conflict` are evidence returned to the runtime; the runtime remains the sole writer of stage and completion.

Do not start implementation until `WI-032`/`WI-033`/`WI-036` land; the runtime must keep completing tasks with no plan nodes present.

## Related Items

- `SCOPE-001` Idealized scopemux map integration.
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary - predecessor.
- `SCOPE-004` Consume map delta, observability, and refactor nodes in context assembly.
- `START-016` Define structured task state for V1.
- Upstream (in-scope integration work per `invariants/integration-scope.md`): `WI-032`, `WI-033`, `WI-036`, plus `FIX-002` (block-type disambiguation) and `FIX-006` (planning-doc alignment) in `$HOME/apps/scopemux-notes`.

## Notes

- Depends on `SCOPE-002` and the upstream plan-InfoBlock work (`WI-032`, `WI-033`), which the integration program owns and develops in `$HOME/apps/scopemux-notes`.
- Authority rule: if a plan node and the task record disagree, the task record wins.

## Implementation (2026-09-24)

Implemented in `opencode-modded-rust` PR #107 (merged into `development`, base `development`), now that `WI-032`/`WI-033`/`WI-036` have merged to `scopemux-core` `main`:

- Projection contract in `opencode-types::plan`: `PlanNodeKind`, `PlanLifecycle`, `PlanNodeDraft`, `PlanReconciliationSignal`, and `project_task_to_plan_nodes` mapping `objective`, `completion_criteria`, `workspace_target`, `artifacts`, and `reopen_reason` to plan-node attributes (backticked criterion symbol -> `projected_symbol`).
- `RetrievalRequest` carries `task_id` + projection-only `plan_nodes`; `RetrievalResponse` carries `plan_signals`; `RetrievalCandidate` carries `origin`/`lifecycle`.
- `opencode-scopemux` FFI synced to the new `ProjectInfoBlock` layout and extended with plan-node create/set/anchor/reconcile; the native provider materializes the task's plan nodes, reconciles against parsed state, and returns `stale`/`conflict`/`implemented` as evidence only.
- Pin advanced to `scopemux-core` `main` `1b3cbbe`.
- Verified: default `cargo test` for the touched crates, clippy clean on touched files, `cargo check --workspace`, and native `cargo test -p opencode-scopemux --features native` 5/5. (Two pre-existing `opencode-session` instruction path tests fail on macOS, unrelated.)
- Authority preserved: no stage/verification/review/completion is written from plan evidence.

## QA Verification (2026-11-05, H-018 step 1)

Base: `development` `06448ca` with the pinned prebuilt native bundle
`~/.cache/opencode-modded-rust/native/linux-x86_64-2a642c10534f`
(`scopemux-core` `2a642c1`).

### Pre-fix reproduction (real blocker)

```bash
$ cargo test -p opencode-scopemux --features native tests::native_provider_reports_plan_reconciliation_evidence -- --exact
...
error: test failed, to rerun pass `-p opencode-scopemux --lib`
  process didn't exit successfully: `target/debug/deps/opencode_scopemux-3b2bac0e52545e8f 'tests::native_provider_reports_plan_reconciliation_evidence' --exact --nocapture` (signal: 11, SIGSEGV: invalid memory reference)
```

`gdb` localized the fault to `cstr(ptr=0x200000001)` in `search_candidates`
(`crates/opencode-scopemux/src/lib.rs:698`). Root cause: the pinned `scopemux-core`
`ProjectInfoBlock` and `ProjectPlanNode` structs gained `observability_kind` after `kind`
(`WI-034`), but the Rust `#[repr(C)]` mirrors omitted it. On a planned InfoBlock the
misaligned `provenance` field read the packed `origin`/`lifecycle` ints
(`0x0000000200000001`) instead of a pointer. This also explains the previously recorded
`native_delta_slice_returns_projected_entries` failure.

### Fix PR

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/158
- Branch: `feature/SCOPE-003-plan-reconciliation-ffi-layout`
- Commit: `d67f759` — add `observability_kind` to the `ProjectInfoBlock` and
  `ProjectPlanNode` FFI mirrors.
- **Awaiting human merge approval (H-018 one-PR-at-a-time gate).** Card stays in `qa`;
  promote only after merge + post-merge QA on `development`.

### Post-fix verification

```bash
$ cargo test -p opencode-scopemux --features native tests::native_provider_reports_plan_reconciliation_evidence -- --exact
test tests::native_provider_reports_plan_reconciliation_evidence ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 17 filtered out

$ cargo test -p opencode-scopemux --features native
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p opencode-types -p opencode-retrieval
opencode_retrieval: test result: ok. 6 passed; 0 failed
opencode_types:     test result: ok. 4 passed; 0 failed

$ SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-session
lib:                 test result: ok. 172 passed; 0 failed
deepseek_fixture:    test result: ok. 2 passed; 0 failed
session_integration: test result: ok. 11 passed; 0 failed

$ cargo fmt --all --check
(exit 0)
```

Authority check: no task stage, verification, review, or completion is written from plan
evidence; the fix only restores the FFI layout.
### Post-merge QA (2026-11-05)

- PR #158 merged into `development` as `a701afd`; feature branch deleted.
- Re-ran on merged `development`:
  - `cargo test -p opencode-scopemux --features native` -> 18 passed, 0 failed.
  - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-session` -> 172 + 2 + 11 passed, 0 failed.
  - `cargo fmt --all --check` -> exit 0.
- Promoted `qa` -> `done`. The reconciliation test and the delta-slice test both pass on the
  merged base; no segfault remains.
