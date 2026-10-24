---
id: "FUTURE-PROOFING-017"
title: "Measure ScopeMux against generic retrieval and set keep-expand-retire criteria"
priority: "P1"
type: "research"
area: "FUTURE-PROOFING"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Measure ScopeMux against generic retrieval and set keep-expand-retire criteria

## Summary

Measure the Workspace Intelligence provider against the generic repository-local provider
on real tasks, and use the result to decide whether to keep, expand, or retire it.

This is the card that makes ScopeMux earn its place. It is a strategic bet, not an
invariant. Decision on 2026-10-24: **B + D (tiered, time-boxed)**.

## Why this exists

The current Qwen-only activation is an experiment. Without a measured comparison, the
product cannot tell whether the map saves tokens, improves task success, or just adds
complexity. The future-proofing plan requires an evidence-based keep/expand/retire
decision.

## Scope

- Build a task pack with at least:
  - a focused bug fix,
  - a multi-file feature,
  - a review-only task,
  - a repair task,
  - a subagent-delegated task.
- Run each task with:
  - the generic provider,
  - the WIP provider (same model, same budget, same task),
  - at least one local model and one frontier model where supported.
- Record:
  - input/output tokens and cost per successful task,
  - context precision/recall against a hand-checked relevant set,
  - task success and verification/review outcome,
  - p50/p95 retrieval and turn latency,
  - map freshness/staleness at request time,
  - fallback rate and fallback reason,
  - subagent token savings.
- Use held-out and out-of-distribution tasks, not only this repository.
- Write a report in `wiki/` or `docs/`, linked from this card.
- Record the keep/expand/retire decision and rationale.

## Keep/expand/retire decision (2026-10-24)

**Selected: B + D — tiered, time-boxed.**

- **B (tiered):** keep WIP as an optional profile for local models and subagents if it
  helps there; expand to other models only when it beats generic retrieval on both cost
  and outcome.
- **D (time-boxed):** first review when 20 real tasks and 10 subagent tasks are recorded,
  or after 6 weeks, whichever comes first. Fixed evaluation budget; no open-ended
  expansion before the review.
- **Expand** only if the comparison shows at least one of:
  - >= 10% token reduction per successful task,
  - a measurable task-success or review-quality improvement,
  and no task-success regression and no worse cost per successful task.
- **Keep tiered** if WIP helps local models/subagents but not frontier models.
- **Retire or keep experimental** if there is no measurable benefit after the review.
- The System-1 routing treatment arm (`DECISION-017`) is evaluated alongside static
  profiles. A null result there does not block the tiered keep; it only means static
  profiles are sufficient for routing.

Options retained for future review:

- **A. Strict:** retire or keep experimental if no measurable token or task improvement
  after the defined sample.
- **B. Tiered:** keep as an optional profile for local models and subagents; expand only
  on evidence.
- **C. Strategic:** keep regardless as part of the owned context/data pillar, but cap
  investment and measure annually.
- **D. Time-boxed:** keep for a fixed window with a fixed budget, then decide.

## Acceptance criteria

- The comparison runs end to end on the task pack.
- Results are committed as a report, not only in chat.
- The report separates harness effects from model effects.
- The B+D decision is recorded, and the 20-task / 10-subagent / 6-week review trigger is
  tracked.
- If the decision is expand, a follow-up profile/activation card is opened.
- If the decision is retire, a removal/fallback card is opened and the generic path is
  verified.
- No private code leaves the machine for a hosted service.

## Non-goals

- A public leaderboard.
- Proving WIP is better on every task.
- Tuning the map to the evaluation set.
- Replacing the decision-layer evaluation.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-015`
- `FUTURE-PROOFING-007` efficiency scoreboard
- `DECISION-006` per-class decision evaluation
- `SCOPE-007` subagent-scoped workspace intelligence
- `wiki/workspace-intelligence.md`
- `wiki/agent-evaluation-strategy.md`
