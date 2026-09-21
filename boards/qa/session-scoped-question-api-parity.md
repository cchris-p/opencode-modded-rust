---
id: "FEAT-039"
title: "Session-scoped question API parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: ""
created: "2026-09-21"
---

# Session-scoped question API parity

## Summary

Child of `GATE-002` (parity gap 2). Make pending question list/reply/reject session-scoped and
validate request ownership before accepting a reply or reject, matching vanilla's
`/api/session/:sessionID/question` routes and handlers.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 2.

## Problem

- `crates/opencode-server/src/routes.rs` exposes only global `GET /question`,
  `POST /question/{id}/reply`, `POST /question/{id}/reject` (`:4925-5041`).
- `list_questions` returns every pending request with no session filter (`:4963-4968`).
- `reply_question`/`reject_question` never verify that the request belongs to an addressed session
  (`:4975-5041`).
- The TUI client lists all questions and filters client-side (`crates/opencode-tui/src/app/app.rs:2983-2986`).

## Vanilla reference

`packages/protocol/src/groups/question.ts` and `packages/server/src/handlers/question.ts`:

- `GET /api/question/request` (location list).
- `GET /api/session/:sessionID/question` returns requests filtered by session.
- `POST /api/session/:sessionID/question/:requestID/reply` and `.../reject`.
- `withOwnedQuestion(sessionID, requestID, ...)` returns `QuestionNotFoundError` when the request is
  missing **or** owned by another session.

## Scope / deliverables

- Add session-scoped routes:
  - `GET /session/{id}/question`
  - `POST /session/{id}/question/{requestID}/reply`
  - `POST /session/{id}/question/{requestID}/reject`
- Enforce ownership: reply/reject return not-found when the request's `session_id` differs from the
  addressed session.
- Keep the global `/question` list only if a concrete local compatibility need remains; otherwise
  have it delegate or deprecate it.
- Update `crates/opencode-tui/src/api.rs` and `app.rs` to use the session-scoped list/reply/reject.
- Include `sessionID` on question events emitted during reply/reject.

## Non-goals

- TUI prompt layout/UX changes (`FEAT-041`).
- Runtime pending-map lifecycle/cleanup (`FEAT-040`).

## Acceptance criteria

- [x] Pending questions for a session are listed only through the session-scoped route.
- [x] Reply/reject to a request owned by a different session is rejected (not-found) and does not
      resolve the original waiter.
- [x] Reply/reject to a missing request returns not-found without panicking.
- [x] The TUI uses session-scoped APIs and no longer needs client-side session filtering for replies.
- [x] Focused tests cover correct-session success and wrong-session rejection.

## Verification

- `cargo fmt --all`
- `cargo check -p opencode-server -p opencode-tui`
- `cargo test -p opencode-server question`

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-038` question schema and tool contract parity.
- `FEAT-040` question runtime event and lifecycle parity.
- `FEAT-041` TUI question prompt UX parity.

## Dev Notes - 2026-09-21

- Implemented session-scoped question routes: `GET /session/{id}/question`, `POST /session/{id}/question/{requestID}/reply`, and `POST /session/{id}/question/{requestID}/reject`.
- Added ownership checks so wrong-session reply/reject returns not found and leaves the original pending request and waiter intact.
- Updated the TUI client and prompt flow to list, reply, and reject through session-scoped endpoints instead of listing globally and filtering client-side.
- Kept legacy global question endpoints in place for compatibility, but the TUI no longer depends on them.
- Verification: `cargo fmt --all`; `cargo check -p opencode-server -p opencode-tui`; `cargo test -p opencode-server question`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/67.
- Status: moved to `qa` for PR/local verification.
