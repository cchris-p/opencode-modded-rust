---
id: "RESEARCH-005"
title: "Vet the internal research process (first-principles causal derivation)"
priority: "P1"
type: "research"
area: "RESEARCH"
spec: "invariants/research-department.md"
status: "todo"
created: "2026-09-27"
---

# Vet the internal research process (first-principles causal derivation)

## Summary

Establish whether a **first-principles research step** — derive a causal
hypothesis for why a mechanism should have an edge, then pre-register and
evaluate it quantitatively — measurably improves strategy research over the
current process of re-optimizing, re-varianting, and trialing adjacent premises.

This is the **internal** research-process question, distinct from `RESEARCH-003`
(external/remote evidence) and `RESEARCH-001` (harness comparison over a local
corpus). It is the upstream gate for the aa-studies canonical **research step**
(`aa-studies` `INFRA-051`): research-stage strategies stay **blocked pending this
vetting**, because the current process has repeatedly produced negatives that are
narrower than they read (see Evidence).

## Why this exists

- The current process advances redesign ideas on **narrative appeal** rather than
  a derived, testable causal claim. `aa-studies` `INFRA-051:27-49` names this
  failure directly: "a redesign idea can advance on narrative appeal rather than
  measured results."
- The **Balke** program is the worked example. Its rigorous negative covers the
  **parameter / construction / adjacent-premise** space only. `BALKE-STRAT-I2`
  changed range *construction*; `BALKE-STRAT-I3/I4/I5` were cheap adjacent-premise
  PoCs (fade/retest); `BALKE-021` lists the genuinely untested mechanism space as
  "documented, NOT pursued." No recorded derivation ever asked *why a timed
  session-range breakout should have an edge at all*. So "Balke has no edge" is
  **not** established; "this rule family, as parameterized/constructed, has no
  pre-cost edge on this corpus" is.
- The same shape recurs across the auxiliary (`GATE-AUX-*`) program: all
  documented `park`/`retired`, all derived from the same non-first-principles
  process.

## What "the research step" means (the thing being vetted)

The aa-studies `INFRA-051` loop stage, with an explicit derivation step:

1. **Derive.** State the causal hypothesis: what microstructure/auction behavior
   (order-flow imbalance at the boundary, volatility compression→expansion,
   time-of-day mechanics, news clustering) should produce the edge, grounded in
   the local canonical corpus (`aa-studies` `docs/tsm/`).
2. **Redesign.** State one pre-registered rule delta (single global rule; no
   per-symbol tuning) and its spec basis or an explicit spec-divergence decision.
3. **Research.** Evaluate that hypothesis through the sanctioned thin
   feasibility path (`strategy-feasibility-conversion`, `shared/min_backtest/`)
   against the locked criteria (`INFRA-MIN-BACKTEST-002`; corrected T2 per
   `INFRA-MIN-BACKTEST-005`).
4. **Gate.** Loop back on negative/park; exit to the canonical pipeline (`GAP`)
   only on a `promising` result. Never promote from inside the loop.

## Scope

Compare two arms on the same research-stage strategy questions (Balke and/or the
`GATE-AUX-*` family):

- **Arm A** — the first-principles research step above (derive → pre-register →
  evaluate).
- **Arm B** — the incumbent process (re-optimize / re-parameterize / trial
  adjacent premises without a recorded derivation).

Headline criterion: does Arm A produce **materially different or better-disciplined
decisions** — a real derivation that changes what is tested, fewer wasted
iterations, or a negative that is correctly scoped — than Arm B?

Secondary criteria: reproducibility/provenance of the derivation; whether the
derivation is falsifiable; cost/latency; whether it over-claims (a derivation is
not evidence).

## Relationship to the other RESEARCH cards

- `RESEARCH-001` — harness comparison over a **local corpus**; null result. Does
  **not** test the research *step*; it tested the tool.
- `RESEARCH-003` — **external/remote evidence** value. A different department
  (`invariants/research-department.md`).
- `RESEARCH-004` — the product utility surface for instantiating the department;
  independent of this card.
- `RESEARCH-002` — Feynman→ort chunk pipeline; `hold`.

## Gate / dependency

- **This card gates** `aa-studies` `INFRA-051` (canonical redesign/research loop
  stage) and the **unblock of research-stage strategies** (`Balke`, `GATE-AUX-*`),
  which are reclassified to `blocked` pending this vetting.
- It is **not** gated on `RESEARCH-003`; the internal derivation step does not
  depend on the external-evidence department.
- Operator owns the decision on `invariants/research-department.md` / any new
  invariant; draft the decision and surface it before it is binding.

## Non-goals

- Making `feynman`, a model, or a provider a dependency.
- Editing aa-studies, opening/closing `GATE-001`, or changing any area
  disposition (this card's findings inform; the block/unblock is operator-owned).
- Re-running the park dispositions' experiments.
- Committing private aa-studies research outputs, prompts, or secrets.

## Done when

- At least one real research-stage task is run through both arms with exact
  commands and observed outputs recorded.
- A clear verdict exists: a first-principles research step is **stronger, equal,
  or weaker** than the incumbent process for reaching scope-correct decisions.
- The scoping defect is demonstrated or refuted (does the incumbent process
  systematically under-scope its negatives?).
- The `invariants/research-department.md` decision is recorded and surfaced for
  approval.
- The unblock decision for `Balke` / `GATE-AUX-*` is made or explicitly deferred.

## Recommended verification

- Confirm both arms ran on the same questions with the same model before
  concluding.
- Confirm each derivation is falsifiable and each claim is evidence-backed, not
  narrative.
- Confirm no private aa-studies content and no disposition was changed by the run.

## Related

- aa-studies `INFRA-051` (canonical research step this card gates)
- aa-studies `BALKE-022` (retrospective), `BALKE-021` (held mechanism space)
- aa-studies `docs/strategy-registry.md` (Balke + `GATE-AUX-*` dispositions)
- `RESEARCH-001`, `RESEARCH-002`, `RESEARCH-003`, `RESEARCH-004`
- `invariants/research-department.md`

## Notes

- Model is not the variable: one model can run both arms.
- A derivation is a prior, not evidence; the pre-registered evaluation is the
  evidence.
- Keep the human in the loop; findings are advisory.
