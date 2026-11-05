---
id: "SCOPE-014"
title: "Measure the context value of LSP-derived semantic facts"
priority: "P2"
type: "research"
area: "SCOPE"
spec: "wiki/scopemux-lsp-boundary.md"
status: "todo"
created: "2026-11-05"
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

## Related Items

- `SCOPE-013` Ingest LSP-derived exact type facts into the ScopeMux map.
- `SCOPE-012` Consolidate ScopeMux and LSP responsibilities.
- `FUTURE-PROOFING-017` Measure ScopeMux against generic retrieval and set
  keep-expand-retire criteria.
- `SCOPE-008` Feed retrieval provenance and map outcomes into the data flywheel.
- `wiki/scopemux-lsp-boundary.md`
- `invariants/workspace-intelligence.md`
