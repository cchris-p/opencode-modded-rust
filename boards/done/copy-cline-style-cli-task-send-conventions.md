---
id: "CLI-001"
title: "Copy Cline-style CLI task send conventions"
priority: "P2"
type: "feature"
area: "CLI"
spec: ""
status: "done"
created: "2026-09-08"
updated: "2026-09-22"
---

# Copy Cline-style CLI task send conventions

## Status - 2026-09-22

Reactivated at user request. This card is now the primary Cline-like CLI task surface and a prerequisite
gate for the remaining `CLI-*` stories: Cline-like functionality must exist before other CLI stories are
refined further. `CLI-006` is the co-prerequisite for status visibility.

Prerequisites are already in place:

- `GATE-001` (done) shipped the per-session prompt queue and the `Queued { position, depth }` run status, so
  `task new`/`send` return `started`/`queued` and `--stream` behaves as described below.
- `CLI-007` (merged, PR #37) shipped `opencode task target list|select|show|clear`, which this card consumes.
- `invariants/cli-task-targeting.md` and `invariants/message-queuing.md` remain binding.

Canonical behavior reference: `wiki/cli-surface.md`.

Handoff: `handoffs/archive/2026-09-22-cli-task-surface-and-status-handoff.md` (`H-006`, complete).

## Summary

Add a CLI-first task interaction surface inspired by Cline's `task` flow, focused on starting tasks, sending follow-up messages, and viewing task conversation state outside the TUI.

## Why this exists

Cline exposes useful command-line conventions for quick coding-assistant interactions without forcing the user through an interactive TUI. The useful part for this product is not broad Cline parity; it is the ability to start or continue chat/task work from the shell, reference relevant files in the message text, and get usable output from a command-oriented workflow.

Background-session lifecycle and attach/detach policy are intentionally handled by separate board items. This card owns the CLI send/task conventions layered on top of the canonical server/session prompt runtime.

## Cline Behaviors To Consider

- `-f` / `--file` attaches files to the task.
- `cline task new` creates a new task.
- `cline task send` sends a follow-up message to the current task and can update task mode or approval state.
- `cline task view` shows task conversation without opening an interactive UI.
- Output can be formatted for rich, plain, or JSON consumers.

## Product Decisions

- The first target surface is `opencode task ...`, not top-level `opencode "prompt"`.
- `opencode task new` starts a new task/session from provided prompt text.
- `opencode task send` sends a follow-up message to the current task/session by default.
- `opencode task view` shows the current or selected task/session conversation outside the TUI.
- `opencode task chat` is not needed in the first implementation because intentional interactive use can go through `opencode attach <url>`.
- Follow-up targeting may use a current task/session pointer by default, with an explicit target option available for safety and scripting.
- CLI-001 must use the canonical server/session prompt path described in `wiki/agent-debugging-without-tui.md`, not the interim `AgentExecutor` path fixed by `CLI-002`.
- CLI-001 must not discover, start, or reuse servers implicitly. It should talk only to an explicitly configured/provided server/session context or to the explicit default task target selected by `CLI-007`.
- File attachment flags are deferred. In the first implementation, the user can include file paths in raw message text and the agent should navigate/read them through normal tools.
- Commands return immediately by default after submitting work. `--stream` follows the request once it
  becomes active; if the request is queued behind an active turn, it reports queued status first and
  begins streaming when the turn starts (matching `GATE-001`).

## Scope

- Define and implement `opencode task new` for starting a task/session from prompt text outside the TUI.
- Define and implement `opencode task send` for sending a follow-up message to the current or explicitly selected task/session outside the TUI.
- Define and implement `opencode task view` for viewing the current or explicitly selected task/session conversation outside the TUI.
- Route task creation, sending, and viewing through the same server/session runtime used by the TUI, including `/session/{id}/prompt` semantics or an equivalent direct call into that path.
- Define how the current task/session pointer is established, updated, displayed, and overridden.
- Integrate with the default task target selected by `CLI-007`, while preserving explicit send-time server/session overrides.
- Support prompt text from command arguments and stdin where that fits the subcommand UX.
- Support raw file path references in prompt text without first-class attachment handling.
- Define how the command finds its explicit server/session context without reintroducing unsafe implicit server discovery or reuse.
- Return enough command output for shell use, including at least the target task/session identifier and current status when available.
- Return immediately by default after task/message submission, and support `--stream` for live output when feasible.
- Keep the first implementation aligned with the existing Rust session/message model rather than creating a parallel task store.

## Non-goals

- General Cline parity.
- Reworking TUI attach, detach, exit, or local server lifecycle behavior.
- Same-workspace automatic attach/reuse decisions.
- Building a full background-session dashboard.
- Implementing broad task monitoring or polling beyond the minimal result needed by the send/new command.
- First-class file or image attachments; split this out if the existing session/provider model can represent attachments cleanly.
- Interactive `task chat`; use `opencode attach <url>` for interactive chat in the first pass.
- Top-level `opencode "prompt"` shorthand.
- Routing through `AgentExecutor`; that path is the interim `opencode run` implementation and is not canonical for TUI/session-equivalent QA.

## Done when

- A user can start a new coding task/session from the CLI with a prompt.
- A user can send a follow-up message to the current task/session from the CLI, and can explicitly override the target for scripting/safety.
- A user can view the current or selected task/session conversation from the CLI without opening the TUI.
- A user can include file paths in CLI message text and the agent can proceed by reading/navigating those paths through normal tools.
- Task CLI behavior uses the canonical server/session prompt runtime so successful task CLI smoke tests are valid evidence for agent-session behavior that the TUI would display.
- By default, task send/new commands submit work and return a task/session identifier plus status; `--stream` streams live output where supported.
- The command behavior is documented enough that it is clear how it differs from the TUI and from `opencode attach <url>`.
- The implementation does not change the approved `ort` default lifecycle: normal TUI exit still follows the current launcher contract, and attach/detach policy remains owned by the related background-session cards.
- CLI task targeting behavior satisfies `invariants/cli-task-targeting.md`.

## Recommended verification

- Start a new CLI task/session with an inline prompt and confirm it appears in normal session history.
- Start a new CLI task/session from stdin and confirm the prompt text is preserved.
- Send a prompt that references a file path and confirm the agent can inspect that path through normal tools.
- Send a follow-up message with no explicit target and confirm it lands in the current task/session.
- Send a follow-up message with an explicit target and confirm it lands in that selected task/session.
- Run `opencode task view` and confirm it prints the selected task/session conversation without opening the TUI.
- Confirm default send/new exits after submission, and `--stream` streams output when implemented.
- Confirm the task CLI path uses the same session/server prompt runtime as the TUI rather than `AgentExecutor`.
- Confirm `ort` TUI launch/exit semantics are unchanged.

## Split-Out Work

- `CLI-006` Add CLI status visibility for tasks and background sessions.
- First-class file/image attachment flags for CLI task commands, if the existing session/provider model can support them cleanly.

## Related Items

- `FEAT-004` Add in-session send-to-fork commands
- `FEAT-007` Add advanced coding-session polling
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers
- `CLI-005` Decide whether same-workspace server attach or reuse should exist
- `CLI-007` Add default task target selection for CLI sends
- `CLI-008` Queue CLI task sends while TUI session is open
- `CLI-002` Route `opencode run` through the canonical session runtime
- `QA-001` Build a repeatable debug/QA verification suite for the session/stream runtime
- `START-008` Full parity deferred
- `START-020` Constrain primary product surface to the V1 workflow

## Notes

- This card was narrowed from a broad Cline-workflow holding bucket into the concrete CLI send/task convention story on 2026-09-16.
- Background-session management is already represented by recent and in-flight attach/detach/exit-convention items; do not use this card to reopen those lifecycle decisions.
- Further refinement on 2026-09-16 selected task subcommands over top-level prompt shorthand, current-pointer targeting, explicit server/session context, raw path text instead of first-class attachments, and return-immediately behavior with optional streaming.
- Follow-up refinement on 2026-09-16 split the explicit default server/session selector into `CLI-007`; CLI-001 should consume that selected target rather than inventing implicit server discovery.
- `CLI-002` fixed `opencode run` as an interim `AgentExecutor` path, but this card should not build on that executor if the goal is TUI/session-equivalent QA. Build on the canonical session/server prompt path instead.

## Implementation Notes - 2026-09-22

- Branch: `feature/CLI-001-task-commands`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/80.
- Implemented `opencode task new|send|view` in `crates/opencode-cli/src/main.rs`.
- `task new` creates a session on the explicit/selected target server, submits through `POST /session/{id}/prompt`, and stores the acknowledged session as the workspace-local default task session.
- `task send` submits follow-up prompts to the explicit or selected default session through `POST /session/{id}/prompt`.
- `task view` reads the selected server's `/session/{id}/message` route and prints the transcript without opening the TUI; `--json` prints the raw message array.
- `--stream` keeps the canonical `/prompt` submission path and follows by polling `GET /session/status` until idle, then prints the transcript.
- Verification passed: `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo run -p opencode-cli -- task --help`; `cargo run -p opencode-cli -- task new --help`; `cargo run -p opencode-cli -- task send --help`; `cargo run -p opencode-cli -- task view --help`.

## Merge Closeout - 2026-09-22

- PR #80 merged into `development` (merge commit `5b71d14`).
- Branch `feature/CLI-001-task-commands` deleted locally and remotely.
- Remains in `qa` pending a recorded post-merge QA report (`H-006` QA notes) or explicit user completion.

## QA Verification - 2026-09-23 (FAIL)

Headless QA on `development` (`54aa9c3`), binary rebuilt with `cargo build -p opencode-cli`. Ran a
throwaway `opencode serve` workspace and drove real turns with `deepseek/deepseek-flash`.

FAIL - `opencode task new` returns HTTP 404 and cannot create a session:

```
$ opencode task new --server http://127.0.0.1:<port> "Reply with exactly the single token PONG"
Error: Request failed (404 Not Found):
```

Root cause: `create_task_session` posts to `/session/` (trailing slash) at
`crates/opencode-cli/src/main.rs:2788`, but the server registers `POST /session` with no trailing
slash (`crates/opencode-server/src/routes.rs:68`) and `server_url` preserves the trailing slash.
Raw evidence: `POST /session/` -> `404`, `POST /session` -> `200`.

PASS - the rest of the surface, verified against a session created directly on the API:

- `task send` with `--server/--session`, and with no flags via the workspace-local selected target,
  accepted and returned `Session`/`Message`/`Status`.
- `task view` (with no flags, and `--json`) printed the transcript.
- stdin prompt input preserved text.
- `--stream` reported `queued` queue position/depth, then `busy` -> `active`, then printed the transcript.
- File-path text (e.g. `./README.md`) was preserved verbatim in the message.
- Missing target and unreachable target both failed clearly with exit code 1 and no silent fallback.

Verified model turns used `deepseek/deepseek-flash`; the CLI's own default model resolved to
`ollama/qwen3:30b` (from `OPENCODE_MODEL_OLLAMA_LOCAL`) and returned a provider network error, which is
an environment/default-model concern outside this card.

Verdict: not done. `task new` is a hard blocker; fix the URL and re-QA. `task send`/`task view`/`--stream`
are QA-verified. Recommend logging the 404 as a separate bug item so the fix is tracked independently of
this card.

Remediation: `BUG-036` fixes the 404 in PR #82
(https://github.com/cchris-p/opencode-modded-rust/pull/82). Re-QA this card after that PR merges.

## QA Re-Verification and Merge Closeout - 2026-09-23

- `BUG-036` (PR #82, merge commit `570eff8`) fixed the `task new` 404 and merged into `development`.
- Re-QA on the fixed binary: `opencode task new --server <url> "<prompt>"` now creates a session,
  submits through `POST /session/{id}/prompt`, and persists the workspace-local selected session.
- Previously passing paths remain passing: `task send` (explicit and selected default), `task view`
  (plain and `--json`), stdin input, `--stream` (queued/busy/transcript), file-path text, and clear
  unreachable-target errors.
- Branch `bug/BUG-036-cli-task-surface-defects` deleted locally and remotely.
- Card moved from `qa` to `done`.