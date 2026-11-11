---
id: "BUG-069"
title: "stats reports zero tokens for server-driven sessions"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-11-10"
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
