---
id: "H-007"
title: "BUG-022/FEAT-053/FEAT-055/FEAT-056 TUI display cluster - Handoff"
status: "open"
created: "2026-09-23"
updated: "2026-09-23"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["BUG-022", "FEAT-053", "FEAT-055", "FEAT-056"]
---

# BUG-022/FEAT-053/FEAT-055/FEAT-056 TUI Display Cluster - Handoff

## Objective

Make the TUI transcript display trustworthy and configurable in one coherent change set:

- `/thinking` actually shows reasoning content live instead of a line count (`BUG-022`).
- The display toggles can be declared in `opencode.json` and seeded at startup (`FEAT-053`).
- Tool calls render through one path with the same block/wrap treatment as every other message part (`FEAT-055`).
- `bash`/`shell` calls render as a wrapped terminal-style block with exit status (`FEAT-056`).

Decided execution shape (from the planning session): **one PR** covering all four items plus the board
updates, branched from `development`, merged only on explicit user direction after local QA.

## Included Board Items

| Item | Title | Priority | Lane at handoff time |
|------|-------|----------|----------------------|
| `BUG-022` | `/thinking` toggle shows a line count instead of the actual reasoning in real time | P2 | `qa` (moved manually earlier; see Note below) |
| `FEAT-053` | Make TUI display-toggle defaults configurable from `opencode.json` | P2 | `todo` |
| `FEAT-055` | Unify and improve tool / script display in the session transcript | P3 | `todo` |
| `FEAT-056` | Bash command display: render a terminal block with text wrap | P2 | `todo` |

**Note on `BUG-022` lane.** It was moved `todo -> qa` in the same session at the user's direction
("qa pass"), but there is no implementation commit for it (only the card-creation commit `16929b7`).
Because this handoff implements it, move `BUG-022` back to `doing` when implementation starts and to
`qa` only after the PR is created. Do not treat the current `qa` placement as a completed QA.

## Pre-Implementation Decisions To Settle

These are the open items the strict refinement gate flagged. Settle them at the top of the
implementation session; the defaults below are the recommended resolution, not yet confirmed.

1. **`FEAT-053` config key names and shape.** The card leaves "names and serde aliases to be settled
   during refinement". Proposed keys under the existing `tui` section: `thinking`, `tool_calls`,
   `tool_details`, `timestamps`, `message_density`, `semantic_highlight`, `header`, `scrollbar`,
   `tips_hidden`. Confirm before touching `crates/opencode-config/src/schema.rs`.
2. **`FEAT-053` precedence.** Card product decision prefers *config = startup default, persisted
   `kv.json` value = explicit runtime override*. Confirm; implement exactly one precedence and unit
   test it.
3. **`FEAT-055` unused render stack.** `MessageView::render_tool_call`, `ToolCallView`/`BashToolView`
   and `tool_views.rs` are exported but appear to have no live caller. Decide: **re-home the per-tool
   views into the live path** (preferred, satisfies FEAT-056's "use `BashToolView` semantics") and then
   delete the dead code, versus deleting outright. Confirm no non-test caller first.
4. **`FEAT-055`/`FEAT-056` wrapping strategy.** Decide whether tool command/output uses word-aware
   wrapping (parity with the markdown `Paragraph::wrap`) with a per-character hard-break fallback for
   long unbroken tokens. Recommended: yes; `wrap_spans` is currently greedy per-character.
5. **`FEAT-055` priority vs `FEAT-056`.** `FEAT-056` (P2) is the concrete instance of `FEAT-055`
   (P3). Recommended: implement the `FEAT-055` core slice first so `FEAT-056` is not built on the stack
   that `FEAT-055` deletes. If the batch must ship in one PR, order the commits 055-core -> 056.

## Dependency Order

Two independent clusters; within each, one edge. Ordering is **explicit in the cards** except where
marked inferred.

**Cluster A - display state**

1. `BUG-022` first. It changes what `show_thinking` means (visibility vs `expanded_reasoning`) in
   `app_context.rs` / `session.rs` / `session_text.rs`.
2. `FEAT-053` second. It seeds those same `AppContext` flags from config, so its `tui.thinking` key
   must encode corrected behavior. `FEAT-053`'s own Notes say to coordinate with `BUG-022`.
   - Explicit edge: `FEAT-053` depends on `BUG-022`.

**Cluster B - tool rendering**

3. `FEAT-055` core first. It collapses the duplicate stacks, routes tool lines through
   `paint_block_lines`/`wrap_block_line`, and stops fixed 96-column truncation.
4. `FEAT-056` second. It adds the terminal-style bash block in the same files
   (`session_tool.rs`, `session.rs`) and must be "the concrete instance of the unified tool render
   path rather than yet another special case".
   - Explicit edge: `FEAT-056` depends on the `FEAT-055` unified path.

The two clusters touch disjoint concerns and can be developed in parallel; only Cluster B requires
serialization because both items edit `session_tool.rs` and `session.rs`.

## Implementation Phases

### Phase 1 - `BUG-022`: live reasoning, collapse-by-exception

Goal: when thinking is shown, reasoning content is visible as it streams; hidden still hides; manual
collapse/expand remains optional.

Files: `crates/opencode-tui/src/components/session.rs`,
`crates/opencode-tui/src/components/session_text.rs`,
`crates/opencode-tui/src/context/app_context.rs`, `crates/opencode-tui/src/command.rs`.

Approach:

- Invert the reasoning expansion default. Today `expanded_reasoning` starts empty and
  `collapsed = !expanded_reasoning.contains(id)`, so shown-but-collapsed renders only the count.
  Either (a) track explicitly *collapsed* reasoning ids and render collapsed only on membership, or
  (b) keep the set but treat the in-progress/streaming part as expanded by default.
- Preserve the existing `thinking_visibility` ui key for the global show/hide flag.
- Make `render_reasoning_part` emit content (not only the `▶ Thinking (N lines)` header) in the
  shown state; decide whether the unreachable `THINKING_PREVIEW_LINES` preview is revived or left
  dead and documented.
- Update `handle_click` so manual collapse/expand still works and does not fight the global toggle.
- Update the `Show/hide thinking blocks` command description if behavior wording changes
  (`command.rs:413-422`).

Acceptance (from card): `/thinking` shows real reasoning text; live reasoning visible without a
per-block click; `/thinking` off hides; manual collapse still works; default behavior stated in Notes;
`cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass with a test that the shown state
emits content lines, not only a count.

### Phase 2 - `FEAT-053`: config-seeded display toggles

Files: `crates/opencode-config/src/schema.rs` (`TuiConfig` at `schema.rs:331-342`),
`crates/opencode-tui/src/context/app_context.rs` (seed flags `:146-160`, `UiKv` `:373-471`),
`docs/opencode-config.md`, `docs/opencode-tui.md`.

Approach:

- Add `tui` keys for the display flags (names confirmed in decisions above).
- Seed `AppContext` display flags from config when the corresponding kv key is absent, replacing the
  hard-coded default literals.
- Implement one precedence rule (recommended: config default, kv runtime override) and unit test it.
- Document the config keys and the previously-hidden runtime store
  (`dirs::state_dir()/opencode/kv.json`, observed at `~/.local/state/opencode/kv.json`).

Constraints/non-goals: keep `kv.json` as the runtime store (do not promote it into config); do not
remove the slash commands, palette entries, or keybinds; do not change what any toggle means;
`FEAT-053` Notes list extra `UiKv` keys that must *not* be mirrored.

Acceptance (from card): setting the display keys changes startup rendering without a `/` command;
precedence defined, implemented, unit-tested; existing toggles still work and still persist to
`kv.json`; docs updated.

### Phase 3 - `FEAT-055` core: one tool render path

Files: `crates/opencode-tui/src/components/session_tool.rs`,
`session.rs` (`paint_block_lines`/`wrap_block_line` `:1312-1419`, `append_rendered_tool_call`
`:1282-1310`), `message.rs` (`:133-270`), `tool_call.rs`, `tool_views.rs`,
`components/mod.rs`.

Approach:

- Make the live transcript use a single tool renderer. Route tool-call lines through
  `paint_block_lines` so they get the gutter, background, and width-aware wrapping that text/file/
  image/footer parts already get.
- Stop hard-truncating output with `format_preview_line(line, 96)`; wrap to content width and keep
  explicit expand/collapse for long output.
- Reconcile inline-vs-block classification (`ToolRenderMode` vs `is_block_tool`) and unify preview
  budgets (bash 10 lines vs other 6; error 3/1).
- Re-home or delete the unused view stack per the decision above; confirm no non-test caller
  (`grep` for `ToolCallView`, `BashToolView`, `tool_views`).
- Choose word-aware wrapping with a hard-break fallback.

Acceptance (from card): exactly one renderer serves the transcript; tool blocks align with other
parts and wrap; no 96-column truncation; expand/collapse still works; per-tool views shown or
documented generic; `cargo check/test -p opencode-tui` pass with tests for the unified path and
wrapping.

### Phase 4 - `FEAT-056`: bash terminal block

Files: `session_tool.rs` (`render_tool_call`, `shell_command_text`, preview/truncation sites),
`session.rs`, `tool_call.rs` (`BashToolView` `:190-263`).

Approach:

- Render bash/shell as a terminal block: `$` prompt line with the command wrapped to width, output in
  a framed/inset region with a consistent gutter, and visible exit/running status.
- Wrap command and output to content width; word-aware with hard-break fallback.
- Keep expand/collapse and preview-on-collapse, but make the collapsed preview wrap.
- Inherit active theme tokens (gutter/border/background), not fixed colors.

Non-goals: PTY allocation, live streaming output, ANSI/SGR parsing, changing execution/permissions,
a real scrollable terminal emulator, non-bash layouts (those stay in `FEAT-055`).

Acceptance (from card): over-width command wraps under the `$` prompt; over-width output wraps instead
of 96-column truncation; exit/failed state visible; collapsed and expanded both wrap and still
summarize hidden lines; theme tokens used; `cargo check/test -p opencode-tui` pass with wrapping
tests.

### Phase 5 - Docs and verification

- Update `docs/opencode-config.md` (new `tui` keys) and `docs/opencode-tui.md` (runtime state file).
- Run the per-card verification sections end to end with `ort-build` then `ort`, including terminal
  resize (narrow/wide) reflow, live reasoning streaming, a failing bash command, and a toggle
  restart-persistence check.
- Run `cargo fmt --all`, `cargo check -p opencode-tui`, `cargo check -p opencode-config`,
  `cargo test -p opencode-tui`, `cargo test -p opencode-config`.

## Parallel-Safe Steps

- Cluster A (Phases 1-2) and Cluster B (Phases 3-4) can be developed concurrently by different
  sessions; they share no files except read-only references.
- Within Cluster B, Phases 3 and 4 must be serialized.
- Within Cluster A, Phase 1 must precede Phase 2.
- Docs (Phase 5) can be drafted alongside Phase 2 and finalized after Phases 1-4.

## Verification / Completion Gates

- Gate A (before starting Phase 2): `BUG-022` behavior verified live; reasoning content visible
  while streaming.
- Gate B (before starting Phase 4): the `FEAT-055` unified path is in place and wrapping works for a
  generic tool, so `FEAT-056` adds the bash block onto the shared path.
- Gate C (before QA handoff): all four cards' acceptance criteria met; `cargo check`/`test` green for
  `opencode-tui` and `opencode-config`; docs updated; no dead tool-view stack remains.

## Single-PR Plan

| PR | Board Items | Branch | Notes |
|----|-------------|--------|-------|
| 1 | `BUG-022`, `FEAT-053`, `FEAT-055`, `FEAT-056` | `feature/tui-display-cluster` | One PR per explicit user decision. Commit order: `BUG-022` -> `FEAT-053` -> `FEAT-055` core -> `FEAT-056` -> docs. Board updates included in the same PR. |

- Target `development`. Do not merge until the user has tested locally on the checked-out branch and
  explicitly says to merge.
- Keep the PR open while the items sit in `qa`.

## Board Updates In The PR

1. Move `BUG-022` back to `doing` at the start (it is currently staged `todo -> qa`).
2. Move `FEAT-053`, `FEAT-055`, `FEAT-056` to `doing` while implementing.
3. After implementation and verification, move all four to `qa` with `status: "qa"` and the matching
   lane directory.
4. Add concise Dev Notes to each card: what changed, decisions taken, verification commands and
   observed results.
5. Add `predecessors`: `FEAT-053 -> BUG-022`; `FEAT-056 -> FEAT-055` (or record the ordering in
   Notes if the boards tooling does not use the field).
6. Do not create or delete board items.

## Execution Sequence

1. Pull latest `development`; create `feature/tui-display-cluster`.
2. Settle the five pre-implementation decisions above.
3. Implement and verify `BUG-022`; commit.
4. Implement and verify `FEAT-053`; update docs; commit.
5. Implement and verify the `FEAT-055` core slice; commit.
6. Implement and verify `FEAT-056` on the unified path; commit.
7. Update board files (lanes, statuses, Dev Notes, predecessors); commit.
8. Run the full verification set; open the PR targeting `development`; move the four items to `qa`.
9. Leave the PR open and the branch checked out for the user's local QA.

## Risks And Rollback

- **`FEAT-055` is the highest-risk item**: it deletes/re-homes exported code and changes the live
  render path. Land it as its own commit so it can be reverted independently within the PR.
- **`FEAT-053` precedence**: an incorrect rule silently reverts a user's in-session toggle on
  restart; the unit test is the guardrail.
- **`BUG-022` default flip**: showing all historical reasoning by default could be noisy; if that is
  not desired, implement "expanded while streaming, collapsed once complete" and record it in Notes.

## Deferred / Out Of Scope

- `FEAT-054` (TUI color scheme consistency) is already `done`; only reference its theme-token
  approach.
- `FEAT-028` (tool-call compaction toggle) behavior is unchanged.
- Streaming/PTY/ANSI bash output is explicitly out of scope.
- No other `todo` items are absorbed into this PR.

## Readiness Assessment

`BUG-022`, `FEAT-055`, and `FEAT-056` are implementation-ready (scope, non-goals, done-when,
verification, and file evidence are explicit). `FEAT-053` is *conditionally* ready: it names its own
open decisions (config key names/aliases and precedence), which the **Decisions To Settle** section
above captures with recommended defaults. Resolve those five decisions first, then execute in the
sequence above.
