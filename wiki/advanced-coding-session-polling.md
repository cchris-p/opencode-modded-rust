# Advanced Coding-Session Polling

## Intent

Advanced polling is a runtime direction for waiting on coding-session state with little or no conversational context.

The goal is to let one session or agent wait for a concrete external condition from another coding session, another agent, git, GitHub, CI, or the board, then continue with a compact result instead of spending context on repeated manual checking.

## Working Definition

Advanced polling means explicit, state-based waits during coding work.

Examples:

- wait until another agent pushes a branch
- wait until a git ref appears or changes
- wait until a PR exists, merges, or has checks available
- wait until CI/checks finish
- wait until a board item moves lanes
- wait until a background session publishes a result or reaches a runtime state

## First Implementation Slice

The first slice is intentionally narrow and was agreed in `FEAT-007`:

- Surface: a single agent tool registered in `crates/opencode-tool` (id `wait_for_state`), alongside `bash` and the other default tools in `crates/opencode-tool/src/registry.rs`.
- Observables: local and remote git refs and branch state only.
- Wait model: the tool call itself is the wait. The tool polls on a bounded interval and returns as soon as the condition is met or the timeout elapses. There is no durable runtime poll object and no background poller in the first slice.
- Wake model: no auto-wake. The calling session resumes when the tool returns. Polling does not pause or wake unrelated sessions.
- Background sessions: the tool is callable from any coding session, including background sessions. Observing another background session as a polling target is deferred.
- Scope: single repository, the caller's workspace. Cross-repository polling is deferred.

### Request Contract

The tool takes a JSON object:

| Field          | Type   | Required            | Default | Notes                                                        |
| -------------- | ------ | ------------------- | ------- | ------------------------------------------------------------ |
| `source`      | string | yes                 | -       | Only `git` is supported in the first slice.                  |
| `condition`   | string | yes                 | -       | One of `branch_exists`, `remote_ref_exists`, `remote_ref_equals`, `remote_ref_changed`. |
| `branch`      | string | for `branch_exists` | -       | Branch name, e.g. `feature/FEAT-015-preserve-model`.         |
| `ref`         | string | for ref conditions  | -       | Full ref name, e.g. `refs/heads/feat/x`.                     |
| `remote`      | string | no                  | `origin` | Remote name used for remote observations.                    |
| `expected_sha`| string | for equals/changed  | -       | Commit SHA the condition compares against.                   |
| `timeout_ms`  | number | no                  | `600000` | Bounded wait. Maximum `1800000`.                             |
| `interval_ms` | number | no                  | `2000`  | Poll interval. Minimum `500`, maximum `30000`.               |

### Condition Semantics

| Condition           | Met when                                                                 |
| ------------------- | ------------------------------------------------------------------------ |
| `branch_exists`     | The named branch resolves locally or on the configured remote.            |
| `remote_ref_exists` | The remote ref exists (`git ls-remote <remote> <ref>` returns a row).     |
| `remote_ref_equals` | The remote ref exists and its SHA equals `expected_sha`.                  |
| `remote_ref_changed`| The remote ref exists and its SHA differs from `expected_sha`.             |

### Result Contract

The result is compact JSON and evidence-backed. It never imports another session's transcript.

| Field        | Type   | Notes                                                              |
| ------------ | ------ | ------------------------------------------------------------------ |
| `status`     | string | One of `met`, `timeout`, `cancelled`, `error`.                     |
| `condition`  | object | Echo of the normalized request that was evaluated.                 |
| `observed`   | object | Observed ref/branch, SHA, and existence at the terminal poll.      |
| `evidence`   | array  | The commands or probes used and their raw observed values.         |
| `elapsed_ms` | number | Wall-clock time spent waiting.                                     |

### Bounds and Failure Behavior

- Every wait has an explicit deadline; there is no unbounded polling.
- Invalid arguments fail fast with `status: "error"` before any polling starts.
- A timeout returns `status: "timeout"` with the last observed state as evidence.
- Cancellation propagates through the tool cancellation token and returns `status: "cancelled"`.

## Background Sessions

Background coding sessions must be first-class polling targets in the final design.

A polling design should not assume the target session is visible in the TUI. It should be possible to observe and wait on a background session by a stable identifier or by another explicit handle.

In the first slice, background sessions are covered only as callers: the tool is not scoped to the currently visible TUI session. Targeting another background session as a polling source is tracked by `FEAT-059`.

## Low-Context Waits

The waiting session should not need another session's full transcript by default.

The polling request describes:

- what condition is being waited for
- which source is being observed
- how long or how often to check
- what result should be returned when the condition is met
- what should happen on timeout or failure

The resolved first-slice result schema is documented under [Result Contract](#result-contract). Results stay compact and source-backed.

## Candidate Surfaces

- Agent tool for requesting a bounded wait. **Chosen for the first slice** (`FEAT-007`, work item `FEAT-057`).
- Runtime API for creating and tracking durable polling waits. Deferred to `FEAT-060`.
- TUI surface for showing pending waits and background session state. Deferred to `FEAT-060`.
- Board-aware workflow support for waiting on another card or session to reach a state. Deferred to `FEAT-059`.

## Candidate Observables

- Local git status and working tree state. **Chosen for the first slice.**
- Remote branch/ref existence and latest commit. **Chosen for the first slice.**
- GitHub PR state, merge state, review/check status, and pushed commits. Deferred to `FEAT-058`.
- CI/check status. Deferred to `FEAT-058`.
- Board item lane and metadata state. Deferred to `FEAT-059`.
- Runtime session status and published session result. Deferred to `FEAT-059`.
- Filesystem artifacts created by another coding session. Deferred.

## Constraints

- Polling should be bounded and explicit.
- Polling should not become hidden autonomous orchestration.
- Polling should report evidence for the observed condition.
- Polling should avoid consuming significant model context while waiting.
- Polling should preserve structured task/session authority rather than replacing it.

## Resolved Questions

- **Which surface ships first?** The agent tool; runtime and TUI surfaces are deferred (`FEAT-060`).
- **What is the minimal useful condition set?** Local/remote git ref and branch state.
- **Should a poll be a durable runtime object?** Not in the first slice. The tool call is the wait.
- **Should agents sleep/wake on polling completion?** No auto-wake in the first slice. The tool returns to its caller.
- **Should polling be allowed across repositories in V1?** Deferred.

## Open Questions

- Should the TUI show all outstanding polls globally or only for the active session? (Tracked by `FEAT-060`.)
- What stable handles should identify a background session as a polling target? (Tracked by `FEAT-059`.)
- When should polling results become durable runtime state rather than a tool return value? (Tracked by `FEAT-060`.)
- Should PR/CI observables require explicit GitHub auth configuration before they are offered? (Tracked by `FEAT-058`.)

## Follow-Up Cards

- `FEAT-057` Implement the git ref/branch polling agent tool (first slice).
- `FEAT-058` Add PR and CI check observables to polling.
- `FEAT-059` Add board lane and background-session observables to polling.
- `FEAT-060` Add a runtime poll registry and TUI surface for outstanding polls.

## Related Planning

- `boards/todo/add-advanced-coding-session-polling.md`
- `invariants/coding-session-polling.md`
- `wiki/v1-runtime-loop.md`
- `wiki/v1-task-state.md`
