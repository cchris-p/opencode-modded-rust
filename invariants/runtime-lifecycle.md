# Runtime Lifecycle Invariants

- The runtime owns lifecycle state transitions.
- The model may make bounded decisions inside the runtime, but it does not control the overall workflow.
- Tasks move through explicit system-defined stages.
- Exiting a session view must not implicitly cancel active session execution.
- A user must be able to leave and later revisit a running session while the runtime continues advancing that session in the background.
- Completion state may only be reached through explicit verification.
