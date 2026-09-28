# RESEARCH-005 — Derivation-first research step: protocol + A/B vetting design

**Status:** protocol defined; the A/B run is **deferred** until the research
prerequisites below are complete. This note is the reference spec for the
aa-studies `RSCH` derivation step (`aa-studies INFRA-051`) that this card vets,
plus the concrete A/B protocol for the vetting run itself.

**Non-goals honored:** no aa-studies disposition is changed, no gate is
opened/closed, no `.set`, no MT spend, no private aa-studies content is
committed, and no experiment is re-run. Findings are advisory; the operator owns
the unblock decision.

---

## Part 1 — The derivation-first research step (fleshed out)

This is the concrete form of the thing `RESEARCH-005` vets. It is written to be
generic across research-stage strategies, with Balke as the worked example.

### 1.1 Why a derivation step exists

The incumbent process advances redesign ideas on **narrative appeal** and then
trials **adjacent premises** (re-parameterize / re-variant) without ever recording
*why a mechanism should have an edge*. Its negatives are therefore
**scope-limited, not premise-falsifying** (aa-studies `INFRA-051:27-49`,
`BALKE-022` §Q2). A derivation step forces the causal claim to the surface before
any run is charged, so the negative that results is scoped to the claim actually
tested.

### 1.2 The causal-hypothesis schema

A derivation must produce, in order, five artifacts. A derivation that cannot
fill all five is **not ready** to trial.

1. **Mechanism** — the named microstructure / auction behavior being claimed
   (e.g. "order-flow imbalance at the range boundary", "volatility
   compression→expansion", "time-of-day auction mechanics", "news clustering").
2. **Causal chain** — the *why*, step by step: mechanism → order-flow imbalance →
   directional price path. This is the falsifiable part; it must not merely assert
   "prices go up after X".
3. **Testable prediction** — a directional, falsifiable statement ("bars that
   satisfy condition C are followed by moves of direction D at rate > the
   unconditional rate over horizon H"), not "the strategy is good".
4. **Single global rule** — one entry/exit rule derived from the prediction, with
   no per-symbol tuning and a pre-registered parameter grid.
5. **Falsification condition** — the pre-stated result that would reject the
   hypothesis, stated *before* the run (e.g. "OOS breadth < 3 of 27 symbols", or
   "gross edge does not survive 1.5x cost").

### 1.3 First-principles grounding (TSM)

The causal chain must cite a first-principles source in `aa-studies/docs/tsm/`
(chapter → method note → formula basis), not rest on assertion. For the Balke
timed-breakout family the relevant map is:

| Mechanism (BALKE-021) | TSM grounding |
|---|---|
| Volatility-expansion momentum ignition | Ch.20 *Advanced Techniques* (price-volatility relationship, volatility trading, trends-vs-noise); Ch.16 *Day Trading* (volatility breakout, slippage) |
| Session-close extreme reversion | Ch.22 *Adding Reality* (trend vs mean-reversion trade-offs, theory of runs); Ch.15 *Short-Term Patterns* (time-of-day) |
| Magnitude/extension filter | Ch.16 (breakout + transaction-cost reality); Ch.8 *Trend Systems* (fat tail) |
| Opening-range / session-structure variants | Ch.16 (opening-range breakout); Ch.15 (time-of-day patterns) |

A derivation that cannot name a chapter/method/formula is incomplete.

### 1.4 Spec basis vs spec-divergence

Each redesign must record one of two decisions:

- **Spec basis** — the rule is already a described behavior of the strategy spec
  (`~/apps/MQL4-2023/docs/balke/`), and the trial re-scopes an input.
- **Spec-divergence** — the rule changes core behavior; the divergence is stated
  explicitly, and the spec + MT mirror + version bump land only at promotion
  (never inside the loop).

### 1.5 Pre-registration template

The pre-registration records, before any full-universe run: the mechanism,
causal chain, testable prediction, single global rule, the frozen parameter grid,
the universe/timeframe/IS-OOS split, the cost model, the locked criteria
(`INFRA-MIN-BACKTEST-002`), the multiplicity charge (`PIPE-025`), and the
falsification condition. Engineering smoke runs (2 symbols, degenerate 1x1 grid)
precede and are correctness checks, not the registered test.

### 1.6 Guards

- **A derivation is a prior, not evidence.** The pre-registered evaluation is the
  evidence; a derivation never becomes canonical truth until a run passes the
  locked gate.
- **No narrative promotion.** A `promising` derivation exits the loop to `GAP`
  only; nothing promotes from inside the loop.
- **Over-claim guard.** A negative scoped by a real derivation is still a
  negative, and is still provisional until the process that produced it is vetted
  (this card).

---

## Part 2 — A/B vetting protocol (RESEARCH-005)

### 2.1 The comparison question

Headline: **does the derivation step produce a materially different or
better-disciplined decision than the incumbent process?** Operationalized as any
of:

- **(D1) different rule tested** — the derived rule is not what the incumbent
  would have trialed (the derivation changed *what* was tested).
- **(D2) correctly scoped negative** — the negative names the claim actually
  falsified, instead of over-reading as "no edge".
- **(D3) fewer wasted iterations** — the derivation avoids a trial the corpus
  evidence already predicts is negative (e.g. another continuation variant after
  E7's continuation negative).

A verdict of "equal" is valid and is the expected null (mirroring `RESEARCH-001`).

### 2.2 Arms

- **Arm B (incumbent):** take one untested `BALKE-021` mechanism and trial it as an
  adjacent-premise rule directly — no recorded derivation, re-variant/parameterize,
  run the thin gate. This reproduces the E1–E7 process.
- **Arm A (derivation-first):** run Part 1 end to end on the **same mechanism** —
  derive, ground in TSM, pre-register, evaluate the *derived* rule through the same
  thin gate.

### 2.3 Held constants

| Constant | Value |
|---|---|
| Model | one model runs both arms (model is **not** the variable) |
| Mechanism | same `BALKE-021` opportunity for both arms |
| Corpus / data | canonical Dukascopy `GMT+0_US-DST`, audited 27-symbol universe |
| Thin harness | `shared/min_backtest/`, `INFRA-MIN-BACKTEST-002` criteria, corrected T2 |
| Cost / fill | `CostModel()` + 1.5x/2x stress; on-bar, next-open fill |
| Multiplicity | both arms charged to `PIPE-025`; the registered run is consumed once |

### 2.4 Runbook (deferred — execute only after prerequisites)

1. Confirm prerequisites (Part 3) are complete.
2. Select the single testbed mechanism (default recommendation:
   volatility-expansion momentum ignition, `BALKE-021` #1 — sharpest D1 contrast,
   FX-corpus-compatible).
3. Run **Arm B** to `/tmp` (never commit to aa-studies): record exact command,
   disposition, and the rule it tested.
4. Run **Arm A**: produce the Part 1.2 five-artifact derivation + 1.5
   pre-registration, then evaluate the derived rule; record exact command and
   disposition.
5. Compare against D1–D3; record the verdict (stronger / equal / weaker) and the
   value-carrying components.
6. Draft the `invariants/research-department.md` decision (keep / amend / narrow)
   and surface it for operator approval.
7. Record the unblock recommendation for Balke / `GATE-AUX-*` (unblock / re-confirm
   park / keep blocked) as advisory only.

---

## Part 3 — Research prerequisites (blocking any Balke mechanism trial)

No Balke mechanism is trialed until all of these hold:

- [x] `RSCH` loop stage canonized in the aa-studies model (`INFRA-051` Pass 1).
- [x] Propagation + enforcement passes complete (`INFRA-051` Pass 2–3).
- [x] The derivation step is fleshed out (Part 1 of this note).
- [x] The A/B vetting protocol is defined (Part 2 of this note).
- [ ] `RESEARCH-005` A/B run executed and a verdict recorded (Part 2.4).
- [ ] `invariants/research-department.md` decision approved by the operator.
- [ ] Operator unblocks the specific strategy (aa-studies `INFRA-052` /
  `INFRA-051` Pass 4 seeds `<AREA>-RSCH-I<n>` only after unblock).

The last three are the actual gate; the first four are what this note supplies.

---

## Related

- `RESEARCH-001` (local-corpus null result; precedent), `RESEARCH-003` (external
  evidence, separate department), `RESEARCH-004` (utility surface), `RESEARCH-002`
  (chunk pipeline, hold)
- `invariants/research-department.md` (internal research step is a separate
  concern — line 13)
- aa-studies `INFRA-051`, `INFRA-052`, `BALKE-021`, `BALKE-022`,
  `docs/results-versioning-standard.md` (RSCH loop), `docs/tsm/`,
  `shared/min_backtest/`
