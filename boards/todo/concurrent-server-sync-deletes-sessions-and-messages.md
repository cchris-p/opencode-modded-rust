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

## Notes

- This was discovered while auditing `ses_9b85fa20680b4dbfa7d2a2507335a4c1` for `BUG-023`; the audit
  observed the DB mutating underneath it. The exact lost session IDs were not captured before
  deletion, but the count deltas are recorded above.
