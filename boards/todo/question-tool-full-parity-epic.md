---
id: "FEAT-024"
title: "Question tool full parity epic"
priority: "P2"
type: "epic"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-18"
---

# Question tool full parity epic

## Summary

Bring the Rust product's question/Q&A feature to full core parity with vanilla OpenCode while preserving room for intentional UX improvements over vanilla behavior.

"Full parity" for this epic means the Rust product supports the same core user and model-facing capabilities as vanilla OpenCode's question feature: first-class question tool semantics, pending-question lifecycle, session-scoped APIs, visible interactive responses in the TUI and CLI/direct-run surfaces, rejection, multi-question answers, single/multiple selection, custom answers, and durable enough runtime integration that questions are normal session events rather than an ad hoc side channel.

Parity does not mean copying every vanilla UI detail. If a better daily-driver UX is identified, prefer the deliberate Rust UX as long as the core capability remains covered and the deviation is documented.

## Vanilla Reference Evidence

Frozen TypeScript reference line: `$HOME/repos/opencode-modded` at commit `e62912b5d18b73316c7bfd6e894b040698f6c880`.

- `packages/schema/src/question.ts` defines `QuestionV2` IDs, options, prompt/info schema, request/reply shapes, and `question.v2.asked|replied|rejected` events.
- `packages/core/src/question.ts` owns pending question requests per Location, stores deferred waiters, publishes ask/reply/reject events, lists pending requests, and rejects outstanding questions on shutdown/finalizer.
- `packages/core/src/tool/question.ts` registers the `question` tool, enforces permission action `question`, records the originating assistant message/tool call, calls `QuestionV2.ask`, and returns a model-facing summary of user answers.
- `packages/protocol/src/groups/question.ts` exposes pending-question list, session question list, session reply, and session reject routes.
- `packages/server/src/handlers/question.ts` validates request ownership by session before reply/reject and filters list results by session.
- `packages/tui/src/routes/session/question.tsx` implements a TUI question prompt with single-question fast reply, multi-question tab navigation, final confirm tab, option shortcuts, custom answers, multi-select, and reject handling.
- `packages/opencode/src/cli/cmd/run/footer.question.tsx` and `question.shared.ts` provide the direct-run CLI footer question flow using a shared state machine.
- `packages/app/src/pages/session/composer/session-question-dock.tsx` implements the web/app dock, cached in-progress answers, minimized/restore behavior, focus management, notifications/toasts, session-scoped reply/reject, custom answers, and multi-question progress.

## Current Rust Evidence

- `crates/opencode-tool/src/question.rs` has a `question` tool, but its schema is narrower than vanilla: no `custom` flag, headers are optional despite vanilla prompt requiring them, output flattens all answers into `Vec<String>` instead of preserving per-question answer arrays, and execution still contains direct stdin fallback behavior.
- `crates/opencode-server/src/routes.rs` wires session prompt question callbacks into global in-memory pending maps and exposes `/question`, `/question/{id}/reply`, and `/question/{id}/reject`.
- `crates/opencode-tui/src/api.rs` can list all questions, reply, and reject, but the API is not session-scoped like vanilla's `/api/session/:sessionID/question` routes.
- `crates/opencode-tui/src/app/app.rs` filters listed questions by active session and walks multi-question requests sequentially.
- `crates/opencode-tui/src/components/question.rs` renders a basic prompt with text, single-choice, and multiple-choice modes, but drops option descriptions, lacks custom-answer option semantics, lacks a tabbed/review flow, and uses letter shortcuts instead of vanilla's digit shortcuts.
- `boards/done/complete-tui-approval-and-question-handling.md` finished the first live TUI integration path; this epic is the broader parity pass, not a replacement for that completed work.

## Scope

- Align the Rust question schema and tool contract with vanilla core semantics.
- Preserve per-question answer arrays through tool output, callbacks, API payloads, and model-facing output.
- Add `custom` answer semantics compatible with vanilla: custom answers default on unless explicitly disabled.
- Preserve and render option descriptions.
- Make pending question list/reply/reject session-scoped at the API boundary, while keeping any internal implementation simple.
- Ensure pending questions are runtime/session events with ask, reply, and reject notifications where the Rust event model supports them.
- Ensure rejected questions unblock the waiting tool call and produce a clear model-visible rejection/error path.
- Bring the TUI flow to core parity for single-question fast path, multi-question navigation/review, multiple selection, custom answers, keyboard shortcuts, and reject/dismiss behavior.
- Evaluate whether a Rust-specific UX should intentionally replace any vanilla behavior, especially for compact terminal layout, step-by-step versus tabbed multi-question flow, and keyboard bindings.
- Cover non-TUI/direct-run or CLI task surfaces once those surfaces exist or are close enough to integrate.

## Non-Goals

- Blindly cloning the web app UI or every animation/detail from vanilla.
- Building a browser app question dock unless the Rust product later grows that app surface.
- Internationalization parity for question strings in this pass.
- Reworking the whole permission system beyond the `question` permission/action needed for this feature.
- Persisting question requests across server restarts unless a later refinement decides that is necessary for daily-driver reliability.

## Parity Gaps To Split Into Child Items

1. Schema and tool contract parity

   Align `QuestionInput`, question prompt structs, option structs, answer output, JSON schema, and model-facing output with vanilla's `QuestionV2.Prompt`, `Info`, `Reply`, and `toModelOutput` behavior.

2. Session-scoped question API parity

   Add routes equivalent to session list/reply/reject semantics and validate that a request belongs to the addressed session before accepting responses. Keep legacy/global endpoints only if there is a concrete local compatibility need.

3. Runtime event and lifecycle parity

   Publish or otherwise surface ask/reply/reject events consistently, clear pending requests on reply/reject/drop, reject waiters on shutdown where feasible, and ensure stale requests do not block a session forever.

4. TUI prompt UX parity and improvements

   Support option descriptions, digit shortcuts, single-question fast reply, multi-question navigation/review, multi-select, custom answers, reject, and submitting-state/error recovery. Decide whether Rust keeps the current sequential flow, adopts vanilla's tabbed confirm flow, or implements a better hybrid.

5. CLI/direct-run parity

   When the Rust direct-run/CLI task surface is ready, handle pending questions without requiring the full TUI. Reuse as much prompt-state logic as practical rather than duplicating divergent behavior.

6. Permission integration parity

   Decide whether the Rust product needs an explicit `question` permission action/rule, then wire it consistently with agent permissions and tool registration.

7. Verification fixtures

   Add focused tests or smoke fixtures for single select, multi select, custom answer, multi-question reply shape, reject/unblock, wrong-session reply rejection, and pending cleanup.

## Intentional Deviation Candidates

- Prefer a compact bottom-of-session terminal prompt over a larger modal if it keeps transcript visibility intact.
- Consider a hybrid multi-question flow: sequential by default for low cognitive overhead, with an explicit review/confirm screen before final submission when there is more than one question.
- Keep keybindings consistent with this Rust TUI's existing prompt conventions where they conflict with vanilla, but support digit shortcuts if they materially improve speed.
- Show the source tool/message context when available if that makes it clearer why the agent is asking.
- Make rejection language explicit enough for the model and user to understand that the question was dismissed, not answered with an empty selection.

## Acceptance Criteria

- A model can call `question` with one or more questions, options with descriptions, optional `multiple`, and optional custom-answer behavior.
- The user can answer each question with one or more labels or a custom answer where allowed.
- Replies preserve answer arrays in question order all the way back to the waiting tool call.
- The model receives a clear answer summary equivalent in usefulness to vanilla's `User has answered your questions: ...` output.
- A user can reject/dismiss a pending question and the session does not hang.
- Question requests are listed and answered through session-owned semantics; wrong-session replies are rejected.
- The TUI can handle single-question and multi-question prompts without dropping to stdin or another UI.
- Pending question state is cleaned up on reply, reject, waiter drop, or session/runtime shutdown where feasible.
- Any UX deviations from vanilla are explicitly documented in this card or child cards before implementation is considered complete.

## Likely Touchpoints

- `crates/opencode-tool/src/question.rs`
- `crates/opencode-session/src/prompt.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-tui/src/api.rs`
- `crates/opencode-tui/src/app/app.rs`
- `crates/opencode-tui/src/components/question.rs`
- CLI/direct-run code paths as they mature
- Permission/agent tool registration paths if a `question` permission action is adopted

## Verification

- `cargo fmt`
- `cargo check -p opencode-tool -p opencode-session -p opencode-server -p opencode-tui`
- Focused unit tests for schema parsing, tool output formatting, pending request ownership, reply/reject cleanup, and TUI question state transitions.
- Manual smoke: launch with `ort-build` then `ort`, trigger a single-select question, a multi-select question, a custom-answer question, a multi-question request, and a rejected question.

## Related Items

- `START-018` Complete TUI approval and question handling
- `START-008` Full parity deferred
- `FEAT-012` CLI AgentExecutor tool loop parity
- `FEAT-021` Queue CLI task sends while TUI session is open
- `PHASE-002` Transport/runtime parity foundation

## Refinement Questions

- Should Rust keep the existing sequential multi-question flow, switch to vanilla's tabbed confirm flow, or use a hybrid with a final review screen?
- Should `question` have a dedicated permission rule/action in Rust, or should it be treated as always allowed for normal agent flows?
- Should pending questions survive TUI detach/reattach only within the same live server, or should they be persisted across server restart?
- Should CLI/direct-run support land in this epic's first implementation wave or wait until the task/direct-run surface is already in place?

## Done When

- Child items exist for the implementation slices above or the epic itself has been refined into an implementation-ready handoff.
- The Rust product covers the core vanilla question feature semantics across runtime, server API, TUI, and relevant CLI surfaces.
- Any preferred UX deviations are documented as deliberate product choices, not accidental parity gaps.
