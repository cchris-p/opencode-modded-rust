---
id: "DECISION-013"
title: "Wave 3 advisory gates: safety screening and memory write policy"
priority: "P2"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 3 advisory gates: safety screening and memory write policy

## Summary

Add two advisory classifiers:

1. **Safety screening** — prompt injection, jailbreak, sensitive-data exposure, or
   unsafe external action.
2. **Memory write policy** — should this fact be persisted, recalled, or forgotten?

Both are advisory and local-first.

## Why this exists

As the harness reads external content and delegates more work, prompt injection and
unintended persistence become real risks. A cheap local classifier can flag them without
sending sensitive state to a hosted engine.

## Scope

- Safety:
  - input: external content, tool output, proposed action, source provenance;
  - classes: benign, suspicious, injection, sensitive-data, unsafe-action;
  - action: annotate, quarantine, require confirmation, block pending review;
  - local-first; no hosted egress for sensitive content.
- Memory:
  - input: candidate fact, source, sensitivity, task relevance, existing memory;
  - classes: persist, recall-only, forget, do-not-store;
  - enforce retention and egress policy.
- Log decisions and outcomes; include human corrections.
- Evaluate on labelled injection and memory cases.
- Authority: `advisory`; quarantine/block actions require a deterministic or human gate.

## Acceptance criteria

- Injection detection is measured on a labelled set, including false positives.
- Sensitive content is screened locally, not by a hosted engine.
- A `do-not-store` decision is enforced by the memory writer.
- The operator can inspect and override.
- A classifier cannot delete durable data on its own.
- Memory decisions follow `wiki/decision-log-egress.md`.

## Non-goals

- A general content-moderation system.
- Replacing the permission engine.
- Auto-deleting user data.
- Hosted screening of private content.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-012`
- `FUTURE-PROOFING-008`
- `FUTURE-PROOFING-012`
- `wiki/decision-log-egress.md`
- `wiki/laya-integration.md` (local presets)
