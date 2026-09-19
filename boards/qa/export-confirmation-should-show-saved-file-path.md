---
id: "BUG-017"
title: "Export confirmation should show saved file path"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-09-19"
---

# Export confirmation should show saved file path

## Summary

After using the export command, the completion/confirmation menu does not display the title or path of the file that was saved.

The user should be able to tell which export file was created without leaving the TUI or guessing from the command result.

## Reported behavior

- The export command completes and shows a confirmation menu.
- The confirmation does not include the exported file's title/name.
- The preferred display may be the saved file path, but long paths could be visually noisy.

## Desired outcome

Make the export completion/confirmation UI identify the saved file clearly enough for the user to trust what happened and find the file afterward.

## Scope

- Inspect the existing export command and confirmation/menu rendering path.
- Decide what saved-file identifier should be displayed in the confirmation state.
- Prefer showing the saved filepath if it fits the existing UI and is useful.
- If full paths are too long for the confirmation surface, choose a compact display such as filename plus parent directory, relative path from workspace, or truncated path with the full path available somewhere nearby.
- Preserve existing export behavior and saved-file location.

## Non-goals

- Redesigning the whole export workflow.
- Changing the export file naming convention unless investigation shows the confirmation cannot identify files reliably with the current naming.
- Adding a file browser or preview workflow.

## Acceptance Criteria

- After a successful export, the confirmation/menu displays the saved file identity.
- The displayed identity is useful when multiple exports exist or were created in quick succession.
- Long paths do not break or dominate the TUI layout.
- Failed exports still show an appropriate error and do not imply a file was saved.

## Refinement Questions

- Should the confirmation show the full absolute filepath, a workspace-relative path, or filename plus parent directory?
- If the path is longer than the available width, should it truncate from the middle, from the start, or wrap?
- Should the confirmation include an action to copy the path, open the file, or just display it?
- Does the export command save outside the current workspace in any normal flow, making absolute paths more important?

## Likely Touchpoints

- TUI export command handling.
- TUI confirmation/menu/dialog rendering.
- Existing transcript export path/name generation.

## Verification

- Run `cargo check -p opencode-tui` or the narrower affected crate check discovered during implementation.
- Manual smoke with `ort-build` then `ort`: export a transcript and confirm the success UI identifies the saved file.
- Repeat with a long workspace/path or long session title to confirm the display remains readable.

## Implementation Notes

- The export success alert now displays the saved path on its own line instead of embedding the raw absolute path inline.
- Paths under the current working directory are displayed relative to that directory, keeping normal workspace exports compact while still identifying the saved file.
- Exports outside the current working directory keep the full absolute path so the destination remains unambiguous.
- Added focused tests for relative-path and outside-base display behavior.

## Verification Results

- `cargo check -p opencode-tui` passed.
- `cargo test -p opencode-tui export_path_display` passed.
- An initial concurrent test/check run timed out while competing for Cargo locks; the same focused tests passed when rerun sequentially.

## Related Items

- `FEAT-001` Improve historical chat transcripts workflow
- `BUG-015` Session rename immediately refreshes current terminal surfaces
- `FEAT-025` Map Ctrl+R to rename
