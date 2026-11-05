---
id: "H-017"
title: "DeepSeek API efficiency improvements for ort - Handoff"
status: "complete"
created: "2026-11-05"
updated: "2026-11-05"
owner: ""
target: "opencode-modded-rust"
blocked_reason: ""
needs_human: ""
items: ["RESEARCH-008"]
---

# DeepSeek API Efficiency Improvements For `ort` - Handoff

## Included Board Items

| Item | Title | Role |
|---|---|---|
| `RESEARCH-008` | Evaluate integrating DeepSeek Harness (dsh) into `ort` | Owning decision card; implementation closes it after QA |

This handoff is the implementation arm of `RESEARCH-008`. The research verdict is
recorded in
`docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md` §6: **do not integrate
dsh**; borrow its DeepSeek-API efficiency capabilities into `ort` instead.

## Decision Locked

- dsh is **not** adopted as a runtime, plugin substrate, Research Department
  dependency, or vendored code. Its Cordis profiles/bundles/HMR, `dsh_session_log`
  upload, plugin inventory, account auth, and Files-API image plumbing are rejected.
- `ort` remains a single Rust core. No second plugin runtime or JS harness is added.
- The adopted scope is DeepSeek-specific provider behavior only.
- The card's original A/B-run requirement is superseded by explicit operator
  direction (2026-11-05): implementing these capabilities is the card's deliverable.
  The source-level comparison and the null/negative verdict on dsh integration are
  the recorded research result.
- Model metadata must come from the live official listing (or models.dev), never a
  stale hardcoded table.
- Reasoning control must be capability-driven and degrade to today's behavior when
  a model or endpoint does not advertise effort support.
- The Anthropic-Messages transport is evidence-gated: implement it behind config,
  measure it, and default it only if it wins. A null result is valid and keeps
  `/chat/completions`.

## Evidence At Creation (live, 2026-11-05)

`GET https://api.deepseek.com/models` (with `DEEPSEEK_API_KEY`) returned:

| Model | context | max output | effort | `anthropic_messages` capabilities |
|---|---|---|---|---|
| `deepseek-flash` | 1,048,576 | 393,216 | `low`/`high`/`max` (default `high`) | `system_prompt_update: in-history`, `tool_update: addition-only` |
| `deepseek-v4-pro` | 1,048,576 | 393,216 | `low`/`high`/`max` (default `high`) | `system_prompt_update: leading-only`, `tool_update: leading-only` |

`deepseek-flash` also advertises text+image input. models.dev agrees on
1,048,576 / 393,216 and marks both as reasoning + tool-capable.

Minimal probes (both HTTP 200):

- `POST /anthropic/v1/messages` with `output_config.effort: "low"` returned a
  `thinking` block with a signature and cache-usage fields.
- `POST /chat/completions` with `reasoning_effort: "low"` was accepted and returned
  `reasoning_content` plus `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens`.

## Current State At Creation

- The primary catalog path is models.dev (`load_models_dev_cache` in
  `crates/opencode-provider/src/bootstrap.rs`), which is correct at 1M/393k.
- The **bundled fallback** catalog (`bundled_v1_models_data`, same file) declares
  `deepseek-flash` at 128k context / 32k output. The dedicated
  `DeepSeekProvider` (`crates/opencode-provider/src/deepseek.rs`) declares
  `deepseek-chat` / `deepseek-reasoner` at 64k / 8k. Both are stale.
- `crates/opencode-provider/src/transform.rs`:
  - `variants()` returns empty for any id containing `deepseek`, so no effort
    variants exist.
  - `OUTPUT_TOKEN_MAX` is `32_000`, below the model's 393,216 output cap.
  - `apply_caching()` only applies cache markers for Anthropic/OpenRouter/Bedrock/
    Gateway; DeepSeek relies on its automatic prefix cache.
- `crates/opencode-provider/src/openai_chat.rs` already echoes
  `reasoning_content` back on follow-up requests (BUG-006) and normalizes leaked
  native tool tokens (BUG-060/061).
- `crates/opencode-session/src/llm.rs` rebuilds the system prompt as leading
  system messages on every request (`build_system_prompt`, `build_messages`), so a
  dynamic-context change rewrites the cached prefix.
- Default model is `deepseek/deepseek-flash`
  (`crates/opencode-config/src/loader.rs`).

## Dependency Order

Passes are sequential except where noted:

1. **Pass 1 — Metadata alignment.** No dependency; fixes the biggest correctness
   gap and unblocks realistic cache/context testing.
2. **Pass 2 — Reasoning effort.** Depends on Pass 1 metadata (`reasoning` flag and
   effort levels) to generate variants correctly.
3. **Pass 3 — Cache-safe prefix.** Independent of Pass 2; can run in parallel if
   file ownership is disjoint. It touches session/request assembly, not the
   provider variant table.
4. **Pass 4 — Anthropic transport evaluation.** Depends on Pass 1–3; it reuses the
   metadata and cache instrumentation.
5. **Pass 5 — Verification, docs, card closeout.** Depends on all passes.

## PR Plan

Use one PR per pass, targeting `development`:

| Pass | Scope | Base gate |
|---|---|---|
| 1 | Live/bundled metadata alignment, output cap, listing refresh | Clean baseline |
| 2 | DeepSeek reasoning effort/thinking variants + config/TUI surfacing | Pass 1 merged |
| 3 | Cache-safe system/tool prefix behavior + tests | Clean baseline (parallel-safe) |
| 4 | Anthropic Messages transport behind config + measurement | Passes 1–3 merged |
| 5 | Docs/invariants, full verification, move `RESEARCH-008` to `qa` | Passes 1–4 merged |

Do not merge `development` into `main`. Do not open a release PR as part of this
handoff.

## Implementation Steps

### Pass 1 — Metadata alignment

Files:
- `crates/opencode-provider/src/bootstrap.rs` (`bundled_v1_models_data`,
  `load_models_dev_cache`, `from_models_dev_model`).
- `crates/opencode-provider/src/deepseek.rs` (dedicated provider struct).
- `crates/opencode-provider/src/transform.rs` (`OUTPUT_TOKEN_MAX`, output-cap
  handling).

Work:
- Replace the stale `deepseek-flash` fallback entry with 1,048,576 context /
  393,216 output, reasoning + tool + image capability, `release_date`, and the
  current cost basis if known.
- Replace or remove the stale `deepseek-chat` / `deepseek-reasoner` models in
  `DeepSeekProvider` so there is one source of truth. Prefer deleting the duplicate
  hardcoded list and consuming the registry/catalog.
- Ensure the models.dev path wins over any bundled fallback and that a corrupt/missing
  cache reliably falls back without reverting to 128k.
- Reconsider `OUTPUT_TOKEN_MAX`: keep it as a safety cap if required, but do not let
  it silently cap DeepSeek below the model's advertised output. Prefer a
  per-model limit derived from catalog metadata, with the 32k value only as a
  fallback when metadata is absent.
- Add a test that asserts `deepseek-flash` resolves to 1,048,576 / 393,216 from both
  the models.dev fixture and the bundled fallback.

**Gate:** in a build with the models.dev cache absent, `deepseek/deepseek-flash`
reports 1,048,576 context and 393,216 output; `deepseek-v4-pro` matches; no stale
64k/8k DeepSeek models are selectable.

### Pass 2 — Reasoning effort / thinking control

Files:
- `crates/opencode-provider/src/transform.rs` (`variants`, effort sets).
- `crates/opencode-provider/src/bootstrap.rs` (`from_models_dev_model` variant
  merge).
- `crates/opencode-provider/src/openai_chat.rs` (wire field mapping).
- `crates/opencode-provider/src/anthropic.rs` (only if the transport already
  exists for other providers; do not add DeepSeek transport here yet).
- `crates/opencode-tui/src/components/dialogs/model_select.rs` and the config
  schema/loader if variants must be selectable/serialized.

Work:
- Stop excluding `deepseek` from `variants()`; derive allowed efforts from the
  live/model metadata (`low`, `high`, `max`, default `high`) and keep unsupported
  values rejected before network I/O.
- Map the selection to the **OpenAI-compatible** wire field first
  (`reasoning_effort`), because that is the current transport and the live probe
  accepted it. Prove whether it changes behavior with a bounded A/B; if it is
  ignored, do not ship a dead control.
- Preserve the existing `reasoning_content` passback path (BUG-006) and the native
  tool-call guards.
- Keep the default at the model's advertised default (`high`) and keep an explicit
  "off"/no-effort path only if the endpoint accepts it.
- Surface the variants through the model selector / config exactly as other
  providers do; do not add a DeepSeek-specific UI path.

**Gate:** a user can select `low`, `high`, or `max` for `deepseek-flash`; the
request body carries the expected field; an unsupported value fails before the
network; the default remains `high`; a bounded A/B shows a measurable difference
(latency, output tokens, or reasoning tokens) or the control is recorded as
behaviorally inert and removed.

### Pass 3 — Cache-safe prompt/tool prefix

Files:
- `crates/opencode-session/src/llm.rs` (`build_system_prompt`, `build_messages`).
- `crates/opencode-provider/src/transform.rs` / `openai_chat.rs` if cache
  instrumentation or message-shape changes are needed.
- Session/system tests under `crates/opencode-session/`.

Work:
- Audit which system-context sources change per request (date, environment,
  instruction files, MCP instructions, skills, dynamic reminders).
- Keep the leading system/tool prefix byte-stable when only dynamic context
  changed. Prefer appending a new system message in history over rewriting the
  leading system message, if the endpoint accepts it.
- Do not move canonical context authority or change model-visible semantics; the
  model must see the same effective context.
- Add a test that two consecutive turns with an unchanged prefix produce identical
  leading bytes, and that a dynamic-context change does not rewrite the prefix.
- Instrument cache hit/miss counters from the provider usage (both endpoints
  already return them) so Pass 4 can measure.

**Gate:** repeated turns with unchanged context preserve the prefix; a dynamic
context change does not invalidate the whole prefix; model-visible context remains
equivalent; cache counters are observable.

### Pass 4 — Anthropic Messages transport (evidence-gated)

Files:
- `crates/opencode-provider/src/anthropic.rs` and provider-type/mapping code.
- `crates/opencode-provider/src/bootstrap.rs` (DeepSeek provider definition).
- `crates/opencode-config/src/loader.rs` only if a config flag is needed.

Work:
- Implement `provider: deepseek` over `https://api.deepseek.com/anthropic` with
  `x-api-key`, `anthropic-version`, Messages request/response shapes, thinking
  signatures, and tool blocks.
- Preserve the existing `/chat/completions` path behind a config switch so the
  change is reversible.
- Implement the documented `system_prompt_update: in-history` and
  `tool_update: addition-only` behavior only for models that advertise it; use
  `leading-only` for `deepseek-v4-pro`.
- Measure on a fixed workload: cache hit rate, input/output tokens, p50/p95
  latency, and cost. Compare against `/chat/completions` with Pass 1–3 applied.
- Default the new transport only if it wins on the measurement; otherwise keep
  `/chat/completions` and record the null result in the card and this handoff.

**Gate:** the transport works end to end for streaming, tools, reasoning, and
compaction; the old path still works when selected; a measurement table is
recorded; the default is justified or the transport remains opt-in with a null
verdict.

### Pass 5 — Verification and closeout

- Update `docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md` with the
  implementation results and measured numbers.
- Update `wiki/deepseek-native-tool-call-normalization.md` only if behavior
  changes; add a short wiki note or provider doc for the new effort/caching
  behavior.
- Update `invariants/providers.md` if a new binding rule is introduced (for
  example, "DeepSeek metadata must come from the live listing").
- Move `RESEARCH-008` to `qa` with a QA report describing the verification.
- Do not move it to `done` without a recorded QA pass or explicit owner direction.

**Gate:** `cargo fmt`, `cargo check`, `cargo clippy --workspace --all-targets`,
targeted tests, and a manual `ort` run against `deepseek/deepseek-flash` all pass;
the card records evidence and remaining gates.

## Verification

Run from the worktree:

```bash
cargo fmt --all -- --check
cargo check
cargo clippy --workspace --all-targets
cargo test -p opencode-provider
cargo test -p opencode-session
cargo test --workspace
```

Manual:

```bash
ort-build
ort   # default deepseek/deepseek-flash
```

Probes (no secret printed):

```bash
curl -s https://api.deepseek.com/models \
  -H "Authorization: Bearer $DEEPSEEK_API_KEY" | jq .
```

Capture cache/latency/token evidence for Pass 4 in the card, not only in chat.

## Risks And Constraints

- **Developer-preview API drift.** DeepSeek's listing and capabilities can change.
  Record the probe date and keep metadata fetch-based.
- **Dead controls.** Do not ship an effort selector if the endpoint ignores it.
- **Prefix changes can be subtle.** Environment/date/instructions/skill lists all
  contribute. Preserve model-visible semantics; do not just suppress updates.
- **Transport switch risk.** The Anthropic path changes message/tool/reasoning
  shapes. Keep it behind config until measured.
- **Egress.** Do not add dsh-style session-log upload, plugin inventory, or extra
  identity headers. Only the provider's own request bodies and required auth.
- **No second core.** Do not add a JS/Cordis runtime or vendor dsh packages.
- **Git workflow.** All PRs target `development`; never `main`.

## Rollback / Decommission

- Pass 1 is a metadata correction; revert the PR if the listing is wrong.
- Pass 2 effort variants can be disabled by removing the generated variants; the
  default path remains.
- Pass 3 must be behavior-preserving; revert if model-visible context changes.
- Pass 4 is config-gated; switching back to `/chat/completions` restores the
  previous behavior without code removal.

## Exit Criteria

- DeepSeek metadata matches the live official listing in both cached and fallback
  paths.
- Effort selection is real, capability-driven, and degrades cleanly.
- Repeated turns preserve the cacheable prefix without changing model-visible
  context.
- The Anthropic transport is implemented behind config and either measured-better
  (defaulted) or recorded as a null result (kept opt-in).
- `RESEARCH-008` records the verdict, implementation evidence, and remaining
  gates, and is in `qa`.

## Next Agent Instructions

1. Read `docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md` §6 and this
   handoff in full before touching code.
2. Use a separate worktree (see chat instructions); do not work in the main
   checkout.
3. Implement Pass 1 first and keep the PR scoped to metadata only.
4. For each pass, run the verification commands and attach evidence to
   `RESEARCH-008`.
5. Do not adopt dsh code, its plugin runtime, or its egress behavior.
6. Move `RESEARCH-008` to `qa` when Pass 5 lands; do not mark it `done` without a
   QA pass or explicit owner direction.

## Implementation Results (2026-11-05)

All five passes landed on `development`; `RESEARCH-008` is in `qa` for verification.

| Pass | PR | Result |
|---|---|---|
| 1 | #151 | Canonical DeepSeek metadata; stale models removed; output cap respects catalog |
| 2 | #153 | `low`/`high`/`max` effort wired end to end; measured behavior change; invalid values rejected pre-network |
| 3 | #155 | Stable leading system prefix + dynamic system message; cache hit/miss counters observable |
| 4 | #156 | Opt-in Anthropic transport implemented and measured; no win, so `/chat/completions` stays default |
| 5 | this closeout | Research doc, provider invariants, wiki note, card moved to `qa` |

Manual verification used the built product binary (`run --dir /tmp/opencode/ort-pass5`)
with `deepseek/deepseek-flash` on the default chat transport, `--variant max`, and
`DEEPSEEK_TRANSPORT=anthropic --variant low`; all returned the expected reply.
