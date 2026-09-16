---
id: "FEAT-005"
title: "Copy Cline-style CLI task send conventions"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-08"
---

# Copy Cline-style CLI task send conventions

## Summary

Add a CLI-first task interaction surface inspired by Cline's `task` and top-level prompt flows, focused on starting tasks and sending follow-up chat messages outside the TUI with file attachments.

## Why this exists

Cline exposes useful command-line conventions for quick coding-assistant interactions without forcing the user through an interactive TUI. The useful part for this product is not broad Cline parity; it is the ability to start or continue chat/task work from the shell, attach relevant files, and get usable output from a command-oriented workflow.

Background-session lifecycle and attach/detach policy are intentionally handled by separate board items. This card owns the CLI send/task conventions layered on top of whatever session/server lifecycle the product supports.

## Cline Behaviors To Consider

- `cline "prompt"` starts a new task from a direct prompt.
- Piped stdin can provide the prompt text.
- `-f` / `--file` attaches files to the task.
- `cline task new` creates a new task.
- `cline task send` sends a follow-up message to the current task and can update task mode or approval state.
- `cline task chat` provides interactive chat with the current task.
- Output can be formatted for rich, plain, or JSON consumers.

## Scope

- Define and implement the product's CLI command shape for starting a task/session from a prompt outside the TUI.
- Define and implement the command shape for sending a follow-up message to an existing or current task/session outside the TUI.
- Support prompt text from command arguments and stdin where that fits the existing CLI conventions.
- Support attaching one or more files to a new task or follow-up message.
- Define how the command chooses the current or target task/session without reintroducing unsafe implicit server reuse.
- Return enough command output for shell use, including at least the target task/session identifier and current status when available.
- Keep the first implementation aligned with the existing Rust session/message model rather than creating a parallel task store.

## Non-goals

- General Cline parity.
- Reworking TUI attach, detach, exit, or local server lifecycle behavior.
- Same-workspace automatic attach/reuse decisions.
- Building a full background-session dashboard.
- Implementing broad task monitoring or polling beyond the minimal result needed by the send/new command.
- Supporting image attachments unless the existing provider/message path can represent them cleanly without expanding this card.

## Done when

- A user can start a new coding task/session from the CLI with a prompt.
- A user can send a follow-up message to a target or current task/session from the CLI.
- A user can attach one or more files to the CLI-created or CLI-updated task/session.
- The command behavior is documented enough that it is clear how it differs from the TUI and from `opencode attach <url>`.
- The implementation does not change the approved `ort` default lifecycle: normal TUI exit still follows the current launcher contract, and attach/detach policy remains owned by the related background-session cards.

## Recommended verification

- Start a new CLI task/session with an inline prompt and confirm it appears in normal session history.
- Start a new CLI task/session from stdin and confirm the prompt text is preserved.
- Attach a text file with the CLI command and confirm the model receives or references the attached content according to the chosen attachment representation.
- Send a follow-up message to an existing task/session and confirm the message lands in that session rather than creating an unintended new one.
- Confirm `ort` TUI launch/exit semantics are unchanged.

## Split-Out Work

- `FEAT-019` Add CLI status visibility for tasks and background sessions.

## Related Items

- `FEAT-004` Add in-session send-to-fork commands
- `FEAT-007` Add advanced coding-session polling
- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist
- `START-008` Full parity deferred
- `START-020` Constrain primary product surface to the V1 workflow

## Notes

- This card was narrowed from a broad Cline-workflow holding bucket into the concrete CLI send/task convention story on 2026-09-16.
- Background-session management is already represented by recent and in-flight attach/detach/exit-convention items; do not use this card to reopen those lifecycle decisions.
