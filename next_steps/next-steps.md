# Where We Left Off - 2026-11-06

## Where We Left Off

`H-018` (remaining ScopeMux program) is **paused before `SCOPE-006`**. Steps 1-6 are
complete and merged; `SCOPE-003`, `SCOPE-012`, `SCOPE-013`, `SCOPE-014`, `SCOPE-004`, and
`SCOPE-010` are in `done` with QA evidence.

`SCOPE-006` was verified blocked on 2026-11-06: the Rust provider creates a fresh
`ProjectContext` per retrieval, registers only seeds/changed files, and has no persistent
whole-workspace registry or watcher. A map-backed code-search tool cannot meet its
"without a filesystem walk" criterion, and `SCOPE-009` cannot measure real freshness.
Evidence: PR #162 (merged `dac31ee`), `SCOPE-006` card, and
`wiki/scopemux-integration-plan.md`.

**Decision A is locked:** build the persistent map. The prerequisite is its own program,
`H-020`, with cards `SCOPE-015`/`018`/`016`/`017`/`019`.

## Needs Human Intervention

- `SCOPE-006` (`doing`): blocked by the persistent-map prerequisite; `attention:` set.
- `SCOPE-009` (`todo`): deferred until the persistent map is integrated; `attention:` set.
- `H-018`: paused with `needs_human`; resume only after `SCOPE-019` merges.
- Future-proofing remains on hold: do not touch `FUTURE-PROOFING-*` or `DECISION-*`.
- Do not start `SCOPE-007`, `SCOPE-008`, or `SCOPE-011`.

## Best Next Step

Execute handoff `H-020`
(`handoffs/H-020-persistent-workspace-map-program-handoff.md`), in this order:

1. `SCOPE-018` shared contracts/policy
2. `SCOPE-016` safe engine/persistent context
3. `SCOPE-017` map supervisor/watcher
4. `SCOPE-019` provider/tool integration

Then resume `H-018` at `SCOPE-006` (map-backed code search with grep fallback) and
`SCOPE-009` (freshness/staleness SLOs).

## Session Kickoff Prompt

Paste this into a new session started in `~/apps/opencode-modded-rust`:

```text
Execute handoff H-020 (handoffs/H-020-persistent-workspace-map-program-handoff.md) using
the handoff-to-pr skill. Implement the 4 prerequisite steps in the documented order, one at
a time, then resume H-018 at SCOPE-006 and SCOPE-009.

Scope boundary: future-proofing is on hold. Do not touch any FUTURE-PROOFING-* or
DECISION-* card, and do not start SCOPE-007, SCOPE-008, or SCOPE-011.

Workflow: claim a role with
  python3 ~/standards/opencode/scripts/agent_roles.py claim --scope "SCOPE-015,016,017,018,019,006,009" --items "SCOPE-018,SCOPE-016,SCOPE-017,SCOPE-019,SCOPE-006,SCOPE-009"
Use a dedicated worktree under $HOME/worktrees/opencode-modded-rust/persistent-map/ for
implementation PRs, commit board/handoff updates directly on development in the main
checkout, and target every PR at development. Never push main.

After each implementation PR, stop and ask me to review and explicitly approve the merge
before starting the next step. Run the per-step verification in H-020 and paste the exact
commands and results into the card and PR body.

Known environment trap: the shared worktree target can serve a stale opencode-scopemux
artifact after switching branches. Run `cargo clean -p opencode-scopemux` before native
tests when behavior looks like an older revision.
```

## Other Independent Work

- `H-019` main-release fleet deployment program is `open` and independent of this program.
- `INFRA-008` portable ScopeMux query bundle is independent.

## Notes

`SCOPE-005`'s local-Qwen-only activation remains the interim rule. The generic provider
stays the default/fallback. Task stage/verification/review/completion authority stays with
the runtime; the map is evidence-only.
