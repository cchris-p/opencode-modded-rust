---
id: "FEAT-071"
title: "Hide model text outside the model and provider menus"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
created: "2026-11-11"
updated: "2026-11-11"
---

# Hide model text outside the model and provider menus

## Summary

Remove ambient model-identity text (provider/model name and variant) from the TUI chrome so it
no longer appears during normal reading and prompting. Model identity should remain visible only
in the surfaces whose job is choosing or inspecting it: the `/models` picker and the
Settings > Provider screen (`/settings`, `/provider`, `/connect`).

Concretely, hide the model from:

- the prompt info row (bottom-left chrome, also used on the Home view),
- the assistant turn footer,
- the `/status` runtime diagnostics dialog.

## Why this matters

The model name is repeated in several persistent places even though it changes rarely. It adds
visual noise to every prompt and every assistant turn, and it competes with information that does
change (agent/mode, duration, context usage). The user wants a quiet transcript and prompt area
while still being able to check or change the model deliberately from the model/provider menus.

## Scope

- `crates/opencode-tui/src/components/prompt.rs`: the info row currently renders
  `agent  model provider · variant`. Keep the agent label; drop the model, provider, and variant
  spans, and the now-unused `current_model`/`current_provider`/variant reads in `render`.
- `crates/opencode-tui/src/components/session.rs`: `assistant_footer` currently renders
  `▣ <mode> · <model> · <duration>`. Keep mode, duration, and `interrupted`; drop the model span
  and the `fallback_model` plumbing that exists only to feed it.
- `crates/opencode-tui/src/app/app.rs`: remove the `Model: ...` line from the `/status` runtime
  dialog and the now-unused `current_model_label` helper.

## Non-goals

- Do not change the `/models` picker (`components/dialogs/model_select.rs`) or the
  Settings > Provider screen (`components/settings.rs`); they remain the place to see connected
  providers and the selected/available models.
- Do not change model/provider selection, persistence, variant cycling, or the `/models` and
  `/settings` commands.
- Do not change the exported transcript header (`assistant_header` in `app.rs`), which keeps model
  metadata for provenance.
- Do not add a configuration flag; this is the desired default behavior.

## Acceptance criteria

- The prompt info row shows only the active agent (and shell mode) — no model, provider, or
  variant text — on both Home and Session views.
- The assistant turn footer shows mode, elapsed duration, and `interrupted` when applicable, but
  never the model name.
- The `/status` dialog no longer prints a `Model:` line.
- `/models` and Settings > Provider still show the connected providers and selected/available
  models exactly as before.
- No dead code or unused-variable warnings remain from the removed model plumbing.

## Verification

- `cargo check -p opencode-tui`
- `cargo test -p opencode-tui`
- Add/keep a unit test around `assistant_footer` asserting a message with `model: Some(...)`
  renders mode + duration without the model string.
- Manual: `ort-build` then `ort`; confirm Home and a running Session show no model name in the
  prompt row or assistant footers, `/status` has no `Model:` line, and `/models` plus
  `/settings` still list providers/models.

## Related Items

- `BUG-053` Assistant turn footer shows the fallback "Default" instead of the active agent
  (build) — same footer rendering area.
- `FEAT-015` Preserve manually selected model and provider across sessions — selection behavior
  stays untouched.
- `FEAT-067` Add Novita AI to provider settings and auth surfaces — the Settings > Provider
  surface that keeps showing providers/models.
- `START-012` Refresh provider and model catalog — catalog surfacing in the model menus.

## Dev Notes

- `components/prompt.rs`: the prompt info row now renders only the agent label (or `shell` in
  shell mode). Removed the `current_model`, `current_provider`, and `current_model_variant`
  reads from `render`; the idle `variants` keybind hint is unchanged.
- `components/session.rs`: `assistant_footer` renders `▣ <mode> · <duration>` (plus
  ` interrupted`) and no longer emits the model span. Removed the `fallback_model` field from
  `MessageRenderCtx`, its per-frame read and layout hash input, and the `assistant_footer`
  parameter.
- `app/app.rs`: removed the `Model: ...` line from the `/status` runtime dialog and the
  now-unused `current_model_label` helper.
- Left intact: `/models` (`components/dialogs/model_select.rs`) and Settings > Provider
  (`components/settings.rs`, `/settings`, `/provider`, `/connect`), which still list connected
  providers and the selected/available models, and the exported transcript header metadata.
- Added `assistant_footer_hides_model_name` in `components/session.rs` tests.

## QA Verification

- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-tui`: clean, no warnings.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui`: 189 passed, 0 failed; the new
  `assistant_footer_hides_model_name` test passes.
- `cargo fmt --all`: no further changes.

