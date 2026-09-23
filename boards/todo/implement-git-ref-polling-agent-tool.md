---
id: "FEAT-057"
title: "Implement the git ref/branch polling agent tool"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "wiki/advanced-coding-session-polling.md"
status: "todo"
created: "2026-09-23"
---

# Implement the git ref/branch polling agent tool

## Summary

Implement the first advanced coding-session polling slice: a bounded agent tool that waits on local and remote git ref/branch state and returns a compact, evidence-backed result.

This is the first implementation card split from `FEAT-007`, and it must not redefine the core polling concept. The contract is already fixed in `wiki/advanced-coding-session-polling.md` under "First Implementation Slice".

## Why this exists

Multi-agent coding work needs one session to wait until another session pushes a branch or advances a ref without repeatedly spending context on manual checks. This card delivers the smallest useful version of that wait.

## Scope

- Add a new tool under `crates/opencode-tool/src/` with id `wait_for_state` and register it in `crates/opencode-tool/src/registry.rs`.
- Implement `source: "git"` with conditions `branch_exists`, `remote_ref_exists`, `remote_ref_equals`, and `remote_ref_changed`.
- Enforce the request bounds from the spec: `timeout_ms` default `600000` / max `1800000`; `interval_ms` default `2000` / min `500` / max `30000`.
- Return the documented compact JSON result: `status`, `condition`, `observed`, `evidence`, `elapsed_ms`.
- Support `status` values `met`, `timeout`, `cancelled`, and `error`, including fast-fail on invalid arguments and cancellation via the tool cancellation token.
- Use the caller's workspace as the single repository scope; do not add cross-repository support.
- Keep the wait inside the tool call; do not add a durable runtime poll object or background poller.

## Non-goals

- PR, CI, board, session, or filesystem observables.
- A runtime poll registry, TUI surface, or auto-wake of paused sessions.
- Cross-repository polling.
- Streaming another session's transcript.

## Done when

- `wait_for_state` is available in the default tool registry.
- Each supported git condition is met and timed out correctly against a local test repository.
- Every wait is bounded and returns an explicit terminal status.
- The result is compact and evidence-backed, with no transcript import.
- Unit/integration tests cover `met`, `timeout`, invalid arguments, and cancellation.
- `wiki/advanced-coding-session-polling.md` matches the shipped request/result contract.

## Recommended verification

- Add tests beside the new tool covering each condition, timeout, invalid arguments, and cancellation.
- Run `cargo test -p opencode-tool`.
- Run `cargo check -p opencode-tool -p opencode-core`.
- Manually exercise a real branch wait with `ort-build` then `ort`, if the tool is reachable from the TUI session.

## Related Items

- `FEAT-007` Add advanced coding-session polling (design parent)
- `FEAT-058` Add PR and CI check observables to polling
- `FEAT-059` Add board lane and background-session observables to polling
- `FEAT-060` Add a runtime poll registry and TUI surface for outstanding polls

## Notes

- The spec section "First Implementation Slice" is authoritative for the contract; this card is the implementation plan.
- Prefer evidence as the actual probe command plus its raw observed value.
