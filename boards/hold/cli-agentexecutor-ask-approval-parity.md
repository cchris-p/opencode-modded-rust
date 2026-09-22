---
id: "CLI-011"
title: "CLI/AgentExecutor ask and approval parity"
priority: "P2"
type: "feature"
area: "CLI"
spec: "invariants/coding-session-behavior.md"
status: "hold"
predecessors: "GATE-002, CLI-002"
created: "2026-09-22"
updated: "2026-09-22"
---

# CLI/AgentExecutor ask and approval parity

## Blocked By

- `GATE-002` Gate: Question tool must match vanilla OpenCode exactly
  (`boards/todo/gate-question-tool-full-parity.md`). The CLI ask/approval path must reach full vanilla
  question parity before it can be completed.
- `CLI-002` Route `opencode run` through the canonical session runtime. The CLI model loop/routing decision
  must land first so ask handling attaches to one loop, not two.

## Summary

Split out of the original `CLI-002` "CLI/AgentExecutor tool-loop parity" on 2026-09-22. The `opencode run`
CLI path must handle ask-gated tools through a real ask/approval (question) path instead of returning an
explicit permission error with no user surface.

## Why this exists

`CLI-002` proved the CLI tool loop can execute tools, but ask-gated tools still fail with a permission error
because no ask UI is wired on that path. Full question parity is owned by `GATE-002`; this card is the CLI
consumer of it.

## Scope

- Provide or reuse a real ask/approval path on the CLI/`AgentExecutor` surface instead of hard-denying or
  erroring on `Ask` tools.
- Reuse the shared question/prompt-state logic rather than duplicating divergent behavior.
- Match vanilla's direct-run question display/behavior where a CLI run surface exists.

## Non-goals

- The CLI model loop/routing decision itself (that is `CLI-002`).
- TUI question prompt UX (`FEAT-041`).
- Defining the question-tool parity baseline (`GATE-002`).

## Done when

- Ask-gated tools on the CLI path are not silently hard-denied or errored without a user path.
- The CLI ask/approval behavior is verified against the frozen vanilla reference.

## Recommended verification

- `cargo fmt --all`
- `cargo check -p opencode-agent -p opencode-cli`
- Manual smoke through the CLI direct-run surface once `CLI-002` lands.

## Related Items

- `CLI-002` Route `opencode run` through the canonical session runtime - predecessor.
- `GATE-002` Gate: Question tool must match vanilla OpenCode exactly - blocking gate.
- `CLI-009` CLI and direct-run question parity - sibling; broader direct-run question routing.
- `FEAT-041` TUI question prompt UX parity - shared prompt logic.
