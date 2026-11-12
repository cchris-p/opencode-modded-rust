---
id: "FEAT-077"
title: "Show selected model in the models menu"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-11-12"
updated: "2026-11-12"
---

# Show selected model in the models menu

## Summary
Currently, the models menu (/models) does not visually indicate which model is currently selected. This feature adds an explicit visual indicator next to the currently selected model in the models menu.

## Related Items
- `FEAT-015`: Preserve manually selected model and provider across sessions (in QA)
- `BUG-074`: Truncated native tool-call block drops the intended file write without retry

## Acceptance Criteria
- [x] When the user opens the models menu (with the /models keybinding), the currently selected model is visually distinguished from the others
- [x] The visual indicator is clear and consistent with the UI design (a muted `(current)` suffix, matching the Themes dialog)
- [x] The selected model is indicated even if the menu is opened before any model is explicitly selected (the default is seeded first)

## Constraints
- Must not break the existing functionality of model selection
- Must work with the current provider and model listing

## Verification
- Open the models menu and observe that the currently selected model is visually indicated
- Change the model selection and verify the indicator updates accordingly

## Implementation Notes
- `ModelSelectDialog` stores the active model ref (`provider/model`) and renders a muted `(current)` marker next to it, mirroring the Themes dialog's current-theme marker.
- `App::refresh_model_dialog` seeds the dialog's current model from `AppContext::current_model` after the default-model seeding logic, so the marker also shows when no explicit selection has been made yet.
- Tests in `crates/opencode-tui/src/components/dialogs/model_select.rs` assert the marker appears only on the active model and is omitted when unset or blank.

## Completion
- Implemented in PR #191 (`feat(tui): mark the active model in the models menu`), merged into `development` at `1852d38`.
- Released in `v0.1.3`: `main` `e2334a3`; immutable release published 2026-11-12 with Linux and macOS assets.
- Fleet deploy `standards-deploy-ort --rev e2334a3` verified 8/10 active machines: `jasper`, `webapps`, `webappssub1`, `matrillobase1`, `matrillo`, `matrillosub1`, `matrillosub2`, `j8rsxy`.
- `webappssub2` is disabled/skipped; `moldavite` and `xp-farmer` were Tailscale-offline and could not be updated.
- QA: `cargo test -p opencode-tui` 216 passed (3 new render tests); PR CI fmt/check/clippy/test/native green.
- Remaining gate: bring `moldavite` and `xp-farmer` online to complete the fleet rollout (tracked on `INFRA-012`/`INFRA-014`).
