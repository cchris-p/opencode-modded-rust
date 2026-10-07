---
id: "FEAT-067"
title: "Show workspace path and current git branch in the bottom-left footer"
priority: "P3"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
created: "2026-10-07"
---

# Show workspace path and current git branch in the bottom-left footer

## Summary

Show the session's workspace path and its currently checked-out git branch in the
bottom-left footer, formatted as `<path with ~ as home>:<branch>` (for example
`~/standards/vendor/opencode-modded-rust:development`). The bottom-left of both
the home screen and the session view currently prints the raw absolute workspace
directory only; add the home-relative shortening and the branch suffix.

## Why this exists

Both footers render the raw `AppContext::directory` value on the left:

- Home: `crates/opencode-tui/src/components/home.rs:203` (`render_footer`, left span).
- Session: `crates/opencode-tui/src/components/session.rs:556` / `:638`
  (`render_session_footer`, left span).

The value is the absolute `std::env::current_dir()` string set in
`crates/opencode-tui/src/app/app.rs:129-138`. It is not shortened relative to the
user's home directory, and nothing in the TUI reports the git branch. The user
wants both, in the compact `path:branch` form most other coding TUIs use.

## Desired behavior

- Bottom-left footer text reads `<path>:<branch>`.
- `<path>` is the workspace directory with a leading `$HOME` replaced by `~`
  (`/Users/me/proj` -> `~/proj`; the home directory itself -> `~`). Non-home
  absolute paths are unchanged.
- `<branch>` is the workspace's currently checked-out branch
  (`git branch --show-current`).
- When the workspace is not a git repository, or HEAD is detached (no current
  branch), the suffix is omitted and only the path is shown (no trailing `:`).
- Applies to both the home/landing footer and the session footer's left label.

## Scope

- Add a small pure helper (in `crates/opencode-tui/src/ui/text.rs`) that:
  - abbreviates a leading home-directory prefix to `~`, and
  - assembles the `<path>:<branch>` label, omitting the suffix when the branch is
    absent/empty.
- Track the workspace branch in `AppContext` (new `git_branch: RwLock<Option<String>>`),
  computed at startup in `App::new` via the existing
  `opencode_util::git::get_current_branch` (`crates/opencode-util/src/util.rs:217`),
  and refreshed on the existing 5-second aux-sync tick in
  `crates/opencode-tui/src/app/app.rs:969`.
- Use the helper for the left label in both `home.rs::render_footer` and
  `session.rs::render_session_footer`, preserving the existing right-aligned
  content and padding math.

## Non-goals

- Changing any right-side footer content (MCP/LSP counts, permissions, version).
- Adding branch to the prompt info line, header, or sidebar.
- Adding a new config toggle or command; this is always shown.
- Detecting or displaying dirty/ahead/behind state, or detached-HEAD commit IDs.
- Reworking git handling elsewhere in the product.

## Likely touchpoints

- `crates/opencode-tui/src/ui/text.rs` (new `abbreviate_home` / `workspace_location_label` + tests)
- `crates/opencode-tui/src/context/app_context.rs` (new `git_branch` field)
- `crates/opencode-tui/src/app/app.rs` (seed at startup, refresh on aux-sync tick)
- `crates/opencode-tui/src/components/home.rs` (`render_footer` left label)
- `crates/opencode-tui/src/components/session.rs` (`render_session_footer` left label)

## Done when

- The home footer bottom-left shows `~/<relpath>:<branch>` for a git workspace.
- The session footer bottom-left shows the same label.
- A non-git workspace shows only the shortened path.
- A detached-HEAD workspace shows only the shortened path.
- The label updates within ~5s after the branch changes.

## Recommended verification

- `cargo check -p opencode-tui`.
- `cargo test -p opencode-tui` including new unit tests for home abbreviation and
  the `path:branch` assembly (with and without a branch).
- `ort-build` then `ort` from a git worktree: confirm the bottom-left shows
  `~/...:development`; `git checkout -b test-branch` and confirm it updates
  within ~5s; run `ort` from a non-git directory and confirm the path-only form.

## Related Items

- `FEAT-052` Default home screen tips to hidden — same home/session footer surface.
- `FEAT-015` Preserve manually selected model and provider across sessions — status/settings surface parity reference.

## Implementation Notes

- Added `abbreviate_home` and `workspace_location_label` to
  `crates/opencode-tui/src/ui/text.rs`, with unit tests covering nested home
  paths, the home directory itself, unrelated paths, and branch omission for
  `None`/empty/whitespace branches.
- Added `git_branch: RwLock<Option<String>>` to `AppContext`
  (`crates/opencode-tui/src/context/app_context.rs:108`).
- `App::new` seeds the branch via a new `detect_git_branch` helper
  (`crates/opencode-tui/src/app/app.rs`), which wraps
  `opencode_util::git::get_current_branch` and normalizes empty output to
  `None`. The existing 5-second aux-sync tick refreshes it so a branch switch is
  reflected without a restart.
- Both footers now render `workspace_location_label(...)` as the left label:
  `home.rs::render_footer` and `session.rs::render_session_footer`; right-aligned
  content and padding math are unchanged.

### Verification

- `cargo check -p opencode-tui` clean.
- `cargo test -p opencode-tui -- --test-threads=1` -> 179 passed (single-threaded;
  the two `components::prompt` tests are order/env-sensitive and pass in isolation).
- Interactive `ort` confirmation (path shown as `~/...`, branch updates after
  `git checkout -b`) is pending the user's local run.
