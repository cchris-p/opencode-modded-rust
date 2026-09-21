---
id: "BUG-025"
title: "Concurrent servers delete each other's sessions and messages via full-snapshot DB sync"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/runtime-lifecycle.md"
status: "todo"
created: "2026-09-21"
---

# Concurrent servers delete each other's sessions and messages via full-snapshot DB sync

## Summary

Every `opencode serve` process persists sessions with a full-snapshot rewrite of one shared SQLite
database, and deletes any persisted session that is not in its own in-memory session manager. When
more than one server is running against the same DB (normal with multiple `ort`/TUI instances), a
server that started earlier does not know about sessions created by a server that started later, so
its next sync deletes the later server's sessions and messages as "stale". This is silent data loss.

Observed live during a read-only audit on 2026-09-21: with several `opencode serve`/`tui` processes
running, the Rust product DB dropped from **19 sessions to 16** and from **477 messages to 210**
between two reads minutes apart, with no action taken by the audit.

## Evidence

- Multiple servers run concurrently against the one global DB (observed with `pgrep -fl opencode`,
  e.g. `opencode tui` + `opencode serve` pairs).
- Persistence is a full snapshot per session, plus destructive stale deletion:
  `crates/opencode-server/src/server.rs:188-229`.
  - `message_repo.delete_for_session(&stale.id)` then `session_repo.delete(&stale.id)` for any
    persisted session not in the in-memory manager (`server.rs:200-207`).
  - For each in-memory session: `session_repo.create`/`update`, then
    `message_repo.delete_for_session` followed by a `message_repo.create` loop (`server.rs:216-225`),
    with no enclosing transaction across the delete and insert loop.
- The in-memory manager is populated from storage only at server start
  (`load_sessions_from_storage`, `server.rs:152`), so each running server's view is a snapshot taken
  at its own startup and is not kept in sync with other servers' writes.
- `sync_sessions_to_storage` is called from many route handlers via
  `persist_sessions_if_enabled` (`crates/opencode-server/src/routes.rs:252-256`), so an older server
  will trigger a destructive snapshot on its next API activity.
- There is no DB path separation per workspace/server: `dirs::data_local_dir()/opencode/opencode.db`
  is global (`crates/opencode-storage/src/database.rs:138-144`).

### User-visible confirmation: resume hint points at a deleted session (2026-09-21)

- User ran the `FEAT-033` resume hint produced on a normal TUI exit:
  `ort --session ses_7de60212abab46dd8fd247818ddec5e4` from
  `/home/admin-xx/repos/opencode-modded-rust`.
- The TUI opened an **empty session with empty contents** instead of erroring.
- DB audit (`~/.local/share/opencode/opencode.db`): **no row exists** for
  `ses_7de60212abab46dd8fd247818ddec5e4` in either `sessions` (Rust product) or `session` (vanilla);
  the only occurrence anywhere in the DB is this bug report's own prompt text. The `sessions` table
  now holds **8 rows**, down further from the 16 recorded above, all in
  `/home/admin-xx/repos/opencode-modded-rust`.
- This confirms the data-loss path end to end: a session the user was actively working in is gone from
  storage, and the resume command for it silently yields an empty session.
- Secondary surface gap (not the root cause): the resume path swallows the `get_session` failure. In
  `crates/opencode-tui/src/app/app.rs:256-259`, `App::new` does
  `let _ = app.sync_session_from_server(&session_id)` and then unconditionally calls
  `ensure_session_view(&session_id)`; `sync_session_from_server` (`app.rs:3628-3664`) propagates the
  non-2xx error from `ApiClient::get_session` (`crates/opencode-tui/src/api.rs:419-428`), so the
  error is dropped and a view is created for a session that does not exist. A missing/deleted session
  should surface a not-found error rather than open an empty session.

## Investigation Notes (2026-09-21): is the session deleted on exit?

User question after the report above: this is related to the "last session link" printed at exit, and
does exiting delete the session?

**Answer: exit is not the deleter.** There is no delete-on-exit code path. The owner server is
`SIGKILL`ed on exit, and the session is removed by a *different, still-running* server when that
server next writes its stale full-snapshot.

Code evidence for "exit does not delete":

- Normal exit prints the resume hint from `TuiExit::Exit` (`crates/opencode-cli/src/main.rs:909-911`)
  and then `LocalTuiServer` drops. Its `impl Drop` only does `child.kill()` / `child.wait()`
  (`main.rs:978-988`) — a `SIGKILL` of the `opencode serve` child, with no graceful flush and no API
  call.
- The only route that removes a session is `delete_session` -> `manager.delete` +
  `persist_sessions_if_enabled` (`crates/opencode-server/src/routes.rs:519-533`), and the only caller
  is the TUI's explicit session-list delete action (`crates/opencode-tui/src/app/app.rs:1465`). Nothing
  invokes it on exit, detach, or launch.
- Deletion instead happens in `sync_sessions_to_storage` (`crates/opencode-server/src/server.rs:188-229`):
  every persisted session **not in the caller's in-memory snapshot** is deleted
  (`server.rs:200-207`). That snapshot is loaded once at server start
  (`server.rs:152`, `server.rs:168-186`), so an older server's snapshot is permanently stale.

Live evidence captured during this investigation:

- Two concurrent Rust servers were running against the one global DB:
  - `opencode tui/serve --port 3188` (PIDs 2744338/2744371), executable reported as `(deleted)` (old
    rebuild still running), started `2026-09-21 11:18:50`.
  - `opencode tui/serve --port 3187` (PIDs 3808719/3808736), started `2026-09-21 17:31:42` — the run
    in the user report.
- `GET http://127.0.0.1:3188/session` returned **5** sessions. Its snapshot includes
  `ses_ca0d8291cd4f44639fce6aa5f5d8dc23` (`/home/admin-xx/apps/relics_notes`), which is **not in the
  DB at all**, and omits 3 of the 8 sessions currently in the DB.
- `GET http://127.0.0.1:3188/session/ses_7de60212abab46dd8fd247818ddec5e4` returned **404** — the
  stale server has no knowledge of the reported session.
- DB state during the audit: `sessions` = 8 rows, `messages` = 915, `parts` = 0, and **zero** orphan
  message/part rows. The matched deletes show `sync_sessions_to_storage` completed its
  message-then-session removal pair.

Conclusion:

- "Deleted when I exit" is a timing misattribution. The resume target had already been pruned by the
  concurrent stale writer; the user noticed it at exit/relaunch because that is when the id is
  exercised.
- Mutual clobbering is ongoing and bidirectional: when the 3188 server next syncs it will re-create
  its unknown-to-the-DB `ses_ca0d8291` **and** delete the several sessions it does not know about,
  including the currently active one; the 3187 server will later delete `ses_ca0d8291` again.
- The "last session link" (`FEAT-033` resume hint) is the **exposure surface, not the cause**: it
  publishes an exact session id that any stale writer may delete, and `--session` then silently opens
  an empty session because the 404 is swallowed (see "Secondary surface gap" above).
- A distinct exit-adjacent loss also exists but does not remove the session row: the `SIGKILL` on exit
  (`main.rs:978-988`) can drop message state that had not yet been flushed, so a surviving session can
  appear empty. The reported case is stronger than that — the whole `sessions` row is gone.

Recommended follow-ups (beyond this card's root fix): make direct load/`--session` fail loudly on a
missing or deleted session instead of opening an empty view (`FEAT-023`); avoid `SIGKILL`-on-exit
without a final flush; consider per-workspace DB scoping so two servers cannot share one destructive
snapshot.

## Why this exists

The runtime invariant `invariants/runtime-lifecycle.md` states that the runtime owns lifecycle state
and that leaving a session view must not implicitly cancel execution. A second, stale server process
is able to erase another server's durable session state. Durability should not depend on which
server instance happens to call sync last.

## Scope

- Stop `sync_sessions_to_storage` from deleting sessions/messages that belong to another live server
  or that it did not create.
- Make persistence additive or scoped rather than a global full-snapshot rewrite.
- Ensure a single-server setup still persists correctly and reloads on restart.
- Preserve message ordering and content exactly across syncs.
- Do not delete user sessions on server restart or on stale-snapshot detection.

## Non-goals

- Redesigning the session schema or migrating existing data beyond what is needed to avoid loss.
- Reworking TUI session display or message streaming.
- Locking every session read/write behind a global mutex if a scoping or upsert approach suffices.

## Done when

- Two servers running against the same DB can each create and persist sessions without deleting the
  other's data.
- A sync from a server with a stale in-memory view never removes sessions or messages that exist in
  storage but not in its snapshot.
- Message writes are atomic (transactional) so an interrupted sync cannot leave a session with
  partially deleted messages.
- A regression test starts two managers/states against one test DB, writes disjoint sessions, syncs
  both, and asserts all sessions and messages survive.

## Proposed Fix (2026-09-21 investigation)

The investigation above localizes the destructive behavior to one loop and confirms the storage layer
already has the primitives for a surgical fix: `SessionRepository::delete`
(`crates/opencode-storage/src/repository.rs:396`), `MessageRepository::delete_for_session`
(`repository.rs:769`), and `MessageRepository::upsert` (`repository.rs:620`). No transaction is used
anywhere in storage today.

Recommended change - make snapshot sync upsert-only and own deletion explicitly:

1. Remove the stale-deletion loop in `sync_sessions_to_storage`
   (`crates/opencode-server/src/server.rs:202-207`). The sync must upsert each in-memory session and
   never delete a persisted row that is absent from its snapshot.
2. Own deletion in the explicit delete path. `delete_session` (`routes.rs:519-533`) already calls
   `manager.delete`, which cascades to children (`session.rs:1328-1354`); make that route delete the
   session row and its messages directly (`session_repo.delete` + `message_repo.delete_for_session`)
   instead of relying on the next snapshot diff.
3. Make the per-session message replace atomic/additive. Wrap the `delete_for_session` + `create` loop
   (`server.rs:222-225`) in a transaction, or switch it to `MessageRepository::upsert`
   (`repository.rs:620`). Without a transaction a crash mid-loop leaves a half-deleted session.
4. Optional hardening: guard `session_repo.update` (`repository.rs:316`) with `updated_at` so a stale
   server cannot overwrite a newer row written by another server.

Secondary, independent surface fix: stop the TUI from swallowing the missing-session error on
`--session`/resume (`crates/opencode-tui/src/app/app.rs:256-259`,
`crates/opencode-tui/src/api.rs:419-428`) and surface a not-found error instead of an empty session.

Immediate mitigation (not a fix): kill leftover `opencode tui`/`serve` pairs so only one server writes
the shared DB. The stale pair on port 3188 was running a `(deleted)` binary.

Confidence:

- Stops the reported symptom (one server deleting another's sessions): ~85%.
- Surgical, low-regression implementation: ~75%.
- Fully correct under all concurrent multi-server races (cross-server writes to the same session,
  crash windows, no lease/ownership): ~55%; residual last-writer-wins and single-writer concerns need
  an owner/lease model.
- Alternative (per-server/per-workspace DB files): ~60% worth doing now - cleaner isolation, but it
  conflicts with cross-workspace `--session` resume from the shared store and is a larger migration.

## Recommended verification

- `cargo test -p opencode-server` with a new two-writer persistence test.
- `cargo test -p opencode-storage` for atomic message replace/upsert behavior.
- `cargo check -p opencode-server -p opencode-storage`.
- Manual: run two `ort` instances in different workspaces, create sessions in each, then confirm both
  sessions and all messages remain in the DB after both have synced.

## Related Items

- `BUG-023` Root-cause why the plan-mode session stalled after tool calls without results - the
  persistence gap that leaves row-only sessions was found in the same audit; this card covers the
  destructive concurrent-sync half.
- `BUG-016` Plan-mode session stalls after tool calls without tool results - existing transcript
  persistence state; this card explains loss of persisted data.
- `FEAT-033` Print a resume command for the last session on TUI exit - source of the resume hint that
  pointed at the deleted session in the 2026-09-21 user-visible confirmation.
- `FEAT-023` Filter session list and load by workspace - direct load by session id should reject or
  clearly warn when the target session is missing/out of scope; the swallowed-error gap above is the
  complementary surface fix to this card's data-loss root cause.

## Notes

- This was discovered while auditing `ses_9b85fa20680b4dbfa7d2a2507335a4c1` for `BUG-023`; the audit
  observed the DB mutating underneath it. The exact lost session IDs were not captured before
  deletion, but the count deltas are recorded above.
- 2026-09-21 user report adds a named lost session id: `ses_7de60212abab46dd8fd247818ddec5e4`, printed
  as a resume command by `FEAT-033` and absent from storage on the next `ort --session` run. See
  "User-visible confirmation" under Evidence.
- Follow-up investigation (same day) confirmed the deletion is performed by a **concurrent stale
  server**, not by exit; two live servers were caught holding divergent snapshots (5 vs 8 sessions)
  with mutual clobbering. See "Investigation Notes (2026-09-21)" and the Evidence section.
- "Proposed Fix (2026-09-21 investigation)" records the recommended upsert-only snapshot plus
  explicit-delete change and its confidence levels (stops the reported symptom ~85%).
