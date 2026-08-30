# V1 Task State Model

## Purpose

Define the minimum durable task record the Rust runtime should own for V1 so bounded work survives session reload and does not depend on transcript reconstruction.

## Why This Is A Separate Record

The current repo already persists sessions, messages, and lightweight todos, but none of those structures are the right authority for bounded task execution:

- `SessionStatus` is a coarse session lifecycle for archival and completion handling.
- `RunStatus` is a transient execution indicator for whether a session is currently busy.
- transcript history is evidence and context, not the canonical source of current task intent or completion state.
- todos are useful substeps, but they are too thin to carry objective, stage, verification, review, and reopen decisions.

V1 should therefore persist a dedicated runtime-owned task record with one active task per session.

## Authoritative Record

The minimum V1 record should be persisted as one row keyed by `session_id` in a dedicated `session_tasks` table.

Recommended columns:

- `session_id`: unique foreign key to `sessions.id`; guarantees one active task per session for V1.
- `task_id`: stable task identifier owned by the runtime.
- `objective`: explicit bounded task objective.
- `completion_criteria`: serialized ordered list of concrete completion checks.
- `workspace_target`: repository or workspace path the task applies to.
- `stage`: current runtime stage.
- `verification_plan`: serialized ordered list of required commands or checks.
- `verification_status`: latest verification outcome as `not_run`, `passed`, `failed`, or `incomplete`.
- `verification_notes`: concise structured or freeform notes about the latest verification run.
- `review_status`: latest review outcome as `not_reviewed`, `approved`, or `changes_requested`.
- `review_notes`: concise notes from the latest review pass.
- `reopen_reason`: nullable reason for the current repair cycle.
- `artifacts`: serialized task-scoped references such as changed files, notes, or branch name.
- `created_at`: record creation timestamp.
- `updated_at`: last authoritative task-state update timestamp.

## Field Authority Rules

- `objective` is the only authoritative statement of what the current task is trying to achieve.
- `completion_criteria` is the only authoritative statement of what must be true before completion.
- `stage` is the only authoritative execution-stage field.
- `verification_plan` defines which checks count as required verification for the current task.
- `verification_status` and `verification_notes` are the authoritative verification result.
- `review_status` and `review_notes` are the authoritative review result.
- `reopen_reason` is required whenever the task returns to repair.
- `artifacts` may support execution and review, but they do not override objective, criteria, stage, verification, or review state.

If transcript text, todo rows, or UI state disagree with the task record, the task record wins.

## Stage Model

`stage` should use the same V1 runtime lifecycle defined in `wiki/v1-runtime-loop.md`:

- `selected`
- `context_prepared`
- `implementing`
- `verifying`
- `reviewing`
- `repairing`
- `completed`

## Automatic Versus Explicit Transitions

The runtime may perform these transitions automatically when their prerequisites are satisfied:

- `selected -> context_prepared` once objective, completion criteria, workspace target, and verification plan exist.
- `context_prepared -> implementing` once fresh task context has been assembled.
- `implementing -> verifying` once the implementation attempt for the current slice is complete enough to run required checks.
- `verifying -> reviewing` once all required verification checks have been recorded, even if they failed.

The runtime must require an explicit recorded outcome for these transitions:

- `implementing -> context_prepared` when the task needs reframing or rebuilt context.
- `implementing -> repairing` when execution fails, is interrupted in a task-relevant way, or produces a known bad result.
- `verifying -> repairing` when verification fails or required checks were missing.
- `reviewing -> completed` only after a recorded non-blocking review outcome.
- `reviewing -> repairing` when review finds a blocking issue or missing work.
- `repairing -> context_prepared` when reopening requires new context construction.
- `repairing -> implementing` only when the existing task context is still valid.

Every explicit reopen into `repairing` must write `reopen_reason` before the stage change is committed.

## Relationship To Existing Session Data

- `sessions` remains the durable container for session identity, history, lifecycle, permissions, and aggregate metadata.
- `session_tasks` becomes the durable container for the one active bounded task in that session.
- `messages` and `parts` remain transcript evidence and tool history.
- `todos` remain optional substeps attached to the session and may be derived from or linked to the active task, but are not authoritative task state.

This keeps session lifecycle, task execution state, transcript support, and todo support explicitly separated.

## Minimum Storage Changes

The minimum safe persistence change for V1 is:

- add a `session_tasks` table with a unique `session_id` foreign key to `sessions`
- serialize list-shaped fields such as `completion_criteria`, `verification_plan`, and `artifacts` as JSON text, matching existing repository patterns for complex fields
- load and save the task record alongside the existing session record during create, update, and resume paths
- leave transcript and todo tables intact rather than overloading them with task authority

This is safer than embedding the authoritative task record only inside session metadata because it gives the runtime a dedicated persistence boundary with explicit ownership and migration scope.

## Resume Expectations

On session reload, the runtime should restore the active task directly from `session_tasks`, then use transcript excerpts and repository facts only as supporting context for the current stage.

The runtime must not infer:

- the current objective from recent assistant text
- completion from a diff existing in the session
- verification success from command execution alone
- review approval from the absence of complaints in the transcript

## Follow-On Implementation Boundaries

- `crates/opencode-types` should own the serializable task record and result enums.
- `crates/opencode-storage` should own the `session_tasks` schema, repository methods, and storage tests.
- `crates/opencode-session` should load, update, and resume the task record without deriving authority from transcript history.
- API and TUI surfaces should display the current task stage and latest verification and review outcomes from the durable record.
