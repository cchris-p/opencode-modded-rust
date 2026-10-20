---
id: "RESEARCH-007"
title: "Evaluate ORT agent harnesses on RuneBench (Qwen local + DeepSeek via Novita)"
priority: "P2"
type: "research"
area: "RESEARCH"
spec: "wiki/agent-evaluation-strategy.md"
status: "todo"
created: "2026-10-20"
---

# Evaluate ORT agent harnesses on RuneBench (Qwen local + DeepSeek via Novita)

## Summary

Run [RuneBench](https://maxbittker.github.io/runebench/) — a public benchmark that
scores coding agents on playing a RuneScape emulator through the
[`rs-sdk`](https://github.com/MaxBittker/rs-sdk) TypeScript SDK — with the **Rust
`ort` harness** as the agent runtime, on two model arms:

- **Arm Q** — the local Qwen the `ort-qwen` profile resolves (host primary Ollama
  model; on Jasper currently `ollama/qwen2.5-coder:14b-instruct-q4_K_M`).
- **Arm D** — the DeepSeek model already used through Novita:
  `novita-ai/deepseek/deepseek-r1-0528` (`ort-deepseek-novita` / `odn`).

The point is not a model leaderboard. It is an outside, adversarial evaluation of
the **harness itself** (tool-call reliability, long-horizon loop stability, MCP
tool driving, timeouts/compaction, and prompt/instruction behavior) on a workload
the repo's own task pack does not cover.

## Why this exists

- `wiki/agent-evaluation-strategy.md` and the parked `START-021`–`START-024`
  evaluation work are grounded in this repo's own daily-driver tasks. A public
  benchmark adds an independent, comparable signal.
- RuneBench is an "orient, decide, act" loop: the agent writes and executes code
  against a live game server, reads state, and iterates. That stresses exactly the
  capabilities ORT's V1 harness is supposed to be good at.
- The repo already shows we care about provider tool-transport parity
  (`anthropic-provider-tool-transport-parity`, `BUG-063`). This benchmark is a
  realistic way to find the next transport/loop bug rather than reasoning about it.

## What RuneBench actually is (from the source material)

- Agents play by writing/executing TypeScript against an emulated LostCity server;
  `rs-sdk` exposes an **MCP `execute_code`** tool that runs snippets with `bot`
  and `sdk` globals. That MCP surface is the natural integration point for `ort`.
- Each agent also gets a folder of game-wiki markdown for strategy.
- Scoring is **max XP rate per 15-second window** (not total XP), chosen to reward
  strategy/exploration over pure grind.
- Known limitations stated by the author: long runtime, small sample counts,
  environment complexity, and noise that hurts numerical comparability. Treat this
  as a capability/stability probe, not a precise ranking.
- The benchmark/comparison code lives at
  `https://github.com/MaxBittker/rs-bench`; the site source is
  `https://github.com/MaxBittker/RuneBench`; the MCP server/SDK is
  `https://github.com/MaxBittker/rs-sdk`.

## Scope

1. **Integration spike** — get `ort` driving `rs-sdk`'s MCP `execute_code`
   against a local server (preferred, controllable tickrate/state) or the demo
   server. Record the exact MCP config, workspace layout, and knowledge-folder
   wiring. If ORT's MCP client cannot carry this loop, that finding is itself the
   deliverable and becomes a follow-up card.
2. **Run the benchmark's own task set** for both arms, same tasks, same server
   settings, same seeds/start state where the harness allows.
3. **Record per-run evidence**: task, model, launcher profile, timeouts used,
   success/failure mode (tool error, transport error, context compaction, loop
   stall, panic), and the benchmark's XP-rate score.
4. **Harness-focused analysis**: which failures are harness bugs vs model
   capability; what would ORT need to be competitive (tool surface, retries,
   timeouts, compaction policy, MCP stability).
5. Where a concrete ORT defect is found, file separate `BUG-*`/`FEAT-*` cards
   instead of expanding this card.

Runtime controls: use `OPENCODE_RUN_TIMEOUT_MS=0` (or a deliberately chosen bound)
and `OPENCODE_STREAM_BUDGET_MS` for long runs; do not silently truncate a run and
report it as a failure.

## Environment prerequisites

- `rs-sdk` checkout with `bun install`; either the local three-process stack
  (engine, webclient bundler, gateway on `ws://localhost:7780`) or the headless
  lite runner per bot.
- **Arm Q needs a reachable Ollama.** Jasper has no Ollama installed today, so
  either point `ort-qwen` at a host that serves the model
  (`OPENCODE_MODEL_ORT_QWEN`) or install Ollama locally; record which was used.
- **Arm D needs `NOVITA_API_KEY`** in the OpenBao-rendered env consumed by
  `_opencode_rust_exec`.
- Build the binary first (`ort-build`) and record the commit under test.

## Non-goals

- Building a permanent evaluation platform, leaderboard, or CI benchmark.
- Tuning or post-training either model.
- Changing ORT's default model or provider lineup based on one benchmark.
- Running ORT's own MCP server inside the game loop, or multi-agent experiments
  (a separate future question).
- Committing benchmark private data, API keys, or transcripts of agent runs that
  contain secrets.

## Done when

- At least one full task is run end to end for **both** arms through `ort` with
  exact commands and the commit under test recorded.
- Per-run results and failure modes are written down in a `docs/` or `wiki/`
  report linked from this card (not only in chat).
- A clear harness verdict exists: what worked, what broke, and whether the
  failures are ORT-specific or model-specific.
- Any concrete ORT defects are filed as their own cards; the rest is explicitly
  deferred.

## Recommended verification

- Re-run one task per arm and confirm the recorded score is in the same range
  (acknowledges the benchmark's own noise caveat).
- Confirm the MCP `execute_code` path was actually exercised (game-state reads and
  actions observed), not a stub.
- Confirm no secrets or private data entered the commit.

## Related Items

- `START-021` Define V1 evaluation task pack (hold; same evaluation intent)
- `START-022` Define review quality rubric and fixtures (hold)
- `START-023` Persist structured evaluation results (hold)
- `START-024` Add lightweight evaluation rerun workflow (hold)
- `PHASE-004` Evaluation harness (archived phase parent)
- `agent-evaluation-strategy` / `define-agent-evaluation-strategy` (wiki + card)
- `BUG-063` DeepSeek native tool-call stream termination (provider transport risk)
- `anthropic-provider-tool-transport-parity` (tool transport risk)

## Notes

- Source talk: "RuneScape Bench: MMOs for Agents | Max Bittker", Recurse Center.
  Transcript pulled to the private `transcript-pulls` repo as
  `transcripts/cNm4zyUtWj0/` (https://www.youtube.com/watch?v=cNm4zyUtWj0).
- The talk also covers agent scamming/multi-agent economics and "breaking
  containment" — interesting, but out of scope for this card.
- Keep the harness comparison honest: a weak score may be model capability, not
  harness quality. State the attribution explicitly in the report.
