---
id: "BUG-074"
title: "Truncated native tool-call block drops the intended file write without retry"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-11-12"
---

# Truncated native tool-call block drops the intended file write without retry

## Summary
A board-item write was lost when the model's native tool-call block was truncated (missing closing token). The provider discarded the leaked markup and surfaced a `StreamEvent::Error`, but the turn ended with no tool executed and no automatic retry, so the file was never created. Re-issuing the same write succeeded and created the file.

## Observed
- The session attempted to create `boards/todo/show-selected-model-in-models-menu.md`.
- At the start of the next session, `read` on that path returned `File not found`.
- The provider emitted: `incomplete DeepSeek native tool-call block (missing closing token) was discarded; no tool was executed`.
- The `boards/todo` directory existed and was writable (`echo > boards/todo/testfile.md` succeeded), so this was not a permission or disk-space issue.
- Re-running the write with identical content created the file (1286 bytes).

## Current behavior
`DeepSeekNativeToolCallExtractor::finish` (`crates/opencode-provider/src/stream.rs:396`) discards an unterminated block and emits an error. That loud-discard behavior is intentional (BUG-063), but the failure is only a provider-stream error: it is not attached to the specific tool call, the agent gets no structured signal to retry, and the turn can end while the caller believes the write succeeded.

## Expected
- A truncated tool call should fail in a way the agent can act on: a tool-level failure or an actionable error that prompts a retry, rather than only a provider-stream diagnostic at end of turn.
- Keep the discarded block observable (log it, redacted) so lost writes are diagnosable.
- Make clear that a truncated native block means the tool did not execute.

## Acceptance Criteria
- [ ] A truncated native tool-call block that would have written a file produces a clear tool failure or retry prompt, not a dropped operation.
- [ ] The agent can distinguish "tool executed" from "tool block discarded".
- [ ] Add a regression test covering a truncated write tool call end-to-end.

## Related Items
- `BUG-063`: DeepSeek/Novita native tool-call leak resurfaced on stale build
- `FEAT-077`: Show selected model in the models menu (the write lost to this bug)

## Evidence
- `crates/opencode-provider/src/stream.rs:396-409`
- `crates/opencode-provider/tests/native_abort_flush.rs`

## Verification
- Reproduce a truncated native tool-call block (truncate the closing token) and confirm the agent receives an actionable failure and retries.
- `cargo test -p opencode-provider`.
