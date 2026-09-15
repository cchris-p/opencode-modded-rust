---
id: "H-001"
title: "BUG-007..BUG-009 V1 tool-quality hardening - Handoff"
status: "in_progress"
created: "2026-09-10"
updated: "2026-09-13"
owner: ""
target: ""
blocked_reason: ""
needs_human: ""
items: ["BUG-007", "BUG-008", "BUG-009"]
---

# BUG-007..BUG-009 V1 Tool-Quality Hardening - Handoff

## Objective

Make the three tool-quality/context defects surfaced by the BUG-006 QA session (`summarize-workspace-files.md`, session `ses_0ffc9b44f96944039f8621c4d980d11b`) production-quality on the agentic coding-session path, so the V1 daily-driver no longer wastes turns on broken or misleading tool output.

Parent epic: `PHASE-001` V1 daily-driver hardening (tracking parent only; no PR of its own).

## Included Board Items

- `BUG-007` `ls` tool returns partial/misleading directory listings
- `BUG-008` `batch` tool unusable on the session path (schema mismatch + registry not wired)
- `BUG-009` Instruction files (AGENTS.md) re-injected on every read tool call

Excluded: `PHASE-001..004` are grouping epics and get no PRs. The other `PHASE-001` children (`FEAT-013`, `START-018`) are out of scope for this handoff.

## Confirmations applied before writing

Refinement gate ran over the whole group and passed after these decisions were recorded on the cards:

- `BUG-007` semantics: **bounded top-level listing** (immediate children; per-level truncation only; empty dirs included; `ls` retained).
- `BUG-008` disposition: **keep `batch` and fix it** (schema/deserializer alignment + wire registry/agent into the session `ToolContext`).
- `BUG-009` mechanism: **session-tracked loaded-instruction set** on the `ToolContext`, populated from read results' `loaded` metadata.

## Execution Status

- `BUG-007` - **in flight**. Branch `bug/BUG-007-ls-top-level-listing`, PR #31 open against `development`, card in `qa`. Live `ls` verification passed (all top-level directories listed, no `bash` fallback). Awaiting user local QA.
- `BUG-008` - not started. Blocked on the keep/fix decision being implemented; card still in `todo`.
- `BUG-009` - not started. Deferred pending local-model context-pressure observation; card still in `todo`.

## Dependencies and Ordering

- `BUG-008` and `BUG-009` both edit the session `ToolContext` construction in `crates/opencode-session/src/prompt.rs` (`prompt.rs:1338-1348`). To avoid a rebase/merge collision on the same lines, land `BUG-008` first, then `BUG-009`.
- `BUG-009` is layered on the `ToolContext` changes established by `BUG-008` (**inferred ordering**; rationale: shared construction site, layered context additions).
- `BUG-007` touches only `crates/opencode-tool/src/ls.rs` and is **parallel-safe** with `BUG-008` (disjoint files). It is sequenced after `BUG-008` below only for a single clean merge order.
- No other card states a dependency on these items.

## PR Plan

| PR | Board Items | Branch | Why this grouping | Merge rule |
| --- | --- | --- | --- | --- |
| 1 | `BUG-008` | `bug/BUG-008-batch-session-path` | Fixes both `batch` defects and establishes the session `ToolContext` wiring (registry + resolved agent name). Touches `batch.rs` and `prompt.rs`. | Merge into `development` when the schema/deserializer agree and a session-path test shows `batch` executing. |
| 2 | `BUG-009` | `bug/BUG-009-instruction-dedup` | Rejection dedup layers on the `ToolContext` plumbing from PR 1; same `prompt.rs` region, so it must follow PR 1. | Merge into `development` after PR 1 when a multi-read session injects each instruction file once. |
| 3 | `BUG-007` | `bug/BUG-007-ls-top-level-listing` | Isolated `ls` behavior change; disjoint from PR 1/2 files. | Merge into `development` when `ls` returns all immediate children and the regression test passes. |

Every PR gets its own branch and its own PR targeting `development`. Do not bundle.

## Implementation Steps

### Phase 1 - `BUG-008` batch on the session path (PR 1)

Board item: `BUG-008`.

1. Align the advertised schema with the deserializer: choose one canonical key for the batch call list and make `BatchTool::parameters()` (`crates/opencode-tool/src/batch.rs:52-79`), `BatchParams` (`batch.rs:11-14`), and tests agree.
2. Wire the session path: in `crates/opencode-session/src/prompt.rs` (`prompt.rs:1338-1348`) set `.with_registry(registry)` on the `ToolContext` (mirror the CLI at `crates/opencode-cli/src/main.rs:3824`), and set the resolved agent name instead of `.with_agent(String::new())`.
3. Add a regression test that exercises `batch` through the session prompt path (not just the CLI) and asserts contained tool calls execute.
4. Verification gate: `cargo test -p opencode-tool -p opencode-session`; confirm `ToolContext` carries registry + agent.

Dependency note: this PR is first because it establishes the `ToolContext` wiring reused by `BUG-009`.

### Phase 2 - `BUG-009` instruction dedup (PR 2)

Board item: `BUG-009`. Requires PR 1 merged.

1. Add a session-keyed loaded-instruction set to the `ToolContext` (extend the construction touched in Phase 1).
2. Have `read.rs` populate it from the instruction file paths it attaches (already emitted in `loaded` metadata, `crates/opencode-tool/src/read.rs:423-447`) and skip instruction files already in the set (`read.rs:425-437`, `read.rs:479-529`).
3. Preserve first-time injection; still surface content when an instruction file changes.
4. Add a regression test: multiple reads under one instruction file => a single injection.
5. Verification gate: `cargo test -p opencode-tool read`.

### Phase 3 - `BUG-007` bounded top-level `ls` (PR 3, parallel-safe with Phase 1)

Board item: `BUG-007`. Independent of PR 1/2 files; may run any time after the handoff starts.

1. Change `ls` to list all immediate children of the requested directory (subdirectories and files), including directories that contain no files.
2. Remove the global recursive file cap (`crates/opencode-tool/src/ls.rs:36`, `ls.rs:175-180`) as the thing that governs whether top-level children appear; any cap must be per-listing and must not hide the directory's own children.
3. Report truncation scoped to what was truncated, not just a global boolean (`ls.rs:283-293`).
4. Add a regression test with more than 100 files asserting all top-level children are present.
5. Verification gate: `cargo test -p opencode-tool ls`.

## Parallel-Safe Steps

- Phase 3 (`BUG-007`) is parallel-safe with Phase 1 (`BUG-008`): disjoint files (`ls.rs` vs `batch.rs` + `prompt.rs`).
- Phases 1 and 2 are **not** parallel-safe with each other (same `prompt.rs` region); keep them sequential.

## Verification / Completion Gates

- Before starting Phase 2: Phase 1 merged to `development` (its `ToolContext` wiring present).
- Before starting Phase 3: none required (parallel-safe).
- Before moving any card out of `qa`: user verifies on a fresh `ort-build`/`ort` server (FEAT-014 rotates the server) for that card's workflow.
- Per-item acceptance criteria and recommended verification live on the cards; this handoff does not restate them in full.

## Implementation Sequence

1. `BUG-008` - branch `bug/BUG-008-batch-session-path` -> PR 1 -> merge to `development`.
2. `BUG-009` - branch `bug/BUG-009-instruction-dedup` -> PR 2 -> rebase on `development` after PR 1 -> merge.
3. `BUG-007` - branch `bug/BUG-007-ls-top-level-listing` -> PR 3 (may start alongside step 1) -> merge to `development`.

## Merge Target

All implementation PRs in this handoff target `development`, so QA and testing happen on `development`. This handoff does not define deployment to `main`.

## Merge Strategy

- Each PR merges independently; no batch/atomic merge is required.
- PR 2 must be rebased on `development` after PR 1 merges (shared `prompt.rs` region).
- PR 3 may merge before, between, or after PR 1/2.
- Do not merge any PR until the item's local QA on the checked-out branch is confirmed by the user.

## QA Notes

- Rebuild before testing: run `ort-build` then `ort` (FEAT-014 ensures a fresh server on the next port).
- `BUG-008`: prompt the agent to read several files at once; `batch` must succeed (no `missing field` error, no `Tool registry not available`).
- `BUG-009`: read several files under this repo; `AGENTS.md` must appear at most once in the session, not per read.
- `BUG-007`: prompt "list the top-level directories in this workspace"; all `crates/` entries must appear without a `bash` fallback.

## Branch Cleanup

- Delete each remote branch after its PR merges: `git push origin --delete <branch>`.
- Delete each local branch after merge: `git branch -d <branch>`.
- Do not delete unrelated local or remote branches.

## Board Maintenance

- Refresh this handoff's `updated` on meaningful changes; set `status: "in_progress"` when execution starts and `complete` only when all three PRs are merged.
- Set `needs_human` only if a real human decision is required (e.g. conflicting edits in `prompt.rs` from concurrent work).
- When complete, move the handoff to `handoffs/archive/` following the repo convention and move the three cards to `done` after their QA.
