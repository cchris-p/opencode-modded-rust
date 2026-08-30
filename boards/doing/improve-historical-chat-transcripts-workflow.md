---
id: "FEAT-001"
title: "Improve historical chat transcripts workflow"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
created: "2026-08-29"
---

# Improve historical chat transcripts workflow

## Summary

Confirm how historical session transcript export works in the Rust product today, document where it diverges from the TypeScript reference, and bring the Rust TUI transcript export workflow up to parity for Markdown export options.

## Why this exists

The current Rust export flow appears simpler than the reference implementation and needs a documented comparison before follow-on work expands transcript fidelity further.

## Current Rust behavior

- TUI transcript export is built in `crates/opencode-tui/src/app/app.rs` by `build_session_transcript`.
- The current Rust TUI export writes Markdown.
- The export dialog already exposes toggles for thinking, tool details, and assistant metadata.
- Before this implementation, the Rust formatter still ignored those toggles and always emitted one fixed Markdown shape based on flattened message content.
- The CLI `export` path in `crates/opencode-cli/src/main.rs` already exports full session data as JSON and is separate from this Markdown transcript workflow.

## Reference behavior

- The reference TUI in `/Users/cchrisleepyles/repos/opencode-modded/packages/tui/src/routes/session/index.tsx` exports a Markdown transcript.
- The reference export flow prompts for selectable transcript options before export.
- The reference formatter in `/Users/cchrisleepyles/repos/opencode-modded/packages/tui/src/util/transcript.ts` supports these options:
- `thinking`
- `toolDetails`
- `assistantMetadata`

## Confirmed divergence

- Rust already had the option toggles in the dialog, so the remaining gap was formatter wiring rather than missing controls.
- Rust previously exported one fixed Markdown shape instead of the reference repo's option-driven Markdown output.

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

## Dev Notes

- Kept the implementation scoped to the Rust TUI transcript export path and left the CLI JSON export code untouched.
- Updated the transcript formatter to render from structured message parts instead of pre-flattened message content so the existing dialog toggles now control output.
- Wired the export dialog toggles through both save-to-file and dialog copy-to-clipboard actions.
- Mirrored the reference behavior by gating reasoning blocks, tool input/output blocks, and assistant header metadata independently.
- Intentional difference from the TypeScript reference: the Rust formatter still uses the already-available local model string in the assistant header instead of a provider display-name lookup helper.

## Verification

- `cargo check -p opencode-tui`
- `cargo test -p opencode-tui transcript_ -- --test-threads=1`
- `cargo test -p opencode-tui --lib -- --test-threads=1` still shows unrelated flaky prompt tests in this workspace, but the new transcript tests pass consistently.

## Branch

- `feature/FEAT-001-historical-chat-transcripts`
