---
id: "BUG-061"
title: "Bound false positives for DeepSeek native tool-call normalization"
priority: "P3"
type: "bug"
area: "BUG"
spec: "invariants/providers.md"
status: "todo"
created: "2026-10-09"
---

# Bound false positives for DeepSeek native tool-call normalization

## Summary

`BUG-060` normalizes DeepSeek native tool-call tokens (`<｜tool▁calls▁begin｜>...<｜tool▁calls▁end｜>`)
that leak into OpenAI-compatible assistant `content` into real tool calls. The trigger is the token
signature, which is the right call (`wiki/deepseek-native-tool-call-normalization.md`). The residual
risk is a false positive: if a model is asked to *demonstrate* the token format, or echoes a prompt
containing it, the runtime could execute an unintended tool call. This card tracks a bounded guard so
that risk is explicit and capped.

## Why this exists

- The normalization is intentionally signature-based and stays inert for non-DeepSeek models, but it
  cannot distinguish "the model is calling a tool" from "the model is quoting the format".
- Model-id gating does not remove the risk (it exists inside the DeepSeek family), so it is not the
  answer here.
- Current behavior is acceptable; this is hardening, not a defect in the shipped fix.

## Options (pick during refinement)

- **Guard:** only apply native-token extraction for a turn that produced **no** structured
  `tool_calls` array. Requires deferring extraction/flush to end-of-stream once the structured
  channel is known absent, which is a small refactor of the two parsers.
- **Flag:** a config/feature switch to disable native normalization entirely.
- **Both:** guard by default, with a kill switch.

## Non-goals

- Re-litigating signature-based vs provider/model-id gating (decided in `BUG-060`).
- Handling native tokens in `reasoning_content` (not reproduced; separate follow-up if observed).

## Done when

- A chosen guard/flag is implemented with tests, and the false-positive scenario (model quoting the
  token format) does not execute a tool under the default configuration.
- `invariants/providers.md` and the wiki note are updated to state the guard.
- `cargo test -p opencode-provider` passes and the live `odn` marker check still passes.

## Related Items

- `BUG-060` DeepSeek native tool-call tokens normalized (qa) - the fix this hardens.
- `BUG-006` DeepSeek tool loop (done) - same class.
- `FEAT-013` Model capability gating (todo).
- `wiki/deepseek-native-tool-call-normalization.md` - decision record.
