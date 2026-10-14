---
id: "RESEARCH-008"
title: "Evaluate integrating DeepSeek Harness (dsh) into ort as a research/vetting harness"
priority: "P2"
type: "research"
area: "RESEARCH"
spec: "invariants/research-department.md"
status: "todo"
created: "2026-10-14"
---

# Evaluate integrating DeepSeek Harness (dsh) into ort as a research/vetting harness

## Summary

Evaluate whether **DeepSeek Harness** (`dsh`, `deepseek-ai/deepseek-harness`)
can serve the **Research Department** in `ort` — as a research/vetting harness
and/or as a source of product capability through its plugin architecture — and
whether integrating it is worth doing.

This is a fresh harness candidate evaluated under
`invariants/research-department.md`, in the same evidence-gated spirit as the
Feynman evaluation (`RESEARCH-001`). It does **not** assume dsh is better than
Feynman, `ort` alone, or product-native tooling; a null/negative result is a
valid and useful outcome.

## Why this exists

- The Research Department's tooling is an **open, evidence-gated decision**, not a
  settled dependency (`invariants/research-department.md`). Feynman was the
  reference harness; a second serious candidate deserves the same measurement.
- `dsh` is a substantially different design from Feynman and from `ort`: an
  **everything-is-a-plugin** architecture built on
  [Cordis](https://github.com/cordiverse/cordis), which may change what is
  reachable for research/vetting or what could be borrowed product-side.
- `RESEARCH-004` commits `ort` to providing the Research-Department **utility
  surface**; which harness(es) that surface instantiates is not yet fixed.
- `RESEARCH-003` (external-evidence value) is still open; dsh may carry a
  remote/external evidence surface that is relevant to it.

## What we know about dsh (recorded 2026-10-14; re-verify before reuse)

- Public repo `git@github.com:deepseek-ai/deepseek-harness.git`, default branch
  `master` (HEAD `d743267388641bc76f17c45ce8b4c231aed1d32c`), MIT license.
- Self-description: "DeepSeek Harness: Everything is a Plugin." TypeScript.
- Run via `npx @deepseek-ai/dsh web` (Web UI at `http://127.0.0.1:3080` by
  default) or from a source checkout with `pnpm install && pnpm run build &&
  pnpm dsh web`.
- Built on Cordis; documented at `https://deepseek-harness.github.io/deepseek-harness/`.
- Marked **developer preview**, iterating rapidly, with explicitly warned
  compatibility-breaking changes — a moving-target risk like Feynman's fork.
- `README`/homepage: `https://github.com/deepseek-ai/deepseek-harness`,
  `https://deepseek.com/harness`.

## Settled model (context, not in question)

- A **department** is defined by concern, not by model/provider/tool; one model
  can operate more than one department (`invariants/research-department.md`).
- External/remote evidence is **advisory, confidence-labeled,
  provenance-bearing, and never canonical**; local artifacts stay authoritative.
- Which tooling serves the department remains an evidence-gated decision; this
  card does not make `dsh` a dependency and does not grant it authority.

## Scope

Answer, with recorded commands and observed outputs, on at least one real
research/vetting task relevant to the live workflow:

- **Harness fit** — can `dsh` perform a comparable research/vetting pass
  (plan/decompose, source gathering, verification, synthesis) with auditable
  provenance?
- **Integration shape** — how would `dsh` be integrated into `ort`? Candidate
  shapes to weigh: run as a separate harness invoked from `ort` (like the
  Feynman reference model), expose its surface through the `RESEARCH-004`
  utility surface, or borrow capability from its plugin/Cordis architecture
  rather than running it. State the recommended shape.
- **Plugin architecture as capability source** — is there anything in the
  everything-is-a-plugin/Cordis design worth borrowing product-side (extension
  points, composition model), independent of running dsh itself?
- **Comparative verdict** — versus `ort` alone (and, where useful, versus the
  recorded Feynman baseline): stronger, equal, or weaker for research/vetting.
- **External-evidence relevance** — does dsh carry a remote/literature/web
  surface that informs the open `RESEARCH-003` question?
- **Cost, latency, complexity, and moving-target risk** — including the
  developer-preview compatibility churn.

Headline criterion: does `dsh` **change, sharpen, or beat** what `ort` (and the
Feynman baseline) already produce for the same task?

## Non-goals

- Making `dsh` (or any harness/model/provider) a runtime dependency of `ort`.
- Vendoring or folding the dsh application into the product binary.
- Granting dsh output (or external evidence) canonical authority, or changing any
  area/gate disposition.
- Building the integration itself — this card evaluates and scopes; the build, if
  any, is a follow-up card.
- Committing private aa-studies content, prompts, or secrets.

## Done when

- At least one real research/vetting task is run through dsh and through `ort`
  alone (Feynman baseline reused where applicable), with exact commands and
  observed outputs recorded; the run is repeatable.
- A clear verdict exists: dsh is **stronger, equal, or weaker** for
  research/vetting, and the value-carrying components (and the noise) are named.
- The **recommended integration shape** is stated (separate harness, utility
  surface, or borrowed capability), with rationale.
- The relationship to `RESEARCH-001` (Feynman), `RESEARCH-003` (external
  evidence), and `RESEARCH-004` (utility surface) is recorded.
- Any follow-up (build/borrow decision, invariant decision, or `RESEARCH-003`
  input) is split into its own card or explicitly deferred.

## Recommended verification

- Read dsh from its own checkout/docs, not from memory; record the exact
  revision and commands used.
- Run both arms end to end on the same task before drawing a conclusion; capture
  raw artifacts, not just summaries.
- Confirm no disposition, invariant, or authority was changed by this card, and
  no private content was committed.

## Related Items

- `RESEARCH-001` Feynman research-method vs opencode/ort (local-corpus null result; comparable baseline)
- `RESEARCH-002` Automate the Feynman-to-ort workable-chunk pipeline via the CLI (hold)
- `RESEARCH-003` Evaluate the external R&D department's evidence supplementation (open value gate)
- `RESEARCH-004` Provide the Feynman/Research-Department utility surface in `ort` (integration home)
- `RESEARCH-006` Feynman remote execution and session-analysis surface
- `invariants/research-department.md`, `invariants/integration-scope.md`
- External repo: `deepseek-ai/deepseek-harness`

## Notes

- Model is not the variable; this is a harness/tooling question, not a
  model-strength comparison.
- dsh is in developer preview with warned breaking changes — treat the checkout
  as a moving target and pin the revision used for any recorded run.
- A null or negative result (no advantage, or not worth integrating) closes this
  card the same as a positive one.
