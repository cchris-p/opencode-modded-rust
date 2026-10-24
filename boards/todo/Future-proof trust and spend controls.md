---
id: "FUTURE-PROOFING-012"
title: "Future-proof trust and spend controls"
priority: "P2"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Future-proof trust and spend controls

## Summary

Make trust and spend explicit substrate features: scoped secrets, per-agent permission
blast radius, audit coverage, and hard spend caps.

The lecture names trust/security and efficiency as the parts of the harness the model
cannot absorb. Today they are spread across permission rules, env handling, and provider
accounts.

## Why this exists

A shell-plus-all-secrets harness can code fine, but it is not safe to delegate more work
to. As the decision layer and remote execution expand, the blast radius expands with
them. Trust must be a designed layer, not a policy document.

## Scope

- Define per-agent secret scopes and a way to deny access by default.
- Define permission blast radius: which tools, paths, networks, and external services an
  agent may touch.
- Add audit coverage for permission decisions, secret access, and spend.
- Add per-session, per-task, and per-day spend caps across model and decision-engine
  usage.
- Add a spend/credit gate that degrades or stops with a resumable state.
- Add a review for catastrophic actions and surprise-cost paths.
- Document the trust model and its limits.

## Acceptance criteria

- An agent cannot read a secret outside its scope.
- A spend cap stops or degrades gracefully and records why.
- Permission and secret access are auditable.
- The decision layer cannot widen a scope.
- The defaults are deny-oriented for new agents.
- A user can see what an agent is allowed to do before delegating.

## Non-goals

- Building an enterprise IAM system.
- Multi-tenant isolation.
- Hiding the permission model from the user.
- Replacing the existing deterministic ruleset.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-011`
- `DECISION-012`
- `invariants/coding-session-behavior.md`
- `invariants/decision-layer.md`
- `wiki/provider-side-visibility.md`
