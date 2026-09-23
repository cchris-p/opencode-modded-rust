# CLI Surface and TUI Lifecycle

## Purpose

Canonical behavior reference for the CLI task surface and the TUI launch/detach/attach lifecycle.
`CLI-*` board items are the only owning IDs for this surface. Older `FEAT-*`/`BUG-*` items that
touched the same behavior are history and are cited only as lineage.

This document states (1) what the product does today and (2) the target behavior the `CLI-*` series
is driving toward.

## Current Behavior (as of 2026-09-23)

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
  `.opencode/task-target.json` with live `/health` + `/session` validation (`CLI-007`, PR #37; done
  2026-09-23).
- `opencode task new`, `opencode task send`, `opencode task view`, and `opencode task status` exist.
  `new`/`send` submit through the canonical `POST /session/{id}/prompt` path; `view` reads
  `/session/{id}/message`; `status` reads `GET /session/status` with plain-text and `--json` output
  (`CLI-001`/`CLI-006`, PR #80/#81; defects fixed by `BUG-036` PR #82; done 2026-09-23).
- `opencode run` currently uses the interim `AgentExecutor` tool loop, not the canonical
  session/server runtime (`CLI-002`, `hold`).

## Target Behavior

| Owner | Target |
|---|---|
| `CLI-001` | `task new`/`send`/`view` on the canonical server/session prompt path; returns target session plus `started`/`queued`; `--stream` follows once active. Delivered 2026-09-23 (PR #80, fix `BUG-036` PR #82). |
| `CLI-006` | CLI status visibility over `GET /session/status` (`idle|busy|queued|retry|error`), plain text plus `--json`. Delivered 2026-09-23 (PR #81, fix `BUG-036` PR #82). |
| `CLI-002` | Route `opencode run` through the canonical session runtime; retire the parallel `AgentExecutor` loop. |
| `CLI-009` | CLI/direct-run question and ask/approval parity; blocked by `CLI-001`/`CLI-006`, `CLI-002`, and `GATE-002`. |
| `CLI-010` | Subagent/child-session surface on the CLI; blocked by `GATE-004`. |
| `CLI-005` | Human decision on same-workspace attach/reuse; must not reintroduce implicit server discovery. |
| `CLI-004` | Explicit `/detach` command (done). |
| `FEAT-033` | Resume hint on normal TUI exit (done); adjacent exit UX, not detach. |

Binding rules: `invariants/cli-task-targeting.md`, `invariants/message-queuing.md`, and
`invariants/runtime-lifecycle.md` (leaving a session view must not cancel active execution; a user
must be able to leave and revisit a running session).

## Intended Headless (Cline-like) Workflow

The target is to drive the same agentic session runtime as the TUI without a TUI, and to keep a server
alive independently of any TUI. Detached + headless together are what make the CLI Cline-like.

Available today:

- `opencode serve` starts a headless server (no TUI); alternatively `ort` + `/detach` leaves a
  TUI-launched server alive.
- `opencode task target select --server <url>` records the server/session to use (`CLI-007`).
- `opencode task new|send|view|status` drive the canonical session runtime headlessly
  (`CLI-001`/`CLI-006`).
- Direct HTTP `POST /session/{id}/prompt` already runs agentic work headlessly.
- `opencode run` / `run --attach` exists but uses the interim `AgentExecutor` path, not the canonical
  session runtime (`CLI-002`, `hold`).

Target shape (the Cline-like workflow):

```sh
opencode serve                                     # or: ort, then /detach
opencode task target select --server <url>
opencode task new "Fix the failing test"           # CLI-001
opencode task send "Now run the focused tests"     # CLI-001
opencode task view                                 # CLI-001
opencode task status                               # CLI-006
opencode attach <url>                              # optional: back to the TUI
```

`CLI-001` and `CLI-006` are delivered, merged into `development` (PR #80, PR #81), and QA-verified
(2026-09-23); `BUG-036` (PR #82) fixed the `task new` 404 and the `idle` status label. The composition
is recorded in `handoffs/archive/2026-09-22-cli-task-surface-and-status-handoff.md` (`H-006`).
`CLI-002` remains on `hold` and retires the parallel `opencode run` engine so headless runs use the
same canonical path.

## Canonical Card Map

| ID | Lane | Role |
|---|---|---|
| CLI-001 | done | Cline-style task send (PR #80; QA done 2026-09-23) |
| CLI-002 | hold | Route `opencode run` through session runtime (gated) |
| CLI-003 | done | Removed server reuse / fresh server per launch |
| CLI-004 | done | Explicit `/detach` command |
| CLI-005 | hold | Same-workspace attach/reuse decision (human gate) |
| CLI-006 | done | CLI status visibility (PR #81; QA done 2026-09-23) |
| CLI-007 | done | Default task target selection (PR #37; QA done 2026-09-23) |
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
- `BUG-036` Fix CLI `task new` 404 and `task status` idle labeling (done 2026-09-23, PR #82) -
  post-merge QA fix for `CLI-001` (`/session` trailing-slash URL) and `CLI-006` (idle label).

## Boundaries / Non-Goals

- This document does not own question-tool parity (`GATE-002`) or subagent parity (`GATE-004`); it
  only lists the CLI stories those gates block.
- It does not define provider or transport behavior.
- Archived and done cards are immutable history; do not re-open or re-own them here.
