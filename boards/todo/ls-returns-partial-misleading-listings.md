---
id: "BUG-007"
title: "ls tool returns partial/misleading directory listings"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-10"
---

# ls tool returns partial/misleading directory listings

## Summary

The `ls` tool produces incomplete and misleading output on real workspaces: it applies a global 100-file cap during a recursive walk and only renders directories that contain at least one listed file. On this repository (more than 100 files) the first `ls` listed only 5 of 18 crates; the model concluded the workspace was incomplete and fell back to `bash ls -la` to get the truth.

## Reported behavior

From session `ses_0ffc9b44f96944039f8621c4d980d11b` (exported `summarize-workspace-files.md`):

- First `ls` on the workspace root returned only `opencode-agent`, `opencode-grep`, `opencode-lsp`, `opencode-session`, `opencode-tool`, `opencode-util` under `crates/`, missing most crates.
- The model noted: "`ls` only showed a few crates ... the workspace appears incomplete/partially populated" and then used `bash ls -la` to recover the full listing.
- The user-visible result is a misleading tree, wasted turns, and lower trust in tool output.

## Root cause (evidence)

`crates/opencode-tool/src/ls.rs`:

- `LIMIT: usize = 100` caps collected **files** during the recursive `WalkDir` (`ls.rs:36`, `ls.rs:175-180`). The walk `break`s once 100 files are collected, which can happen before all top-level directories are visited.
- The directory tree is derived **only from the listed files** (`ls.rs:183-213`): directories are inserted per file parent, so any directory with no listed file is omitted entirely.
- Output reports `truncated`/`count` from the capped set (`ls.rs:283-293`), so the truncation is real but the top-level view is silently incomplete.

Also note: the reference OpenCode tool set has no `ls`/`list` tool (reference uses `glob`); `ls` is Rust-introduced, so there is no direct parity behavior to copy.

## Scope

- Make `ls` return a correct listing for the requested directory: never silently omit directories or misrepresent the tree because of a per-file cap.
- Decide the intended semantics: (a) a bounded top-level listing (children first, truncate within a directory rather than globally), or (b) align the tool with the reference `glob` behavior. Record the decision in the card before implementing.
- Report truncation accurately (which directory/level was truncated), not just a boolean.
- Include directories that contain no files.
- Keep the tool practical for large repos (avoid dumping a whole monorepo by default).

## Non-goals

- Broad tool-set parity with the reference.
- Changing tool names or removal of `ls` unless the semantics decision says so.

## Done when

- `ls <absolute-path>` shows a correct, complete top-level view of the directory (all immediate child directories, plus files), with truncation clearly scoped and reported.
- A directory with no files still appears.
- A regression test covers a tree with more than 100 files and asserts that all top-level children are present.

## Recommended verification

- `cargo test -p opencode-tool ls`
- Live: fresh server, prompt "list the top-level directories in this workspace" and confirm all crates appear without a `bash` fallback.

## Related Items

- `BUG-004` Coding sessions run as bare chat (this defect surfaces on the agentic path)
- `PHASE-001` V1 daily-driver hardening

## Notes

- Surfaced during BUG-006 QA; see `summarize-workspace-files.md`.
- Confirm the intended listing semantics against the product's narrow workflow before changing behavior.
