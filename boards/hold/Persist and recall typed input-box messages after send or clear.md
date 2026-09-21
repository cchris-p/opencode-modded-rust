---
id: "FEAT-036"
title: "Persist and recall typed input-box messages after send or clear"
priority: "P3"
type: "feature"
area: "FEAT"
spec: ""
status: "hold"
created: "2026-09-21"
---

# Persist and recall typed input-box messages after send or clear

## Summary

Explore whether the TUI should keep the text a user types into the prompt input box after that text leaves the box — either because the user sent it (Enter) or cleared it (`Ctrl+C`) — so recent typed messages can be recovered, recalled, or reused instead of being lost.

## Why this exists

Typing a prompt is real work. Today the input box is effectively ephemeral: sending a message submits and clears it, and `Ctrl+C` clears the draft outright. Once cleared, the text is gone unless the user retypes it or digs it out of the sent transcript.

That feels wrong for a daily-driver TUI. Users often want to:

- Re-send or lightly edit a just-sent prompt.
- Recover a draft that was cleared by accident or by a `Ctrl+C` habit.
- Cycle back through recent prompts the way a shell exposes history.
- Fork or branch from a previous prompt (overlaps `FEAT-004`).

## Current behavior (to confirm during exploration)

- Enter submits the input box contents and clears the box.
- `Ctrl+C` clears the prompt input (`FEAT-006`) with no history retained.
- There is no user-visible, recallable store of previously typed-but-not-sent drafts or recently sent prompts.
- Sent messages remain visible in the transcript, but are not re-enterable into the input box.

## Scope

Keep this exploratory until the open questions below are resolved. Likely areas to investigate:

- Decide what gets stored: only sent messages, only cleared drafts, or both.
- Decide where storage lives: in-memory per session, in-memory per TUI run, or durable (session metadata, SQLite, or a config/cache file).
- Decide the recall interaction: up/down arrow history, a dedicated keybind, a picker, or a command.
- Decide whether recall is per-session or global across sessions.
- Decide how recall interacts with the existing prompt editing, selection-copy, and multi-line behavior.
- Define what happens to sensitive content, very long prompts, and duplicate entries.
- Keep any first implementation small and reversible.

## Open questions / TBD

- Does "storing" mean history/recall, or auto-restoring the exact last draft when the input box becomes empty again?
- Should `Ctrl+C` clear-to-history behave differently from Enter submit-to-history?
- Is history bounded (N entries) and is it persisted across TUI/server restarts?
- Per-workspace, per-session, or global history scope?
- Which keybind surface is free, given `Ctrl+C` already clears and `Ctrl+D` exits?
- Does durable storage belong in the product DB (`~/.local/share/opencode/opencode.db`) or a lighter local file?
- Should recalled prompts be editable in place by default, or inserted as-is?

## Non-goals

- Rebuilding the full transcript or message store.
- Command-line shell-style history expansion (`!!`, `!$`, etc.).
- Cross-user or cross-machine prompt sync.
- Committing to a specific storage schema before the interaction model is chosen.

## Done when

- The intended storage/recall behavior is clearly defined (what is stored, where, and for how long).
- The recall interaction is chosen and documented, with any keybind conflicts resolved.
- Scope is reduced to a first, small, verifiable implementation slice.
- Follow-up implementation cards can be split from this item without re-litigating the concept.

## Likely touchpoints

- TUI prompt input state and key handling (prompt clear/submit paths).
- TUI keybind registration and help/command-palette hints.
- Session/TUI state that survives prompt clears.
- Possibly storage layer, if durable persistence is chosen.

## Related Items

- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt (history on clear intersects this)
- `FEAT-004` Add in-session send-to-fork commands (reuses prompt-box contents)
- `FEAT-001` Improve historical chat transcripts workflow
- `BUG-013` Cursor on input field needs to always be visible
- `BUG-014` Prompt input cursor renders out of place

## Notes

- Keep this item exploratory: do not treat any single storage/interaction choice as decided yet.
- Prefer the smallest daily-driver slice (for example, in-memory recall of the last N prompts) before durable persistence.