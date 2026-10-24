---
id: "DECISION-011"
title: "Wave 2 gates: model-tier routing and board-task classification"
priority: "P2"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 2 gates: model-tier routing and board-task classification

## Summary

Add two cost/orchestration decisions:

1. **Model-tier routing** — choose a cheap or strong model for the next step, and
   escalate on low confidence or task complexity.
2. **Board/task classification** — suggest a lane, type, or next action for a task.

Both are advisory at first.

## Why this exists

Routing is the direct efficiency lever: most steps do not need the strongest model. Board
classification is the software-factory lever: a bug report can be triaged without a
human rewriting the card.

## Scope

- Routing:
  - input: task stage, complexity signals, current model, budget, confidence, failure
    history;
  - classes: cheap, standard, strong, escalate, stop;
  - hard caps: cost per task, escalation rate;
  - fallback: current configured model.
- Board/task:
  - input: task text, repository state, existing cards, lane rules;
  - classes: lane suggestion, type, next action, duplicate/related;
  - never mutate a disposition; only suggest.
- Record decisions and outcomes.
- Evaluate routing on cost per successful task and task success.
- Evaluate board classification on human acceptance rate.

## Acceptance criteria

- Routing reduces cost per successful task on the task pack without a success regression.
- Escalation is bounded and observable.
- A routing decision can be replayed.
- Board suggestions are advisory and never auto-move a card.
- A human acceptance rate is measured.
- The configured model remains the safe fallback.

## Non-goals

- Changing the default model automatically.
- Auto-merging or auto-closing board items.
- Building a pricing optimizer.
- Routing across providers without auth/budget checks.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-009`
- `FUTURE-PROOFING-007`
- `wiki/decision-layer.md`
- `wiki/agent-evaluation-strategy.md`
