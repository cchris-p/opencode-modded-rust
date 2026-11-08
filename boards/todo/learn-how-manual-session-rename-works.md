---
id: "FEAT-029"
title: "Learn how manual session rename works end to end"
priority: "P2"
type: "research"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-21"
---

# Learn how manual session rename works end to end

## Summary

Document and discuss how a user-initiated (manual) session rename actually works across the TUI, API, server, and session storage, so future rename work has one grounded reference instead of rediscovering the flow per item.

## Why this exists

Manual rename has been touched by several stories (`FEAT-025`, `BUG-015`) but there is no single write-up of the path a rename takes or of the rules that protect a manual title from later automated title generation. This item captures that understanding and records open questions before more rename changes are planned.

## Questions to answer

- What are all the current manual rename entry points, and do they share one code path?
- What exactly happens between pressing Enter in the rename dialog and the new title being visible and persisted?
- How does the system distinguish a default/generated title from a user-chosen one, and what prevents an auto-title pass from overwriting a manual rename?
- What validation or edge cases exist today (empty title, whitespace, very long titles, renaming a non-active session)?
- Are there gaps or inconsistencies worth splitting into implementation items?

## Current understanding (starting point, verify before relying on it)

- TUI dialog: `SessionRenameDialog` holds the target `session_id` plus the current title, edits an input buffer, and `confirm()` returns `(session_id, trimmed_title)` while rejecting an empty title (`crates/opencode-tui/src/components/dialogs/session_rename.rs`).
- There is also a rename-from-session-list path via the session list dialog, so at least two entry points converge on the same update call.
- TUI client: `ApiClient::update_session_title` issues `PATCH /session/{id}` with an `UpdateSessionRequest { title }` and returns the updated `SessionInfo` (`crates/opencode-tui/src/api.rs:489`).
- Server: `update_session` applies `session.set_title(title)` and persists sessions (`crates/opencode-server/src/routes.rs:2128`); a separate `set_session_title` handler also exists (`crates/opencode-server/src/routes.rs:589`).
- Session core: `Session::set_title` sets the title and touches `updated_at` (`crates/opencode-session/src/session.rs:621`).
- Auto-title guard: `ensure_title` only generates a title while `is_default_title()` is true, where default means a `"New session - <rfc3339>"` or `"Child session - <rfc3339>"` prefix (`crates/opencode-session/src/prompt.rs:2308`, `crates/opencode-session/src/session.rs:430`). A manual rename therefore changes the title to a non-default value and blocks later auto-overwrite.
- UI refresh: `BUG-015` made both rename entry points apply the returned `SessionInfo` to local state immediately rather than relying only on a later list refresh or server sync.
- Shortcut: `FEAT-025` mapped `Ctrl+R` to the existing rename action in the normal session view.

## Scope

- Trace and describe the full manual rename flow in one place: entry points, dialog behavior, API call, server handler, session mutation, persistence, and UI refresh.
- Capture the default-vs-manual title rule and where it is enforced.
- Record validation/edge-case behavior actually observed in code and in a manual smoke run.
- List concrete follow-up items only if a real gap is found; do not implement changes here.

## Non-goals

- Redesigning title generation, summarization, or session storage.
- Adding new rename commands, shortcuts, or surfaces.
- Changing session ordering or list semantics.

## Done when

- The end-to-end manual rename flow is written down with file references.
- The default/generated-vs-manual title protection is explicitly stated.
- Entry-point parity and any validation gaps are called out.
- Any required follow-up work is split into implementation-ready board items, or explicitly stated as unnecessary.

## Recommended verification

- Read the TUI dialog, `ApiClient::update_session_title`, `update_session`/`set_session_title` handlers, `Session::set_title`, and `ensure_title` together and confirm the described flow matches.
- Run `ort-build`, then `ort`: rename the active session with `Ctrl+R`, rename from the sessions list, and rename a non-active session; confirm the title updates immediately and survives a restart.
- Manual smoke: rename a brand-new session before its first message and confirm a later auto-title pass does not overwrite the manual title.

## Related Items

- `BUG-015` Session rename immediately refreshes current terminal surfaces
- `FEAT-025` Map Ctrl+R to rename
- `SKILLS-004` Add session-summary cascade skill by session name

## Notes

- Treat this as a learn-and-document item first; only spin off implementation work if the trace surfaces a real defect or inconsistency.
