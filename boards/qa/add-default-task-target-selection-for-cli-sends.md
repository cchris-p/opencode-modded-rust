---
id: "FEAT-020"
title: "Add default task target selection for CLI sends"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-09-16"
---

# Add default task target selection for CLI sends

## Summary

Add an explicit way to select and change the default server/session target used by `ort task` CLI sends.

## Why this exists

`FEAT-005` adds CLI task commands for starting and continuing work outside the TUI, but those commands need a safe default target. The user wants active servers/sessions listed interactively so they can choose which server/session receives future task commands, and change that choice at any time.

This must not reintroduce unsafe implicit server discovery, stale-server reuse, or automatic TUI attach. Selecting a default send target is a user-directed action that records an explicit preference for later CLI task commands.

## Product Decisions

- Add a CLI surface for selecting the default task target, likely under `ort task target ...` or equivalent naming chosen during implementation.
- The selector lists active, observable servers and their sessions when that information is available.
- The user can interactively choose the default server and optionally the default session for future `ort task send` and `ort task view` commands.
- The user can override the selected default at send time with explicit server/session options for scripting and safety.
- The user can change or clear the selected default at any time.
- The selected default is for CLI task sends/views, not a request for normal `ort` TUI launches to auto-attach or reuse a server.

## Scope

- Define where the default target is stored and how it is scoped, such as workspace-local, global, or both with clear precedence.
- Define the command UX for listing, selecting, showing, and clearing the default task target.
- List active servers/sessions in a way that is safe against stale records and clearly marks unavailable targets.
- Store enough target information for later task sends, including at least server URL and, when selected, session ID.
- Make `ort task send` and `ort task view` use the selected default when no explicit server/session is provided.
- Make `ort task new` update the current/default session pointer when it successfully creates a new task on the selected server.
- Preserve an explicit override path for scripts, such as `--server` and `--session`.

## Non-goals

- Automatically attaching normal `ort` TUI launches to the selected server.
- Automatically reusing stale or last-known servers without a live check.
- Defining detach behavior for TUI-launched servers; that is `FEAT-017`.
- Deciding same-workspace automatic attach/reuse for normal TUI launch; that is `FEAT-018`.
- Building the full task/session status dashboard; broader status visibility is `FEAT-019`.

## Done when

- A user can list candidate active servers/sessions from the CLI.
- A user can interactively select the default server/session target for future CLI task sends.
- A user can show and clear the current default target.
- `ort task send` and `ort task view` use the selected default when no explicit target is provided.
- Explicit send-time target options override the selected default.
- Stale or unreachable selected targets fail clearly and do not silently fall back to a different server.
- The behavior is documented as separate from normal TUI attach/reuse.
- The behavior satisfies `invariants/cli-task-targeting.md`.

## Recommended verification

- Start or detach at least one server and confirm the selector lists it as a candidate.
- Select a default server/session and confirm `ort task send` without explicit target sends there.
- Change the selected target and confirm subsequent sends use the new target.
- Clear the selected target and confirm `ort task send` requires an explicit target or prompts according to the implemented UX.
- Stop a selected server and confirm sends fail clearly instead of silently choosing another server.
- Confirm normal `ort` launch/exit semantics remain unchanged.

## Related Items

- `FEAT-005` Copy Cline-style CLI task send conventions
- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist
- `FEAT-019` Add CLI status visibility for tasks and background sessions
- `FEAT-021` Queue CLI task sends while TUI session is open

## Notes

- Created on 2026-09-16 after clarifying that the desired workflow is explicit selection of a default server/session target for CLI task sends, not automatic TUI server reuse.
- Implemented in PR #37: https://github.com/cchris-p/opencode-modded-rust/pull/37
- Added `opencode task target list|select|show|clear`, workspace-local `.opencode/task-target.json` storage, and live target validation via `/health` plus `/session`.
- Verified with `cargo fmt`, `cargo check -p opencode-cli`, empty/unavailable target command checks, help output, and a live temporary-server smoke test for list/select/show/clear.
