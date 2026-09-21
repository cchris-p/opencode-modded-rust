---
id: "GATE-001"
title: "Gate: prompt queuing and queue display must match vanilla OpenCode exactly"
priority: "P0"
type: "gate"
area: "GATE"
spec: "invariants/message-queuing.md"
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Gate: prompt queuing and queue display must match vanilla OpenCode exactly

## Gate requirement

Queuing system and visual display of queue must match exactly opencode vanilla.

This is a hard gate. The blocked stories below must not be completed until every acceptance
criterion here is satisfied and verified against the pinned reference line.

## Summary

The Rust product has no prompt queue today: concurrent sends to one session spawn concurrent
prompt runs that race and overwrite session snapshots, and the TUI has no queued-message
display at all (`grep -i queued crates/opencode-tui/src` returns nothing). The
`prompt_async` endpoint reports `queued` without ever executing. The requirement is not a
bespoke queue UX: it is exact parity with vanilla OpenCode's queuing system and its visual
display of the queue.

## Source of truth

Frozen TypeScript reference line: `$HOME/repos/opencode-modded` at commit
`e62912b5d18b73316c7bfd6e894b040698f6c880` (per `AGENTS.md`). The following were confirmed
present at that commit:

- TUI pending boundary: `packages/tui/src/routes/session/index.tsx:244-250`.
- TUI queued-message display (`QUEUED` badge): `packages/tui/src/routes/session/index.tsx:1388-1452`.
- Manage queued prompts keybind (`<leader>q`): `packages/tui/src/config/keybind.ts:102`.
- CLI run serial prompt queue: `packages/opencode/src/cli/cmd/run/runtime.queue.ts`.
- CLI footer queue count and queued prompt list:
  `packages/opencode/src/cli/cmd/run/footer.ts`,
  `packages/opencode/src/cli/cmd/run/types.ts:45,83-92`.

## Why this exists

- Current server behavior is concurrent, not queued; see `invariants/message-queuing.md` for the
  evidence-backed current snapshot and the target invariants.
- The TUI currently optimistically renders a user message but has no queued state or badge.
- The CLI `run` surface has a serial queue and footer queue display upstream that the Rust port
  does not match.
- Parity must be exact, including colors, placement, and edit/remove behavior, not merely
  "a queue exists".

## Scope

- Server-side per-session queue semantics matching vanilla: at most one active prompt turn per
  session, remaining prompts queued in submit order, queued prompts observable and
  editable/removable until they begin.
- TUI visual display of the queue matching vanilla exactly, including the `QUEUED` badge
  condition, placement, agent-color background, selected foreground, bold weight, and timestamp
  replacement.
- Manage-queued-prompts surface and keybind matching vanilla (`<leader>q`).
- CLI `run` footer queue count and queued-prompt list matching vanilla.
- Remove false `queued` reporting so no endpoint claims a prompt is queued without executing it.

## Non-goals

- Upstream sync or adopting unrelated upstream changes.
- Non-queue TUI features.
- The concurrent-server DB sync data loss tracked by `BUG-025` (adjacent, not blocked here).
- Redesigning queue UX beyond exact vanilla parity.

## Acceptance criteria

- [ ] The server allows at most one active prompt turn per session; additional sends queue in
      submit order.
- [ ] Queued prompts remain observable and editable/removable until they begin, matching vanilla.
- [ ] The TUI renders the vanilla `QUEUED` badge on user messages that are queued, using the same
      boundary condition (index after the in-flight assistant message), agent-color background,
      selected foreground, bold weight, and timestamp replacement.
- [ ] `<leader>q` opens the manage-queued-prompts surface with vanilla behavior.
- [ ] The CLI `run` footer exposes the same queue count and queued-prompt list as vanilla.
- [ ] No endpoint returns `queued` without executing the queued prompt.
- [ ] Side-by-side parity evidence is captured against the pinned reference commit.

## Blocked Items

- `FEAT-021` Queue CLI task sends while TUI session is open.
- `FEAT-005` Copy Cline-style CLI task send conventions.
- `FEAT-019` Add CLI status visibility for tasks and background sessions.

## Recommended verification

- Side-by-side comparison against the pinned reference commit for each acceptance criterion.
- `cargo test -p opencode-server` once the queue implementation lands.
- Manual: send a second prompt while the first is running and compare the TUI queue display
  against vanilla.

## Related Items

- `invariants/message-queuing.md` - current-behavior snapshot and target invariants for queueing.
- `FEAT-036` Persist and recall typed input-box messages after send or clear - local draft
  persistence, not a prompt queue.
- `BUG-025` Concurrent servers delete each other's sessions and messages via full-snapshot DB sync
  - adjacent concurrency data-loss bug, not blocked by this gate.

## Notes

- Created on 2026-09-21 as a hard gate for queue parity with vanilla OpenCode.
- `bd` does not parse `predecessors` or any block field today; the `## Blocked By` sections on the
  blocked cards carry the visible dependency, and `predecessors: "GATE-001"` records it in
  frontmatter ahead of the planned standardized board relationship schema.
