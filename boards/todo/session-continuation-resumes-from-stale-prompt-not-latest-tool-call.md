---
id: "BUG-044"
title: "Continuing a session resumes from an earlier user prompt instead of the latest tool call"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-26"
---

# Continuing a session resumes from an earlier user prompt instead of the latest tool call

## Summary

After a session stalls, continuing it re-enters from an earlier user message rather than the latest
assistant/tool-call state. In the captured session the user's last input was `Yes`; when the stalled
session was continued, it resumed from that `Yes` turn instead of from the latest pending `bash`
tool call, replaying the earlier work rather than continuing where the run actually stopped.

The continuation/resume point must be the most recent persisted part (including an unresolved tool
call), not an older user prompt.

## Reported behavior

- A session runs several turns and then stalls on a hanging tool call.
- The user continues the session.
- The continuation restarts from an earlier user prompt (`Yes`) rather than from the last tool
  call/output, so previous work is repeated and the stalled step is not carried forward.

## Evidence

Exported transcript: [`deepseek-flp-skill-hang-session.md`](../../docs/transcripts/deepseek-flp-skill-hang-session.md)

- Session ID: `ses_2c1168ee88f644f49315ca1736064d20` (2026-09-26).
- User turns in the export: `Is that skill that helps resolve missing sample files for .flp projects
  set up?` then `Yes`.
- The final recorded state is an assistant `bash` tool call with no result; continuing the session
  resumes from the `Yes` user prompt instead of that pending tool call.

## Why this exists

Continuation is the recovery path after a stall or restart. If it resets to the last user message,
the runtime repeats completed work, loses the unresolved tool-call state, and cannot be used to debug
the exact step that failed. The resume point needs to be unambiguous and based on the latest recorded
part.

## Scope

- Determine why continuation selects the last user message (`Yes`) instead of the most recent
  assistant/tool state.
- Continue from the latest persisted part, including a pending or unresolved tool call, rather than
  replaying an older user prompt.
- When the latest part is an unresolved tool call, resolve/annotate it (durable error result) and
  continue from there.
- Ensure the resume point survives a hang/restart unambiguously.

## Non-goals

- Resume-after-`Esc` behavior; `FEAT-035` already covers continuing from an interrupted point.
- The provider stream stall itself; owned by `BUG-038`.
- The run terminal-state / interrupt contract; that is `BUG-043`.
- Persisting in-flight turn progress; that is `BUG-047`.
- Unresolved-tool-call transcript repair already shipped under `BUG-016`.

## Done when

- Continuing a stalled session proceeds from the latest recorded state, not an earlier user prompt.
- A pending/unresolved tool call is handled (resolved or errored) as part of continuation.
- No duplicate or replayed turn from an older user message occurs on continue.
- There is test coverage for continue-after-stall selecting the latest state.

## Recommended verification

- A session-loop test that stalls on an unresolved tool call, then continues, asserting the resumed
  request is built from the latest state and does not re-issue the previous user prompt.
- Inspect the persisted messages/parts before and after continue to confirm the chosen resume point.
- `cargo test -p opencode-session`; `cargo check -p opencode-session -p opencode-server -p opencode-tui`.
- Live: reproduce the captured stall, continue, and confirm the run resumes from the last output.

## Likely root cause to confirm

The captured session's post-`Yes` turn was never persisted (server memory had 12 messages, the DB
only 7), so the last **persisted** message is the user prompt `Yes`. Continuation selecting the last
persisted state therefore lands on `Yes` — this may be the persistence gap (`BUG-047`) rather than a
defect in continuation-point selection. Confirm which before implementing: if `BUG-047` is fixed,
this symptom may disappear.

## Related Items

- `FEAT-035` Resume an interrupted session from where it left off - continuation after interrupt;
  this card is continuation-point selection after a stall.
- `BUG-047` Assistant turn progress is not persisted until the run completes - likely root cause of
  the stale `Yes` resume point (in-memory 12 vs DB 7).
- `BUG-043` A session run can end without a terminal state - the stall that makes continuation
  necessary in the captured session.
- `BUG-038` DeepSeek reasoning turn stalls mid-turn - silent non-completion that leaves a stale
  resume point.
- `BUG-016` / `BUG-023` Unresolved tool-call state in the session prompt loop.

## Notes

- Captured export: `docs/transcripts/deepseek-flp-skill-hang-session.md` (workspace
  `dump-musicproduction`).
- Relevant files likely include the session continuation/queue path
  (`crates/opencode-session/src/prompt.rs`) and the server queue drain
  (`crates/opencode-server/src/routes.rs`).
