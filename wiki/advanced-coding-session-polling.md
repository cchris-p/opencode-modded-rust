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

## Background Sessions

Background coding sessions must be first-class polling targets.

A polling design should not assume the target session is visible in the TUI. It should be possible to observe and wait on a background session by a stable identifier or by another explicit handle.

TBD: exact handles for background sessions.

## Low-Context Waits

The waiting session should not need another session's full transcript by default.

The polling request should describe:

- what condition is being waited for
- which source is being observed
- how long or how often to check
- what result should be returned when the condition is met
- what should happen on timeout or failure

The polling result should be compact and source-backed.

TBD: exact result schema.

## Candidate Surfaces

- Runtime API for creating and tracking polling waits.
- Tool interface for agents to request a wait.
- TUI surface for showing pending waits and background session state.
- Board-aware workflow support for waiting on another card or session to reach a state.

TBD: which surface ships first.

## Candidate Observables

- Local git status and working tree state.
- Remote branch/ref existence and latest commit.
- GitHub PR state, merge state, review/check status, and pushed commits.
- Board item lane and metadata state.
- Runtime session status and published session result.
- Filesystem artifacts created by another coding session.

TBD: first implementation should choose a small subset.

## Constraints

- Polling should be bounded and explicit.
- Polling should not become hidden autonomous orchestration.
- Polling should report evidence for the observed condition.
- Polling should avoid consuming significant model context while waiting.
- Polling should preserve structured task/session authority rather than replacing it.

## Open Questions

- Should a poll be a durable runtime object?
- Should agents be allowed to sleep/wake on polling completion?
- Should the TUI show all outstanding polls globally or only for the active session?
- What is the minimal useful set of git/GitHub/board conditions?
- Should polling be allowed across repositories in V1?

## Related Planning

- `boards/todo/add-advanced-coding-session-polling.md`
- `invariants/coding-session-polling.md`
- `wiki/v1-runtime-loop.md`
- `wiki/v1-task-state.md`
