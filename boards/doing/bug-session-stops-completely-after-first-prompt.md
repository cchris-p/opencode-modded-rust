---
id: "BUG-003"
title: "BUG: Session stops completely after first prompt"
priority: "P1"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "doing"
created: "2026-09-01"
---

# BUG: Session stops completely after first prompt

## Summary

The current product still has a session-blocking bug where a session completely stops after the first prompt instead of continuing through a normal request-response loop. Treat this as a primary workflow blocker until the exact failure point is identified and the same session flow is proven stable across Codex/OpenAI, Ollama, and Anthropic.

## Reported behavior

- The issue is currently reported as always reproducible.
- The session stops entirely after the first prompt.
- The bug has been observed on Codex/OpenAI, Ollama, and Anthropic.
- The current report does not yet pin down whether the backend session actually stops or whether only the TUI stops updating, so investigation must distinguish those cases explicitly.

## Why this exists

The product is currently being shaped as a personal daily-driver on a narrow workflow. A session that halts after one prompt breaks the core loop regardless of provider, model quality, or later UX polish. This bug therefore blocks confidence in the runtime as a serious everyday tool.

## Investigation goals

- Reproduce the failure on the current Rust product with a minimal prompt.
- Determine whether the stop happens in provider execution, runtime stage progression, server event streaming, session persistence, or only the TUI refresh path.
- Determine whether the bug is truly provider-agnostic or whether the same symptom has different causes on Codex/OpenAI, Ollama, and Anthropic.
- Identify whether the first prompt fully completes and the second prompt fails, or whether the very first prompt itself stalls before normal completion.
- Capture the exact user-visible symptom, any logs, session state transitions, and whether the server remains healthy after the stop.

## Scope

- Investigate the active session loop end to end: TUI, client API layer, server routes, session runtime, and provider execution boundaries.
- Reproduce the issue against Codex/OpenAI, Ollama, and Anthropic using the same or equivalent simple prompt where practical.
- Verify whether the bug affects only the reused local TUI server path or also a freshly started server.
- Verify whether the bug depends on a specific model, auth path, provider configuration source, or question/approval flow.
- Keep the first pass focused on root cause and the smallest correct fix.
- Do not broaden this item into general provider setup cleanup unless investigation proves the bug is caused by a provider configuration defect already tracked elsewhere.

## Failure boundaries to check

- Does the first prompt receive a complete assistant response, then later prompts fail?
- Does the first prompt itself stop before completion?
- Does the TUI stop accepting input, or does it accept input while no new work is executed?
- Does the server still emit session updates after the visible stop?
- Does the stored session status change to a blocked, error, or completed state unexpectedly?
- Does the issue reproduce only when `ort` reuses an existing local TUI server?
- Does the same session recover after restarting the app or reopening the session?

## Done when

- The root cause is identified clearly enough to explain why the session stops after the first prompt.
- The implemented fix addresses the actual failure point rather than masking the symptom.
- A session can handle repeated prompts without halting unexpectedly.
- The fix is verified locally against Codex/OpenAI, Ollama, and Anthropic.
- Verification explicitly confirms that the issue is no longer present for each provider path, not just for one successful provider.
- Any provider-specific residual gaps discovered during investigation are split into separate follow-up board items instead of being hidden inside this bug.

## Required verification

- Reproduce the bug before the fix with a minimal prompt and capture the exact visible failure.
- Test a fresh-launch path and, if relevant, a reused-local-server path.
- After the fix, verify at least one stable multi-prompt session with Codex/OpenAI.
- After the fix, verify at least one stable multi-prompt session with Ollama.
- After the fix, verify at least one stable multi-prompt session with Anthropic.
- For each provider above, explicitly have the user confirm that the session no longer stops after the first prompt.
- Record any provider/model-specific caveats discovered during verification.

## Suggested verification script

- Start a new session.
- Send a minimal first prompt such as `reply with exactly OK`.
- Wait for completion and confirm the session remains healthy.
- Send a second prompt such as `reply with exactly STILL OK`.
- Send a third prompt such as `summarize the last two answers in one line`.
- Repeat the same pattern for Codex/OpenAI, Ollama, and Anthropic.
- Note whether failure occurs on the first prompt, after first completion, or only on subsequent prompts.

## User validation requirement

Once fixed, have the user test all three provider paths personally:

- Codex/OpenAI
- Ollama
- Anthropic

The bug should not be considered closed until the user confirms the session no longer stops after the first prompt on all requested provider paths, or any remaining provider-specific failures are broken out into separate tracked bugs.

## Questions for the user

- Keep this section in the card even if the bug is not investigated immediately. These answers should be captured before implementation starts if they are still unknown.

- What are the exact steps you use when this happens?
- Does the first prompt fully finish, or does it stall mid-response?
- What exact text, status message, spinner state, or error do you see when the stop happens?
- Does the TUI freeze, or can you still interact with it after the session stops?
- If you reopen the same session, does it remain stuck or resume?
- Which exact model did you use for Codex/OpenAI when you saw it?
- Which exact Ollama model and endpoint did you use when you saw it?
- Which exact Anthropic model did you use when you saw it?
- Does the issue happen both on a fresh `ort` launch and when `ort` reuses an existing local TUI server?
- Did this start after a specific recent change, or has it been present the whole time you've been testing this path?

## Related Items

- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop
- `START-015` Mirror OpenAI auth configuration in settings
- `START-018` Complete TUI approval and question handling
- `START-019` Add native Ollama support for the local-model-first V1 path
- `FEAT-002` Keep sessions running after TUI exit

## Notes

- Treat this as a real runtime blocker until disproven, not as a minor UX glitch.
- Investigation should prefer evidence from real runs, session state, and logs over speculative fixes.
- If the bug turns out to be a reused-server state issue, document that explicitly and verify both reused and fresh-launch behavior after the fix.

## Investigation - 2026-09-08 20:18 EDT

- User reported the current Rust product is unusable.
- Matched this report to `BUG-003` because the active card already tracks the session-blocking unusable behavior.
- Initial investigation will distinguish backend runtime failure from TUI/API update failure before proposing an implementation fix.
- Ruled out the initial backend idle-status hang hypothesis with a route-level regression probe: `/session/{id}/prompt` emits `session.status: idle` after mock stream completion.
- Direct build evidence: `cargo build -p opencode-cli -p opencode-tui` succeeds; `ort-build` is not available in this shell.
- Direct runtime evidence with current config/default: `./target/debug/opencode run 'reply with exactly OK'` fails through OpenRouter with `401 Unauthorized: User not found`.
- Direct runtime evidence with Ollama: `./target/debug/opencode run -m ollama/qwen3:30b 'reply with exactly OK'` fails because `http://127.0.0.1:11434/v1/chat/completions` is unreachable.
- Direct runtime evidence with Anthropic: `./target/debug/opencode run -m anthropic/claude-haiku-4-5 'reply with exactly OK'` succeeds and returns `OK`.
- Direct runtime evidence with OpenAI before the fix: OpenAI models returned a blank assistant response. A direct Responses API stream showed the real failure was `credit_balance_exhausted`, emitted as nested `error.error.message` followed by `response.failed`; the Rust parser treated those event shapes as unknown/empty.
- Implemented fix: parse nested OpenAI Responses `error` events and `response.failed` events so provider failures surface to the user instead of producing blank assistant output.
- Verification after the fix: `./target/debug/opencode run -m openai/gpt-5-mini 'reply with exactly OK'` now reports `You have no credits remaining...` instead of a blank response.
- Residual blockers are configuration/environment issues, not fixed by this parser patch: OpenRouter credentials currently fail with 401, Ollama is not reachable locally, and OpenAI account credits are exhausted.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/22
- Merged into `development` on 2026-09-08 20:48 EDT via PR #22; awaiting QA on configured provider paths.

## Investigation - 2026-09-09 (deepseek/OpenRouter reproduction) - ROOT CAUSE FOUND

### Repro contract

- Surface: rust TUI/server session prompt + `opencode run` streaming path
- Input: `Hello, please analyze what we need to work on. I am testing out BUG-003 in this opencode session` and later longer verbose prompts
- Entity: rust session `ses_ce2007be05e04f1e8386c49203674344` (rust-schema `sessions`/`messages` tables), model `openrouter/deepseek/deepseek-v4-flash:free` (default config), later reproduced with `deepseek/deepseek-v4-flash` via `DEEPSEEK_API_KEY`
- Expected: full coherent assistant reply; session stays usable for follow-up prompts
- Actual: assistant reply persisted as scrambled fragments (`I dont context in yet analyze work, could1./ticket...`), then the next prompt (`hello again?`) was stored but never produced an assistant turn; the session visually "just stops"

### Runtime target

- Rust product binary `target/debug/opencode`; reused detached local TUI server on port 3000 (PID observed)
- DB evidence from `~/.local/share/opencode/opencode.db`: the garbled assistant text is stored verbatim in the rust `messages.data` column. This proves the corruption happened during stream assembly/persistence, NOT during markdown export. The `hello-please-analyze-...md` file in the repo is a faithful export of that stored (already corrupt) message.
- The same OpenRouter `deepseek-v4-flash:free` model works correctly in the reference (non-rust) product in the same session, which isolates the defect to the rust provider/session stream layer.

### Evidence chain

1. Data source (DB): `messages.data` for the failing assistant message contains garbled text exactly as exported. Confirmed by direct SQLite read.
2. Provider output: direct `curl` to `https://api.deepseek.com/chat/completions` and `https://openrouter.ai/api/v1/chat/completions` with the same prompts returns coherent SSE JSON. Raw model text reconstructed from the SSE stream is complete and well-formed.
3. Rust reproduction: `./target/debug/opencode run -m deepseek/deepseek-v4-flash "Write three complete sentences describing what boards workflow is..."` produced visibly garbled/truncated output; `reply with exactly OK` (very short) happened to survive.
4. Root-cause code inspection: all of the SSE-streaming providers that garbled share one broken pattern in `chat_stream`:

   ```rust
   response.bytes_stream().map(move |chunk_result| {
       // loop over text.lines() and `return Ok(event)` for the FIRST parseable data: line
   })
   ```

   Because `.map()` emits exactly one item per HTTP chunk, this parser:
   - keeps only the first SSE `data:` frame in any chunk and silently DROPS all later frames in that same chunk (the dominant cause of the missing text), and
   - cannot reassemble an SSE frame that is split across two network chunks (partial JSON fails to parse and is dropped).
5. Only `openai.rs` `chat_stream_legacy` already used a correct buffered drain (`try_unfold` + `drain_legacy_sse_events`). Every other provider (deepseek, openrouter, anthropic, google, vertex, groq, mistral, perplexity, cohere, together, xai, azure, vercel, cerebras, deepinfra, github_copilot legacy, gitlab) duplicated the lossy per-chunk parser.

### Failure point

The item is lost in the provider SSE parsing stage (stage 3 in the evidence pipeline): the HTTP chunk stream is converted to `StreamEvent`s before any session/session-loop logic runs. When the provider is a streaming text model and a single network chunk contains multiple deltas (or a delta straddles two chunks), most text deltas never become `TextDelta` events, so the assistant message assembled by the session layer is missing most of its content. This also explains why the stored provider/model/token metadata looked incomplete and why short canned replies sometimes "worked".

Note: this is the concrete, provider-agnostic root cause behind the repeated "session stops / unusable after first prompt" reports on Codex/OpenAI, Anthropic, and now deepseek. It is not a TUI-refresh or backend-idle hang. The separate "second prompt never answered" symptom is a downstream consequence of the first turn's stream being broken (event stream ends early / hangs on garbled partial frames), and must be re-verified on a fixed stream path.

## Implementation - 2026-09-09 (buffered SSE streaming fix)

### What changed

- Added a shared buffered SSE adapter `opencode_provider::stream::sse_event_stream(chunks, parse_line)` in `crates/opencode-provider/src/stream.rs`:
  - buffers raw bytes across HTTP chunk boundaries,
  - splits complete lines (CRLF-aware),
  - feeds every complete line to a provider parse closure and emits EVERY returned event (zero-or-more per line),
  - flushes a trailing partial frame at end-of-stream so the final frame without a trailing newline is not lost,
  - propagates transport errors as `ProviderError::StreamError`.
- Added shared per-line parse helpers:
  - `openai_compat_line_events` (handles `data: [DONE]` and OpenAI-compatible payloads),
  - `anthropic_line_events`,
  - module-local `google_line_events`, `vertex_line_events`, `vercel_line_events`, `gitlab_line_events`, `copilot_line_events`.
- Converted every provider that used the lossy per-chunk pattern to call `sse_event_stream` with its parser: `deepseek`, `openrouter`, `anthropic`, `google`, `vertex`, `groq`, `mistral`, `perplexity`, `cohere`, `together`, `xai`, `azure`, `vercel`, `cerebras`, `deepinfra`, `github_copilot` (legacy stream), `gitlab`. `openai.rs` already had a correct buffered path and was left unchanged; `responses.rs` already buffers frames correctly and was left unchanged.
- Removed the now-unused `TextDelta("")` no-op emissions and the first-event-`return` behavior that caused the data loss.

### Tests

- Added regression tests in `crates/opencode-provider/src/stream.rs`:
  - `sse_event_stream_emits_every_event_when_one_chunk_has_many_frames` — guards the primary bug (multiple SSE frames in one chunk are all delivered).
  - `sse_event_stream_reassembles_frames_split_across_chunks` — guards cross-chunk frame reassembly.
  - `sse_event_stream_flushes_trailing_frame_without_newline` — guards end-of-stream flush.
- All three pass; full `cargo test -p opencode-provider` (74+ tests) and `cargo test -p opencode-session` pass.

### Verification (live)

- Before: `./target/debug/opencode run -m deepseek/deepseek-v4-flash "Write three complete sentences..."` garbles output.
- After rebuild: the same verbose prompts return complete, coherent multi-sentence replies across repeated runs, e.g. a seashells prompt returned three full sentences and a moon prompt returned coherent multi-sentence output on repeated runs (one transient network error retried successfully).
- Unit regression: multi-frame-per-chunk and split-frame cases both pass.

### Residual / follow-ups (not fixed here)

- The streaming text is now reliable, but the multi-turn "second prompt never answers" behavior must be re-verified on the fixed provider path via the full TUI/server loop on the checked-out branch. If it still reproduces after the stream fix, that residual is a separate session-loop defect and should be split into its own card.
- Bedrock uses a binary Amazon eventstream framing rather than newline-delimited SSE and was intentionally left out of this change; it should be reviewed separately if it shows similar symptoms.
- After user verification, provider-path checks required by this card (Codex/OpenAI, Ollama, Anthropic, deepseek/OpenRouter) should be run per the "Required verification" section above.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/24 (branch `bug/BUG-003-buffered-sse-streaming`, base `development`)