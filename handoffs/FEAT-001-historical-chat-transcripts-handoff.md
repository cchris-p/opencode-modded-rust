# FEAT-001 Historical Chat Transcripts - Handoff

## Included Board Items
- `FEAT-001` Improve historical chat transcripts workflow

## Why This Composition
- This is a self-contained TUI feature centered on transcript export behavior.
- It should remain its own PR because it has a distinct user-facing QA surface and should not be bundled with auth or planning work.

## Dependency Order
- Explicit dependency: none written directly on the card.
- Inferred prerequisite: none within the current backlog, because the existing Rust TUI transcript-export path already exists.
- Inferred implementation boundary: the CLI JSON export path must remain unchanged while the TUI Markdown export path gains selectable options.

## PR Plan
| PR | Board Items | Branch | Why this grouping | Merge rule |
| --- | --- | --- | --- | --- |
| 1 | `FEAT-001` | `feature/FEAT-001-historical-chat-transcripts` | One focused feature PR keeps transcript export review and QA isolated to a single workflow. | Merge into `development` when TUI export options exist for thinking, tool details, and assistant metadata, and the CLI JSON export path remains unchanged. |

## Merge Target
All implementation PRs in this handoff target `development`.
The purpose is to land dev work there so QA and testing happen on `development`.
This handoff does not define deployment to `main`.

## Merge Strategy
- This PR may merge independently.
- It should merge separately from `START-015` because the features have different codepaths and QA needs.
- It does not require batch merge with either planning handoff.

## QA Notes
- After merge to `development`, export the same session multiple times with different option combinations and verify the Markdown output changes appropriately.
- Verify reasoning content appears only when the thinking option is enabled.
- Verify tool input/output details appear only when the tool-details option is enabled.
- Verify assistant headers include or omit metadata based on the assistant-metadata option.
- Confirm the CLI JSON export path remains unchanged.

## Branch Cleanup
- Delete local branch `feature/FEAT-001-historical-chat-transcripts` after the PR is merged into `development`.
- Delete remote branch `feature/FEAT-001-historical-chat-transcripts` after the PR is merged into `development`.
- Do not delete unrelated local or remote branches.

## Execution Sequence
1. Branch from `development` to `feature/FEAT-001-historical-chat-transcripts`.
2. Confirm the current Rust TUI transcript-export behavior against the code and compare it with the reference repo's option-driven Markdown export.
3. Add export-time options for thinking, tool details, and assistant metadata in the Rust TUI.
4. Update the Markdown transcript formatter so output responds to those options while leaving CLI JSON export untouched.
5. Document any intentional remaining differences from the reference implementation in the board item's dev notes and the PR description.
6. Open a PR referencing `FEAT-001` and target `development`.
7. Merge into `development` when ready, verify the export behavior there, then delete the local and remote feature branch.
