# START-015 OpenAI Auth Settings - Handoff

## Included Board Items
- `START-015` Mirror OpenAI auth configuration in settings

## Why This Composition
- This is a concrete product feature with a distinct TUI and auth integration surface.
- It should remain its own PR because it touches settings UX, auth persistence, and provider wiring, all of which deserve focused code review and QA.

## Dependency Order
- Explicit upstream context: this card is a follow-up to completed `START-009` and `START-012`.
- No unmet explicit backlog prerequisite remains.
- Inferred implementation dependency: reuse the existing persisted auth store and server/plugin auth routes rather than creating a parallel auth system.
- This handoff is independent of the planning handoffs for merge purposes.

## PR Plan
| PR | Board Items | Branch | Why this grouping | Merge rule |
| --- | --- | --- | --- | --- |
| 1 | `START-015` | `feature/START-015-openai-auth-settings` | One feature PR keeps the OpenAI auth path coherent and testable without pulling in unrelated work. | Merge into `development` when API-key flow works, login flow is implemented or explicitly split into follow-up work, and persistence uses the shared auth store. |

## Merge Target
All implementation PRs in this handoff target `development`.
The purpose is to land dev work there so QA and testing happen on `development`.
This handoff does not define deployment to `main`.

## Merge Strategy
- This PR should merge separately.
- It does not need to wait for the planning handoffs because its prerequisite provider settings surface already exists.
- It should not batch-merge with unrelated TUI work unless implementation proves another PR is inseparable.

## QA Notes
- After merge to `development`, verify the TUI exposes a dedicated OpenAI auth path from `Settings > Provider`.
- Verify API key entry persists and is reused by the OpenAI provider path.
- Verify persisted auth is visible through the shared auth state used by existing CLI or server auth commands.
- If login support is implemented, verify the end-to-end settings login flow stores usable OpenAI auth.
- If lower-level auth plumbing still blocks parity, verify the gap is documented explicitly and tracked by a follow-up board item.

## Branch Cleanup
- Delete local branch `feature/START-015-openai-auth-settings` after the PR is merged into `development`.
- Delete remote branch `feature/START-015-openai-auth-settings` after the PR is merged into `development`.
- Do not delete unrelated local or remote branches.

## Execution Sequence
1. Branch from `development` to `feature/START-015-openai-auth-settings`.
2. Review the reference OpenAI settings flow in `/Users/cchrisleepyles/repos/opencode-modded` and the current Rust auth plumbing.
3. Implement the settings-time OpenAI API key flow and the best-available mirrored login path using existing auth mechanisms.
4. Wire persisted auth into the shared store and provider flow already used by the app.
5. Document any intentional deviations or newly discovered lower-level auth gaps on the board item and in the PR.
6. Open a PR referencing `START-015`, target `development`, and keep the PR body specific about any remaining auth limitations.
7. Merge into `development` when ready, verify the feature there, then delete the local and remote feature branch.
