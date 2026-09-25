---
name: scopemux-self-qa
description: Agent-driven QA for this product when a board item reaches QA or the user asks to QA, verify, self-QA, "who does QA", or confirm a change works. Covers the attach/detach server harness, the headless QA check scripts, the retrieval-provider signal, and the scopemux provider activation scope. Use ONLY when QA or verification of this product is the task; the agent performs QA itself and must not hand QA back to the user.
license: MIT
compatibility: opencode
metadata:
  audience: maintainers
  workflow: self-qa
---

# Scopemux Self-QA

## Policy: the agent does QA, not the human

The human no longer performs QA for this product. Do not ask the user to
"test this", attach files, click through the TUI, or confirm behavior.

- When a board item reaches a QA/verification step, or the user asks to QA or
  verify a change, the agent runs QA end to end and records evidence on the
  card (or in the handoff for cross-repo work).
- "QA" means: build the artifact, attach a server, drive the real code path
  headlessly, capture a deterministic signal, detach, and report pass/fail.
- Only escalate to the human when a check is genuinely impossible to run
  (missing credentials, missing hardware) — say exactly what is blocked and
  why, not "please test this".
- Never fabricate QA. A green unit test is not QA of the integrated behavior;
  drive the product path and observe it.

## The attach/detach server harness

The convention is: **attach** a locally built server, drive it, **detach** it.

- `scripts/scopemux-qa-server.sh` — attach. Sources the shared standards
  aliases (`ollama-start-0`, `opencode-use-ollama-local`), pins the QA
  workspace model to `ollama/qwen3:30b`, sets `RUST_LOG=info`, and serves the
  product binary. Accepts `--binary`, `--workspace`, `--port`
  (env `SCOPEMUX_QA_*`).
- `scripts/scopemux-qa-check.sh` — detach-style check. Creates a session,
  sends a file-seeded prompt, reads the effective retrieval provider from the
  server log, aborts the turn, and exits non-zero on failure.

QA server and server output live on `127.0.0.1`; the default port is `4096`
and the default workspace is `$HOME/worktrees/scopemux-qa` (sample C and Rust
files). Check the server log path the server helper was started with (commonly
`/tmp/scopemux-qa-server.log`).

### Why a server, not the TUI

A server can be driven headlessly and repeatedly with `curl`, so QA is
deterministic and recorded. Never rely on the interactive TUI for QA, and do
not use `ort` to QA a feature branch: `ort` builds the main checkout and
unsets `OPENCODE_CONFIG_CONTENT`. Use the branch's built binary, or the
worktree shared-target binary
(`$OPENCODE_WORKTREE_ROOT/opencode-modded-rust/.shared-target/debug/opencode`).

## Procedure for any board item

1. **Identify the artifact.** Check out the branch to test; build it
   (`ort-build`, or `SCOPEMUX_CORE_DIR=<core> cargo build -p opencode-cli`).
   Note the commit and binary path.
2. **Attach.** Start the server helper against the workspace under test.
   Confirm `GET /config` resolves the intended model.
3. **Drive the real path.** Use the HTTP API (see below) to exercise the
   feature. Reproduce the exact user-facing flow, not a proxy.
4. **Capture a deterministic signal.** Read it from a log line, an API field,
   or the SQLite store. Strip ANSI before parsing. Do not eyeball the TUI.
5. **Detach.** Stop the running turn and/or the server; leave the checkout as
   you found it.
6. **Record evidence on the board item**: commit, binary, command(s), observed
   signal, pass/fail, log path. Move the card only on real evidence.

## Headless request pattern

Retrieval and other prompt-time behavior only run when the prompt carries
**file-seed parts**, which come from `@relative/path` mentions resolved against
the session directory. Plain text does not trigger retrieval.

```sh
BASE=http://127.0.0.1:4096
DIR=$HOME/worktrees/scopemux-qa
# create a session rooted at the workspace
SID=$(curl -s -X POST "$BASE/session?directory=$DIR" \
        -H 'content-type: application/json' -d '{}' | jq -r .id)
# enqueue a prompt; returns immediately, does not block on the model
curl -s -X POST "$BASE/session/$SID/prompt_async" \
  -H 'content-type: application/json' \
  -d '{"message":"QA check on @sample.rs"}' >/dev/null
# stop the turn when done
curl -s -X POST "$BASE/session/$SID/prompt/abort" >/dev/null
```

The effective retrieval provider is logged by
`crates/opencode-session/src/prompt.rs`:

```
INFO opencode_session::prompt: retrieval provider selected provider=scopemux candidates=N
```

`scripts/scopemux-qa-check.sh` wraps this pattern; prefer it, and extend it
rather than re-implementing curl flow inline.

## Scopemux provider activation scope

The native provider is scoped to the **local Qwen-via-Ollama** model
(`invariants/providers.md`). QA must assert both directions:

- `ollama` + a `qwen*` model id + a supported file seed → `provider=scopemux`
  with `candidates > 0`.
- any other model → `provider=generic`.

The provider can be selected but still return zero candidates when
scopemux-core fails to parse or index the workspace — treat
`provider=scopemux candidates=0` as a failure, not a pass.

## Extending the harness for other QA

The harness generalizes: attach a server, create a session rooted at a
fixture workspace, drive an endpoint, read evidence, detach. Add a dedicated
`scripts/scopemux-qa-<subject>.sh` check with the same shape (create session →
drive → extract signal → abort → non-zero exit on failure) and record its
signal source. Keep signals in logs or API fields; do not depend on the
interactive TUI.

## Pitfalls

- **No file seed, no retrieval.** `@file` mentions are required; text-only
  prompts never exercise retrieval.
- **Missing signal.** Set `RUST_LOG=info` when attaching, or the provider
  line will not appear.
- **ANSI in logs.** Strip escapes (`perl -pe 's/\e\[[0-9;]*[A-Za-z]//g'`)
  before grepping/parsing values.
- **Duplicate tree-sitter runtime.** scopemux-core vendors tree-sitter and the
  product links the `tree-sitter` crate; version/ABI mismatch breaks
  `ts_parser_set_language`, and duplicate runtimes can break the core search
  index even when ABIs match. If the provider falls back, check for
  `duplicate symbol` linker warnings and `ts_parser_set_language failed`.
- **The server is unsecured** (no `OPENCODE_SERVER_PASSWORD`) and binds
  `127.0.0.1`; do not expose it.
- **Shared checkout.** Other sessions may hold the main checkout on another
  branch. Prefer a git worktree and the shared-target binary for QA, and do
  not disturb unrelated uncommitted work.

## Evidence template

```
QA: <board item id> — <what was verified>
commit: <sha>   binary: <path>   server: http://127.0.0.1:4096
command: <script or curl>
observed: provider=<...> candidates=<...> (log: <path>)
result: PASS|FAIL (reason)
```
