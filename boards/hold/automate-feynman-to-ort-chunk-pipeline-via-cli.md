---
id: "FEAT-066"
title: "Automate the Feynman-to-ort workable-chunk pipeline via the CLI"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "wiki/cli-surface.md"
status: "hold"
created: "2026-09-26"
---

# Automate the Feynman-to-ort workable-chunk pipeline via the CLI

## Summary

Automate the manual workflow in which Feynman produces **workable chunks** (represented as board items) and the operator processes them **concurrently using `ort`**. This item covers the pipeline mechanics only; whether Feynman's research method is worth using at all is `FEAT-065`.

**Status:** the mechanism is resolved below. The process is manual today; this card tracks building the automation.

## Settled model

- Feynman is a separate research harness; `ort` is the coding/agent product.
- Feynman's research methods emit self-contained, independently executable chunks.
- Each chunk is a board item (markdown card with valid frontmatter).
- `ort` processes chunks concurrently (one session per chunk).
- Today: the operator does this by hand.

## Resolved mechanism

The automation is a dispatcher that rides the **canonical `opencode task` CLI path** over a headless server. It must not use `opencode run`, which still uses the interim `AgentExecutor` loop (`CLI-002`, hold).

1. **Headless server.** Start `opencode serve` (or `ort` + `/detach`) for the target workspace.
2. **Chunk source.** A chunk is a board-item file in a target board lane. It must be self-contained and independently executable, with a stable `id` used for idempotency.
3. **Session per chunk.** Create/select one session per chunk; `opencode task target select --server <url>` (or explicit `--server`/`--session` options, which override the default) fixes the target. Do not let chunks share a session unless they must run in order.
4. **Dispatch.** `opencode task new` / `opencode task send` submit each chunk's prompt through the canonical `POST /session/{id}/prompt` path.
5. **Concurrency.** The runtime allows one active run per session and concurrency across sessions (`invariants/message-queuing.md`, 1). Dispatch N chunks across N sessions; per-session FIFO preserves order within a chunk (`invariants/message-queuing.md`, 2).
6. **Tracking.** Poll `opencode task status` (`idle|busy|queued`, `CLI-006`) and `opencode task view` to follow each chunk; write status back onto the chunk card.
7. **Idempotency and resume.** Key the dispatcher on chunk `id`, so a rerun does not double-dispatch completed or in-flight chunks.
8. **Provenance.** Each chunk card records the exact Feynman invocation (or research method) that produced it, so the chunk is auditable back to its source.

## Resolved decisions

- **Surface:** `opencode task` (canonical), not `opencode run` (interim, `CLI-002` hold).
- **Concurrency unit:** a session, matching the runtime's per-session FIFO + cross-session concurrency model.
- **Chunk identity:** the board item `id` is the idempotency and status key.
- **Manual-now:** no automation is built by this card yet; it records the contract to build against.

## Open build questions (resolve during implementation)

- Where does the dispatcher live — a Feynman skill/tool, a script in this repo, or a thin wrapper between the two?
- What is the exact chunk schema and which board lane is the source?
- Concurrency bound and backpressure choice.
- How are Feynman-produced chunks validated (frontmatter, independence) before dispatch?
- Failure isolation and retry policy per chunk.
- Exact status write-back format on the card.
- How the dispatcher discovers the `ort` binary, server URL, and workspace.

## Non-goals

- Deciding whether Feynman's research method is worth it (`FEAT-065`).
- Folding Feynman features into `ort`.
- Changing the `ort` CLI/task/queue runtime; this rides existing surfaces.
- Editing aa-studies or syncing `feynman-modded`.
- Auto-merging or auto-completing chunk work; orchestration gates stay human-controlled.

## Done when

- The dispatcher exists and can turn a set of Feynman-produced chunk cards into concurrent `ort` task sessions.
- It is idempotent by chunk id and writes status/provenance back to the cards.
- A dry run over at least one real chunk set is recorded with exact commands and outputs.
- Failure isolation and resume behavior are demonstrated.

## Recommended verification

- Confirm the flow uses `opencode task` on the canonical prompt path and not `opencode run`.
- Verify concurrency: two chunks dispatched to two sessions run in parallel, and two sends to one session preserve FIFO order.
- Kill and rerun the dispatcher mid-flight and confirm no chunk is double-dispatched or silently dropped.
- Confirm each chunk card carries its originating Feynman provenance.

## Related Items

- `FEAT-065` Evaluate whether Feynman's research method beats opencode/ort alone
- `CLI-001` CLI task send (canonical path)
- `CLI-006` CLI status visibility
- `CLI-002` Route `opencode run` through the canonical session runtime (hold; why we avoid `run`)
- `CLI-007` Default task target selection
- `wiki/cli-surface.md`, `invariants/cli-task-targeting.md`, `invariants/message-queuing.md`

## Notes

- The mechanism is intentionally "straightforward": existing `serve` + `task` surfaces already support concurrent per-session work; this card only wires chunks to them.
- Keep the human in the loop for orchestration gates; this automates dispatch and tracking, not merge/completion decisions.

## Re-triage — 2026-09-26 (FEAT-065 null result)

`FEAT-065` completed with a **null result**: Feynman's research method was not
stronger than opencode/`ort` alone for vetting `GATE-001` or finding
improvements, at ~3x latency and higher operational complexity
(`docs/research/FEAT-065-feynman-gate-001-vetting.md`).

This card's premise — automating a Feynman→`ort` chunk pipeline — assumes
Feynman produces chunks worth dispatching. With no demonstrated Feynman
advantage, the automation has no established value case, so this card is moved
to `hold`. The resolved mechanism/contract above is retained for a future
reopen.

**Reopen condition:** a concrete Feynman value case (a task where Feynman's
method demonstrably beats opencode/`ort` alone), or an independent need to
orchestrate board-item chunks through `ort` that does not depend on Feynman's
research advantage (e.g. a general chunk-dispatch need).

## Addendum — 2026-09-27 canonical-replacement decision

`aa-studies/docs/feynman_role_for_optimization.md` is now out of date and
archived at `aa-studies/docs/archive/feynman_role_for_optimization.md` with an
ARCHIVED banner (aa-studies `development`, commit `eb468fd`). This card
(`FEAT-066`) and `FEAT-065` are the canonical artifacts for the Feynman
role/pipeline question; the archived aa-studies doc is retained for provenance
only. This does not change the `hold` status or the reopen condition above.