---
id: "RESEARCH-007"
title: "Breadth survey: improving local qwen3:30b instantiation, reasoning, and effectiveness"
priority: "P2"
type: "research"
area: "RESEARCH"
spec: "invariants/providers.md"
status: "todo"
created: "2026-10-12"
---

# Breadth survey: improving local qwen3:30b instantiation, reasoning, and effectiveness

## Summary

Tracker for a **breadth-first** survey of the local Qwen-via-Ollama path (`ollama/qwen3:30b`, the
ScopeMux activation model): where its reasoning/effectiveness actually lives across model choice,
Ollama instantiation parameters, chat template, tool-call wire format, reasoning surface, prompt
scaffolding, the agent loop, routing policy, and measurement.

Deliberately coverage, not depth. The full scope — research questions, the nine-row coverage map with
sources, the breadth-vs-depth policy, and the recommended two-track approach — lives in the companion
document:

- **`docs/research/RESEARCH-007-qwen-effectiveness-breadth-survey.md`**

The starting evidence is `BUG-065`: a live qwen3:30b session loaded a skill, made zero tool calls, and
fabricated a board answer, while the default DeepSeek model handles the same prompt conflict far
better.

## Why this exists

- `BUG-062` proved the qwen path had a real parser gap; its QA residual ("intermittently answers
  without calling tools even with tools attached") is a **model/behavior** problem, not transport.
- `EPIC-001` documents that reliability fixes have been "reactive and one-symptom-at-a-time." A single
  qwen effectiveness survey avoids repeating that trap for the local model.
- Qwen-via-Ollama is the only model that activates the native ScopeMux provider
  (`invariants/providers.md`), so its effectiveness gates that integration's value.
- Operator prior: DeepSeek (even the older novita default) reasons more reliably, localizing the qwen
  gap to instruction-following/obedience under conflicting prompts and/or model size/quantization.

## Key questions

See the doc for the full question list. Headline: **is qwen's failure model capability or our
scaffolding**, and what is the cheapest measurable lift — prompt, runtime params, or model/quant swap?

## Recommended approach (summary)

- **Track 1 — product scaffolding (in progress as `BUG-065`):** fix the fallback prompt, skill
  directive, and optional loop guard.
- **Track 2 — bounded qwen A/B:** on `QA-002` + the `BUG-065` fixture, sweep
  {temperature, `num_ctx`, thinking on/off, prompt variant} and measure tool-loop success and
  fabrication rate.
- **Routing fallback:** if qwen cannot be made reliable, narrow its role per `invariants/providers.md`.

## Non-goals

- Deep research on any single row before the breadth pass is recorded.
- Changing ScopeMux provider behavior or retrieval (separate concern).

## Done when

- The companion doc's nine rows each carry a one-line finding plus a source.
- One bounded A/B result exists with exact params/prompts and repeated-run outcome.
- A single recommendation is recorded and any concrete change is split into its own card.

## Related Items

- `docs/research/RESEARCH-007-qwen-effectiveness-breadth-survey.md` — the full survey scope and
  coverage map.
- `BUG-065` Fallback system prompt drives Qwen/DeepSeek to answer without tools and fabricate state.
- `EPIC-001` DeepSeek default-model reliability hardening.
- `BUG-062` qwen3:30b Ollama tool-loop investigation.
- `QA-002` Deterministic DeepSeek stream/loop fixture harness.
- `SCOPE-005` ScopeMux provider activation for local qwen.
- `invariants/providers.md`, `invariants/coding-session-behavior.md`.

## Notes

- Operator will review the companion doc before work proceeds.
