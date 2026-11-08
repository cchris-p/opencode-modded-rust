# RESEARCH-005 — A/B run results (derivation-first vs incumbent)

**Date:** 2026-09-28
**Card:** `RESEARCH-005` (`boards/todo/vet-internal-research-process-first-principles.md`)
**Protocol:** Part 2 of `RESEARCH-005-internal-research-process-vetting.md`
**Status:** A/B run executed; verdict recorded; **invariant decision drafted for
operator approval** (advisory — not binding). No aa-studies disposition changed.

**Non-goals honored:** no aa-studies disposition changed; no gate opened/closed;
no `.set`; no MT spend; no private aa-studies artifact committed (the testbed ran
in `/tmp`); no experiment from the park dispositions re-run.

---

## 1. Testbed and held constants

Single testbed mechanism: **`BALKE-021` #1 — volatility-expansion momentum
ignition** (recorded default recommendation).

| Constant | Value |
|---|---|
| Model | one model ran both arms (model is **not** the variable) |
| Mechanism | same `BALKE-021` #1 opportunity for both arms |
| Corpus / data | Dukascopy `GMT+0_US-DST`, M5, on-bar, next-open fill |
| Universe | audited 27-symbol universe, `universe_hash 7f5d101e0ba33505`, snapshot `41f21102b227ecb7` |
| Window | 2006-12-12 → 2026-01-01 (60/20/20 calendar; dev 2018-05-19, val 2022-03-11, oos 2026-01-01) |
| Harness | `shared/min_backtest/` (harness `1.1.0`), `INFRA-MIN-BACKTEST-002` gate; corrected T2 (`-005`) |
| Cost / fill | `CostModel()` (2.6 pips round trip) + 1.5x/2x stress; on-bar, next-open |
| Base version | `v1.00` |
| Multiplicity | both arms charged to `PIPE-025`; registered run consumed once |

## 2. Arms

### Arm B — incumbent (no recorded derivation)
Canonical Balke per-symbol range-breakout, conditioned on a **volatility-expansion
admission gate**: a breakout is admitted only when the breakout bar's true range
≥ `vexp_k` × prior ATR (`vexp_k = 0` disables; grid `{0,1,1.5,2}`). A
re-variant/parameterize trial of the same mechanism, with no written causal chain.

### Arm A — derivation-first
Five-artifact derivation (per `RESEARCH-005` note Part 1.2), pre-registered before
the run:

1. **Mechanism** — order-flow imbalance at a compression→expansion transition.
2. **Causal chain** — low-volatility range accumulates one-sided resting flow;
   an expansion bar triggers stop/continuation flow → directional path.
3. **Testable prediction** — after a compressed ATR regime, an expansion bar's
   close location within its range predicts continuation beyond the
   unconditional rate over the next bars.
4. **Single global rule** (no per-symbol tuning):

   ```
   compression = ATR[i-1] < trailing_median(ATR, 2016)[i-1]
   expansion   = ATR[i]   > expansion_mult * trailing_median(ATR, 2016)[i]
   direction   = sign(Close[i] - (High[i]+Low[i])/2)
   ```

   enter next open, one trade/day, fixed 60-pip stop, flat at next day's first bar.
   Frozen grid `{1.5, 2.0}`.
5. **Falsification condition** — OOS breadth < 1/3, or gross edge not surviving
   1.5x cost, or the negative being concentrated in a single symbol/period.

TSM grounding (Part 1.3 map): Ch.20 *Advanced Techniques* (price-volatility
relationship, volatility trading, trends-vs-noise); Ch.16 *Day Trading*
(volatility breakout, slippage).

## 3. Exact commands and observed outputs

Run from `~/apps/aa-studies` with the `aa-studies` pyenv interpreter; testbed at
`/tmp/opencode/rsch005_ab/`:

```bash
python /tmp/opencode/rsch005_ab/run_ab.py --smoke   # 2 symbols, sanity
python /tmp/opencode/rsch005_ab/run_ab.py           # 27-symbol registered run
```

Artifacts: `/tmp/opencode/rsch005_ab/verdict_full.json`,
`/tmp/opencode/rsch005_ab/experiment_registry.jsonl`,
`/tmp/opencode/rsch005_ab/arm_a/`, `/tmp/opencode/rsch005_ab/arm_b/`.

Harness hashes (sha256): `ab_strategies.py 8e24cf32…`, `run_ab.py b485d8ef…`.
Rule-set hashes: Arm A `a1052dd4f0e02e8c81c1710b8bd1825a53715604ed75e017b82ec66620804268`,
Arm B `9140cc62141468820a1ad945c3289d827ae05df54e58003aa77262a1670d20e2`.

### Observed gate results

| Metric | Arm A (derivation-first) | Arm B (incumbent) |
|---|---|---|
| Selected | `expansion_mult=2.0` | `vexp_k=2.0` |
| Trades | 498 | 87,535 |
| OOS net | **+0.00079** | −0.421 |
| OOS breadth base / 1.5x | **13/27 / 13/27** | 0/27 / 0/27 |
| Gross/cost (E1) | 1.29 | −1.32 |
| `net/robust_max_dd` (D1) | 0.009 | −0.685 |
| Parameter profitability | 0% (per-market 50%) | 0% |
| Time thirds net | (−0.0009, −0.0038, +0.0024) | (−0.401, −0.416, −0.623) |
| Disposition | **park** | **park** |

Arm A passes P1/P2/B1 (positive OOS net; 13/27 breadth ≥ 9) but fails P3, P4,
P5 (1.5x cost, marginal), P7 (best-symbol **and** best-trade removal),
T1/T2, E1 (gross/cost 1.29 < 2), D1 (0.009 < 1) and B2 (many passing symbols
with < 20 trades). Arm B fails P1/P2/P3/P4/P5/T1/T2/E1/D1/B1 — a robustly
negative gross edge.

## 4. Comparison (D1/D2/D3)

- **D1 (different rule tested) — yes.** Arm A tested a global compression→
  expansion close-location continuation independent of the range boundary
  (498 trades); Arm B tested the range-breakout conditioned on expansion
  (87,535 trades). Materially different rule and trade population.
- **D2 (correctly scoped negative) — yes, materially.** Arm B's negative reads
  "the breakout family has no pre-cost edge". Arm A's negative is scoped:
  *the momentum-ignition mechanism has a weak positive pre-cost gross signal
  (E1 1.29) that is not robust — it vanishes when the best symbol or best trade
  is removed, does not clear the 2x cost bar, and is concentrated in one third.*
  That is a different, better-scoped claim than "no edge".
- **D3 (fewer wasted iterations) — yes (process-level).** Arm A pre-registered
  2 cells; Arm B selected an isolated maximum over 4 cells (P4). Arm A reached a
  scoped finding with ~175x fewer trades.

## 5. Verdict

**Arm A (derivation-first) is stronger than Arm B (incumbent) for reaching a
scope-correct decision**, while both still `park`. The derivation did **not**
rescue a tradeable edge; it changed the *decision content and scope* — from a
blunt "no pre-cost edge" to a precise "weak, fragile, non-robust pre-cost
signal that fails the locked robustness/cost gate". This supports the thesis
behind `RESEARCH-005`: the incumbent process systematically under-scopes its
negatives.

Caveats: single testbed; thin triage-only harness (`INFRA-MIN-BACKTEST-014`
fidelity caveats); Arm A's positive aggregate is knife-edge (D1 ≈ 0, P7 both),
so "stronger process" must not be read as "edge exists".

## 6. Invariant decision (draft — for operator approval)

**Decision: KEEP `invariants/research-department.md` as amended (2026-09-28).**
The sequenced two-sub-step loop (internal `project-principles-redesign` →
external `research-department` consultation) is the correct model; the A/B run
does not warrant a change. No new invariant is proposed.

## 7. Unblock recommendation (advisory — operator owns the decision)

- **Process:** adopt the derivation-first `RSCH` step (already canonized in
  aa-studies `INFRA-051`) — the run supports it.
- **Balke:** re-confirm `park`, now with the improved scoping (weak fragile
  pre-cost signal, no robust edge). Not a basis to promote.
- **`GATE-AUX-*` / MTHR:** not evaluated by this card; `RESEARCH-003` (external
  evidence value) is the remaining separate gate.

No aa-studies disposition was changed by this run.
