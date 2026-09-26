---
id: "FEAT-065"
title: "Equip the Feynman harness to run ort processes and evaluate its research method"
priority: "P2"
type: "research"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Equip the Feynman harness to run ort processes and evaluate its research method

## Summary

`feynman` stays its own harness — that decision is made and is not part of this item. The open work is now:

1. **Equip Feynman to drive `ort` processes** when its research methods need coding/agent execution, instead of being limited to its own bundled tools.
2. **Evaluate whether Feynman's research method is actually stronger than opencode/`ort` alone** for the workflow we have.

This is a design-and-evaluate item, not an implementation item; concrete build work splits out afterward.

## Decision (settled)

- Feynman is **not** folded into `ort`. It runs as an independent research harness alongside this product.
- `ort` remains the coding/agent product; Feynman remains the research agent.
- The remaining challenge is the boundary: how Feynman reaches into `ort` when a research method needs to run a coding/agent process, and whether that combination beats using opencode/`ort` alone.

## Why this exists

The only real workflow today is research for the private `aa-studies` program, which is stopped on `GATE-001`. Feynman was built for exactly this kind of research, but its value over just driving opencode/`ort` directly is unproven. Before investing in any integration, we need to (a) define how Feynman would invoke `ort` processes at all, and (b) get evidence on whether its research methods outperform the plain agent.

## The integration question: how Feynman reaches `ort`

`ort` already exposes non-interactive surfaces Feynman could shell out to or call:

- `opencode run "<message>"` — single non-interactive run (`crates/opencode-cli`; note `CLI-002` still uses the interim `AgentExecutor` loop).
- `opencode task new|send|view|status` and `task target` — explicit server/session targeting (`CLI-001`/`CLI-006`, done).
- `opencode serve` + `opencode attach <url>` — detached/headless server control.
- HTTP API on the server, and the existing tool/plugin/MCP surfaces.

Questions this item must answer: which mechanism is the right contract for a research harness, how does Feynman discover the `ort` binary and workspace, how are results/artifacts returned, and how is provenance recorded? Keep this as a *documented boundary* with the smallest viable mechanism; do not build one here.

## The evaluation question: is Feynman's research method stronger?

Compare, on the same real aa-studies task, Feynman's research method against opencode/`ort` used alone:

- Depth, citation quality, and source verification (Feynman has a verifier agent, alphaXiv, literature databases, web/PDF tools).
- Reproducibility and auditability of the method (explicit `feynman "..."` invocations vs an opencode session).
- Usefulness for the blocked program — the gate is stuck on liaison/engine delivery, so state concretely what research can and cannot move.
- Cost, complexity, and moving-target risk of the periodically synced fork.

Null result is acceptable: if opencode/`ort` alone is just as strong, say so and close.

## What we know about aa-studies and GATE-001 (gh-verified 2026-09-26)

- `cchris-p/aa-studies` is private, default branch `main`, a boards-managed AA strategy research repo for forex strategies (Balke, Technical Pivot, MACD Thresholds, MACD Histogram Alerts, plus Connors/Welsh/Taylor/Satori).
- `GATE-001` "V1 Readiness Gate - unlock actual analysis" (`boards/todo/`) is the single gate that must open before AA-driven analysis that informs MT4 live testing. Canonical sources: `docs/v1-readiness-gate.md`, `docs/strategy-readiness.md`; program handoff `handoffs/07-v1-readiness-gate-handoff.md`.
- The gate is at a standstill / OPEN-pending on external delivery and deliberate holds, not on analysis tooling: `BALKE-006` (awaiting liaison deliverables), `AABT-005` (hold), and future-stage holds (`TPIV-002`, `TPIV-012/014/017/018/019`, post-gate `GATE-002/003/004`).
- aa-studies ships `docs/feynman_role_for_optimization.md`, which scopes Feynman as a coding/verification copilot for the walkforward optimization loop, not the optimizer runtime, and rules out alphaXiv/literature workflows for that objective. A "Feynman for research" role must state whether it extends, leaves intact, or supersedes that doc.
- aa-studies is commonly edited through OpenCode sessions, which is not the same as explicit `feynman` CLI usage.

## Questions to answer

- What is the minimal contract for Feynman to run an `ort` process: `run`, `task`, `serve`+HTTP, or a plugin/tool bridge? What does each require of `ort`?
- How should Feynman's research methods compose an `ort` call (input, workspace, session target, output artifacts, failure handling)?
- Does Feynman's research method (research agents, verification, literature tools) measurably outperform opencode/`ort` alone on a real aa-studies task?
- Can research move the GATE-001 standstill at all, or is the value only downstream once the gate opens?
- What is the relationship to `docs/feynman_role_for_optimization.md`?
- What does the periodic fork sync mean for relying on any Feynman-based method?

## Scope

- Inspect the local fork at `/home/admin-xx/repos/feynman-modded` (README, `skills/`, `prompts/`, `.feynman/agents/`, `src/`) and map its research methods and extension points with file references.
- Inspect the `ort` CLI/HTTP surfaces in this repo and document the candidate mechanism(s) by which Feynman could run an `ort` process.
- Run one real aa-studies-related research task through Feynman and, where practical, the same task through opencode/`ort` alone; record commands and outputs.
- Produce a comparative verdict on research-method strength, plus a recommended integration boundary (or "no integration needed").
- Split any concrete build work (a Feynman skill/tool, a wrapper, or an `ort` surface change) into its own board item.

## Non-goals

- Folding Feynman features into `ort` (decided against).
- Implementing the Feynman↔`ort` integration or changing `ort` surfaces in this item.
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
- `ort` non-interactive surfaces available: `run`, `task new|send|view|status`, `task target`, `serve`, `attach` (`wiki/cli-surface.md`).
- `gh repo view cchris-p/aa-studies` → private, default branch `main`; `GATE-001` status `todo`, blocked on liaison/hold items, not on analysis tooling.

## Done when

- The candidate mechanisms for Feynman to run an `ort` process are documented, with a recommended minimal contract (or a documented decision that none is needed).
- Feynman's research methods and extension points are written down with local-checkout file references.
- At least one real aa-studies-related research task is run and compared against opencode/`ort` alone, with exact commands and observed outputs.
- A clear verdict exists on whether Feynman's research method is stronger, equal, or weaker.
- The relationship to `docs/feynman_role_for_optimization.md` is stated.
- Any concrete build work is split into its own board item or explicitly deferred.

## Recommended verification

- Confirm the local `/home/admin-xx/repos/feynman-modded` checkout matches the recorded version and that `ort` surfaces were read from this repo, not assumed.
- Re-read `GATE-001` via `gh` and confirm the standstill still hinges on liaison/hold items.
- Run the comparison task end to end before drawing any research-strength conclusion; capture both the Feynman command and the opencode/`ort` command.

## Related Items

- `CLI-001`/`CLI-006` CLI task send/view/status (the likely integration surface)
- `CLI-002` Route `opencode run` through the canonical session runtime
- `FEAT-031` Investigate the builtin "general" agent and decide whether to remove it
- `SKILLS-002` Plan URL-backed skills parity
- `START-004` Assess current Rust state

## Notes

- The harness decision is final: Feynman is separate. This item is about the boundary and the evidence, not about re-litigating adoption.
- The aa-studies gate is blocked on liaison/engine delivery, not analysis tooling: verify that a research agent can actually change the outcome before assuming it helps.
- "Stronger research method than opencode alone" is a falsifiable claim; a null result is a valid conclusion.
