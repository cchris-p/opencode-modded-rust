---
id: "SKILLS-006"
title: "Surface skills in the command menu as annotated skill entries"
priority: "P2"
type: "feature"
area: "SKILLS"
spec: "wiki/skills-parity-audit.md"
status: "todo"
created: "2026-09-25"
---

# Surface skills in the command menu as annotated skill entries

## Summary

Skills are currently reachable only through a dedicated browse dialog or a separate prompt-level
completion path. The user wants each available skill to appear directly in the command/slash menu,
annotated as a skill, so a skill can be run from the menu without first opening the skill picker and
selecting one.

The agreed surface is the slash-command popup (`SlashCommandPopup`) opened by `/`. Skills appear only
while the user is filtering with a non-empty query; the bare `/` list keeps showing the suggested
built-in commands.

Evidence:

- The command registry holds only built-in slash commands; `/skills` opens the browse dialog rather
  than listing skills inline (`crates/opencode-tui/src/command.rs:519-527`,
  `crates/opencode-tui/src/app/app.rs:2037-2038`, `crates/opencode-tui/src/app/app.rs:2326-2332`).
- Skill names are injected into the prompt's own completion list as bare `/name` candidates with no
  source/type annotation, separate from the command registry
  (`crates/opencode-tui/src/components/prompt.rs:988-1001`,
  `crates/opencode-tui/src/components/prompt.rs:284-285`,
  `crates/opencode-tui/src/app/app.rs:3320-3326`).
- The slash-command popup renders only registry commands and has no notion of an entry source or
  badge (`crates/opencode-tui/src/components/slash_command.rs:98-170`).
- `SkillSummary` already carries `name` and optional `description`, so a skill entry can render a
  title plus a skill annotation (`crates/opencode-tui/src/api.rs:56-60`).

## Why this exists

Running a skill today requires a detour: open the skill list, Scroll/search, select, and only then the
prompt is populated. The command menu is the primary entry point for actions, and skills are actions.
Surfacing them there (and labeling them as skills) removes a selection step and makes skills
discoverable in the same place as every other command.

## Refinement Decisions

- Surface: the `SlashCommandPopup` only. The prompt's inline `/` completion list is left as-is.
- Bare `/`: built-in suggested commands only. Skill entries are added only once the popup query is
  non-empty (i.e. while filtering), so pressing `/` alone does not dump the full skill catalog.
- Selection: selecting a skill entry inserts `/name ` into the prompt (matching the skill list dialog
  at `crates/opencode-tui/src/app/app.rs:1449-1451`) and closes the popup; it does not execute the
  skill immediately. The user can append arguments and submit.
- Annotation: each skill row renders a muted `· skill` suffix, distinct from built-in command rows.

## Scope

- List loaded skills as entries in `SlashCommandPopup` only, sourced from the existing
  `list_skills` API, merged with built-in command matches while filtering.
- Show skill entries only while the popup query is non-empty; the bare `/` list keeps showing only
  suggested built-in commands.
- Annotate each skill entry with a muted `· skill` suffix in the list row.
- Selecting a skill entry inserts `/name ` into the prompt and closes the popup; it does not execute
  immediately.
- Keep the `/skills` browse dialog and the `/skill` prompt completion working; this is additive.

## Non-goals

- URL-backed or other new skill sources/trust/caching models (`SKILLS-002`).
- Changing skill referencing, reinjection, or execution semantics (`SKILLS-005`).
- Restyling unrelated command categories or the command palette's non-skill entries.

## Done when

- Typing `/` followed by any query character surfaces matching loaded skills in the popup without
  opening the skill picker; pressing `/` alone still shows only built-in suggested commands.
- Each skill row is annotated with a muted `· skill` suffix and is visually distinguishable from
  built-in command rows.
- Selecting a skill entry inserts `/name ` into the prompt (trailing space included) and closes the
  popup; the prompt is then submittable as a normal skill invocation with no picker dialog.
- Empty skill lists, large skill lists, and duplicate/near-duplicate skill names do not break the
  popup or its filtering; a skill name colliding with a built-in command does not produce a duplicate
  row or a broken selection.
- The existing `/skills` dialog and `/skill` inline completion still work unchanged.

## Implementation Notes

Affected surfaces:

- `crates/opencode-tui/src/components/slash_command.rs`: add a skill list to `SlashCommandPopup`
  (`set_skills`, `skills: Vec<SkillSummary>`), merge fuzzy-matched skills into `refresh_filter()` only
  when `!query.is_empty()`, dedup against registry command names, render the muted `· skill` suffix
  for non-registry rows, and have `select_current()` emit a skill action when the selected row is not
  a registry command.
- `crates/opencode-tui/src/command.rs`: add a `CommandAction` variant that carries the selected skill
  name (e.g. `InsertSkill(String)`) so popup selection can flow through the existing
  `take_action()` -> `execute_command_action()` path.
- `crates/opencode-tui/src/app/app.rs`: handle the new action by setting prompt input to
  `format!("/{} ", name)` and closing the popup; load/cache skills into the popup when it opens
  (`app.rs:741`) and reuse the existing loader (`refresh_skill_list_dialog`, `app.rs:3320-3326`) so
  the dialog, the inline completion, and the popup stay in sync from one source.

Constraints:

- Do not show skills in the bare `/` list; gate on non-empty query in `refresh_filter()`.
- Preserve `SlashCommandPopup`'s close-on-select behavior and do not route skill selection through a
  dialog.
- Keep the annotation purely presentational; store entry kind in the data model rather than
  re-parsing the rendered label.
- Treat skill loading as best-effort like the existing dialog path; a load failure must not prevent
  the popup from showing built-in commands.

## Recommended verification

- `cargo check -p opencode-tui -p opencode-server`
- `cargo test -p opencode-tui`
- Manual: `ort-build` then `ort`; press `/` and confirm no skills appear yet, type a letter, confirm
  matching skills appear with the `· skill` suffix, press Enter on one, and confirm the prompt now
  reads `/name ` with no dialog opened.

## Related Items

- `SKILLS-001` Align skills with current and reference OpenCode behavior (done; foundation)
- `SKILLS-003` Achieve exact local skills list parity (done; foundation)
- `SKILLS-002` Plan URL-backed skills parity (source model, explicitly out of scope)
- `SKILLS-004` Add session-summary cascade skill by session name
- `SKILLS-005` Compare skill referencing and per-step reinjection parity with vanilla (qa; invariant)
- `BUG-037` Slash menu freezes keyboard input until terminal resize (same slash-menu surface; check for
  interaction before changing popup state handling)

## Notes

- There are two separate surfaces today: the `SlashCommandPopup` registry menu and the prompt's inline
  completion list. This card scopes to the popup only; feed the popup from the same skill loader used
  by the dialog and inline completion so the three do not drift.
- Prefer extending the existing command entry model (an explicit `source`/`kind`) over string-sniffing
  annotations at render time.
