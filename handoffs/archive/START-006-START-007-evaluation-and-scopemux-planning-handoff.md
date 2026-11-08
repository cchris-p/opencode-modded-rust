# START-006-START-007 Evaluation And ScopeMux Planning - Handoff

## Included Board Items
- `START-006` Define agent evaluation strategy
- `START-007` Plan ScopeMux integration

## Why This Composition
- These items are both forward-looking planning work that sit one layer downstream from the runtime-loop definition.
- They are cohesive enough to share one PR because both refine how the V1 architecture should evolve after the baseline runtime model is agreed.
- Grouping them avoids a pair of small adjacent strategy PRs while keeping the review surface focused on planning rather than product code changes.

## Dependency Order
- Explicit dependency: none written directly on the cards.
- Inferred prerequisite for the handoff: `START-004` and `START-005` should generally land first because both evaluation language and ScopeMux boundaries should be grounded in the assessed runtime model.
- Inferred internal ordering: `START-006` and `START-007` can be developed in either order inside the same PR, but both should reference the runtime-loop terminology if it exists.
- Inferred non-dependency: this handoff is independent of `START-015` and `FEAT-001`.

## PR Plan
| PR | Board Items | Branch | Why this grouping | Merge rule |
| --- | --- | --- | --- | --- |
| 1 | `START-006`, `START-007` | `feature/START-006-START-007-eval-scopemux-planning` | One PR is appropriate because both cards are small planning outputs on the same architecture layer and do not need separate QA environments. | Merge into `development` when both documents are complete, concrete, and aligned with the current runtime-loop model. |

## Merge Target
All implementation PRs in this handoff target `development`.
The purpose is to land dev work there so QA and testing happen on `development`.
This handoff does not define deployment to `main`.

## Merge Strategy
- This handoff is planned as one PR for a small tightly coupled planning group.
- Do not merge this PR before the `START-004` and `START-005` handoff if those upstream architecture decisions are still in flux.
- This PR may merge independently of the feature handoffs once the planning group is complete.

## QA Notes
- After merge to `development`, verify the evaluation strategy covers task selection, task success, review quality, regression detection, and non-speed signals for real personal-use workflows.
- Verify the ScopeMux plan defines what remains generic in V1, what is deferred to future ScopeMux support, and what abstraction boundary the runtime should preserve.
- Confirm both documents are concrete enough to drive follow-up implementation work without reopening basic scope questions.
- Confirm any new follow-up tasks are created explicitly rather than left implied.

## Branch Cleanup
- Delete local branch `feature/START-006-START-007-eval-scopemux-planning` after the PR is merged into `development`.
- Delete remote branch `feature/START-006-START-007-eval-scopemux-planning` after the PR is merged into `development`.
- Do not delete unrelated local or remote branches.

## Execution Sequence
1. Branch from `development` to `feature/START-006-START-007-eval-scopemux-planning`.
2. Review the current V1 docs and the runtime-loop handoff output if it has already landed.
3. Write the agent evaluation strategy document for `START-006`.
4. Write the ScopeMux integration-boundary plan for `START-007`.
5. Update both board items with summary links and any new follow-up references.
6. Open one PR referencing both `START-006` and `START-007`, target `development`, and enumerate both item IDs in the PR body.
7. Merge into `development` when ready, verify both planning outputs there, then delete the local and remote feature branch.

## Archived Outcome
- Archived on 2026-08-30 after handoff completion.
- `START-006` had already been completed before this handoff was resumed.
- The remaining `START-007` work landed in PR #14: https://github.com/cchris-p/opencode-modded-rust/pull/14
- Added `wiki/scopemux-integration-plan.md` and explicit follow-up card `START-025`.
