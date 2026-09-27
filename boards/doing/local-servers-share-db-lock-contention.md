---
id: "BUG-052"
title: "Concurrent local servers sharing one opencode.db fail session sync with database is locked"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
created: "2026-09-27"
---

# Concurrent local servers sharing one opencode.db fail session sync with database is locked

## Summary

Multiple local Rust servers (each TUI spawns its own `serve` for its workspace) share the single
database at `dirs::data_local_dir()/opencode/opencode.db`. When more than one server writes at the
same time, session sync fails with SQLite `database is locked`:

```
ERROR opencode_server::routes: failed to sync sessions to storage:
Query error: error returned from database: (code: 5) database is locked
```

This is **write/lock contention** over one SQLite file, not the full-snapshot overwrite bug already
fixed by `BUG-025`. It is a distinct concurrency defect and is a plausible amplifier of the
stale-snapshot / flip-flop behavior reported in `BUG-051`.

## Evidence (live, 2026-09-26)

Runtime target: Rust product (`target/debug/opencode`); three TUI+serve pairs were running
(`:3187` pid 55953, `:3188` pid 90502, `:3189` pid 91261), all pointed at the shared DB
`~/Library/Application Support/opencode/opencode.db`.

`traces/server.log`:

```
2026-09-26T22:27:15.487818Z ERROR opencode_server::routes: failed to sync sessions to storage:
  Query error: error returned from database: (code: 5) database is locked
2026-09-26T22:29:40.493358Z ERROR opencode_server::routes: failed to sync sessions to storage:
  Query error: error returned from database: (code: 5) database is locked
2026-09-26T22:29:59.272571Z ERROR opencode_server::routes: failed to sync sessions to storage:
  Query error: error returned from database: (code: 5) database is locked
```

SQLite code 5 is `SQLITE_BUSY` (the file is locked by another connection). `rg` for the sync path in
`crates/opencode-server/src/routes.rs` (`sync_sessions_to_storage`) shows the write is not retried and
the failure is logged and dropped.

## Why this matters

- A failed sync means the in-memory session state was not persisted; combined with `BUG-047`
  (in-flight progress not otherwise durable), a write failure can silently lose turn progress.
- Lock contention between servers is a plausible trigger for the in-memory snapshot alternation
  observed in `BUG-051` (a server reloading or re-applying a persisted snapshot while another writer
  holds the file).
- Theoretically, a session can be open on two servers at once (the shared DB loads all sessions at
  startup), so any write can contend at arbitrary times.

## Scope

- Make storage writes robust to concurrent writers: set/raise SQLite `busy_timeout`, or retry on
  `SQLITE_BUSY` with backoff, and/or enable WAL mode so readers and writers do not block each other.
- Ensure a transient sync failure does not silently drop session state (retry, queue, or surface).
- Confirm the chosen approach across the actual multi-server topology (several TUI-spawned serves on
  one DB), not just a single server.

## Non-goals

- `BUG-025` Concurrent servers delete each other's sessions/messages via full-snapshot DB sync
  (done) - that was snapshot overwrite, this is lock contention.
- The stale-snapshot clobber itself; that is `BUG-043` / `BUG-051`.
- The per-server isolation model (one fresh server per `ort` run); not changing that here.

## Done when

- With multiple servers writing the shared DB concurrently, `sync sessions to storage` no longer
  fails with `database is locked` (or is retried to success).
- A transient lock does not lose session/turn progress.
- There is a test or harness that exercises concurrent writers against one DB file.

## Recommended verification

- Reproduce with at least two live servers writing (run two workspaces/TUIs) and confirm no
  `database is locked` errors in `traces/server.log`.
- Unit/integration test with two connections to one SQLite file writing concurrently, asserting no
  unhandled `SQLITE_BUSY`.
- `cargo test -p opencode-storage -p opencode-server`; `cargo check --workspace`.

## Related Items

- `BUG-025` Concurrent servers delete each other's sessions and messages via full-snapshot DB sync
  (done) - adjacent multi-server defect, different mechanism.
- `BUG-051` Continuation prompt flashes / unordered - observed alongside this lock contention.
- `BUG-043` A session run can end without a terminal state - shares the sync/merge write path.
- `BUG-047` Assistant turn progress is not persisted until the run completes - a lost sync makes the
  persistence gap worse.
- `AGENTS.md` storage paths - canonical DB location for this product.

## Notes

- Created 2026-09-27 from the live `traces/server.log` evidence captured during the `BUG-051`
  investigation.
- Not the same as `BUG-025`: this card is about failing writes under concurrent access, not one
  server overwriting another's snapshot.
- Likely files: `crates/opencode-server/src/routes.rs` (`sync_sessions_to_storage`,
  `persist_sessions_if_enabled`) and `crates/opencode-storage/src/database.rs` (connection setup,
  pragmas).

## Hold Decision (2026-09-27)

- Moved `todo` -> `hold` by user direction. The defect is real and still un-fixed: the SQLite pool
  opens with default journal mode and no `busy_timeout` (`crates/opencode-storage/src/database.rs:43-45`),
  so a concurrent writer still returns `SQLITE_BUSY`. It is distinct from `BUG-025` (snapshot
  overwrite) and from the `BUG-043`/`BUG-047`/`BUG-051` snapshot-merge fixes.
- It only fires when **two or more local servers write the shared `opencode.db` at the same time**.
  The current single-workspace daily-driver flow does not routinely create that topology.
- Reactivation trigger: before any multi-workspace / concurrent-server daily-driver use, or as part
  of V1 robustness hardening. The fix is small and low-risk (WAL + `busy_timeout`/retry on
  `SQLITE_BUSY`).
- Not archived: this is a silent write-loss path, not a non-issue.

## Dev Notes (2026-09-27)

- Reactivated `hold` -> `doing` to land the low-risk concurrency hardening described above.
- Root cause: `Database` opened the shared `opencode.db` as a bare `sqlite:<path>?mode=rwc` URL,
  which leaves the default rollback journal in place and installs no busy handler. A second writer
  therefore returned `SQLITE_BUSY` (code 5, `database is locked`) immediately instead of waiting.
- Fix in `crates/opencode-storage/src/database.rs`:
  - open with `SqliteConnectOptions` instead of a raw URL
  - `journal_mode=WAL` so readers do not block behind a writer
  - `synchronous=NORMAL` (the WAL-appropriate durability setting)
  - `busy_timeout=30s` so a concurrent writer waits for the lock to clear instead of failing
  - added `Database::open(path)` (used by `Database::new`) so tests can point multiple connections
    at one file.
- No retry wrapper was added: the busy timeout lets the existing `sync_sessions_to_storage` write
  wait out a transient lock rather than error, which is the smaller correct change for this card.

## Verification (2026-09-27)

Environment: isolated git worktree `~/worktrees/opencode-modded-rust/bug-052` on
`bug/BUG-052-sqlite-busy-timeout-wal`, based on `development` at `bceebcd`, Linux.

- `cargo test -p opencode-storage -p opencode-server` - pass (storage 5/5, server 71/71, skill-route 3/3).
- `cargo check --workspace` - pass.
- `database::tests::shared_database_uses_wal_and_busy_timeout` asserts `PRAGMA journal_mode=wal`
  and `PRAGMA busy_timeout=30000` on a live connection.
- `database::tests::concurrent_writer_waits_for_lock_instead_of_failing` holds a write transaction
  on one connection and proves a second connection's insert waits for the lock and succeeds rather
  than returning `SQLITE_BUSY`.
- Live two-server reproduction (two TUI/serve pairs writing the shared DB) was not run in this
  environment; the deterministic concurrent-connection test covers the same lock-contention path.