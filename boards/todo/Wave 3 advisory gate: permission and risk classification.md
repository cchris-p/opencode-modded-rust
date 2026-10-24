---
id: "DECISION-012"
title: "Wave 3 advisory gate: permission and risk classification"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "invariants/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 3 advisory gate: permission and risk classification

## Summary

Add an advisory classifier that estimates the risk of a proposed tool action:

- read-only,
- reversible write,
- destructive write,
- external side effect,
- secret/credential access.

The classifier **does not** grant or deny permission. The deterministic ruleset and human
approval remain authoritative.

## Why this exists

The permission ruleset is pattern-based. It can say "bash is allowed" but not "this
specific command is catastrophic." An advisory risk signal can improve prompts, defaults,
and review without weakening the deterministic authority.

## Why Wave 3

This is safety-critical. It must come after the decision log, calibration, and shadow
infrastructure are proven. A false "safe" is much worse than a false "unsafe."

## Scope

- Define the input: tool, command/path, workspace, session intent, permission context.
- Define risk classes and the advisory action:
  - annotate,
  - require stronger confirmation,
  - recommend denial,
  - recommend review.
- Keep the deterministic ruleset as the only authority.
- Never let a classifier widen a permission scope.
- Log the classification, confidence, and the actual permission outcome.
- Evaluate on a labelled set of risky and benign actions, with emphasis on recall of
  catastrophic actions.
- Prefer local Laya for state containing secrets or private paths.
- Authority: `advisory` only. Promotion beyond advisory is out of scope.

## Acceptance criteria

- The ruleset still decides allow/ask/deny.
- A classifier cannot widen a scope.
- Catastrophic-action recall is measured and reviewed.
- False-safe cases are tracked as incidents and blocked from promotion.
- Sensitive state never goes to a hosted engine.
- The operator can see the advisory reason.

## Non-goals

- Replacing the permission engine.
- Auto-allowing low-risk actions.
- Classifying user intent.
- Changing `invariants/coding-session-behavior.md` without review.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-013`
- `FUTURE-PROOFING-012`
- `invariants/decision-layer.md`
- `invariants/coding-session-behavior.md`
- `crates/opencode-permission/src/engine.rs`
- `crates/opencode-permission/src/ruleset.rs`
