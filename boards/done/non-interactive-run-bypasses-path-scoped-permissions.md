---
id: "BUG-066"
title: "Non-interactive run bypasses path-scoped permission rules"
priority: "P1"
type: "bug"
area: "BUG"
spec: "docs/opencode-permission.md"
status: "done"
created: "2026-11-04"
---

# Non-interactive run bypasses path-scoped permission rules

## Summary

`opencode-modded-rust run` (and the GitHub run path) executes tool calls without
an ask callback. `AgentExecutor` builds each `ToolContext` with no `.with_ask`,
so only the coarse `ensure_tool_allowed(tool_name)` check runs. That check
evaluates the agent ruleset with pattern `"*"`, which cannot match path- or
command-scoped rules such as `edit: { "/path/app/*": deny }` or
`bash: { "git push*": deny }`.

Result: a non-coding agent whose rules deny code writes and destructive shell
commands can still write code and run those commands in plain headless `run`.
The same agent is correctly denied through the ORT TUI/server path, which wires
a permission callback.

## Evidence

Reproduced on mb1 with the Ametrine `ametrine-chief-financial-officer` agent,
which has path-scoped `edit` denies and command-scoped `bash` denies:

```bash
cd ~/apps/ametrine-site

# Plain run: writes the file despite app/ being denied.
opencode-modded-rust run --agent ametrine-chief-financial-officer \
  "Use the write tool to create app/permission-test.txt with DENIED."

# Result: "Successfully wrote 7 bytes ... app/permission-test.txt"

# Server prompt path with the same agent: denied as expected.
curl -s -X POST http://127.0.0.1:4099/session/<id>/prompt \
  -H 'content-type: application/json' \
  -d '{"agent":"ametrine-chief-financial-officer","message":"..."}'
```

The server run returned `Permission denied: Tool 'bash' is denied by agent
permission rules`; the plain run did not.

## Root cause

- `crates/opencode-agent/src/executor.rs`:
  - `execute_tool` and `execute_tool_without_subsessions` construct
    `ToolContext` without `.with_ask(...)`.
  - `ensure_tool_allowed` calls `AgentInfo::tool_permission_decision(tool_name)`,
    which calls `PermissionNext::evaluate(agent, tool_name)` with pattern `"*"`.
- Path- and command-scoped rules are only evaluated when a tool emits a
  `PermissionRequest` and the host supplies an ask callback.
- `ToolContext::ask_permission` returns `Ok(())` when no callback is set, so the
  request is silently allowed.

## Fix

- Give `AgentExecutor` a built-in fail-closed ask callback over its agent
  ruleset.
- Evaluate every requested pattern with `opencode_permission::evaluate`; take the
  strictest action across patterns (`Deny` > `Ask` > `Allow`).
- In non-interactive execution, `Ask` fails closed with a clear error because
  there is no user to approve it.
- Wire the callback into both tool-execution paths and carry it into subagent
  executors.
- Keep the TUI/server callback behavior unchanged.

## Done when

- Plain `run` denies a path-scoped `edit` and a command-scoped `bash` request
  instead of executing them.
- `Ask` rules fail closed in non-interactive execution.
- Unit tests cover deny, allow, strictest-across-patterns, and ask-fails-closed.
- `cargo test -p opencode-agent` and `cargo check --workspace` pass.
- The TUI/server permission path still allows and denies the same cases.

## Implementation

- `crates/opencode-agent/src/executor.rs`:
  - added `ask_callback` to `AgentExecutor`, built from the agent ruleset;
  - `noninteractive_permission_callback` evaluates every requested pattern with
    `opencode_permission::evaluate`, takes the strictest action, allows
    `allow`, rejects `deny`, and fails closed on `ask`;
  - both `execute_tool` and `execute_tool_without_subsessions` attach the
    callback to the `ToolContext`;
  - subagent executors build their own callback from the child agent rules.
- `docs/opencode-permission.md` documents the non-interactive behavior.
- `invariants/qa/README.md` records the focused checks and the ScopeMux build
  requirement.

## Verification (self-QA, 2026-11-04)

Commands and results:

```text
cargo test -p opencode-agent
  -> 22 passed, 0 failed

SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-cli
  -> Finished dev profile

SCOPEMUX_PREBUILT_DIR=$HOME/.cache/opencode-modded-rust/native/linux-x86_64-2a642c10534f \
  cargo build -p opencode-cli --bin opencode-modded-rust
  -> Finished dev profile
```

Runtime smoke against the Ametrine `ametrine-chief-financial-officer` agent in
`~/apps/ametrine-site` after rebuilding the binary:

- Plain run, bash redirection:
  `Permission denied: Tool 'bash' is denied by agent permission rules`;
  `/tmp/ametrine-denied-test.txt` was not created.
- Plain run, path-scoped edit `boards/permission-test.yaml`:
  `Permission denied: Tool 'edit' is denied by agent permission rules`;
  no file created.
- Plain run, allowed edit `docs/ametrine-permission-test.md`: write succeeded;
  test artifact removed.
- TUI/server path is unchanged: it uses its own session callback and does not
  construct `AgentExecutor`.

QA profile: standards `self-qa` bootstrap (no general project-local QA skill
exists; `invariants/qa/README.md` was added with the evidence-backed rules).

## Disposition

Closed 2026-11-04 by direct closeout after self-QA, per owner direction.

## Related Items

- `BUG-048` - bash permission arity panic in the same permission path.
- `START-026` - permission allow/deny config determinism.
- Ametrine `AMETRINE-010` - the corporate-agent deployment that exposed this.
- `docs/opencode-permission.md` - permission layer contract.