---
id: "H-020"
title: "SCOPE-015/016/017/018/019 Persistent Workspace Map Program — Program Handoff"
status: "open"
created: "2026-11-06"
updated: "2026-11-06"
owner: ""
target: "opencode-modded-rust"
blocked_reason: ""
needs_human: ""
items: ["SCOPE-015", "SCOPE-018", "SCOPE-016", "SCOPE-017", "SCOPE-019"]
---

# Persistent Workspace Map Program — Program Handoff

## Current State

`H-018` is paused before `SCOPE-006` because the prerequisite was verified missing
(2026-11-06):

- `ScopemuxProvider::retrieve` creates a fresh `ProjectContext` per call, adds only
  `seed_files`/`changed_files`, and drops it. There is no persistent whole-workspace node
  registry and no watcher.
- `SCOPE-006` cannot meet its "without a filesystem walk" criterion on that provider.
- `SCOPE-009` cannot measure real freshness (no last revision, dirty set, watcher lag, or
  re-anchor result).
- Evidence and the corrected `WI-030` note are in PR #162 (merged `dac31ee`).

**Decision A is locked:** build the persistent map rather than redefine or retire
`SCOPE-006`. The architecture below is the implementation contract.

## What This Delivers

A persistent, per-workspace ScopeMux map that:

- opens one context per canonical workspace root and reuses it across retrieval/tool calls;
- consumes `opencode-watcher` events and applies incremental engine updates;
- measures freshness without deciding fallback policy;
- exposes query slices to the existing retrieval provider and a code-search capability to
  the tool surface;
- keeps the generic provider as the default/fallback and task authority unchanged.

This unblocks `SCOPE-006` (map-backed code search + grep fallback) and `SCOPE-009`
(freshness/staleness SLOs).

## Locked Architecture

| Concern | Owner | Must not own |
|---|---|---|
| Raw FFI, safe context wrapper, incremental parse/rebuild | `opencode-scopemux` engine layer | provider selection, prompt policy, tool formatting |
| Per-workspace map lifecycle, eviction, watcher subscription, dirty-set scheduling | one map supervisor/registry | task state, inclusion thresholds, model activation |
| File events, ignore matching, debounce | `opencode-watcher` | parsing, freshness policy, retrieval decisions |
| Query slices (search/delta/anchors/impact/review) | provider adapter over the supervisor | creating contexts per call, watching files |
| Freshness measurement (last revision, lag, dirty size, re-anchor result) | engine/supervisor -> shared value type | deciding "stale means fall back" |
| Freshness policy (use/warn/generic fallback) | `opencode-session` / tool facade | engine or watcher |
| Model/provider activation | one shared predicate; runtime injects capability | duplicated in tool and provider |
| Prompt budget, inclusion threshold, role filtering | `opencode-session` | map engine |
| Tool surface and output formatting | `opencode-tool` | FFI, model string parsing, ignore policy |
| Evidence/provenance types | `opencode-types` | per-consumer duplicate result structs |

Dependency direction:

```
opencode-types          (shared evidence, freshness, fallback enums)
      ^
opencode-scopemux       (ffi -> engine -> supervisor -> provider adapter)
      ^                         ^
opencode-session        (activation, inclusion, prompt policy, wiring)
      ^
opencode-tool           (tool surface; backend injected via ToolContext)

opencode-watcher        (generic file events only)  -> supervisor subscribes
```

### DRY rules

DRY applies to decisions and vocabulary, not to forcing grep and map search to share an
algorithm.

- One ignore/exclusion policy consumed by watcher, grep, and indexing.
- One language/prose predicate shared with the `SCOPE-010` matrix.
- One activation predicate; the runtime injects the capability into the tool.
- One evidence type carrying file, symbol, block id, confidence, provenance.
- One freshness vocabulary and one fallback-reason enum.
- One path-normalization helper and one token budget/truncation utility.

### Anti-patterns

- `ScopemuxProvider` owning lifecycle, watcher, parsing, freshness, and prompt policy.
- A global map singleton that also watches files and decides fallback.
- The tool calling FFI or parsing model strings to re-derive activation.
- A tool-only result type that duplicates `RetrievalCandidate`/provenance.
- Provider, tool, and session each defining stale thresholds or ignore lists.
- Holding a `*mut ProjectContext` across an `await` or across workers.

## Program Order

| # | Board ID | Work | Branch |
|---|---|---|---|
| 1 | `SCOPE-018` | Shared contracts and policy (block id, freshness, ignore/language, activation, capability) | `feature/SCOPE-018-shared-map-contracts` |
| 2 | `SCOPE-016` | Safe engine layer and persistent context lifecycle | `feature/SCOPE-016-safe-engine` |
| 3 | `SCOPE-017` | Map supervisor plus watcher-driven incremental updates | `feature/SCOPE-017-map-supervisor` |
| 4 | `SCOPE-019` | Wire into retrieval provider and tool search capability | `feature/SCOPE-019-map-integration` |
| 5 | `SCOPE-006` | Resume map-backed code-search tool (via `H-018`) | `feature/SCOPE-006-scopemux-code-search` |
| 6 | `SCOPE-009` | Resume freshness/staleness SLOs (via `H-018`) | `feature/SCOPE-009-map-freshness-slo` |

Dependency order is strict: `SCOPE-018` -> `SCOPE-016` -> `SCOPE-017` -> `SCOPE-019` ->
`SCOPE-006` -> `SCOPE-009`.

## Operating Model — One PR at a Time, Human-Gated

Repeat for each step:

1. Confirm the previous PR is merged into `development` and the code repo is current.
2. Fetch `origin/development` and create the step branch from it in a dedicated worktree
   under `$HOME/worktrees/opencode-modded-rust/persistent-map/`.
3. Commit board/handoff updates directly on `development` in the main checkout.
4. Move the card to `doing`; implement and run the per-step verification.
5. Open a ready-to-review PR against `development`; put exact commands/results in the card
   and PR body.
6. **STOP for explicit human merge approval.** Never merge on tests alone and never start
   step N+1 before step N merges.
7. After merge: move the card to `qa`, run post-merge QA on `development`, then promote to
   `done` only with recorded evidence; delete the merged branch.

## Environment

- Main checkout for board/QA: `$HOME/apps/opencode-modded-rust` on `development`.
- Implementation worktree root: `$HOME/worktrees/opencode-modded-rust/persistent-map/`.
- Run `scripts/setup-worktree-build-cache.sh` once so worktree builds share the target
  cache.
- Native bundle: `export SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)"`.
- **Known trap:** the shared worktree target can serve a stale `opencode-scopemux`
  artifact after switching worktrees/branches. Run `cargo clean -p opencode-scopemux`
  before native tests when behavior looks like an older revision.
- Use `ort-build` before a manual TUI run; `ort` serves the newest built binary.

## Per-Step Runbooks

### Step 1 — `SCOPE-018` shared contracts and policy

Card: `boards/todo/Centralize shared workspace-map contracts and policy.md`.

Work:

- Add `block_id`/missing provenance to shared evidence; avoid a tool-specific result type.
- Add `MapFreshness` (`fresh`, `lagging`, `stale`, `unknown`) and measurement fields.
- Centralize the ignore/exclusion policy shared by grep, watcher, and indexing.
- Centralize the code/prose predicate with the `SCOPE-010` matrix.
- Centralize activation and expose the `WorkspaceSearch` capability for injection.
- Add shared path-normalization and token-budget helpers.
- Update `wiki/workspace-intelligence.md` and `wiki/scopemux-integration-plan.md`.

Verify:

```bash
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-types -p opencode-retrieval -p opencode-tool -p opencode-watcher
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-scopemux -p opencode-session
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-types -p opencode-retrieval -p opencode-tool -p opencode-watcher --all-targets
```

Tests: ignore-policy agreement, code/prose classification, freshness/fallback serialization,
capability injection.

### Step 2 — `SCOPE-016` safe engine layer

Card: `boards/todo/Extract a safe ScopeMux engine layer with persistent context lifecycle.md`.

Work:

- Split `opencode-scopemux` into `ffi`, `engine`, and `provider`.
- Engine owns one `ProjectContext`: open, add/update/remove, incremental rebuild, query.
- No raw pointer outside `ffi`/`engine`; one owner worker; no pointer across `await`.
- Use `project_update_file`, `project_context_remove_file`, incremental rebuild counters.
- Prove: create once -> edit -> update -> query sees the new symbol; incremental counters
  show a smaller dirty set than a full rebuild.
- Keep provider output unchanged.

Verify:

```bash
cargo clean -p opencode-scopemux
SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
  cargo test -p opencode-scopemux --features native
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-scopemux
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux --all-targets
```

### Step 3 — `SCOPE-017` map supervisor and watcher

Card: `boards/todo/Add a workspace map supervisor with watcher-driven incremental updates.md`.

Work:

- Registry/supervisor: one map per canonical root, acquire/release, eviction/TTL, shutdown.
- Subscribe to `opencode-watcher`; debounce/coalesce add/change/unlink; shared ignore policy.
- Route updates through the engine; never create a context per query.
- Record freshness counters: last revision, dirty-set size, watcher lag, rebuild duration,
  re-anchor result. Do not decide fallback.
- Ensure a failed update keeps the previous map and marks it stale/degraded.
- No task-state writes.

Verify:

```bash
cargo clean -p opencode-scopemux
SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
  cargo test -p opencode-scopemux --features native
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-watcher -p opencode-scopemux
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux -p opencode-watcher --all-targets
```

Tests: registry reuse, edit -> incremental update, unlink -> removal, ignore-policy
agreement, eviction/shutdown, no cross-workspace leakage.

### Step 4 — `SCOPE-019` provider and tool integration

Card: `boards/todo/Wire the persistent map into retrieval and tool search.md`.

Work:

- `ScopemuxProvider` uses the supervisor; no per-call context creation.
- Keep generic fallback/provenance for absent/stale/unavailable maps.
- Inject `WorkspaceSearch` into `ToolContext`; add the map-backed code-search tool with
  file/symbol/block id/confidence/provenance/freshness.
- Keep bounded grep fallback and the code/prose mode split.
- Activation follows the existing Qwen-only rule; the tool does not parse model strings.
- Evidence-only: no task authority.

Verify:

```bash
cargo clean -p opencode-scopemux
SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -1)" \
  cargo test -p opencode-scopemux --features native
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tool -p opencode-session -p opencode-retrieval
cargo test -p opencode-tool grep
cargo fmt --all --check
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy -p opencode-scopemux -p opencode-tool -p opencode-session --all-targets
scripts/scopemux-qa-check.sh --file <supported-file> --expect scopemux
```

### Step 5 — resume `H-018` at `SCOPE-006`

- `SCOPE-006` re-verifies node registration against the persistent map, then implements the
  map-backed code-search tool, markdown/prose split, and grep fallback.
- Follow the `SCOPE-006` card and `H-018` step 7/8 wording; do not re-open the architecture
  decision.
- Then `SCOPE-009`: define freshness states/SLOs, surface `MapFreshness` in
  `RetrievalResponse`, stale -> generic fallback, and edit -> reparse -> re-anchor tests.

## Verification Standards

- Paste exact commands and results into the card and PR body.
- `cargo fmt --all --check` must pass.
- Clippy on touched crates where feasible.
- Native suite must pass except failures reproduced on base `development` and recorded as
  pre-existing.
- Run `scripts/scopemux-qa-check.sh` for activation/representation changes.
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
<SCOPE-XXX>; step N of H-020
```

## Human Liaison Checklist

- Each PR targets `development`, never `main`.
- The generic provider still completes a task with the map absent/stale/unsupported.
- No task-state authority moved into the map.
- Exact LSP facts remain opt-in only.
- The Qwen-only activation gate is unchanged.
- One ignore policy, one language predicate, one activation predicate, one evidence shape.
- No `FUTURE-PROOFING-*`/`DECISION-*` work was touched.
- Branches are deleted after merge; the board stays the source of truth.

## Boundaries

Out of scope:

- All `FUTURE-PROOFING-*` and `DECISION-*` work.
- `SCOPE-007` subagent-scoped intelligence, `SCOPE-008` data flywheel, `SCOPE-011`
  out-of-process transport.
- Capability profiles, artifact packaging, System-1 routing.
- New language grammars or a new index engine.
- Multi-workspace distributed maps or cross-process caches.
- Task stage/verification/review/completion authority in the map.

## Risks and Decisions

- **FFI lifetime/threading** is the primary risk. Prove single-owner engine access in
  `SCOPE-016` before adding the supervisor.
- **Watcher storms** are handled with debounce/coalescing and a bounded dirty set.
- **Stale correctness:** a failed update must surface as stale/degraded, never silently
  serve old nodes as fresh.
- **Over-DRY:** share policy and vocabulary, not grep and map algorithms.
- **Scope creep:** if `scopemux-core` lacks an API, record it as a prerequisite; do not
  build a parallel index.
- **Decision A is locked**; do not re-litigate SCOPE-006/009 scope in this program.

## Resume Instructions

Open a new session in `~/apps/opencode-modded-rust` and send:

> Execute handoff `H-020`
> (`handoffs/H-020-persistent-workspace-map-program-handoff.md`) with the `handoff-to-pr`
> skill. Implement the steps in the documented order, one at a time. Future-proofing is on
> hold: do not touch `FUTURE-PROOFING-*`, `DECISION-*`, `SCOPE-007`, `SCOPE-008`, or
> `SCOPE-011`. Use a dedicated worktree for implementation PRs, commit board/handoff
> updates directly on `development`, and stop after each PR for explicit merge approval.
> Never push `main`. After `SCOPE-019`, resume `H-018` at `SCOPE-006` and then `SCOPE-009`.
