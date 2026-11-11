---
id: "BUG-069"
title: "stats reports zero tokens for server-driven sessions"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-11-10"
updated: "2026-11-11"
---

# stats reports zero tokens for server-driven sessions

## Summary

After a server-driven agentic prompt completes, per-message token records are
populated, but `opencode-modded-rust stats` reports zero input/output tokens and
zero tool usage totals for the session's token fields (tool usage itself is
listed).

## Evidence

`qwen30-eval` token run `token-T03-short-1`:

- `results/evidence/token-T03-short-1/messages.json`:
  three assistant messages with input 5333, 5441, 5861 and output 650, 439, 274
  (sums 16635 / 1363).
- `results/evidence/token-T03-short-1/stats.txt`:

  ```
  Sessions: 1
  Messages: 6
  Total Cost: $0.0000
  Tokens: input=0 output=0 reasoning=0 cache_read=0 cache_write=0
  ```

The session is loaded from storage, but its `usage` totals appear to remain zero
even though message-level usage is present.

## Impact

- Token and cost accounting for server-driven work is unusable.
- Evaluation and budget tooling must parse message records instead of `stats`.
- Any future spend cap or cost-per-task metric built on `stats` would be wrong.

## Expected

- Aggregate per-message usage into the session usage on completion and persist
  it.
- If session usage is absent or zero, compute `stats` from message records as a
  fallback.
- Add a test with two or more assistant messages and assert that `stats` equals
  the summed message tokens.

## Related

- `BUG-068` — the attach path that exposed this.
- `qwen30-eval` `Q30-008`.
## Implementation 2026-11-11

Branch: `bug/BUG-069-stats-usage`. Not merged; owner opens and merges the PR.

Changes:
- `SessionPrompt` now calls `session.update_usage(session.get_usage())` after
  each step finalizes an assistant message, so the session usage (and the
  persisted snapshot merged by the server) carries aggregated tokens.
- `stats` uses `effective_usage_totals`: when the stored session usage is
  absent or all zero it sums assistant message `metadata.usage`
  (`prompt_tokens`, `completion_tokens`, `reasoning_tokens`,
  `cache_read_tokens`, `cache_write_tokens`).
- Regression test `tests::stats_falls_back_to_summed_message_usage_when_session_usage_is_zero`
  uses two assistant messages plus a user message and asserts `stats` totals
  equal the summed message tokens.

Verification:
- `cargo fmt --all`
- `SCOPEMUX_PREBUILT_DIR=... cargo test -p opencode-storage -p opencode-server -p opencode-cli`
  -> all suites passed (23 + 75 + 3 + 5 + 2, 0 failed).
