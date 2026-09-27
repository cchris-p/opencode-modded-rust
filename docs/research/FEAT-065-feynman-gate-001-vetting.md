# FEAT-065 — Feynman vs opencode/ort for vetting aa-studies `GATE-001`

Companion note to the `FEAT-065` card. This is a **sanitized** record of a
read-only research comparison; it contains no private aa-studies artifact
contents, prompts, or secrets. External documents are referenced by path only.

- Item: `FEAT-065` ("Use Feynman to vet GATE-001 results and improve the ideal
  strategy and param sets")
- Handoff: `H-013`
  (`handoffs/archive/2026-09-26-feat-065-feynman-gate-001-vetting-handoff.md`)
- Date: 2026-09-26
- Target: aa-studies `development` @ `e7abe68` (private repo, read-only)

## Question

Can Feynman and its research method vet aa-studies `GATE-001` results and surface
improvements to the current ideal strategy set and finalist param sets **more
strongly than opencode/`ort` alone**? Null result is a valid answer.

## Method

Both arms ran the **same** read-only task prompt against the same aa-studies
checkout and the **same model** (`deepseek/deepseek-flash`), so the comparison is
harness-vs-harness, not model-vs-model.

- **Arm A — Feynman 0.5.8** (standalone bundle, own Node runtime), invoked as
  `feynman "<task>"` from the aa-studies tree. Explicit invocation recorded.
- **Arm B — opencode/`ort`** (Rust product binary), invoked as
  `opencode run --dir <aa-studies> -m deepseek/deepseek-flash "<task>"`.

Task sections: (1) condition-by-condition verdicts A, B.1–B.7, C with
file-level evidence and cross-document inconsistency flags; (2) enumerate the
current ideal set and finalist param sets; (3) ranked improvement candidates;
(4) local limits.

### Environment note (a finding in itself)

Feynman's default/recommended model (`anthropic/claude-opus-5-5`) was
unavailable because the Anthropic account had no credits. Feynman had to be
reconfigured to `deepseek/deepseek-flash` to run at all and to match `ort`. The
default model path therefore carries provider-credit and separate-auth-surface
risk that `ort` does not add.

## Arm A — Feynman result

Produced a structured report **and** persisted it to a standalone artifact path
outside the repo, then asserted (and was independently confirmed) that
aa-studies was left clean.

Condition verdicts: A **GAP**; B headline **CONTRADICTION**; B.1 **VERIFIED**;
B.2 **VERIFIED**; B.3 **VERIFIED**; B.4 **VERIFIED**; B.5 **PARTIAL**;
B.6 **VERIFIED**; B.7 **PARTIAL**; C **VERIFIED**.

Material findings unique to Arm A (independently re-verified by this agent):

- The MHAL `v1.02` B.7 universe artifact covers only **two** of the four
  required established timeframes while declaring four, and the summary text
  repeats the contradiction. This is a direct B.7 completeness defect.
- A broken evidence citation path in the gate card's MTHR block.
- A metadata/summary generator inconsistency in that same MHAL artifact.

Feynman's literature/web tools (alphaXiv, Semantic Scholar, web/PDF) were **not
used and not needed** for this objective; alphaXiv was not configured. No
multi-agent decomposition or separate verification pass was observable in the
run output.

## Arm B — opencode/`ort` result

Produced a structured report inline (session record; not persisted to a file by
default).

Condition verdicts: A **GAP**; B.1 **PARTIAL/CONTRADICTION**; B.2
**PARTIAL/CONTRADICTION**; B.3 **VERIFIED**; B.4 **PARTIAL**; B.5 **VERIFIED**;
B.6 **VERIFIED**; B.7 **PARTIAL/CONTRADICTION**; C **VERIFIED**.

Material findings unique to Arm B:

- The condition header's "across MQL4 + MQL5 + AA" clause is structurally
  unverifiable for V1 because MQL5/MT5 work is declared out of scope and tracked
  as a separate item; Arm A did not flag this.
- A sharper, more prominent framing of the TPIV selection-bias weakness
  (survivor count meets the bar, statistical significance does not).

Both arms independently found the core cross-document drift: gate card vs
registry version lines, `BALKE-008/009` "retained" vs closed, `VERS-005`
done-vs-hold, a stale version line in `docs/strategy-readiness.md`, stale
handoff-07 track rows, and the now-false `SKIP_SYMBOLS` paragraph.

## Comparison on the card's criteria

| Criterion | Arm A (Feynman) | Arm B (opencode/ort) |
|---|---|---|
| Depth / structure | Strong; plan-first sections; one material B.7 defect | Strong; equally structured; different material condition defect |
| Citation / source verification | Good in-repo citations; research/literature tools unused | Good in-repo citations; no extra source tooling needed either |
| Reproducibility / auditability | Explicit `feynman "..."` invocation + persisted artifact | Session record only by default (output redirectable) |
| Workable chunks | Report lists actionable next actions; not board-shaped | Same |
| Independent vetting strength | Caught the MHAL B.7 coverage defect; missed the MQL5 clause | Caught the MQL5 clause; missed the MHAL B.7 coverage defect |
| Improvement finding | 10 candidates; broad; one candidate unique | 8 candidates; heavy overlap; better-ranked significance candidate |
| Cost / latency | ~4.6 min wall clock; separate harness, auth, moving-target fork | ~1.6 min wall clock; product's own runtime, no extra harness |
| Complexity / fork risk | Higher: standalone install, own Node, provider auth, periodic fork | Lower: nothing beyond the product binary |

Each arm caught one material condition-level issue that the other missed, and
their improvement lists overlapped heavily.

## Verdict

- **Vetting `GATE-001`: EQUAL.** Both arms produced materially equivalent,
  high-quality condition vetting, each with one distinct genuine catch. No
  repeatable Feynman advantage was demonstrated.
- **Finding improvements: EQUAL.** Overlap is high; the marginal difference is
  breadth (Arm A) vs ranking sharpness (Arm B). Not a repeatable advantage.

**Null result confirmed.** Feynman's method is not stronger than opencode/`ort`
alone for this objective, at roughly 3× the latency and materially higher
operational complexity.

## Value-carrying components (and noise)

- **Carries value:** ordinary local-file reasoning + **artifact discipline**
  (persisting a review to a standalone path). The artifact-discipline piece is
  trivially reproducible in `ort` by redirecting `opencode run` output.
- **Marginal value:** explicit-invocation provenance is a modest auditability
  nicety; the `ort` session record already exists.
- **Noise / no demonstrated value here:** literature/alphaXiv/web research tools
  (not used for a local-artifact objective, consistent with the existing
  aa-studies doc), and any multi-agent decomposition/verifier pass (not
  observable in this run).

## Relationship to `docs/feynman_role_for_optimization.md`

**Supplement, not supersede.** That doc scopes Feynman for the walkforward
optimization loop (coding/verification copilot) and already rules out
alphaXiv/literature for that objective. `FEAT-065` covers a **different**
objective (GATE-001 result vetting) and reaches a parallel conclusion: the
research/literature tooling adds nothing, and the separate harness is not
justified by the vetting output. It does not replace the optimization-copilot
guidance, so the correct relationship is supplement.

An old session artifact exists in the aa-studies tree
(`session-ses_fd61.md`) but contains **no** Feynman references; the durable
aa-studies artifact for this topic remains
`docs/feynman_role_for_optimization.md` itself.

## Advisory improvement candidates (aa-studies-owned; not actioned here)

Recorded for the operator; aa-studies was not edited and no gate disposition was
changed.

1. Complete the MHAL `v1.02` B.7 timeframe coverage (all established TFs), and
   repair the artifact metadata/summary generator so declared coverage matches
   actual rows.
2. Reconcile the gate card / `docs/v1-readiness-gate.md` /
   `docs/strategy-readiness.md` / registry status and version drift (including
   the `BALKE-008/009`, `VERS-005`, and `SKIP_SYMBOLS` text).
3. Resolve the TPIV statistical-robustness gap (bias-adjusted / reality-check
   statistic) before treating the survivor set as robust.
4. Re-examine the TPIV EURCHF near-miss exclusion (strongest AA survivor,
   excluded on measured-MT economics) for a measurement-vs-structural verdict.
5. Make an explicit operator/standards decision on the MTHR low-frequency gate:
   the park is frequency-driven, and a pooled-gate reading flips many cells.
6. Harden the MHAL scope around its marginal/CHF-cohort concentration before
   the pending paper/forward test.
7. Re-probe Balke range-reversion cost-robustness (small positive gross edge,
   not cost-robust) via a pre-registered thin test; re-park if it cannot clear
   the cost wall.
8. Strengthen `shared/robust_selection.py` selection-bias control and re-report
   existing universes audit-only.

## Guardrails honored

- aa-studies always read on `development`, never `main`; refetched before use.
- No optimization/backtest scripts run (data mount absent).
- No aa-studies edits; `git status` confirmed clean after both arms.
- Findings advisory only; no gate disposition changed.
- No private aa-studies content committed here (references by path only).
