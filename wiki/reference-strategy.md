# Reference Strategy

## Role of the TypeScript repo

`opencode-modded` remains useful as:

- a planning workspace
- a source of workflow ideas
- a reference implementation for selected behavior
- a place to track deferred feature review and sync ideas

## Documentation convention

- Files in `opencode-modded` may be referenced by explicit path for history, planning context, or behavior reference.
- Normative Rust-product guidance must live in `scopemux-code`.
- Cross-repo references are for context and traceability, not shared ownership.

## What is not carried over automatically

- upstream bug-fix churn
- every architectural decision from OpenCode
- every auxiliary surface area unrelated to the first Rust product goal

## Current stance

- Draw the line at the current implementation for now.
- Finish the Rust product before considering future upstream feature review.
- If upstream later ships a major capability worth adopting, evaluate it as a dedicated board item.

## Frozen reference line

- The working TypeScript reference line is the local `opencode-modded` commit `e62912b5d18b73316c7bfd6e894b040698f6c880`.
- This freeze is for planning clarity, not as a permanent refusal to learn from later upstream work.
- Any later feature adoption should start with a new board item rather than silently moving the reference line.

## Future sync review concept

If sync review is revisited later, it should:

- inspect recent upstream changes
- identify feature-level candidates rather than bulk syncing
- convert worthwhile candidates into explicit board items
- ignore changes that do not improve product utility or local-model performance
