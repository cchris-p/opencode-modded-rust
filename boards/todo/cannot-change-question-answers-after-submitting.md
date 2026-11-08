---
id: "BUG-058"
title: "Cannot revise question answers: review only steps back one question, and submitted answers are final"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/option-selection.md"
status: "todo"
created: "2026-09-30"
---

# Cannot revise question answers: review only steps back one question, and submitted answers are final

## Summary

When the agent presents a multi-question `question` request, the user cannot go back and change an
answer they already gave. The review screen's only backward affordance is "Go back", which pops the
single most recent answer and re-opens only the last question
(`crates/opencode-tui/src/app/app.rs:3523-3530`); there is no way to jump to an earlier question and
change its answer, and after answers are submitted to the server the flow is discarded so nothing can
be revised at all (`crates/opencode-tui/src/app/app.rs:3573-3578`).

The same report also observed that the session keeps running when the user leaves a question
unanswered or quits out of it. That continuation contract is owned by `BUG-056` and is recorded here
only as related evidence, not as duplicate scope.

## Reported behavior

User report (live use, `ort` TUI):

> I cannot go back and change my answers after I provide my answers to questions. Also the session
> continues even when I quit out of questions AND when I haven't answered questions, it needs to stop,
> wait for my answers and pick back up with those answers considered.

Two related but distinct defects are reported:

- **A (this card, new):** after answering questions there is no way to return to an earlier question
  and change its answer. The only "back" path is one step, and only before submission.
- **B (tracked by `BUG-056`):** the run does not reliably park on an unanswered/abandoned question.

## Reproduction (defect A)

1. Trigger a `question` request with two or more questions (`ort` TUI).
2. Answer the first two questions so the flow reaches the review screen.
3. On the review screen choose "Go back".
4. Observe: only the final answer is discarded and the final question is re-opened. There is no way
   to navigate to the first question and change its answer.
5. Submit the answers; observe: the request resolves and no revision is possible afterward.

Single-question requests are worse: they fast-reply with no review screen at all
(`crates/opencode-tui/src/app/app.rs:3431-3435`), so the first activation submits immediately.

## Vanilla reference

Frozen reference line per `AGENTS.md`: `$HOME/repos/opencode-modded` `dev` at
`f54ce313b99a6661d7758ad042f7a6e05c8e0972`.

- `packages/tui/src/routes/session/question.tsx`: one tab per question plus a final confirm tab
  (`tabs = questions().length + 1`, line 23). `Tab`/`Shift+Tab` and `Left`/`Right` cycle through
  **all** tabs (`:232-247`), so the user can return to any earlier question and change its answer
  before submitting from the confirm tab.
- After the confirm tab submits, the request resolves and the run resumes; vanilla also has no
  post-submit revision. That part is a deliberate shared boundary, not a gap.

Parity requirement: while a question request is still pending, the user must be able to revisit any
previously answered question and change its answer, matching vanilla tab navigation. Post-submit is
out of scope (matches vanilla).

## Code evidence

- Review screen is built with only `Submit answers` and `Go back`
  (`crates/opencode-tui/src/app/app.rs:3468-3515`, options at `:3495-3506`).
- "Go back" handling pops exactly one answer and sets the index to the last question, then re-opens
  it: `crates/opencode-tui/src/app/app.rs:3523-3530`.
- Sequential flow advances one question at a time with no arbitrary navigation:
  `crates/opencode-tui/src/app/app.rs:3412-3436` and `:3517-3542`.
- Submit resolves the request and discards the flow (`pending_question_flow = None`), closing the
  prompt: `crates/opencode-tui/src/app/app.rs:3573-3578`.
- Single-question fast reply skips review entirely: `crates/opencode-tui/src/app/app.rs:3431-3435`.
- Reject path sends a rejection and closes (`reject_question_flow`):
  `crates/opencode-tui/src/app/app.rs:3600-3626`. On the server the ask resolves on reply/reject and
  the run resumes: `crates/opencode-server/src/routes.rs:4114-4171`.

## Required solution direction

- While a request is pending, allow the user to return to any already-answered question and change its
  answer. Replace the one-step "Go back" with a navigation model that can reach every step (e.g. a
  per-question tab/step index with `Tab`/`Shift+Tab` and/or `Left`/`Right`, mirroring the vanilla
  route), or make the review screen list each question as an editable entry.
- Rework the answer store so an edit replaces the answer at its original index instead of only
  pushing/popping the tail (`flow.answers` handling at `app.rs:3523-3542` must not assume the last
  answer is the one being changed).
- Preserve the existing submit/reject semantics and error recovery. Do not change the runtime
  suspend/continue contract here (that is `BUG-056`).
- Keep the surface compliant with `invariants/option-selection.md`: text-entry questions use focus
  selection, digits stay typeable, and the focused option is distinguishable with accurate hint text.

## Scope

- Multi-question review and navigation in `crates/opencode-tui/src/app/app.rs`
  (`open_question_review`, `advance_question_flow`, `open_current_question_step`) and any supporting
  state in `PendingQuestionFlow`.
- Header/render hints in `crates/opencode-tui/src/components/question.rs` for the new navigation.
- Tests asserting that an earlier answer can be changed and that the submitted payload reflects the
  edit.

## Non-goals

- The suspend-until-answered / no-continuation runtime contract and the quit-out observation
  (`BUG-056`).
- Question schema/tool contract (`FEAT-038`), runtime events/lifecycle (`FEAT-040`), or permissions
  (`FEAT-043`).
- Post-submit revision (matches vanilla; not a goal).
- CLI/direct-run question handling (`CLI-009`, on hold behind `GATE-002`).

## Done when

- With a multi-question request pending, the user can move to any question (including earlier ones)
  and change its answer before submitting.
- The submitted `answers` payload uses the edited value at the correct index, not a duplicated or
  reordered entry.
- Navigation and activation keys match the surface hints and `invariants/option-selection.md`.
- Regression tests cover editing an earlier answer and the resulting submit payload.
- `cargo test -p opencode-tui` passes, plus `cargo fmt --all -- --check` and `cargo clippy` for the
  touched crate.

## Recommended verification

- Unit: `QuestionPrompt`/flow tests that answer Q1 and Q2, navigate back to Q1, change it, advance to
  review, submit, and assert the payload has the changed Q1 answer at index 0.
- Live: `ort-build` then `ort`; trigger a two-question request and confirm answers for any question
  can be changed before submission.

## Related Items

- `BUG-056` Session continues instead of stopping when the question tool is presented (todo) -
  owns the suspend-until-answered / quit-out continuation contract reported alongside this defect.
- `FEAT-041` TUI question prompt UX parity (done) - introduced the sequential flow and review screen
  this card extends.
- `BUG-039` Question prompt and review screen layout (done) - review-screen layout, not navigation.
- `FEAT-038` Question schema and tool contract parity (qa) - the tool/callback contract this surface
  submits into.

## Notes

- 2026-09-30: Created from a live-use report. The revision gap (defect A) is the new, actionable
  defect and is scoped here. The "session continues on quit/unanswered" part (defect B) is tracked by
  `BUG-056`; this card cross-references it rather than duplicating it, and `BUG-056` was annotated
  with the quit-out observation.
- Design caution: replacing the one-step "Go back" changes `flow.answers` indexing assumptions in
  `advance_question_flow` (`app.rs:3523-3542`); treat the answer store as index-addressed, not
  append/pop-only.
