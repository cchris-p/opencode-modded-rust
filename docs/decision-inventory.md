# Decision Inventory And Baseline

<!-- markdownlint-disable MD013 -->

This is the `DECISION-002` inventory: every decision point the harness actually makes
today, its implementation, its baseline status, and its proposed System-1 class. It
implements the inventory requirement in `wiki/decision-layer.md` and is normative only
as a measurement record; the rules live in `invariants/decision-layer.md`.

**No behavior changes here.** Every row states what the code does now. Where a baseline
is missing the row says `unmeasured` and names the measurement that would fill it.

## How to read a row

- **Implementation** — `deterministic` (rules/branching), `generative` (a model
  produces the choice as prose), or `absent` (no decision point exists; nothing is
  classified).
- **Baseline** — a measured number, or `unmeasured` plus the plan.
- **Sensitivity** — the tier the class's input state carries, using the tiers defined
  in `wiki/decision-log-egress.md` (`public`, `internal`, `sensitive`, `secret`).
- **Proposed authority** — `advisory`, `gated`, `delegated`, or `forbidden`, per
  `wiki/decision-layer.md`. Nothing starts above `advisory` before evaluation.
- **EV rank** — expected value if System 1 took the class over: 1 is highest.

## Inventory

| # | Decision class | Code location | Implementation | Baseline | Sensitivity | Proposed authority | EV rank |
|---|---|---|---|---|---|---|---|
| 1 | Permission allow / ask / deny | `crates/opencode-permission/src/ruleset.rs:170` (`evaluate`), `:191` (`disabled`); `crates/opencode-permission/src/engine.rs:93` (`is_approved`), `:155` (`respond`) | deterministic | unmeasured; plan: replay 30 days of permission prompts from the session DB and count prompt/auto/deny per pattern | `internal` | `advisory` (risk hint only; rules and the human stay authoritative) | 5 |
| 2 | Command-risk pre-screen (arity/panic guard) | `crates/opencode-permission/src/arity.rs` | deterministic | unmeasured; plan: count arity-triggered prompts and panic-adjacent rejections | `internal` | `advisory` | 6 |
| 3 | Agent selection | `crates/opencode-server/src/agentic.rs:45` (`resolve_agentic_context`), `crates/opencode-agent/src/agent.rs:192` (`build`) | deterministic | unmeasured; plan: count agent switches and explicit `@agent` mentions per 100 sessions | `internal` | `delegated` (safe, reversible choice among registered agents) | 7 |
| 4 | Tool availability / filtering | `crates/opencode-server/src/agentic.rs:77` (`resolve_tools`), `crates/opencode-agent/src/agent.rs:111` (`PermissionNext::evaluate`) | deterministic | unmeasured; plan: record tool set size and permission-filtered tools per agent | `internal` | `forbidden` (tool exposure is a permission surface) | — |
| 5 | Model / provider selection | `crates/opencode-server/src/routes.rs:3464` (resolved model fallback), agent/config `model` fields | deterministic | unmeasured; plan: histogram of model per turn and per-session cost from the DB | `internal` | `delegated` (cheap/strong within a declared budget) | 4 |
| 6 | Title generation | `crates/opencode-session/src/prompt.rs:2802` (`ensure_title`), `:3975` (`generate_session_title_llm`) | generative | n/a (generation, not a decision) | `internal` | `forbidden` (stay with the model) | — |
| 7 | Session summary | `crates/opencode-session/src/compaction.rs:237` (`generate_summary_prompt`) | generative | n/a | `internal` | `forbidden` (what to say stays generative) | — |
| 8 | Compaction trigger | `crates/opencode-session/src/prompt.rs:2767` / `:3553` (`should_compact`), `compaction.rs:203` (`is_overflow`) | deterministic | unmeasured; plan: log overflow-vs-proactive triggers and tokens saved per compaction | `internal` | `delegated` (Wave 1: when to compact, which spans survive) | 2 |
| 9 | Tool-result pruning | `crates/opencode-session/src/compaction.rs:267` (`should_prune_tool_result`), `:284` (`prune`) | deterministic | unmeasured; plan: count pruned results and their later re-read rate | `internal` | `delegated` | 8 |
| 10 | Tool-call normalization / repair | `crates/opencode-provider/src/stream.rs:378` (`DeepSeekNativeToolCallExtractor`), `:415` (`finish`), `:876` (`finish_native_stream`) | deterministic | unmeasured; plan: count discarded/leaked native blocks per provider per 1k turns (BUG-074 documents a lost write) | `internal` | `gated` (classify, then retry or surface; never silently drop) | 3 |
| 11 | Loop continuation / turn termination | `crates/opencode-session/src/prompt.rs:1824` (`finish_reason != "tool-calls"`), `:1333` (`max_steps`), `crates/opencode-provider/src/stream.rs:145`/`:201` (idle/budget bounds) | deterministic | unmeasured; plan: classify terminal reasons per session (stop, max_steps, timeout, error) | `internal` | `advisory` (continue/stop/escalate hint; bounds stay authoritative) | 9 |
| 12 | Retrieval sufficiency | `crates/opencode-session/src/retrieval.rs:35` (`build_request`), `:89` (`min_confidence`), `:100` (`apply_inclusion_threshold`); `crates/opencode-session/src/prompt.rs:354` (budget) | deterministic | unmeasured; plan: record candidate count, kept count, and threshold rejects per prompt | `internal` | `delegated` (sufficient/insufficient within the budget) | 1 |
| 13 | Workspace-intelligence routing | `crates/opencode-server/src/routes.rs` (`workspace_intelligence_policy`), `crates/opencode-session/src/prompt.rs:354` (`active_profile`) | deterministic | unmeasured; plan: log profile selected, rung, and budget per prompt | `internal` | `advisory` (may only narrow the capability profile) | 10 |
| 14 | Failure classification (transport vs model vs harness) | `crates/opencode-provider/src/retry.rs` (`IsRetryable`, `with_retry`), `crates/opencode-server/src/error.rs:9` (`ApiError`) | absent as a class (retryability only) | unmeasured and unimplemented; plan: Wave 1 gate `DECISION-007` hand-labels 300 failures | `internal` | `advisory` first, `gated` after evaluation | 1 (Wave 1 gate) |
| 15 | Review severity / triage | none in-product (a `reviewer` agent is config-defined only) | absent | n/a; plan: Wave 1 gate `DECISION-008` builds a lab set from review comments | `sensitive` (private diffs) | `advisory` only; humans decide | 2 (Wave 1 gate) |
| 16 | Board / task classification | none in-product (boards workflow is external, `bd`/`board.toml`) | absent | n/a; plan: Wave 2 gate classifies 200 historical cards | `internal` | `advisory` only; never mutates disposition | 11 |
| 17 | Memory write / recall policy | session persistence: `crates/opencode-server/src/routes.rs:265` (`persist_sessions_if_enabled`), `crates/opencode-storage` | deterministic (persist all) | unmeasured; plan: count sessions persisted vs recalled, and recall hit rate | `sensitive` (transcripts) | `forbidden` for write authority; `advisory` for recall hints | 12 |
| 18 | Safety screening (jailbreak / injection / secrets) | partial: permission `ruleset.rs`/`arity.rs`; no content screening | absent as a class | unmeasured; plan: build a labelled injection/jailbreak set before any enforcement | `secret` when secrets may appear | `advisory` only; local engine preferred | 13 |

## Wave 1 classes (fully specified)

Wave 1 is defined in `wiki/decision-layer.md`: low risk, stable boundaries, survives the
V1 task-stage rewrite.

### Failure classification (`DECISION-007`)

- **Question**: is this failure a transport error, a model error, a harness error, or an
  environment error?
- **Current implementation**: retryability only (`retry.rs`), plus `ApiError` variants.
  There is no user-visible classification.
- **State in**: provider error string, HTTP status, retry attempt, stream phase,
  terminal reason.
- **Options**: `transport`, `model`, `harness`, `environment`.
- **Sensitivity**: `internal`.
- **Authority now**: advisory. **After evaluation**: gated (may choose retry vs surface).
- **Baseline plan**: hand-label 300 historical failures from session DB + server log;
  record the current "retry everything retryable" false-retry rate.
- **Why it ranks first**: it gates every other recovery decision and is cheap to label.

### Review triage (`DECISION-008`)

- **Question**: is this review comment blocking, non-blocking, noise, or approve?
- **Current implementation**: absent. Review is a user/config-defined agent producing
  prose.
- **State in**: diff hunk, comment text, file path, prior reviewer verdicts.
- **Options**: `blocking`, `non_blocking`, `noise`, `approve`.
- **Sensitivity**: `sensitive` (private diffs) — local engine first, hosted only after
  minimization.
- **Authority now**: advisory. Stays advisory; humans own dispositions.
- **Baseline plan**: build a labelled review-comment set with an owner review pass.

### Compaction trigger and retention (`DECISION-009`)

- **Question**: compact now, and which spans must survive?
- **Current implementation**: deterministic (`should_compact`, `is_overflow`, prune).
- **State in**: token usage vs model limits, message ages, tool-result sizes, active
  task stage.
- **Options**: `compact_now`, `defer`, plus a must-survive span set.
- **Sensitivity**: `internal`.
- **Authority now**: delegated (deterministic). A classifier may propose the trigger but
  the runtime keeps the budget and the retention floor.
- **Baseline plan**: compare overflow-triggered vs proactive compaction and the
  tokens-saved/re-read tradeoff over a week of sessions.

### Retrieval sufficiency (`DECISION-009`)

- **Question**: is the current context enough, or which source to read next?
- **Current implementation**: deterministic thresholds (`min_confidence`,
  `apply_inclusion_threshold`, budget).
- **State in**: candidate files, anchors, confidence, token estimate, representation.
- **Options**: `sufficient`, `insufficient`, plus next-source candidates.
- **Sensitivity**: `internal`.
- **Authority now**: delegated within the token budget; may not widen the profile.
- **Baseline plan**: log kept/rejected candidates and measure redundant-read rate; pair
  with `FUTURE-PROOFING-017` (ScopeMux vs generic retrieval).

## Not a decision (explicitly excluded)

- Title generation, session summary prose, and final answers are **generation**; the
  decision layer must not take them.
- Anything with `forbidden` authority (permissions, tool exposure, dispositions, merges,
  invariants, secret access) is listed only to record that it exists and is out of scope.

## Measurement plan (fills the `unmeasured` rows)

1. Add a bounded, local-only decision sample to the session DB or server log using the
   existing trace path; no new egress.
2. Replay one week of sessions per class; record counts, p50/p95 latency, and the
   agreement between the deterministic path and a shadow classifier.
3. Feed the results into the per-class evaluation harness (`DECISION-006`) before any
   class moves above `advisory`.

## Code-accuracy notes

- Line numbers are pinned to `development` at the inventory date; re-grep before citing
  them in another document.
- `absolute` paths, transcripts, and secrets must never enter a hosted decision request
  (`wiki/decision-log-egress.md`).
