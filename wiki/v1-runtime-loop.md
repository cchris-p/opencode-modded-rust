# V1 Runtime Loop

### Purpose

Define the minimum runtime-owned workflow that makes `scopemux-code` usable as a serious personal daily-driver without falling back to an unconstrained chat loop.

This document translates `wiki/v1.md` and the current invariants into the concrete V1 execution model.

### Core Outcome

The V1 runtime is responsible for moving one bounded task through explicit stages from selection to completion.

The model may help inside a stage, but it does not decide whether a task skips stages, becomes complete without verification, or treats transcript history as authoritative state.

### Task Shape

V1 work is intentionally narrow.

Each active task must have:

- one explicit objective
- explicit completion criteria
- a bounded repository scope
- verification expectations appropriate to the task
- a current lifecycle stage owned by the runtime

The runtime may keep session history, chat transcript, and lightweight todos, but those support the task. They do not replace structured task state.

### Authoritative Runtime Stages

V1 uses the following task stages:

1. `selected`
   - A bounded task has been chosen or created.
   - Objective, completion criteria, and repository target exist before execution continues.
2. `context_prepared`
   - The runtime has assembled fresh working context for the current task.
   - Context is task-scoped, role-specific, and token-budgeted.
3. `implementing`
   - The model inspects the repository and makes the intended changes.
   - The runtime may pause here for approvals or user questions.
4. `verifying`
   - Task-relevant checks run against the produced change.
   - Verification is explicit and recorded, not assumed.
5. `reviewing`
   - A fresh-context review checks the result against task intent and verification outcome.
   - Review is separate from implementation.
6. `repairing`
   - The task reopens because verification failed, review found a blocking issue, or the result missed the task objective.
   - Repair returns to `context_prepared` or `implementing`, depending on how much context must be rebuilt.
7. `completed`
   - The runtime marks the task complete only after verification has run and review has not identified a blocking problem.

### Allowed Stage Transitions

- `selected -> context_prepared`
- `context_prepared -> implementing`
- `implementing -> verifying`
- `implementing -> context_prepared` when the task must be reframed before changes continue
- `implementing -> repairing` when an attempted change clearly failed or was interrupted in a task-relevant way
- `verifying -> reviewing` when required checks have run
- `verifying -> repairing` when checks fail or required checks were missing
- `reviewing -> completed` when the task result satisfies the objective, verification passed, and no blocking review issue remains
- `reviewing -> repairing` when review finds a blocking issue or missing work
- `repairing -> context_prepared`
- `repairing -> implementing` only when the existing task context is still valid and no rebuild is needed

The runtime must reject direct transitions that bypass `verifying` or `reviewing` on the way to `completed`.

### Minimum Authoritative Task State

V1 needs a structured task object with enough information to survive session exits and rebuild execution context safely.

At minimum, the runtime-owned task state must record:

- stable task ID
- objective
- explicit completion criteria
- repository or workspace target
- current stage
- task status summary
- planned verification commands or checks
- latest verification result
- latest review result
- reopen reason when the task is sent to `repairing`
- links to artifacts such as changed files, notes, or branch name when relevant

Conversation history is supporting evidence only. The transcript must not be the only place where the current objective, stage, or completion decision lives.

`START-016` should refine this into the durable persisted model and field-level authority rules.

### Context Construction Inputs

Context for a task is rebuilt from durable state plus repository facts, not from the full chat transcript by default.

The runtime should assemble task context from:

- the task objective and completion criteria
- the current lifecycle stage
- relevant repository files and symbols
- task-relevant invariants and wiki docs
- recent verification or review findings when reopening work
- only the minimum transcript excerpts that still matter for the current stage

Context rules for V1:

- prefer fresh stage-specific context over long accumulated conversation history
- include only the files and docs needed for the current task slice
- keep implementation context separate from review context
- preserve enough structured state that the TUI can leave and later revisit a live task without losing the authoritative task picture

### Implementation Boundary

The implementation stage is where repository discovery, code edits, and task-local execution happen.

Implementation may:

- inspect files and repository structure
- edit code, docs, or tests relevant to the bounded task
- run commands needed to produce the result
- ask for approval when the runtime requires it
- ask the user short clarifying questions when the task cannot proceed safely

Implementation may not:

- declare the task complete on its own
- treat a produced diff as sufficient evidence of success
- silently broaden the task beyond the recorded objective and completion criteria

### Verification Boundary

Verification is the first explicit quality gate after implementation.

Verification must:

- run task-relevant checks intentionally, not implicitly
- compare results against the task's stated completion criteria
- record pass, fail, or incomplete status in structured task state

Verification is not satisfied merely because a command executed. The runtime must retain whether the checks passed and whether they were sufficient for the task.

`START-017` should turn this boundary into enforced runtime behavior.

### Review Boundary

Review is a separate stage that uses fresh context and is not allowed to rely on the implementer's execution transcript as the main authority.

Review must examine:

- whether the task objective was actually met
- whether verification ran and was interpreted correctly
- whether the change introduced a blocking regression or omission
- whether the task should pass or reopen for repair

The reviewer is not required to restudy the whole repository. It should receive focused context built for review.

### Completion Criteria

A V1 task may reach `completed` only when all of the following are true:

- the task still matches a bounded explicit objective
- explicit completion criteria exist and are satisfied
- implementation work is finished for the current scope
- required verification ran
- verification passed or produced an explicitly acceptable result for the task
- fresh-context review did not identify a blocking issue
- the runtime records completion in structured task state

If any of those conditions fail, the runtime reopens the task into `repairing` instead of marking it complete.

### Session And TUI Expectations

- The TUI is the main V1 operator surface.
- Leaving the session view must not cancel active execution.
- Returning to the TUI must show the current authoritative task stage, not just a transcript summary.
- Approval prompts and follow-up questions are stage events inside the runtime loop, not ad hoc side channels.

`START-018` should complete the TUI interaction pieces needed for this loop.

### What This Leaves To Follow-Up Work

- `START-016` defines the durable task-state schema and authority rules.
- `START-017` enforces the verification and review gates in runtime behavior.
- `START-018` completes TUI approval and question handling for runtime stage events.
- `START-019` defines the minimum provider path required for dependable local-model execution in this loop.
- `START-021` and `START-022` define how this loop is evaluated with repeatable tasks and review fixtures.

### Bottom Line

V1 is not a freeform chat session with tools. It is a runtime-owned bounded-task loop with explicit state, focused context construction, mandatory verification, and separate fresh-context review before completion.
