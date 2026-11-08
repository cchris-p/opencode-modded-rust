---
id: "BUG-020"
title: "Timeline shows only user-sent prompts"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "done"
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

## Dev Notes

- Branch: `bug/BUG-020-timeline-user-prompts`
- Change: extracted `timeline_entries_from_messages` in `crates/opencode-tui/src/app/app.rs` and filtered entries to `MessageRole::User`; `handle_open_timeline` now builds the timeline from that helper.
- Decision: kept filtering at entry construction (per card scope) and preserved each entry's `message_id`, so `Enter` still calls `SessionView::scroll_to_message`. No change to `timeline.rs`; the existing empty state covers sessions with no user prompts.
- Decision: did not touch `handle_fork_session`, which has a similar all-message mapping but is out of scope for this card.
- Verification: `cargo fmt -p opencode-tui -- --check` clean; `cargo test -p opencode-tui -- --test-threads=1` 44 passed; `cargo check -p opencode-tui` clean; `cargo clippy -p opencode-tui --all-targets` no new warnings at the changed lines. Added tests `timeline_entries_include_only_user_prompts` and `timeline_entries_empty_when_no_user_prompts`.
- Note: real-terminal `/timeline` check (`ort-build`/`ort`) was not run from the non-interactive agent shell; available for local QA on the PR branch.

## Completion - 2026-09-21

- Merged as PR #58 (merge commit `b116502`) into `development`.
- Branch `bug/BUG-020-timeline-user-prompts` deleted remotely and locally after merge.
- Promoted from `qa` to `done` on explicit user direction. No separate QA report was recorded: automated verification passed (44 `opencode-tui` tests, `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets` with no new warnings), and real-terminal `/timeline` QA remains available on `development`.
- User confirmed the timeline behavior works on 2026-09-21; card remains closed in `done`.
