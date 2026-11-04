# opencode-permission

`opencode-permission` provides a unified permission decision layer for constraining high-risk tool operations.

## Responsibilities

- Rule set definition and parsing
- Operation arity/granularity classification
- Allow / deny / ask decisions
- Integration with plugin hooks (`PermissionAsk`)

## Module structure

- `ruleset.rs` – Rule structure, matching, parsing
- `arity.rs` – Operation granularity and parameter categories
- `engine.rs` – Permission engine logic

## Use cases

- `opencode-tool` pre-execution permission check
- `opencode-session` in-session approval flow
- TUI/Server permission request and response

## Non-interactive execution

`AgentExecutor` supplies a fail-closed ask callback for every tool context. It
evaluates each requested pattern against the agent ruleset:

- `allow` proceeds;
- `deny` returns `ToolError::PermissionDenied`;
- `ask` also fails closed because a non-interactive run has no user to prompt.

This keeps plain `opencode-modded-rust run` (and the GitHub run path) consistent
with the TUI/server permission callback. Use the TUI or a server session when an
`ask` decision must reach a human.

## Development notes

- New rules should start from a minimal default policy
- Permission results should be explainable (matched rule, action, target)
- Avoid hardcoding business logic in the rule engine

## Validation

```bash
cargo check -p opencode-permission
```
