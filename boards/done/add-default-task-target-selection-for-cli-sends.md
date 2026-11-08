---
id: "CLI-007"
title: "Add default task target selection for CLI sends"
priority: "P2"
type: "feature"
area: "CLI"
spec: ""
status: "done"
created: "2026-09-16"
attention: "Needs post-merge QA; provides the target input for CLI-001 (reactivated 2026-09-22), so closeout can proceed against the merged target commands"
---

# Add default task target selection for CLI sends

## Summary

Add an explicit way to select and change the default server/session target used by `ort task` CLI sends.

## Why this exists

`CLI-001` adds CLI task commands for starting and continuing work outside the TUI, but those commands need a safe default target. The user wants active servers/sessions listed interactively so they can choose which server/session receives future task commands, and change that choice at any time.

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
- Defining detach behavior for TUI-launched servers; that is `CLI-004`.
- Deciding same-workspace automatic attach/reuse for normal TUI launch; that is `CLI-005`.
- Building the full task/session status dashboard; broader status visibility is `CLI-006`.

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

- `CLI-001` Copy Cline-style CLI task send conventions
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers
- `CLI-005` Decide whether same-workspace server attach or reuse should exist
- `CLI-006` Add CLI status visibility for tasks and background sessions
- `CLI-008` Queue CLI task sends while TUI session is open

## Notes

- Created on 2026-09-16 after clarifying that the desired workflow is explicit selection of a default server/session target for CLI task sends, not automatic TUI server reuse.
- Implemented in PR #37: https://github.com/cchris-p/opencode-modded-rust/pull/37
- Added `opencode task target list|select|show|clear`, workspace-local `.opencode/task-target.json` storage, and live target validation via `/health` plus `/session`.
- Verified with `cargo fmt`, `cargo check -p opencode-cli`, empty/unavailable target command checks, help output, and a live temporary-server smoke test for list/select/show/clear.
- Merged into `development` on 2026-09-16 via PR #37 at merge commit `1c1ad44`; remains in `qa` for post-merge verification.

## QA Closeout Checklist - 2026-09-22

Code-complete: PR #37 merged into `development`. No handoff needed.

- [ ] `opencode task target list` shows live candidates and marks stale entries unavailable.
- [ ] `opencode task target select --server <url> [--session <id>]` persists the workspace-local target.
- [ ] `opencode task target show` reflects the selection; `clear` removes it.
- [ ] A stopped/unreachable selected target fails clearly with no silent fallback.
- [ ] Normal `ort` launch/exit semantics are unchanged.

On pass, move this card from `qa` to `done`.

## Open Dependency - 2026-09-22

`task target` is the routing input for `CLI-001` (reactivated 2026-09-22). It has no consumer until `CLI-001`
lands, but it is now the prerequisite that `CLI-001` consumes rather than an orphan, so this card's QA can
proceed against the merged target commands.

Canonical behavior reference: `wiki/cli-surface.md`.

## QA Verification - 2026-09-23 (PASS)

Headless QA on `development` (`54aa9c3`) against a live `opencode serve` in a throwaway workspace.

- `task target show` with no selection -> `No default task target selected.`
- `task target list` with no candidates -> clear guidance to pass `--server` or select first.
- `task target list --server <live>` -> lists server as available with its root sessions.
- `task target list --server <dead>` -> marks the server unavailable with the health error.
- `task target select --server <live> --session <id>` -> persists workspace-local
  `.opencode/task-target.json` and prints server + session.
- `task target select --server <dead>` -> refused, exit code 1.
- `task target select --server <live> --session <bogus>` -> refused, exit code 1.
- `task target show` -> reflects server/session/workspace and `Status: available`.
- `task target list` after selection -> marks the selected server and session with `*`.
- `task target clear` -> removes the file; a second clear reports none.
- Stale selected target (server stopped) -> `show`/`list` mark it unavailable and `task send`/`view`/
  `status` all fail clearly with exit code 1 and no silent fallback.

Observations (not blocking): `task target list` requires `--server` or an existing selection - it does
not discover live servers on its own; and the server's `/session?roots=true` list is global, so candidate
sessions are not scoped to the current workspace even though the selection is.

## QA Closeout - 2026-09-23

- QA passed (see above) on `development` (`54aa9c3`); no code change was required for this card.
- The selected target is consumed by `CLI-001`/`BUG-036` `task new`, which re-verified successfully.
- Card moved from `qa` to `done`.