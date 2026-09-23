---
id: "BUG-040"
title: "Grep tool blocks the async runtime and wedges the server and TUI"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-23"
---

# Grep tool blocks the async runtime and wedges the server and TUI

## Summary

A `grep` call could run a full filesystem walk synchronously on a tokio runtime
thread. On a large tree (for example a repo root containing a Rust `target/` with
hundreds of thousands of files) this starved the runtime: the server stopped
answering requests and the TUI appeared to freeze with the session "in progress".

## Evidence

Observed live on `2026-09-23`:

- The TUI on port `3187` was idle while its server `opencode serve --port 3187`
  (pid `94282`, workspace `/Users/cchrisleepyles/repos/opencode-modded-rust`) did
  not respond to `GET /session` (the sibling server on `3188` answered normally).
- `sample 94282` for 5s showed one tokio worker spending **2777 of 2976 samples**
  in a single stack:
  `accept_prompt → drain_session_queue → run_prompt_turn →
  SessionPrompt::prompt_with_update_hook → loop_inner → execute_tool_calls →
  ToolRegistry::execute → GrepTool::execute → std::fs::File::open → open()`.
- `/Users/cchrisleepyles/repos/opencode-modded-rust/target` held **243,614 files
  / 36 GB**; a repo-root grep walked all of it, opening and line-reading every
  file on the async runtime thread.
- The BUG-039 session (`ses_2d2c826f00c44632b69924e444ce534a`) issued a bare
  `grep {"pattern":"BUG-039"}` (no path) and is the corresponding freeze
  environment; see `opencode session inspect "Board item to PR and closeout on
  BUG-039 worktree"`.

## Root cause

`GrepTool::execute` (`crates/opencode-tool/src/grep_tool.rs`) performed
`walkdir::WalkDir` plus synchronous `File::open`/`BufRead::read_line` directly in
its `async fn`, and did not exclude build/vendor directories. Blocking the
runtime thread starved the server, so message persistence and HTTP responses
stalled and the TUI looked frozen.

## Fix

Implemented in `fix/tool: stop grep scans from stalling the session` (commit
`1fb80d0`):

- Run the walk/scan inside `tokio::task::spawn_blocking`.
- Skip `target/`, `node_modules/`, `.git/`, `.opencode/`, `dist/`, `build/`, and
  other build/vendor dirs by default (still searchable when passed as the
  explicit path).
- Bound the scan: 15s deadline, 20k-file cap, 8MB-per-file read cap, and a cap
  on stored matches; report when a budget stops a search.
- Stop following symlinks.
- Fix a potential panic when truncating a line at a non-char boundary.

## Verification

- Merged: the fix is integrated into `development` (merge `b76bb78`, unifying the
  BUG-027/028/029/038 stability cluster with this work); `cargo test -p
  opencode-tool` passes on the merged tree.
- Unit: `cargo test -p opencode-tool` (adds `skips_build_and_vendor_directories`
  and `reports_no_matches`).
- Manual: run grep at the repo root and confirm the turn completes, `target/`
  matches are absent, and the server stays responsive during the call.
- Regression: confirm a long grep no longer produces `STARVATION`/blocked-worker
  behavior in the TUI trace (`<data-local>/opencode/traces/tui.log`).

## Done when

- A repo-root grep returns without wedging the server or TUI.
- Matches under excluded build/vendor directories are not returned unless the
  path targets them explicitly.
- The scan is provably bounded (deadline/file/match caps surfaced in output).

## Related Items

- `BUG-038` DeepSeek reasoning turn stalls mid-turn (distinct: silent
  non-completion rather than runtime starvation).
- `BUG-027` Session keeps freezing during thinking mode.
