# Where We Left Off - 2026-11-17 14:16 EST

## Where We Left Off
The main-release deployment program (`H-019`) is fully reflected on the board: `INFRA-015`
is in `qa`, `INFRA-014` records the `v0.1.5` verification, and `INFRA-012` is down to its
owner-gated control plane. `BUG-070`'s follow-up PR #188 is merged (`a495356`), and
`DECISION-002` produced `docs/decision-inventory.md`. Both `complete` handoffs are archived.
Only owner-gated work and one owner decision remain on the previously ranked list.

## Needs Human Intervention
- `INFRA-012` (doing): owner setup for the release-triggered deploy control plane (systemd
  unit or self-hosted runner on `matrillobase1`), plus `moldavite` and `xp-farmer`, which are
  Tailscale-offline and missing the current revision.
- `BUG-056` (todo): needs a decision on where a subagent question surfaces before any code
  change (option A parent-side aggregation vs option B withhold the tool). See the card's
  "Decision needed before implementation" section.
- `H-021` (open handoff): owner gate per release/rollout; only the control-plane and
  offline-machine gap remain.
- `H-019` (open handoff): partial until `INFRA-012` closes and the `qa` items are
  owner-verified.
- 63 stale `qa` cards and 12 intentional `hold` cards.

## Stale Handoffs
- `H-017`, `H-016` -> archived 2026-11-17 (both `complete`, items merged).
- `H-019` -> partial; `H-021` -> partial and trimmed to the remaining gap.

## Doing Review
- `INFRA-012`: keep in `doing`; owner-gated remainder only.
- No other `doing` cards and no open PRs.

## Remaining
1. `INFRA-012` control plane [needs your OK] — choose/record the mechanism (systemd unit or
   self-hosted runner on `matrillobase1`), then re-deploy `moldavite`/`xp-farmer` when online.
2. `BUG-056` [needs your OK] — pick option A or B, then implement the suspend-on-question
   path for every tool-capable runner.
3. Close `INFRA-012`/`INFRA-015`/`H-019`/`H-021` once the owner QA sweep confirms the
   `qa` deployment cards.

## Observational Gaps (no card yet)
- Hosted macOS runner availability has forced manual asset recovery for `v0.1.0`, `v0.1.3`,
  `v0.1.4`, and `v0.1.5` (documented in `docs/release-deployment.md`). The release token also
  lacks `actions:write`, so a failed macOS job cannot be re-run from the CLI. Neither is
  tracked as a card.
- The `qa` lane (65 cards, 63 stale) needs a dedicated owner QA sweep or batch closeout.

## Best Next Step
- None that can proceed without you. The two highest-value items are `INFRA-012`'s control
  plane and `BUG-056`'s surfacing decision; both need your call first.

## Completed
- 2026-11-17: `H-019` board closeout — `INFRA-015` moved `doing` -> `qa`, `INFRA-014` records
  the `v0.1.5` verification, `INFRA-012` attention note rewritten, runbook fallback
  corrected, `H-021` trimmed, loose `H-021`/`FEAT-079`/`INFRA-012` files committed (`187dfd8`).
- 2026-11-17: `BUG-070` follow-up PR #188 merged (`a495356`); branch, worktree, and remote
  refs cleaned; card stays in `qa` with a merge closeout note (`7c21903`).
- 2026-11-17: `DECISION-002` delivered `docs/decision-inventory.md` (18 rows, Wave 1 fully
  specified) and moved to `qa`.
- 2026-11-17: archived `complete` handoffs `H-017` and `H-016`; refreshed `BUG-056` with
  confirmed code evidence and the surfacing decision it needs.
