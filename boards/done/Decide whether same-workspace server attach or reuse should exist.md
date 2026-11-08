---
id: "CLI-005"
title: "Decide whether same-workspace server attach or reuse should exist"
priority: "P2"
type: "feature"
area: "CLI"
status: "done"
predecessors: "CLI-001, CLI-006"
created: "2026-09-16"
updated: "2026-09-26"
---

# Decide whether same-workspace server attach or reuse should exist

## Decision - 2026-09-26: NO-GO (explicit-only)

**Resolved: same-workspace server attach/reuse will not be implemented in any implicit form.**
`ort` keeps starting a fresh server per launch; reattachment is always explicit. No prompt,
suggestion, or automatic discovery path is added, and no persisted process/server discovery record
is introduced. The no-reuse invariant is now permanent, not conditional.

Reasoning (why this is the Cline-aligned resolution):

1. The copied Cline surface is explicit-target by construction. `CLI-001`'s product decisions
   require `opencode task` to talk "only to an explicitly configured/provided server/session context
   or to the explicit default task target selected by `CLI-007`" and explicitly forbid the command
   from discovering, starting, or reusing servers implicitly. Resolving `CLI-005` as no-go preserves
   that contract instead of adding a second, hidden targeting path.
2. Reuse adds no missing capability to the Cline-like workflow. That workflow is already complete
   and explicit: `opencode serve` (or `ort` + `/detach`) -> `opencode task target select
   --server <url>` -> `opencode task new|send|view|status` -> optional `opencode attach <url>`
   (`wiki/cli-surface.md`, "Intended Headless (Cline-like) Workflow").
3. Every auto-detect/prompt/suggest variant needs exactly the persisted server/process discovery
   record that `CLI-003` removed. The lineage shows that record is unsafe: `FEAT-002` introduced it,
   `FEAT-014` tried to tame it, and `CLI-003` deleted it because a stale detached server served
   pre-fix binaries and a wrong/other workspace (`BUG-003`, `BUG-004`, `BUG-011`; `CLI-003` "Why this
   exists"). Do not trade that safety invariant for convenience.
4. Explicit detach already covers the legitimate "leave work running" need. `/detach` keeps the
   launched server alive and prints `opencode attach <url>` / `ort --attach <url>`, so reattach stays
   a deliberate, user-directed action (`CLI-004`, done).
5. Cline itself exposes explicit task targeting, not implicit same-workspace server discovery.
   Aligning with the copied conventions means explicit-only.

Consequences (no product code change):

- Every `ort` launch starts a fresh server for the activated workspace; no launch path reads or
  writes a discovery record (`CLI-003`).
- Normal exit (`Ctrl-D`, Esc, `/exit`) terminates the launched server; only `/detach` leaves it alive
  (`CLI-004`).
- Same-workspace reattachment is only `opencode attach <url>` / `ort --attach <url>`, always explicit,
  never inferred.
- Headless work targets an explicit server via `opencode task target select --server <url>` or a
  per-command `--server` override (`CLI-007`).
- Detach continues to write no persisted record; this decision adds none.
- Difference from `opencode attach <url>`: attach is the sole reattachment path and is always a
  user-issued, explicit URL; no launch-time detection augments it.

Decision gate satisfied: this is the explicit go/no-go record the card's "Done when" required. The
approved branch is "rejected", so the no-reuse invariant stays documented here and in
`invariants/runtime-lifecycle.md` / `invariants/cli-task-targeting.md`.

## Blocked By - 2026-09-22

- `CLI-001` Copy Cline-style CLI task send conventions (prerequisite gate).
- `CLI-006` Add CLI status visibility for tasks and background sessions (prerequisite gate).
- This is a human decision gate; do not start or resolve it until `CLI-001`/`CLI-006` land and the user
  gives explicit 110% confirmation.

## Summary

Decide whether future `ort` launches should detect an already-running server for the same workspace
and offer, prompt for, or automatically perform attach/reuse.

## Context

During CLI-003 refinement, the user said this would be a cool feature but may be annoying in some
situations, and should be documented as a follow-up item planned for implementation only once there
is "110% confirmation".

This is intentionally separate from `CLI-004`, which only covers an explicit detach command/action.

## Scope

- Decide whether same-workspace server detection should exist at all.
- Decide whether detection should prompt, attach automatically, or only print a suggestion.
- Define how a matching workspace server would be discovered without reintroducing unsafe stale
  server reuse.
- If a process/server record is proposed, define exactly what it is allowed to do and what it is
  forbidden to do.
- Decide whether detach should write any persisted display/discovery record; this was explicitly
  deferred from `CLI-004`.
- Define how this differs from explicit `opencode attach <url>`.

## Non-goals

- Implementing same-workspace attach/reuse before explicit 110% confirmation.
- Changing CLI-003's default behavior: normal `ort` starts fresh and normal TUI exit terminates the
  server it launched.
- Defining the explicit detach command itself; that is tracked in `CLI-004`.
- Adding a persisted detach/server record without the same explicit 110% confirmation gate.

## Done when

- The project has an explicit go/no-go decision for same-workspace attach/reuse.
- If approved, the desired user interaction and safety constraints are clear enough to implement.
- If rejected, the no-reuse invariant remains documented and this item records why.

## Related Items

- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers

## Closeout - 2026-09-26

- Decision recorded: no-go / explicit-only (see "Decision - 2026-09-26" above).
- Invariants updated: `invariants/runtime-lifecycle.md` and `invariants/cli-task-targeting.md` now
  record the resolved no-go instead of an open decision.
- Canonical reference updated: `wiki/cli-surface.md` (`CLI-005` target row and canonical card map).
- No code change: the current fresh-server-per-launch behavior already satisfies the decision.
- Card moved from `hold` to `done`; `attention` gate cleared.
