---
id: "FUTURE-PROOFING-004"
title: "Version and canary the policy layer independently"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Version and canary the policy layer independently

## Summary

Make prompts, skills, shims, thresholds, capability profiles, and decision policies
versioned independently of the binary, with canary, replay, and rollback.

This is the change-management answer: iterate policy quickly without turning every tweak
into a release event, and without letting user-visible internals become contracts.

## Why this exists

Policy changes are the most frequent changes and the most likely to regress behavior.
If they require a binary release, iteration is slow and rollback is expensive. If they
are unversioned, a session cannot be reproduced or audited.

## Scope

- Define a policy version identifier and a manifest of policy components.
- Record the policy version on the session and in the decision log.
- Add a canary mechanism: apply a policy version to a subset of sessions/tasks.
- Add replay: run a decision log or task pack against a candidate policy before
  promotion.
- Add rollback: select the previous policy version without rebuilding.
- Add a compatibility check for policy schemas and decision-class schemas.
- Document the promotion flow: draft -> canary -> replay -> promote -> monitor -> roll
  back if needed.

## Acceptance criteria

- Every session records its policy version.
- A policy change can be canaried without a binary release.
- A decision log can be replayed under a candidate policy.
- Rollback is one action and is tested.
- An incompatible policy schema fails fast with a clear error.
- The user-facing surface does not expose internal policy identifiers.

## Non-goals

- Building a hosted policy service.
- Making the binary hot-swappable.
- Versioning every prompt string individually.
- Changing the current config precedence rules.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-003`
- `FUTURE-PROOFING-005`
- `DECISION-005`
- `wiki/harness-substrate.md`
- `invariants/future-proofing.md`
