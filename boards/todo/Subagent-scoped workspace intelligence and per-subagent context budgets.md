---
id: "SCOPE-007"
title: "Subagent-scoped workspace intelligence and per-subagent context budgets"
priority: "P1"
type: "feature"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Subagent-scoped workspace intelligence and per-subagent context budgets

## Summary

Make workspace intelligence first-class for subagents: each subagent gets a map slice
scoped to its own objective, stage, seed nodes, and token budget, instead of inheriting
the parent's context.

This is the highest-value WIP use case: a small model or a spawned subagent cannot afford
a file dump.

## Why this exists

`ort` can spawn subagents (`task` tool, agent roles). Today a subagent's retrieval is not
explicitly scoped by its own task; it can inherit too much context or too little. A map
can anchor the subagent to the relevant nodes and return a compact delta.

## Scope

- Extend the retrieval request with a subagent identity and parent task id.
- Derive the subagent's seed nodes from its own objective, not the parent transcript.
- Enforce a per-subagent token and cost budget.
- Return a slice scoped to the subagent's stage and role.
- Keep the parent task record authoritative; the subagent result is evidence.
- Record the subagent's map request, provider, budget, and fallback.
- Add a fallback to generic retrieval when WIP is absent or the language is unsupported.
- Evaluate on at least one delegated coding task and one research/review task.

## Acceptance criteria

- A subagent request carries its own objective/stage/seeds/budget.
- The slice does not include unrelated parent context by default.
- The per-subagent budget is enforced and recorded.
- The parent can still complete when the subagent's map provider fails.
- Token usage per subagent is lower than the generic path on the evaluation task, or a
  null result is recorded.
- No authority moves to the subagent or the map.

## Non-goals

- A new subagent orchestration model.
- Changing the parent task lifecycle.
- Cross-repository maps.
- Auto-spawning subagents without user/policy control.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-015`
- `FUTURE-PROOFING-017`
- `DECISION-016`
- `DECISION-011` model-tier routing (subagent routing)
- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`
- `invariants/coding-session-behavior.md`
