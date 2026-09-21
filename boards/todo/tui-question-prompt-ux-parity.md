---
id: "FEAT-041"
title: "TUI question prompt UX parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# TUI question prompt UX parity

## Summary

Child of `GATE-002` (parity gap 4). Bring the TUI question prompt to core parity: render option
descriptions, support digit shortcuts, single-question fast reply, multi-question navigation plus a
review/confirm step, multi-select, custom answers, reject/dismiss, and submitting/error recovery.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 4.

## Problem

- `crates/opencode-tui/src/components/question.rs` renders only option labels and uses letter
  shortcuts (`:210-233`).
- `QuestionOption` has no `description` field (`:21-25`), and both construction sites drop the
  server-provided description (`crates/opencode-tui/src/app/app.rs:3043-3050`, `:3111-3114`).
- Text input appears only when a question has no options (`:142-144`); there is no custom-answer
  option for choice questions.
- Multi-question requests are walked sequentially with no review/confirm screen
  (`app.rs:3055-3121`); rejection is handled but there is no submitting state or retry affordance.

## Vanilla reference

`packages/tui/src/routes/session/question.tsx`: single-question fast reply, multi-question tab
navigation with a final confirm tab, digit option shortcuts, custom answers, multi-select, and reject.

## Scope / deliverables

- Add `description` to the TUI `QuestionOption` and render it under each label.
- Render a custom-answer affordance when the question's `custom` flag is enabled ("Type your own
  answer"); route typed input back as a label answer.
- Support digit shortcuts (`1`..`9`) for options while keeping navigation keys consistent with the
  Rust TUI.
- Implement multi-question flow with an explicit review/confirm screen before submission when there
  is more than one question.
- Add reject/dismiss and submitting/error-recovery states.
- Document approved UX deviations from vanilla in the card.

## Intentional deviation candidates (resolve during refinement)

- Compact bottom-of-session prompt instead of a large modal to keep the transcript visible.
- Sequential multi-question flow by default with a review screen, rather than vanilla's tabbed flow.
- Keep Rust TUI keybind conventions where they conflict with vanilla, while supporting digit keys.

## Non-goals

- Session-scoped API routing (`FEAT-039`).
- Runtime event names/cleanup (`FEAT-040`).
- CLI/direct-run question handling (`FEAT-042`).

## Acceptance criteria

- [ ] Option descriptions are visible in the TUI prompt.
- [ ] A single-question request can be answered without any multi-step navigation.
- [ ] Multi-question requests can be navigated and reviewed before final submission.
- [ ] Multi-select works and preserves selected labels per question.
- [ ] A custom answer can be entered when `custom` is enabled.
- [ ] A question can be rejected/dismissed without hanging the session.
- [ ] Any deviation from vanilla's TUI flow is documented in this card.
- [ ] TUI unit tests cover state transitions for single, multi, custom, and reject.

## Verification

- `cargo fmt --all`
- `cargo check -p opencode-tui`
- `cargo test -p opencode-tui --lib -- --test-threads=1`

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-038` question schema and tool contract parity.
- `FEAT-039` session-scoped question API parity.
- `FEAT-040` question runtime event and lifecycle parity.
- `START-018` complete TUI approval and question handling - first integration pass.
