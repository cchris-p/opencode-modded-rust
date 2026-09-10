---
id: "PHASE-002"
title: "Transport and runtime parity foundation"
priority: "P2"
type: "epic"
area: "PHASE"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-10"
---

# Transport and runtime parity foundation

## Summary

Close the structural parity gaps that let behavior diverge across providers and runtime paths, so the same agentic session semantics hold everywhere: Anthropic tool transport, a single session prompt loop, and a CLI/executor path that runs the same tool loop as the TUI/server path.

## Why this exists

`BUG-004`..`BUG-006` made the OpenAI-compatible chat path agentic. Equivalent behavior is still missing or divergent elsewhere: Anthropic cannot receive tools, the live v1 loop and the v2 `llm.rs` loop coexist, and the CLI `AgentExecutor` attaches the system prompt but never attaches tools or runs a tool loop. Left alone, these paths will keep drifting and re-breaking parity.

## Scope

- `FEAT-010` Anthropic provider tool transport parity.
- `FEAT-011` Consolidate the v1/v2 session prompt loops.
- `FEAT-012` CLI/`AgentExecutor` tool-loop parity.
- Keep `invariants/coding-session-behavior.md` and `wiki/coding-session-parity-audit.md` current as behavior changes.

## Done when

- Every supported coding-session entry path (TUI/server, CLI) attaches agent context + tools and executes a tool loop.
- Supported providers that advertise tool calling can actually receive and return tools.
- Exactly one session model-loop implementation is the source of truth.

## Related Items

- `FEAT-010` Anthropic provider tool transport parity
- `FEAT-011` Consolidate v1/v2 session prompt loops
- `FEAT-012` CLI/AgentExecutor tool-loop parity
- `BUG-006` DeepSeek tool loop (completed; OpenAI-compatible path)
- `START-008` Full parity deferred (boundary; this phase is targeted, not broad parity)

## Notes

- This is a targeted parity foundation, not the deferred broad parity of `START-008`.
- Prefer sharing one tool/context resolution path rather than duplicating per provider/entry.
