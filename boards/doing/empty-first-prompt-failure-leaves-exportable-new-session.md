---
id: "BUG-033"
title: "First prompt failure leaves an exportable empty new session"
priority: "P1"
type: "bug"
area: "BUG"
spec: "docs/opencode-session.md"
status: "doing"
created: "2026-09-23"
---

# First prompt failure leaves an exportable empty new session

## Summary

When the TUI starts from Home, creates a server session, and then the first prompt fails before the server accepts the prompt, the product can leave behind a real persisted session with zero messages. Exporting that session produces a misleading `New session` transcript with `_No messages_` instead of preserving the failed send as an error-only UI state.

Observed evidence:

- Transcript: `new-session-2026-09-23t0248384742177950000.md`.
- Session ID: `ses_25260ec9a6f44870862f5e974bb01b46`.
- The Rust SQLite DB contains one `sessions` row for that ID, with placeholder title, `updated_at == created_at`, status `active`, and zero `messages` rows.

Likely trigger after the recent provider/model listing PR: `client.create_session` succeeds, but `/session/{id}/prompt` can fail during provider/model resolution before `accept_prompt` materializes the first user message.

## Scope

- Fix the TUI Home-route first prompt failure path so a just-created server session is cleaned up when prompt acceptance fails.
- Preserve the existing visible send error alert.
- Preserve existing behavior for failures on already-open sessions; those should remove only the optimistic message, not delete the existing session.

## Non-goals

- Fixing the underlying provider/model request failure that caused the prompt send to fail.
- Changing transcript export formatting.
- Changing server-side prompt acceptance semantics.

## Acceptance Criteria

- If the first prompt from Home creates a server session and prompt submission fails, the just-created session is deleted and the TUI returns to Home.
- The failed prompt does not leave behind an exportable empty `New session` transcript.
- The user still sees the failed send error.
- Prompt failures on existing sessions continue to remove only the optimistic user message and leave the session intact.

## Verification

- Inspect the TUI Home-route `submit_prompt` failure branch.
- Run `cargo fmt --all --check` and `cargo check -p opencode-tui` when Rust tooling is available.
- Manual smoke: force `/session/{id}/prompt` to fail after session creation, confirm the new session is removed and no empty transcript remains.

## Related Items

- `BUG-016` Plan-mode session stalls after tool calls without tool results: earlier evidence class around transcripts revealing missing durable messages.
- `BUG-026` Save session title after new session creation: related first-message persistence/title timing behavior.
- `START-030` Match vanilla OpenAI model retrieval parity: recent provider/model listing work that likely exposed the prompt-resolution failure path.

## Dev Notes - 2026-09-23

- Updated the Home-route `submit_prompt` failure branch in `crates/opencode-tui/src/app/app.rs`.
- When `create_session` succeeds but `send_prompt` fails before prompt acceptance, the TUI now deletes the just-created server session, removes local session state, returns to Home, and still shows the failed send alert.
- Existing-session send failures are unchanged and continue to remove only the optimistic user message.
- Verification blocked locally: `cargo fmt --all --check` and `cargo check -p opencode-tui` both failed because `cargo` is not installed in this environment.
