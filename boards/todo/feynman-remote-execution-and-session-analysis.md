---
id: "RESEARCH-006"
title: "Feynman remote execution and session-analysis surface"
priority: "P1"
type: "gate"
area: "RESEARCH"
spec: "invariants/research-department.md"
status: "todo"
created: "2026-09-28"
---

# Feynman remote execution and session-analysis surface

> **SUPERSEDED / MERGED 2026-09-28.** This item is merged into **`FEYNMAN-001`**
> (`feynman` repo, `boards/todo/feynman-via-opencode-research-department.md`).
> Track it there; the cross-repo runbook is
> `feynman/handoffs/feynman-via-opencode-research-loop-handoff.md`.

## Summary

Make Feynman **remotely executable** and its **chat sessions analyzable**, so the
consultative `research-department` step of the research loop can produce a
**recorded, auditable disposition** with provenance.

This is a **gate item** for the locked-in research loop
(`docs/research/research-loop-lock-in-proposal.md`): the consultation sub-step is
consultative, and "consultative" only means something if the consultation is
capturable and attributable. Without these two capabilities, the `research-department`
step emits nothing the premise gate can consume, so the loop cannot run.

## Why this exists

- The loop's premise gate consumes a **recorded disposition**
  (`support` / `refute` / `ambiguous` / `nothing` / `error`). That disposition
  must be derivable from a Feynman session that was (a) invoked programmatically
  from the loop and (b) captured/parsed after the fact.
- An interactive, non-capturable Feynman run cannot be audited, cannot be
  attributed, and therefore cannot feed a gate — it stays a narrative, which is
  exactly the failure mode the loop exists to remove.

## Scope

Two hard prerequisites, tracked as one gate:

1. **Remote execution** — a documented, repeatable invocation path that runs
   Feynman from the loop (CLI / programmatic), not a manual interactive session.
2. **Session analysis** — a documented capture + parse path that turns a Feynman
   session into a recorded, auditable disposition with provenance (command,
   model, output, disposition).

## Non-goals

- Vendoring the Feynman application into the product binary.
- Making Feynman, a model, or a provider a runtime dependency.
- Granting external evidence canonical authority (it stays advisory).
- Auto-promotion or changing any area/gate disposition.

## Done when

- A documented, repeatable remote-invocation path exists and is demonstrated.
- A documented capture/parse path exists and yields a recorded disposition with
  provenance.
- The consultative step's disposition contract (`support` / `refute` /
  `ambiguous` / `nothing` / `error`) is implementable on top of both.
- The advisory/authority boundary is enforced and documented.

## Related

- aa-studies `INFRA-053` (the `RSCH` derivation + external-consultation sub-loop
  this gates; the loop is defined in aa-studies, not here)
- `RESEARCH-004` (broader Feynman/Research-Department utility surface in `ort`;
  this card is the execution/analysis prerequisite that `RESEARCH-004`'s
  "invocation utilities" + "artifact discipline" depend on)
- `RESEARCH-003` (external-evidence value gate), `RESEARCH-005` (internal
  derivation vetting), `RESEARCH-002` (chunk pipeline, hold)
- `invariants/research-department.md`

## Notes

- Model is not the variable; Feynman is the reference harness, not a required one.
- Keep the human in the loop; findings are advisory.
