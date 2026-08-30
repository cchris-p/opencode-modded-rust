# Task State Invariants

- Structured task state is authoritative.
- Conversation history is not authoritative task state.
- Each task must have an explicit objective.
- Each task must have explicit completion criteria before it is considered complete.
- Persistent state must outlive any individual model turn.
- Session state must remain available across TUI exits so a revisited session can resume from current live execution rather than a stale transcript snapshot.
