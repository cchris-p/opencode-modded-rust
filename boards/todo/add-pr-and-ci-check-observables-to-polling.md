---
id: "FEAT-058"
title: "Add PR and CI check observables to polling"
priority: "P3"
type: "feature"
area: "FEAT"
spec: "wiki/advanced-coding-session-polling.md"
status: "todo"
created: "2026-09-23"
---

# Add PR and CI check observables to polling

## Summary

Extend the advanced polling tool with GitHub PR and CI/check observables so a session can wait until a PR exists, merges, or has checks available or finished.

This is a follow-up card split from `FEAT-007`; it extends the observables of the existing tool without changing the core polling concept.

## Why this exists

Git refs cover "a branch was pushed", but agent workflows also need to wait on PR and check state. Today that requires broad manual polling and context-heavy checks.

## Scope

- Add a GitHub source path to the polling contract once `FEAT-057` ships.
- Support waiting on PR existence, merge state, and check/CI completion.
- Reuse the compact evidence-backed result shape and bounded timeout/interval behavior.
- Define behavior when GitHub auth or a remote origin is unavailable, and fail fast with `status: "error"` and a clear reason.

## Non-goals

- Board, session, or filesystem observables.
- A runtime poll registry or TUI surface.
- Cross-repository polling beyond resolving the caller's own remote.

## Done when

- PR and CI conditions are available and documented in `wiki/advanced-coding-session-polling.md`.
- Missing auth/remote fails fast with actionable evidence.
- Tests cover met, timeout, and auth-failure paths without requiring live GitHub access.

## Recommended verification

- Run `cargo test -p opencode-tool`.
- Confirm the tool still passes the `FEAT-057` git-condition tests.

## Related Items

- `FEAT-007` Add advanced coding-session polling (design parent)
- `FEAT-057` Implement the git ref/branch polling agent tool

## Notes

- Decide explicitly whether GitHub observables require configured auth before being offered; record the answer in the spec.
