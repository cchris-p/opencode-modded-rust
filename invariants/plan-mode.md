# Plan Mode Invariants

Canonical behavior reference: `wiki/agent-modes-and-custom-agents.md`.

These invariants pin plan mode and its plan file to the reference OpenCode behavior (`GATE-003`).

- The plan file path is session-scoped and timestamped: `<time.created>-<slug>.md`. It resolves through
  one shared helper (`opencode_core::plan_file_path`); callers must not construct the path ad hoc or
  use a fixed `PLAN.md`.
- A VCS project stores its plan under `<worktree>/.opencode/plans/`; a project without VCS stores it
  under the product data `plans/` directory (`~/Library/Application Support/opencode/plans/` on macOS,
  `~/.local/share/opencode/plans/` on Linux).
- Two sessions in the same worktree never share a plan file.
- The `plan` agent may edit only the resolved plan file (and its global-data counterpart when the
  rules reference it); every other `edit` is denied. Build mode keeps full edit access.
- The path advertised in the injected plan-mode reminder is the exact path the plan agent is allowed
  to edit and the path `plan_exit` reports.
- Entering plan mode injects the ported `plan-mode.txt` reminder with the resolved path and
  create-vs-edit wording; the plans directory is created when the file is absent.
- Leaving plan mode injects the `build-switch.txt` reminder, with the "a plan file exists" line when a
  plan file is present.
- `plan_exit` is the only plan-mode model tool. `plan_enter` is a client-driven permission action, not
  a model tool.
