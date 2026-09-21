---
id: "BUG-020"
title: "Timeline shows only user-sent prompts"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "doing"
created: "2026-09-21"
---

# Timeline shows only user-sent prompts

## Summary

The `/timeline` dialog currently lists every message in the session, including assistant and system messages. It should show only the prompts the user actually sent, so the timeline is a navigable list of user turns.

## Reported behavior

- Opening `/timeline` lists user, assistant, and system messages.
- The list is dominated by messages the user did not type, making it hard to jump between the prompts they sent.

## Required solution direction

- Filter timeline entries to user-sent prompts only.
- Do not show assistant or system messages in the timeline.
- Keep selecting an entry jumping to that message's position, as it does today.
- Do not add role toggles or other filtering controls in this item.

## Scope

- Fix entry construction in `crates/opencode-tui/src/app/app.rs` (`handle_open_timeline`, around line 2157), which currently maps every message regardless of role.
- Keep filtering at the point entries are built; the dialog itself does not need to change its role rendering for this fix.
- Only touch `crates/opencode-tui/src/components/dialogs/timeline.rs` if filtering at entry construction is insufficient to satisfy the done criteria.

## Done when

- `/timeline` lists only prompts the user sent.
- Assistant and system messages no longer appear in the timeline.
- Selecting a listed prompt still navigates to that message.
- The timeline remains usable when a session has few or no user prompts.

## Verification

- In a session with at least one user prompt, assistant reply, and system message, open `/timeline`.
- Confirm only the user prompt entries appear.
- Confirm selecting an entry scrolls/navigates to that message as before.
- Confirm the empty state is shown when no user prompts exist.

## Notes

- Current code evidence: `handle_open_timeline` maps over all session messages and assigns `user`/`assistant`/`system` roles without filtering.
