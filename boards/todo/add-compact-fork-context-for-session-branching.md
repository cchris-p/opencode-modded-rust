---
id: "FEAT-003"
title: "Add compact fork context for session branching"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "wiki/v1.md"
status: "todo"
created: "2026-09-08"
---

# Add compact fork context for session branching

## Summary

Add a compact context mode that can summarize the useful state of an existing session for use when starting a related forked session.

## Why this exists

Forking a prompt into a new session should not require blindly carrying the whole prior transcript every time. Some forks need the full/prior transcript for fidelity, but others need a smaller purpose-built context packet so the new session starts focused without losing the important state from the original work.

## Scope

- Define the compact context shape used when branching from an existing session.
- Include the information needed for a new forked session to continue productively from the prior work, such as user intent, relevant conclusions, constraints, current repository/workspace identity, and unresolved questions.
- Keep compact context separate from full/prior transcript mode so callers can choose explicitly.
- Prefer existing session messages, structured task state, and repository context assembly paths before inventing new persistence.
- Make compact context available through an internal runtime or API boundary that a later in-session command can call.
- Generate compact context on demand for the first implementation instead of adding a new persisted artifact.

## Non-goals

- Replacing normal session transcript storage.
- Building full ScopeMux retrieval integration in this card.
- Designing a broad multi-session task graph.
- Making compaction automatically mutate or truncate the original session.

## Done when

- The product has a defined compact context representation for session branching.
- A caller can request compact context for an existing session without including the entire transcript by default.
- Full/prior transcript mode remains distinct from compact mode.
- The compact context includes enough source-session metadata to make the fork traceable to the original session.
- Tests or focused verification cover compact context generation from a session with prior user and assistant messages.

## Recommended verification

- Create or use a session with multiple user and assistant turns.
- Generate compact fork context for that session.
- Confirm the compact context preserves the core objective, constraints, current state, and unresolved next action without copying the whole transcript.
- Confirm requesting full/prior transcript context still produces the fuller context path separately.
- Confirm the original session remains unchanged after compact context generation.

## Product decisions

- Compact context is generated on demand for the first implementation.
- The first implementation should not persist compact context unless code investigation shows an existing artifact path can be reused without expanding scope.
- If model-assisted summarization is needed for useful compact context, keep it behind this same explicit compact mode rather than making it implicit in full/prior transcript mode.

## Related Items

- `FEAT-001` Improve historical chat transcripts workflow
- `FEAT-002` Keep sessions running after TUI exit
- `SKILL-001` Add session-summary cascade skill by session name
- `START-005` Define V1 runtime loop
- `START-016` Define structured task state for V1
- `START-025` Add retrieval-provider boundary for task context assembly

## Notes

- This card supports the later direct `send-to-fork` command by defining the compact-context option that command should expose.
- Keep the first pass minimal: enough compact context for a usable fork, not a final long-term memory system.
