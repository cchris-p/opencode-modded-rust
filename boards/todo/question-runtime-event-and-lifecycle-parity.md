---
id: "FEAT-040"
title: "Question runtime event and lifecycle parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Question runtime event and lifecycle parity

## Summary

Child of `GATE-002` (parity gap 3). Publish ask/reply/reject question events consistently, clear
pending requests on reply/reject/drop, reject waiters on shutdown, and ensure a rejected or stale
question unblocks the waiting tool call with a clear error.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 3.

## Problem

- Ask/reply/reject are only surfaced as generic `session.updated` with a `source` string
  (`crates/opencode-server/src/routes.rs:2711-2718`, `:4990-5039`).
- A rejected question maps to `ToolError::ExecutionError("question rejected")` (`:2722-2724`) rather
  than a clear rejection error.
- Pending requests are never cleared on session/runtime shutdown, and a dropped waiter produces a
  generic error (`:2725-2727`). There is no expiry for stale requests.

## Vanilla reference

`packages/core/src/question.ts`:

- Pending map keyed by request ID; each entry holds the request and a deferred.
- `ask` publishes `question.v2.asked`, awaits the deferred, and always deletes pending on completion.
- `reply` publishes `question.v2.replied` (with `sessionID`, `requestID`, `answers`) then succeeds the
  deferred and deletes pending.
- `reject` publishes `question.v2.rejected` then fails the deferred with `RejectedError`
  ("The user dismissed this question") and deletes pending.
- A finalizer rejects every outstanding waiter on location/service shutdown.

## Scope / deliverables

- Emit explicit ask/reply/reject question events carrying `sessionID`, `requestID`, and (on reply)
  `answers`, in addition to or replacing the generic `session.updated` source notifications.
- Clear pending request + waiter entries on reply, reject, waiter drop/cancel, and session shutdown.
- Map rejection to a clear tool-facing error (e.g. `ToolError::QuestionRejected`) distinct from a
  dropped-connection error.
- Reject/clean up outstanding waiters when a session run is aborted or the runtime shuts down so a
  session cannot hang forever on a stale question.

## Non-goals

- Session-scoped route ownership (`FEAT-039`).
- TUI rendering changes (`FEAT-041`).

## Acceptance criteria

- [ ] Ask, reply, and reject each produce a distinguishable event including `sessionID` and
      `requestID`.
- [ ] Pending state is removed after reply, reject, waiter drop, and shutdown.
- [ ] A rejected question unblocks the waiting tool call with a clear rejection error.
- [ ] Aborting/closing a session does not leave a waiter pending indefinitely.
- [ ] Tests cover reply cleanup, reject/unblock, and drop cleanup.

## Verification

- `cargo fmt --all`
- `cargo check -p opencode-server -p opencode-session`
- `cargo test -p opencode-server question`

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-039` session-scoped question API parity.
- `FEAT-041` TUI question prompt UX parity.
