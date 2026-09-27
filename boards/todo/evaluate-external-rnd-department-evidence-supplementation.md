---
id: "FEAT-067"
title: "Evaluate the external R&D department's evidence supplementation"
priority: "P2"
type: "research"
area: "FEAT"
spec: "invariants/research-department.md"
status: "todo"
created: "2026-09-27"
---

# Evaluate the external R&D department's evidence supplementation

## Summary

Test whether the **Research Department** — consulting external/remote evidence
(papers, literature databases, web sources) — measurably improves research,
vetting, and evaluation over local-canonical-only reasoning.

This is the untested half of the Feynman question. `FEAT-065` returned a null
result, but it explicitly tested only **reasoning over a local canonical corpus**;
its own note records that the literature/web tools were unused. This card tests
the other objective: does an **advisory external evidence layer** add value by
supplementing local sources?

Feynman is a **probe harness** here, not a committed dependency. The durable
principle being tested is canonized in `invariants/research-department.md`; which
tooling serves the department stays an evidence-gated decision.

## Why this exists

- Local artifacts are **canonical** (textbook-derived references, gate criteria);
  as a set they can become heavy for a single agent to hold.
- External evidence is a **separate department of insight** that can be layered
  onto the research step conducted over those local sources.
- `FEAT-065` never exercised that department, so its null result cannot settle it.
- The department is **cross-domain** (not only finance). aa-studies is used here
  only because it is the one live workflow with real vetting questions.
- The alternative to using an external harness is to own a remote-evidence store
  and give `ort` the tools to operate within it. That is a **different** card,
  gated on this evaluation.

## Settled model (context, not in question)

- A **department** is a separation-of-concern area, not a model/provider
  boundary. Model choice does not define a department, and one model may operate
  both departments (`invariants/research-department.md`).
- External evidence is **advisory, confidence-labeled, provenance-bearing, and
  never canonical**.
- Local artifacts stay authoritative; area authority such as `GATE-001` remains
  unchanged. Research Department findings **inform** only.
- Which tooling serves the department (external harness vs product-native) is not
  settled by this card.

## Scope

Compare an external-evidence pass against local-only reasoning on **1–2 real,
external-evidence questions** that matter to the live workflow:

- **Arm A** — Feynman remote workflows (alphaXiv, literature databases, web/PDF,
  source-verifying agent) invoked explicitly for auditable provenance.
- **Arm B** — `ort` with its own web/URL tooling on the same questions.
- The question must genuinely require **outside** evidence (e.g. a robustness or
  selection-bias methodology decision), not something answerable from the local
  corpus — otherwise it repeats `FEAT-065`.

Headline criterion: does the external layer **change or sharpen an advisory
recommendation** a local-only pass would miss?

Secondary criteria:

- Citation quality and verifiability; source drift and dead-link risk.
- Provenance and auditability of each external claim.
- Domain fit — is the relevant (e.g. quant-finance) literature actually reachable,
  rather than only ML/LLM-centric sources?
- Cost, latency, and operational complexity.

## Required deliverable — invariant decision

This card must record an explicit decision on `invariants/research-department.md`:
keep as written, amend, or narrow. Draft the decision in the card/PR and **surface
it for approval** before it becomes binding; changing an invariant is a policy
change.

## Non-goals

- Making Feynman (or any harness/model/provider) a dependency.
- Folding Feynman features into `ort`.
- Editing aa-studies, opening/closing `GATE-001`, or changing any disposition.
- Building the Feynman→`ort` chunk pipeline (`FEAT-066`) or an owned remote
  evidence store/provider (separate, gated card).
- Committing private aa-studies content, prompts, or secrets.

## Done when

- At least one real external-evidence task is run through both arms with exact
  commands and observed outputs recorded.
- A clear verdict exists: the external department is **stronger, equal, or
  weaker** than local-only reasoning for this objective.
- The value-carrying components (and the noise) are identified.
- The invariant decision is recorded and surfaced for approval.
- Any follow-up (owned tooling, method adoption) is split into its own card or
  explicitly deferred.

## Recommended verification

- Confirm both arms ran end to end on the same questions before concluding.
- Confirm each external claim carries a verifiable source and provenance.
- Confirm local canonical authority was untouched (no disposition changed, no
  private content committed).
- Re-read `invariants/research-department.md` and reconcile it with
  `wiki/scopemux-integration-plan.md`'s "keep retrieval generic and local" stance.

## Related Items

- `FEAT-065` Evaluate whether Feynman's research method beats opencode/ort alone (local-corpus null result; predecessor)
- `FEAT-066` Automate the Feynman-to-ort workable-chunk pipeline via the CLI (hold)
- `SKILLS-002` Plan URL-backed skills parity (product-native external-source path)
- `START-025` Add retrieval-provider boundary for task context assembly
- `invariants/research-department.md`, `invariants/retrieval.md`, `wiki/scopemux-integration-plan.md`
- aa-studies `GATE-001` (external, private; live testbed)

## Notes

- Model is not the variable: one model can operate both departments, so a
  model-based comparison is not the point (`invariants/research-department.md`).
- Feynman is the probe, not the destination. The owned-tooling path is deferred
  until the department's value is measured.
- Keep the human in the loop: findings are advisory and no gate/authority changes.
