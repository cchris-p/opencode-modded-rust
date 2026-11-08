---
id: "DECISION-004"
title: "Implement the Jev-Laya engine client and degradation ladder"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Implement the Jev-Laya engine client and degradation ladder

## Summary

Implement the Jev and Laya adapters behind the decision boundary, plus the degradation
ladder: preferred engine -> local engine -> deterministic rules -> baseline behavior.

Jev is the preferred hosted engine. Laya is the local and fine-tuning path. Neither is a
hard runtime dependency.

## Why this exists

The boundary is only real if two engines can implement it and if the harness survives
one of them failing. This card proves engine neutrality and availability.

## Scope

- Jev adapter: typed request/response, model selection, auth from the managed env,
  timeouts, retries, credit metering.
- Laya adapter: local HTTP or in-process path, health check, model/checkpoint selection.
- Redaction and state minimization before any hosted call.
- Sensitivity-tier routing: `sensitive`/`secret` never go to Jev.
- Budget enforcement per class and per session.
- `402`/quota/insufficient-credit handling as stop-and-resume, not crash.
- Degradation to Laya, then deterministic rules, then baseline.
- Record engine, cost, latency, and fallback reason in the decision record.
- Secrets from OpenBao-managed env; never commit or log them.
- Tests with a mock server; no live credentials in CI.

## Acceptance criteria

- The same decision class runs through Jev, Laya, and a stub without code changes.
- A hosted failure degrades to local/rule behavior and completes the task.
- A `402` stops cleanly and resumes without repeating completed decisions.
- A sensitive class never leaves the machine.
- Budgets are enforced and recorded.
- No secret appears in logs, errors, or the decision record.
- The offline test passes.

## Non-goals

- Fine-tuning Laya (that is `DECISION-014`).
- Benchmarking accuracy (that is `DECISION-006`).
- Building a new HTTP service for ort.
- Supporting every decision engine.

## Related

- `DECISION-001`
- `DECISION-003`
- `DECISION-005`
- `DECISION-014`
- `FUTURE-PROOFING-008`
- `wiki/decision-layer.md`
- `wiki/decision-log-egress.md`
- `STD-089` Jev/Laya umbrella
