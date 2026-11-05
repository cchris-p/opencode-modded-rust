---
id: "SCOPE-014"
title: "Measure the context value of LSP-derived semantic facts"
priority: "P2"
type: "research"
area: "SCOPE"
spec: "wiki/scopemux-lsp-boundary.md"
status: "done"
created: "2026-11-05"
updated: "2026-11-05"
---

# Measure the context value of LSP-derived semantic facts

## Summary

Evaluate whether LSP-derived exact semantic facts improve ScopeMux context quality and
efficiency enough to keep, expand, or retire the enrichment. This is the evidence gate for
`SCOPE-013`.

## Why this exists

Adding type facts increases runtime complexity and can add LSP latency. The WIP invariants
require evaluation against generic/heuristic retrieval and allow a null or negative result.
The product should not keep an enrichment just because it is technically possible.

## Scope

- Compare at least three arms on real, held-out tasks:
  1. heuristic ScopeMux only,
  2. ScopeMux with LSP-derived exact facts,
  3. generic retrieval baseline.
- Record tokens per task and per subagent, context precision/recall, task success, review
  quality, p50/p95 latency, LSP fallback rate, cache hit rate, and cost per successful task.
- Record whether type facts change candidate inclusion or only confidence.
- Include unsupported-language and no-server runs as explicit fallback evidence.
- Record a keep/expand/retire decision with a retirement criterion.

## Acceptance criteria

- The three arms are measured on the same held-out task set.
- Results include both outcome and cost, not only token counts.
- A no-server run proves the task still completes with heuristic evidence.
- The decision records whether the benefit is worth the added LSP runtime cost.
- The evaluation is recorded on the card and linked from the handoff.
- A null or negative result is documented and accepted as valid.

## Non-goals

- Implementing the ingestion path (that is `SCOPE-013`).
- Evaluating unrelated LSP features such as diagnostics.
- Tuning model prompts to manufacture a win.
- Changing the WIP keep/expand/retire criteria without human review.

## Dev Notes

- Reuse `FUTURE-PROOFING-017` evaluation mechanics where possible.
- Keep task sets fixed before comparing arms.
- Separate latency added by LSP from latency added by map construction.
- Report exact vs heuristic edge counts so the effect is attributable.

## QA Verification

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/154 (merged into `development`).
- Evaluation recorded in `docs/research/SCOPE-014-exact-facts-evaluation.md`.
- Harness: `crates/opencode-scopemux/examples/evaluate_exact_facts.rs` (native, real
  `rust-analyzer` for the exact arm).
- Three arms on the same fixed held-out set:
  - heuristic ScopeMux: 1/3 success, precision 0.11, p95 448 ms
  - ScopeMux + exact LSP facts: 1/3 success, precision 0.13, 1 exact edge, p95 762 ms
  - generic: 0/3 success
- **Decision: RETIRE from default activation.** No task-success gain, marginal precision,
  and a large latency increase. Enrichment is opt-in via
  `OPENCODE_SCOPEMUX_EXACT_FACTS=1`; default behavior is the heuristic map, identical to
  the no-LSP path.
- Retirement criterion: fully remove enrichment unless a >=20-task held-out set shows a
  task-success or >=10% relative precision gain with <=+20% p95 latency and <10% fallback
  rate.
- No-server completion is covered by the SCOPE-013 equality test and the heuristic/generic
  arms in this run.
- Real-server smoke: `cargo test -p opencode-lsp real_rust_analyzer_definition_smoke --
  --ignored` passes against `rust-analyzer 1.98.1`.

## Related Items

- `SCOPE-013` Ingest LSP-derived exact type facts into the ScopeMux map.
- `SCOPE-012` Consolidate ScopeMux and LSP responsibilities.
- `FUTURE-PROOFING-017` Measure ScopeMux against generic retrieval and set
  keep-expand-retire criteria.
- `SCOPE-008` Feed retrieval provenance and map outcomes into the data flywheel.
- `wiki/scopemux-lsp-boundary.md`
- `invariants/workspace-intelligence.md`

## QA Re-confirmation (2026-11-05, H-018 step 4)

The code changed after the original report (PR #158 FFI fix), so the harness was re-run on
merged `development`:

```bash
$ SCOPEMUX_PREBUILT_DIR=~/.cache/opencode-modded-rust/native/linux-x86_64-2a642c10534f   cargo run -p opencode-scopemux --example evaluate_exact_facts --features native -- --help
```

(The example has no `--help` handling; it executes the evaluation. Fresh run:)

- heuristic: 1/3 success, avg precision 0.11, p95 488.1 ms, exact edges 0
- heuristic+exact: 1/3 success, avg precision 0.11, p95 840.7 ms, exact edges 0,
  4 fact calls
- heuristic+exact(cache): 1/3 success, p95 1430.1 ms, 1 fallback
- generic: 0/3 success

```bash
$ rg -n "OPENCODE_SCOPEMUX_EXACT_FACTS" crates/opencode-session/src/prompt.rs
306:                std::env::var("OPENCODE_SCOPEMUX_EXACT_FACTS").as_deref(),
```

The fresh run still shows no task-success gain, no precision gain, and a latency increase
(p95 488 -> 841 ms), so the recorded **RETIRE from default activation** decision stands.
`docs/research/SCOPE-014-exact-facts-evaluation.md` remains accurate in its conclusion;
enrichment stays opt-in via `OPENCODE_SCOPEMUX_EXACT_FACTS=1`. Promoting `qa` -> `done`
(the decision is the deliverable).