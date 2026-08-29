---
id: "FEAT-001"
title: "Improve historical chat transcripts workflow"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-08-29"
---

# Improve historical chat transcripts workflow

## Summary

Confirm how historical session transcript export works in the Rust product today, document where it diverges from the TypeScript reference, and bring the Rust TUI transcript export workflow up to parity for Markdown export options.

## Why this exists

The current Rust export flow appears simpler than the reference implementation and needs a documented comparison before follow-on work expands transcript fidelity further.

## Current Rust behavior

- TUI transcript export is built in `crates/opencode-tui/src/app/app.rs` by `build_session_transcript`.
- The current Rust TUI export writes Markdown, but it has no export-time multiselect options.
- The current Rust transcript is built from TUI session state after message parts are flattened into a simplified local representation.
- That flattening currently includes visible text, reasoning, files, tool calls, and tool results, but it does not expose the richer stored session part taxonomy as export-time options.
- The CLI `export` path in `crates/opencode-cli/src/main.rs` already exports full session data as JSON and is separate from this Markdown transcript workflow.

## Reference behavior

- The reference TUI in `/Users/cchrisleepyles/repos/opencode-modded/packages/tui/src/routes/session/index.tsx` exports a Markdown transcript.
- The reference export flow prompts for selectable transcript options before export.
- The reference formatter in `/Users/cchrisleepyles/repos/opencode-modded/packages/tui/src/util/transcript.ts` supports these options:
- `thinking`
- `toolDetails`
- `assistantMetadata`

## Confirmed divergence

- Rust does not currently offer the reference repo's export-time option selection for transcript contents.
- Rust currently exports one fixed Markdown shape instead of the reference repo's option-driven Markdown output.

## Scope

- Confirm the current Rust transcript-export behavior against the current code before changing it.
- Preserve Markdown as the output format for this story.
- Add export-time options in the Rust TUI that match the reference behavior for:
- thinking
- tool details
- assistant metadata
- Make the Rust Markdown transcript output respond to those options.
- Document any intentional remaining differences from the reference implementation inside the card's Dev Notes during implementation.

## Non-goals

- Changing the CLI JSON session export format.
- Designing a new transcript format other than Markdown.
- Adding extra Rust-only fine-grained part selection in this story.
- Solving every possible model-specific formatting difference beyond what is needed for parity with the reference options.

## Done when

- The card documents how Rust transcript export currently works and how it differs from the reference repo.
- Exporting a session transcript from the Rust TUI still produces Markdown.
- The Rust export workflow offers selectable options matching the reference behavior for thinking, tool details, and assistant metadata.
- The exported Markdown transcript changes appropriately when those options are toggled.
- The implementation clearly separates this Markdown transcript work from the existing CLI JSON session export path.

## Recommended verification

- Export the same session multiple times with different option combinations and confirm the Markdown output changes as expected.
- Verify reasoning content only appears when the thinking option is enabled.
- Verify tool input/output details only appear when the tool-details option is enabled.
- Verify assistant headers include or omit assistant metadata based on the selected option.
- Confirm the CLI JSON export path remains unchanged.

## Notes

- If extra Rust-only transcript granularity still looks useful after parity lands, track it as a separate follow-up board item rather than expanding this story.
