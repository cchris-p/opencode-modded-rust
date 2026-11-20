# Where We Left Off - 2026-11-20 13:40 EST

## Where We Left Off
Everything that was ranked and could move without an owner decision has moved. The
`H-019` deployment program is delivered: `INFRA-012`'s release-triggered control plane
(`standards-deploy-ort-poll`) runs on `matrillobase1` every 5 minutes, `moldavite` and
`xp-farmer` are intentionally `pending`, and every program card is in `qa`. `BUG-070`'s
follow-up merged, `DECISION-002` produced `docs/decision-inventory.md`, and `BUG-056`'s
Option A fix is PR #206.

## Needs Human Intervention
- Owner QA sweep for the deployment cards now in `qa` (`INFRA-009/010/011/012/013/014/015`).
- `BUG-056` live QA: a real subagent `question` must show "Asked by a subagent" in the
  parent TUI, keep the parent parked, and deliver the answer as the tool result (merged,
  awaiting that check).
- Watch the first automatic deploy on the next `main` release.

## Stale Handoffs
- None live. `H-016`, `H-017`, `H-019`, `H-021` are archived.

## Doing Review
- No cards in `doing`.

## Remaining
1. Owner QA sweep for the `qa` deployment cards and the other stale `qa` cards.
2. Live QA for `BUG-056`: a real subagent `question` must show "Asked by a subagent", keep
   the parent parked, and deliver the answer as the tool result.
3. First automatic release→deploy run: confirm the poller deploys once and reports skips.

## Observational Gaps (no card yet)
- The `qa` lane (67 cards, most stale) needs a dedicated owner QA sweep or batch closeout.

## Best Next Step
- Owner QA: run the live subagent-question check for `BUG-056`, then sweep the `qa` lane.

## Completed
- 2026-11-17: `H-019` board closeout (`187dfd8`).
- 2026-11-17: `BUG-070` follow-up PR #188 merged (`a495356`, `7c21903`).
- 2026-11-17: `DECISION-002` delivered `docs/decision-inventory.md` (`29633e1`).
- 2026-11-18: `INFRA-012` control plane live — `standards-deploy-ort-poll` (standards
  commit `66fdd41`), cron on `matrillobase1`, `moldavite`/`xp-farmer` set `pending`;
  product docs/cards updated and `H-019`/`H-021` archived (`e637752`).
- 2026-11-18: `BUG-056` Option A implemented, PR #206 merged (`ec5c024`), branch/worktree
  cleaned; card in `qa` awaiting the live subagent-question check.
- 2026-11-20: carded the macOS-runner/`actions:write` gap as `INFRA-017` and fixed it —
  PR #207 merged (`a059d09`): automatic bounded release retry with the built-in token,
  idempotent publish, `fail-fast: false`. Card in `qa` until the next real release proves it.
