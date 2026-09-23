---
id: "BUG-028"
title: "Session wedges with a recurring 400 when an assistant tool_calls turn has no matching tool messages"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-21"
---

# Session wedges with a recurring 400 when an assistant tool_calls turn has no matching tool messages

## Summary

A session that ends a turn with an assistant `tool_calls` but leaves one or more of those calls
without a matching tool result becomes **permanently unusable**. Every subsequent prompt re-sends the
same invalid history, so the provider rejects each request with:

```
400 Bad Request: An assistant message with 'tool_calls' must be followed by tool messages
responding to each 'tool_call_id'. (insufficient tool messages following tool_calls message)
```

The user cannot recover the session by typing "continue" or a new prompt; the only workaround is to
abandon the session. This is the provider-visible consequence of an unresolved tool-call turn: the
runtime may store an assistant message with `tool_use` parts and no matching `tool_result`, and the
OpenAI chat wire conversion then emits an assistant message with `tool_calls` that is not immediately
followed by enough `role: "tool"` messages.

## Reported behavior / Evidence

Newest reproduction (2026-09-21), transcript
[`docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md`](../../docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md)
(session `ses_c06a60f9235747e8b9a6a06999391d13`):

1. The assistant (Build) emitted a `write` tool call with empty arguments (`{}`). The tool returned
   `Error: Invalid arguments: file_path is required` (transcript lines ~3098-3114).
2. The run was then aborted: `Aborted by user.` (transcript line ~3116).
3. The user sent a normal follow-up prompt (transcript line ~3120-3122).
4. The next assistant turn (Plan) failed immediately with the `400 ... tool_calls must be followed by
   tool messages ... insufficient tool messages` error (transcript line ~3128).

Earlier reproduction (2026-09-15), transcript
[`docs/archive/session-tool-call-summary-order-2026-09-15.md`](../../docs/archive/session-tool-call-summary-order-2026-09-15.md)
(session `ses_cf186e35b53a42debd3dcd1375e01707`): the assistant emitted `grep` + `bash` tool calls
with no results, the user said `continue I think you're stuck`, and then the **exact same 400**
appeared twice in a row (lines 47, 59). The session did not recover between attempts.

Confirmed observations:

- The error text is identical across both sessions and months: `An assistant message with 'tool_calls'
  must be followed by tool messages responding to each 'tool_call_id'. (insufficient tool messages
  following tool_calls message)`.
- The failure is **recurring for the session**, not a one-off provider hiccup: once the invalid turn
  is in history, every new prompt includes it and fails the same way.
- Both repros follow a tool-call turn that did not cleanly resolve to results (stall, error, or
  abort), which is the `BUG-016` / `BUG-023` territory.

Do not conflate with the other 400: `BUG-019` owns
`400 Invalid assistant message: content or tool_calls must be set`, which is a different message
shape (a reasoning-only aborted turn) and a different code path.

## Why this exists

`invariants/coding-session-behavior.md` treats a session as durable and continuable. A session that
accepts a prompt, fails at the provider with a hard `400`, and repeats that failure forever violates
continuability and strands the user's work. Because the bad state is in the conversation history
itself, the defect is self-perpetuating: nothing in the request path repairs or refuses to send an
invalid tool-call history.

## Code evidence

Provider wire conversion trusts the internal history:

- `crates/opencode-provider/src/openai_chat.rs` `convert_messages` (`:80-128`) and
  `convert_assistant_parts` (`:132-218`):
  - `tool_use` parts become `assistant.tool_calls` (`:159-170`, emitted `:209-211`).
  - Tool results are emitted as separate `{ role: "tool", tool_call_id, content }` messages **only**
    from `tool_result` parts (`:172-178`, `:215-217`).
  - Consequently, if a `tool_use` id has no matching `tool_result`, the outgoing body has an
    assistant message with `tool_calls` that is not followed by enough `tool` messages, and the
    provider rejects the whole request.
- Legacy bare-text tool results push `(String::new(), text)` (`:175-178`), producing a `role: "tool"`
  message with no `tool_call_id`, which cannot satisfy the call either.

Runtime repair guards exist but are scoped to a live run and append at the end of the list:

- `unresolved_tool_call_ids` / `append_missing_tool_results`
  (`crates/opencode-session/src/prompt.rs:1548-1591`).
- Call sites after tool execution (`prompt.rs:1403-1411`) and abort cleanup
  `abort_pending_tool_calls` (`prompt.rs:1593-1610`, invoked `:1430-1433`).
- Both append a new assistant message holding the tool results at the **end** of
  `session.messages` (`:1585-1589`, `:1605-1609`), not immediately after the owning assistant
  `tool_calls` message. If any turn lands in between (an abort placeholder, or a user prompt typed
  before the guard runs), the repaired `role: "tool"` messages are no longer adjacent to their
  `tool_calls` and the history stays provider-invalid.
- Nothing validates or repairs the history at request-build time, so a history already persisted in a
  bad shape is re-sent verbatim on every prompt.

`execute_tool_calls` itself looks correct for a single turn: it collects calls from the last assistant
message (`prompt.rs:1620-1638`) and records an error `ToolResult` for a failed tool (`:1659-1662`).
The wedge therefore implies a state or ordering gap *around* that path, not that the parser forgets
every failure.

## Trigger hypotheses (confidence-ranked)

- **H1 (85%) - Provider-invalid history is re-sent forever.** An assistant `tool_calls` turn lacks
  the immediately-following matching `role: "tool"` messages, and no request-time repair exists, so
  every subsequent prompt 400s on the same history. Evidence: both repros repeat the identical error
  on retry.
- **H2 (60%) - Repair placement is wrong.** `append_missing_tool_results` /
  `abort_pending_tool_calls` append results at the end of the message list (`prompt.rs:1585-1589`,
  `:1605-1609`). An intervening turn (abort placeholder or user prompt) puts the tool messages after
  a non-tool message, which the provider still rejects.
- **H3 (50%) - Torn-down/persisted turn loses or reorders results.** An aborted or torn-down run
  leaves a tool-call turn whose results are not durable or not ordered adjacent, so on resume the
  provider sees the unresolved shape. Overlaps `BUG-025` (concurrent sync deletes messages).
- **H4 (40%) - Invalid/empty tool arguments can leave an unresolved call.** The 2026-09-21 repro was
  triggered by `write` with empty input returning `Invalid arguments: file_path is required`; confirm
  whether that error path always attaches a `tool_result` for the exact call id.
- **H5 (25%) - Missing request-time normalization.** The provider conversion trusts internal history
  and has no "drop or repair an unresolved tool-call turn" guard, so any bad state is fatal rather
  than skipped/synthesized.

## Open questions

1. In the 2026-09-21 session, was the failed `write` call recorded with a matching `tool_result`, and
   what is the exact message/part order in the history at the moment of the 400?
2. Does typing a new prompt *before* the abort/repair guard runs insert a user message between the
   `tool_calls` assistant and its repaired `role: "tool"` messages?
3. Is the 400 reproducible from a clean history by constructing a `tool_use` with no result, or is an
   abort/interrupt required?
4. After the fix, can a session that already failed once recover, or must the malformed turn be
   repaired on load?

## Scope

- Guarantee the outgoing provider request invariant: every assistant `tool_use` id has a matching
  `role: "tool"` reply immediately following the owning assistant message.
- Repair or safely drop a provider-invalid tool-call history at request-build time, not only inside a
  live prompt run.
- Fix repair placement so synthesized/aborted tool results are adjacent to the assistant message that
  owns the calls, before any intervening user/assistant turn.
- Keep the existing `BUG-016` guard; make this the defensive layer that prevents a wedge even when the
  live-loop guard is bypassed (abort, torn-down run, resumed persisted history).
- Record the confirmed root cause and add a regression test at the failure point.

## Non-goals

- Re-litigating or reverting the `BUG-016` unresolved-tool-call repair guard.
- The interrupt `400` shape and abort semantics owned by `BUG-019`, or resume behavior in `FEAT-035`.
- Broad provider transport rewrites beyond guaranteeing request validity.
- Redesigning the session schema or migrating existing data beyond repairing malformed turns.

## Done when

- A session whose history contains an assistant `tool_calls` turn without matching tool messages no
  longer wedges: the next prompt either succeeds after repair or fails with a clear, actionable
  message, and is not permanently stuck.
- Every outgoing OpenAI-compatible request satisfies the tool-call/tool-message adjacency invariant.
- A regression test builds a history with an unresolved `tool_use` plus an intervening user/assistant
  turn, runs the request path, and asserts the result is valid (repaired or the turn dropped) rather
  than a provider `400`.
- The 2026-09-21 trigger sequence (invalid-argument tool call, abort, follow-up prompt) completes
  without the recurring 400.

## Recommended verification

- `cargo test -p opencode-provider` with a `convert_messages` test for an assistant `tool_use` with no
  matching `tool_result`, including an intervening user message.
- `cargo test -p opencode-session` for repair placement beside the owning assistant message.
- `cargo check -p opencode-provider -p opencode-session -p opencode-server`.
- Live: `ort-build`, then `ort`; force a failing/empty-argument tool call, abort the run, then send a
  follow-up prompt and confirm it completes instead of returning the `400`.
- Confirm a previously malformed history can be resumed rather than only avoiding new wedges.

## Related Items

- `BUG-016` Plan-mode session stalls after tool calls without tool results - ships the live-loop
  unresolved-tool-call repair guard; this card covers the provider rejection and session wedge that
  remain when that guard is bypassed or the repair is misplaced.
- `BUG-023` Root-cause why the plan-mode session stalled after tool calls without results - documents
  this exact `400` in the 2026-09-15 recurrence but scoped it out; this card picks it up as the
  provider-validity defect.
- `BUG-019` Escape does not interrupt the running session - owns the different
  `400 Invalid assistant message: content or tool_calls must be set` shape; same class of
  provider-invalid history, different cause.
- `BUG-025` Concurrent servers delete each other's sessions and messages - full-snapshot sync can
  strip tool results from an otherwise valid turn, producing this unresolved shape.
- `BUG-012` Session summary runs before tool results - same provider-error family (ordering of tool
  results vs. the turn that needs them), different trigger.
- `FEAT-035` Resume an interrupted session from where it left off - resume must not re-send an invalid
  tool-call history; depends on the repair this card adds.
- `BUG-027` Session keeps freezing during thinking mode - adjacent abort/interrupt and loop-exit
  behavior on the same `prompt.rs` path.

## Dev Notes

- Implemented the request-build-time normalizer in
  `crates/opencode-provider/src/openai_chat.rs`. `convert_messages` now ends by calling
  `normalize_tool_call_replies`, so every OpenAI-compatible provider that builds a chat-completions
  body (`deepseek`, `openrouter`, `openai`, `mistral`, `groq`, `xai`, `together`, `perplexity`,
  `cohere`, `azure`, `cerebras`, `deepinfra`, ...) enforces the tool-call adjacency invariant.
- Normalizer behavior:
  - every assistant `tool_calls` message is immediately followed by one `role: "tool"` message per
    call id, in call order;
  - a tool reply already present anywhere in the list is reused and moved next to its owning
    assistant message (duplicates dropped);
  - a call id with no reply gets a synthetic `tool` error message
    (`Tool result unavailable: the previous turn ended before this tool call produced a result.`)
    plus a `tracing::warn!` naming the call id;
  - tool messages that cannot be tied to a call id are preserved as-is so no content is lost.
- Design decision: the fix is enforced at the provider boundary rather than by changing
  `append_missing_tool_results` / `abort_pending_tool_calls` placement in
  `crates/opencode-session/src/prompt.rs`. The boundary is the only place that can guarantee the
  outgoing wire shape for a history that is already malformed on disk, so it covers the abort,
  stalled-run, resumed-history, and `BUG-025` data-loss cases in one root-cause-agnostic change.
  Internal repair placement was left as-is; the boundary normalizer makes it non-fatal.
- A history that already wedged before the fix is repaired at request time, so the session can
  continue without manual intervention.

## Verification

- `cargo test -p opencode-provider` -> 94 passed + 7 integration passed, 0 failed.
- New tests in `openai_chat::tests`:
  - `unresolved_tool_call_gets_synthetic_reply_before_user_message` (the wedge case: tool calls, then
    a user "continue", no results);
  - `existing_tool_reply_is_moved_adjacent_to_owning_assistant_message` (reply after an intervening
    user message is moved next to the owning assistant message and not duplicated);
  - `resolved_tool_calls_keep_their_reply_content` (normal multi-call turn unchanged).
- Updated `assistant_reasoning_is_echoed_as_reasoning_content` to assert the now-guaranteed synthetic
  reply for its unresolved tool call.
- `cargo check -p opencode-provider -p opencode-session -p opencode-server` passed.
- `cargo fmt -p opencode-provider -- --check` clean.

## Notes

- Evidence for this card:
  [`docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md`](../../docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md)
  (`ses_c06a60f9235747e8b9a6a06999391d13`, 2026-09-21) and
  `docs/archive/session-tool-call-summary-order-2026-09-15.md`
  (`ses_cf186e35b53a42debd3dcd1375e01707`, 2026-09-15). Keep new exports under `docs/transcripts/`.
- `docs/transcripts/tool-call-issue.md` is `BUG-019`'s evidence and shows a different message; do not
  merge the two.
- Primary files: `crates/opencode-provider/src/openai_chat.rs`,
  `crates/opencode-session/src/prompt.rs`, `crates/opencode-server/src/routes.rs`.
- Delivered fix is the request-build-time normalizer in `openai_chat.rs` described in Dev Notes. The
  original open question (exact failing message order in the two repros) is now moot for the wire
  invariant, which holds regardless of ordering; a live TUI reproduction is still the recommended QA
  check before closeout.
- Non-OpenAI-compatible providers (`anthropic`, `google`, `bedrock`) use their own converters and were
  intentionally not changed in this card; if the same wedge appears there it should be tracked
  separately.

## Completion

- 2026-09-21: PR #64 merged into `development` (merge commit `5445b4514ec1f8856c1586425a4488473dd0be9a`);
  branch `bug/BUG-028-tool-call-reply-validity` deleted remotely and locally; local checkout is back on
  `development` and current.
- **Remains in `qa`.** The merge delivered the request-boundary normalizer and its tests, but no QA
  report is recorded and the live TUI acceptance check (invalid-argument tool call -> abort ->
  follow-up prompt completes without the recurring `400`) has not been run. Promote to `done` only
  after that QA report is recorded or the user explicitly completes the card.

## QA Notes (2026-09-23)

- Re-confirmed the delivered request-boundary normalizer is present in `development`:
  `openai_chat::convert_messages` ends by calling `normalize_tool_call_replies`, so every
  OpenAI-compatible provider emits `role: "tool"` replies adjacent to the owning assistant
  `tool_calls`.
- `cargo test -p opencode-provider tool_call` -> 5 passed, including
  `unresolved_tool_call_gets_synthetic_reply_before_user_message`,
  `assistant_tool_use_becomes_tool_calls_and_tool_messages`, and
  `resolved_tool_calls_keep_their_reply_content`.
- `cargo test -p opencode-provider` -> 99 passed + 7 integration passed, 0 failed.
- No regression from the BUG-038 stream-timeout change on the same request path.
- The live TUI acceptance sequence (empty-argument tool call -> abort -> follow-up prompt) was **not**
  run in this headless environment; that remains the only open live check.

## Merge Closeout - 2026-09-23

- Cluster closeout PR #89 merged into `development`; card moved `qa -> done`.
- The request-boundary normalizer and its tests were already merged (PR #64); this closeout records
  the re-verification on the current tree.
