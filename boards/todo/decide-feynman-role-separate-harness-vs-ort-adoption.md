---
id: "FEAT-065"
title: "Decide Feynman's role: separate research harness vs features adopted into ort"
priority: "P2"
type: "research"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Decide Feynman's role: separate research harness vs features adopted into ort

## Summary

Determine what the `feynman` research agent is actually for, whether it can do real research work against the standstill `aa-studies` GATE-001 program, and whether its capabilities should be adopted into `ort` or left to run as an independent harness.

Working hypothesis to test: `ort` (a coding-session TUI/CLI) and `feynman` (a research agent on the Pi/TypeScript stack) are far enough apart in runtime, tooling, and purpose that `feynman` should keep operating as its own harness rather than be absorbed into `ort`.

This is a learn-and-decide item, not an implementation item.

## Why this exists

`feynman` is a separate upstream TypeScript research agent. `cchris-p/feynman-modded` is a custom fork synced periodically, so it is a moving target and not a dependency. Before treating any of it as product direction, we need one grounded write-up of what it offers, whether it is useful for the one real workflow we have, and whether that argues for porting features into `ort` or running it side by side.

The immediate and currently only use case is research for the private `aa-studies` program, which is stopped on `GATE-001`. That blocked program is the forcing scenario for this evaluation.

## What we know about aa-studies and GATE-001 (gh-verified 2026-09-26)

- `cchris-p/aa-studies` is private, default branch `main`, a boards-managed AA strategy research repo for forex strategies (Balke, Technical Pivot, MACD Thresholds, MACD Histogram Alerts, plus Connors/Welsh/Taylor/Satori).
- `GATE-001` "V1 Readiness Gate - unlock actual analysis" lives in `boards/todo/` and is the single gate that must open before AA-driven analysis that informs MT4 live testing. Canonical sources: `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`; program handoff `handoffs/07-v1-readiness-gate-handoff.md`.
- The gate is at a standstill / OPEN-pending on external delivery and deliberate holds, not on analysis tooling: `BALKE-006` (awaiting liaison deliverables), `AABT-005` (hold), and explicit future-stage holds (`TPIV-002`, `TPIV-012/014/017/018/019`, post-gate programs `GATE-002/003/004`).
- aa-studies already ships `docs/feynman_role_for_optimization.md`, which positions Feynman as a coding/verification copilot for the walkforward optimization loop, explicitly **not** the optimizer runtime, and explicitly tells readers **not** to use alphaXiv/literature workflows for that objective.
- aa-studies is commonly edited through OpenCode sessions, but that is not the same as running Feynman CLI commands; auditable Feynman usage means explicit `feynman "..."` invocations.

The standstill is important: the gate is blocked on liaison/engine delivery that a research agent cannot unblock directly. Any claim that Feynman helps must say concretely what work it moves.

## The decision to make

Pick one end state, or a hybrid with an explicit boundary:

1. **Adopt into ort** — fold selected Feynman research capabilities (workflows, research agents, literature/alphaXiv tools, citation verification) into this Rust product.
2. **Separate harness (working hypothesis)** — keep Feynman as an independent research tool invoked alongside `ort`, with `ort` remaining the coding/agent product and a documented handoff/provenance boundary.

Supporting questions: is the skills/subagent overlap enough to justify porting, or are the runtime (Pi/TypeScript vs Rust), provider/model assumptions, and research-tool integrations (alphaXiv, literature databases, web/PDF) too divergent?

## Reconciling with aa-studies' existing Feynman doc

`docs/feynman_role_for_optimization.md` restricts Feynman to coding-copilot duties and rules out literature workflows for that objective. This item must state whether a "Feynman for research" role (a) extends/supersedes that doc, (b) is a separate concern that leaves it intact, or (c) is out of scope. Surface the conflict rather than silently overriding another repo's standard.

## Questions to answer

- What is Feynman's capability envelope (deep research, literature review, paper/code audit, replication, training recipes, source comparison, citation verification, coding copilot), and which parts are research vs coding?
- Can Feynman do useful work on the *blocked* aa-studies program, or does the standstill depend on liaison/engine work no research agent can unblock?
- If the value is in the open gate's downstream V1 analysis, what is the smallest real research task that proves it?
- Is there any `ort`-relevant capability worth porting, or is the gap large enough that porting is not justified?
- If separate harness: what is the invocation/handoff boundary between `ort` (coding) and `feynman` (research), where do artifacts live, and who records provenance?
- What does the periodic fork sync imply for relying on Feynman (moving target, not a dependency)?

## Scope

- Inspect the local fork at `/home/admin-xx/repos/feynman-modded` (README, `skills/`, `prompts/`, `.feynman/agents/`, `src/`) and enumerate research vs coding capabilities with file references.
- Read the aa-studies GATE-001 card, `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`, and `docs/feynman_role_for_optimization.md` via `gh` (aa-studies is not local) and characterize the standstill.
- Run at least one real Feynman research task tied to aa-studies and record the exact command and outputs.
- Produce the adopt / separate-harness / hybrid recommendation with an explicit boundary.
- If adoption or a handoff boundary is recommended, split the smallest concrete slice into its own board item.

## Non-goals

- Implementing anything in `ort`.
- Editing aa-studies, or making `feynman` a dependency of either product.
- Syncing, patching, or re-forking `feynman-modded` as part of this item.
- Copying private aa-studies research outputs, prompts, or secrets into this repository.

## How to inspect and verify

Feynman is examined from its local checkout, not through `gh`. The fork is the working copy and source of truth for what the tool does:

- Local fork: `/home/admin-xx/repos/feynman-modded` (fork `cchris-p/feynman-modded`, synced periodically). Read `README.md`, `skills/`, `prompts/`, `.feynman/agents/`, and `src/` directly.
- Version is read from the local `package.json`, not a remote lookup.

`gh` is used for `aa-studies`, because it is private and not present locally:

- Read the gate card and docs via `gh api repos/cchris-p/aa-studies/contents/<path>`; confirm reachability with `gh repo view cchris-p/aa-studies --json name,isPrivate,defaultBranchRef,url`.
- Record the `gh` command and observed result at the point of each claim; re-verify when this item is revisited.

Facts recorded 2026-09-26 (re-verify before reuse):

- Local fork `/home/admin-xx/repos/feynman-modded`: package `@companion-ai/feynman` version `0.5.8`; research skills include deep-research, literature-review, paper-code-audit, replication, ml-training-recipe, source-comparison; bundled agents are researcher/reviewer/writer/verifier.
- `gh repo view cchris-p/aa-studies --json name,isPrivate,defaultBranchRef,url` → private, default branch `main`.
- `GATE-001` status `todo`, blocked on liaison/hold items, not on analysis tooling.

## Done when

- Feynman's research/coding capability envelope is written down with local-checkout file references.
- The aa-studies GATE-001 standstill is characterized with `gh`-verified evidence and an explicit statement of whether research can move it.
- At least one real aa-studies-related Feynman research run is documented end to end, with the exact command and observed outputs.
- A clear adopt vs separate-harness (or hybrid with boundary) recommendation exists.
- The relationship to `docs/feynman_role_for_optimization.md` is stated.
- Any adoption/handoff candidate is split into its own board item or explicitly deferred.

## Recommended verification

- Confirm the local `/home/admin-xx/repos/feynman-modded` checkout matches the recorded version and that `ort` capabilities were read from this repo, not assumed.
- Re-read `GATE-001` via `gh` and confirm the standstill still hinges on liaison/hold items rather than tooling.
- Smoke-test one Feynman research workflow tied to aa-studies and confirm outputs are usable before drawing any adoption conclusion.

## Related Items

- `FEAT-031` Investigate the builtin "general" agent and decide whether to remove it
- `SKILLS-001` Align skills with current and reference OpenCode behavior
- `SKILLS-002` Plan URL-backed skills parity
- `SKILLS-005` Compare skill referencing and per-step reinjection parity with vanilla
- `START-004` Assess current Rust state

## Notes

- Treat the fork as a periodically synced external reference, not a stable dependency; re-verify facts on each revisit.
- The aa-studies gate is blocked on liaison/engine delivery, not analysis tooling: verify that a research agent can actually change the outcome before assuming it helps.
- The likely outcome is a separate harness, but do not pre-commit to it; the adopt/separate decision must follow the evidence.
