---
id: "FEAT-038"
title: "Question schema and tool contract parity"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: ""
created: "2026-09-21"
---

# Question schema and tool contract parity

## Summary

Child of `GATE-002` (parity gap 1). Align the Rust `question` tool input/output schema and
contract with vanilla `QuestionV2.Prompt`/`Info`/`Reply` and the tool's `toModelOutput`, and make
the tool use the session question callback instead of blocking on stdin.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 1.

## Problem

- `crates/opencode-tool/src/question.rs:99` ignores `ctx` and reads process stdin through a local
  `ask_question` helper (`:130-212`). Only `plan.rs` calls `ctx.question(...)`, so the `question`
  tool never reaches the live server/TUI flow.
- `QuestionDef.header` and `QuestionOption.description` are optional (`crates/opencode-tool/src/tool.rs:44-59`),
  but vanilla requires both.
- The tool flattens every answer into `Vec<String>` and returns JSON (`question.rs:107-119`),
  losing per-question arrays and not matching vanilla's model-facing text.
- No `custom` answer flag exists anywhere.

## Vanilla reference

Frozen reference line per `AGENTS.md`. Authoritative shapes live in `packages/schema/src/question.ts`
and `packages/core/src/tool/question.ts`:

- `Option = { label, description }`, both required.
- `base = { question, header, options, multiple? }`.
- `Info = base + custom?` (default `true`).
- Tool `Input = { questions: Prompt[] }`; `Output = { answers: Answer[] }` with `Answer = string[]`.
- `toModelOutput` = `User has answered your questions: "Q"="A1, A2", "Q2"="Unanswered". You can now
  continue with the user's answers in mind.`
- Tool description includes: custom answers default on, answers are arrays of labels, and
  "(Recommended)" guidance.

Note: vanilla's tool `Input` uses `Prompt` (no `custom`); this card exposes `custom` as an optional
model-facing field per the gate's acceptance criteria ("optional custom-answer behavior"), defaulting
to true so behavior matches vanilla when omitted.

## Scope / deliverables

- `crates/opencode-tool/src/tool.rs`: `QuestionDef.header: String` (required);
  `QuestionOption.description: String` (required); keep `multiple` default `false`; add
  `custom: bool` with a serde default of `true`.
- `crates/opencode-tool/src/plan.rs`: update the two `QuestionDef` literals to the new types and set
  `custom: false` (binary yes/no confirmations).
- `crates/opencode-tool/src/question.rs`: remove the stdin code path; deserialize
  `{ questions: Vec<QuestionDef> }`; call `ctx.question(questions.clone()).await?`; return the vanilla
  model-facing string as `output`; store the structured per-question arrays in `metadata["answers"]`.
- Tool `description()` copied verbatim from vanilla (custom note, label arrays, "(Recommended)").
- `parameters()` JSON schema: per-question `required: ["question","header","options"]`, option
  `required: ["label","description"]`, `header` `maxLength: 30`, `custom` boolean default true.
- `crates/opencode-server/src/routes.rs`: `QuestionPromptInfo` carries `custom`; header/description
  map through as always-present values.
- `crates/opencode-tui/src/api.rs`: `QuestionPromptInfo` carries `custom` (default true) so the field
  survives the full pipeline.

## Non-goals

- TUI custom-answer/digit/review UX (`FEAT-041`).
- Session-scoped routes and ownership checks (`FEAT-039`).
- Runtime event/lifecycle cleanup (`FEAT-040`).
- Execution-time permission assertion (`FEAT-043`).

## Acceptance criteria

- [ ] A `question` call with `{question, header, options:[{label, description}], multiple?, custom?}`
      validates; missing `header` or an option missing `description` is rejected.
- [ ] `custom` omitted defaults to `true`; `multiple` omitted defaults to `false`.
- [ ] The tool never reads stdin; with no question callback configured it fails with an explicit error.
- [ ] Per-question answer arrays are preserved in the tool result metadata.
- [ ] Model-facing output matches vanilla's format, using `Unanswered` for empty answers and joining
      multi-select labels with `, `.
- [ ] `cargo test -p opencode-tool question` covers schema parsing (required header/description,
      custom/multiple defaults) and output formatting (single, multi-select, custom text, unanswered).

## Verification

- `cargo fmt --all`
- `cargo check -p opencode-tool -p opencode-session -p opencode-server -p opencode-tui`
- `cargo test -p opencode-tool question`

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-039` session-scoped question API parity.
- `FEAT-040` question runtime event and lifecycle parity.
- `FEAT-041` TUI question prompt UX parity.
- `CLI-009` CLI and direct-run question parity (hold).
- `FEAT-043` question permission integration parity.
- `FEAT-044` question parity verification fixtures.
- `CLI-002` Route `opencode run` through the canonical session runtime - gated by `CLI-001`/`CLI-006`.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/65

## Dev Notes (2026-09-21, PR feature/FEAT-038-question-schema-tool-contract)

- `crates/opencode-tool/src/tool.rs`: `QuestionDef.header` is now a required `String`,
  `QuestionOption.description` is now a required `String`, and `QuestionDef` gained
  `custom: bool` defaulting to `true` (`default_custom`).
- `crates/opencode-tool/src/question.rs`: removed the stdin code path and the flattened
  `QuestionResponse`. `execute` now deserializes `{ questions: Vec<QuestionDef> }`, calls
  `ctx.question(...)`, returns vanilla's model-facing text, and stores the per-question answer arrays
  under `metadata["answers"]`. The `description()` text and JSON schema were updated to vanilla
  (`required: ["question","header","options"]`, option `required: ["label","description"]`,
  `header.maxLength: 30`, `custom` default true). Empty question lists are rejected.
- `crates/opencode-tool/src/plan.rs`: updated the two `QuestionDef` literals to the required
  header/description strings and set `custom: false` for the binary plan-mode confirmations.
- `crates/opencode-server/src/routes.rs`: `QuestionPromptInfo`/`QuestionOptionInfo` now carry
  always-present `header`/`description` and a `custom` flag.
- `crates/opencode-tui/src/api.rs`: `QuestionPromptInfo` carries `custom` (default true) so the field
  survives the full pipeline; rendering remains `FEAT-041`.
- Verification: `cargo fmt --all`; `cargo check --workspace` clean; `cargo test -p opencode-tool`
  (44 pass, including 12 new `question::tests` covering schema parsing, defaults, schema shape,
  callback routing with per-question arrays, no-callback error instead of stdin, and model output).

### Deviations

- Vanilla's tool `Input` uses `QuestionV2.Prompt`, which has no `custom`; this card exposes `custom` as
  an optional model-facing field per the gate's acceptance criteria, defaulting to true so behavior
  matches vanilla when omitted.
- `options` remains optional-with-default in Rust (lenient superset) to preserve the existing free-text
  question path; vanilla marks the key required.

## Merge Closeout (2026-09-21)

- PR #65 (`feature/FEAT-038-question-schema-tool-contract`) merged into `development` at merge commit
  `405a6e1ad6651e77897b08da02d3ed886a8be77f`; remote and local feature branch deleted.
- Card intentionally kept in `qa` for post-merge local verification; not moved to `done` because no QA
  report is recorded and the user's closeout request covered the merge, not final completion.
- `GATE-002` remains open until `FEAT-039`..`FEAT-044` are delivered or resolved; `CLI-009` stays in
  `hold` behind `CLI-001`/`CLI-006` and `CLI-002`.

## QA Feedback Fix (2026-09-21)

- Feedback: pressing `Enter` in a TUI option question submitted the question even when no option was
  toggled/selected.
- Fix: `crates/opencode-tui/src/components/question.rs` now leaves option prompts open and returns no
  answer from `QuestionPrompt::confirm()` when the selected answer set is empty; text prompts keep their
  existing behavior.
- Regression coverage: added component tests for single-choice and multiple-choice no-selection guards,
  plus selected-option submission.
- Verification: `cargo fmt --all`; `cargo test -p opencode-tui question::tests`.
