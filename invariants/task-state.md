# Task State Invariants

- Structured task state is authoritative for bounded work execution.
- V1 supports one active structured task per session.
- Conversation history is supporting evidence only, not authoritative task state.
- Lightweight todos support execution but do not define the task objective, stage, or completion decision.
- Session lifecycle state and task execution state are separate concerns.
- Each task must have an explicit objective.
- Each task must have explicit completion criteria before it is considered complete.
- Each task must record its current runtime stage explicitly.
- Each task must retain a structured verification plan and the latest verification result.
- Each task must retain the latest review result before completion.
- Any reopen into repair must record an explicit reason.
- Persistent task state must outlive any individual model turn and any TUI exit.
- Session resume must restore the current task from durable state without reconstructing authority from transcript history or todo text.
- The durable task record must round-trip through the storage layer with stable field ownership for objective, criteria, stage, verification, review, and reopen state.
