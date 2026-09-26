---
id: "H-013"
title: "FEAT-065 Feynman GATE-001 result-vetting handoff"
status: "in_progress"
created: "2026-09-26"
updated: "2026-09-26"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["FEAT-065"]
---

# FEAT-065 Feynman GATE-001 Result-Vetting - Handoff

## Objective

Answer `FEAT-065`: **can Feynman and its research capabilities be used to fully
vet aa-studies `GATE-001` results and surface further opportunities/improvements
to the current (ideal) strategy set and finalist param sets?**

This is a **repeated, pre-completion review** of `GATE-001` (which is in
progress / near completion), not a post-gate comparison. Feynman is declared
capable of analyzing gate progress now; the review should be repeatable as the
gate matures. The deliverable is a defensible verdict plus the value-carrying
method components, backed by recorded commands and artifacts.

The pipeline-mechanics half (turning Feynman output into `ort`-executable
chunks) is `FEAT-066` and is **out of scope** here.

## Read This First

- Board card: `boards/todo/evaluate-feynman-research-method-vs-ort.md`
  (`FEAT-065`) - canonical scope, questions, non-goals, Done-when.
- `AGENTS.md` → "aa-studies Workflow (Live Research Repo)".
- aa-studies local checkout now exists at `$HOME/repos/aa-studies`
  (`development`, cloned 2026-09-26).
- Feynman local fork: `$HOME/repos/feynman-modded` (recorded `0.5.8`).

## Environment Rules (do not violate)

- **Always read/work aa-studies on `development`, never `main`.**
  `git -C "$HOME/repos/aa-studies" fetch origin development` first. Prefer
  reading the local project directory directly; `gh api ... ?ref=development` is
  a fallback only.
- This machine **cannot run** aa-studies optimization/backtest scripts (they
  need data mounting that is not present). Do not attempt them. Board, docs,
  skills, and read-only analysis of committed artifacts are allowed.
- Do **not** edit aa-studies, open/close `GATE-001`, or change any gate
  disposition. Feynman findings are **advisory**.
- **Never copy private aa-studies outputs, prompts, or secrets into this repo.**
  Record sanitized summaries only.
- Explicit `feynman "..."` invocations are required for auditable provenance.

## Context To Re-Verify Before Starting

Facts recorded 2026-09-26 (re-verify; do not reuse blindly):

- `gh repo view cchris-p/aa-studies` → private, default branch `main`.
- `GATE-001` ("V1 Readiness Gate - unlock actual analysis") is `todo` but
  **in progress / near completion** (Conditions A–C; B.1–B.7). Canonical docs:
  `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`,
  `docs/strategy-registry.md`; handoff `handoffs/07-v1-readiness-gate-handoff.md`.
- Gate result set to vet: parity closure for the four confirmed suites;
  execution-mode matrix (on-bar canonical); US-DST V1 baseline; Condition B.7
  robust readiness-param-set universes; and documented-negative/B.7 gaps
  (Balke parked, MTHR parked; TP universe in progress; MHAL forward-only).
- Feynman fork `@companion-ai/feynman` `0.5.8`; skills include deep-research,
  literature-review, paper-code-audit, replication, ml-training-recipe,
  source-comparison; agents researcher/reviewer/writer/verifier.
- aa-studies ships `docs/feynman_role_for_optimization.md` (an early attempt
  scoping Feynman as a local coding/verification copilot, excluding
  alphaXiv/literature workflows) - this item supersedes or supplements it.

## Phase 0 - Define the vetting target precisely

1. Fetch aa-studies `development`; list the current condition status of
   `GATE-001` and the register coverage (`docs/strategy-registry.md`).
2. Enumerate the **current (ideal) strategy set** and the **finalist param sets**
   the gate is locking in (per suite: Balke, Technical Pivot, MACD Thresholds,
   MACD Histogram Alerts; note the documented-negative gaps).
3. Write the vetting rubric: for each gate condition, what artifact/evidence
   proves it, and what an independent reviewer should be able to falsify.
4. Identify concrete **improvement questions** worth asking (e.g. MTHR's
   frequency-driven near-miss, Balke's parked negative, TP survivor robustness,
   cross-symbol/universe gaps).
5. Record the rubric and the read-only evidence map in the card's Dev Notes
   (sanitized; no private artifact contents).

Exit gate: a written rubric + target list that both arms can be run against.

## Phase 1 - Arm A: Feynman

- From the aa-studies tree (or with absolute paths), run explicit
  `feynman "..."` commands for the vetting and the improvement questions.
- Capture every invocation and the produced artifact path verbatim.
- Prefer Feynman's verifier/reviewer agents and its artifact-discipline tools
  where they add signal. Note whether literature/alphaXiv adds value for this
  objective (the old aa-studies doc excluded them).

Exit gate: Feynman produces a vetting result and improvement candidates.

## Phase 2 - Arm B: opencode/ort alone

- Run the **same** task with opencode/`ort` alone (no Feynman), using the
  product's own surfaces read from this repo (`wiki/cli-surface.md`,
  `crates/opencode-cli/`, `crates/opencode-server/`) - not assumed.
- Capture the session/commands and outputs.

Exit gate: a comparable vetting result and improvement list from Arm B.

## Phase 3 - Compare and verdict

Compare on the card's scope criteria: depth/structure, citation/source
verification, reproducibility/auditability, workable-chunk production, and
cost/complexity/latency/fork risk. Produce a clear verdict:

- **stronger / equal / weaker** for GATE-001 vetting, and
- **stronger / equal / weaker** for finding improvements.

Identify which method components carry the value and which are noise. A **null
result is valid** - if opencode/`ort` alone is as strong, record that and close.

## Phase 4 - Relationship to the old Feynman-role doc

Decide explicitly whether `FEAT-065` **supersedes or supplements**
`docs/feynman_role_for_optimization.md`, including whether the old session/notes
artifact still exists (may be stale). Record the decision on the card.

## Phase 5 - Follow-ups and closeout

- Split any follow-up (adopt a method component; more Feynman work;
  improvement candidates promoted into aa-studies) into their own board items or
  explicitly defer, per the card's Done-when.
- Update `FEAT-065` Dev Notes with: exact commands, observed results, verdict,
  value components, and the doc-relationship decision.
- Move the card `todo -> doing -> qa` (research item; keep it in `qa` until the
  user confirms). Do not mark it `done` unilaterally.

## Deliverable / PR Plan

- Research item: the durable output is a **sanitized vetting verdict** recorded
  on the `FEAT-065` card (and a companion note under `docs/` if the verdict
  needs more room). **No private aa-studies content is committed here.**
- Branch: `research/FEAT-065-feynman-gate-001-vetting` off `development`;
  PR into `development` with the findings. Keep the PR open for user review;
  do not merge on tests alone.

## Overall Verification

- Confirm both arms ran end to end on the same task before concluding.
- Confirm every claim cites a recorded command/artifact and re-reads
  `GATE-001` on aa-studies `development`.
- Confirm no gate disposition was changed and no private content leaked.
- Confirm the card's Done-when checklist is satisfied.

## Risks And Guardrails

- **Completion-dependency confusion**: do NOT wait for `GATE-001` to close;
  vet progress now and repeat later.
- **Branch confusion**: `main` is behind - always `development`.
- **Data-mount temptation**: optimization/backtest scripts cannot run here;
  treat this as a review/analysis task only.
- **Authority creep**: findings inform; `GATE-001`'s own rules stay
  authoritative.
- **Stale facts**: re-verify the recorded facts before reuse.

## Deferred / Out Of Scope

- `FEAT-066` Feynman→`ort` chunk-pipeline automation.
- Folding Feynman features into `ort`.
- Editing aa-studies or changing any gate disposition.
- Syncing/patching/re-forking `feynman-modded`.

## Execution Progress

- 2026-09-26: Handoff created. `FEAT-065` reframed as a repeated pre-completion
  GATE-001 result-vetting review; aa-studies cloned to `$HOME/repos/aa-studies`
  (`development`); `AGENTS.md` updated with the aa-studies workflow.
- No work started. Next agent begins at Phase 0.
- 2026-09-26: Phases 0–5 executed. Arm A (Feynman 0.5.8) and Arm B
  (opencode/`ort`) ran the same read-only task on aa-studies `development`
  @ `e7abe68` with the same model (`deepseek/deepseek-flash`; Feynman's default
  Anthropic model had no credits). Verdict: **EQUAL for vetting, EQUAL for
  improvement-finding — null result confirmed**. Both arms left aa-studies clean;
  no gate disposition changed. Sanitized note:
  `docs/research/FEAT-065-feynman-gate-001-vetting.md`. `FEAT-065` moved to `qa`.
  Branch `research/FEAT-065-feynman-gate-001-vetting`; PR open for review (not
  merged). Handoff remains `in_progress` until the user reviews/merges.
