---
id: "FEAT-076"
title: "Session title generation parity with vanilla OpenCode"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-11-11"
updated: "2026-11-11"
---

# Session title generation parity with vanilla OpenCode

## Summary

Bring automatic session titling in line with vanilla OpenCode so a new session gets a model-generated, human-readable title instead of the first user prompt copied verbatim.

## Why this exists

ORT currently carries the first line of the user's prompt into the durable session title. A prompt like "create board item to match parity with opencode vanilla session naming feature" becomes the session name, which is noisy, exposes raw prompt text in session lists and headers, and diverges from vanilla behavior. Vanilla generates the title with a dedicated hidden `title` agent and a small model, using a strict title prompt and deterministic cleanup; it never uses the user's input as the title.

## Current ORT behavior

- `generate_session_title` returns the first line of the user message verbatim, truncated to 100 characters (`crates/opencode-session/src/prompt.rs:3863`). This is the fallback and the title users currently see.
- `apply_initial_session_title` writes that fallback synchronously as soon as the first user message is accepted, so it becomes the durable visible title before any model call (`crates/opencode-server/src/routes.rs:287`; SSE path at `crates/opencode-server/src/routes.rs:2366-2416`).
- The async upgrade `generate_session_title_llm` uses the session's main provider/model and an inline generic instruction ("Generate a short title ..."), not the builtin `title` agent (`crates/opencode-session/src/prompt.rs:3877`).
- The builtin `title` agent exists but is not used for title generation (`crates/opencode-agent/src/agent.rs:296-315`; `wiki/agent-modes-and-custom-agents.md`). Its prompt is a short "3-5 word" instruction, not the vanilla title prompt.
- `small_model` config and `ProviderRegistry::get_small_model` exist but have no production callers (`crates/opencode-config/src/schema.rs:64`, `crates/opencode-provider/src/bootstrap.rs:2213`).
- Title lifecycle guards already exist and work: `is_default_title`, `is_auto_title_pending`, `should_generate_title`, `set_auto_title`, `finalize_auto_title`, `mark_title_user_owned` (`crates/opencode-session/src/session.rs:434-479`). Manual rename protection must be preserved.

## Vanilla reference

Pinned reference commit `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (branch `dev`).

- `SessionPrompt.ensureTitle` (`packages/opencode/src/session/prompt.ts:193-254`):
  - Skips sessions with a `parentID` and sessions whose title is not the default title.
  - Requires exactly one real (non-synthetic) user message before generating.
  - Resolves the `title` agent; model priority is the title agent's configured model, then the provider small model, then the session model.
  - Sends the conversation context as `"Generate a title for this conversation:\n"` plus model messages, with `retries: 2` and a `small: true` hint.
  - Strips `<think>...</think>` blocks, takes the first non-empty line, and caps the title at 100 characters (`97 + "..."`).
  - Leaves the default title in place when generation fails or returns nothing; it never writes the user's prompt text as the title.
- Title agent definition: `packages/opencode/src/agent/agent.ts:234-246` (hidden, temperature 0.5, tools denied).
- Title prompt: `packages/opencode/src/agent/prompt/title.txt` (single line, <= 50 characters, same language as the user, no tool names, examples, etc.).
- Small model selection: `Provider.getSmallModel` (`packages/opencode/src/provider/provider.ts:1939-1998`), honoring `small_model` config first, then provider/model-family priority.

## Parity gaps

1. The verbatim first line is the durable visible title instead of the default placeholder.
2. Title generation bypasses the builtin `title` agent and the vanilla title prompt.
3. No small-model routing; the session's main model is always used.
4. `get_small_model` is dead code for title generation.
5. No per-request retries and no `small: true` request hint.
6. Cleanup differs: ORT removes every `"` and `'` and only skips lines that start with `<think>`; vanilla strips whole think blocks and otherwise preserves the line.
7. Title agent definition drifts from vanilla: `Subagent` mode instead of hidden primary, temperature 0.3 vs 0.5, `max_tokens` 1024, short prompt vs `title.txt`.
8. Guard parity is incomplete: ORT does not require exactly one real user message and does not explicitly skip child sessions (the default-title guard mostly covers the latter today).

## Scope / deliverables

- Port the vanilla `title.txt` prompt and align the builtin `title` agent definition (hidden, temperature 0.5, tools denied, same prompt).
- Add a title model resolution path: title agent configured model -> `small_model` config / provider small model -> session model. Wire the existing `get_small_model` instead of leaving it unused.
- Build the title request through the title agent prompt rather than the inline generic instruction, including the `"Generate a title for this conversation:\n"` prefix, retries, and the `small` hint where the provider layer supports it.
- Match vanilla cleanup: strip `<think>...</think>` blocks, take the first non-empty line, cap at 100 characters.
- Stop writing the user's prompt text as the visible/durable title. Keep the default placeholder until a real title is generated, and keep the title retryable if generation fails.
- Preserve and extend the existing lifecycle guarantees: manual rename wins, every prompt entrypoint persists and broadcasts the final title, and a restart preserves it.
- Add tests for model-selection priority, the one-real-user-message guard, cleanup behavior, no-verbatim fallback, and manual-rename protection.

## Decisions / recommended approach

- Match vanilla's user-visible behavior: no verbatim fallback. If generation fails, keep the session on its placeholder/default title and retry later rather than substituting the user's text.
- This supersedes the `BUG-026` decision to generate and persist a synchronous fallback titled from user input. `BUG-026`'s durability and entrypoint coverage stay in force; only the title content source changes.
- Keep title generation off the hot path where it already is, but do not compromise correctness by showing raw input while a better title is pending.

## Non-goals

- Manual rename UX, shortcuts, or session-list editing (`FEAT-029`, `BUG-015`, `FEAT-025`).
- Session list filtering/loading or workspace scoping.
- Storage schema changes or backfilling titles for existing sessions.
- Summarization, compaction, or the `summary` agent.
- Session fork/branch title derivation.

## Done when

- Sending a first prompt never sets the session title to the user's prompt text verbatim.
- The title is produced through the title agent prompt via the small model when available, falling back to the session model otherwise, with retries.
- All prompt entrypoints (`/prompt`, `/prompt/async`, `/stream`, agentic loop) persist and broadcast the same generated title.
- A failed title generation leaves a retryable placeholder rather than exposing raw input.
- Manual rename continues to win over generated titles.
- Tests cover the resolution priority, guards, cleanup, no-verbatim behavior, and rename protection.

## Recommended verification

- Unit tests: title model resolution order (agent model -> configured small model -> provider small model -> session model), one-real-user-message guard, `<think>` block stripping, 100-character cap, and no-verbatim fallback.
- Unit test: a user rename followed by another prompt keeps the user title.
- `ort-build`, then `ort`; create a new session, send a long multi-line prompt, and confirm the title is a concise generated title, not the prompt text.
- Restart the server and confirm the title survives; rename the session and send another prompt to confirm the rename is preserved.
- Exercise the default `deepseek/deepseek-flash` path and a provider with a small-model candidate (or set `small_model` explicitly) to confirm selection and latency behavior.
- Simulate a provider failure (bad credentials/offline) and confirm no verbatim title appears.

## Likely touchpoints

- `crates/opencode-session/src/prompt.rs` (`generate_session_title`, `generate_session_title_llm`, `ensure_title`)
- `crates/opencode-server/src/routes.rs` (`apply_initial_session_title`, `spawn_title_upgrade`, prompt/stream entrypoints)
- `crates/opencode-agent/src/agent.rs` (`AgentInfo::title`, prompt/definition parity)
- `crates/opencode-provider/src/bootstrap.rs` (`get_small_model` wiring)
- `crates/opencode-config/src/schema.rs` (`small_model` config)
- `crates/opencode-session/src/session.rs` (title lifecycle guards)
- `crates/opencode-tui` (only if title refresh handling needs to reflect the new generation path)

## Dev Notes

Implemented on `feat/FEAT-076-session-title-generation-parity`.

- Added `opencode_core::SESSION_TITLE_PROMPT`, ported verbatim from vanilla `title.txt`, and pointed the builtin `title` agent at it. The agent is still hidden; its mode is now `Primary`, temperature `0.5`, and it no longer carries the old 3-5 word prompt.
- `generate_session_title_llm` now builds the vanilla request shape (instruction line + first user message, title-agent system prompt, temperature 0.5), retries transient provider errors up to three attempts, strips `<think>...</think>` blocks, takes the first non-empty line, and caps the title at 100 characters. It returns `Option<String>` and never falls back to user text.
- Added `session_title_source` with the vanilla guards: root session only, default/pending title only, and exactly one non-empty user message.
- Removed the synchronous verbatim fallback (`apply_initial_session_title`). A failed or empty generation now leaves the default placeholder in place and stays retryable. A model result that echoes the user's first line is rejected.
- Title model resolution now follows configured `title` agent model -> `small_model` config / provider small-model family priority -> session model. `ProviderRegistry::get_small_model` was previously unused and is now wired for `/prompt`, `/prompt/async`, `/stream`, and child task paths. Manual-rename protection is preserved by re-checking `should_generate_title` after the model call.
- Updated `wiki/agent-modes-and-custom-agents.md` to show the title agent as a hidden primary.

Verification:

- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace` passes.
- `cargo test -p opencode-provider --lib`: 152 passed, including new `test_get_small_model_priority_and_config`.
- `cargo test -p opencode-agent --lib`: 22 passed.
- `cargo test -p opencode-session title`: all 13 title-related tests pass, including new failure/echo/multi-user/cleanup cases.
- `cargo test -p opencode-server --lib`: 76 passed, including the updated default-title and user-rename tests.
- `cargo clippy -p opencode-core -p opencode-provider -p opencode-agent -p opencode-session -p opencode-server --all-targets`: no new errors; only pre-existing warnings.
- Known pre-existing failures unrelated to this change: `opencode-session` `instruction::tests::test_find_up_stops_at_stop_dir` and `test_find_up_walks_parents` (macOS path/UTF-8, also documented in `BUG-026`).

## Related Items

- `BUG-026` Save session title after new session creation (superseded fallback decision; keep its durability work)
- `FEAT-029` Learn how manual session rename works end to end
- `BUG-015` Session rename immediately refreshes current terminal surfaces
- `FEAT-025` Map Ctrl+R to rename
- `GATE-004` subagent feature parity (title agent definition/role alignment)

## Notes

- Reference behavior is from the pinned `dev` commit `f54ce313b99a6661d7758ad042f7a6e05c8e0972`.
- The `title` agent is hidden/internal, so changing its mode should not add a picker entry.
- Keep the existing durable-write and retry guards; the change is what supplies the title content, not when it is persisted.
