# Plan: BUG-020 — Timeline shows only user-sent prompts (full chain to done)

## Target board item

- Path: `boards/todo/timeline-shows-only-user-prompts.md`
- ID: `BUG-020` (P2, bug, area `BUG`, status `todo`)
- Spec: `wiki/v1.md`
- Lane order (`board.toml`): `todo → hold → refinement → doing → qa → done`
- Skills to execute, in order:
  1. `board-item-to-pr` (implement + PR, card → `qa`)
  2. `pr-merge-closeout` (merge PR, card → `done`, per explicit user direction)
  3. `git-branch-cleanup` scoped to the BUG-020 PR branch only

## Requested outcome (clarified)

Full chain for BUG-020: implement the filter, open a PR against `development`,
merge it, delete the merged branch (remote + local), and update the card. Final
lane is **`done`** (user explicitly chose completion; this is the explicit
direction `pr-merge-closeout` requires to promote past `qa` without a QA report).

## Clarifications already resolved

- No BUG-020 PR/branch existed when planning started; user confirmed they want the
  entire chain (implement → PR → merge → cleanup → done), not a merge of an existing PR.
- Final board lane: `done`.
- User's merge instruction is explicit approval to merge; QA-report absence does not
  block (per `pr-merge-closeout`).
- Merge method: repo allows merge/squash/rebase; recent PRs (#52, #56, #57) all used
  **merge commits**, so use `--merge`. `delete_branch_on_merge=false`, so delete explicitly.

## Preflight findings / current state

- `development` is the stable integration branch (all recent PRs base = `development`);
  remote default branch is `main`, but code work integrates via `development`.
- Local checkout: on `development`, clean except untracked `.opencode/plans/`.
- `gh pr list --state open` is empty; no `bug/BUG-020-*` branch locally or on origin.
- Card is in `boards/todo/`; no implementation commit exists (`git log --all --grep=BUG-020`
  only shows the card-creation commits).
- `development` and `origin/development` are in sync (`afda439`, PR #57 merged).
- `development` is not protected; direct post-merge board commits are allowed and match
  repo history (`0cc68de docs(boards): complete BUG-009 …`).
- Environment quirks:
  - Bare `bd` fails: `.python-version` names an uninstalled pyenv env
    (`opencode-modded-rust`). Workaround: `PYENV_VERSION=3.12.12 bd …`. Do not edit `.python-version`.
  - `ort` / `ort-build` are not on the non-interactive PATH, so agent-run manual TUI QA is
    not possible; automated tests are the agent-side verification.
- Unrelated stale remote branch `origin/bug/BUG-018-cursor-core-fix` (PR #52, already
  merged) exists. **Out of scope**: do not delete it unless the user separately asks.

## Readiness assessment

**Ready — no refinement needed, no human-intervention marker.** Clear goal, scope
boundaries, "Done when" list, verification recipe, and exact code evidence. Frontmatter
has no `attention:` and the body has no `[HUMAN]`.

## Confirmed code state (evidence)

- `/timeline` registered in `crates/opencode-tui/src/command.rs` (name line 216,
  `CommandAction::Timeline` line 223).
- Dispatch to `handle_open_timeline()` at `crates/opencode-tui/src/app/app.rs:1813`.
- `handle_open_timeline()` at `app.rs:2160`; lines **2168–2195** build a `TimelineEntry`
  for **every** message, mapping `User/Assistant/System` roles with no filter.
- Selection/jump path `app.rs:1598–1610`: on `Enter`, reads
  `timeline_dialog.selected_message_id()` and calls `session_view.scroll_to_message(&msg_id)`;
  preserving `message_id` keeps this working.
- `TimelineEntry { message_id, role, preview, timestamp }` and empty state
  (`"No messages in timeline"`) in `crates/opencode-tui/src/components/dialogs/timeline.rs`
  (struct lines 11–17; empty state lines 96–101; role icons 106–110).
- Message model: `crates/opencode-tui/src/context/session_context.rs`
  (`struct Message` line 21, `enum MessageRole` line 38+).
- `MessageRole::User` is only produced for real user prompts (agent
  `opencode-agent/src/message.rs:44`), so role-based filtering is the correct reading of
  "user-sent prompts".
- `handle_fork_session()` (`app.rs:2199+`) has the same all-message pattern but is **out of
  scope** for BUG-020; do not change it.

## Root cause

`handle_open_timeline` collects entries for all messages instead of only messages with
`role == MessageRole::User`, so assistant/system turns appear in `/timeline`.

## Fix approach

Keep the fix at entry construction in `app.rs` (per scope). Extract a small pure helper so
it is unit-testable without constructing a full `App`:

```rust
fn timeline_entries_from_messages(msgs: &[Message]) -> Vec<TimelineEntry> {
    msgs.iter()
        .filter(|m| m.role == MessageRole::User)
        .map(|m| TimelineEntry {
            message_id: m.id.clone(),
            role: "user".to_string(),
            preview: m.content.chars().take(60).collect::<String>().replace('\n', " "),
            timestamp: m.created_at.format("%H:%M:%S").to_string(),
        })
        .collect()
}
```

`handle_open_timeline` then becomes:

```rust
let entries = session_ctx
    .messages
    .get(&session_id)
    .map(|msgs| timeline_entries_from_messages(msgs))
    .unwrap_or_default();
```

Rationale: smallest correct change; filtering at entry construction as the card requires;
helper extraction matches the repo's testable-free-function convention
(`format_transcript_message`, `transcript_incompleteness_warning`,
`map_api_permission_request`, `export_path_display_from`). `message_id` is preserved, so
navigation still works; `timeline.rs` needs no change (existing empty state covers "no
user prompts"). No role toggles/filters added.

## Files expected to change

- `crates/opencode-tui/src/app/app.rs` — filter + helper + unit tests.
- `crates/opencode-tui/src/components/dialogs/timeline.rs` — only if testing proves needed (not expected).
- `boards/doing/timeline-shows-only-user-prompts.md` — relane/status + Dev Notes.
- `boards/qa/timeline-shows-only-user-prompts.md` — relane/status (via `bd`).
- `boards/done/timeline-shows-only-user-prompts.md` — final relane/status + completion note.
- `.opencode/plans/BUG-020-timeline-user-prompts.md` — this plan (untracked; not committed).

## Phase A — Implement + PR (`board-item-to-pr`)

1. Confirm board project (`board.toml`, `boards/` lanes); resolve card at
   `boards/todo/timeline-shows-only-user-prompts.md`.
2. Relane `todo → doing` **directly** (`git mv` + `status: "doing"`), matching repo history
   (e.g. commit `28a383c`). Do not use `bd --promote` here (one step from `todo` lands in
   `hold`).
3. Git prep from stable base:
   - `git fetch origin`
   - `git switch development && git pull --ff-only`
   - `git switch -c bug/BUG-020-timeline-user-prompts`
4. Implement the helper + filter in `app.rs`.
5. Add tests (see below).
6. Verify: fmt, focused + full TUI tests, check, clippy (see commands).
7. Add Dev Notes to the card (branch, what changed, decisions, verification results).
8. Commit the implementation:
   `fix(tui): show only user prompts in timeline (BUG-020)` (code, tests, doing relane).
9. Relane `doing → qa` with `PYENV_VERSION=3.12.12 bd --promote BUG-020` (promote fits
   this one-step move and updates `status`), then commit
   `docs(boards): move BUG-020 to qa for PR-branch verification`.
10. Push: `git push -u origin bug/BUG-020-timeline-user-prompts`.
11. Open PR (**only after confirming none exists**: `gh pr list --search "BUG-020"`):
    `gh pr create --base development --head bug/BUG-020-timeline-user-prompts
    --title "fix(tui): show only user prompts in timeline (BUG-020)" --body …`
    Body: implementation points, decisions, verification results, and card link.
12. Leave branch checked out; card is now `qa`.

## Phase B — Merge closeout (`pr-merge-closeout`)

13. Confirm PR open, base `development`, head branch, and `git status` clean.
14. Read the card and judge code/task completeness against its goal, scope, and "Done when"
    list. Confirm the PR delivers only the in-scope filter + tests and does not alter the
    fork dialog or add role controls. If scoped work is missing, stop and surface it.
15. Review the full PR diff from its true base: `gh pr diff <N>`.
16. Merge with a merge commit (matches repo history):
    `gh pr merge <N> --merge`
17. Return local checkout to stable and sync:
    - `git switch development`
    - `git pull --ff-only`
    - Confirm the merge commit / `fix(tui)…` commit is present on `development`.

## Phase C — Branch cleanup + board `done`

18. Delete the merged PR branch:
    - remote: `git push origin --delete bug/BUG-020-timeline-user-prompts`
    - local: `git branch -d bug/BUG-020-timeline-user-prompts` (use `-d`, not `-D`; if it
      refuses, stop and report)
    - Do **not** delete `origin/bug/BUG-018-cursor-core-fix` or any other branch.
19. Update the card and commit directly on `development` (matches `0cc68de` pattern):
    - Append a completion note: merged PR number, merge commit, branch deleted, and that
      completion was explicitly directed by the user.
    - Relane `qa → done` with `PYENV_VERSION=3.12.12 bd --promote BUG-020` (one step,
      `qa → done`), or `git mv` + `status: "done"` if `bd` is unavailable.
    - Commit `docs(boards): complete BUG-020 and move card to done (PR #<N>)` and
      `git push` to `origin/development`.
20. Handoffs: confirm no active handoff covers BUG-020
    (`handoffs/2026-09-15-ort-workspace-…`, `handoffs/2026-09-16-cli-task-targeting-handoff.md`).
    If one does, update it; otherwise no handoff change.

## Tests to add

In `crates/opencode-tui/src/app/app.rs` `#[cfg(test)] mod tests` (line 4733):

- Add a small message builder for explicit role/id/content (or `user_message`/`system_message`
  helpers alongside the existing `assistant_message` pattern).
- `timeline_entries_include_only_user_prompts`: build `[User, Assistant, System]`; assert one
  entry, `message_id` == user id, `role == "user"`, preview/timestamp populated.
- `timeline_entries_empty_when_no_user_prompts`: assistant/system only and `&[]` → empty
  (drives the dialog's existing empty state).
- Optional: assert `TimelineDialog::open(entries)` selects index 0 and
  `selected_message_id()` returns the user id (guards the Enter/jump path).

Do not add or alter fork-dialog tests.

## Verification commands

- `cargo fmt -p opencode-tui` then `git diff --check` (pre-commit hook also runs `cargo fmt --all`).
- `cargo test -p opencode-tui timeline_entries -- --test-threads=1`
- `cargo test -p opencode-tui -- --test-threads=1` (triage pre-existing failures; BUG-018 notes
  unrelated TUI test/fmt drift — do not fix opportunistically).
- `cargo check -p opencode-tui`
- `cargo clippy -p opencode-tui --all-targets`
- Manual TUI QA (optional, user-run, post-merge on `development`): `ort-build` then `ort`,
  open `/timeline` in a session with user prompt + assistant reply + system message, confirm
  only user prompts show, `Enter` navigates, empty state shows with no user prompts.

## Merge details

- PR base: `development`; head: `bug/BUG-020-timeline-user-prompts`.
- Method: merge commit (`gh pr merge <N> --merge`).
- Branch protection: none on `development`; no review gate.
- `delete_branch_on_merge=false` → delete remote + local explicitly after merge.
- Post-merge board completion commits land directly on `development` (repo history precedent).

## Acceptance criteria (from card)

- `/timeline` lists only prompts the user sent.
- Assistant and system messages no longer appear in the timeline.
- Selecting a listed prompt still navigates to that message.
- Timeline stays usable when a session has few or no user prompts.
- No new role toggles/filtering controls.

## Risks / rollback

- Very low risk: one filter + helper extraction; no dialog/rendering change. Rollback is
  reverting the `app.rs` change (pre-merge) or a revert PR (post-merge).
- `bd --promote` from `doing`/`qa` fits the lane order; if `bd` errors, fall back to
  `git mv` + `status` edit.
- Manual TUI QA can't run from the non-interactive agent shell; automated tests gate the
  merge, and the user can QA the merged `development` state afterward.
- Merge is irreversible-ish on the remote; the user has explicitly approved it.

## Open questions

- None blocking. Recorded assumptions: "user-sent prompts" == `MessageRole::User`; final
  lane `done` per explicit user direction; merge method = merge commit.
