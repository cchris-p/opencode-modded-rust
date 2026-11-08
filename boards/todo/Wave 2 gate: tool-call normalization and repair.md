---
id: "DECISION-010"
title: "Wave 2 gate: tool-call normalization and repair"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 2 gate: tool-call normalization and repair

## Summary

Add a decision gate for malformed, leaked, split, or ambiguous tool calls:

- valid, execute,
- repairable, repair and execute,
- ambiguous, ask or re-prompt,
- invalid, return a structured tool error.

This is a local-model reliability lever: small models leak tool syntax into content and
split arguments across deltas more often than frontier models.

## Why this exists

`BUG-060`, `BUG-063`, and the `EPIC-001` history show provider/tool transport defects
that surface as wasted turns or silent failures. The parser fixes are deterministic, but
deciding whether a payload is repairable vs invalid is a classification.

## Why Wave 2

It depends on the tool transport stabilizing and on the decision boundary being in place.
It is higher risk than failure classification because a bad repair can execute the wrong
tool.

## Scope

- Define the input: raw tool-call payload, provider, model, parser state, expected schema.
- Define classes: valid, repairable, ambiguous, invalid, unsafe.
- Define repair actions: reparse, split, quote, drop, ask, error.
- Require a schema check and a permission check before execution, regardless of the
  classifier.
- Record the decision and the repair in the log.
- Evaluate against recorded malformed fixtures and shadow runs.
- Authority: `advisory` first; `gated` only for repairs that are schema-validated and
  permission-checked.

## Acceptance criteria

- Known malformed fixtures are classified and repaired or rejected correctly.
- No repair can bypass the permission ruleset.
- An ambiguous call asks rather than guesses when confidence is low.
- The original payload and the repair are both recorded.
- A repair that fails schema validation becomes a structured tool error.
- The local-model failure rate improves without a frontier-model regression.

## Non-goals

- Fixing provider parsers.
- Adding new tools.
- Auto-executing unvalidated repairs.
- Classifying user intent.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-007`
- `BUG-060` DeepSeek native tool-call normalization
- `BUG-063` DeepSeek native tool-call leak
- `openai-chat-tools-wire-format`
- `wiki/decision-layer.md`
