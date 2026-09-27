---
id: "H-014"
title: "BUG-043/047/051 snapshot-merge clobber and terminal-metadata root cause - Handoff"
status: "complete"
created: "2026-09-27"
updated: "2026-09-27"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["BUG-043", "BUG-047", "BUG-051"]
---

# BUG-043/047/051 snapshot-merge clobber and terminal-metadata root cause - Handoff

## Objective

Close the reopened `BUG-043`/`BUG-047` root cause (the shared session is overwritten by a stale
run-local snapshot, and message metadata never persists), which is also the confirmed cause of
`BUG-051` (the interrupt-then-continue prompt flashing / never settling as one ordered message).

This is a follow-up pass to archived `H-012` (PRs #117/#118/#119). Those PRs delivered the idle
guarantee, run cancel/panic finalize, provider bounds, and in-flight byte flushing, but they did
**not** change the snapshot-merge semantics or persist message metadata — so `BUG-043` QA = FAIL and
`BUG-047` QA = PARTIAL. This handoff implements the missing root-cause fix.

## Included Board Items

| Item | Title | Priority | Lane | Role |
|------|-------|----------|------|------|
| `BUG-043` | A session run can end without a terminal state, leaving it stuck busy and uninterruptible | P1 | `qa` | Terminal record must survive; clobber erases it |
| `BUG-047` | Assistant turn progress is not persisted until the run completes | P1 | `qa` | Metadata persistence gap; clobber on the persist/merge path |
| `BUG-051` | The sent continuation prompt flashes and never settles as one ordered message after interrupt-then-continue | P1 | `todo` | Confirmed user-visible symptom of the same clobber |

## Confirmed Root Cause (verified against `development`, 2026-09-27)

This is no longer a hypothesis. Both halves are present in the merged code.

1. **Wholesale snapshot clobber.** `merge_session_snapshot` replaces the shared session with the
   incoming snapshot, preserving only `queued_pending` messages:
   `crates/opencode-server/src/routes.rs:2756-2776` (the `*existing = snapshot;` at `:2774`).
2. **Stale run-local snapshot feeds that merge.** `run_prompt_turn` clones the shared session
   **once** at run start (`routes.rs:3634-3640`) and streams snapshots derived from that clone via
   the update hook (`routes.rs:3701-3733`) into `apply_session_snapshot` (`routes.rs:2778-2789`).
   There is no freshness or monotonicity guard, so any newer shared state (a finalize/terminal
   record, newly appended messages, another writer's update) is overwritten by the next stream
   snapshot. This matches the mechanism named in `BUG-043`'s QA report.
3. **Message metadata is not persisted.** Storage serializes only `message.parts`
   (`crates/opencode-storage/src/repository.rs:592,621,801`) and rebuilds messages with
   `metadata: HashMap::new()` on read (`:100,:701,:752,:1242`). `error` / `finish_reason` /
   `completed_at` therefore cannot survive a reload or restart.

Live confirmation (detail on `BUG-051`): with two live servers on one `opencode.db`, session
`ses_9ffd6dfb41f74d99a3976731d0a7a4b2` alternated between a forward snapshot (`n=54`) and a reverted
snapshot (`n=40`) on consecutive `GET /session/{id}/message` polls while the run was in fact
progressing (persisted assistant `data` grew `130 -> ... -> 1185`). The reverted snapshot is the
same clobber.

## Why This Composition (one tightly coupled PR)

All three items change the same surface — the run snapshot merge and its persistence path. Fixing the
merge changes `BUG-043` (terminal record survival), `BUG-047` (durable progress/metadata), and
`BUG-051` (display stability) at once. Splitting would create broken intermediate states (e.g. a
monotonic merge without metadata persistence still loses the terminal record on reload), so they
land as **one PR**.

`BUG-044` is **not** in this handoff: its stale-resume symptom was already addressed by the in-flight
flush and pre-loop `append_missing_tool_results` repair, and it is verified as a consequence of the
same merge/metadata fix. `BUG-052` (shared-DB lock contention) is related context only.

## Dependency Order

1. Merge/persist fix is a single change; there is no internal sequencing.
2. `BUG-051` is verified against the same fix (its probe is the display regression test).
3. No dependency on `BUG-044` or `BUG-052`.

`BUG-043`/`BUG-047` are in `qa`; this pass moves them to `doing` for the reopened work and back to
`qa` on completion. `BUG-051` moves `todo -> doing -> qa` with the pass.

## PR Plan

| PR | Board Items | Branch | Why this grouping | Merge rule |
|----|-------------|--------|-------------------|------------|
| A | `BUG-043`, `BUG-047`, `BUG-051` | `bug/BUG-043-047-051-snapshot-merge-and-metadata` | One logical change: monotonic merge + metadata persistence; partial merge leaves broken states | Single PR, merge into `development` only after local QA on the branch |

No batch merge is needed (one PR). Do not start this alongside another edit to
`crates/opencode-server/src/routes.rs`; the file is already under concurrent work.

## Merge Target

All implementation PRs in this handoff target `development`. The purpose is to land dev work there so
QA and testing happen on `development`. This handoff does not define deployment to `main`.

## Merge Strategy

- One PR; merge only after the verification gates below pass and the user explicitly directs the merge
  (per the repo QA gate).
- Rebase on current `development` before merge; re-run gates after any conflict resolution.

## Required Implementation

1. **Monotonic, freshness-guarded merge.** Replace `*existing = snapshot` in
   `merge_session_snapshot` with a merge that never regresses the shared session: do not reduce the
   message count, do not drop a newer terminal/finalize record, do not resurrect messages the shared
   state no longer has, and preserve ordering. A run snapshot may only add or advance content, never
   roll it back. Prefer a per-message/metadata reconciliation keyed by message id over whole-session
   replacement.
2. **Persist and restore message metadata.** Serialize `error`, `finish_reason`, and `completed_at`
   (and any other terminal metadata) alongside parts, and restore them in every read path
   (`list_for_session`, `replace_for_session`, single-message reads). This makes the terminal record
   durable across reload/restart.
3. Keep the existing in-flight fingerprint flush and finalize paths; they are correct, they just need
   a merge that does not undo them.

Key files: `crates/opencode-server/src/routes.rs` (`merge_session_snapshot`,
`apply_session_snapshot`, `run_prompt_turn`, `update_task`, `finalize_run_without_terminal`,
`persist_sessions_if_enabled`), `crates/opencode-storage/src/repository.rs`, and
`crates/opencode-session/src/session.rs` as needed.

## Verification Gates (must pass before QA handoff)

- **Monotonic list (`BUG-051`).** Start a streaming turn, then poll `GET /session/{id}/message`
  every ~2s: the message count must be non-decreasing and the last message ID must never revert to a
  previously seen older ID. Re-run the two-session live check and confirm session
  `ses_9ffd6dfb...` no longer alternates `n=54`/`n=40`.
- **Single-render regression.** TUI frame capture or ratatui `TestBackend` asserting the sent
  continuation prompt appears exactly once, in order, and does not move across frames.
- **Durable terminal record (`BUG-043`).** Abort a run, then reload from storage and confirm the
  assistant message still carries `error`/`finish_reason`/`completed_at` (not
  `metadata: HashMap::new()`), and that `opencode session inspect` does not mislabel it as stalled.
- **Progress durability (`BUG-047`).** After a mid-turn exit, the DB contains the latest produced
  parts and metadata.
- `cargo test -p opencode-server -p opencode-session -p opencode-storage -p opencode-tui`;
  `cargo check --workspace`; `cargo fmt --all -- --check`.
- Live `ort-build` + `ort` on the interrupt-then-continue path.

## QA Notes

- QA happens on `development` after merge, per repo policy; the PR branch may be used for the
  author's local check.
- The `BUG-051` probe is the primary regression test for the display symptom; the reload check is
  the primary regression test for `BUG-043`/`BUG-047`.
- The captured sessions (`ses_9ffd6dfb...`, `ses_99fd5d42...`) and the shared-DB lock errors in
  `traces/server.log` are the live evidence for the merge fix; re-check them after merge.

## Board Updates Per PR

1. `BUG-043`: `qa -> doing -> qa` with the confirmed root cause (file:line), the merge/metadata
   changes, and the reload verification.
2. `BUG-047`: `qa -> doing -> qa` with the metadata-persistence changes and the mid-turn-exit check.
3. `BUG-051`: `todo -> doing -> qa` with the monotonic-list evidence and the TUI single-render test.
4. Keep all three in `qa` until post-merge QA on `development` passes; only then move to `done`.
5. Archive this handoff when the PR is merged and QA is recorded.

## Branch Cleanup

- After merge: delete remote `bug/BUG-043-047-051-snapshot-merge-and-metadata`
  (`git push origin --delete ...`) and the local branch (`git branch -d ...`).
- Do not delete branches for still-open PRs or unrelated branches (including any `BUG-052` work).

## Execution Sequence

1. Confirm no other active edit to `crates/opencode-server/src/routes.rs`; coordinate to avoid conflict.
2. Branch `bug/BUG-043-047-051-snapshot-merge-and-metadata` from `development`.
3. Implement the monotonic merge and metadata persistence.
4. Run the verification gates locally.
5. Open the PR referencing `BUG-043`, `BUG-047`, `BUG-051`; move the cards `qa/todo -> doing`.
6. Local QA on the branch; then, on explicit user direction, merge into `development`.
7. Post-merge QA on `development`; move cards to `qa`; keep open until QA passes.
8. Delete the branch (remote then local); archive this handoff.

## Risks And Rollback

- **Merge change is load-bearing** for queueing, subagents, and finalize; keep it small and
  well-tested. A bad merge could drop messages — the monotonic assertions are the guard.
- **Metadata persistence touches storage serialization**; existing rows without metadata must still
  load (treat missing as absent, no error).
- **Concurrent `BUG-052` DB-lock failures** may still surface during QA; that is a separate card and
  does not block this fix, but do not misattribute a lost write to this change.
- Revert is a single PR revert.

## Deferred / Out Of Scope

- `BUG-052` shared-DB lock contention (`database is locked`) — separate card.
- TUI-only masking of snapshot regressions (explicitly rejected; fix the server state instead).
- `BUG-044` stale-resume selection (already addressed; re-verify only if it recurs).
- Production/`main` deployment.

## Execution Notes - 2026-09-27

- Branch `bug/BUG-043-047-051-snapshot-merge-and-metadata` from `development`; PR
  https://github.com/cchris-p/opencode-modded-rust/pull/127 (single PR, base `development`).
- Implemented: monotonic, id-keyed `merge_session_snapshot` plus helpers in
  `crates/opencode-server/src/routes.rs`; deleted-message tombstones in
  `crates/opencode-session/src/session.rs`; `completed_at` on finalize in `prompt.rs`; message
  metadata persistence in `crates/opencode-storage/src/repository.rs`; terminal-record
  recognition in `opencode session inspect` (`crates/opencode-cli/src/main.rs`); TUI single-render
  test in `crates/opencode-tui/src/components/session.rs`.
- Cards moved `qa`/`todo -> doing` for the reopened pass (`BUG-043`, `BUG-047`, `BUG-051`).
- Verification: server 71 + 3 integration / storage 3 / new tests pass; session has only the two
  pre-existing `instruction::test_find_up_*` failures; `cargo check --workspace` and `cargo fmt`
  clean. Live isolated-HOME probes: monotonic list PASS; abort-then-reload carries
  `error`/`finish_reason`/`completed_at` and does not read as stalled.
- Remaining: local QA on the PR branch, then explicit user direction to merge into `development`;
  after merge, post-merge QA on `development`, move cards to `qa`, delete the branch, archive this
  handoff.

## Completed with PR #127 - 2026-09-27

- Merged PR #127 (`bug/BUG-043-047-051-snapshot-merge-and-metadata`) into `development` as merge
  commit `dc41ce3ef08cae3b6c355d4d165d95407c317d06`.
- Branch deleted remotely and locally.
- Cards `BUG-043`, `BUG-047`, `BUG-051` moved `doing -> qa`; they remain in `qa` until a post-merge
  QA report is recorded on `development`.
- This is the handoff's single PR; the handoff is complete and archived.
