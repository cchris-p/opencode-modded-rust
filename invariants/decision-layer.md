# Decision-Layer Invariants

These are binding truths for the System-1 decision layer (Jev, Laya, or any future
engine). Explanatory detail lives in `wiki/decision-layer.md`.

## Authority

- A decision provider PROPOSES. The runtime DISPOSES.
- A classifier MUST NOT grant or deny a permission by itself.
- A classifier MUST NOT change a task disposition, merge a change, close a board
  item, or weaken an invariant.
- Deterministic rulesets, invariants, and human gates remain authoritative.
- Every class MUST declare an authority tier: advisory, gated, delegated, or
  forbidden.
- No class MAY start above advisory. Promotion requires evaluation evidence and an
  explicit owner decision.

## Separation

- Decisions and generation MUST be separate calls.
- A decision provider MUST answer typed questions; it MUST NOT be required to
  generate prose, code, or plans.
- The runtime MUST be able to complete work with the decision provider disabled.

## Engine neutrality

- The stable core MUST NOT import a specific decision engine.
- Jev, Laya, deterministic rules, and future engines MUST implement the same
  decision boundary.
- Replacing one engine with another MUST NOT require a core-loop change.
- The engine client MUST be replaceable by a stub for tests.

## Typed, calibrated, budgeted, audited

- Every decision MUST have a declared input schema, output schema, confidence,
  engine, policy version, cost, and latency.
- Every decision MUST be recorded in an append-only log with provenance.
- Every class MUST have a cost and escalation budget.
- A decision result below the class confidence floor MUST escalate or fall back.
- A class MUST NOT run without an evaluation pack and an owner.

## Data and egress

- The decision log MUST follow `wiki/decision-log-egress.md`.
- Secrets MUST never be logged or sent to a hosted engine.
- Sensitive and secret classes MUST use a local engine or a deterministic rule.
- Hosted egress MUST require an allowed sensitivity tier, redaction, minimization,
  a permitted capability profile, and remaining budget.
- The decision log MUST NOT be synchronized to a remote store by default.

## Degradation and cost

- The degradation ladder MUST be: preferred engine -> local engine -> deterministic
  rules -> baseline behavior.
- A hosted-engine outage, timeout, quota, or `402` MUST degrade, not crash.
- A billing or credit error MUST be a stop-and-resume gate.
- A session that exhausts its decision budget MUST continue under the next rung.

## Evaluation

- Every class MUST be evaluated on hand-labelled or human-reviewed ground truth.
- Evaluation MUST include accuracy, precision/recall, calibration, cost, latency,
  and escalation rate.
- Evaluation MUST use held-out and out-of-distribution splits.
- A class MUST NOT be promoted on in-distribution gains alone.
- A null or negative result MUST be a valid outcome.

## Post-Jev

- Jev MUST be replaceable by a fine-tuned Laya per class.
- A swap MUST pass the swap test and the offline test before promotion.
- A class the model has absorbed MUST be demoted or removed through policy.
- Jev MUST NOT become a hard runtime dependency.
- The decision log MUST remain usable if the engine vendor changes or disappears.

## Relationship to existing invariants

- `invariants/verification.md` still applies: the implementer is not the final
  authority, and review runs in fresh context. A classifier cannot replace review.
- Permission authority still follows `invariants/coding-session-behavior.md` and
  the deterministic ruleset. A classifier is advisory input, not the ruleset.
- Provider selection still follows `invariants/providers.md`. A router may
  recommend; configuration and policy dispose.
- Workspace-intelligence provider/rung routing is a decision class. It MAY narrow a
  capability profile, MUST NOT widen it, and MUST NOT bypass WIP fallback or egress rules
  (`invariants/workspace-intelligence.md`).

## Governance

- Changing this invariant is a policy change and requires human review.
- New decision classes MUST be recorded as board items with an evaluation plan and
  a retirement criterion.
- No vendor, benchmark, or convenience exception overrides the authority rule.

## Related

- `wiki/decision-layer.md`
- `wiki/harness-substrate.md`
- `wiki/decision-log-egress.md`
- `wiki/post-jev-roadmap.md`
- `invariants/future-proofing.md`
- `invariants/verification.md`
