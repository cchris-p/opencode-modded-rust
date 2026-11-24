---
id: "TUI-001"
title: "Match vanilla context percentage and token counters in the TUI"
priority: "P2"
type: "bug"
area: "TUI"
spec: "wiki/harness-substrate.md"
status: "qa"
created: "2026-11-23"
updated: "2026-11-23"
---

# Match vanilla context percentage and token counters in the TUI

## Summary

The ORT TUI context readout does not match vanilla OpenCode, so the context
percentage cannot be trusted to show how full the window is or how close the
session is to its limit. The main defects are a wrong token total, a different
format, and a different placement.

Vanilla reference: `packages/tui/src/feature-plugins/sidebar/context.tsx`,
`packages/tui/src/component/prompt/index.tsx`,
`packages/tui/src/routes/session/subagent-footer.tsx`,
`packages/tui/src/util/locale.ts`
(pin `f54ce313b99a6661d7758ad042f7a6e05c8e0972`).

## Vanilla behavior

Context tokens are the **last assistant message** with `tokens.output > 0`, not
a sum over the session:

```ts
tokens = input + output + reasoning + cache.read + cache.write
pct = Math.round((tokens / model.limit.context) * 100)
```

Model limit is resolved from the **last assistant message's provider/model**.

Presentation:

- Prompt footer: `Locale.number(tokens)` + ` (pct%)`, then ` · cost` only when
  cost > 0. `Locale.number` abbreviates: `1200 -> "1.2K"`, `1000000 -> "1.0M"`.
- Sidebar "Context" block:
  - `{tokens.toLocaleString()} tokens` (comma grouped, e.g. `12,345 tokens`)
  - `{pct ?? 0}% used`
  - `{money.format(cost)} spent`
- No color/threshold change when nearing the limit in vanilla; the percentage is
  the signal.

## ORT current behavior / divergence

`crates/opencode-tui/src/components/sidebar.rs`:

- `total_tokens` sums every assistant message in the session, so the value grows
  without bound and does not represent current context fill.
- Renders `Tokens 12,345  56%` / `Cost $0.00`.

`crates/opencode-tui/src/components/session.rs` (header):

- Uses the last assistant message (correct) but formats
  `12,345 56% ($0.12)` instead of vanilla `12.3K (56%) · $0.12`.
- Always shows cost, even `$0.00` (vanilla hides zero cost in the prompt footer).
- Uses comma grouping instead of vanilla's K/M abbreviation.

`crates/opencode-tui/src/components/prompt.rs`:

- No context usage in the prompt hint line, which is where vanilla shows it.

## Proposed fix

1. Add one shared context-usage helper (last assistant message tokens, percent
   from that message's model, K/M and comma formatters) used by the header,
   sidebar, and prompt footer so the numbers cannot drift apart.
2. Header: `Locale.number` style `1.2K (56%)`, append ` · $0.12` only when cost
   > 0.
3. Sidebar: `N tokens` / `P% used` / `$X.XX spent` per vanilla.
4. Prompt hint: right-align the same usage string (hidden while retrying, as in
   vanilla), so the fill is always visible with the default hidden sidebar.
5. Keep the model-limit lookup exact on `provider/model` with the existing
   suffix fallback so a full model ref and a bare model id both resolve.

## Acceptance

- Header, sidebar, and prompt use the last assistant message token total and the
  same percentage as vanilla for the same session.
- Formatting matches vanilla (`1.2K (56%) · $0.12`, `12,345 tokens`,
  `56% used`, `$0.12 spent`).
- Unit tests cover token selection (last message, not sum), percent math, and
  both number formats.
- Manual TUI check: percentage increases with each turn and matches the vanilla
  value for the same model/session.

## Related

- `BUG-069` (stats token accounting)
- `BUG-068` / TUI footer cards

PR: https://github.com/cchris-p/opencode-modded-rust/pull/208 (branch `bug/TUI-001-context-token-parity`)

## Implementation notes (2026-11-23)

- Added `crates/opencode-tui/src/components/context_usage.rs` as the single
  source for context tokens/percentage, model-limit lookup, cost sum, and the
  vanilla K/M + comma formatters.
- Header (`session.rs`): `12.3K (56%) · $0.12`; cost omitted when zero.
- Sidebar (`sidebar.rs`): `12,345 tokens` / `56% used` / `$0.12 spent`; token
  total now comes from the last assistant message instead of the session sum.
- Prompt hint (`prompt.rs`): right-aligned usage text, hidden while retrying and
  in shell mode to match vanilla.
- Tests: `cargo test -p opencode-tui` (229 passed), including new
  `context_usage` tests for last-message selection, percent math, and number
  formats. `cargo build -p opencode-cli` succeeds.

Open question for the owner: vanilla has no color/threshold change as the
window fills. This card matches vanilla exactly; a separate near-limit color
warning would be an ORT-only addition.
