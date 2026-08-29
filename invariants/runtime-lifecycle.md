# Runtime Lifecycle Invariants

- The runtime owns lifecycle state transitions.
- The model may make bounded decisions inside the runtime, but it does not control the overall workflow.
- Tasks move through explicit system-defined stages.
- Completion state may only be reached through explicit verification.
