---
id: "START-018"
title: "Complete TUI approval and question handling"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "doing"
created: "2026-08-29"
---

# Complete TUI approval and question handling

## Summary

Finish the TUI approval and question-response flow so the interactive V1 workflow can handle runtime permission checks and follow-up prompts without placeholder behavior.

## Why this exists

`START-004` found that the TUI already provides a strong interaction foundation, but approval and question handling still contain visible TODO-level gaps in the app and API layers.

## Scope

- Implement the missing TUI path for answering runtime questions and approvals.
- Complete any required TUI API client support for those flows.
- Align the TUI behavior with the V1 runtime-loop expectation that approvals and follow-up questions are runtime stage events, not ad hoc side channels.
- Keep the work focused on the TUI interaction loop, not broader visual redesign.

## Non-goals

- Broad TUI redesign outside the approval and question flow.
- Reworking the core runtime stage model beyond what is necessary to surface and answer current approval or question events.
- Expanding provider auth or other unrelated modal flows.

## Acceptance Detail

- The TUI can list and surface pending runtime approval requests in a user-respondable flow.
- The TUI can list and surface pending runtime questions in a user-respondable flow.
- A user response is sent through the current server API path instead of placeholder or stubbed local behavior.
- Rejecting or dismissing a question or approval uses the existing server-side interaction model where supported, rather than leaving the request stuck pending.
- The visible approval and question flow works as part of the active session experience and does not require dropping to CLI-only handling.
- The implementation keeps the interaction focused on current runtime events instead of inventing a second side-channel workflow.

## Likely Touchpoints

- `crates/opencode-tui/src/app/app.rs` for the visible approval or question interaction flow
- `crates/opencode-tui/src/api.rs` for client support that is currently missing or incomplete
- `crates/opencode-server/src/routes.rs` only if a minimal server adjustment is required to complete the existing interaction contract
- existing question and approval related TUI components if the current placeholders should be wired rather than replaced

## Verification

- `cargo test -p opencode-server`
- `cargo check -p opencode-server -p opencode-tui`
- `cargo test -p opencode-tui app::app::tests::permission_requests_prefer_explicit_pattern_for_resource`
- `cargo test -p opencode-tui app::app::tests::permission_requests_fall_back_to_metadata_path`
- `cargo test -p opencode-tui -p opencode-server` currently hits an unrelated existing failure in `components::prompt::tests::tab_autocomplete_uses_first_candidate`

## Done when

- The TUI can surface and answer runtime approval requests.
- The TUI can surface and answer runtime questions.
- The flow no longer depends on placeholder or stubbed behavior.

## Notes

- This item exists because TUI is part of V1, not as a parity exercise.
- Keep any server-side support changes minimal and directly tied to the TUI workflow.
- Assessment evidence for the gap is recorded in `wiki/current-rust-state.md`.
- Runtime-loop expectations for this interaction live in `wiki/v1-runtime-loop.md`.

## Dev Notes

- Wired server-side tool permission and question callbacks into `SessionPrompt` so live runtime requests are exposed through the existing HTTP routes instead of failing or staying invisible to the TUI.
- Expanded the question API payload to preserve per-question headers, options, and `multiple` semantics, then updated the TUI client to fetch and reply through those routes.
- Replaced the inline TUI TODO placeholders with real approval and question reply handling, including sequential multi-question progression for one pending request.
- Synced pending permission and question state during session refresh so the active session view can surface live runtime interactions without dropping to CLI handling.

## Branch

- `feature/START-018-tui-approval-question-handling`

## Related Items

- `START-004` Assess current Rust state
