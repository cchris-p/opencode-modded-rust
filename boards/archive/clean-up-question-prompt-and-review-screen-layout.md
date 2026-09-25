---
id: "BUG-042"
title: "Clean up question prompt and review screen layout (wrapping, width, no inner scroll)"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/option-selection.md"
status: "todo"
created: "2026-09-25"
---

# Clean up question prompt and review screen layout (wrapping, width, no inner scroll)

## Summary

The question (QnA) prompt and its multi-question review/confirm screen render in a cramped box. The
box looks squashed, uses roughly half the available horizontal space, and text can appear to spill
past the defined border instead of wrapping cleanly inside it. This is a focused layout/width cleanup
of the question prompt and the review screen only.

Reported: "it is so squashed and text appears outside of the defined box."

## Reported Behavior

- The question prompt box and the review/confirm ("Submit answers" / "Go back") screen look
  vertically compressed and hard to read.
- Long question text, option labels/descriptions, and review summaries overrun the drawn border
  rather than wrapping inside it.
- The box uses about half the horizontal space even on wide terminals, so lines wrap earlier than
  necessary and the box feels narrow.
- Most visible with long question text, long descriptions, and the review summary on multi-question
  requests.

## Decisions (locked 2026-09-25)

Do not re-litigate these during implementation.

- **Scope is the question prompt and the review/confirm screen only.** The permission/confirm
  (`[y]/[n]/[a]/[p]`) prompt is out of scope for this card.
- **Wrapping inside the box is the primary fix.** No content may render past the border.
- **Prefer no inner scroll.** If wrap fits the content, do not introduce scrolling. The box should
  grow upward from its bottom anchor to fit wrapped content; only if content exceeds the terminal
  should any scroll/truncation fallback apply.
- **Use about two-thirds of the horizontal space**, not the current ~half / 80-column cap.
- **Keep the prompt bottom-anchored.** Do not convert it to a centered modal; the compact
  bottom-prompt placement (the documented `GATE-002` deviation) stays.
- **Remove the unused `ConfirmDialog`** (`crates/opencode-tui/src/components/dialogs/confirm.rs`)
  once confirmed unreferenced.

## Relationship to BUG-039

`BUG-039` (qa) added wrapping, tail-scroll, caret editing, and paste routing to the question prompt.
This card is a **separate follow-up** and must not re-open `BUG-039`'s caret/paste scope. Cover only
what `BUG-039` did not settle: horizontal width, the no-inner-scroll preference, and any residual
overflow on the question prompt and review screen.

## Prerequisite Verification

Before implementing, confirm which build shows the problem:

- Rebuild with `ort-build` and reproduce with `ort` so the running binary includes `BUG-039`.
- If content still renders outside the border with wrapping present, capture the exact question text,
  terminal size, and whether it is the prompt or the review screen. Record those details on this card
  so the fix targets the real path, not a stale binary.

## Evidence / Investigation Starting Points

- `crates/opencode-tui/src/components/question.rs`
  - `render` (`:443-606`): wrapping and tail-scroll landed under `BUG-039`.
  - Width is `area.width.saturating_sub(2).min(80)` (`:544`) and the popup is left/bottom anchored
    (`:562-567`). The 80-column cap is the direct cause of the ~half-width look on wide terminals.
- Review/confirm screen: the multi-question review reuses the same `question.rs` renderer via
  `QuestionType::Review` (`crates/opencode-tui/src/app/app.rs`), so a renderer fix covers both.
- `crates/opencode-tui/src/components/dialogs/confirm.rs`: standalone fixed 50x7 centered dialog
  (`:66-67`) with an unwrapped centered message (`:86-95`). A workspace-wide search confirms the
  only references are its own re-exports at `crates/opencode-tui/src/components/dialogs/mod.rs:71`
  and `crates/opencode-tui/src/components/mod.rs:26`; it is never constructed or rendered. Remove
  it (confirmed safe).
- Backdrop: `question_prompt.is_open` triggers a full-frame backdrop
  (`crates/opencode-tui/src/app/app.rs:4359-4361`, `:4418-4422`), so the box floats on an empty
  screen; keep in mind when judging density, but the backdrop itself is not in scope.

## Expected Behavior

- All question, option, description, and review text wraps within the box; nothing renders past the
  border at any supported terminal size.
- The box targets roughly two-thirds of the available width and is bottom-anchored.
- The box grows upward to fit wrapped content; inner scrolling is not used when wrap fits.
- Hint rows ("Up/Down to navigate...", "Enter to submit...") and the `>` input line remain visible
  and readable.
- The review/confirm screen receives the same width and wrapping behavior as the option screen.

## Scope

- Width: replace the `.min(80)` cap with a ~2/3-of-area width target, respecting viewport margins.
- Wrapping: guarantee containment for question text, option labels/descriptions, input line, and
  review summary.
- Height: size from wrapped content and grow upward; avoid inner scroll unless content exceeds the
  terminal.
- Remove the unused `ConfirmDialog` and its re-exports.
- Update/extend focused render tests.

## Non-goals

- The permission/`[y]/[n]/[a]/[p]` confirm prompt (`components/permission.rs`).
- `BUG-039`'s caret-editing and paste-routing behavior; do not regress or re-litigate it.
- Converting the prompt to a centered modal or changing the compact bottom-prompt placement.
- Question/answer protocol, schema, or tool-contract changes.
- Multi-question sequencing and review semantics.

## Acceptance Criteria

- No rendered content appears outside the box border for long question text, long option
  descriptions, or a long review summary.
- On a wide terminal the box occupies about two-thirds of the horizontal space (not ~half, not
  capped at 80 columns).
- The box remains bottom-anchored and grows upward to fit wrapped content; no inner scroll is
  introduced when content fits.
- The review/confirm screen wraps and is sized the same way as the question prompt.
- `ConfirmDialog` is removed and no references remain.
- Focused `TestBackend` render tests assert containment and width at a short and a wide terminal.
- `cargo fmt`, `cargo check -p opencode-tui`, and `cargo test -p opencode-tui` pass.

## Likely Touchpoints

- `crates/opencode-tui/src/components/question.rs` — width target, wrapped sizing, remove
  unnecessary scroll when wrap fits.
- `crates/opencode-tui/src/components/dialogs/confirm.rs` — delete.
- `crates/opencode-tui/src/components/dialogs/mod.rs` — drop the `confirm` module and re-export.
- `crates/opencode-tui/src/components/mod.rs` — drop the `ConfirmDialog` re-export.
- `invariants/option-selection.md` — layout/interaction rules these prompts must satisfy.

## Open Questions

- Should the ~2/3 width have an absolute upper bound to avoid very long line lengths on ultra-wide
  terminals, or is two-thirds always acceptable?
