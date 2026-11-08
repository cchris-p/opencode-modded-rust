# SCOPE-014: Context Value Of LSP-Derived Semantic Facts

**Date:** 2026-11-05
**Related:** `SCOPE-013`, `SCOPE-012`, `LSP-002`, `H-016`, `wiki/scopemux-lsp-boundary.md`.

## Question

Does ingesting optional, exact LSP facts at ScopeMux map anchors improve context quality
enough to justify the added LSP runtime cost? This is the keep/expand/retire gate for
`SCOPE-013`.

## Method

Harness: `crates/opencode-scopemux/examples/evaluate_exact_facts.rs` (native feature,
prebuilt scopemux artifact). It builds a fixed fixture workspace once and runs three arms
on the same held-out task set:

1. **heuristic** — `ScopemuxProvider` only.
2. **heuristic+exact** — the same provider with `LspExactFactProvider` backed by
   `rust-analyzer`; repeated once to exercise the fact cache.
3. **generic** — `GenericRepositoryProvider`.

Fixture (`scope014-fixture`): `src/lib.rs`, `src/shapes.rs` (trait + two impls),
`src/render.rs`, and a distractor `src/report.rs`.

Tasks (seed, objective, expected cross-file target):

| # | Seed | Objective | Expected file |
|---|---|---|---|
| 1 | `src/lib.rs` | `render_report` | `src/render.rs` |
| 2 | `src/lib.rs` | `area_total` | `src/shapes.rs` |
| 3 | `src/report.rs` | `render_label` | `src/render.rs` |

Environment: `rust-analyzer 1.98.1`, Linux x86_64, native scopemux artifact
`2a642c10534f`. Retrieval representation `Search`, token budget 20.

## Measured results

| arm | success | avg precision | avg recall | tokens | exact edges | fallbacks | p50 | p95 | fact calls | cache hits | cost/success |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| heuristic | 1/3 | 0.11 | 0.33 | 8 | 0 | 0 | 436.3 ms | 447.6 ms | 0 | 0 | 8.0 tokens |
| heuristic+exact | 1/3 | 0.13 | 0.33 | 8 | 1 | 0 | 459.7 ms | 761.8 ms | 4 | 0 | 8.0 tokens |
| heuristic+exact (cache pass) | 1/3 | 0.11 | 0.33 | 8 | 1 | 1 | 248.6 ms | 608.9 ms | 2 | 0 | 8.0 tokens |
| generic | 0/3 | 0.00 | 0.00 | 0 | 0 | 0 | 0.0 ms | 0.1 ms | 0 | 0 | 0.0 tokens |

Per-task detail (first exact pass):

| arm | task | candidates | tokens | precision | recall | exact edges | fact calls |
|---|---|---:|---:|---:|---:|---:|---:|
| heuristic | render_report | 2 | 2 | 0.00 | 0.00 | 0 | 0 |
| heuristic | area_total | 0 | 0 | 0.00 | 0.00 | 0 | 0 |
| heuristic | render_label | 12 | 6 | 0.33 | 1.00 | 0 | 0 |
| heuristic+exact | render_report | 2 | 2 | 0.00 | 0.00 | 0 | 1 |
| heuristic+exact | area_total | 0 | 0 | 0.00 | 0.00 | 0 | 0 |
| heuristic+exact | render_label | 13 | 6 | 0.38 | 1.00 | 1 | 3 |
| generic | render_report | 1 | 0 | 0.00 | 0.00 | 0 | 0 |
| generic | area_total | 1 | 0 | 0.00 | 0.00 | 0 | 0 |
| generic | render_label | 1 | 0 | 0.00 | 0.00 | 0 | 0 |

## Analysis

- **No task-success gain.** Both ScopeMux arms resolve the same single task; the exact arm
  does not make either failing task succeed. `area_total` yields no map candidates at all,
  so there is no anchor to enrich.
- **Marginal precision change.** The exact arm adds one exact edge on `render_label`
  (precision 0.33 -> 0.38 on that task, 0.11 -> 0.13 on average), but the relevant file was
  already found heuristically.
- **Latency cost.** p95 rises from ~448 ms to ~762 ms (+70%) on the exact pass. The cache
  pass does not demonstrate a stable win: anchor selection can pick a different
  symbol/position between runs, so the fact cache missed (0 hits) and one anchor reported
  unavailable.
- **No-server fallback is safe.** `SCOPE-013` tests already prove that an unavailable fact
  source leaves the candidate set identical to the no-LSP baseline. This run's generic and
  heuristic arms also complete normally.
- The neutral contract and adapter are sound; the measured benefit is not.

## Decision: RETIRE from default activation (keep opt-in/evaluation only)

`SCOPE-013`'s enrichment is **removed from default activation**. The runtime no longer
injects exact facts merely because LSP is configured. It is opt-in via
`OPENCODE_SCOPEMUX_EXACT_FACTS=1` for further evaluation. Default behavior is the heuristic
map, identical to the no-LSP path, so there is no added latency or variability.

The neutral `ExactFactProvider` contract, the `LspExactFactProvider` adapter, and the
enrichment code stay in place behind the opt-in gate because they are small, tested, and
useful for a larger evaluation. They must not be treated as settled capability.

### Retirement criterion

Fully remove the enrichment path (and keep only the neutral contract if another consumer
needs it) if a larger evaluation does not show either:

- a task-success gain, or
- a precision gain of at least 10% relative,

on a held-out set of at least 20 real tasks, while p95 retrieval latency increases by no
more than 20% and the fallback rate stays below 10%. If that bar is not met, delete the
enrichment rather than expanding it.

### What would justify expanding

- Query implementation/type-hierarchy edges, not only definition, for anchors where the
  heuristic resolver is known to miss (trait objects, macros, generics).
- Make anchor selection deterministic and cache-friendly (stable symbol/position per
  candidate), then re-measure cache hit rate.
- Use a larger, real task set with labelled relevant files, and separate LSP latency from
  map-construction latency.

## Limitations

- Three synthetic tasks from one fixture; not a statistically meaningful sample.
- One run per arm; `rust-analyzer` indexing and candidate ordering introduce variance.
- The map provider only parses the request's seeds, so cross-file targets depend on
  reference nodes the parser happens to emit.
- The exact arm currently queries definition only; that under-represents the
  implementation/type-hierarchy value the boundary allows.

## Reproduction

```bash
SCOPEMUX_PREBUILT_DIR=<native bundle> \
  cargo run -p opencode-scopemux --features native --example evaluate_exact_facts
```

The harness prints the markdown table above plus one JSON line per measurement.
