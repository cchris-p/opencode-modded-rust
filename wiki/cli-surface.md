# CLI Surface and TUI Lifecycle

## Purpose

Canonical behavior reference for the CLI task surface and the TUI launch/detach/attach lifecycle.
`CLI-*` board items are the only owning IDs for this surface. Older `FEAT-*`/`BUG-*` items that
touched the same behavior are history and are cited only as lineage.

This document states (1) what the product does today and (2) the target behavior the `CLI-*` series
is driving toward.

## Current Behavior (as of 2026-09-22)

### TUI launch

- Every `ort` launch starts a fresh local server bound to the activated workspace. No persisted
  server/process record is read or written, and no prior server is reused (`CLI-003`, done).
  Evidence: `crates/opencode-cli/src/main.rs:959-989` (`LocalTuiServer` with no record path;
  kill-on-drop unless detached).

### Detach

- `/detach` is available from the TUI command registry and prompt slash suggestions; it has no
  default keybinding. Evidence: `crates/opencode-tui/src/command.rs:382-389`.
- Detach exits the TUI and leaves the server launched for that TUI alive. It sets
  `AppState::Detaching` and returns `TuiExit::Detach` (`crates/opencode-tui/src/app/app.rs:345-346`,
  `crates/opencode-tui/src/lib.rs:26`), and the CLI disables the `LocalTuiServer` kill-on-drop guard
  only for that result (`crates/opencode-cli/src/main.rs:897-908,973-976`).
- After detach the terminal prints the server URL, workspace, `opencode attach <url>`, and
  `ort --attach <url>` (`crates/opencode-cli/src/main.rs:904-907`).
- Detach writes no persisted record. Canonical owner: `CLI-004`.

### Attach / reattach

- Reattachment is explicit: `opencode attach <url>` (`crates/opencode-cli/src/main.rs:72-82`) or
  `ort --attach <url>` / `opencode tui --attach <url>` (`crates/opencode-cli/src/main.rs:70,108`).
- Normal exit (`Ctrl-D`, Esc, `/exit`) terminates the launched server; only detach leaves it alive.

### Session prompt queue

- Per-session FIFO prompt queue with accept-time materialization and the vanilla `QUEUED` badge
  (`GATE-001`, done). One runner runs per session; `GET /session/status` exposes `idle|busy|queued`
  plus position/depth.

### CLI task surface

- `opencode task target list|select|show|clear` exists and stores a workspace-local
  `.opencode/task-target.json` (`CLI-007`, merged PR #37, in `qa`).
- `opencode task new`, `opencode task send`, `opencode task view`, and a CLI status surface do
  **not** exist yet (`CLI-001`/`CLI-006` in `todo`).
- `opencode run` currently uses the interim `AgentExecutor` tool loop, not the canonical
  session/server runtime (`CLI-002`, `hold`).

## Target Behavior

| Owner | Target |
|---|---|
| `CLI-001` | `task new`/`send`/`view` on the canonical server/session prompt path; returns target session plus `started`/`queued`; `--stream` follows once active. Prerequisite gate for the rest of the series. |
| `CLI-006` | CLI status visibility over `GET /session/status` (`idle|busy|queued|retry|error`), plain text plus `--json`. Co-gate. |
| `CLI-002` | Route `opencode run` through the canonical session runtime; retire the parallel `AgentExecutor` loop. |
| `CLI-009` | CLI/direct-run question and ask/approval parity; blocked by `CLI-001`/`CLI-006`, `CLI-002`, and `GATE-002`. |
| `CLI-010` | Subagent/child-session surface on the CLI; blocked by `GATE-004`. |
| `CLI-005` | Human decision on same-workspace attach/reuse; must not reintroduce implicit server discovery. |
| `CLI-004` | Explicit `/detach` command (done). |
| `FEAT-033` | Resume hint on normal TUI exit (done); adjacent exit UX, not detach. |

Binding rules: `invariants/cli-task-targeting.md`, `invariants/message-queuing.md`, and
`invariants/runtime-lifecycle.md` (leaving a session view must not cancel active execution; a user
must be able to leave and revisit a running session).

## Canonical Card Map

| ID | Lane | Role |
|---|---|---|
| CLI-001 | todo | Cline-style task send (prerequisite gate) |
| CLI-002 | hold | Route `opencode run` through session runtime (gated) |
| CLI-003 | done | Removed server reuse / fresh server per launch |
| CLI-004 | done | Explicit `/detach` command |
| CLI-005 | hold | Same-workspace attach/reuse decision (human gate) |
| CLI-006 | todo | CLI status visibility (co-gate) |
| CLI-007 | qa | Default task target selection (prerequisite input) |
| CLI-008 | archive | Queue CLI sends (delivered by `GATE-001`) |
| CLI-009 | hold | Direct-run question and ask/approval parity (gated) |
| CLI-010 | hold | CLI subagent surface (gated) |

## Lineage (history only)

- `FEAT-002` Keep sessions running after TUI exit (archived) - introduced the detached local
  `serve` process (PR #8). Archived because its reusable detached-server record caused the
  stale-server QA trap; superseded by `FEAT-014`.
- `FEAT-014` Enforce a single local TUI server per workspace (done) - replaced `FEAT-002` reuse.
- `CLI-003` Remove local TUI server reuse (done) - removed the record/reuse entirely.
- `CLI-004` Explicit `/detach` (done) - re-added deliberate, user-directed detach on the
  fresh-server model (PR #35, PR #62).
- `FEAT-033` Print a resume command on normal TUI exit (done) - adjacent exit UX.

## Boundaries / Non-Goals

- This document does not own question-tool parity (`GATE-002`) or subagent parity (`GATE-004`); it
  only lists the CLI stories those gates block.
- It does not define provider or transport behavior.
- Archived and done cards are immutable history; do not re-open or re-own them here.
