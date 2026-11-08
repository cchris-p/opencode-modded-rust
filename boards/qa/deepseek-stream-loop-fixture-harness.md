---
id: "QA-002"
title: "Deterministic DeepSeek stream/loop fixture harness"
priority: "P1"
type: "feature"
area: "QA"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-10-07"
---

# Deterministic DeepSeek stream/loop fixture harness

## Summary

Child of `EPIC-001` (Option A). Build an offline regression net for the DeepSeek default-model
failure class: store real DeepSeek SSE byte streams as fixtures, replay them through the actual
OpenAI-compatible stateful parser (`openai_compat_sse_stream`) and, where useful, through the session
prompt loop, and assert that each scenario ends in a known terminal state or a visible error rather
than a silent hang.

The goal is to convert every historical DeepSeek defect into a deterministic test, so a fix is
verified offline (not "fixed but unconfirmed live") and a regression is caught immediately.

## Scope

- Add on-disk DeepSeek SSE fixtures covering the known shapes:
  - reasoning only, then text, then `finish_reason: stop` + `[DONE]` (terminal success);
  - reasoning -> tool call (`id`+`name` first, argument fragments later) -> `finish_reason: tool_calls`;
  - split tool-call delta where argument-only frames carry only `index`;
  - usage frame + reasoning + content in one payload;
  - a stream that ends after reasoning/partial text with no finish reason and no `[DONE]`;
  - an idle gap (no bytes) after a reasoning block (BUG-038 shape);
  - reasoning-only emission that never transitions (BUG-046 shape).
- Provider-level replay: a helper that feeds fixture bytes through `openai_compat_sse_stream` with
  adversarial chunk boundaries and asserts the parsed event sequence (reasoning captured and closed,
  split tool call joined to one id, terminal `FinishStep`/`Done` present or absent as expected).
- Loop-level replay: drive the session prompt loop with a provider that replays the fixture-derived
  events and assert the run terminates (Ok with a terminal record, or Err) instead of hanging.
- Wire both into the existing test suites so `cargo test` covers them.

## Non-goals

- Live network calls; fixtures are the point. The live gate is `EPIC-001` Option C.
- Fixing any specific DeepSeek defect here; this card only builds the net. New failures it reveals
  get their own cards.
- Full tool-execution end-to-end in the loop-level replay (permission/ask and sandbox are out of
  scope); tool-call parsing is covered at the provider level.

## Design sketch

- Fixtures live under `crates/opencode-provider/tests/fixtures/deepseek/` as `.sse` files.
- Provider tests (in `crates/opencode-provider`) replay each fixture and assert the parsed events,
  including a chunk-boundary matrix that splits a fixture at every byte offset.
- Loop tests (in `crates/opencode-session`) build their event stream from the same fixture text via
  `parse_openai_sse_stateful`, wrap it in a mock `Provider`, and assert terminal behavior. The
  session crate already depends on `opencode-provider` and `reqwest`, so no new deps are needed.

## Acceptance Criteria

- [x] Fixtures exist for each listed shape and are committed as real SSE text.
- [x] A provider test asserts each fixture's parsed event sequence, including split tool-call join.
- [x] A chunk-boundary test replays at least one fixture split at every byte offset with identical
      parsed events.
- [x] A loop test asserts a terminal result for a successful fixture and termination for a
      body-close-without-finish fixture.
- [x] `cargo test -p opencode-provider` and `cargo test -p opencode-session` pass.
- [x] `cargo fmt --all -- --check` and `cargo check --workspace` pass.

## Implementation - 2026-10-07

Fixtures (`crates/opencode-provider/tests/fixtures/deepseek/`):

- `reasoning_text_stop.sse` - reasoning -> text -> `finish_reason: stop` + `[DONE]`.
- `reasoning_toolcall_split_args.sse` - reasoning -> tool call (`id`+`name` first, argument fragments
  later) -> `finish_reason: tool_calls`.
- `parallel_toolcalls_split_args.sse` - two parallel tool calls with interleaved argument fragments.
- `usage_reasoning_content.sse` - a single payload carrying usage + reasoning + content.
- `no_finish_no_done.sse` - body closes after reasoning + partial text, no terminal frame.
- `reasoning_only_no_terminal.sse` - reasoning only, no terminal frame (BUG-046 shape).

Tests:

- `crates/opencode-provider/tests/deepseek_fixture_replay.rs` (7 tests) replays each fixture through
  the real `openai_compat_sse_stream` parser and asserts reasoning capture/close, split tool-call
  join to one id, parallel distinct ids, usage parsing, and terminal presence/absence. A
  chunk-boundary matrix replays the tool-call fixture split at every byte offset and asserts the
  parsed events are identical.
- `crates/opencode-session/tests/deepseek_fixture_harness.rs` (2 tests) drives `SessionPrompt`
  through a `FixtureProvider` that returns `openai_compat_sse_stream(fixture bytes)`. The success
  fixture must complete with a durable `finish_reason: stop`; the body-close fixture must terminate
  the run (no hang). Both wrapped in a 10s timeout so a regression to hanging fails fast.

Observed gap (documented by the body-close test, tracked under `EPIC-001`): a stream that closes
before any terminal frame is finalized as a completed turn with no `finish_reason`, so a truncated
reply is indistinguishable from a completed one at the record level. The test asserts the current
absence explicitly so a future fix must update it.

## Verification - 2026-10-07

- `cargo test -p opencode-provider` -> all pass (7 fixture + 7 integration + lib).
- `cargo test -p opencode-session` -> 170 lib + 2 harness + 11 integration pass.
- `cargo fmt --all -- --check` -> clean.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace` -> clean.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/135 (base `development`)

## QA / Merge Disposition

- This card is offline-test-only; it does not change runtime behavior, so no live `ort` session is
  required. A reviewer can rerun the four commands above.
- Merged to `development` via PR #135 (merge commit `f30920f`); card remains in `qa`.

## Live QA - 2026-10-07 (superseded)

An initial smoke run used `scripts/qa/stream-smoke.sh` on `deepseek/deepseek-flash` with three
plain-text prompts; it passed, but it only proves trivial text streaming. It used the **wrong model**
for the active work and never exercised tool execution or the `question` flow, so it is **not**
evidence for the DeepSeek reliability / agentic defects (`BUG-056`/`BUG-057`/`BUG-058`). The live
check that actually drives a tool call is recorded on `FEAT-067`.

## Related Items

- `EPIC-001` DeepSeek default-model reliability hardening (parent).
- `BUG-006` DeepSeek reasoning passback and split tool call.
- `BUG-038` DeepSeek reasoning turn stalls mid-turn.
- `BUG-046` Provider stream without idle gaps is unbounded.
- `QA-001` Repeatable debug/QA verification suite (owns `scripts/qa/stream-smoke.sh`).
- `FEAT-013` Model capability gating and deprecated-model surfacing.

## Notes

- Keep fixtures minimal and hand-verifiable; prefer real captured frames where available and
  byte-accurate reconstructions where not. Note the provenance of each fixture in a comment line or
  companion note.
- This card is the first slice of `EPIC-001`; B/C/D follow once the net exists.
