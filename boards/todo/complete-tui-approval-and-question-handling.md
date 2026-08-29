---
id: "START-018"
title: "Complete TUI approval and question handling"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "todo"
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
- Keep the work focused on the TUI interaction loop, not broader visual redesign.

## Done when

- The TUI can surface and answer runtime approval requests.
- The TUI can surface and answer runtime questions.
- The flow no longer depends on placeholder or stubbed behavior.

## Notes

- This item exists because TUI is part of V1, not as a parity exercise.
- Keep any server-side support changes minimal and directly tied to the TUI workflow.

## Related Items

- `START-004` Assess current Rust state
