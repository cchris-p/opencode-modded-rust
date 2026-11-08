---
id: "DECISION-009"
title: "Wave 1 gates: compaction trigger and retrieval sufficiency"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 1 gates: compaction trigger and retrieval sufficiency

## Summary

Add two context decisions:

1. **Compaction trigger** — should this turn compact now, and which spans must survive?
2. **Retrieval sufficiency** — is the current context enough, or which source should be
   read next?

Both reduce token cost and avoid wasteful reads. Both are advisory until evaluated.

## Why this exists

Context assembly is the main harness cost and the main local-model reliability lever.
The current runtime uses provider context limits and fixed behavior; it does not decide
whether more context is worth the tokens. `BUG-009` (instruction re-injection) was
deferred because a 1M-token model hid the cost; local models do not hide it.

## Scope

- Define compaction input: token count, turn shape, task state, open tool results,
  policy budget.
- Define compaction classes: compact now, defer, compact with retention set.
- Define retrieval input: task intent, current context, candidate sources, ScopeMux
  nodes, budget.
- Define retrieval classes: sufficient, read one more, read a different source, ask.
- Integrate behind the decision boundary; log all decisions.
- Evaluate on real sessions: tokens saved, task success, review quality, retries.
- Authority: `advisory` first; `gated` only when the action is reversible and budgeted.

## Acceptance criteria

- A compaction decision records what was kept and why.
- A retrieval decision records the chosen source and the reason.
- Token savings are measured against the current behavior.
- No task regression in the evaluation pack.
- A low-confidence decision falls back to current behavior.
- Decisions are replayable from the log.

## Non-goals

- Replacing the compaction agent's generation.
- Building ScopeMux itself.
- Forcing compaction on every turn.
- Changing the model context window.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-010`
- `BUG-009` instruction files re-injected on every read
- `START-025` retrieval-provider boundary
- `SCOPE-006` replace code grep with ScopeMux node search
- `wiki/decision-layer.md`
