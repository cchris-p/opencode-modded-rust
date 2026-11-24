---
id: "H-022"
title: "TUI-002/TUI-003 prompt input paste summary and @file reference styling - Handoff"
status: "complete"
created: "2026-11-24"
updated: "2026-11-24"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["TUI-002", "TUI-003"]
---

# TUI-002/TUI-003 Prompt Input Parity - Handoff

## Objective

Bring two vanilla OpenCode prompt-input behaviors to ORT's session prompt:

- `TUI-002`: collapse large pastes into a `[Pasted ~N lines]` placeholder that
  expands back to the full text on submit.
- `TUI-003`: style `@file` / `@agent` references in the input box (file =
  warning + bold, agent = secondary + bold).

Both cards are currently in `boards/qa/` and are scoped to the **main session
prompt only** (`crates/opencode-tui/src/components/prompt.rs`). Do not change
the question tool or dialog inputs.

## Important: implementation already exists uncommitted

A complete implementation was written in the current worktree by the previous
session but was **not committed**. Before doing anything:

```bash
git -C ~/apps/opencode-modded-rust status --short
git -C ~/apps/opencode-modded-rust diff -- \
  crates/opencode-config/src/schema.rs \
  crates/opencode-tui/src/app/app.rs \
  crates/opencode-tui/src/components/prompt.rs \
  crates/opencode-tui/src/context/app_context.rs
```

If those diffs and the two new board cards are present, **verify, commit, and
open the PR** — do not re-implement from scratch. If the diff is absent (fresh
clone / other machine), implement from the design below.

Never discard unrelated dirty work in the worktree. Stage only the TUI-002/
TUI-003 files and cards.

## Included Board Items

| Item | Title | Lane |
|------|-------|------|
| `TUI-002` | Show a paste summary with line count in the prompt | `qa` |
| `TUI-003` | Style `@file` references in the prompt input | `qa` |

Related: `BUG-059` (`boards/qa/pasted-text-not-bounded-in-prompt-input.md`) owns
`normalize_prompt_text`; the paste path must reuse it. Do not duplicate that
normalization.

## Vanilla reference (pin `f54ce313b99a6661d7758ad042f7a6e05c8e0972`)

Repo: `~/repos/opencode-modded` (`git fetch origin dev` first).

- Paste: `packages/tui/src/component/prompt/index.tsx` (`pasteInputText`,
  `pasteText`, `expandTrackedPastedText`), `packages/tui/src/prompt/part.ts`.
  - Normalize CRLF/CR → LF, trim for the check.
  - `lineCount = newlines + 1`.
  - Summarize when `lineCount >= 3 || trimmed.length > 150`, unless
    `experimental.disable_paste_summary`.
  - Insert `[Pasted ~N lines]` as virtual text; expand on submit; editing an
    overlapping range drops the part.
- Reference style: `packages/tui/src/component/prompt/autocomplete.tsx`
  (`insertPart`), `packages/tui/src/components/prompt/index.tsx`
  (`fileStyleId`), `packages/tui/src/theme/index.ts`:
  - `extmark.file`: `foreground: theme.warning`, `bold`.
  - `extmark.agent`: `foreground: theme.secondary`, `bold`.
  - `extmark.paste`: warning background, contrast foreground, bold.

## Design / implementation map

All paths under `crates/`.

1. `opencode-config/src/schema.rs`
   - Add `Config::paste_summary_enabled() -> bool` =
     `!experimental.disable_paste_summary.unwrap_or(false)`.
2. `opencode-tui/src/context/app_context.rs`
   - Add `pub paste_summary_enabled: RwLock<bool>` (default `true`) plus
     `set_paste_summary_enabled` / `paste_summary_enabled` accessors.
3. `opencode-tui/src/app/app.rs`
   - After `context.set_experimental_background_subagents(...)`, call
     `context.set_paste_summary_enabled(config.paste_summary_enabled())`.
   - Route `Event::Paste` and `Ctrl+V` clipboard paste through a new
     `paste_into_active_input` that calls `Prompt::paste_text` for the session
     prompt and `QuestionPrompt::insert_text` when a question is open.
   - Keep `input_newline` using `Prompt::insert_text` (raw), not the paste path.
4. `opencode-tui/src/components/prompt.rs`
   - `PastedSegment { start, end, text }` + `Prompt.pasted: Vec<PastedSegment>`.
   - `Prompt::paste_text`:
     - `normalize_prompt_text` (BUG-059), trim for the measurement.
     - `line_count = trimmed.matches('\n').count() + 1`.
     - summarize when enabled and (`line_count >= 3` or
       `trimmed.chars().count() > 150`); insert `[Pasted ~{line_count} lines]`,
       store the trimmed text.
     - otherwise insert the normalized text inline.
   - `shift_pasted_segments(at, removed, inserted)`: edit before/at placeholder
     shifts it, edit after leaves it, overlapping edit drops it.
   - Call the shift helper from Char insert, Backspace, Delete,
     `insert_text`, and `apply_selected_suggestion`.
   - `take_input` returns the expanded text; history/stash store expanded text.
   - `set_input`, `clear`, and history recall clear `pasted`.
   - Tracking for styling: add `line_ranges: Vec<(usize, usize)>` to
     `WrappedPromptInput` and a `WrapTrack` accumulator threaded through
     `wrap_prompt_input` / `push_wrapped_whitespace` /
     `append_pending_prompt_whitespace` so each wrapped line knows its raw byte
     range. This must not change wrap, cursor, or height behavior.
   - `reference_token_ranges(input)`: `@token` at a whitespace-delimited token
     start (not mid-token, e.g. emails).
   - `Prompt::style_ranges(theme)`: paste ranges use
     `theme.selected_foreground(Some(theme.warning))` on `theme.warning` + bold;
     `@` refs use `theme.warning` (files) or `theme.secondary` (known agents) +
     bold; skip refs that overlap a paste range.
   - `build_styled_line` intersects style ranges with each line range and emits
     spans; use it in `render` instead of plain `Line::from(line.clone())`.

## Acceptance

- `>= 3` lines or `> 150` chars paste → `[Pasted ~N lines]`; small paste inline.
- Submit sends the full pasted text; stash/history do not lose it.
- Editing before a placeholder shifts it; editing inside drops hidden content.
- `experimental.disable_paste_summary: true` disables collapsing.
- `@path` renders warning + bold; `@<known-agent>` secondary + bold; `a@b.com`
  not styled.
- Wrap/cursor/height behavior and all existing prompt tests unchanged.
- Rendered placeholder shows the warning background and contrast foreground.

## Verification (already passing on the uncommitted work)

```bash
cd ~/apps/opencode-modded-rust
cargo fmt --all
cargo test -p opencode-tui        # 238 passed
cargo test -p opencode-config     # 67 passed
cargo clippy -p opencode-config -p opencode-tui --all-targets
```

- `cargo check --workspace` fails locally because `third_party/scopemux-core`
  is not fetched and no prebuilt bundle is present. Use
  `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-config -p opencode-tui`
  for the scoped check; do not treat the workspace gap as a regression.
- The previous session also pinned timestamps in
  `tab_autocomplete_uses_first_candidate` to remove a pre-existing same-ms
  flake while editing the file. Keep that fix.

## Next-agent steps

1. Run `python3 ~/standards/opencode/scripts/agent_roles.py list`; claim a free
   slot with scope `TUI-002,TUI-003 prompt input parity` before touching code.
2. Inspect `git status`/`git diff` as above. Preserve unrelated dirty work.
3. If the implementation is present: review it against this doc, run the
   verification commands, then go to step 5.
4. If absent: implement per the design map, add tests matching the existing
   `components::prompt::tests` style (paste summarize/expand, small inline,
   long single line, surrounding text, delete-into-placeholder, rendered style,
   reference token detection, line-range mapping), and run verification.
5. Create `feature/TUI-002-003-prompt-input-parity` from `development`, commit
   only the four code files plus the two board cards (and this handoff), push,
   and open a PR targeting `development` (never `main`).
6. Leave both cards in `qa`. Report the PR URL, the exact commands run, and any
   remaining gap. Do not merge to `main`; `development → main` is a separate
   explicitly-approved release.

## Completion

- PR [#211](https://github.com/cchris-p/opencode-modded-rust/pull/211) merged
  into `development` as `790532e`. Commit `dd0d73b` carried the implementation,
  the two `boards/qa` cards, and this handoff.
- Verified before merge: `cargo fmt --all`, `cargo test -p opencode-tui` (238
  passed), `cargo test -p opencode-config` (67 passed), `cargo clippy
  -p opencode-config -p opencode-tui --all-targets` (161 warnings before and
  after - no new ones), `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check
  -p opencode-config -p opencode-tui`.
- Both cards remain in `qa` with QA evidence and a merge-closeout note.
- Scope held to the main session prompt; the question tool paste path is
  unchanged apart from receiving raw text.

## Next Agent Instructions

**Current state:** TUI-002 and TUI-003 are merged into `development` (`790532e`)
and sit in `qa`; no code work remains on this handoff.
**Start here:** post-merge QA of the paste and `@` reference behavior on
`development`; then promote both cards to `done` if it passes.
**Verify:** `cargo test -p opencode-tui components::prompt`, `cargo test
-p opencode-config`, plus a manual paste of `>= 3` lines and an `@file` token in
`ort` (build first with `ort-build`).
**Known issues:** none from this change. Full `cargo check --workspace` is
blocked only by the unfetched `third_party/scopemux-core` environment gap.
**Gates:** post-merge QA/owner confirmation before moving the cards to `done`.
**Do not:** merge anything to `main`; widen styling to the question tool or
dialog inputs.
**References:** `boards/qa/Show a paste summary with line count in the prompt.md`,
`boards/qa/Style @file references in the prompt input.md`, this handoff.
