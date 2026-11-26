# Where We Left Off - 2026-11-25 22:00 EST

## Where We Left Off
The short recommendation list is complete. `TUI-001` merged via PR #208, then
`BUG-074` (truncated native tool-call recovery, PR #209) and `BUG-058` (revise
earlier `question` answers, PR #210) merged into `development`; all three cards
are in `qa` awaiting live QA. `INFRA-003` (CI clean-clone native build guard) was
closed as already delivered by `ci.yml`/`native-artifacts.yml` now that
`scopemux-core` is public. `BUG-056` and `INFRA-017` remain merged and awaiting
live QA. No `doing` cards and no live handoffs. The queue is dominated by the
stale `qa` lane (~74 cards) and the `todo` backlog (~53 cards).

## Needs Human Intervention
- Live QA for the three TUI/bug merges: `TUI-001` (context percentage fills like
  vanilla), `BUG-074` (truncated block with complete call executes; invalid JSON
  does not), `BUG-058` (Tab back to Q1, edit, submit uses the edit).
- Deployment cards in `qa` (`INFRA-009/010/011/012/013/014/015`, `INFRA-017`):
  owner QA sweep; watch the first automatic `main` release→deploy run.
- `BUG-056` (qa): live subagent `question` must show "Asked by a subagent" in the
  parent TUI, keep the parent parked, and deliver the answer as the tool result.
- `qa` lane overall (most cards have no activity since 2026-10-23): needs a
  dedicated owner QA sweep or batch closeout; `bd --check-attention` lists them.
- `hold` and `refinement` lanes have stale, human-clearance items (`BUG-035`,
  `BUG-057`, `CLI-002`, `CLI-009`, `FEAT-036`, `FEAT-062`, `RESEARCH-002`,
  `START-020…024`, `DECISION-001`, `EPIC-001`, `FUTURE-PROOFING-001/013`,
  `SKILLS-002/004`, `START-029`). Their `bd` flags are staleness, not explicit
  `attention:` reasons.

## Stale Handoffs
- None live. `handoffs/` contains only `handoffs/archive/` (populated); no
  non-archived handoff represents pending work.
- Untracked branch note: `origin/fix/START-036-image-attachments` still exists with
  no open PR or board card activity on it; not deleted (out of scope).

## Doing Review
- No cards in `doing`.

## Recommended Order
1. Owner QA sweep — `TUI-001`, `BUG-074`, `BUG-058`, then the deployment cards
   and `BUG-056`; complete or archive `qa` cards that pass. (human gate)
2. First automatic release→deploy proof — confirm the poller deploys once, reports
   skips, and `release-retry` recovers a failed attempt. (human watch)
3. Next agent-executable candidates when QA is not the blocker: `BUG-050`
   (permanent-allow config write failures), `FEAT-069` (Flameox CPU profiling),
   or a `DECISION-*` Wave 1 gate. (P2/P3, `todo`)

## Best Next Step
- Owner QA sweep across the freshly merged `qa` cards; no short-list
  agent-executable item remains.

## Offer
- The short list is done. If you want, I can pick the next `todo` card
  (`BUG-050` is the top P2 bug) and run it through `board-item-to-pr`. [awaiting
  your OK]

## Completed
- 2026-11-25: `BUG-074` truncated native tool-call recovery, PR #209 merged
  (`97a4a4b`); branch deleted; card in `qa`.
- 2026-11-25: `BUG-058` earlier-question answer revision, PR #210 merged
  (`5130c61`); branch deleted; card in `qa`.
- 2026-11-25: `INFRA-003` closed as delivered by `ci.yml` + `native-artifacts.yml`
  (public `scopemux-core`, no secret needed); documented in `AGENTS.md`.
- 2026-11-24: `TUI-001` vanilla context/token parity, PR #208 merged (`c6ad0cc`);
  card in `qa` awaiting live QA.
- 2026-11-20: `INFRA-017` bounded release retry, PR #207 merged (`a059d09`);
  card in `qa` until the next real release proves it.
- 2026-11-18: `BUG-056` Option A implemented, PR #206 merged (`ec5c024`); card in
  `qa` awaiting the live subagent-question check.
- 2026-11-18: `INFRA-012` control plane live (`standards-deploy-ort-poll` on
  `matrillobase1`); `H-019`/`H-021` archived (`e637752`).
- 2026-11-17: `H-019` board closeout (`187dfd8`); `DECISION-002` delivered
  `docs/decision-inventory.md` (`29633e1`).
