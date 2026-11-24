# Where We Left Off - 2026-11-24 13:23 EST

## Where We Left Off
Two more P2 bugs are implemented and awaiting QA/merge. `TUI-001` (vanilla
context percentage and token counters) is merged into `development` via PR #208
(`c6ad0cc`). `BUG-074` (recover complete tool calls from truncated native blocks)
is in PR #209 and `BUG-058` (revise earlier `question` answers) is in PR #210;
both cards are in `qa` and both branches are pushed. `BUG-056` and `INFRA-017`
remain merged and awaiting live QA. No `doing` cards and no live handoffs;
`H-016`, `H-017`, `H-019`, `H-021` are archived. The queue is dominated by the
stale `qa` lane (74 cards) and a 54-card `todo` backlog.

## Needs Human Intervention
- `TUI-001` (qa): live QA — run a real session and confirm header/prompt
  percentage tracks vanilla as the window fills.
- `BUG-074` (qa): live/regression check on PR #209 — a truncated native block
  with a complete inner call must execute; invalid JSON must not.
- `BUG-058` (qa): live check on PR #210 — answer Q1/Q2, Tab back to Q1, edit,
  submit, and confirm the edited answer is used.
- Deployment cards in `qa` (`INFRA-009/010/011/012/013/014/015`, `INFRA-017`):
  owner QA sweep; watch the first automatic `main` release→deploy run.
- `BUG-056` (qa): live subagent `question` must show "Asked by a subagent" in the
  parent TUI, keep the parent parked, and deliver the answer as the tool result.
- `qa` lane overall (74 cards, most with no activity since 2026-10-23): needs a
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
1. Owner QA sweep — `TUI-001`, `BUG-074` (PR #209), `BUG-058` (PR #210), then the
   deployment cards and `BUG-056`; merge the two bug PRs after QA. (human gate)
2. First automatic release→deploy proof — confirm the poller deploys once, reports
   skips, and `release-retry` recovers a failed attempt. (human watch)
3. `INFRA-003` — CI clean-clone native build guard. (P3, `todo`, quick win)

## Best Next Step
- Owner QA sweep first; the only remaining agent-executable item in this list is
  `INFRA-003`, which is a safe quick win to pick up while QA is pending.

## Offer
- I can take these in order, one at a time, and trim this file as each is done.
  The QA sweep, PR merges, and release watch need your OK/eyes; `INFRA-003` is
  safe to start now. Proceed with `INFRA-003`? [awaiting your OK]

## Completed
- 2026-11-24: `BUG-074` truncated native tool-call recovery implemented in PR #209
  (card in `qa`).
- 2026-11-24: `BUG-058` earlier-question answer revision implemented in PR #210
  (card in `qa`).
- 2026-11-24: `TUI-001` vanilla context/token parity merged, PR #208 (`c6ad0cc`),
  branch cleaned; card in `qa` awaiting live QA (`ea85605`).
- 2026-11-20: `INFRA-017` bounded release retry fixed, PR #207 merged (`a059d09`);
  card in `qa` until the next real release proves it.
- 2026-11-18: `BUG-056` Option A implemented, PR #206 merged (`ec5c024`); card in
  `qa` awaiting the live subagent-question check.
- 2026-11-18: `INFRA-012` control plane live (`standards-deploy-ort-poll` on
  `matrillobase1`); `H-019`/`H-021` archived (`e637752`).
- 2026-11-17: `H-019` board closeout (`187dfd8`); `DECISION-002` delivered
  `docs/decision-inventory.md` (`29633e1`).
