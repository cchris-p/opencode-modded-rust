---
id: "FEAT-026"
title: "Add cross-session transcript inspection"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-18"
---

# Add cross-session transcript inspection

## Summary

Let a session look at other sessions' chats and transcripts so an agent (or the user driving it) can read what happened in another session without leaving the current one.

## Why this exists

Work is frequently spread across several sessions: an earlier session investigated a bug, another produced a plan, another is still running a task. To reuse that work the current session currently has to rely on the user manually copying text, on transcript export, or on the user re-explaining what happened.

A session that can explicitly reference another session's chat/transcript makes prior work directly inspectable. The current session can read a specific earlier conversation, cite it, and build on it, while the referenced session stays untouched.

This is the read/inspection counterpart to the fork and polling features: forking copies context forward, polling waits on state, and this lets one session look back at another session's actual chat history.

## What this means right now

Cross-session transcript inspection means a session can:

- enumerate the sessions available to it,
- select one explicitly, and
- read that session's messages/transcript in a read-only way.

The first implementation should not automatically pull another session's full transcript into the current session's context. Reading is a deliberate, user- or agent-triggered action against a clearly identified target session.

## Scope

- Define the access surface for inspecting another session: a runtime/API boundary plus whichever of a tool, in-session command, or TUI read view is needed for the daily-driver workflow.
- Support listing candidate sessions and selecting one by stable identity (session id and/or name).
- Support reading a selected session's messages/transcript, including user/assistant turns and enough structure to be useful (for example role, text, timestamps, and tool/part summaries where available).
- Reuse the existing session persistence/store and transcript-building paths rather than inventing a second history store.
- Keep inspection read-only: reading another session must not mutate it, resume it, or change its lifecycle state.
- Make the amount of data returned explicit and bounded so a large transcript does not silently blow up the reading session's context; prefer pagination, ranges, or summaries over dumping everything.
- Respect workspace scoping from `FEAT-022`/`FEAT-023` by default: a session should only see sessions that belong to its workspace unless an explicit cross-workspace override is provided.
- Make the target session identity visible in the result so the reading session can trace and cite what it read.

## Non-goals

- Injecting another session's full transcript into the current session's context automatically.
- Writing to, resuming, or otherwise mutating the inspected session.
- Replacing `FEAT-003` compact fork context or `SKILL-001` session-summary cascade; this card is about reading raw-ish session chats.
- Merging two sessions into one.
- Building a full global multi-workspace session browser.
- Guaranteeing live/streaming updates of an in-progress session's transcript in the first pass.
- Replacing `FEAT-007` polling, which waits on observable external state rather than reading chat history.

## Done when

- A session can enumerate the sessions available to it under the current workspace scope.
- A session can explicitly select another session and read its chat/transcript without mutating it.
- The read result identifies the source session clearly enough to cite or trace.
- The returned content is bounded or paginated rather than unconditionally importing a full transcript.
- Sessions from other workspaces are excluded by default and require an explicit override.
- The feature reuses existing session storage and transcript paths instead of adding a parallel history store.
- Verification covers reading a completed session and a session outside the current workspace.

## Recommended verification

- Create session A with a few user/assistant turns, then start session B.
- From session B, list sessions and confirm session A is discoverable by its identity.
- From session B, read session A's transcript and confirm the content matches session A without any modification to session A.
- Confirm the read result is bounded/paginated on a long transcript and does not silently return the entire history.
- Create a session in another workspace and confirm it is hidden from session B by default and only reachable with an explicit override.
- Confirm session A still resumes and accepts new prompts normally after being inspected.

## TBD

- Exact first access surface: tool, in-session command, TUI read view, or a combination.
- Whether the agent or only the user may trigger cross-session reads.
- Exact result shape: raw messages, rendered transcript, structured summary, or a selectable combination.
- Whether in-progress sessions can be read and how partial/streaming messages are represented.
- How ranges/pagination are expressed (index range, message id cursor, turn count, etc.).
- Whether reading another session counts as an approval-gated action.
- How this interacts with workspace identity for legacy sessions that predate `FEAT-022`.

## Related Items

- `FEAT-001` Improve historical chat transcripts workflow
- `FEAT-002` Keep sessions running after TUI exit
- `FEAT-003` Add compact fork context for session branching
- `FEAT-004` Add in-session send-to-fork commands
- `FEAT-007` Add advanced coding-session polling
- `FEAT-022` Persist session workspace identity
- `FEAT-023` Filter session list and load by workspace
- `SKILL-001` Add session-summary cascade skill by session name

## Notes

- Keep the first implementation read-only and explicitly targeted; the value is not having to copy transcripts by hand, not building autonomous session orchestration.
- Prefer bounded, explicit reads over implicit context import so this cannot quietly consume the reading session's context window.
