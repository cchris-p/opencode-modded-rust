---
id: "TUI-003"
title: "Style @file references in the prompt input"
priority: "P2"
type: "feature"
area: "TUI"
spec: "wiki/harness-substrate.md"
status: "qa"
created: "2026-11-24"
updated: "2026-11-24"
---

# Style @file references in the prompt input

## Summary

When a file (or agent) is referenced in the ORT prompt with `@`, the token
renders as plain text. Vanilla OpenCode gives the inserted reference a distinct
style in the input box: file references are warning-colored and bold, agent
references are secondary-colored and bold. This makes it obvious that the token
resolved to a workspace file.

Vanilla reference: `packages/tui/src/component/prompt/autocomplete.tsx`
(`insertPart`), `packages/tui/src/component/prompt/index.tsx`
(`fileStyleId`, `restoreExtmarksFromParts`), `packages/tui/src/theme/index.ts`
(`extmark.file`, `extmark.agent`) (pin `f54ce313b99a6661d7758ad042f7a6e05c8e0972`).

## Vanilla behavior

- Selecting a `@file` completion inserts `@path ` as virtual text and creates an
  extmark styled `extmark.file` (`foreground: warning`, `bold`).
- `@agent` selections use `extmark.agent` (`foreground: secondary`, `bold`).
- The style tracks the token's range as the buffer is edited.

## Plan

- [x] Track the raw-input byte range each wrapped line renders (`WrapTrack` +
      `WrappedPromptInput::line_ranges`).
- [x] Detect `@token` references at whitespace-delimited token starts.
- [x] Style file refs with `theme.warning` + bold; known agents with
      `theme.secondary` + bold.
- [x] Build styled spans per wrapped line from those ranges without changing the
      wrap/cursor model.
- [x] Keep pasted placeholders from being re-styled as references.

## Acceptance

- A rendered `@path` token in the prompt is warning-colored and bold.
- `@<known-agent>` renders secondary-colored and bold.
- Mid-token `@` (e.g. an email address) is not styled.
- Existing wrapping, cursor placement, and height logic are unchanged.

## Tests

`cargo test -p opencode-tui components::prompt` — reference-token detection,
line-range mapping, and a rendered-render assertion for the warning foreground.

## Related

- [TUI-002](Show%20a%20paste%20summary%20with%20line%20count%20in%20the%20prompt.md)
  — the companion prompt-input parity fix.
## Implementation

- `crates/opencode-tui/src/components/prompt.rs`:
  - `WrapTrack` / `WrappedPromptInput::line_ranges` record the raw-input byte
    range each wrapped line renders.
  - `reference_token_ranges` finds whitespace-delimited `@token` refs.
  - `Prompt::style_ranges` assigns `extmark.file` (warning, bold) or
    `extmark.agent` (secondary, bold for known agents).
  - `build_styled_line` emits styled spans per wrapped line; pasted
    placeholders take precedence over reference styling.
  - Cursor/wrap/height logic is untouched.

## QA evidence

- `cargo test -p opencode-tui` — 238 passed (includes a render test asserting
  the warning background/foreground for the placeholder and `@file` token).
- `cargo clippy -p opencode-config -p opencode-tui --all-targets` — no new
  warnings from this change.
## Merge closeout

- PR #211 merged into `development` (merge commit `790532e`); branch
  `feature/TUI-002-003-prompt-input-parity` deleted after merge.
- Remaining gate: post-merge QA on `development` (agent-driven QA passed on the
  PR branch before merge).
