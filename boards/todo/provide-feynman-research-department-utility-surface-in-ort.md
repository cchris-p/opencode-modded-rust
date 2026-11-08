---
id: "RESEARCH-004"
title: "Provide the Feynman/Research-Department utility surface in ort"
priority: "P2"
type: "feature"
area: "RESEARCH"
spec: "invariants/research-department.md"
status: "todo"
created: "2026-09-27"
---

# Provide the Feynman/Research-Department utility surface in ort

## Summary

`ort` provides the utilities and standards that make instantiating the **Research
Department** — with **Feynman as the reference harness** — easy in any workspace.
This turns the department into a first-class capability of the ort stack rather
than an out-of-band tool that must be rebuilt per project.

This is the product-side home for "instantiate Feynman wherever it is needed." The
durable principle is `invariants/research-department.md`; the value question is
`RESEARCH-003`.

## Why

- The Research Department is cross-domain; every project that needs external
  evidence should be able to instantiate it the same way.
- Rebuilding it ad hoc loses the provenance/authority discipline and duplicates
  work.
- Keeping the utilities in `ort` keeps the department integrated with the stack
  and consistent with the advisory, non-canonical rule.

## What the surface includes (candidate)

- **Instantiation standard** — a short runbook/spec for standing the department up
  in a workspace: which harness, how it is invoked, how provenance is captured,
  and the advisory/authority boundary.
- **Source plumbing** — URL-backed skill sources (`SKILLS-002`) so
  external-research skills can be sourced without vendoring.
- **Retrieval integration** — expose external/advisory evidence through the
  `START-025` retrieval-provider boundary as a distinct, confidence-labeled
  provider (`invariants/retrieval.md`).
- **Invocation utilities** — a sanctioned, provenance-recording way to run a
  research harness from `ort` (for example a skill or thin CLI wrapper), including
  the canonical `opencode task` path rather than the interim `opencode run` where
  applicable.
- **Artifact discipline** — a default place/format for research outputs so results
  are auditable and never silently canonical.

## Gate / dependency

- Value is established by `RESEARCH-003` (external/remote-evidence
  supplementation). The full build waits on a positive verdict; the low-risk
  standard/doc piece may proceed independently.
- Independent of `RESEARCH-002` (chunk pipeline), which is a separate consumer.

## Non-goals

- Vendoring the Feynman application into the product binary.
- Making any harness, model, or provider a runtime dependency.
- Granting external evidence canonical authority.
- Auto-promotion or changing any area/gate disposition.

## Done when

- The instantiation standard exists and is linked from
  `invariants/research-department.md`.
- At least one workspace can instantiate the department through the `ort` surface
  with recorded provenance and no private-data leakage.
- The advisory/authority boundary is enforced and documented.
- Follow-ups are split into their own cards or explicitly deferred.

## Related

- `RESEARCH-003` Evaluate the external R&D department's evidence supplementation (value gate)
- `RESEARCH-002` Automate the Feynman-to-ort workable-chunk pipeline via the CLI (consumer)
- `RESEARCH-001` Feynman vs opencode/ort null result (local-vetting predecessor)
- `SKILLS-002` URL-backed skills parity; `START-025` retrieval-provider boundary
- `invariants/research-department.md`, `invariants/retrieval.md`,
  `wiki/scopemux-integration-plan.md`
- aa-studies `INFRA-051` (loop stage that consumes the department)

## Notes

- Model is not the variable: one model can operate both departments.
- Feynman is the reference harness, not a required one.
- Keep the human in the loop; findings are advisory.
