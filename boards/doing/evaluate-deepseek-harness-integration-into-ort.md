---
id: "RESEARCH-008"
title: "Evaluate integrating DeepSeek Harness (dsh) into ort as a research/vetting harness"
priority: "P2"
type: "research"
area: "RESEARCH"
spec: "invariants/research-department.md"
status: "doing"
created: "2026-10-14"
---

# Evaluate integrating DeepSeek Harness (dsh) into ort as a research/vetting harness

## Summary

Evaluate whether **DeepSeek Harness** (`dsh`, `deepseek-ai/deepseek-harness`)
can serve the **Research Department** in `ort` — as a research/vetting harness
and/or as a source of product capability through its plugin architecture — and
whether integrating it is worth doing.

This is a fresh harness candidate evaluated under
`invariants/research-department.md`, in the same evidence-gated spirit as the
Feynman evaluation (`RESEARCH-001`). It does **not** assume dsh is better than
Feynman, `ort` alone, or product-native tooling; a null/negative result is a
valid and useful outcome.

## Why this exists

- The Research Department's tooling is an **open, evidence-gated decision**, not a
  settled dependency (`invariants/research-department.md`). Feynman was the
  reference harness; a second serious candidate deserves the same measurement.
- `dsh` is a substantially different design from Feynman and from `ort`: an
  **everything-is-a-plugin** architecture built on
  [Cordis](https://github.com/cordiverse/cordis), which may change what is
  reachable for research/vetting or what could be borrowed product-side.
- `RESEARCH-004` commits `ort` to providing the Research-Department **utility
  surface**; which harness(es) that surface instantiates is not yet fixed.
- `RESEARCH-003` (external-evidence value) is still open; dsh may carry a
  remote/external evidence surface that is relevant to it.

## What we know about dsh (recorded 2026-10-14; re-verify before reuse)

- Public repo `git@github.com:deepseek-ai/deepseek-harness.git`, default branch
  `master` (HEAD `d743267388641bc76f17c45ce8b4c231aed1d32c`), MIT license.
- Self-description: "DeepSeek Harness: Everything is a Plugin." TypeScript.
- Run via `npx @deepseek-ai/dsh web` (Web UI at `http://127.0.0.1:3080` by
  default) or from a source checkout with `pnpm install && pnpm run build &&
  pnpm dsh web`.
- Built on Cordis; documented at `https://deepseek-harness.github.io/deepseek-harness/`.
- Marked **developer preview**, iterating rapidly, with explicitly warned
  compatibility-breaking changes — a moving-target risk like Feynman's fork.
- `README`/homepage: `https://github.com/deepseek-ai/deepseek-harness`,
  `https://deepseek.com/harness`.

## Settled model (context, not in question)

- A **department** is defined by concern, not by model/provider/tool; one model
  can operate more than one department (`invariants/research-department.md`).
- External/remote evidence is **advisory, confidence-labeled,
  provenance-bearing, and never canonical**; local artifacts stay authoritative.
- Which tooling serves the department remains an evidence-gated decision; this
  card does not make `dsh` a dependency and does not grant it authority.

## Scope

Answer, with recorded commands and observed outputs, on at least one real
research/vetting task relevant to the live workflow:

- **Harness fit** — can `dsh` perform a comparable research/vetting pass
  (plan/decompose, source gathering, verification, synthesis) with auditable
  provenance?
- **Integration shape** — how would `dsh` be integrated into `ort`? Candidate
  shapes to weigh: run as a separate harness invoked from `ort` (like the
  Feynman reference model), expose its surface through the `RESEARCH-004`
  utility surface, or borrow capability from its plugin/Cordis architecture
  rather than running it. State the recommended shape.
- **Plugin architecture as capability source** — is there anything in the
  everything-is-a-plugin/Cordis design worth borrowing product-side (extension
  points, composition model), independent of running dsh itself?
- **Comparative verdict** — versus `ort` alone (and, where useful, versus the
  recorded Feynman baseline): stronger, equal, or weaker for research/vetting.
- **External-evidence relevance** — does dsh carry a remote/literature/web
  surface that informs the open `RESEARCH-003` question?
- **Cost, latency, complexity, and moving-target risk** — including the
  developer-preview compatibility churn.

Headline criterion: does `dsh` **change, sharpen, or beat** what `ort` (and the
Feynman baseline) already produce for the same task?

## Non-goals

- Making `dsh` (or any harness/model/provider) a runtime dependency of `ort`.
- Vendoring or folding the dsh application into the product binary.
- Granting dsh output (or external evidence) canonical authority, or changing any
  area/gate disposition.
- Building the integration itself — this card evaluates and scopes; the build, if
  any, is a follow-up card.
- Committing private aa-studies content, prompts, or secrets.

## Done when

- At least one real research/vetting task is run through dsh and through `ort`
  alone (Feynman baseline reused where applicable), with exact commands and
  observed outputs recorded; the run is repeatable.
- A clear verdict exists: dsh is **stronger, equal, or weaker** for
  research/vetting, and the value-carrying components (and the noise) are named.
- The **recommended integration shape** is stated (separate harness, utility
  surface, or borrowed capability), with rationale.
- The relationship to `RESEARCH-001` (Feynman), `RESEARCH-003` (external
  evidence), and `RESEARCH-004` (utility surface) is recorded.
- Any follow-up (build/borrow decision, invariant decision, or `RESEARCH-003`
  input) is split into its own card or explicitly deferred.

## Recommended verification

- Read dsh from its own checkout/docs, not from memory; record the exact
  revision and commands used.
- Run both arms end to end on the same task before drawing a conclusion; capture
  raw artifacts, not just summaries.
- Confirm no disposition, invariant, or authority was changed by this card, and
  no private content was committed.

## Decision (2026-11-05)

**Do not integrate dsh.** Recorded verdict and evidence in
[`docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md`](../../docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md) §6.
Instead, borrow targeted DeepSeek-API efficiency capabilities product-side; the
implementation handoff is
[`handoffs/H-017-deepseek-api-efficiency-handoff.md`](../../handoffs/H-017-deepseek-api-efficiency-handoff.md).

Adopted scope: live model metadata, reasoning-effort/thinking control, cache-safe
prompt/tool prefix handling, and an evidence-gated Anthropic-Messages transport.
Rejected: Cordis/profiles/bundles, `dsh_session_log`/plugin-inventory egress,
DeepSeek account auth, Files-API image plumbing, and any vendored dsh code.

The original A/B-run requirement is superseded by explicit operator direction
(2026-11-05): implementing these capabilities is this card's deliverable. This is
an operator-approved scope change, not a silently cleared gate.

## Related Items

- `H-017` DeepSeek API efficiency implementation handoff
- `RESEARCH-001` Feynman research-method vs opencode/ort (local-corpus null result; comparable baseline)
- `RESEARCH-002` Automate the Feynman-to-ort workable-chunk pipeline via the CLI (hold)
- `RESEARCH-003` Evaluate the external R&D department's evidence supplementation (open value gate)
- `RESEARCH-004` Provide the Feynman/Research-Department utility surface in `ort` (integration home)
- `RESEARCH-006` Feynman remote execution and session-analysis surface
- `invariants/research-department.md`, `invariants/integration-scope.md`
- External repo: `deepseek-ai/deepseek-harness`

## Notes

- Research phase started 2026-11-05. Source-level comparison recorded in
  [`docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md`](../../docs/research/RESEARCH-008-deepseek-harness-vs-opencode.md),
  pinned to dsh `fcc0ad4` (package `0.2.2-rc.1`) and vanilla OpenCode
  `f54ce313b` (`1.18.31`).
- Headline source-level findings: dsh makes the model adapter, tool registry,
  session log, and agent loop replaceable from configuration (Cordis
  "everything-is-a-plugin" profiles/bundles/patches/HMR), while OpenCode has a
  fixed Effect service graph with a bounded runtime hook/transform set and
  build-time `LayerNode` replacement; dsh also ships an OS-level filesystem
  sandbox seam, fail-closed one-shot approval, agent teams (shared task DAG),
  workflow scripts, goals/schedules/jobs/deliverables, and browser/computer-use.
  OpenCode remains TUI-first and coding-first and has no OS sandbox.
- Implementation is tracked in `handoffs/H-017-deepseek-api-efficiency-handoff.md`.
  The card moves to `qa` after that handoff's verification pass; it is not
  `done` until a QA pass or explicit owner direction.
- Live probe (2026-11-05): `deepseek-flash` is 1,048,576 context / 393,216 output
  with effort `low`/`high`/`max` and `anthropic_messages` capabilities
  `system_prompt_update: in-history`, `tool_update: addition-only`; `ort`'s
  bundled fallback and dedicated DeepSeek struct are stale (128k/32k and 64k/8k).
- Model is not the variable; this is a harness/tooling question, not a
  model-strength comparison.
- dsh is in developer preview with warned breaking changes — treat the checkout
  as a moving target and pin the revision used for any recorded run.
- A null or negative result (no advantage, or not worth integrating) closes this
  card the same as a positive one.

## Implementation Log (H-017)

### Pass 1 — Metadata alignment

- Single source of truth: added `DeepSeekModelSpec` / `DEEPSEEK_MODELS` and
  `bundled_models()` / `bundled_models_dev()` in
  `crates/opencode-provider/src/deepseek.rs`. Both the direct provider and the
  bundled fallback catalog now consume it; the stale `deepseek-chat` /
  `deepseek-reasoner` 64k/8k entries are deleted.
- Bundled fallback now declares `deepseek-flash` and `deepseek-v4-pro` at
  1,048,576 context / 393,216 output, reasoning + tool capable, with
  `deepseek-flash` also text+image.
- `transform::max_output_tokens` now returns the catalog per-model limit and uses
  `OUTPUT_TOKEN_MAX` (32k) only when metadata is absent, so DeepSeek is never
  silently capped below its advertised output.
- Tests: `deepseek_metadata_matches_live_listing_in_bundled_and_models_dev_paths`
  (bundled + models.dev fixture, runtime mapping),
  `catalog_matches_live_listing`, `models_dev_view_keeps_reasoning_and_image_input`,
  and the updated `max_output_tokens` cases.
- Verification (2026-11-05): `cargo fmt --all -- --check`, `cargo check
  --workspace` (with `SCOPEMUX_SKIP_NATIVE_BUILD=1`), `cargo clippy --workspace
  --all-targets` (no new warnings in the touched files), `cargo test -p
  opencode-provider` (148 tests pass).

### Pass 2 — Reasoning effort control

- `transform::variants` now returns `low`/`high`/`max` for direct DeepSeek
  (`@ai-sdk/openai-compatible`) models and leaves DeepSeek models routed through
  other providers on their provider's policy.
- The session runner previously dropped `PromptInput.variant` on the floor: it now
  threads the variant through `loop_inner` into `ChatRequest.variant`, and
  `resume_session` restores it from session metadata `model_variant`.
- `DeepSeekProvider` maps the selected variant to the OpenAI-compatible
  `reasoning_effort` field in both `chat` and `chat_stream`. Non-advertised values
  (`medium`/`none`/etc.) are rejected with `InvalidRequest` before network I/O.
  No variant leaves the field absent, so the endpoint default `high` applies.
- `/provider` listing (`synthetic_variant_names`) now advertises `low`/`high`/`max`
  for direct DeepSeek models only.

Bounded A/B against `POST https://api.deepseek.com/chat/completions`
(`deepseek-flash`, same fixed prompt, `max_tokens=1200`, 2026-11-05):

| effort | completion tokens | reasoning tokens | latency |
|---|---|---|---|
| default | 55 | 53 | 1093 ms |
| `low` | 51 | 49 | 1020 ms |
| `high` | 64 | 62 | 1301 ms |
| `max` | 69 | 67 | 1291 ms |

The control changes behavior (reasoning tokens and latency both move), so it ships.
The endpoint also accepted `medium`/`none`/`minimal` with HTTP 200, but the live
listing advertises only `low`/`high`/`max`; the client rejects anything else before
sending.
- Tests: `test_variants_deepseek_direct_uses_live_efforts`,
  `test_variants_deepseek_via_other_provider_is_unchanged`,
  `reasoning_effort_maps_supported_levels_and_rejects_others`,
  `deepseek_lists_live_reasoning_efforts`,
  `deepseek_models_via_other_providers_keep_their_policy`.
- Verification (2026-11-05): `cargo test -p opencode-provider` (151 tests),
  `cargo test -p opencode-session` (184 tests), `cargo test -p opencode-server
  variant_listing` (2 tests), `cargo check --workspace` and `cargo clippy
  --workspace --all-targets` (no new warnings in touched files) all pass.

### Pass 3 — Cache-safe prompt prefix and cache counters

- Split the system prompt at `build_system_prompt_parts` in
  `crates/opencode-server/src/agentic.rs`: the byte-stable part is the agent/model
  base prompt; the environment block, date, and available-skills list are
  `dynamic_context`.
- `ResolvedAgenticContext` carries `dynamic_context`; the server passes it through
  `prompt_with_update_hook` → `loop_inner` → `build_chat_messages`, which emits the
  stable system message first and the dynamic context as its own system message.
  Model-visible content and ordering are unchanged (same two blocks, still before
  history), but a date/skills change no longer rewrites the leading bytes.
- Cache counters: `Usage` now aliases DeepSeek's `prompt_cache_hit_tokens` into
  `cache_read_input_tokens` and derives miss from the prompt total; streaming
  usage events carry `cache_read_tokens`/`cache_miss_tokens`; DeepSeek streaming
  requests send `stream_options.include_usage=true` so the terminal usage chunk
  arrives. The session loop persists `cache_read_tokens`/`cache_miss_tokens` in
  the assistant usage metadata and `MessageUsage.cache_read_tokens`.
- Tests: `dynamic_context_does_not_rewrite_the_stable_system_prefix` (session),
  `build_system_prompt_parts_keeps_dynamic_context_out_of_the_stable_prefix`
  (server), `deepseek_cache_hit_alias_populates_cache_read` and
  `stateful_usage_exposes_deepseek_cache_counters` (provider).
- Live check (2026-11-05): a streaming `deepseek-flash` request with
  `stream_options.include_usage` ended with
  `prompt_cache_hit_tokens: 0`, `prompt_cache_miss_tokens: 33`, and a non-streaming
  request returned the same fields.
- Verification (2026-11-05): `cargo test -p opencode-provider` (154 tests),
  `cargo test -p opencode-session` (185 tests), `cargo test -p opencode-server`
  (78 tests), `cargo check --workspace`, and `cargo clippy --workspace
  --all-targets` (no new warnings in touched files) all pass.

### Pass 4 — Anthropic Messages transport (evidence-gated)

- Added `crates/opencode-provider/src/deepseek_anthropic.rs`, selected by
  `provider.deepseek.options.transport = "anthropic"` (or
  `DEEPSEEK_TRANSPORT=anthropic`). The default remains `/chat/completions`.
- Requests use `https://api.deepseek.com/anthropic/v1/messages`, `x-api-key`, and
  `anthropic-version: 2023-06-01`. Leading system messages become a `system` block
  array; late system messages stay in history (the `in-history` shape). Tools map
  to `{name,description,input_schema}`. Assistant reasoning is echoed back as a
  `thinking` block (empty signature; the endpoint accepts it on tool follow-ups,
  where it is mandatory). Tool results map to user `tool_result` blocks and
  consecutive results merge into one user message.
- Streaming uses a new stateful Anthropic SSE parser
  (`parse_anthropic_sse_stateful` / `anthropic_compat_sse_stream`) that keeps the
  content-block index table, so thinking closes correctly and tool-input deltas
  carry the real `tool_use` id. Usage events carry the full prompt total and the
  cache-read/miss split.
- `output_config.effort` accepts `low`/`high`/`max`; unsupported values fail before
  network I/O.

End-to-end probes (live, 2026-11-05, `deepseek-flash`):

- system as a block array accepted; in-history `role: system` accepted.
- tools returned `stop_reason: tool_use` with a `thinking` + `tool_use` block; a
  tool-result follow-up without the thinking block is rejected, with an empty
  signature it succeeds.
- streaming emitted `thinking_delta`, `text_delta`, `input_json_delta`, and a
  terminal `message_delta` usage frame.

Fixed-workload comparison (same ~2.9k-token stable prefix, 6 runs each, second run
onward warm):

| transport | total input | output | cache read | hit rate | p50 | p95 |
|---|---|---|---|---|---|---|
| `/chat/completions` | 2896 | 12.3 | 2688 | 92.8% | 854 ms | 1484 ms |
| `/anthropic/v1/messages` | 2896 | 15.2 | 2688 | 92.8% | 1008 ms | 1433 ms |

The Anthropic transport does **not** measurably win: cache hit rate is identical,
token counts are equivalent, and latency is within noise (chat is slightly faster
at p50). Per the handoff it stays opt-in and `/chat/completions` remains the
default; the null result is recorded rather than promoted.
- Tests: 4 conversion/response tests in `deepseek_anthropic`, the stateful SSE
  parser test, and `deepseek_transport_option_selects_anthropic`.
- Verification (2026-11-05): `cargo test -p opencode-provider` (160 tests),
  `cargo test -p opencode-session` (185 tests), `cargo test -p opencode-server`
  (78 tests), `cargo check --workspace`, and `cargo clippy --workspace
  --all-targets` (no new warnings in touched files) all pass.
