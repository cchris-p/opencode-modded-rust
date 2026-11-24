# Where We Left Off - 2026-11-24 10:23 EST

## Where We Left Off
The board moved forward on TUI fidelity: `TUI-001` (vanilla context percentage and
token counters) is implemented and merged into `development` via PR #208 (`c6ad0cc`),
with the branch deleted. That work replaced the sidebar's session-wide token sum
with vanilla's last-assistant-message semantics and unified the header, sidebar,
and prompt footer on one formatter. `BUG-056` and `INFRA-017` remain merged and
still awaiting live QA. No `doing` cards and no live handoffs; `H-016`, `H-017`,
`H-019`, `H-021` are archived. The queue is now dominated by a large, stale `qa`
lane (72 cards) and a 56-card `todo` backlog.

## Needs Human Intervention
- `TUI-001` (qa): live QA — run a real session and confirm header/prompt percentage
  tracks vanilla as the window fills; no QA report recorded yet.
- Deployment cards in `qa` (`INFRA-009/010/011/012/013/014/015`, `INFRA-017`):
  owner QA sweep; watch the first automatic `main` release→deploy run.
- `BUG-056` (qa): live subagent `question` must show "Asked by a subagent" in the
  parent TUI, keep the parent parked, and deliver the answer as the tool result.
- `qa` lane overall (72 cards, most with no activity since 2026-10-23): needs a
  dedicated owner QA sweep or batch closeout; staleness flags are reported by
  `bd --check-attention`.
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
1. Owner QA sweep — start with `TUI-001` live percentage check, then the deployment
   cards and `BUG-056`; archive or complete `qa` cards that have actually passed.
   (human gate)
2. `BUG-074` — truncated native tool-call block silently drops the intended file
   write; clear acceptance criteria and an end-to-end regression test make this the
   strongest agent-executable bug. (P2, `todo`)
3. First automatic release→deploy proof — confirm the poller deploys once, reports
   skips, and `release-retry` recovers a failed attempt. (human watch)
4. `BUG-058` — let users revise earlier `question` answers instead of the single
   one-step back; parent run-parking is owned by `BUG-056`. (P2, `todo`)
5. `INFRA-003` — CI clean-clone native build guard. (P3, `todo`, quick win)

## Best Next Step
- Owner QA sweep: run the live `TUI-001` context check, then clear the stale `qa`
  lane; the first agent-executable item afterwards is `BUG-074`.

## Offer
- I can take these in order, one at a time, and trim this file as each is done.
  The QA sweep and release watch need your OK/eyes; `BUG-074` and `INFRA-003` are
  safe to start now. Proceed with `BUG-074`? [awaiting your OK]

## Completed
- 2026-11-24: `TUI-001` vanilla context/token parity implemented, PR #208 merged
  (`c6ad0cc`), branch cleaned; card in `qa` awaiting live QA (`ea85605`).
- 2026-11-20: `INFRA-017` bounded release retry fixed, PR #207 merged (`a059d09`);
  card in `qa` until the next real release proves it.
- 2026-11-18: `BUG-056` Option A implemented, PR #206 merged (`ec5c024`); card in
  `qa` awaiting the live subagent-question check.
- 2026-11-18: `INFRA-012` control plane live (`standards-deploy-ort-poll` on
  `matrillobase1`); `H-019`/`H-021` archived (`e637752`).
- 2026-11-17: `H-019` board closeout (`187dfd8`); `DECISION-002` delivered
  `docs/decision-inventory.md` (`29633e1`).
