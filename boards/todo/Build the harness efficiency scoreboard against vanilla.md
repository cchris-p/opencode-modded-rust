---
id: "FUTURE-PROOFING-007"
title: "Build the harness efficiency scoreboard against vanilla"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/agent-evaluation-strategy.md"
status: "todo"
created: "2026-10-24"
---

# Build the harness efficiency scoreboard against vanilla

## Summary

Measure the claim that `ort` is a lower-overhead harness than vanilla OpenCode, on the
same real tasks, with the same models and config.

This is the scoreboard that justifies the product thesis. Without it, "more efficient"
is an architectural assertion.

## Why this exists

The harness substrate treats efficiency as a first-class, non-absorbable axis. The
current evaluation strategy ranks speed last and has no cost/token scoreboard. The
flywheel makes models better; it does not make the harness cheaper.

## Scope

Run both harnesses on at least five representative tasks:

- a focused bug fix,
- a small feature,
- a docs-plus-code change,
- a test repair,
- a context-heavy investigation.

Record per task and per harness:

- input and output tokens,
- cost (model + decision engine + tooling),
- prompt-cache hit rate,
- p50/p95 turn latency,
- startup time,
- peak RSS,
- binary and disk footprint,
- recovery behavior after interruption,
- task success and verification outcome.

Pin the model, provider, config, task pack, and hardware where possible. Record the
commit and policy version for both harnesses.

## Acceptance criteria

- A reproducible scoreboard script or procedure exists.
- Results are committed as a report, not only in chat.
- The report states which differences are harness-caused vs model-caused.
- Vanilla and `ort` run the same task pack.
- Cost per successful task is reported, not just raw token counts.
- The method is documented in `wiki/agent-evaluation-strategy.md`.

## Non-goals

- A public leaderboard.
- A permanent benchmark platform.
- Tuning either harness to win the scoreboard.
- Comparing unrelated models.
- Claiming a win from a single noisy run.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-006`
- `DECISION-001`
- `wiki/agent-evaluation-strategy.md`
- `wiki/harness-substrate.md`
- `INFRA-007` published binary (footprint evidence)
