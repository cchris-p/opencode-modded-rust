# Where We Left Off - 2026-11-05 13:45 EST

## Where We Left Off

ScopeMux is integrated and usable: the native provider ships by default, activation is still
the local-Qwen-only gate, and the ScopeMux/LSP ownership boundary plus optional exact-fact
enrichment have landed on `development`. The Workspace Intelligence Protocol program
(`FUTURE-PROOFING-013`) is now **on hold by owner direction**, so every `FUTURE-PROOFING-*`
gate, profile-driven activation, artifact packaging, keep/expand/retire evaluation, and
System-1 routing are out of scope until further notice. What remains is the `SCOPE-*` work:
five cards in `qa` and six in `todo`. The `doing` lane is empty.

## Needs Human Intervention

- `FUTURE-PROOFING-013` Workspace Intelligence Protocol program (`refinement`): on hold by
  owner. Not ranked and not resumed.
- `SCOPE-004` data-flywheel/slice card: intentionally parked in `qa` by its own closeout
  until repair-context depth and prompt-budget injection land, or the owner explicitly
  completes it. Needs an owner decision: finish the follow-ups or split them out and ship.
- `H-016` handoff (`handoffs/SCOPE-LSP-responsibility-boundary-handoff.md`): all covered PRs
  (#148, #149, #150, #152, #154) are merged and the handoff is `complete`, so it is
  archive-ready. Not archived because this request was report-only; needs approval.

## Stale Handoffs

- `handoffs/SCOPE-LSP-responsibility-boundary-handoff.md` (`H-016`): archive-ready ->
  awaiting approval to move to `handoffs/archived/`.
- `handoffs/H-017-deepseek-api-efficiency-handoff.md`: `complete`; out of ScopeMux scope,
  also archive-ready. Left in place.

## Doing Review

- None. `boards/doing/` is empty.

## QA / Closeout (code merged, lane not reconciled)

- `SCOPE-003` project task state into plan nodes: PR #107 merged 2026-09-24; the card note
  still says "open" (stale). No QA section on the card; needs a QA pass before promotion.
- `SCOPE-004` map delta/observability/refactor nodes: PR #124 merged; agent self-QA recorded
  2026-09-26, but intentionally kept in `qa` pending prompt-budget injection and the repair
  `reopen_reason` check.
- `SCOPE-012` ScopeMux/LSP boundary: PR #148 merged; boundary docs, architecture guard, and
  tool-surface guard recorded as passing. Promotion candidate.
- `SCOPE-013` exact type facts: PR #152 merged; native tests pass, unavailable facts are
  byte-identical to baseline. Two known pre-existing native segfault tests reproduced on
  base. Promotion candidate.
- `SCOPE-014` exact-fact value: PR #154 merged; evaluation recorded and **exact-fact
  enrichment retired from default**, opt-in via `OPENCODE_SCOPEMUX_EXACT_FACTS=1`.
  Promotion candidate (decision is recorded).

## Recommended Order

1. `SCOPE-003` closeout - fix the stale "PR open" note, run the QA/verification pass, then
   promote to `done` if it passes. [quick win]
2. `SCOPE-012`, `SCOPE-013`, `SCOPE-014` - one reconciliation pass over the recorded QA
   evidence, then promote each to `done` (or leave with the exact failed step). [quick win]
3. `SCOPE-004` follow-ups - implement prompt-budget slice injection and the repair
   `reopen_reason`-to-anchor check, then promote. If the owner prefers to ship the current
   slice, split the follow-ups into their own cards instead.
4. `SCOPE-010` language coverage and fallback matrix - independent research card that
   defines the unsupported-language and markdown/prose fallback semantics `SCOPE-006`
   consumes. [quick win]
5. `SCOPE-006` code-grep -> ScopeMux node search with grep fallback - first verify node
   registration/periodic update empirically; the card's `WI-030` Rust-grammar blocker
   appears stale (the provider now maps `.rs` and has a Rust candidate test), so confirm
   before planning around it. [highest-value independent feature]
6. `SCOPE-009` map freshness/staleness SLOs - independent definition work; the
   `FUTURE-PROOFING-017` link only gates evaluation, not the SLO itself.

## On Hold With Future-Proofing

- `SCOPE-007` subagent-scoped intelligence and per-subagent budgets - tied to
  `FUTURE-PROOFING-015` profile-driven activation.
- `SCOPE-008` retrieval provenance/outcomes into the data flywheel - depends on
  `FUTURE-PROOFING-009` flywheel contract and `FUTURE-PROOFING-017`.
- `SCOPE-011` out-of-process transport - depends on `FUTURE-PROOFING-014` protocol and
  `FUTURE-PROOFING-016` artifact packaging.

## Independent (Not In H-018)

- `INFRA-008` Embed ScopeMux queries so the published ort binary is portable - the native
  provider is statically linked, but the Tree-sitter `.scm` queries are resolved from a
  build-machine absolute path that does not exist on artifact-only hosts, so ScopeMux
  silently falls back to generic. Implemented on
  `feature/INFRA-008-embed-scopemux-queries`; PR #160 open, card in `doing`. Independent of
  the program handoff.

## Best Next Step

The program is packaged as handoff `H-018`
(`handoffs/H-018-scopemux-remaining-program-handoff.md`). Start a separate session and
execute it top to bottom; the first step is the `SCOPE-003` closeout.

Note: `SCOPE-005`'s local-Qwen-only activation remains the interim activation rule for the
whole product until the future-proofing program resumes.

## Session Kickoff Prompt

Paste this into a new session started in `~/apps/opencode-modded-rust`:

```text
Execute handoff H-018 (handoffs/H-018-scopemux-remaining-program-handoff.md) using the
handoff-to-pr skill. Implement all 8 steps in the documented order, one at a time, and
update the board cards and this handoff as you go.

Scope boundary: future-proofing is on hold. Do not touch any FUTURE-PROOFING-* or
DECISION-* card, and do not start SCOPE-007, SCOPE-008, or SCOPE-011.

Workflow: claim a role with
  python3 ~/standards/opencode/scripts/agent_roles.py claim --scope "SCOPE-003,004,006,009,010,012,013,014" --items "SCOPE-003,SCOPE-012,SCOPE-013,SCOPE-014,SCOPE-004,SCOPE-010,SCOPE-006,SCOPE-009"
Use a dedicated worktree for implementation PRs, commit board/handoff updates directly on
development in the main checkout, and target every PR at development. Never push main.

After each implementation PR, stop and ask me to review and explicitly approve the merge
before starting the next step. Run the per-step verification in the handoff and paste the
exact commands and results into the card and PR body.
```

## Offer

Handoff `H-018` is created. Start the separate session with the prompt above; I can also
run individual steps here if you prefer. [awaiting your OK]
