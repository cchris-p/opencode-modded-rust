---
id: "BUG-064"
title: "Interrupted session keeps running after the user stops it"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-10-12"
---

# Interrupted session keeps running after the user stops it

## Summary

A live `ort` session was interrupted by the user, but the agent kept going afterward. This is a
recurrence of the user-control failure class where the UI action or session interruption does not
reliably cancel the active run and stop further model/tool work.

The exact interrupt mechanism is not yet pinned down from the report. Treat this as
investigation-first: confirm whether the user used `Esc`/interrupt, killed the TUI, detached, closed
the terminal, or otherwise stopped an active session. Then identify whether the ongoing work was still
running in the same server, in a detached/background task, or in a stale server/runtime.

This bug is distinct from a merely slow interrupt: the acceptance bar is that once the product tells
the user the session is interrupted/stopped, no further model work or tool/file mutations happen unless
the user explicitly resumes or sends another prompt.

## Reported Behavior

User report, 2026-10-12:

> I have interrupted session but it keeps on going.

Transcript fragment:

```text
($PROJECT_ENV) user ~/path/to/workspace [development] $ ls create-a-board-item-for-this-issue.md
create-a-board-item-for-this-issue.md
```

The transcript is intentionally redacted to omit personally identifying shell/user/path details. It
preserves the relevant signal: in a `development` workspace, a file named
`create-a-board-item-for-this-issue.md` existed after the user believed the session had been
interrupted. That file may be evidence of post-interrupt work and should be inspected only if the
workspace is available and relevant to reproduction.

## Desired Behavior

- When the user interrupts a running session, the active generation/tool loop stops promptly.
- No additional tool calls, filesystem writes, or model steps occur after the interrupt is accepted.
- The session returns to a durable terminal/interrupted state that can be resumed intentionally.
- Any pending question/review/permission prompt created by the interrupted run is rejected or resolved
  into durable provider-valid state; it must not keep the run alive behind the user's back.
- If interruption cannot stop a detached/background runtime, the UI must report that clearly instead
  of implying the session is stopped.

## Investigation - 2026-10-12

Known evidence:

- The report is a live-use recurrence, not a synthetic test failure.
- The observed shell prompt was in a `development` workspace; the exact path is intentionally omitted
  from this card.
- The user saw a session continue after interruption strongly enough to request a new bug card.
- The exact interrupt path and session id are not yet known.

Reporter questions to answer if not recoverable from logs/session history:

- What exact action counted as "interrupted" in this report: `Esc` twice, TUI quit, terminal close,
  detach/reattach, process kill, or another action?
- What visible sign made it clear the session kept going: new file, new tool prompt, more assistant
  text, continued thinking/status, or shell/process activity?
- Did the user send any prompt or answer any question after the interrupt, or was the continued work
  fully autonomous?

Engineering questions to answer during investigation:

- Did the server log for the affected session show an abort request and `aborted:true`,
  `aborted:false`, or no abort route hit at all?
- Did the transcript grow after the interrupt timestamp, and if so were those entries model tokens,
  tool calls, queued prompt continuation, or background cleanup?
- Was the running agent attached to the same local server that received the interrupt, or did a stale
  server/background runtime continue independently?
- Did the continued work mutate files after the interrupt, and can those mutations be timestamped
  relative to the interrupt event?

Candidate failure classes:

- **Abort not sent:** the TUI/session surface accepted the key or exit action locally but never called
  the server abort route.
- **Abort sent to wrong runtime:** the interrupt hit a different server/session than the one executing
  the active turn.
- **Abort acknowledged but not observed:** the active provider/tool future did not observe cancellation
  and continued streaming or executing.
- **Queued continuation survived abort:** a queued prompt, question answer, permission response, or
  continuation event resumed the run after the interrupt should have terminally stopped it.
- **Detached/background work escaped cancellation:** a spawned task outlived the cancelled session turn.
- **Status/display drift:** the run actually stopped, but stale output/status made it appear to keep
  running.

Initial confidence report:

- Confidence in any single culprit is low until the affected session id and server log are recovered.
- Most likely first place to look: whether the interrupt was sent to the same runtime that was still
  executing the turn. This covers both "abort not sent" and "abort sent to wrong runtime" and best
  matches the symptom where the UI/user believes the session stopped while work continues elsewhere.
- Second place to look: cancellation observation inside the active provider/tool future. If the abort
  route logged `aborted:true` for the correct session, then the culprit is likely an await/spawned task
  that did not observe cancellation.
- Third place to look: queued continuation paths, especially question/permission/review responses or
  continuation events that may survive abort and restart work after the terminal interrupted state.
- Status/display drift is the lowest-confidence culprit unless investigation shows no transcript
  growth and no file/tool mutations after the interrupt.

## Scope Boundaries

- In scope: live `ort` TUI/session interruption, server abort handling, active run cancellation,
  pending question/review/permission cleanup, post-interrupt transcript growth, and post-interrupt
  filesystem/tool mutations.
- Out of scope: changing normal resume semantics after a correctly recorded interrupted state;
  `FEAT-035` owns intentional resume behavior.
- Out of scope: generic provider slowness unless the provider continues producing output after an
  accepted interrupt.

## Related Items

- `BUG-019` Escape does not interrupt the running session - original abort-path defect.
- `BUG-029` Esc cannot interrupt a thinking turn; the interrupt hint toggles between states - TUI
  confirmation-state defect.
- `BUG-043` A session run can end without a terminal state, leaving it stuck busy and uninterruptible
  - terminal-state and cancellation contract.
- `BUG-051` Interrupt-then-continue continuation prompt flashes / never settles - continuation/display
  behavior around interrupted sessions.
- `BUG-057` Interrupt still does not stop a running session; deepseek keeps thinking after Esc - prior
  recurrence requiring live evidence.
- `BUG-056` Session continues instead of stopping when the question tool is presented to the user -
  related class where session execution continues when it should park for user control.

## Acceptance Criteria

- Reproduce or localize the report with session id, redacted workspace identity, server log path, and
  interrupt action.
- Prove which failure class occurred using runtime evidence, not code inspection alone.
- Identify whether any post-interrupt work occurred and classify it as model text, tool call,
  filesystem mutation, queued continuation, pending prompt resolution, or stale display/status.
- Add or update regression coverage for the confirmed class.
- Verify that accepted interrupts prevent further model/tool work for the affected path.
- Record the verification command(s), server-log evidence, and any affected session ids on this card.
