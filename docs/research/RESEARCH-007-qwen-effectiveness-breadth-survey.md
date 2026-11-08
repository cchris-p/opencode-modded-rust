# RESEARCH-007 — Qwen3 local effectiveness breadth survey

**Date:** 2026-10-12
**Card:** `RESEARCH-007` (`boards/todo/breadth-survey-local-qwen-instantiation-reasoning-effectiveness.md`)
**Trigger:** `BUG-065` — a live `ollama/qwen3:30b` session loaded a skill, made zero tool calls, and
fabricated a board answer.
**Status:** Survey scope recorded; **breadth-first, not depth**. Depth is deferred to the one row that
wins a bounded A/B.

---

## Purpose

Map the whole "Qwen effectiveness surface" for the local Qwen-via-Ollama path (`ollama/qwen3:30b`, the
ScopeMux activation model) so the next round of work prices the cheap levers first instead of repeating
`EPIC-001`'s "reactive and one-symptom-at-a-time" pattern.

This is **coverage, not depth**: cover every row with a one-line finding and a source, then drill into
exactly one lever.

## Background

- `BUG-062` proved the qwen path had a real parser gap; its QA residual — "intermittently answers
  without calling tools even with tools attached" — is a **model/behavior** problem, not transport.
- `BUG-065` is the stark instance: the model obeyed the fallback prompt's "one word answers are best"
  and skipped the loaded skill workflow entirely.
- Qwen-via-Ollama is the only model that activates the native ScopeMux provider
  (`invariants/providers.md`), so its effectiveness gates that integration's value, not just chat.
- Operator prior: DeepSeek (and even the older novita DeepSeek default) reasons more reliably. That
  localizes the qwen gap to **instruction-following/obedience under conflicting prompts** and/or
  **model size / quantization** — both testable, both falsifiable.

## Research questions (answer before changing anything)

1. Is qwen's failure **model capability** or **our scaffolding**? (`BUG-065` says the prompt conflict
   is real and fixable.)
2. Which instantiation parameter (context length, quantization, temperature/top_p, thinking on/off)
   moves tool-use reliability the most?
3. Does Ollama+qwen prefer native structured `tool_calls` or in-content `<tool_call>` markup, and does
   the Qwen chat template render the tool schema correctly?
4. Does Qwen3 "thinking mode" help or hurt agentic instruction-following (and the brevity conflict)?
5. Should local qwen be a default agentic model at all, or a narrowed/opt-in local path?
6. What is the cheapest measurable lift: prompt scaffolding, runtime params, or a model/quant swap?

## Coverage map (topics, one line each, with sources)

Breadth is organized as the full effectiveness surface; depth is deferred to whichever row wins.

1. **Model choice & quantization** — Qwen3 30B-A3B (MoE) vs dense variants; `Q4_K_M` vs `Q8_0`; which
   build/tag Ollama serves by default. *Sources: Ollama library `qwen3` tags; Qwen3 model card
   (Hugging Face).*
2. **Ollama runtime parameters** — `num_ctx`, `num_predict`, `temperature`, `top_p`,
   `repeat_penalty`, `seed`, `keep_alive`; how the product passes/omits them. *Sources: Ollama API &
   Modelfile parameter docs; `crates/opencode-provider/src/bootstrap.rs:2841` ollama wiring.*
3. **Chat template & thinking switch** — Qwen3 ChatML template, `/think` `/no_think` and
   `enable_thinking`; whether the template emits the tool-call block. *Sources: Qwen3 docs/chat
   template; Ollama template handling; llama.cpp chat-template notes.*
4. **Tool-call wire format** — native structured `tool_calls` vs Hermes-style
   `<tool_call>{json}</tool_call>` in text/reasoning; Ollama tool-calling quirks. *Sources: Ollama
   tool-calling docs; `crates/opencode-provider/src/stream.rs` and `openai.rs`; `BUG-062`.*
5. **Reasoning / thinking surface** — `reasoning_content` vs `reasoning`; thinking budget; whether
   thinking competes with instruction-following. *Sources: `BUG-062` investigation; Ollama thinking
   docs.*
6. **Prompt & instruction tuning for ~30B models** — act-vs-answer phrasing, few-shot anchoring,
   system-prompt length, explicit anti-fabrication. *Sources: model cards; provider prompt guides;
   agentic system prompts (Cline/OpenHands/Claude Code class).*
7. **Agent loop & guardrails** — force-tool / `tool_choice`, no-op-after-skill guard, terminal-state
   invariant, `max_steps`. *Sources: `crates/opencode-session/src/prompt.rs`; `EPIC-001` Option B;
   comparable agent loops.*
8. **Routing / activation policy** — when to use local qwen vs DeepSeek; ScopeMux activation scope;
   whether qwen should be default for agentic work. *Sources: `invariants/providers.md`; `SCOPE-005`;
   `EPIC-001`.*
9. **Evaluation & measurement** — deterministic tool-loop fixtures plus a small agentic metric
   (tool-use success rate, fabrication rate, turns-to-terminal). *Sources: `QA-002` harness; product
   QA scripts; external tool-use benchmarks (e.g. BFCL) as reference only.*

## Breadth vs depth policy

- Cover all nine rows with a one-line finding and a source before writing any fix beyond `BUG-065`.
- Depth is allowed for exactly one row: the one that wins the bounded A/B (below).
- This is the deliberate counter to `EPIC-001`'s single-symptom-fix anti-pattern.

## Recommended approach

Two tracks, cheapest-first:

- **Track 1 — product scaffolding (in progress as `BUG-065`).** Fix the fallback prompt, the skill
  directive, and an optional loop guard. Cheap, deterministic, and benefits the default DeepSeek model
  too. Decide using the transcript fixture.
- **Track 2 — bounded qwen A/B.** With `QA-002` + the `BUG-065` transcript/mock fixture, sweep the
  cheap knobs — {temperature, `num_ctx`, thinking on/off, prompt variant} — and measure tool-loop
  success and fabrication rate over repeated runs. Only if Track 2 cannot lift reliability, evaluate a
  model/quant swap (rows 1–3).
- **Routing fallback.** If qwen cannot be made reliable for agentic work, narrow its role per
  `invariants/providers.md` (local-model path) rather than let it degrade the daily-driver default.

## Non-goals

- Deep research on any single row before the breadth pass is recorded.
- Changing ScopeMux provider behavior or retrieval (separate concern).
- Re-litigating the `BUG-060`/`BUG-061`/`BUG-062` transport fixes.

## Done when

- All nine rows carry a one-line finding plus a source.
- One bounded A/B result exists with the exact params/prompts and repeated-run outcome.
- A single recommendation is recorded: cheapest lift, or a model/quant/routing decision.
- Any concrete change is split into its own bug/feature card (do not implement inside this survey).

## Related items

- `BUG-065` — the concrete defect this survey scopes beyond.
- `EPIC-001` — owns the recurring reliability class.
- `BUG-062` — parser/encoding sibling; source of the residual.
- `QA-002` — the measurement net to extend.
- `SCOPE-005` — ScopeMux provider activation for local qwen.
- `invariants/providers.md`, `invariants/coding-session-behavior.md`.

## Notes

- Prefer naming sources by site/section over guessing deep URLs; confirm each before citing.
- Keep the "instruction-following under conflict" hypothesis falsifiable via the Track 2 A/B.
