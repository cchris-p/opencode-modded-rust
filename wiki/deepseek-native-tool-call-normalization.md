# DeepSeek Native Tool-Call Normalization

## Purpose

Record why DeepSeek's native tool-call tokens that leak into an OpenAI-compatible
assistant `content` field are normalized into real tool calls in the **shared**
OpenAI-compatible parsers, and why the normalization is keyed on the **token
signature** rather than on a provider or model id. This is the decision record
for `BUG-060`.

## Background

On the `odn` profile (`novita-ai/deepseek/deepseek-r1-0528`), the model sometimes
returns a tool call as DeepSeek's native special-token text inside `content`
instead of an OpenAI `tool_calls` array:

```
...Here's the tool call:<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>function<｜tool▁sep｜>bash
{"command": "echo captured"}
<｜tool▁call▁end｜><｜tool▁calls▁end｜>
```

with `finish_reason: "stop"` and no `tool_calls` field. Because the runtime
emitted every `content` delta as plain text, the call vanished: no `tool_call`
part, no `tool_result`, no execution.

The model is flaky rather than uniformly broken: replaying the runtime's exact
captured request against the endpoint leaked the native tokens in `content` in
roughly 3 of 5 attempts and returned a proper `tool_calls` array in the rest. The
runtime does send the tool definitions, so this is model-side protocol leakage,
not a missing-tools bug.

The tokens are DeepSeek-proprietary: `<`, U+FF5C, `tool`, U+2581, `calls`,
U+2581, `begin`, U+FF5C, `>` and analogous forms for the call/sep/end markers.
They are produced only by DeepSeek-family tokenizers.

## Decision

Normalize the token block into `ToolCallStart`/`ToolCallEnd` events in the shared
OpenAI-compatible parsers, triggered by the literal token signature, and strip
the raw tokens from the visible text. Both parser layers are covered:

- the generic legacy parser (`crates/opencode-provider/src/openai.rs`,
  `parse_legacy_sse_data`) used by `novita-ai`, `ollama`, `opencode`, and any
  `@ai-sdk/openai-compatible` provider;
- the stateful parser (`crates/opencode-provider/src/stream.rs`,
  `parse_openai_sse_stateful` / `openai_compat_sse_stream`) used by `deepseek`,
  `openrouter`, `xai`, `groq`, `mistral`, `together`, `cohere`, `cerebras`,
  `deepinfra`.

The extractor buffers across chunk/delta boundaries so a token split between
deltas is not mis-emitted as text.

## Why not provider/model-id gating

- **Brittleness.** The same model appears as `deepseek/deepseek-r1-0528`,
  `novita-ai/deepseek/…`, and `openrouter/deepseek/…`. Gating on provider id
  would miss cross-provider leaks; ids and hosting routes churn.
- **Redundancy.** A model that emits DeepSeek's tokens *is* DeepSeek-family
  (including `deepseek-r1-distill-*`, which share the tokenizer). A
  `"deepseek"`-id allowlist and the token gate select nearly the same set; the
  token gate is simply more robust to naming.
- **Correctness.** A hardcoded id allowlist silently re-breaks if a provider
  renames a route or a new hosting path appears. The signature does not care.
- **No added safety.** The main downside of signature detection (see below) is
  not mitigated by id gating, because the false-positive scenario exists within
  the family too.

The signature makes the normalization inherently model-specific in effect: for
every non-DeepSeek model the extractor is unreachable and does nothing, so the
blast radius outside the DeepSeek family is zero. "This is not prevalent in
other models" is the expected consequence, not a gap - other models cannot emit
the format.

## Alternatives considered

- **Provider/model-id allowlist.** Rejected: brittle, redundant, no safety gain.
- **Per-model behavior config.** Considered as a longer-term option; adds
  config surface for a single wire signal that already self-identifies.
- **Surface the raw tokens (status quo).** Rejected: the product silently drops
  a model-requested tool call, which is the original defect.
- **Model capability metadata (`FEAT-013`).** Orthogonal. The catalog marks the
  model `tool_call: true`, and it does call tools - just sometimes in its native
  encoding. Capability flags cannot express "returns OpenAI tool_calls most of
  the time".

## Risks and guardrails

- **False positives.** If a model is asked to *demonstrate* the token format, or
  echoes a prompt containing it, the runtime could execute a bogus tool call.
  Model-id gating does not remove this, since the risk lives inside the family.
- **Reasoning leakage.** The confirmed, reproducible path is native tokens in
  `content`. Token leakage via `reasoning_content` was not reproduced; if it is
  observed, the extractor would need a reasoning-aware variant.
- **Mitigation options.** A lightweight guard (only normalize when the turn
  produced no structured `tool_calls`, or a config/feature flag to disable the
  behavior) can reduce the false-positive surface. Tracked as `BUG-061`.

## Normative status

The behavior is an explicit, documented DeepSeek-family compatibility
normalization, not hidden magic. The binding rule lives in
`invariants/providers.md`; the implementation is `BUG-060`
(PR #138). Do not replace the signature check with a provider/model-id
allowlist.

## Related

- `BUG-060` - the defect and fix.
- `BUG-006` - the prior DeepSeek tool-loop fix (OpenAI-style split tool calls
  and reasoning passback), same class on a different wire signal.
- `FEAT-013` - model capability gating.
- `invariants/providers.md` - the normative rule.
