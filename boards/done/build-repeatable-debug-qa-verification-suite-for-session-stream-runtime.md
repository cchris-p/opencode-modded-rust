---
id: "QA-001"
title: "Build a repeatable debug/QA verification suite for the session/stream runtime"
priority: "P1"
type: "feature"
area: "QA"
spec: "boards/qa/bug-session-stops-completely-after-first-prompt.md"
status: "done"
created: "2026-09-09"
---

# Build a repeatable debug/QA verification suite for the session/stream runtime

## Summary

Build a repeatable, low-friction verification suite that lets QA and development catch the two failure classes behind `BUG-003` (streamed model output getting garbled/truncated, and the session loop silently stopping after the first prompt) without needing a live TUI session or manually inspecting garble. The suite must run deterministically in CI/tests and include an env-gated live smoke path for real-provider checks.

## Why this exists

`BUG-003` was only caught through live interactive testing: a user opened a session, saw a garbled first reply or a silent stall on a later prompt, and there was no automated regression net to prove a fix worked. Two distinct root causes were fixed (lossy per-chunk SSE parsing in `stream::sse_event_stream`, and the prompt-loop break guard comparing unordered message IDs in `prompt.rs`), but without tests the same classes can silently regress. This item isolates the verification suite deliverable so debug/QA tooling is tracked separately from the bug fixes themselves.

## Scope

- A deterministic SSE stream-integrity test layer over the buffered parser (`sse_event_stream` and the per-provider line parsers) that feeds adversarial byte layouts: multiple SSE frames coalesced into one network chunk, a single frame split across chunks, CRLF endings, `data: [DONE]` with no trailing newline, keepalive/comment lines, and multi-byte UTF-8 split at a chunk boundary. Assert exact full-text reassembly and that `Done` is delivered.
- A deterministic multi-turn session-loop integration test that drives two or three consecutive prompts through the real prompt/session runtime with a scripted provider and asserts every turn completes, full text is preserved, and the run returns to an idle state after each turn.
- A live smoke script (env-gated on `DEEPSEEK_API_KEY`) that runs a real two-to-three-turn deepseek session against the freshly built binary and asserts non-empty, non-truncated assistant replies on every turn.
- Wire the tests so `cargo test` for `opencode-provider` and `opencode-session` covers the deterministic layers, and the live smoke is runnable via a documented script during QA on the dev branch.

## Non-goals

- Replacing manual TUI QA entirely (the live TUI path still needs human confirmation per `BUG-003`).
- Broad multi-provider live test matrix beyond the working deepseek key; other provider paths stay unit-level/deterministic.
- Production CI infrastructure changes; the suite runs locally and under existing cargo test.
- Refactoring the provider/session architecture beyond what the tests need.

## Done when

- `cargo test -p opencode-provider` includes the adversarial SSE integrity tests and they pass.
- `cargo test -p opencode-session` includes a 3-turn loop test that passes and would fail if either the SSE parser regressed or the loop stopped after turn one.
- A `scripts/qa/stream-smoke.sh` live script exists, is env-gated, and a real deepseek 3-turn run completes with full replies.
- The suite is documented in the `BUG-003` card as the standard regression check.

## Dev Notes

- Initial implementation adds regression tests for the two confirmed root causes and a live smoke script, committing directly to `development` per the current workflow (no PR).
- The deterministic tests must not require network or provider keys.

### Implementation - 2026-09-09

- Delivered and committed directly to `development` as part of commit `bce4900`.
- SSE integrity tests added in `crates/opencode-provider/src/stream.rs` (test module):
  - CRLF line endings, keepalive/comment lines, multi-byte UTF-8 split at a chunk boundary, exact long-stream reassembly over tiny 7-byte chunks, `[DONE]` delivered exactly once with trailing partial data, and non-data-line behavior of `anthropic_line_events` / `openai_compat_line_events`.
  - Result: 12 `stream::tests` pass; full `cargo test -p opencode-provider` (81 lib + 7 integration) passes.
- Deterministic multi-turn session-loop test added in `crates/opencode-session/src/prompt.rs` (`session_handles_three_consecutive_prompts`) using a `SequencedStreamProvider` that returns one canned reply per `chat_stream` call.
  - Verified the test fails under the old ID-comparison guard ("after prompt 2 there should be 2 assistant reply(ies)") and passes under the positional guard.
  - Full `cargo test -p opencode-session` (144 lib + 11) passes.
- Live smoke script `scripts/qa/stream-smoke.sh` (env-gated on `DEEPSEEK_API_KEY`): launches a fresh detached server, drives a 3-turn deepseek session over the HTTP API, asserts each turn completes with non-empty assistant text. Skip path returns 0 when no key.
  - Verified live: `stream-smoke: PASSED (3 turns completed on deepseek/deepseek-v4-flash)`.

## Verification

- `cargo test -p opencode-provider`: 81 + 7 pass.
- `cargo test -p opencode-session`: 144 + 11 pass (includes the 3-turn regression).
- Live smoke on real deepseek: PASSED (3 turns), no network in deterministic layers.
- Standard regression check for both BUG-003 failure classes: run the two cargo suites, then `DEEPSEEK_API_KEY=<key> scripts/qa/stream-smoke.sh` when a key is available.

## Related Items

- `BUG-003` Session stops completely after first prompt (source bug these tests guard)
- `START-005` Define V1 runtime loop
- `START-017` Enforce verification and review stages in runtime

## Notes

- Keep the deterministic tests independent of provider credentials so they run in plain `cargo test`.
- Guard any live network test behind an env var and skip cleanly when the key is absent.

## QA Report - 2026-09-09 (user)

- User confirmed BUG-003 fix works on the fresh server: "Great that works!" after multi-turn deepseek reproduction no longer stops after the first prompt.
- Suite is green and used as the regression net for that confirmation: `cargo test -p opencode-provider` (81+7), `cargo test -p opencode-session` (144+11 incl. 3-turn regression), and `scripts/qa/stream-smoke.sh` (live deepseek 3-turn, PASSED).
- Closing as completed.