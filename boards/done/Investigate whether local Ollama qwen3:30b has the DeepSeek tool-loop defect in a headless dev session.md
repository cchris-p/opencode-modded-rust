---
id: "BUG-062"
title: "Investigate whether local Ollama qwen3:30b has the DeepSeek tool-loop defect in a headless dev session"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-10-12"
---

# Investigate whether local Ollama qwen3:30b has the DeepSeek tool-loop defect in a headless dev session

## Summary

The DeepSeek tool loop was fixed in `BUG-006` (split tool-call delta joined into one
`toolCall`; `reasoning_content` captured and echoed back so the follow-up request is
accepted). The concern is that the local Qwen path (`ollama/qwen3:30b`, the ScopeMux
activation model) may still have the same class of defect, because `ollama` is routed
through `OpenAIProvider::openai_compatible` — the generic legacy OpenAI-compatible
transport — rather than one of the providers explicitly re-routed through the stateful
parser in `BUG-006`.

This card is an **investigation** first: determine whether qwen3:30b on local Ollama
exhibits the DeepSeek tool-loop symptoms (a tool call that is rendered/streamed but not
executed, a split/empty-name tool call, a dropped-reasoning follow-up failure, or a tool
call that leaks as text). Only if it reproduces do we implement the matching fix.

## Why we suspect it

- `ollama` provider construction: `crates/opencode-provider/src/bootstrap.rs:2841`
  calls `OpenAIProvider::openai_compatible(base_url, api_key)`.
- That path streams through the legacy parser
  `OpenAIProvider::parse_legacy_sse_data` (`crates/opencode-provider/src/openai.rs:318`)
  — a different parser from `parse_openai_sse_stateful` / `openai_compat_sse_stream`
  (`crates/opencode-provider/src/stream.rs`) that `BUG-006` hardened for the named
  providers (`deepseek`, `openrouter`, `xai`, `together`, `perplexity`, `mistral`,
  `groq`, `cohere`, `cerebra`, `deepinfra`). `ollama` was not in that list.
- The request side already shares `openai_chat::openai_chat_completions_body`, so the
  `reasoning_content` echo added by `BUG-006` (`crates/opencode-provider/src/openai_chat.rs`)
  may or may not reach the Ollama wire path — confirm, do not assume.
- Prior art: the archived session notes (`docs/archive/session.md:345`) cite an upstream
  report of `qwen3-coder:30b` via Ollama producing "valid OpenAI-style tool calls that
  OpenCode Desktop displayed rather than executed" — the same user-visible symptom.
- Related in-flight sibling: `BUG-060` is the novita/DeepSeek native-token variant of the
  same "tool call not executed" class; this card is the qwen/Ollama-Local variant.

## Hypotheses to confirm or refute

1. **Split / empty-name tool call.** `ollama` streaming tool-call deltas arrive with the
   `id`/`name` split from the argument fragments (or name only on the first frame), and
   the legacy parser mis-assembles them into a stray empty-name `tool-call-N`
   (`Tool '' not found in registry`) — the `BUG-006` symptom, on the Ollama path.
2. **Reasoning passback.** qwen3 thinking emits `reasoning_content`/`thinking` deltas; if
   the follow-up request does not echo the prior assistant reasoning, Ollama/its
   OpenAI-compatible shim rejects or truncates the tool-loop continuation.
3. **Tool call leaks as text.** qwen emits the call in a non-`tool_calls` shape (native
   tokens or fenced JSON) that the legacy parser passes through as `TextDelta`, so no tool
   runs and the run ends with `finish_reason: stop` (the `BUG-060` symptom, on qwen).
4. **None of the above** — the legacy parser already retains per-index ids/names and the
   shared request body already echoes reasoning, so qwen works; the card then closes as
   verified-good with a regression test added.

## Investigation method — headless CLI, simulated dev session

Do **not** investigate through the interactive TUI (`scopemux-self-qa` policy: drive the
real path headlessly and capture a deterministic signal).

- **Target:** local server built from the current `development` checkout, model
  `ollama/qwen3:30b` (`invariants/providers.md` activation model), against a fixture
  workspace with a small known file tree.
- **Attach:** `scripts/scopemux-qa-server.sh` (or `opencode serve` on a chosen port),
  with `RUST_LOG=info` and the QA workspace pinned.
- **Simulate a working dev session:** enqueue a multi-step agentic prompt that forces a
  real read → edit → run loop, e.g. an `@file`-seeded prompt like "read `sample.rs`, run
  its tests with the `bash` tool, then apply a small change and re-run", via
  `/session/<id>/prompt_async`. This is a realistic dev loop, not a one-shot echo.
- **Deterministic signals to capture:**
  - Presence/absence of `tool_call` + `tool_result` parts for each step (from the session
    store or exported transcript).
  - Any stray empty-name tool call / `Tool '' not found` server log line.
  - Any `400 reasoning_content` (or equivalent) on the follow-up request.
  - Any raw tool-token or fenced-JSON leak appearing as assistant `content`.
  - Final `finish_reason` and whether the run reaches a terminal answer.
- **Control:** repeat the identical prompt/session shape on `deepseek/deepseek-flash`
  (known-good after `BUG-006`) to prove the harness, permissions, and fixture are sound.
- **Repeat** the qwen run several times — the sibling `BUG-060` case was flaky (2/5 vs
  3/5), so a single green run is not sufficient.
- **Detach** (abort the turn, stop the server) and leave the checkout as found.

## Done when (investigation)

- A recorded evidence block states PASS (qwen executes the full tool loop correctly) or
  FAIL (which of hypotheses 1–3 reproduces), with commit, binary path, server URL, the
  exact prompt(s), the captured signals, and log/store paths.
- The deepseek control run is recorded alongside it.
- If FAIL, a concrete, minimal follow-up fix plan is written on this card (files,
  symptom-to-code mapping) and the card is promoted to implementation; if PASS, the
  evidence is recorded and the card closes as verified-good, with any cheap regression
  test noted.

## Scope

- Investigation only for now: `scripts/` harness runs, server logs, session store reads.
- If a fix is needed, the likely surface is
  `crates/opencode-provider/src/openai.rs` (`parse_legacy_sse_data`) and/or
  `crates/opencode-provider/src/bootstrap.rs` (`ollama` provider wiring), reusing the
  `BUG-006`/`BUG-060` normalizers rather than adding a parallel parser.

## Non-goals

- Reworking provider architecture or consolidating the v1/v2 prompt loop.
- Changing qwen model catalog metadata.
- Fixing unrelated ScopeMux retrieval behavior on qwen (separate concern).

## Related Items

- `BUG-006` DeepSeek tool loop fails: reasoning passback + split tool-call delta (done) —
  the fix whose applicability to qwen this card tests.
- `BUG-060` Novita DeepSeek R1 leaks native tool-call tokens; tools not executed (doing) —
  same "tool not executed" class, different provider/wire signal.
- `BUG-005` OpenAI-compatible chat providers reject requests once tools are attached
  (done) — the wire-format prerequisite.
- `QA-002` Deterministic DeepSeek stream/loop fixture harness (qa) — the regression net to
  extend if a defect is found.
- `SCOPE-005` / `activate-scopemux-provider-for-local-qwen` (done) — why the qwen/Ollama
  path matters; the ScopeMux provider only activates on `ollama` + `qwen*`.
- `invariants/coding-session-behavior.md` — the tool-execution and terminal-state rules.

## Notes

- Created from an operator report that the DeepSeek tool-loop fix may not cover qwen.
- Routing facts above are from a read of `development` at creation time; re-verify line
  numbers before editing.
- Coordinate with the `BUG-060` (novita) work if both land on the same shared
  content-to-tool-call extractor.

## Investigation - 2026-10-12 (reproduced, FAIL)

Headless server (isolated `HOME`/DB), model `ollama/qwen3:30b`, workspace
`~/worktrees/bug062-qwen-dev`, driven with a multi-step read→write→bash dev
prompt. Commit under test: `8caf076` (`development`), binary
`target/debug/opencode`. A logging proxy captured the exact product request and
the raw Ollama SSE.

Findings:

- The product attaches the tools correctly: the captured request had `stream: true`,
  **19** well-formed `type: "function"` tools (`read`, `write`, `bash`, ...), and
  `[system, user]` messages.
- On the failing run the model **did not emit a structured `tool_calls` array**.
  Instead it narrated the calls inside Ollama's `reasoning` field as
  `<tool_call>{"name":...,"arguments":{...}}</tool_call>` blocks (e.g. `read`, `write`,
  `bash`), with `finish_reason: "stop"`. Raw captured blocks:

  ```
  <tool_call>{"name": "read", "arguments": {"file_path": ".../hello.txt"}}</tool_call>
  <tool_call>{"name": "write", "arguments": {"file_path": ".../result.txt", "content": "Hello World\nDONE"}}</tool_call>
  <tool_call>{"name": "bash", "arguments": {"command": "wc -l result.txt"}}</tool_call>
  ```

- The legacy OpenAI-compatible parser (used by `ollama` via
  `OpenAIProvider::openai_compatible`) ignored both the `reasoning` field (it only read
  `reasoning_content`/`reasoning_text`) and any inline `<tool_call>` markup. So nothing
  became a `toolCall`: **no tool ran, `result.txt` was never created**, and the run ended
  with the model's hallucinated answer `3 result.txt` (`finish: stop`).
- Model-side flakiness was confirmed BUG-060-style: replaying the exact captured request
  directly against Ollama 11/11 times returned structured `tool_calls`
  (`finish_reason: "tool_calls"`), and 4/4 fresh product runs also executed the loop
  correctly. The failure is an intermittent generation that the product must not silently
  drop.

This confirms the card's hypothesis 3 (tool call leaks as text/reasoning, tools not
executed) plus a second, distinct gap: Ollama's `reasoning` field was never parsed at all.

## Implementation - 2026-10-12

Branch `bug/BUG-062-qwen-tool-loop` (worktree), on top of `development` after the
`BUG-060`/`BUG-061` DeepSeek native normalization landed (`1c46925`).

The first push conflicted with the just-landed `BUG-060`/`BUG-061` DeepSeek
normalization in the same shared parsers; this is the reconciled implementation. It
extends the same "native tool-call normalization" feature rather than adding a parallel
one.

- `crates/opencode-provider/src/stream.rs`:
  - New `NativeToolCallBuffer` for Qwen/Hermes `<tool_call>{json}</tool_call>` markup,
    with cross-fragment buffering so a split open tag / split JSON joins into one call.
  - New `QwenNativeNormalizer` wrapping two buffers (content + reasoning): extracts markup,
    **defers** synthesized calls to end-of-stream, and drops them when a structured
    `tool_calls` array was already seen this turn — the same `BUG-061` guard the DeepSeek
    extractor uses. Gated by the shared `native_normalization_enabled()` kill switch.
  - The DeepSeek extractor's pass-through text is routed through the Qwen content buffer
    (`finish_native_stream`), so both encodings coexist in one stream without double text.
- `crates/opencode-provider/src/openai.rs`:
  - `LegacySseParserState` gains the same `QwenNativeNormalizer`; `parse_legacy_sse_data`
    (the `ollama` path) runs the reasoning + content channels through it and chains the
    DeepSeek pass-through text through it (`finish_legacy_native`).
- Both parsers now also read Ollama's `reasoning` field (alongside
  `reasoning_content`/`reasoning_text`); that part is independent of the normalization
  kill switch.
- Tests: buffer extract/strip, split open tag, reasoning-field native call → structured
  `ToolCallStart`/`ToolCallEnd`, content-channel native call (no markup leak), discard when
  a structured call is present, and disabled-kill-switch pass-through.

No session-layer change was needed: the loop already executes any `ToolCall` parts and
continues regardless of `finish_reason` (`crates/opencode-session/src/prompt.rs:1537`).

### Verification

- `cargo test -p opencode-provider`: 124 lib + 9 fixture + 7 integration pass;
  `cargo fmt --all` clean (DeepSeek `BUG-060`/`BUG-061` fixture + guard tests still green).
- Headless end-to-end via a deterministic mock that replays the captured failure shape
  (native `<tool_call>` in the `reasoning` field, split across frames) and then a plain
  final answer. Same workspace/model/config, fixed binary
  `$OPENCODE_WORKTREE_ROOT/opencode-modded-rust/.shared-target/debug/opencode`:
  - **Fixed (reconciled)**: parts `1xreasoning, 2xtext, 3xtool_call, 3xtool_result`;
    tools executed `read, write, bash`; `result.txt` created.
  - **Contrast (pre-fix `development` binary, same mock)**: parts `1xtext`; no tool calls;
    `result.txt` absent — the silent-drop repro.

### Follow-ups / related

- The `BUG-060` wiki/invariant scope was written for the DeepSeek signature and notes that
  reasoning-channel leakage "was not reproduced"; `BUG-062` is that reasoning-aware variant
  and the invariant/wiki should be widened to cover the Qwen signature.
- Consider a `QA-002` fixture for the qwen `<tool_call>`-in-`reasoning` case using the
  captured bytes.

### PR

- https://github.com/cchris-p/opencode-modded-rust/pull/140

## QA Report - 2026-10-12 (post-merge, agent)

Merged into `development` as `683d730` (PR #140). The merge tree is identical to the
tested branch commit `2db39af`, and the binary
`$OPENCODE_WORKTREE_ROOT/opencode-modded-rust/.shared-target/debug/opencode` was built
from that content. Headless QA on `development`.

**Deterministic native-path check (the defect).** Mock replaying the captured failure
shape (native `<tool_call>` in `reasoning`, split across frames) then a plain final
answer:

- PASS: `1xreasoning, 2xtext, 3xtool_call, 3xtool_result`; `read/write/bash` executed;
  `result.txt` created. Pre-fix `development` on the same mock gave `1xtext` and no
  `result.txt`.

**Real `ollama/qwen3:30b` dev session (multi-step read→write→bash), captured via logging
proxy, 3 runs:**

- 2/3 PASS: full structured loop, `3xtool_call` + `3xtool_result`, `result.txt` created.
- 1/3 distinct residual (out of scope): the model emitted **no tool call at all** — no
  `tool_calls` array and no native `<tool_call>` markup in `content` or `reasoning`
  (`finish_reason: stop`), just a hallucinated `result.txt` answer. This is model
  behavior, not a parser/encoding gap; the parser cannot synthesize a call the model
  never emitted.

**Regression:** `cargo test -p opencode-provider` green on the merged content
(124 lib + 9 DeepSeek fixture + 7 integration), including the `BUG-060`/`BUG-061`
normalization + kill-switch tests.

**Residual follow-up (not this card):** qwen3:30b intermittently answers without
calling tools even with tools attached. That reliability gap belongs with the
`EPIC-001` default-model reliability work, not the native tool-call normalization.

Card moved to `done`: the parser defect is fixed and verified; the residual is a
separate model-reliability concern.
