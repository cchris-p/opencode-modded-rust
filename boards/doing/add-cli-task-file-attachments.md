---
id: "CLI-012"
title: "Add file attachments to CLI task commands"
priority: "P2"
type: "feature"
area: "CLI"
spec: ""
status: "doing"
predecessors: "CLI-001"
created: "2026-10-14"
updated: "2026-10-14"
---

# Add file attachments to CLI task commands

## Summary

Add `-f`/`--file` attachment support to the headless `opencode task new|send` surface so a file can be
attached to a prompt from the shell, matching the Cline-style convention `CLI-001` was based on
(`copy-cline-style-cli-task-send-conventions.md:44`). This is the "First-class file/image attachment
flags for CLI task commands" split-out that `CLI-001` deferred (`:87`, `:119`).

## Why this exists

`CLI-001` deliberately shipped text-only prompts: the user puts raw file paths in the message and the
agent reads them with its tools (`copy-cline-style-cli-task-send-conventions.md:60,74,87`). That works
for a shared filesystem but breaks the actual headless-attach case, where the target server may be
remote and cannot open the client's local path. There is no way today to send file bytes or a file
reference alongside a CLI prompt. Headless messaging itself already works
(`task new|send|view|status`, `done`); only attachments are missing.

## Current State / Gap

The storage and provider layers already support file parts; the gap was only the prompt API and the
CLI flag wiring.

- **Model supports files:** `PartType::File { url, filename, mime }` exists in the session message
  model (`crates/opencode-session/src/message.rs`), and `PromptInput`/`PartInput` already carry
  `PartInput::File` through `SessionPrompt::create_user_message`/`add_file_part`
  (`crates/opencode-session/src/prompt.rs`), including `data:` and `file://` handling.
- **Gap 1 - server API:** `SessionPromptRequest` only accepted `message`/`command` strings
  (`crates/opencode-server/src/routes.rs`); `accept_prompt` materialized a text-only user message.
- **Gap 2 - CLI:** `TaskCommands` had no `-f`/`--file`; `send_task_prompt` posted `{"message": prompt}`.

## Delivered Design

### Server - parts on `POST /session/{id}/prompt`

- `SessionPromptRequest` gained `parts: Option<Vec<opencode_session::PartInput>>` (reuses the existing
  `PartInput` union: `text`, `file`, `agent`, `subtask`).
- `session_prompt` derives `prompt_text` from `message`/`command` as before, or from text parts when
  only parts are supplied; the explicit parts flow through `accept_prompt_with_parts` into
  `PendingPrompt.parts`.
- `run_prompt_turn` still resolves `@mention` text into parts, then appends explicit non-text parts
  (deduping file URLs already produced by an `@mention`), so the parts reach the turn through the same
  `PromptInput` path and land on the materialized user message at drain.
- The original 8-arg `accept_prompt` is retained as a thin wrapper (`Vec::new()` parts) for the
  `prompt_async` route and existing tests.

### CLI - `-f`/`--file` on `task new` and `task send`

- Repeatable `-f`/`--file <PATH>` (`TaskCommands`, `crates/opencode-cli/src/main.rs`).
- `build_task_file_parts` decides the URL form from the target host:
  - **Loopback** (`127.0.0.1`, `localhost`, `::1`): `file://<abs path>` with mime `text/plain`,
    `application/x-directory`, or an extension-detected type. Zero-copy; the server reads it via the
    normal read path.
  - **Non-loopback (remote)**: inline `data:<mime>;base64,<bytes>` so the server can use bytes it
    cannot otherwise reach.
- Text vs binary is decided by a UTF-8 round-trip (`text/plain`), else a small extension map
  (`png/jpg/jpeg/gif/webp/pdf`), else `application/octet-stream`.
- A 10 MiB cap, missing paths, and remote directories fail with a clear error and exit 1.
- `--file` never influences target resolution; it only adds parts to the prompt sent to the already
  resolved explicit/default target (`invariants/cli-task-targeting.md`).

## Resolved Decisions (open questions from refinement)

- Shared-filesystem detection: **inferred from the target host** (loopback vs not), with no extra flag.
- Size cap: **fixed 10 MiB** (reference parity); no env override in the first pass.
- Prompt API: **generic `Vec<PartInput>`** on `/prompt`, not a file-only subset, so the existing
  `agent`/`subtask` part types are available to future clients.

## Scope

- `SessionPromptRequest.parts`, `accept_prompt_with_parts`, `PendingPrompt.parts`, and the merge in
  `run_prompt_turn`. (Delivered.)
- `-f`/`--file` on `task new`/`task send` plus the URL/mime/cap policy. (Delivered.)
- `wiki/cli-surface.md` and `invariants/cli-task-targeting.md` updated. (Delivered.)

## Non-goals

- `opencode run` / `--mini` file flags and any interactive run surface (`CLI-002`, `GATE-002`).
- First-class image rendering or a TUI attachment UI.
- Implicit server discovery; `--file` must never change target resolution.
- Queued-visible file parts before drain (parts are retained in the queue and applied when the turn
  drains; live badge-time rendering of attachments is out of scope).

## Done when

- A user can attach one or more files to `opencode task new` and `opencode task send` from the shell.
- A loopback target receives a `file://` reference and a remote target receives inline bytes.
- Missing files, oversize files, and remote directories fail visibly with exit code 1.
- `--file` does not alter target selection.
- `wiki/cli-surface.md` documents the flag and its local-vs-remote behavior.

## Recommended verification

- `opencode task new -f ./README.md --server <local-url> "Summarize this file"` -> agent reads the path.
- `opencode task send -f ./artifact.bin --server <remote-url> "What is in this?"` -> bytes inline.
- Oversize, missing, and remote-directory cases exit 1 with a clear message.
- `cargo fmt --all`; `cargo check -p opencode-cli -p opencode-server`; `cargo test -p opencode-cli`;
  `cargo test -p opencode-server`.

## Implementation Notes - 2026-10-14

- Branch: `feature/CLI-012-cli-task-file-attachments` (from `development`).
- Server: `crates/opencode-server/src/routes.rs` - `SessionPromptRequest.parts`,
  `accept_prompt_with_parts`, `PendingPrompt.parts`, `run_prompt_turn` merge.
- CLI: `crates/opencode-cli/src/main.rs` - `-f`/`--file` on `TaskCommands::New`/`Send`;
  `build_task_file_parts`, `server_is_loopback`, `guess_attachment_mime`, `TASK_ATTACH_MAX_BYTES`;
  `send_task_prompt` now sends a `parts` array. `crates/opencode-cli/Cargo.toml` adds `base64`.
- Tests added: `server_is_loopback_classifies_local_and_remote_targets`,
  `guess_attachment_mime_detects_text_and_binary`,
  `build_task_file_parts_uses_file_url_for_loopback`,
  `build_task_file_parts_inlines_data_url_for_remote`,
  `build_task_file_parts_rejects_remote_directory`.
- Verification: `cargo fmt --all`; `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-cli -p
  opencode-server`; `cargo test -p opencode-cli` (21 passed); `cargo test -p opencode-server`
  (71 + 3 passed).

## Related Items

- `CLI-001` Copy Cline-style CLI task send conventions - parent; this is its deferred split-out.
- `CLI-006` Add CLI status visibility for tasks and background sessions - peer CLI surface.
- `CLI-002` Route `opencode run` through the canonical session runtime - owns the deferred interactive
  run surface; not a dependency of this card.
- `CLI-009` CLI and direct-run question parity - peer, `hold`.
- `GATE-001` - queued prompt behavior this card preserves.

## Notes

- On 2026-10-14 the storage layer was confirmed to already expose `PartType::File`; the work is API +
  CLI wiring, not new modeling.
- Reference behavior mirrored from `packages/opencode/src/cli/cmd/run.ts` (`--file`/`--attach`,
  `data:` vs `file://`, 10 MiB cap) at the current reference pin.
