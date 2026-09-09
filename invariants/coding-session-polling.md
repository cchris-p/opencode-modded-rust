# Coding-Session Polling Invariants

- Coding-session polling waits on explicit observable state, not on vague transcript interpretation.
- Polling must be bounded by a declared condition and timeout or cancellation path.
- Polling must support background coding sessions as explicit targets.
- Polling results must be compact and evidence-backed.
- A polling result must not import another session's full transcript unless explicitly requested.
- Polling must not replace structured task state as the authority for task objective, stage, verification, or completion.
- Polling must preserve user control over when another session's result is adopted into the current task.
- TBD items in the polling design must remain unresolved planning notes until converted into explicit board work or invariants.
