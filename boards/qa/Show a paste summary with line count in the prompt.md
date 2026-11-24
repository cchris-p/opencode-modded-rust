---
id: "TUI-002"
title: "Show a paste summary with line count in the prompt"
priority: "P2"
type: "feature"
area: "TUI"
spec: "wiki/harness-substrate.md"
status: "qa"
created: "2026-11-24"
updated: "2026-11-24"
---

# Show a paste summary with line count in the prompt

## Summary

Pasting more than a couple of lines into the ORT prompt dumps the entire block
into the input box, pushing the draft off screen. Vanilla OpenCode collapses a
large paste to a `[Pasted ~N lines]` placeholder and expands it back to the full
text on submit. ORT has no equivalent.

Vanilla reference: `packages/tui/src/component/prompt/index.tsx`
(`pasteInputText`, `pasteText`, `expandTrackedPastedText`),
`packages/tui/src/prompt/part.ts`, `packages/tui/src/theme/index.ts`
(pin `f54ce313b99a6661d7758ad042f7a6e05c8e0972`).

## Vanilla behavior

- Paste normalizes CRLF/CR to LF, then trims the content for the summary check.
- `lineCount = newlines + 1`.
- If `lineCount >= 3` or `trimmed.length > 150`, and
  `experimental.disable_paste_summary` is not set, insert the virtual text
  `[Pasted ~N lines]` and keep the real text in a hidden part.
- The placeholder is styled `extmark.paste`: warning background, contrast
  foreground, bold.
- On submit the placeholder expands to the original pasted text; editing an
  overlapping range drops the part.
- `experimental.disable_paste_summary` turns the collapse off (inline paste).

## Plan

- [x] Add `Config::paste_summary_enabled()` reading
      `experimental.disable_paste_summary` (default on).
- [x] Expose the resolved flag on `AppContext` and seed it in `App::new`.
- [x] Add `Prompt::paste_text` implementing the line/size threshold, trimming,
      placeholder insertion, and `[Pasted ~N lines]` text.
- [x] Track placeholder byte ranges and shift/drop them on edits.
- [x] Expand placeholders in `take_input` (what is submitted), and stash/history
      store expanded text so content is never lost.
- [x] Render the placeholder with the warning background + contrast foreground.
- [x] Route bracketed-paste and clipboard-paste through the new path.

## Acceptance

- Pasting >= 3 lines or > 150 chars shows `[Pasted ~N lines]`; small pastes stay
  inline.
- Submitting the draft sends the full pasted text.
- Editing before the placeholder shifts it; editing inside drops the hidden
  content instead of sending a stale placeholder.
- `experimental.disable_paste_summary: true` restores inline paste.

## Tests

`cargo test -p opencode-tui components::prompt`
`cargo test -p opencode-config`

Covered: multiline summary + expand, small inline paste, long single line,
surrounding typed text, delete-into-placeholder, rendered placeholder style,
config flag default/on/off.

## Related

- [BUG-059](pasted-text-not-bounded-in-prompt-input.md) — normalization of
  CRLF/CR/tabs at the paste boundary; `paste_text` reuses it.
- [TUI-003](Style%20%40file%20references%20in%20the%20prompt%20input.md) — the
  other prompt-input parity fix in this pair.
## Implementation

- `crates/opencode-config/src/schema.rs`: `Config::paste_summary_enabled()`.
- `crates/opencode-tui/src/context/app_context.rs`: `paste_summary_enabled`
  flag + accessors, default `true`.
- `crates/opencode-tui/src/app/app.rs`: seed the flag from config; route
  bracketed/clipboard paste through `paste_into_active_input`, which uses
  `Prompt::paste_text` for the session prompt.
- `crates/opencode-tui/src/components/prompt.rs`: `PastedSegment` tracking,
  `paste_text`, edit-time range shifting, expansion in `take_input`, stash and
  history store expanded text, placeholder style in `render`.

## QA evidence

- `cargo test -p opencode-tui` — 238 passed.
- `cargo test -p opencode-config` — 67 passed.
- `cargo clippy -p opencode-config -p opencode-tui --all-targets` — no new
  warnings from this change.
- `cargo check --workspace` blocked by a pre-existing local environment gap
  (`third_party/scopemux-core` not fetched / no prebuilt bundle), not by this
  change; `cargo check -p opencode-config -p opencode-tui` passes.

## Follow-up noted (not in scope)

`tab_autocomplete_uses_first_candidate` was timing-flaky (depends on two frecency
bumps landing in the same millisecond); pinned its timestamps in the test while
touching the file.
## Merge closeout

- PR #211 merged into `development` (merge commit `790532e`); branch
  `feature/TUI-002-003-prompt-input-parity` deleted after merge.
- Remaining gate: post-merge QA on `development` (agent-driven QA passed on the
  PR branch before merge).
