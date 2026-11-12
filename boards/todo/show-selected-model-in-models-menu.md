---
id: "FEAT-077"
title: "Show selected model in the models menu"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-11-12"
---

# Show selected model in the models menu

## Summary
Currently, the models menu (/models) does not visually indicate which model is currently selected. This feature will add an explicit visual indicator (such as a checkmark or highlight) next to the currently selected model in the models menu.

## Related Items
- `FEAT-015`: Preserve manually selected model and provider across sessions (in QA)

## Acceptance Criteria
- [ ] When the user opens the models menu (with the /models keybinding), the currently selected model should be visually distinguished from the others
- [ ] The visual indicator should be clear and consistent with the UI design (e.g., a checkmark, highlight, or bold text)
- [ ] The selected model should be indicated even if the menu is opened before any model is explicitly selected (in which case the default model should be shown as selected)

## Constraints
- Must not break the existing functionality of model selection
- Must work with the current provider and model listing

## Verification
- Open the models menu and observe that the currently selected model is visually indicated
- Change the model selection and verify the indicator updates accordingly

## Implementation Notes
- `ModelSelectDialog` now stores the active model ref (`provider/model`) and renders a muted `(current)` marker next to it, mirroring the Themes dialog's current-theme marker.
- `App::refresh_model_dialog` seeds the dialog's current model from `AppContext::current_model` after the default-model seeding logic, so the marker also shows when no explicit selection has been made yet.
- Tests in `crates/opencode-tui/src/components/dialogs/model_select.rs` assert the marker appears only on the active model and is omitted when unset or blank.
