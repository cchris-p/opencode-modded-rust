---
id: "START-025"
title: "Add retrieval-provider boundary for task context assembly"
priority: "P2"
type: "feature"
area: "START"
spec: "wiki/scopemux-integration-plan.md"
status: "done"
created: "2026-08-30"
---

# Add retrieval-provider boundary for task context assembly

## Summary

Introduce a narrow runtime retrieval interface so task context assembly can stay generic in V1 while leaving a clean integration point for future `ScopeMux` support.

## Why this exists

`START-007` defines `ScopeMux` as a future retrieval-quality layer rather than a V1 dependency, but the current runtime still builds context directly inside session and prompt code. `SCOPE-002` cannot plug into anything until this boundary exists.

## Scope

- Define the minimum retrieval request and response shapes the runtime needs.
- Keep the first provider generic and repository-local.
- Refactor only enough context assembly to consume that boundary cleanly.
- Feed the request from authoritative `SessionTask` state rather than transcript text.

## Current runtime (recon)

- Live path is `SessionPrompt::loop_inner` (`crates/opencode-session/src/prompt.rs:1065`); context is assembled inline at `prompt.rs:1107-1217`, then `build_chat_messages` (`prompt.rs:2042`) and `parts_to_content` (`prompt.rs:2072`).
- Explicit file/symbol selection today is `@`-mention resolution (`resolve_prompt_parts` `prompt.rs:2954`, `extract_file_references` `prompt.rs:3023`) plus LSP symbol end-line expansion (`lookup_symbol_end_line` `prompt.rs:672`). There is no first-class seed/symbol input and no role concept (implementing vs reviewing).
- File discovery primitives already exist: `opencode-grep` (`crates/opencode-grep/src/search.rs`), the `read` tool (`crates/opencode-tool/src/read.rs:50`), and `opencode-lsp`. `.gitignore` is not honored today (hardcoded `EXCLUDED_DIRS` in `crates/opencode-tool/src/grep_tool.rs:24`).
- Authoritative task state is `Session.task: Option<SessionTask>` (`crates/opencode-session/src/session.rs:312`, `SessionTask` in `crates/opencode-types/src/task.rs:52`) and is currently **not consulted by assembly**. It owns `objective`, `completion_criteria`, `workspace_target`, `stage`, `verification_*`, `review_*`, `reopen_reason`.
- The v2 loop (`crates/opencode-session/src/llm.rs`, `StreamInput` `llm.rs:39`) is not normally reachable but is slated to merge with v1 (`FEAT-011`), so the boundary must be usable by both.
- No retrieval/context-provider trait exists; model it on `Provider` (`crates/opencode-provider/src/provider.rs:52`).

## Proposed design

- **Contract types in `opencode-types`** (no internal deps), matching `wiki/scopemux-integration-plan.md:113-131`:
  - Request: `objective`, `stage`, `workspace_root`, optional `seed_files`/`seed_symbols`/`changed_files`, `role` (`Implementing`/`Reviewing`), `token_budget`.
  - Response: ordered candidates (file/symbol/snippet), each with provenance, confidence, and enough metadata for the runtime to choose final inclusion.
- **Boundary in a small `opencode-retrieval` crate** (depended on by `opencode-session`): an `async_trait` `RetrievalProvider` plus a `GenericRepositoryProvider` backed by `opencode-grep`/`opencode-lsp`/read primitives. A dedicated crate enforces "one provider among peers" and keeps `ScopeMux` logic out of `opencode-session`.
- **Runtime stays authoritative**: the provider returns evidence; the runtime keeps budgets, inclusion thresholds, and role filtering, and never lets the provider touch `advance_task`/`complete` (`session.rs:745`, `:864`).

## Insertion points

- Build the request from `Session.task` (objective/stage/workspace_target/reopen_reason) rather than transcript inference.
- In `prompt.rs`, route file/symbol selection for the live loop through the provider; keep `@`-mention resolution as an explicit seed source.
- Make the same boundary callable from `llm.rs` (`StreamInput`/`build_messages`) so the v1/v2 consolidation does not split it again.
- Default provider is generic; an absent/failed provider degrades to today's behavior.

## Decisions to settle

- Role model: introduce `RetrievalRole` now, or derive it from the agent name for the first cut?
- Budget: assembly-level token/item budget now, or reuse agent/compaction budgets first?
- Gitignore: does the generic provider own ignore handling, or stay with the existing hardcoded exclusions?
- Scope of the refactor: v1 live path only, or v1 + v2 `StreamInput` together?

## Implementation plan

1. Add contract types to `opencode-types` with serde and unit tests.
2. Add `opencode-retrieval` with the trait, a no-op/generic provider, and tests.
3. Encode a `RetrievalRequest` from `SessionTask`.
4. Refactor `prompt.rs` selection to consume the provider, preserving current behavior when the provider returns generic candidates.
5. Wire the same boundary into `llm.rs`.
6. Add integration coverage (below).

## Tests

- `cargo test -p opencode-session` (task transitions and prompt loop): `crates/opencode-session/tests/session_integration.rs:95`, `:134`, `:167`; prompt tests at `prompt.rs:4041`, `:5321`.
- New: a request is derived from `SessionTask` fields; the generic provider returns ranked candidates with provenance; assembly output is unchanged for a baseline task when the provider is generic; task still completes when the provider is absent.
- `cargo fmt`, `cargo check`, and `cargo clippy --workspace --all-targets` (README.md:149-153).

## Done when

- The runtime has one explicit retrieval-provider boundary for task context assembly.
- V1 still works without `ScopeMux`.
- Future `ScopeMux` integration can target that boundary instead of scattering logic across the runtime.

## Implementation

First cut landed on `feature/START-025-retrieval-provider-boundary` (PR #104, awaiting QA):

- Contract types in `opencode-types` (`crates/opencode-types/src/retrieval.rs`).
- `opencode-retrieval` crate: `RetrievalProvider` trait + `GenericRepositoryProvider`.
- v1 live-path consumption in `SessionPrompt::create_user_message`, deriving the request from `Session.task` + explicit seed files and retaining candidates as message provenance.
- Provider is optional; absence/failure leaves generic behavior unchanged.
- Role is fixed to `Implementing`; budgets reuse existing agent/compaction limits; gitignore handling unchanged; v2 `StreamInput` adoption deferred to the `FEAT-011` consolidation.

Verification: `cargo test -p opencode-types -p opencode-retrieval` and `cargo test -p opencode-session retrieval` pass; `cargo clippy -p opencode-retrieval` clean.

## Related Items

- `PHASE-003` (phase parent)
- `START-005` Define V1 runtime loop
- `START-007` Plan ScopeMux integration
- `START-016` Define structured task state for V1
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary - first consumer of this boundary.
- `FEAT-011` / `consolidate-v1-v2-session-prompt-loops` - must not split the boundary.

## Notes

- Keep this focused on the boundary, not on shipping graph retrieval.
- Recon performed 2026-09-23; file references above are from that pass and should be re-verified before editing.