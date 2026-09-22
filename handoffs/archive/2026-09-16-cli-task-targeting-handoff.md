---
id: "H-003"
title: "CLI task targeting and queued sends - Handoff"
status: "superseded"
created: "2026-09-16"
updated: "2026-09-21"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["CLI-007", "CLI-008", "CLI-001", "CLI-006"]
---

# CLI Task Targeting and Queued Sends - Handoff

## Objective

Implement the CLI task workflow for `ort task ...` without reintroducing unsafe implicit server reuse. The user must be able to select a default server/session target, send/view tasks from the shell, and safely send work to a session that is open in the TUI by queueing the request on the canonical server/session runtime.

## Binding Invariants

- `invariants/cli-task-targeting.md` is binding for this entire feature set.
- `invariants/coding-session-behavior.md` still applies: task sends must use the canonical agentic session path, not a bare chat or parallel executor.
- `invariants/runtime-lifecycle.md` still applies: normal TUI lifecycle behavior must not be changed by this work unless a separate board item explicitly owns that change.

## Included Board Items

- `CLI-007` Add default task target selection for CLI sends.
- `CLI-008` Queue CLI task sends while TUI session is open.
- `CLI-001` Copy Cline-style CLI task send conventions.
- `CLI-006` Add CLI status visibility for tasks and background sessions.

## Excluded Work

- `CLI-004` detach behavior is separate and already tracked.
- `CLI-005` same-workspace normal TUI attach/reuse remains separate; do not implement automatic TUI server reuse here.
- First-class file/image attachments are out of scope for the first `task` implementation.
- Interactive `task chat` is out of scope; use `opencode attach <url>` for interactive TUI use.

## Current Code Evidence

- CLI entrypoint: `crates/opencode-cli/src/main.rs`.
- Current remote `run --attach` uses `/session/{id}/stream`; do not copy that as the canonical task send path.
- Canonical agentic async prompt path exists at `POST /session/{id}/prompt` in `crates/opencode-server/src/routes.rs` and returns `{"status":"started"}` after spawning the prompt work.
- Stub-like `POST /session/{id}/prompt_async` currently just appends messages and returns queued status; do not assume it satisfies agentic queued execution without fixing it.
- Session status exists at `GET /session/status` and the TUI consumes it through `crates/opencode-tui/src/api.rs`.
- TUI sends prompts through `POST /session/{id}/prompt`, matching the desired canonical path.
- TUI receives `OPENCODE_TUI_BASE_URL` during launch; that is useful context, but default task target selection must remain explicit and must not make normal `ort` launch auto-attach.

## Execution Notes

- 2026-09-21: Superseded by `H-004` (`handoffs/2026-09-21-session-prompt-queue-gate-handoff.md`) for the
  remaining `CLI-008`/`CLI-001`/`CLI-006` scope. `CLI-007` merged as PR #37; the queue semantics
  that this handoff left open are now pinned by `GATE-001` and `invariants/message-queuing.md`. Use
  `H-004` for implementation.
- 2026-09-16: PR 1 / `CLI-007` opened as https://github.com/cchris-p/opencode-modded-rust/pull/37 on branch `feature/CLI-007-task-target-selection` targeting `development`.
- PR 1 implements `opencode task target list|select|show|clear`, workspace-local `.opencode/task-target.json` storage, and live validation of explicit target servers/sessions. It intentionally does not add `task new`, `task send`, `task view`, queueing, status dashboards, or implicit TUI server reuse.
- PR 1 verification: `cargo fmt`; `cargo check -p opencode-cli`; `cargo run -p opencode-cli -- task target list`; `cargo run -p opencode-cli -- task target show`; `cargo run -p opencode-cli -- task target list --server http://127.0.0.1:9`; `cargo run -p opencode-cli -- task target --help`; live temporary-server smoke for list/select/show/clear.
- 2026-09-16: PR 1 merged into `development` at merge commit `1c1ad44`; `CLI-007` remains in `qa` for post-merge verification.

## Recommended Implementation Sequence

### PR 1 - `CLI-007` Default Task Target Selection

Branch: `feature/CLI-007-task-target-selection`

1. Add CLI subcommands for target management, likely `opencode task target list|select|show|clear` with `ort` inheriting the same binary behavior.
2. Store a selected default target with at least server URL and optional session ID. Prefer workspace-scoped storage first unless implementation discovery shows an existing config pattern that clearly supports global plus workspace precedence.
3. `target list` must live-check candidate servers/sessions before presenting them as active. Stale/unreachable records may be displayed only as unavailable.
4. `target select` must be user-directed. It can support interactive selection and direct flags for scripts.
5. `target show` and `target clear` must be non-destructive outside the target pointer.
6. Add explicit override model for later sends: command-line `--server` and `--session` must win over the stored target.

Verification gate:

- Select, show, change, and clear a target.
- Stop a selected server and confirm later target use fails clearly instead of silently choosing another server.
- Confirm normal `ort` launch still starts according to the existing launcher contract.

### PR 2 - `CLI-008` Per-Session Prompt Queue

Branch: `feature/CLI-008-session-prompt-queue`

1. Add server-side per-session prompt queueing around the canonical `SessionPrompt` execution path.
2. Queued work must preserve submit order per session.
3. A CLI send to a TUI-open or busy session must enqueue instead of racing, interleaving, replacing, or rejecting solely because the TUI is open.
4. The TUI must observe queued prompts/results through normal session updates.
5. Decide whether `/session/{id}/prompt` itself becomes queue-aware or whether a fixed `/prompt_async` is the queue endpoint. Either way, execution must run through `SessionPrompt`, agent context resolution, permission/question handling, and normal message persistence.
6. Return a clear queued/submitted response including target session and request/message identifier where available.

Verification gate:

- Open a session in the TUI, send a CLI/API prompt to that session, and confirm the prompt/result appears in the TUI.
- While one prompt is running, submit another and confirm ordered execution.
- Submit several prompts quickly and confirm no history corruption or interleaving.

### PR 3 - `CLI-001` CLI Task Send/New/View

Branch: `feature/CLI-001-cli-task-commands`

1. Add `opencode task new`, `opencode task send`, and `opencode task view`.
2. Use the explicit target options or selected default target from `CLI-007`.
3. `task new` creates a session on the target server, submits the prompt through the canonical queued/session prompt path, and updates the current/default session pointer only after session creation succeeds.
4. `task send` sends a follow-up to the selected or explicit session through the canonical queued/session prompt path.
5. `task view` reads the current or explicit session conversation without mutating the selected target or session state.
6. Support prompt text from arguments and stdin.
7. Keep file path handling as raw prompt text in this first implementation; do not add first-class file attachments here.
8. Default behavior returns after submit/queue with target session plus status; implement `--stream` only if it can correctly follow the queued request.

Verification gate:

- `task new "prompt"` creates a session and submits work.
- `task new < prompt.md` preserves stdin prompt text.
- `task send "follow-up"` uses the selected/current target.
- `task send --server ... --session ...` overrides the default target.
- `task view` prints the selected session conversation without opening the TUI.
- Prompt text can include file paths and the agent can inspect them through normal tools.
- The path uses `SessionPrompt`/server session runtime, not `AgentExecutor`.

### PR 4 - `CLI-006` CLI Status Visibility

Branch: `feature/CLI-006-cli-task-status`

1. Add compact status/listing commands for active/recent task sessions.
2. Reuse `GET /session/status` and session list data; do not create a separate status database.
3. Include enough status to distinguish active/busy/idle/completed/failed/queued where the runtime exposes it.
4. Keep status output separate from send/view semantics.

Verification gate:

- Show at least one active/busy session.
- Completed or idle sessions must not look active.
- If JSON output is added, verify it parses cleanly.

## Ordering and Dependencies

- `CLI-007` should land first because `CLI-001` depends on the default target contract.
- `CLI-008` should land before `CLI-001` if `task send` is expected to support TUI-open sessions from the start.
- `CLI-001` can land after `CLI-007` and `CLI-008`.
- `CLI-006` can land after `CLI-008` or after `CLI-001`; it is useful but not required to prove send/new/view behavior.
- Do not bundle all four cards into one PR. The queue and CLI command surfaces are large enough to deserve separate QA.

## Suggested User-Facing CLI Shape

```sh
ort task target list
ort task target select
ort task target show
ort task target clear

ort task new "Fix the failing provider test"
ort task new --server http://127.0.0.1:3000 "Start this on that server"
ort task send "Now run the focused tests"
ort task send --server http://127.0.0.1:3000 --session ses_123 "Follow up"
ort task view
ort task view --session ses_123
```

Names may change during implementation, but the behavior must satisfy `invariants/cli-task-targeting.md`.

## PR Workflow

- Create one branch and PR per implementation phase, targeting `development`.
- Keep each board item in `qa` after its PR is ready for local user verification.
- Do not merge a PR solely because tests pass; wait for explicit user merge direction after local testing.
- Preserve unrelated worktree changes and stage only files for the active board item.

## Documentation Updates

- Update `README.md` CLI overview when `task` commands exist.
- Update `docs/opencode-cli.md` with `task target`, `task new`, `task send`, `task view`, and status commands as they land.
- Reference `invariants/cli-task-targeting.md` from implementation PR descriptions.

## Readiness Assessment

These board items are implementation-ready as a sequence. The remaining choices are implementation details, not blockers, as long as the invariants and ordering above are followed.

## Reactivation Addendum (2026-09-22)

- `CLI-001` and `CLI-006` were reactivated on 2026-09-22 as prerequisite gates for the remaining `CLI-*`
  stories. This handoff remains superseded by `H-004`; use the reactivated cards for current scope.
- `CLI-007` has merged (PR #37) and is the target input consumed by `CLI-001`. `CLI-008` remains archived.

## Archived Outcome

- Superseded by `H-004` (`handoffs/2026-09-21-session-prompt-queue-gate-handoff.md`) on 2026-09-21 for the
  `CLI-008`/`CLI-001`/`CLI-006` scope.
- Folded into `H-004` and archived on 2026-09-22 because both handoffs covered the same board-item set.
- `CLI-007` delivered via PR #37; `GATE-001` (in `H-004`) delivered the queue; `CLI-001`/`CLI-006` were
  reactivated on 2026-09-22 under `H-004`.
