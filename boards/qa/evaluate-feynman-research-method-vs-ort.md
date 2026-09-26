---
id: "FEAT-065"
title: "Use Feynman to vet GATE-001 results and improve the ideal strategy and param sets"
priority: "P2"
type: "research"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-09-26"
---

# Use Feynman to vet GATE-001 results and improve the ideal strategy and param sets

## Summary

`feynman` runs as its own research harness, separate from `ort`. This item answers the remaining research question: **can Feynman and its research capabilities be used to fully vet GATE-001 results and surface further opportunities / improvements to the current (ideal) strategy set and finalist param sets?**

This is a **pre-completion evaluation/review step** for aa-studies `GATE-001`: Feynman is declared capable of analyzing GATE-001 progress now, with **no dependency on the gate being complete**. The review is expected to be **repeated** as the gate matures. "Ideal set" here means the **current** set, framed as a late-stage review lens rather than a one-shot post-gate audit.

This is deliberately scoped to one topic so it can be resolved in a dedicated session with full focus. The pipeline mechanics (how chunks are produced and dispatched to `ort`) are split out to `FEAT-066`.

## Settled model (context, not in question)

- Feynman is a **separate harness**, not folded into `ort`. `ort` stays the coding/agent product.
- Feynman's role is to run its research methods and produce **workable chunks** — self-contained, independently executable units of work.
- A workable chunk is represented as a **board item**.
- The operator processes those chunks **concurrently using `ort`** (one session per chunk; the runtime allows one active run per session and concurrency across sessions).
- The process is **manual for now**; automation via the CLI is `FEAT-066`.
- Feynman may analyze aa-studies `GATE-001` progress at any time; this evaluation does **not** wait for the gate to close.
- The review is designed to run **repeatedly** as the gate nears completion, not once.

## Why this exists

We have committed to running Feynman alongside `ort`, but not yet established whether its research method earns its place for two jobs on the only live workflow (private `aa-studies`): **(1) independently vetting the GATE-001 result set** as it nears completion, and **(2) finding more opportunities/improvements in the current strategy set and finalist param sets**. The gate locks in a V1 baseline (parity closure, execution modes, US-DST data, robust readiness-param-set universes, and documented-negative gaps); before it completes we want the strongest available independent review of that baseline and any missed improvements. If Feynman cannot beat just pointing opencode/`ort` at the same questions, the extra harness, moving-target fork, and separate provenance are not worth it. This must be measured, not assumed.

## Evaluation scope

Compare Feynman's research method against opencode/`ort` used alone, on the same real aa-studies `GATE-001` task read from the `development` branch (the gate is in progress / near completion):

- **Vetting GATE-001 progress.** Independently verify the gate's evidence and dispositions: parity convergence and closed dispositions for the four confirmed suites, the execution-mode matrix, US-DST enforcement, versioning, board-clear coverage read against `docs/strategy-registry.md`, and the Condition B.7 readiness-param-set universes (including documented-negative/B.7 gaps).
- **Finding opportunities/improvements.** Stress-test the current (ideal) strategy set and finalist param sets for missed survivors, near-miss cases (e.g. MTHR's frequency-driven negative), parameter-set robustness, and cross-symbol/universe gaps.
- Depth and structure of investigation (plan-first decomposition, parallel researchers, synthesis, verification pass).
- Citation quality and source verification (Feynman has a verifier agent, alphaXiv, literature databases, web/PDF tools, dead-link cleanup).
- Reproducibility and auditability (explicit `feynman "..."` invocations vs an opencode session record).
- Ability to produce workable chunks (board-item-shaped) that `ort` can execute, versus what opencode/`ort` produces alone.
- Cost, complexity, latency, and the periodic-fork moving-target risk.

Null result is valid: if opencode/`ort` alone is as strong, record that and close.

## What we know about aa-studies and GATE-001 (development, read 2026-09-26)

- `cchris-p/aa-studies` is private, default branch `main`, a boards-managed AA strategy research repo for forex strategies (Balke, Technical Pivot, MACD Thresholds, MACD Histogram Alerts, plus Connors/Welsh/Taylor/Satori and 12 auxiliary `GATE-AUX-*` strategies).
- **The `development` branch is the always-current branch for aa-studies work.** Use it for **every** interaction — both Feynman runs and any refining/working on this board item. Never treat `main` as current; the operator pushes GATE-001 changes to `development` first and it stays ahead of `main`.
- `GATE-001` "V1 Readiness Gate - unlock actual analysis" is the single gate that must open before AA-driven analysis that informs MT4 live testing. It is **in progress / near completion** at status `todo` in `boards/todo/`. Canonical sources: `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`, `docs/strategy-registry.md`; program handoff `handoffs/07-v1-readiness-gate-handoff.md`.
- The gate's substantive result set (what this item vets) now includes: parity closure for the four confirmed suites; the execution-mode matrix (on-bar canonical); the US-DST V1 baseline; the Condition B.7 **robust readiness-param-set universes**; and **documented-negative/B.7 gaps** where a suite has no deployable edge (Balke parked, MTHR parked; TP universe in progress; MHAL forward-only).
- "Ideal set" = the **current** strategy set and finalist param sets the gate is locking in; this review lens is applied **repeatedly before completion**, not once after.
- aa-studies ships `docs/feynman_role_for_optimization.md`, an **early attempt** scoping Feynman as a coding/verification copilot for the walkforward optimization loop and ruling out alphaXiv/literature workflows for that objective. This item **supersedes or supplements** that doc for the result-vetting objective (an old session and possibly-stale notes may exist).
- aa-studies also has `.opencode/skills/strategy-readiness-eval/SKILL.md` and `.opencode/skills/aa-canonical-readiness` for readiness evaluation — useful inspection surfaces, but not the gate itself.
- aa-studies is commonly edited through OpenCode sessions, which is not the same as explicit `feynman` CLI usage.

## Questions to answer

- Can Feynman's research method independently vet GATE-001 progress (evidence, dispositions, readiness-param-set universes, documented-negative gaps) more strongly than opencode/`ort` alone?
- Does it surface missed opportunities or concrete improvements to the current ideal strategy set and finalist param sets?
- Which method components carry the value for a repeated pre-completion review (research agents, verification, literature/alphaXiv tools, artifact discipline), and which are noise?
- Can Feynman's findings legitimately change any gate disposition, or do they only inform (gate authority stays with `GATE-001`)?
- What is the relationship to `docs/feynman_role_for_optimization.md` (supersede vs supplement), and does an old session/notes artifact still exist?

## Non-goals

- Building the Feynman→`ort` automation (`FEAT-066`).
- Folding Feynman features into `ort` (decided against).
- Editing aa-studies, opening/closing `GATE-001`, or changing any gate disposition — Feynman findings inform; the gate's own rules remain authoritative.
- Making `feynman` a dependency of either product.
- Syncing, patching, or re-forking `feynman-modded`.
- Copying private aa-studies research outputs, prompts, or secrets into this repository.

## How to inspect and verify

Feynman is examined from its local checkout, not through `gh`:

- Local fork: `/home/admin-xx/repos/feynman-modded` (fork `cchris-p/feynman-modded`, synced periodically). Read `README.md`, `skills/`, `prompts/`, `.feynman/agents/`, and `src/` directly.
- Version is read from the local `package.json`, not a remote lookup.
- `ort` surfaces are read from this repo (`wiki/cli-surface.md`, `crates/opencode-cli/`, `crates/opencode-server/`), not assumed.

`aa-studies` is **always read on the `development` branch**, whichever path is used below — this applies equally to Feynman runs and to any refining/working on this board item.

**Path 1 (preferred): read the local project directory directly.** The aa-studies project directory can be read directly from `$HOME/repos/aa-studies`, like any other local checkout:

- If it is not present yet, clone it there first (the private repo needs auth) so later runs can read it directly instead of via the API.
- Keep it current: `git -C "$HOME/repos/aa-studies" fetch origin development` and inspect `development` (check it out, or use `git -C "$HOME/repos/aa-studies" show development:<path>` / a temporary worktree if you must not disturb the existing checkout).
- Read the gate card, `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`, `docs/strategy-registry.md`, and skills directly from the filesystem.

**Path 2 (fallback): read from GitHub via `gh`.** Use this only when no local directory is available. **Use `?ref=development`**, not `main`:

- Read the gate card and docs via `gh api repos/cchris-p/aa-studies/contents/<path>?ref=development`; confirm reachability with `gh repo view cchris-p/aa-studies --json name,isPrivate,defaultBranchRef,url` and list the branch tree with `gh api "repos/cchris-p/aa-studies/git/trees/development?recursive=1"`.
- Record the `gh` command and observed result at the point of each claim; re-verify when this item is revisited.

Facts recorded 2026-09-26 (re-verify before reuse):

- Local fork `/home/admin-xx/repos/feynman-modded`: package `@companion-ai/feynman` version `0.5.8`; research skills include deep-research, literature-review, paper-code-audit, replication, ml-training-recipe, source-comparison; bundled agents are researcher/reviewer/writer/verifier.
- `gh repo view cchris-p/aa-studies` → private, default branch `main`; `GATE-001` on `development` is `todo` but **in progress / near completion** (Conditions A–C; B.1–B.7), with Balke and MTHR parked as documented-negative/B.7 gaps and TP's robust universe campaign in progress.

## Done when

- At least one real aa-studies `GATE-001` vetting task is run through Feynman and through opencode/`ort` alone, with exact commands and observed outputs recorded, and the same task is repeatable as the gate nears completion.
- A clear verdict exists: Feynman's method is stronger, equal, or weaker for vetting GATE-001 results and for finding improvements to the ideal strategy set + finalist param sets.
- The value-carrying method components are identified.
- The relationship to `docs/feynman_role_for_optimization.md` (supersede or supplement) is stated, including whether an old session/notes artifact still exists.
- Any follow-up (e.g. adopting a method component, or further Feynman work) is split into its own board item or explicitly deferred.

## Recommended verification

- Confirm the local `/home/admin-xx/repos/feynman-modded` checkout matches the recorded version and that `ort` capabilities were read from this repo, not assumed.
- Re-read `GATE-001` on aa-studies `development` via `gh` and confirm the current condition status and documented-negative gaps before drawing conclusions.
- Run both arms of the comparison end to end before drawing a conclusion; capture the raw artifacts, not just summaries.
- Confirm findings are treated as advisory: no gate disposition is changed by this item.

## Related Items

- `FEAT-066` Automate the Feynman-to-ort workable-chunk pipeline via the CLI
- `CLI-001`/`CLI-006` CLI task send/view/status (chunk execution surface)
- `CLI-002` Route `opencode run` through the canonical session runtime
- `FEAT-031` Investigate the builtin "general" agent and decide whether to remove it
- `SKILLS-002` Plan URL-backed skills parity
- `START-004` Assess current Rust state
- `H-013` `handoffs/2026-09-26-feat-065-feynman-gate-001-vetting-handoff.md` - next-agent runbook for this item
- aa-studies `GATE-001`, `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`, `docs/strategy-registry.md`, `docs/feynman_role_for_optimization.md` (external, private)

## Notes

- The harness decision is final: Feynman is separate. This item is only about method strength for vetting and improvement-finding.
- This is a **pre-completion** review: Feynman can analyze GATE-001 progress now, and the review should be repeatable as the gate matures. "Ideal set" means the current set.
- **Always read aa-studies on `development`** — for Feynman runs and for refining/working this board item alike. `main` is behind.
- Prefer reading the **local project directory** at `$HOME/repos/aa-studies` directly; use the `gh` API with `?ref=development` only as a fallback when no local checkout is available.
- Feynman findings are advisory — they do not open/close `GATE-001` or change dispositions.
- `docs/feynman_role_for_optimization.md` is an early attempt; this item supersedes or supplements it (an old session/notes artifact may still exist and may be stale).
- "Stronger research method than opencode alone" is a falsifiable claim; a null result is a valid conclusion.

## Dev Notes

### 2026-09-26 — comparison executed; verdict EQUAL (null)

Executed handoff `H-013` end to end (Phases 0–5). Companion sanitized note:
`docs/research/FEAT-065-feynman-gate-001-vetting.md`.

**Environment / exact commands**

- aa-studies refetched and read on `development` @ `e7abe68` (never `main`);
  `git status` confirmed clean after both arms. No optimization/backtest runs.
- Feynman 0.5.8 was not previously installed and the checkout at
  `$HOME/repos/feynman-modded` cannot run here (needs Node >=22.22, host has
  v20). Installed the released 0.5.8 standalone bundle.
- Feynman's default model `anthropic/claude-opus-5-5` failed with
  "credit balance is too low"; ran Arm A with
  `FEYNMAN_MODEL=deepseek/deepseek-flash` to hold the model constant with Arm B.
- Arm A: `FEYNMAN_MODEL=deepseek/deepseek-flash feynman "<shared task>"` from the
  aa-studies tree. Persisted its own report artifact outside the repo.
- Arm B: `opencode run --dir <aa-studies> -m deepseek/deepseek-flash "<shared task>"`
  using `target/debug/opencode` (built 2026-09-25). Same prompt text, same repo,
  same model as Arm A.
- Raw captures: `/tmp/opencode/feat-065/arm-a-feynman.txt`,
  `/tmp/opencode/feat-065/arm-b-opencode.txt` (not committed; no private content).

**Observed result**

- Both arms delivered comparable structured condition vetting (A GAP; B.1–B.7
  partial/verified with cross-document drift; C verified) and heavily
  overlapping improvement lists.
- Each caught one material issue the other missed. Arm A found an MHAL `v1.02`
  B.7 timeframe-coverage defect (artifact declares four established TFs but
  contains only two) — independently re-verified here. Arm B found that the
  "MQL4 + MQL5 + AA" condition header is structurally unverifiable for V1 because
  MQL5/MT5 is out of scope.
- Arm A ~4.6 min wall clock; Arm B ~1.6 min. Arm A adds a separate harness,
  provider-auth surface, own Node runtime, and a moving-target fork.

**Verdict**

- Vetting `GATE-001`: **EQUAL**. Finding improvements: **EQUAL**.
- **Null result confirmed**: Feynman's research method is not stronger than
  opencode/`ort` alone for this objective, at ~3x latency and higher operational
  complexity.

**Value components**

- Carries value: ordinary local-file reasoning + review-artifact discipline
  (reproducible in `ort` by redirecting `opencode run`).
- Marginal: explicit-invocation provenance.
- No demonstrated value here: literature/alphaXiv/web tools (unused), and any
  multi-agent decomposition/verifier pass (not observable).

**Doc relationship**

- **Supplement, not supersede** `docs/feynman_role_for_optimization.md`: that doc
  scopes a different objective (walkforward optimization copilot) and its
  no-alphaXiv ruling is reinforced here. Old session artifact
  `session-ses_fd61.md` exists but has no Feynman references; the durable
  aa-studies artifact remains that doc.

**Guardrails**

- Read-only on aa-studies; no edits, no gate-disposition change; findings
  advisory; no private content committed.

### Follow-ups (explicitly deferred / advisory)

- **Method adoption**: deferred — null result gives no method component to adopt.
  Optional low-cost item if desired: standardize persisting `opencode run`
  research output to a file.
- **Further Feynman work on this objective**: explicitly deferred (no advantage
  shown).
- **aa-studies improvement candidates**: advisory only, operator-owned,
  aa-studies not edited. Consolidated list recorded in the companion note
  (`docs/research/FEAT-065-feynman-gate-001-vetting.md`).
- **`FEAT-066`** (Feynman→`ort` chunk pipeline) remains out of scope and
  unaffected.
