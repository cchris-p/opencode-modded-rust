---
id: "FEAT-065"
title: "Evaluate whether Feynman's research method beats opencode/ort alone"
priority: "P2"
type: "research"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Evaluate whether Feynman's research method beats opencode/ort alone

## Summary

`feynman` runs as its own research harness, separate from `ort`. This item answers the remaining research question: **does Feynman's research method actually enable a stronger method than using opencode/`ort` alone?**

This is deliberately scoped to one topic so it can be resolved in a dedicated session with full focus. The pipeline mechanics (how chunks are produced and dispatched to `ort`) are split out to `FEAT-066`.

## Settled model (context, not in question)

- Feynman is a **separate harness**, not folded into `ort`. `ort` stays the coding/agent product.
- Feynman's role is to run its research methods and produce **workable chunks** — self-contained, independently executable units of work.
- A workable chunk is represented as a **board item**.
- The operator processes those chunks **concurrently using `ort`** (one session per chunk; the runtime allows one active run per session and concurrency across sessions).
- The process is **manual for now**; automation via the CLI is `FEAT-066`.

## Why this exists

We have committed to running Feynman alongside `ort`, but not yet established whether its research method is better than just pointing opencode/`ort` at the same questions. If it is not, the extra harness, moving-target fork, and separate provenance are not worth it. This must be measured, not assumed.

## Evaluation scope

Compare Feynman's research method against opencode/`ort` used alone, on the same real task from the only live workflow (private `aa-studies`, currently stopped on `GATE-001`):

- Depth and structure of investigation (plan-first decomposition, parallel researchers, synthesis, verification pass).
- Citation quality and source verification (Feynman has a verifier agent, alphaXiv, literature databases, web/PDF tools, dead-link cleanup).
- Reproducibility and auditability (explicit `feynman "..."` invocations vs an opencode session record).
- Ability to produce workable chunks (board-item-shaped) that `ort` can execute, versus what opencode/`ort` produces alone.
- Cost, complexity, latency, and the periodic-fork moving-target risk.

Null result is valid: if opencode/`ort` alone is as strong, record that and close.

## What we know about aa-studies and GATE-001 (gh-verified 2026-09-26)

- `cchris-p/aa-studies` is private, default branch `main`, a boards-managed AA strategy research repo for forex strategies (Balke, Technical Pivot, MACD Thresholds, MACD Histogram Alerts, plus Connors/Welsh/Taylor/Satori).
- `GATE-001` "V1 Readiness Gate - unlock actual analysis" (`boards/todo/`) is the single gate that must open before AA-driven analysis that informs MT4 live testing. Canonical sources: `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`; program handoff `handoffs/07-v1-readiness-gate-handoff.md`.
- The gate is at a standstill / OPEN-pending on external delivery and deliberate holds, not on analysis tooling: `BALKE-006` (awaiting liaison deliverables), `AABT-005` (hold), and future-stage holds (`TPIV-002`, `TPIV-012/014/017/018/019`, post-gate `GATE-002/003/004`).
- aa-studies ships `docs/feynman_role_for_optimization.md`, which scopes Feynman as a coding/verification copilot for the walkforward optimization loop, not the optimizer runtime, and rules out alphaXiv/literature workflows for that objective. State whether this research-method evaluation extends, leaves intact, or supersedes that doc.
- aa-studies is commonly edited through OpenCode sessions, which is not the same as explicit `feynman` CLI usage.

## Questions to answer

- Does Feynman's research method produce a measurably better brief/chunk set than opencode/`ort` alone on a real aa-studies question?
- Which method components carry the value (research agents, verification, literature/alphaXiv tools, artifact discipline), and which are noise?
- Can the method move the GATE-001 standstill at all, or is its value only downstream once the gate opens?
- Are Feynman-produced chunks better shaped for `ort` execution than chunks produced by opencode/`ort` alone?
- What is the relationship to `docs/feynman_role_for_optimization.md`?

## Non-goals

- Building the Feynman→`ort` automation (`FEAT-066`).
- Folding Feynman features into `ort` (decided against).
- Editing aa-studies, or making `feynman` a dependency of either product.
- Syncing, patching, or re-forking `feynman-modded`.
- Copying private aa-studies research outputs, prompts, or secrets into this repository.

## How to inspect and verify

Feynman is examined from its local checkout, not through `gh`:

- Local fork: `/home/admin-xx/repos/feynman-modded` (fork `cchris-p/feynman-modded`, synced periodically). Read `README.md`, `skills/`, `prompts/`, `.feynman/agents/`, and `src/` directly.
- Version is read from the local `package.json`, not a remote lookup.
- `ort` surfaces are read from this repo (`wiki/cli-surface.md`, `crates/opencode-cli/`, `crates/opencode-server/`), not assumed.

`gh` is used for `aa-studies`, because it is private and not present locally:

- Read the gate card and docs via `gh api repos/cchris-p/aa-studies/contents/<path>`; confirm reachability with `gh repo view cchris-p/aa-studies --json name,isPrivate,defaultBranchRef,url`.
- Record the `gh` command and observed result at the point of each claim; re-verify when this item is revisited.

Facts recorded 2026-09-26 (re-verify before reuse):

- Local fork `/home/admin-xx/repos/feynman-modded`: package `@companion-ai/feynman` version `0.5.8`; research skills include deep-research, literature-review, paper-code-audit, replication, ml-training-recipe, source-comparison; bundled agents are researcher/reviewer/writer/verifier.
- `gh repo view cchris-p/aa-studies` → private, default branch `main`; `GATE-001` status `todo`, blocked on liaison/hold items, not on analysis tooling.

## Done when

- At least one real aa-studies-related task is run through Feynman and through opencode/`ort` alone, with exact commands and observed outputs recorded.
- A clear verdict exists: Feynman's method is stronger, equal, or weaker.
- The value-carrying method components are identified.
- The relationship to `docs/feynman_role_for_optimization.md` is stated.
- Any follow-up (e.g. adopting a method component, or further Feynman work) is split into its own board item or explicitly deferred.

## Recommended verification

- Confirm the local `/home/admin-xx/repos/feynman-modded` checkout matches the recorded version and that `ort` capabilities were read from this repo, not assumed.
- Re-read `GATE-001` via `gh` and confirm the standstill still hinges on liaison/hold items.
- Run both arms of the comparison end to end before drawing a conclusion; capture the raw artifacts, not just summaries.

## Related Items

- `FEAT-066` Automate the Feynman-to-ort workable-chunk pipeline via the CLI
- `CLI-001`/`CLI-006` CLI task send/view/status (chunk execution surface)
- `CLI-002` Route `opencode run` through the canonical session runtime
- `FEAT-031` Investigate the builtin "general" agent and decide whether to remove it
- `SKILLS-002` Plan URL-backed skills parity
- `START-004` Assess current Rust state

## Notes

- The harness decision is final: Feynman is separate. This item is only about method strength.
- The aa-studies gate is blocked on liaison/engine delivery, not analysis tooling: verify that a research agent can actually change the outcome before assuming it helps.
- "Stronger research method than opencode alone" is a falsifiable claim; a null result is a valid conclusion.
