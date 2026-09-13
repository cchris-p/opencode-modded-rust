---
id: "FEAT-015"
title: "Preserve manually selected model and provider across sessions"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "wiki/v1.md"
status: "todo"
created: "2026-09-11"
---

# Preserve manually selected model and provider across sessions

## Summary

When a user manually picks a non-default provider/model (for example via `Settings > Provider`), that choice must be remembered and loaded for every session afterwards, instead of falling back to the built-in/default model on the next session or app restart.

This is the follow-up to `BUG-010`, which fixed the repo default model/provider taking effect. The default now wins only when the user has not overridden it; this card covers making an explicit manual override stick.

## Desired behavior

- If the user selects a provider/model that is **not** the default, that selection is the active provider/model for every new session from then on.
- The selection survives: creating a new session, opening an older session, and restarting the TUI/app.
- The default model/provider still applies only when no manual selection has ever been made, or after the user explicitly resets to default.
- The effective selection stays visible in `Settings > Provider` (effective provider/model/source), so the user can tell why a given model is active.

## Current state

- `Settings > Provider` Enter calls `opencode-tui` `provider_selection_patch(model_ref)`, PATCHes `/config`, and the server persists it via `update_config` (writes the project config). The TUI then sets `context.current_model`.
- `opencode-server` `effective_provider_setup` derives `effective_model` from the loaded config (`config.model`) when `OPENCODE_TUI_MODEL` is unset.
- `opencode-tui` `refresh_model_dialog` only adopts `setup.effective_model` when `context.current_model` is `None`.
- Session prompts send the in-memory `context.current_model` (`selected_model_for_prompt`); session views fall back to the last assistant message's model when none is set.

Suspected gaps to confirm during implementation:

- Whether the persisted selection is written to the same config file the loader treats as the project default (`update_config` writes `opencode.json`; the repo ships `opencode.jsonc`), and whether a stray `opencode.json` is created/left behind.
- Whether a newly created session always reads the persisted/current selection rather than re-deriving the default.
- Whether switching to an existing session can leave the active model stale relative to the persisted selection.
- Whether the manual selection should be user-global (machine-level) or project-level; pick and document one behavior.

## Scope

- Persist a manually selected provider/model so it becomes the normal default for subsequent sessions.
- Restore that selection on TUI/server startup.
- Keep the effective-source reporting in `Settings > Provider` accurate.
- Add tests for the persistence + restore path.

## Non-goals

- Reworking provider auth or the broader provider-setup UX (`START-027` owns the authoritative path).
- Per-agent or per-session model overrides beyond what already exists.
- Full upstream parity for model-selection persistence.

## Likely touchpoints

- `crates/opencode-tui/src/components/settings.rs` (`provider_selection_patch`, effective source display)
- `crates/opencode-tui/src/app/app.rs` (`refresh_model_dialog`, `set_active_model_selection`, new-session flow)
- `crates/opencode-tui/src/api.rs` (`patch_config`)
- `crates/opencode-server/src/routes.rs` (`effective_provider_setup`, `patch_config` -> `update_config`)
- `crates/opencode-config/src/loader.rs` (`update_config` target file, project vs global persistence)

## Done when

- After a manual non-default selection, a freshly created session uses that provider/model without the user re-selecting it.
- The selection survives a full TUI/app restart.
- No stray/duplicate config file is created that silently changes precedence.
- `Settings > Provider` shows the restored selection and its source.
- A user who never selected manually still gets the built-in default, and an explicit reset-to-default restores that.

## Verification

- Select a non-default provider/model in `Settings > Provider`.
- Create a new session and confirm the selected model is used (check the status line and the request).
- Restart `ort-build`/`ort`, start another session, and confirm the same model is active.
- Reset to the default and confirm the default becomes active again.
- `cargo test -p opencode-config -p opencode-tui -p opencode-server`.

## Related Items

- `BUG-010` Repo default model and provider ignored due to config precedence
- `START-027` Unify provider setup into one authoritative user path
- `START-019` Add native Ollama support for the local-model-first V1 path
- `START-015` Mirror OpenAI auth configuration in settings
