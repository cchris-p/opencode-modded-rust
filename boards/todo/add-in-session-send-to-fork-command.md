---
id: "FEAT-004"
title: "Add in-session send-to-fork commands"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "wiki/v1.md"
status: "todo"
created: "2026-09-08"
---

# Add in-session send-to-fork commands

## Summary

Add direct in-session fork commands that start a new session from the current prompt-box contents, with separate commands for full/prior transcript context and compact context.

## Why this exists

During real coding work, a user often wants to split a tangent, alternate implementation approach, review pass, or follow-up question out of the current session without losing the useful setup already established. Direct in-session commands make that flow intentional: send a prompt to a fresh session, carry the right context, and keep the original session intact.

## Scope

- Add an in-session command named `send-to-fork` that uses full/prior transcript context.
- Add an in-session command named `send-to-fork-compact` that uses compact context from `FEAT-003`.
- Use the current prompt-box contents as the default fork prompt text.
- Start a new session from that forked prompt.
- Preserve the original session and its lifecycle state.
- Make the new session traceable to the source session in the user-visible flow or stored metadata where the current architecture supports it.
- Return or navigate to the new session in a way that matches existing TUI session behavior.

## Non-goals

- Automatically deciding to fork without explicit user action.
- Merging forked-session results back into the source session.
- Building a general task-graph UI.
- Replacing normal new-session creation.
- Implementing compact context generation inside this card if `FEAT-003` has not landed yet.

## Done when

- The in-session command list exposes `send-to-fork` and `send-to-fork-compact` without duplicating existing session actions.
- Running `send-to-fork` creates a new session seeded by the forked prompt and includes the relevant prior conversation context.
- Running `send-to-fork-compact` creates a new session seeded by the forked prompt and uses the compact-context path from `FEAT-003`.
- The source session remains active or otherwise unchanged except for any explicit trace metadata required by the implementation.
- The TUI clearly indicates or navigates to the forked session after creation.

## Recommended verification

- Start a session and exchange enough messages to create meaningful prior context.
- Run the in-session `send-to-fork` command with a fork prompt.
- Confirm a new session starts and can answer using the prior transcript context.
- Run the in-session `send-to-fork-compact` command with a fork prompt.
- Confirm the second new session starts from compact context rather than the whole transcript.
- Confirm the original session still accepts prompts after both forks.
- Confirm session switching or navigation still follows existing TUI behavior.

## Dependency

- `FEAT-003` should land before `send-to-fork-compact` is considered complete.
- If implementing this card first, gate `send-to-fork-compact` behind a clear unavailable state rather than silently falling back to full/prior transcript context.

## Product decisions

- `send-to-fork` uses the current prompt-box contents and full/prior transcript context.
- `send-to-fork-compact` uses the current prompt-box contents and compact context.
- The product uses separate commands instead of one command with a context-mode picker.

## Related Items

- `FEAT-001` Improve historical chat transcripts workflow
- `FEAT-002` Keep sessions running after TUI exit
- `FEAT-003` Add compact fork context for session branching
- `SKILL-001` Add session-summary cascade skill by session name
- `START-005` Define V1 runtime loop
- `START-016` Define structured task state for V1

## Notes

- Treat this as a direct product command, not as a skill-only workflow.
- Keep the first implementation focused on one clear fork path from the active session to one newly created session.
