---
id: "H-018"
title: "SCOPE-003/004/006/009/010/012/013/014 ScopeMux Remaining Program — Program Handoff"
status: "open"
created: "2026-11-05"
updated: "2026-11-05"
owner: ""
target: "opencode-modded-rust"
blocked_reason: ""
needs_human: ""
items: ["SCOPE-003", "SCOPE-012", "SCOPE-013", "SCOPE-014", "SCOPE-004", "SCOPE-010", "SCOPE-006", "SCOPE-009"]
---

# SCOPE-003/004/006/009/010/012/013/014 ScopeMux Remaining Program — Program Handoff

## Current State

ScopeMux is integrated and usable in `opencode-modded-rust`:
- the native provider is built by default (`opencode-cli` `scopemux-native`);
- activation is still the hardcoded local-Qwen gate in
  `crates/opencode-session/src/retrieval.rs:102`;
- the ScopeMux/LSP ownership boundary and optional exact-fact enrichment are merged into
  `development`;
- exact LSP facts are **retired from default activation** (`SCOPE-014`), opt-in via
  `OPENCODE_SCOPEMUX_EXACT_FACTS=1`.

**Future-proofing is on hold by owner direction.** Do not touch `FUTURE-PROOFING-*`,
`DECISION-*`, `SCOPE-007`, `SCOPE-008`, or `SCOPE-011`. The Qwen-only gate stays as the
interim activation rule; no capability profiles, artifact packaging, flywheel contract,
System-1 routing, or out-of-process transport in this program.

Remaining ScopeMux work is four `qa` closeouts plus four implementation cards:

| Board ID | Title | Priority | Status | Type |
|---|---|---|---|---|
| `SCOPE-003` | Project authoritative task state into scopemux plan nodes | P2 | qa | closeout |
| `SCOPE-012` | Consolidate ScopeMux/LSP responsibilities | P1 | qa | closeout |
| `SCOPE-013` | Ingest LSP-derived exact type facts | P2 | qa | closeout |
| `SCOPE-014` | Measure the context value of exact facts | P2 | qa | closeout |
| `SCOPE-004` | Consume map delta/observability/refactor nodes | P3 | qa | follow-up PR |
| `SCOPE-010` | Language coverage and fallback matrix | P2 | todo | new PR |
| `SCOPE-006` | Replace code grep with ScopeMux node search | P2 | todo | new PR |
| `SCOPE-009` | Map freshness and staleness SLOs | P2 | todo | new PR |

`SCOPE-001`, `SCOPE-002`, and `SCOPE-005` are done. `boards/doing/` is empty.

## The 8 Steps, In Order

| # | Board ID | Work | Kind | Branch |
|---|---|---|---|---|
| 1 | `SCOPE-003` | QA pass, stale-note fix, promote if green | board closeout | none |
| 2 | `SCOPE-012` | Re-run boundary guards, record, promote | board closeout | none |
| 3 | `SCOPE-013` | Re-run exact-fact tests, record, promote | board closeout | none |
| 4 | `SCOPE-014` | Confirm evaluation report/decision, promote | board closeout | none |
| 5 | `SCOPE-004` | Prompt-budget injection + repair re-anchor | implementation PR | `feature/SCOPE-004-followups` |
| 6 | `SCOPE-010` | Coverage/fallback matrix + unsupported provenance | implementation PR | `feature/SCOPE-010-language-coverage-matrix` |
| 7 | `SCOPE-006` | Map-backed code search with grep fallback | implementation PR | `feature/SCOPE-006-scopemux-code-search` |
| 8 | `SCOPE-009` | Freshness states, SLOs, stale fallback | implementation PR | `feature/SCOPE-009-map-freshness-slo` |

Dependency order (strict): `SCOPE-003` -> `SCOPE-012`/`SCOPE-013`/`SCOPE-014` ->
`SCOPE-004` -> `SCOPE-010` -> `SCOPE-006` -> `SCOPE-009`.

Rationale for the one reorder versus `next_steps/next-steps.md`: `SCOPE-010` defines the
unsupported-language and markdown/prose fallback semantics that `SCOPE-006` consumes, so it
runs first. `SCOPE-006` is otherwise independent of `SCOPE-010`'s implementation.

`SCOPE-003`/`012`/`013`/`014` have merged code; they need QA reconciliation, not new
feature PRs. `SCOPE-004` is already in `qa` but needs its two deferred follow-ups.

## Operating Model — One PR at a Time, Human-Gated

Repeat this cycle for each step.

1. **Gate check:** confirm the previous PR (if any) is merged into `development` and the
   code repo `development` is current.
2. **Branch (implementation steps only):** `git fetch origin development` and create the
   step's branch from `origin/development`.
3. **Board:** move the card to `doing` (main checkout, direct on `development`) before
   implementation; for closeout steps, keep it in `qa` while verifying.
4. **Implement + verify:** implement, run the per-item verification, record results on the
   card.
5. **Open PR (implementation steps):** open a ready-to-review PR against `development`;
   append branch, PR URL, verification, and known limitations to the card; keep the card
   in `doing` until merge.
6. **STOP (human):** ask the human liaison to review and explicitly approve the merge.
   Never merge on tests alone. Never start step N+1 before step N's PR is merged.
7. **After merge:** move the card to `qa`, delete the merged branch, pull `development`,
   run the post-merge QA, then promote to `done` only with recorded QA evidence.
8. **Closeout steps:** run the QA, record `## QA Verification` with exact commands and
   results, then move `qa` -> `done`. If evidence is already sufficient, re-confirm it in
   one short pass rather than inventing new work.

Commit board/handoff updates directly on `development` in the main checkout. Commit code
on the feature branch. Do not push `main`.

## Shared Decisions — Lock Early

- **Future-proofing out of scope.** No `FUTURE-PROOFING-*`, no capability profiles, no
  decision-layer routing, no artifact packaging, no out-of-process transport.
- **Provider default/fallback:** the generic repository-local provider stays the default
  for every model except the local Qwen profile, and the fallback whenever ScopeMux is
  absent, unsupported, or fails.
- **Exact LSP facts stay retired from default** (`SCOPE-014`). Do not re-enable
  `OPENCODE_SCOPEMUX_EXACT_FACTS` by default; opt-in only.
- **Authority:** the map is evidence-only. The runtime remains the sole writer of task
  stage, verification, review, and completion.
- **Rust is supported by `scopemux-core` now** (`language_for_path("*.rs") == Some(6)` and
  a Rust candidate test exists). The older "Rust not supported / `WI-030`" text in cards is
  stale; verify, then correct the card text rather than planning around it.
- **Markdown/prose is not code.** Coverage/fallback semantics come from `SCOPE-010` and are
  reused by `SCOPE-006`.
- **LSP owns point semantics and diagnostics; ScopeMux owns locate/search/slice/delta/
  review.** Enforced by `crates/opencode-core/tests/scope_lsp_boundary.rs`.
- **Keep the generic provider working at every step.** Any new signal is additive and
  optional.

## Environment

- Main checkout for board/QA: `$HOME/apps/opencode-modded-rust` on `development`.
- Implementation worktree root: `$HOME/worktrees/opencode-modded-rust/scopemux-program`
  (a dedicated worktree; this repo requires a separate worktree for ScopeMux PR flows).
  Run `scripts/setup-worktree-build-cache.sh` once so worktree builds share the target
  cache.
- Build with `ort-build` when a manual TUI run is needed; `ort` serves the most recent
  binary, so always rebuild before a manual check.
- Native tests: `cargo test -p opencode-scopemux --features native` (loads the prebuilt
  bundle via `scripts/ensure-native-artifacts.sh`; set `OPENCODE_RUST_PREBUILT_NATIVE=0`
  only for a source build).
- Fast workspace checks: `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p <crate>`.
- QA harness: `scripts/scopemux-qa-check.sh` and `scripts/scopemux-qa-server.sh`.

## Per-Item Runbooks

### Step 1 — `SCOPE-003` closeout (board only)

Card: `boards/qa/project-task-state-into-scopemux-plan-nodes.md`.

State: PR #107 **merged** 2026-09-24 into `development`; the card's Implementation section
still says "open" and there is no QA section.

Work:
- Correct the stale "PR #107 (open)" text to merged.
- Run the verification below and add a `## QA Verification` section with exact commands,
  commit, and results.
- Known risk: `SCOPE-013`'s QA recorded `native_provider_reports_plan_reconciliation_evidence`
  (a native plan-reconciliation test) segfaulting on base `development`. Reproduce on the
  current base first. If it still segfaults, that is a real blocker: fix it in a small PR
  under this step and keep the card in `qa`, or leave it in `qa` with the exact failing
  test recorded. Do not promote on a segfault.
- Promote `qa` -> `done` only if the reconciliation tests pass.

Touchpoints:
- `crates/opencode-types/src/plan.rs` (`project_task_to_plan_nodes`, lifecycle/signals)
- `crates/opencode-types/src/retrieval.rs` (`plan_nodes`, `plan_signals`)
- `crates/opencode-scopemux/src/lib.rs` (plan-node FFI, reconcile, native tests)
- `crates/opencode-session/src/retrieval.rs` (provider selection and response handling)

Verify:
```bash
cargo test -p opencode-types -p opencode-retrieval
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-session
cargo test -p opencode-scopemux --features native
cargo fmt --all --check
```

### Step 2 — `SCOPE-012` closeout (board only)

Card: `boards/qa/Consolidate ScopeMux and LSP responsibilities and remove duplicate point-query paths.md`.

State: PR #148 merged; docs and two guards are on `development`.

Work: re-run the guards, add or refresh `## QA Verification`, promote to `done`.

Touchpoints:
- `wiki/scopemux-lsp-boundary.md`
- `wiki/workspace-intelligence.md`
- `wiki/scopemux-integration-plan.md`
- `wiki/scopemux-map-integration.md`
- `crates/opencode-core/tests/scope_lsp_boundary.rs`
- `crates/opencode-tool` ownership/tool-surface tests

Verify:
```bash
cargo test -p opencode-core --test scope_lsp_boundary
cargo test -p opencode-tool ownership_tests
cargo fmt --all --check
```

### Step 3 — `SCOPE-013` closeout (board only)

Card: `boards/qa/Ingest LSP-derived exact type facts into the ScopeMux map.md`.

State: PR #152 merged; exact facts are opt-in only.

Work: re-run the native exact-fact tests, confirm the two documented pre-existing native
segfault tests on base, record results, promote to `done`.

Touchpoints:
- `crates/opencode-retrieval/src/lib.rs` (`ExactFactProvider`)
- `crates/opencode-lsp/src/lib.rs` (`LspExactFactProvider`)
- `crates/opencode-scopemux/src/lib.rs` (`enrich_with_exact_facts`, bounded anchors)
- `crates/opencode-session/src/prompt.rs` (opt-in env gate)

Verify:
```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-retrieval -p opencode-scopemux
cargo test -p opencode-scopemux --features native
cargo fmt --all --check
```

### Step 4 — `SCOPE-014` closeout (board only)

Card: `boards/qa/Measure the context value of LSP-derived semantic facts.md`.

State: PR #154 merged; `docs/research/SCOPE-014-exact-facts-evaluation.md` records the
three-arm evaluation and the **retire from default** decision.

Work: confirm the report, harness, and opt-in gate still match the card. Re-run only if the
code changed since the report. Add a short QA confirmation and promote to `done` (the
decision is the deliverable).

Touchpoints:
- `docs/research/SCOPE-014-exact-facts-evaluation.md`
- `crates/opencode-scopemux/examples/evaluate_exact_facts.rs`
- `crates/opencode-session/src/prompt.rs` (`OPENCODE_SCOPEMUX_EXACT_FACTS`)

Verify:
```bash
cargo run -p opencode-scopemux --example evaluate_exact_facts --features native -- --help
rg -n "OPENCODE_SCOPEMUX_EXACT_FACTS" crates/opencode-session/src/prompt.rs
```

### Step 5 — `SCOPE-004` follow-ups (implementation PR)

Card: `boards/qa/consume-map-delta-observability-refactor-nodes.md`.

State: PR #124 merged and self-QA recorded, but the card is intentionally still `qa` for two
deferred follow-ups:

1. **Prompt-budget injection.** Retrieval candidates are currently recorded as message
   metadata only in `crates/opencode-session/src/prompt.rs` (provider, representation,
   candidate paths). Inject the selected slice into prompt context under
   `RetrievalRequest::token_budget` without handing the map authority over inclusion.
2. **Repair re-anchor check.** Run the repair cycle end to end so `reopen_reason` maps to
   affected anchors and prior plan nodes, and record the result.

Touchpoints:
- `crates/opencode-session/src/prompt.rs` (candidate consumption -> prompt context)
- `crates/opencode-session/src/retrieval.rs`
- `crates/opencode-types/src/retrieval.rs` (`token_budget`, `reopen_reason`, slices)
- `crates/opencode-retrieval/src/lib.rs`
- `crates/opencode-scopemux/src/lib.rs` (`Impact`/`Review` slices, plan anchors)

Verify:
```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-session -p opencode-retrieval
cargo test -p opencode-scopemux --features native
cargo fmt --all --check
```
Add a token-budget assertion and a repair/reopen-reason anchor test. Keep the generic
provider behavior unchanged.

### Step 6 — `SCOPE-010` coverage/fallback matrix (implementation PR)

Card: `boards/todo/Define language coverage and fallback matrix for workspace intelligence.md`.

Work: write the coverage matrix and implement the explicit fallback semantics. This is a
research card with a small code component; do not add grammars.

Required outputs:
- Coverage matrix in `wiki/workspace-intelligence.md` for C, C++, Python, JavaScript,
  TypeScript, Rust (first-class with known resolution gaps), markdown/prose (generic
  search), and unsupported languages.
- Mixed-language workspace behavior: per-file detection, partial coverage.
- Provenance field distinguishing "unsupported language" from "no matches".
- Tests for supported, partially supported, and unsupported workspaces.

Touchpoints:
- `wiki/workspace-intelligence.md`
- `wiki/scopemux-integration-plan.md`
- `crates/opencode-scopemux/src/lib.rs` (`language_for_path`, `workspace_supported`)
- `crates/opencode-types/src/retrieval.rs` (provenance/fallback reason)
- `crates/opencode-retrieval/src/lib.rs` (generic fallback)

Verify:
```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-scopemux -p opencode-retrieval
cargo test -p opencode-scopemux --features native
cargo fmt --all --check
```

### Step 7 — `SCOPE-006` map-backed code search (implementation PR)

Card: `boards/todo/scope-006-replace-code-grep-with-scopemux-node-search.md`.

First, empirically verify node registration and periodic update on a real codebase (this
repo once `language_for_path("*.rs") == Some(6)` is confirmed; otherwise a supported-language
repo). Record the finding; do not build on an unverified index.

Then implement the tool:
- A code-search tool that answers "where is X" from the registered node set in one call,
  with provenance (file, symbol, block id, confidence).
- Persistent `grep` fallback with its existing bounds (`spawn_blocking`, build/vendor
  exclusions, bounded scans in `crates/opencode-tool/src/grep_tool.rs`).
- Tool surface separates markdown/prose search (grep) from code search (ScopeMux).
- Activation follows the existing provider scope rule (local Qwen only; generic elsewhere).
- Evidence-only: no task-state authority.

Touchpoints:
- `crates/opencode-tool/src/grep_tool.rs` (fallback, bounds to preserve)
- `crates/opencode-tool/src/registry.rs` and `crates/opencode-tool/src/lib.rs` (register)
- new module under `crates/opencode-tool/src/` for the code-search tool
- `crates/opencode-scopemux/src/lib.rs` (node search / search FFI)
- `crates/opencode-types/src/retrieval.rs` (candidate provenance)
- `docs/opencode-tool.md` (tool docs)

Verify:
```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tool -p opencode-scopemux
cargo test -p opencode-scopemux --features native
cargo test -p opencode-tool grep
cargo fmt --all --check
```
Update the card's stale `WI-030` note based on the node-registration finding.

### Step 8 — `SCOPE-009` freshness/staleness SLOs (implementation PR)

Card: `boards/todo/Define map freshness and staleness SLOs.md`.

Work:
- Define `fresh`, `lagging`, `stale`, `unknown` and the runtime response for each
  (use/warn/fall back to generic).
- Measure last parsed revision, content hashes, watcher lag, dirty-set size, and re-anchor
  result.
- Surface freshness in `RetrievalResponse` provenance.
- Mark plan nodes stale/conflict when anchors change; never silently drop them.
- Add edit -> reparse -> re-anchor latency and stale-fallback tests.
- Record the SLO in `wiki/workspace-intelligence.md`.

Touchpoints:
- `crates/opencode-types/src/retrieval.rs` (freshness state on response)
- `crates/opencode-scopemux/src/lib.rs` (compute freshness; watcher/parse state)
- `crates/opencode-session/src/retrieval.rs` (stale -> generic fallback)
- `crates/opencode-watcher` if watcher lag is the source
- `wiki/workspace-intelligence.md`
- `wiki/scopemux-map-integration.md`

Verify:
```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-types -p opencode-session -p opencode-scopemux
cargo test -p opencode-scopemux --features native
cargo fmt --all --check
```

## Verification Standards (every step)

- Paste exact commands and results into the card and the PR body.
- `cargo fmt --all --check` must pass.
- Clippy on touched crates where feasible (`cargo clippy -p <crate> --all-targets`).
- Native suite must pass except for failures reproduced on base `development` and recorded
  as pre-existing.
- Run `scripts/scopemux-qa-check.sh` for any change to provider selection, representation,
  or activation, and record the `provider=`/`representation=` signal.
- Never mark a card `done` without a QA section on that card.

## PR Description Template

```markdown
## Summary
<what and why, which board item>

## Acceptance Criteria
- [x] <AC text from the board card>

## Code Touchpoints
- <file:line or file list>

## Verification
- <exact commands run + results>

## Known Limitations
- ...

## Board Card
<SCOPE-XXX>; predecessor/successor steps in H-018
```

## Human Liaison Checklist

- Each implementation PR targets `development`, not `main`.
- The generic provider still completes a task with ScopeMux absent/unsupported.
- No task-state authority moved into the map.
- Exact LSP facts remain opt-in only.
- The Qwen-only activation gate is unchanged.
- No `FUTURE-PROOFING-*`/`DECISION-*` work was touched.
- Card lane moves match the documented sequence; no card promoted without QA evidence.
- Branches are deleted after merge; the board stays the source of truth.

## Out of Scope / Future Work

- `SCOPE-007` subagent-scoped intelligence and per-subagent budgets (`FUTURE-PROOFING-015`).
- `SCOPE-008` provenance/outcomes into the data flywheel (`FUTURE-PROOFING-009`).
- `SCOPE-011` out-of-process transport (`FUTURE-PROOFING-014`/`016`).
- The entire Workspace Intelligence Protocol program (`FUTURE-PROOFING-013`) until the
  owner resumes it.

## Session Kickoff Instructions

Open a new session in `~/apps/opencode-modded-rust` and send the prompt in
`next_steps/next-steps.md` under "Session Kickoff Prompt", or use this condensed form:

> Execute handoff `H-018` (`handoffs/H-018-scopemux-remaining-program-handoff.md`) with the
> `handoff-to-pr` skill. Implement the 8 steps in the documented order, one at a time.
> Future-proofing is on hold: do not touch `FUTURE-PROOFING-*`, `DECISION-*`, `SCOPE-007`,
> `SCOPE-008`, or `SCOPE-011`. Use a dedicated worktree for implementation PRs, commit
> board/handoff updates directly on `development`, and stop after each PR for the human
> liaison's explicit merge approval. Never push `main`.
