# Where We Left Off - 2026-11-17 14:16 EST

## Where We Left Off
`H-019`'s delivered work is now reflected on the board: `INFRA-015` moved to `qa` with a
refreshed runbook, `INFRA-014` records the `v0.1.5` fleet verification, and `INFRA-012`'s
attention note no longer cites `v0.1.0`. Releases are at `v0.1.5` (`main` `4292f41`). The
remaining deployment work is owner-gated or environmental. The substantive backlog is the
decision-layer and future-proofing programs in `todo` (58 cards) plus a 65-card `qa` lane that
is 63 items stale.

## Needs Human Intervention
- `INFRA-012` (doing): owner setup for the release-triggered deploy control plane (systemd
  unit or self-hosted runner on `matrillobase1`), plus `moldavite` and `xp-farmer`, which are
  Tailscale-offline and missing the current revision.
- `H-021` (open handoff): owner gate per release/rollout; payload shipped, only the
  control-plane and offline-machine gap remain.
- `H-019` (open handoff): still partial until `INFRA-012` closes and the `qa` items are
  owner-verified.
- 63 stale `qa` cards and 12 intentional `hold` cards.

## Stale Handoffs
- `H-017` (complete) -> archive-ready, needs your OK.
- `H-016` (complete) -> archive-ready, needs your OK.
- `H-019` (open) -> partial: five items in `qa`, `INFRA-012` in `doing`.
- `H-021` (open, needs_human) -> partial, mostly superseded; trimmed to the remaining gap.

## Doing Review
- `INFRA-012`: keep in `doing`; only the owner-gated control plane and two offline machines
  remain. Do not auto-resume the owner-gated part.
- No other `doing` cards.
- Open PR #188 (`BUG-070`, card in `qa`) since 2026-11-11.

## Recommended Order
1. `BUG-070` PR #188 closeout (merge into `development`, clean branches, card stays `qa`
   until owner QA) [quick win].
2. `INFRA-012` remaining control-plane trigger [needs your OK]: choose/record the control host
   mechanism (systemd unit or self-hosted runner on `matrillobase1`), then re-deploy
   `moldavite`/`xp-farmer` once they are online.
3. `BUG-056` (P1) session must suspend on the `question` tool instead of continuing — core
   interactive parity for the daily-driver workflow; investigation diagnostics are recorded.
4. `DECISION-002` (P1) inventory and baseline harness decision points — first card in the
   dependency chain for the 14-card `DECISION` program.

## Observational Gaps (no card yet)
- Hosted macOS runner availability has forced manual asset recovery for `v0.1.0`, `v0.1.3`,
  `v0.1.4`, and `v0.1.5` (now documented in `docs/release-deployment.md`). The release token
  also lacks `actions:write`, so a failed macOS job cannot be re-run from the CLI. Neither is
  tracked as a card.
- The `qa` lane (65 cards, 63 stale) needs a dedicated owner QA sweep or batch closeout.

## Best Next Step
- `BUG-070` PR #188 closeout: it is the only open PR, the card is already in `qa`, and closing
  it is cheap. Step 2 is the natural follow-on but needs your control-plane decision.

## Offer
- I can execute these in order, one at a time, and trim this file as each is done. Proceed?
  [awaiting your OK]

## Completed
- 2026-11-17: `H-019` board closeout — `INFRA-015` moved `doing` -> `qa` with refreshed notes,
  `INFRA-014` records the `v0.1.5` verification, `INFRA-012` attention note rewritten,
  runbook fallback corrected, `H-021` trimmed to the remaining gap, and the loose `H-021` /
  `FEAT-079` / `INFRA-012` files committed.
