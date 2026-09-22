---
id: "BUG-026"
title: "Save session title after new session creation"
priority: "P2"
type: "bug"
area: "BUG"
spec: "docs/opencode-session.md"
status: "done"
created: "2026-09-21"
---

# Save session title after new session creation

## Summary

A newly created session starts with a placeholder title (`New session - <rfc3339>`). The
human-readable title should be generated and **persisted** once the session has its first
user message, so it survives a server restart and every prompt entrypoint behaves the same.

Today the generated title is only produced as a side effect of the first *assistant* step
postprocessing and is only guaranteed to reach storage at the end of a completed prompt. If
the first assistant turn uses tools, the prompt is aborted, the entrypoint is the SSE
`/stream` path, or the process exits before the final sync, the session keeps the
placeholder title.

## Current behavior / code evidence

- `Session::new` seeds the placeholder: `title: format!("New session - {}", now.to_rfc3339())`
  (`crates/opencode-session/src/session.rs:378`).
- Title generation only runs inside the non-streaming prompt loop:
  `Self::ensure_title(...)` (`crates/opencode-session/src/prompt.rs:1295`), defined at
  `crates/opencode-session/src/prompt.rs:2307`, which calls
  `generate_session_title_llm` with `generate_session_title` as fallback.
- That call is gated by `should_run_first_step_postprocessing(post_first_step_ran, has_tool_calls)`
  (`crates/opencode-session/src/prompt.rs:1415`), which returns `false` when the first
  assistant turn has tool calls, so a tool-first session is not titled at that point.
- The SSE entrypoint `stream_message` (`crates/opencode-server/src/routes.rs:1369`) writes
  messages and streams events but never calls `ensure_title`.
- Persistence happens through `persist_sessions_if_enabled` ->
  `ServerState::sync_sessions_to_storage` (`crates/opencode-server/src/server.rs:188`). For
  the task prompt path this is the post-prompt sync at `crates/opencode-server/src/routes.rs:2057`.
- Session creation also persists (`crates/opencode-server/src/routes.rs:411-436`), so the
  placeholder is what is durable until the title is generated and re-synced.
- Manual rename already exists: `set_session_title` route (`/session/{id}/title`) and
  `ApiClient::update_session_title` (`crates/opencode-tui/src/api.rs:479`), covered by
  `BUG-015` for immediate surface refresh.

## Expected behavior

- After the **first user message** is accepted, the session gets a generated (or fallback)
  title that is written to storage without waiting for the whole prompt loop to finish.
- The generated title survives: switching sessions, restarting the TUI/app, and restarting
  the server.
- Behavior is identical across `/prompt`, `/prompt/async`, and `/stream`.
- A user rename is never overwritten by a later generated title.
- Aborted or failed generation is retried on a later prompt without clobbering a user title.

## Ideas for handling this

1. **Trigger on first user message, not first assistant step.**
   Move title generation to the point where the first user message is appended (shared by all
   prompt entrypoints) instead of gating on `should_run_first_step_postprocessing`. Pro:
   titles exist even for tool-first or aborted turns. Con: needs a provider handle at that
   point and must stay out of the hot path.

2. **Persist on set (targeted write).**
   After `Session::set_title`, flush just this session (`session_repo.update` or an equivalent
   `persist_session`) and broadcast `session.updated`, rather than depending on the
   end-of-prompt full `sync_sessions_to_storage`. Pro: durable immediately, cheap for one row.
   Con: introduces a second write path to keep consistent with the full sync.

3. **Generate the fallback synchronously, upgrade to the LLM title asynchronously.**
   Write `generate_session_title(first_user_text)` immediately (instant, no provider call),
   then spawn the LLM title and persist it only if the title is still the default. Pro:
   removes latency from the prompt path; resilient to aborts. Con: title can visibly change
   shortly after.

4. **Unify the entrypoints behind one post-first-message hook.**
   Extract a single helper (e.g. `maybe_title_after_first_user_message`) called by
   `session_prompt`, `prompt_async`, and `stream_message`, with the update/persist hook
   supplied by the caller. Pro: no entrypoint can silently skip titling. Con: refactor spans
   `routes.rs` and `opencode-session`.

5. **Idempotency + retry marker.**
   Record a `title_generated` / `title_is_default` marker in session state and retry while
   the title is default. Use `is_default_title()` as the guard so a manual rename stops
   further generation. Pro: safe retries, no rename clobbering. Con: one more piece of
   persisted metadata to migrate.

6. **TUI reflects the saved title.**
   When `session.updated` (source `title`/`prompt.final`) arrives or when a prompt response
   returns the session, update the current header/session list from the returned session
   rather than relying on a later list refresh (extends `BUG-015`). Pro: confirms the save to
   the user. Con: mostly presentation, not the root fix.

**Leaning:** combine 1 + 3 for the generation timing, add 2 (or 4's caller hook) for the
durable write, and use `is_default_title()` (5) as the rename guard. Treat 6 as the follow-up
UI polish if the current surfaces still lag.

## Open questions

- Should title generation happen at session creation (with an empty/placeholder title) or on
  first user message? Generation needs message text, so first user message is the natural
  trigger.
- Which model/provider generates the title — the session's selected model, or a fixed cheap
  model? Confirm the cost/latency tradeoff for the blocking path.
- Should a failed LLM title keep the fallback title, or leave the placeholder and retry later?
- Do child/forked sessions get titles via the same path (`Child session - <ts>`), or keep
  their derived/fork suffix?

## Non-goals

- Reworking manual rename UX (`BUG-015` owns immediate surface refresh).
- Session-summary cascading by session name (`SKILL-001`).
- The broader v1/v2 prompt-loop consolidation epic — reference it rather than folding this
  into it.

## Likely touchpoints

- `crates/opencode-session/src/prompt.rs` (`ensure_title`, `should_run_first_step_postprocessing`, call site)
- `crates/opencode-session/src/session.rs` (`Session::new`, `set_title`, `is_default_title`)
- `crates/opencode-server/src/routes.rs` (`create_session`, `session_prompt`, `stream_message`, post-prompt persist)
- `crates/opencode-server/src/server.rs` (`sync_sessions_to_storage`, per-session persist)
- `crates/opencode-storage/src/repository.rs` (`SessionRepository::update`)
- `crates/opencode-tui/src/api.rs` / `crates/opencode-tui/src/app/app.rs` (title refresh)

## Done when

- A new session gets a non-placeholder title persisted after the first user message, even
  when the first assistant turn only calls tools or the prompt is aborted.
- The title survives a server restart (`list_sessions` returns the generated title).
- `/prompt`, `/prompt/async`, and `/stream` all title the session consistently.
- A manually renamed session is never overwritten by generated titles.
- Tests cover the tool-first and abort paths.

## Verification

- Create a new session and send a prompt whose first assistant turn only calls tools; confirm
  the session list shows a generated title, not `New session - ...`.
- Restart the server and confirm the title is still present.
- Send a prompt through `/stream` and through `/prompt`; confirm both persist a title.
- Rename a session, then run another prompt; confirm the rename is preserved.
- `cargo test -p opencode-session -p opencode-server -p opencode-tui`.

## Dev Notes

Merged via PR https://github.com/cchris-p/opencode-modded-rust/pull/71 (merge commit `0310c38`)
into `development`; branch `bug/BUG-026-save-session-title-after-new-session-creation` deleted
remotely and locally. Re-verified on `development`: `cargo test -p opencode-session -p
opencode-server -p opencode-tui` (new tests pass; only the pre-existing macOS/UTF-8 failures
remain).

Implemented the leaning from the card: **1 + 3** for timing, **2** for the durable write, and
`is_default_title()`-style guards for **5**.

- **Title is driven by the first user message, not the first assistant step.**
  `SessionPrompt::loop_inner` now calls `ensure_title` once before the step loop whenever
  `Session::should_generate_title()` is true, and the old call inside the
  `should_run_first_step_postprocessing` block was removed. Tool-first and aborted turns are
  now titled (`crates/opencode-session/src/prompt.rs`).
- **Durable fallback written at accept time.** `apply_initial_session_title` writes the instant
  `generate_session_title` fallback for a still-placeholder session and is called by the shared
  `accept_prompt` (covers `/prompt` and `/prompt/async`) and by `stream_message` (covers
  `/stream`). `accept_prompt`'s existing `persist_sessions_if_enabled` flushes it; `/stream`
  persists before streaming so an aborted turn still keeps the title
  (`crates/opencode-server/src/routes.rs`).
- **Async LLM upgrade.** The agentic runner upgrades the fallback in-loop and its snapshots carry
  the title; the `/stream` path has no runner, so `spawn_title_upgrade` performs the LLM upgrade
  off the request path. Both keep the fallback if the LLM fails.
- **Rename and retry guards.** `Session` gained `title_auto` metadata plus
  `is_auto_title_pending` / `should_generate_title` / `set_auto_title` / `finalize_auto_title` /
  `mark_title_user_owned`. `set_session_title` and `update_session` mark the title user-owned, so
  a manual rename clears the pending marker and is never overwritten. A failed LLM upgrade stays
  pending and is retried on a later prompt; a successful upgrade finalizes
  (`crates/opencode-session/src/session.rs`).
- **Tests.** `ensure_title_titles_tool_first_session_from_first_user_message`,
  `ensure_title_never_overwrites_user_rename`, `ensure_title_retries_pending_fallback_without_clobbering_it`,
  `test_auto_title_lifecycle_and_user_rename_guard`,
  `accepted_prompt_persists_fallback_title_without_completing_the_turn` (asserts the SQLite row),
  and `accepted_prompt_keeps_user_renamed_title`.

Verification run: `cargo test -p opencode-session -p opencode-server -p opencode-tui`. New tests
pass. Two pre-existing `opencode-session` instruction-path tests and two pre-existing
`opencode-tui` prompt tests fail unchanged on the clean base (macOS path/UTF-8 environment), not
touched by this change.

## Related Items

- `BUG-015` Session rename immediately refreshes current terminal surfaces
- `SKILL-001` Add session-summary cascade skill by session name
- `FEAT-015` Preserve manually selected model and provider across sessions