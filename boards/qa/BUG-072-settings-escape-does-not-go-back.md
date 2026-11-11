---
id: "BUG-072"
title: "Esc cannot leave Settings/Provider; it is swallowed by the session interrupt keybind"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-11-11"
updated: "2026-11-11"
---

# Esc cannot leave Settings/Provider; it is swallowed by the session interrupt keybind

## Summary

After opening Settings (including via `/settings`, `/provider`, or `/connect`), pressing `Esc`
does not return to the previous screen. The only way out is to quit the TUI. On the Home view this
looks like being trapped on Settings with no path back to the prompt.

## Root cause

`Esc` is bound to `session_interrupt` (`crates/opencode-tui/src/context/keybind.rs:194`). The
interrupt handler (`crates/opencode-tui/src/app/app.rs:588`) runs before the Settings key handler
(`app.rs:761`) and consumes `Esc` on every route:

- the shell-mode branch does not apply on Settings,
- the `Route::Session` branch does not apply on Settings, so control reaches the fall-through
  `clear_interrupt_confirmation(); return Ok(());`,
- that return prevents `handle_settings_key` from ever seeing `Esc`, so `Router::go_back()` is
  never called.

## Fix

Gate the interrupt handler to the routes where interrupting a run is meaningful
(`Route::Home | Route::Session`). On `Route::Settings` (and `Route::Help`) `Esc` now falls through to
the route-specific handler, and `handle_settings_key` calls `Router::go_back()`. Sub-states
(API-key/OAuth/Ollama input, auth-method selection) still consume `Esc` first to cancel themselves,
so exiting Settings takes one extra `Esc` only while a sub-input is open.

## Verification

- New unit test `session_interrupt_only_owns_escape_on_transcript_routes` asserts the predicate is
  true for Home/Session and false for Settings/Help.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui --lib` — 204 passed, 0 failed.
- `cargo fmt --all -- --check` — clean.
- Manual: open `/settings` from Home, press `Esc`, confirm the prompt/Home view returns; repeat with
  `/provider` and `/connect`.

## Related Items

- `FEAT-073` Hide model text outside the model and provider menus — Settings remains the place to see
  providers/models; this restores leaving it.
- `BUG-071` Session step budget — unrelated, same `app.rs` key dispatch area.
