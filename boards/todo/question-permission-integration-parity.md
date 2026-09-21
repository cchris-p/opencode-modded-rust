---
id: "FEAT-043"
title: "Question permission integration parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Question permission integration parity

## Summary

Child of `GATE-002` (parity gap 6). Record and implement the resolved decision for how the
`question` tool interacts with permissions. Resolved default: do not add a new execution-time
permission assert; keep the existing ruleset/tool-availability behavior and document the deviation
from vanilla.

## Parent

`GATE-002` question-tool full parity (`boards/todo/gate-question-tool-full-parity.md`), gap 6.

## Resolved decision

- Treat `question` as available wherever the agent's tool-list resolution allows it. The existing
  `question` rule (default `Deny`; `build` and `plan` override to `Allow`) in
  `crates/opencode-permission/src/ruleset.rs:227-231,274-277,289-292`, enforced by tool-list
  filtering in `crates/opencode-server/src/agentic.rs:127-145`, is sufficient.
- Do **not** add a second permission assertion inside `QuestionTool::execute`.
- Documented deviation from vanilla, which asserts `action: "question"` inside the tool and fails
  with `Permission denied: question`. The Rust product's tool-availability filtering already prevents
  disallowed agents from calling the tool, so the execution-time assert is redundant for the current
  agent model.

## Scope / deliverables

- Confirm the existing ruleset/tool-filtering behavior is the intended gate for `question`.
- Add a short test asserting that `question` is offered to `build`/`plan` and withheld from agents
  whose decision is `Deny`.
- Keep this card's decision reflected in `GATE-002`.

## Non-goals

- Reworking the permission system or adding new permission actions.
- Execution-time permission assertion (deliberately not done).

## Acceptance criteria

- [ ] The resolved decision above is recorded and consistent with `GATE-002`.
- [ ] A focused test verifies tool-list availability for allowed vs denied agents.
- [ ] The deviation from vanilla's execution-time assert is documented.

## Verification

- `cargo fmt --all`
- `cargo check -p opencode-permission -p opencode-server`
- `cargo test -p opencode-permission question` and the agentic tool-resolution test.

## Related Items

- `GATE-002` question-tool full parity - parent gate.
- `FEAT-038` question schema and tool contract parity - execution path that intentionally omits the assert.
