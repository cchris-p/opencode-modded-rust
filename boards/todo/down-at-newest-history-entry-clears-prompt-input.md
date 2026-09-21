---
id: "BUG-030"
title: "Down at the newest prompt-history entry clears the input box"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-21"
---

# Down at the newest prompt-history entry clears the input box

## Summary

Walking the prompt history back down to the newest entry empties the input box. When the pre-browse
draft is empty — the normal post-send or post-`Ctrl+C` state — pressing `Down` one or more times at
the bottom of the history walk replaces the box contents with `""` instead of leaving the box as the
user left it. This is the "down all the way" edge of the historical-prompt / `Ctrl+C` / `Up`+`Down`
feature.

## Problem

The history walk is a round trip: `Up` starts from the draft and climbs toward older messages, and
`Down` is supposed to walk back. Clearing at the bottom means a single extra `Down` press silently
discards the box contents and makes the input feel unreliable for a daily-driver workflow. This card
fixes the *bottom-of-walk* behavior; the trigger condition for bare arrows is `FEAT-037` and the
storage/clear model is `FEAT-036`.

## Current behavior (evaluated)

History is stored and recalled, and a non-empty draft *is* captured and restored correctly. The
defect is the empty-draft fallback.

- Bare `Up`/`Down` call `history_previous` / `history_next`
  (`crates/opencode-tui/src/components/prompt.rs:609-614`).
- The first `history_previous` captures the current box contents into `history_draft` and points
  `history_index` at the newest entry (`prompt.rs:784-805`).
- `history_next` walks toward newer entries; when there is no newer entry it sets
  `history_index = None` and restores `self.history_draft.take().unwrap_or_default()`
  (`prompt.rs:807-823`). When the captured draft is empty or absent, this yields `""`, so the box
  is cleared.
- An empty captured draft is common: the box is empty in the normal post-send / post-`Ctrl+C`
  state (`take_input` at `prompt.rs:645-667`, `clear` at `prompt.rs:669-678`).
- `Alt+Up` / `Alt+Down` dispatch to the same functions via `history_previous_entry` /
  `history_next_entry` (`crates/opencode-tui/src/app/app.rs:560-567`,
  `crates/opencode-tui/src/context/keybind.rs:168-169`), so the same fallback applies to the
  explicit recall bindings.
- Existing unit test `history_navigation_preserves_draft`
  (`crates/opencode-tui/src/components/prompt.rs:1747-1768`) covers only the non-empty-draft path,
  where the draft is restored. The empty-draft path is untested and is where the box empties.

Reproduction: send one prompt so history is non-empty, leave the box empty, press `Up` then `Down`.
The box ends empty; there is no pre-browse draft to restore, so `Down` has nothing safe to do and
should simply leave the newest entry visible.

## Reference behavior (vanilla opencode)

Vanilla opencode keeps a single history cursor and returns an empty prompt when the cursor reaches
the draft slot (`{ input: "", parts: [] }`), guarded by
`if (current.input !== input && input.length) return` in `move`
(`packages/tui/src/prompt/history.tsx`, frozen TS reference line). The guard means a fresh non-empty
draft is never clobbered by history movement. The Rust fork captures the draft instead, so the
bottom-of-walk case needs an explicit rule rather than inheriting vanilla's empty-on-return path.

## Decided behavior

This is the accepted resolution of the card's original open question: **`Down` at the newest entry
must never clear text the user can still see.**

- `Down` while a newer history entry exists → move to it (unchanged).
- `Down` at the newest entry with a captured **non-empty** pre-browse draft → restore that draft
  exactly, cursor at end, `history_index = None` (unchanged).
- `Down` at the newest entry with **no captured draft or an empty captured draft** → no-op: keep
  displaying the newest entry, do not write `""` into the box, and keep `history_index` at the
  newest index so repeated `Down` stays inert.
- To leave history and blank the box, the user uses `Ctrl+C` (`FEAT-006`) or types over the
  recalled entry; `Down` is no longer an implicit clear.
- `Alt+Down` follows the same rule, since it shares `history_next`.

## Implementation plan

All changes are local to the prompt component; no keybind, storage, or app changes.

1. In `history_next`
   (`crates/opencode-tui/src/components/prompt.rs:807-823`), replace the unconditional fallback
   branch:

   ```rust
   } else {
       self.history_index = None;
       self.input = self.history_draft.take().unwrap_or_default();
       self.cursor_position = self.input.len();
   }
   ```

   with a branch that only restores a meaningful draft:

   ```rust
   } else {
       match self.history_draft.take() {
           Some(draft) if !draft.is_empty() => {
               self.input = draft;
               self.cursor_position = self.input.len();
               self.history_index = None;
           }
           _ => {
               // No pre-browse draft to restore: keep the newest entry visible
               // instead of clearing the box. Stay on the newest index so further
               // `Down` presses are inert until the user edits, clears, or recalls.
               self.history_index = Some(idx);
           }
       }
   }
   ```

2. Keep `history_previous` (`prompt.rs:784-805`), `reset_history_cursor`
   (`prompt.rs:825-828`), `take_input`, and `clear` unchanged. The invariant is that `history_index`
   is `None` outside a walk, except for the inert "parked on newest entry" state introduced above,
   which any edit (`reset_history_cursor`) or clear immediately exits.
3. Add focused unit tests next to `history_navigation_preserves_draft`
   (`prompt.rs:1747`), using the existing `with_isolated_prompt` helper
   (`prompt.rs:1638-1657`).

## Tests

- `history_next_at_newest_without_draft_does_not_clear`: history `["alpha", "beta"]`, box empty,
  `history_previous_entry()` → `"beta"`, then `history_next_entry()` → input stays `"beta"` (not
  `""`); a second `history_next_entry()` still leaves `"beta"`.
- `history_navigation_preserves_draft` (existing, `prompt.rs:1747`): keep passing — non-empty
  captured draft still restored on return to bottom.
- `history_next_from_recall_back_to_draft_then_repeat_is_inert`: draft `"draft"`, walk up twice,
  walk down to `"draft"`, extra `history_next_entry()` leaves `"draft"`.
- `history_next_without_recall_is_noop`: box empty, no prior `Up`, `history_next_entry()` leaves the
  box unchanged.

## Non-goals

- Changing what history is stored, where, or how `Ctrl+C`-cleared drafts are captured (`FEAT-036`).
- The cursor-boundary trigger gate for bare `Up`/`Down` (`FEAT-037`).
- Adding a history picker, frecency changes, or vertical multi-line editor work.
- Changing `Ctrl+C` clear semantics (`FEAT-006`).
- Persisting the pre-browse draft across a `Ctrl+C` while browsing.

## Acceptance criteria

- [ ] `Down` at the newest history entry never introduces `""` into an otherwise non-empty box.
- [ ] With an empty pre-browse draft, `Down` at the newest entry leaves the newest entry visible
      and repeated `Down` presses are inert.
- [ ] A captured non-empty pre-browse draft is restored exactly, cursor at end.
- [ ] `Alt+Down` exhibits the same bottom-of-walk behavior as bare `Down`.
- [ ] Existing `history_navigation_preserves_draft` still passes.
- [ ] New tests cover the empty-draft, non-empty-draft, and repeat-`Down` cases.
- [ ] `cargo test -p opencode-tui --lib prompt -- --test-threads=1` passes for the new tests
      (existing unrelated failures excepted).

## Recommended verification

- `ort-build`, then `ort`; send one prompt so history has an entry.
- With an empty box, press `Up` then `Down`; confirm the newest entry remains and the box does not
  empty.
- Press `Down` again; confirm nothing changes. Press `Ctrl+C`; confirm the box clears.
- Type a draft, press `Up` a few times, then `Down` back to the bottom; confirm the draft returns and
  extra `Down` presses do not clear it.
- Repeat with `Alt+Up` / `Alt+Down`.
- `cargo test -p opencode-tui --lib prompt -- --test-threads=1`.

## Related Items

- `FEAT-037` Gate prompt history navigation to cursor boundaries - adjacent slice: it fixes the
  *trigger* condition for bare `Up`/`Down`; this card fixes what happens when `Down` runs out of
  newer entries.
- `FEAT-036` Persist and recall typed input-box messages after send or clear - parent
  historical-prompt feature; owns what is stored and how `Ctrl+C` interacts with history.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input clear
  semantics; `Ctrl+C` remains the way to blank the box.
- `BUG-018` Prompt input cursor renders out of place - same prompt component; keep cursor placement
  consistent when the box contents are restored.
- `BUG-013` Cursor on the input field needs to always be visible - cursor behavior in the prompt.
- `PHASE-001` V1 daily-driver hardening.

## Notes

- 2026-09-21: Evaluation confirmed the non-empty-draft path restores the draft
  (`history_navigation_preserves_draft`), so the reported "clearing" is the empty captured-draft
  fallback at `prompt.rs:819` (`unwrap_or_default()`). Reproduction: empty box, `Up` then `Down`
  ends empty.
- Decision recorded in "Decided behavior": the bottom of the history walk parks on the newest entry
  rather than clearing; `Ctrl+C` is the explicit way to return to a blank box. This diverges from
  vanilla's empty-on-return, which is guarded by vanilla's input-match check that the fork does not
  have.
- Keep the implementation to the single `history_next` branch plus tests; do not refactor the
  history model in this card.
