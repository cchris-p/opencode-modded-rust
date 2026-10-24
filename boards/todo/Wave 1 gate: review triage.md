---
id: "DECISION-008"
title: "Wave 1 gate: review triage"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 1 gate: review triage

## Summary

Classify review findings as:

- blocking,
- non-blocking,
- noise,
- approve.

Use the classification to reduce review spam, order fixes, and give the operator a
compact verdict.

## Why this exists

Review is a required stage (`invariants/verification.md`), but its output is often a
long list where real blockers are mixed with style noise. A calibrated triage decision
makes review faster and more useful without weakening the human/fresh-context gate.

## Why Wave 1

Review triage is independent of the task-stage rewrite and has a clear outcome signal:
did a human later confirm the finding was real?

## Scope

- Define the input: review finding, task intent, diff summary, verification evidence.
- Define the classes and the action mapping.
- Keep the human/fresh-context review authoritative; the classifier only triages.
- Record the classification and the later human confirmation as an outcome label.
- Evaluate against a labelled review set, including false-blocking and missed-issue cases.
- Authority: `advisory` first. It may reorder or collapse display, but may not suppress a
  finding from the operator without a policy decision.

## Acceptance criteria

- A labelled review set exists with human-confirmed outcomes.
- Blocking recall is high enough that no real blocker is hidden in shadow.
- Noise classification is measured separately from blocking recall.
- The operator can always expand the full review.
- The outcome label is logged for future fine-tuning.
- No classifier approval replaces the review gate.

## Non-goals

- Replacing the reviewer agent.
- Changing `invariants/verification.md`.
- Auto-merging on approval.
- Judging code style without a defined rubric.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-007`
- `invariants/verification.md`
- `wiki/agent-evaluation-strategy.md`
- `wiki/decision-layer.md`
