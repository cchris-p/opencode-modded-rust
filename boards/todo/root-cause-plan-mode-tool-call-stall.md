---
id: "BUG-023"
title: "Root-cause why the plan-mode session stalled after tool calls without results"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-21"
---

# Root-cause why the plan-mode session stalled after tool calls without results

## Summary

`BUG-016` observed a real failure and shipped a repair guard, but it did not establish *why* the
session stopped after emitting tool calls. The guard makes the transcript self-consistent by writing
error results for unresolved tool calls; it does not explain the underlying stall. This item is the
focused root-cause investigation for that stall, using the original session export as evidence.

This is deliberately narrower than `BUG-016`: `BUG-016` fixed the symptom (unresolved tool-call
parts are always resolved). This card asks why execution never produced results in the first place.

## Evidence

- Transcript: [`new-session-2026-09-19t0409592215950000.md`](../../docs/transcripts/new-session-2026-09-19t0409592215950000.md)
- Session ID: `ses_9b85fa20680b4dbfa7d2a2507335a4c1`.
- User prompt: `Give me all the skills board items`.
- Mode/model: `Plan · deepseek/deepseek-v4-flash`.
- The assistant emitted `ls` plus two `grep` tool calls, then the export ends with no tool results
  and no final answer.
- Export timing is suspicious: **created** `2026-09-19T04:09:59.221` and **updated**
  `2026-09-19T04:10:00.955` — a ~1.7s window. The export may have captured a mid-run state rather
  than a proven terminal stall.

Related but separate symptom (do not conflate):

- `docs/transcripts/tool-call-issue.md` (`ses_33afb883…`, 2026-09-21) fails differently: the provider
  rejects a follow-up with `400 Invalid assistant message: content or tool_calls must be set`. That
  is a persisted-message-shape problem, not the silent stall. It may share a root cause around how
  tool-call turns are reconstructed for the next provider request.

## What BUG-016 already fixed

- The prompt loop now calls `append_missing_tool_results` after `execute_tool_calls`:
  `crates/opencode-session/src/prompt.rs:1365-1387`.
- `unresolved_tool_call_ids` / `append_missing_tool_results`:
  `crates/opencode-session/src/prompt.rs:1515-1560`.
- Abort cleanup reuses the same detection (`mark_aborted`, `abort_pending_tool_calls`):
  `crates/opencode-session/src/prompt.rs:1403-1406`.
- Regression coverage: `append_missing_tool_results_repairs_unresolved_calls`
  (`prompt.rs:4569`) and `abort_pending_tool_calls_marks_unresolved_calls_as_error` (`prompt.rs:4505`).

That guard explains why the *export* should now be self-consistent. It does not explain the
1.7s stall on the pre-guard binary.

## Open questions to answer

1. Did the run actually terminate, or was the transcript exported while the run was still in flight?
   - Determine how/when transcript export snapshots session state, and whether it can capture an
     assistant turn before tool results are appended.
2. Did `execute_tool_calls` start at all? If not, did the loop exit or the spawned server task get
   dropped before reaching it (`crates/opencode-server/src/routes.rs` spawn site for the prompt task)?
3. If tool execution started, did it hang on a permission/ask callback that never resolved in plan
   mode? Note `ToolContext` is built with `with_agent(String::new())`
   (`prompt.rs:1347`) — verify whether plan-mode permission/ask resolution depends on a real agent
   name/session identity here.
4. After `continue` (`prompt.rs:1387`), did the provider re-request fail or return an empty stream
   that was treated as completion without a durable error?
5. Is this reproducible at all on the current binary, or was it a one-off provider/session edge case
   specific to DeepSeek plus plan mode?

## Scope

- Establish, with evidence, the point in the prompt/tool loop where the 2026-09-19 session stopped.
- Determine whether the observed export was a true stall or an export-timing artifact.
- Fix the confirmed root cause if it is a real runtime defect.
- If the stall is an export timing artifact, fix export to snapshot completed state (or clearly mark
  in-progress state) rather than leaving a tool-call-only transcript.
- Add a regression test at the confirmed failure point, not only at the guard.

## Non-goals

- Re-litigating or reverting the `BUG-016` repair guard; it stays.
- Changing plan mode into build mode or permitting edit tools in plan mode.
- Broad provider transport rewrites beyond the confirmed root cause.
- Root-causing the separate `tool-call-issue.md` `400` symptom unless evidence shows a shared cause.

## Done when

- There is a written, evidence-backed explanation of why the audited session stopped after tool
  calls, including which loop stage it stopped at.
- Either a targeted fix for the confirmed cause is merged, or the item explicitly concludes the
  original export was an export-timing artifact and the export path is corrected or documented.
- A test reproduces the confirmed failure mode (or the export-timing case) and fails before the fix.
- The `Give me all the skills board items` prompt completes in `ort` plan mode with a final answer.

## Recommended verification

- Inspect transcript export timing in the storage/export path and add a test that exports mid-run and
  asserts the resulting state is either complete or explicitly marked in-progress.
- Build a session-loop test that feeds a provider stream ending in tool calls and asserts
  `execute_tool_calls` is reached and results are appended before any loop exit.
- `cargo test -p opencode-session`
- `cargo check -p opencode-session -p opencode-server -p opencode-tui`
- Live: `ort-build`, then `ort`, run the original prompt in plan mode, and export the transcript.

## Investigation - 2026-09-21 (confidence-ranked)

Method: read the Rust product DB directly, trace the TUI export path and the server persistence
path, and cross-reference the other exported transcripts. Findings are labelled with a confidence
estimate for how strongly the current evidence supports them.

### Confirmed evidence

Storage correctness:

- The Rust product DB on macOS is `~/Library/Application Support/opencode/opencode.db`, from
  `dirs::data_local_dir()` (`crates/opencode-storage/src/database.rs:138-144`). `BUG-016`'s storage
  caveat has this inverted: it treated `~/.local/share/...` as the Rust DB and `~/Library/Application
  Support/...` as vanilla.
- The audited session row **is present** in that DB: `ses_9b85fa20680b4dbfa7d2a2507335a4c1`,
  created `2026-09-19T04:09:59.221Z`, with `updated_at == created_at`.
- That session has **0 rows in `messages`** (and 0 in `parts`). Everything shown in the export lives
  only in the TUI's memory.
- The vanilla OpenCode DB (`~/.local/share/opencode/opencode.db`) uses a different schema
  (`session`/`message`/`part`) and contains no rows for this session.

Export path:

- Transcripts are built from the TUI's in-memory session/message maps, not from storage:
  `build_session_transcript` reads `self.context.session.read()`
  (`crates/opencode-tui/src/app/app.rs:2241-2277`). So an export faithfully shows whatever the TUI
  had received through `session.updated` events, including messages that were never persisted.

Persistence path:

- A prompt runs inside a spawned task (`crates/opencode-server/src/routes.rs:1747`); progress is
  published to the in-memory session manager through an update hook (`routes.rs:1792-1814`).
- Persistence happens **after the run finishes** via `persist_sessions_if_enabled(&task_state)`
  (`crates/opencode-server/src/routes.rs:2066`).
- Persistence is a full-snapshot rewrite: for each session it deletes all messages then re-creates
  them, and deletes any persisted session that is not in the in-memory manager
  (`crates/opencode-server/src/server.rs:188-229`).
- `create_session` persists a row-only session (`routes.rs:413-438`). A session that never reaches
  the post-run persist is therefore exactly "session row, zero messages, `updated_at == created_at`".

Timing:

- The audited session ran at `2026-09-19T04:09:59Z`. The `BUG-016` guard merged at
  `2026-09-19T04:38:03Z` (`933784d`). The session ran on a **pre-guard** binary, so unresolved
  tool-call state was still possible.

Recurrence:

- `docs/archive/session-tool-call-summary-order-2026-09-15.md`
  (`ses_cf186e35b53a42debd3dcd1375e01707`) is the same session as an **empty-message row** in the
  Rust DB. It shows the assistant emitting `grep` + `bash` tool calls with no results, then the user
  saying "continue I think you're stuck", then repeated provider `400`:
  `An assistant message with 'tool_calls' must be followed by tool messages responding to each
  'tool_call_id'`.
- Empty-message sessions recur across `2026-09-15`, `2026-09-19`, and `2026-09-21`; the 09-21 cluster
  has 5 sessions within ~15 minutes.
- Sessions are only created when a prompt is submitted (no idle-creation path found), so empty rows
  are runs that never persisted, not unused sessions.

Concurrent-writer data loss (likely a separate defect):

- Multiple `opencode serve`/`tui` processes run at once against the one global DB (observed via
  `pgrep`), each with its own in-memory manager.
- While this investigation was running, the DB changed underneath it: session count `19 -> 16` and
  total messages `477 -> 210` between two reads minutes apart, with no action by this audit.
- Because sync is a full snapshot plus "delete anything not in my manager"
  (`server.rs:200-207`), a server started earlier can delete sessions/messages created by a server
  started later. This deserves its own card.

### Hypotheses with confidence

- **H1 (98%) - The export is a TUI in-memory snapshot, not a DB read.** Evidence: `app.rs:2241`.
- **H2 (95%) - The run never persisted because it never completed.** The DB has the session row but
  no messages and `updated_at == created_at`; post-run persist is the only message-persist path, so a
  completed run would have left messages.
- **H3 (90%) - The stall was unresolved tool-call state on a pre-guard binary.** Session predates the
  guard by ~28 minutes; the 09-15 recurrence shows the same state causing provider `400`s.
- **H4 (80%) - The stall is provider/loop-side (the turn stopped after emitting tool calls) and not
  plan-mode-specific.** The same signature occurs in Build mode on 09-15 with non-plan tools
  (`grep`, `bash`).
- **H5 (75%) - Full-snapshot persistence is a structural durability hole.** Any hung or torn-down run
  leaves an unpersisted session, and concurrent syncs can delete sessions/messages. True
  independently of the stall.
- **H6 (20%) - Plan-mode permission/ask hang.** `ToolContext` is built with
  `with_agent(String::new())` (`crates/opencode-session/src/prompt.rs:1347`), which is suspicious, but
  plan mode allows `ls`/`grep` and no ask is visible, and 09-15 was Build mode, so this cannot be the
  general cause.
- **H7 (10%) - Pure export-timing artifact with a later successful run.** Contradicted by the empty
  DB; if the run had completed, messages would have been persisted.
- **H8 (30%) - Lower-reproducibility provider edge case** specific to DeepSeek streaming; cannot be
  excluded without logs.

### Evidence that would raise confidence

- Logs for the 2026-09-19 run were not found, and no persist-on-shutdown path was located. Capturing
  logs for a reproduction would separate H4 (hang) from H5 (torn-down persistence).
- Reproduce on the current post-guard binary: if it no longer stalls, H3 is confirmed as the fix; if
  the session still fails to persist, H5 is confirmed.
- Instrument `session_prompt` to record whether `execute_tool_calls` is reached and whether
  `persist_sessions_if_enabled` runs at `routes.rs:2066`.

## Related Items

- `BUG-016` Plan-mode session stalls after tool calls without tool results - symptom fix and prior
  investigation; this item supplies the missing root cause.
- `BUG-019` Escape does not interrupt the running session - abort/cancel path intersects with how the
  loop exits and leaves resolved tool-call state.
- `BUG-012` Session summary runs before tool results - another ordering defect in the same loop.
- `BUG-006` DeepSeek tool loop reasoning passback and split toolcall - provider-specific tool-call
  handling.
- `BUG-005` OpenAI-compatible chat providers reject requests once tools are attached - related
  persisted-message-shape issues.

## Notes

- Transcript references live under `docs/transcripts/`; keep new exports there and link them by
  relative path from board items.
- `tool-call-issue.md` under `docs/transcripts/` is the second reference failure and should be
  promoted to its own card if the `400 Invalid assistant message` symptom recurs.
- The concurrent-writer deletion of sessions/messages (`sync_sessions_to_storage` full snapshot plus
  stale deletion, `server.rs:188-229`) is a separate, evidence-backed defect and should get its own
  card rather than being folded into this root-cause item.
