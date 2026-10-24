---
id: "FUTURE-PROOFING-008"
title: "Define decision-log egress retention and privacy policy"
priority: "P1"
type: "docs"
area: "FUTURE-PROOFING"
spec: "wiki/decision-log-egress.md"
status: "todo"
created: "2026-10-24"
---

# Define decision-log egress retention and privacy policy

## Summary

Make `decision-log-egress` the enforced policy of record for the System-1 decision log:
what is recorded, which sensitivity tier may go to a hosted engine, how long records are
kept, how they are deleted, and how fine-tune exports are governed.

The policy text already exists in `wiki/decision-log-egress.md`. This card turns it into
an enforceable contract and a review gate.

## Why this exists

The decision log is the compounding data asset and the largest privacy risk. A hosted
engine sees state; a log can become a second private transcript store. Without a policy,
the layer cannot be trusted with sensitive work.

## Scope

- Ratify the schema and sensitivity tiers in `wiki/decision-log-egress.md`.
- Implement the `secret` prohibition: never log or send secrets, even redacted.
- Implement tier-based engine routing: hosted allowed only for `public`/`internal`.
- Implement redaction and state minimization per class.
- Implement retention and deletion commands, including dependent training exports.
- Implement the no-silent-sync rule.
- Add a policy check in the decision boundary and in CI for schema changes.
- Document the change-control process for the policy.
- Add tests for denied egress, retention expiry, deletion, and export lineage.

## Acceptance criteria

- A decision with a `secret` tier is rejected by the engine client and never written.
- A `sensitive` class is denied to a hosted engine by default.
- Redaction runs before egress, and a redaction failure blocks the request.
- Retention expiry and deletion are tested.
- A fine-tune export records its source ids and deletion behavior.
- The log is not synced to the session archive by default.
- A human review is required to change the policy.

## Non-goals

- Defining every future decision class.
- Building a hosted log service.
- Encrypting the local log beyond OS file permissions in the first slice.
- Sharing the log across machines by default.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-009`
- `DECISION-005`
- `DECISION-014`
- `wiki/decision-log-egress.md`
- `invariants/decision-layer.md`
