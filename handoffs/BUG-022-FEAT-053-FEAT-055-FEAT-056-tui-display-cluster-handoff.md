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

## Settled Pre-Implementation Decisions

The strict refinement gate flagged five open items. They are now resolved from code evidence and are
**binding for implementation**; do not reopen them without an explicit new user decision.

1. **`FEAT-053` config key names and shape - SETTLED: snake_case keys, no serde aliases.**
   Add these optional fields to `TuiConfig` (`crates/opencode-config/src/schema.rs:331`) and populate
   `Config.tui` (`schema.rs:23`): `thinking`, `tool_calls`, `tool_details`, `timestamps`,
   `message_density`, `semantic_highlight`, `header`, `scrollbar`, `tips_hidden`. Each is `Option<T>`
   with `#[serde(skip_serializing_if = "Option::is_none")]`. Use snake_case with no aliases, matching
   the existing `TuiConfig` fields (`scroll_speed`, `diff_style`) and the vanilla reference tui schema
   (`packages/opencode/src/config/tui-migrate.ts:72`). `message_density` accepts the existing
   `MessageDensity` strings `compact`/`cozy` (`app_context.rs:76-91`). Do **not** add a `sidebar` key
   (not persisted on `AppContext`; owned by `FEAT-008`).
2. **`FEAT-053` precedence - SETTLED: persisted kv override > config startup default > built-in default.**
   A present `kv.json` value always wins; config is consulted only when the kv key is absent; the
   current hard-coded literal is the final fallback. Config is therefore a declarative startup
   default that never silently reverts an explicit in-session toggle.
   - Add `UiKv::get_bool_opt`, `UiKv::get_string_opt`, and `UiKv::get_timestamps_opt` returning
     `Option` (`app_context.rs:390-433`), then seed each flag with
     `kv_opt.or(config_value).unwrap_or(builtin)` (`app_context.rs:146-160`).
   - Config must reach `AppContext`. `AppContext::new()` takes no config (`app_context.rs:127`) and is
     built at `app.rs:114`. In `App::new()`, after the workspace directory is resolved, run
     `opencode_config::ConfigLoader::load_all(workspace_dir)` (already a TUI dependency,
     `crates/opencode-tui/Cargo.toml:12`) and pass the merged `Config` into a new
     `AppContext::new_with_config(&Config)`; keep `AppContext::new()` delegating to
     `Config::default()` so the `prompt.rs:1704` test caller is unaffected. Only `config.tui` is
     consumed; provider/model authority stays with the server.
   - Unit test: kv set + config set -> kv wins; kv absent + config set -> config wins; both absent ->
     built-in.
3. **`FEAT-055` unused render stack - SETTLED: port the semantics into the live line renderer, then delete the stack outright.**
   Evidence it is fully dead: `MessageView` is never constructed (`message.rs` only defines it and
   re-exports it from `components/mod.rs:41`); `ToolCallView`, `ToolResultView`, `BashToolView`,
   `ReadToolView`, `WriteToolView` and every `tool_views.rs` type appear only at their definition and
   the `components/mod.rs:55-61` re-exports; none of the three files contain `#[cfg(test)]` tests. The
   views render into a `Frame` (`fn render(&self, frame, area, theme)`), not `Vec<Line>`, so
   "re-homing" them into the live transcript would require rewriting them anyway.
   - Re-implement the per-tool content as line spans in `session_tool.rs` (bash command + output +
     exit/running status, read path, write path, edit diff, todowrite list, glob/grep counts).
   - Delete `components/message.rs`, `components/tool_call.rs`, and `components/tool_views.rs`; drop
     their `mod`/`pub use` entries in `components/mod.rs` (`:23`, `:33`, `:41`, `:55-61`).
   - `components/thinking.rs` (`ThinkingBlock`, `mod.rs:52`) is also defined-but-never-called; fold
     its removal into `BUG-022` unless that phase reuses it.
4. **`FEAT-055`/`FEAT-056` wrapping strategy - SETTLED: word-aware wrapping with a hard-break fallback.**
   Upgrade the shared wrapper `wrap_spans` (`session.rs:1453-1485`) so it breaks at whitespace and only
   hard-breaks a single token wider than the content width, preserving span styles and explicit `\n`.
   `wrap_block_line` -> `paint_block_lines` then gives tool/bash lines the same treatment as markdown
   `Paragraph::wrap`. Tests assert whole words move to the next line and an over-width unbroken token
   hard-breaks.
5. **`FEAT-055` vs `FEAT-056` order - SETTLED: `FEAT-055` core first, then `FEAT-056`.**
   `FEAT-056` must be the concrete instance of the unified path, not a special case layered on the
   stack `FEAT-055` removes. Within the single PR commit `055`-core before `056`; in the board, record
   `FEAT-056 -> FEAT-055` as a predecessor.

## Dependency Order

Two independent clusters; within each, one edge. Ordering is **explicit in the cards** except where
marked inferred.

**Cluster A - display state**

1. `BUG-022` first. It changes what `show_thinking` means (visibility vs per-block collapse) in
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

- Invert the reasoning expansion default (settled): track an explicit per-block
  `collapsed_reasoning: HashSet<String>` (replacing `expanded_reasoning`, `session.rs:46`) and render
  the collapsed count only when the `{msg.id}:{part_idx}` id is a member. When `show_thinking` is on,
  reasoning is expanded by default, including historical and in-progress content; a manual collapse
  persists for that block. Invert the click handler (`session.rs:1041-1051`) accordingly and keep the
  visibility scrub at `session.rs:972` (`retain` on the collapsed set now scrubs collapsed ids).
- Preserve the existing `thinking_visibility` ui key for the global show/hide flag.
- Make `render_reasoning_part` (`session_text.rs:47`) emit content in the shown state; the collapsed
  state keeps the `▶ Thinking (N lines)` header. Once collapse is membership-based,
  `THINKING_PREVIEW_LINES`/`preview_lines` are unused; remove them or document them, but do not revive
  the old "collapsed = preview" behavior.
- Remove the unused `ThinkingBlock` path (`components/thinking.rs`, `mod.rs:52`) as part of this
  phase; it has no live caller (the transcript uses `session_text::render_reasoning_part`).
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
`crates/opencode-tui/src/app/app.rs` (config load + `AppContext::new_with_config`),
`docs/opencode-config.md`, `docs/opencode-tui.md`.

Approach:

- Add the nine `tui` keys and their snake_case shape per Settled Decision 1 in
  `crates/opencode-config/src/schema.rs`.
- Add `UiKv` optional getters and seed each `AppContext` flag as
  `kv_opt.or(config.tui.<key>).unwrap_or(builtin)` per Settled Decision 2; `timestamps` keeps its
  string/`bool` encoding via `get_timestamps_opt`.
- Wire the merged config in: load `ConfigLoader::load_all(workspace_dir)` in `App::new()` and pass it
  through `AppContext::new_with_config(&Config)` per Settled Decision 2.
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
`:1282-1310`), `components/mod.rs`; delete `message.rs`, `tool_call.rs`, `tool_views.rs`.

Approach:

- Make the live transcript use a single tool renderer. Route tool-call lines through
  `paint_block_lines` so they get the gutter, background, and width-aware wrapping that text/file/
  image/footer parts already get.
- Stop hard-truncating output with `format_preview_line(line, 96)`; wrap to content width and keep
  explicit expand/collapse for long output.
- Reconcile inline-vs-block classification (`ToolRenderMode` vs `is_block_tool`) and unify preview
  budgets (bash 10 lines vs other 6; error 3/1).
- Port the per-tool view semantics into the live line renderer, then delete `components/message.rs`,
  `components/tool_call.rs`, and `components/tool_views.rs` and their re-exports per Settled
  Decision 3 (caller check already done: no non-test caller; `grep` for `ToolCallView`,
  `BashToolView`, `tool_views` returns definitions and re-exports only).
- Upgrade the shared `wrap_spans` to word-aware wrapping with a hard-break fallback per Settled
  Decision 4.

Acceptance (from card): exactly one renderer serves the transcript; tool blocks align with other
parts and wrap; no 96-column truncation; expand/collapse still works; per-tool views shown or
documented generic; `cargo check/test -p opencode-tui` pass with tests for the unified path and
wrapping.

### Phase 4 - `FEAT-056`: bash terminal block

Files: `session_tool.rs` (`render_tool_call`, `shell_command_text`, preview/truncation sites),
`session.rs`, `tool_call.rs` (`BashToolView` `:190-263`, semantics only; the file is deleted in Phase 3).

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
2. Apply the settled pre-implementation decisions above (no open choices remain).
3. Implement and verify `BUG-022`; commit.
4. Implement and verify `FEAT-053`; update docs; commit.
5. Implement and verify the `FEAT-055` core slice; commit.
6. Implement and verify `FEAT-056` on the unified path; commit.
7. Update board files (lanes, statuses, Dev Notes, predecessors); commit.
8. Run the full verification set; open the PR targeting `development`; move the four items to `qa`.
9. Leave the PR open and the branch checked out for the user's local QA.

## Risks And Rollback

- **`FEAT-055` is the highest-risk item**: it deletes exported code and changes the live
  render path. Land it as its own commit so it can be reverted independently within the PR.
- **`FEAT-053` precedence**: an incorrect rule silently reverts a user's in-session toggle on
  restart; the unit test is the guardrail.
- **`BUG-022` default flip**: showing reasoning expanded whenever thinking is on can be noisy on
  long sessions. This is an accepted consequence of the card's product decision; if a streaming-only
  default is wanted, reopen it with the user before implementation.

## Deferred / Out Of Scope

- `FEAT-054` (TUI color scheme consistency) is already `done`; only reference its theme-token
  approach.
- `FEAT-028` (tool-call compaction toggle) behavior is unchanged.
- Streaming/PTY/ANSI bash output is explicitly out of scope.
- No other `todo` items are absorbed into this PR.

## Readiness Assessment

All five pre-implementation decisions are settled above and the refinement gate passes. `BUG-022`,
`FEAT-053`, `FEAT-055`, and `FEAT-056` each have explicit scope, non-goals, done-when, verification,
file evidence, and a fixed implementation order. The handoff is implementation-ready; begin at Step 2
of the Execution Sequence.
