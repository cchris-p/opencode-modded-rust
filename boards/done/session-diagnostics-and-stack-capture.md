---
id: "FEAT-061"
title: "On-demand session diagnostics and stack capture"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-09-23"
---

# On-demand session diagnostics and stack capture

## Summary

Turns the "debug a freeze by session name" workflow into first-class tooling:
CLI inspection, an in-TUI diagnostics section, and an on-demand native stack
capture command.

## What shipped

- **CLI (from BUG-040 work)** — `opencode session find "<name>"` (substring
  match on title/id/slug, table or JSON) and `opencode session inspect
  "<name|id>"` (resolves by name, prints status/timings, message and part
  counts, tokens, bounded per-message part previews, a stall verdict, the trace
  path, and `--json`/`--full` variants). Reuses `SessionRepository` /
  `MessageRepository`.
- **TUI** — the `/status` dialog now includes a **Session Diagnostics** section
  for the active session: id/title, running status, message/part counts, output
  tokens, last message role/time, a stall verdict derived from the in-memory
  session view, and the trace path.
- **Stack capture** — `opencode debug stacks --pid <PID>` (or `--port <PORT>`,
  resolved with `lsof`) captures native stacks with macOS `sample` (gdb on other
  unix) into `<data-local>/opencode/stacks/stacks-<pid>-<timestamp>.txt`. Use it
  against a wedged server process.

## Verification

- `cargo check -p opencode-tui -p opencode-cli` clean.
- `opencode debug stacks --pid <pid> --seconds 2` captured a real `sample`
  report for a test process.
- `opencode session find/inspect` exercised against the live product database.

## Related Items

- `BUG-040` Grep tool blocks the async runtime and wedges the server and TUI
- `FEAT-026` Add cross-session transcript inspection (agent/API read surface)
