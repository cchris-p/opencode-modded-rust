# Laya Integration

## Purpose

Define what `laya` is, why a fast local decision engine is interesting to `scopemux-code`, and the narrow boundary the Rust runtime should preserve so `laya` can be adopted later without becoming a hidden dependency or weakening runtime authority.

## What Laya Is

`laya` is a multilingual, non-autoregressive **System 1 decision engine**. It answers typed questions over any state in a single forward pass — no text generation, so nothing to parse and nothing to hallucinate.

- Output types: `choice` (finite labels), `score` (ordinal rubric), `noul` (calibrated `P(true)`).
- Three checkpoints: `laya` (English, ModernBERT-large), `laya-multilingual` (100+ languages, 8,192-token option), and `laya-typed-decisions` (typed-decision workflows).
- A `Router` detects script/language and dispatches to the right checkpoint; routing alone is sub-millisecond and never downloads a checkpoint.
- Latency is measured around 33 ms/question on a T4 GPU (about 200 ms/question on CPU).
- Schema-driven decisions: a JSON schema or pydantic model becomes questions, and `decide()` returns typed values plus calibrated per-field confidence.

Integration surfaces shipped by the project:

- `laya[serve]` / `laya-serve`: FastAPI HTTP server exposing `GET /health` and `POST /v1/systemone`, with optional bearer auth via `LAYA_API_KEY` (`laya/serve.py`).
- `laya[mcp]` / `laya-mcp-server`: an MCP stdio server exposing four tools — `laya_status`, `laya_route`, `laya_predict`, `laya_preset` (`laya/mcp/server.py`).
- Built-in presets: `guard` (jailbreak / prompt-injection / sensitive-data screening), `moderation`, `triage`, and `model_router` (`laya/presets.py`).
- Prediction hooks (`on_predict_start` / `on_predict_end` / `on_route` / `on_load` / `on_evict` / `on_error`) that can observe or reshape a call, plus `shortlist` for large label sets and a LangChain/LangGraph integration layer.
- `laya-ts`: ONNX-based TypeScript inference for Node and the browser; there is no Rust binding.

Reference: `~/repos/laya` (fork of `NandhaKishorM/laya`, upstream `https://github.com/NandhaKishorM/laya`), package `0.3.20`, inspected at commit `23a1752`. Licensed Apache-2.0. Laya's own `AGENTS.md` forbids depending on a hosted service; it is meant to run in-process or on the user's own hardware, which is compatible with this product's local-first stance.

## Why It Might Matter Here

`scopemux-code` is local-model-first and keeps the runtime authoritative over task state, lifecycle, verification, and permissions. Several of those decision points are bounded classification or gating problems that do not need a generative model:

- triaging an incoming request (intent, difficulty, whether tools are needed)
- pre-classifying a tool call as low/high risk before the permission prompt
- screening untrusted input or tool output for prompt injection, jailbreaks, or sensitive data
- a fast first-pass score on whether output satisfies explicit task criteria
- optional provider/model routing for a request

These are exactly Laya's `choice` / `score` / `noul` shape. A 33 ms local forward pass is a plausible alternative to spending a frontier-model turn on the same decision.

## Candidate Integration Surfaces

| Laya capability | Product need | Where it would plug in |
| --- | --- | --- |
| MCP stdio server (`laya_predict`, `laya_preset`, `laya_route`, `laya_status`) | quick structured decisions available to the agent as tools | the existing MCP client (`crates/opencode-mcp`) registers Laya as one MCP server |
| HTTP `/v1/systemone` (`laya-serve`) | runtime-driven decisions without model-in-the-loop tool calls | a Rust client calling a local sidecar, behind a decision-provider boundary |
| `guard` preset | screen untrusted prompt/tool content for injection or secrets | input/output screening near provider assembly and `wiki/provider-side-visibility.md` boundaries |
| `triage` / custom questions | classify a request or task before choosing a mode/agent | mode and agent selection, alongside `wiki/agent-modes-and-custom-agents.md` |
| `moderation` / `noul`-style checks | content and safety gates | policy checks on user-visible content |
| `score` / `decide` confidence | first-pass verification and review signal | verification/review context, without replacing the fresh-context review invariant |
| `model_router` preset | pick a provider/model for a request | optional provider selection; the product-owned default remains `deepseek/deepseek-flash` |
| `shortlist` | choose among many labels/tools | narrowing large option sets before a `choice` question |

Two adoption shapes fall out of this:

1. **Agent-visible (near-term, low coupling).** Run `laya-mcp-server` and let the agent call Laya as MCP tools. No Rust code changes beyond MCP registration; decisions remain model-driven and optional.
2. **Runtime-owned (strategic).** Insert a narrow **decision-provider** boundary in the Rust runtime and let a local Laya sidecar be one implementation of it. This is the shape that lets the runtime use Laya without handing it workflow authority.

## Required Abstraction Boundary

If Laya is adopted beyond the MCP-tool path, the runtime should preserve one explicit decision boundary, modeled on the retrieval boundary in `wiki/scopemux-integration-plan.md`:

1. The runtime defines a decision request from session, task, or tool state (objective, stage, candidate labels or rubric, relevant text).
2. A decision provider returns typed values, per-field confidence, and provenance.
3. The runtime decides how to act on those results — never the other way around.

The rule is that orchestration code consumes decision outputs; it must not embed Laya-specific calls throughout lifecycle, verification, or permission logic. The default provider can be a no-op or heuristic, so the product works with Laya absent.

## Guardrails and Invariants

Laya outputs are **evidence, not authority**. The following stay with the runtime:

- `invariants/runtime-lifecycle.md`: the runtime owns lifecycle transitions; the model (or Laya) only makes bounded decisions inside it.
- `invariants/task-state.md`: objective, criteria, stage, verification plan, and review result remain runtime-owned structured state.
- `invariants/verification.md`: the implementer is not the final authority, review runs in fresh context, and completion requires explicit verification. Laya may supply a first-pass signal, but it cannot become the authoritative reviewer or completion gate.
- Permission handling: Laya may pre-rank a tool call as risky to improve the prompt, but the human gate and the configured ruleset (`crates/opencode-permission`, `classify_permission` in `crates/opencode-server/src/agentic.rs`) remain authoritative. Laya must never auto-approve a call the ruleset would `Ask` or `Deny`.
- Provider policy: Laya runs locally, so it adds no outbound payload by itself, but any text fed to it is still subject to the sensitivity classification in `wiki/provider-side-visibility.md` if the result is later relayed to a remote model. The product-owned default provider/model is unchanged.

Additional operational rules:

- Keep Laya optional and non-blocking; the runtime must behave identically when no decision provider is configured.
- Pin the Laya fork commit; do not track upstream automatically.
- Treat confidence below an explicit threshold as "no decision" rather than a weak yes.

## Honest Limits

- There is no Rust binding. Every integration path runs a Python 3.10+ sidecar (`laya-mcp-server` over stdio or `laya-serve` over HTTP) with `torch` and Hugging Face checkpoints. That is a real operational cost inside a Rust product.
- Laya is decision-only. It is explicitly **not** for open Q&A, summarization, rewriting, code, or multi-hop reasoning, and `choice` sets above roughly 20 labels need `shortlist`.
- GPU gives the sub-40 ms figure; CPU is closer to 200 ms per question, and checkpoints download on first use.
- Calibrated probabilities are still model output, so the confidence rules above apply.
- `laya-ts` targets Node and the browser via ONNX, not Rust; it is not a shortcut around the sidecar for the core product.

## Phase Guidance

### V1

- Do not adopt Laya as a dependency.
- Preserve the decision-provider boundary only where it is cheap to do so; do not build it speculatively.
- Keep the existing heuristic permission and mode-selection logic authoritative.

### V2

- Allow the MCP-tool path as an opt-in experiment if it improves triage or guardrail quality.
- Prototype one runtime-owned decision (for example permission risk pre-classification) behind the boundary and compare against the heuristic baseline using `wiki/agent-evaluation-strategy.md`.

### V3+

- Expand Laya use only if measured task outcomes improve or if it demonstrably reduces cost/latency without weakening verification or permission guarantees.

## Non-Goals

- replacing the product's generative provider or default model
- letting Laya own task state, lifecycle, verification, or permission decisions
- a native Rust port of the Laya tokenizer/model
- shipping a Python/torch runtime as a hard V1 requirement
- automatic upstream tracking of the Laya fork

## Bottom Line

Laya is a strong fit for the fast, bounded decisions `scopemux-code` keeps making around a generative model, and its local-only design matches this product's stance. It should enter through an optional MCP-tool path or a narrow decision-provider boundary, as evidence the runtime may act on — never as an authority over lifecycle, task state, verification, or permissions.
