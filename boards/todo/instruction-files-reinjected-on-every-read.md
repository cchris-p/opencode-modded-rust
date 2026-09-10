---
id: "BUG-009"
title: "Instruction files (AGENTS.md) re-injected on every read tool call"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-10"
---

# Instruction files (AGENTS.md) re-injected on every read tool call

## Summary

Every `read` tool call appends the applicable instruction files (`AGENTS.md`, etc.) to its result with no session-level deduplication. On a real session this repeated the entire `AGENTS.md` after each of several reads, wasting a large amount of context on identical content.

## Reported behavior

From session `ses_0ffc9b44f96944039f8621c4d980d11b` (exported `summarize-workspace-files.md`):

- Each `read` result ended with a `<system-reminder>` containing the full `AGENTS.md` text.
- The same `AGENTS.md` block appeared after roughly six separate reads in one short session, plus repeated instruction content in other tool output.

## Root cause (evidence)

`crates/opencode-tool/src/read.rs`:

- `resolve_instruction_prompts(...)` walks from the read file up to the project root and returns nearby instruction files (`read.rs:479-529`).
- The read result appends all of them under `<system-reminder>` unconditionally (`read.rs:425-437`).
- There is no dedup keyed on files already attached in this session/message, and the tool context does not receive the conversation's prior `loaded` metadata.

Reference behavior (`packages/opencode/src/tool/read.ts` + `session/instruction.ts`): the read tool tracks a `loaded` list in result metadata and `Instruction.resolve(messages, filepath, messageID)` skips instruction files already present in the message history, so instructions are attached once, not per read.

## Scope

- Deduplicate instruction injection across reads within a session/message: attach each instruction file at most once (or only when its content changed), mirroring the reference `loaded`-tracking approach.
- **Decision (2026-09-10): track loaded instruction files in session state, keyed by session.** The session prompt loop already builds the tool `ToolContext`; add a shared loaded-instruction set (e.g. an `Arc<Mutex<HashSet<String>>>` or session metadata) to `ToolContext`, populate it from each read's resolved instruction file paths, and have `read.rs` skip files already in the set. This matches the reference `Instruction.resolve(messages, ...)` semantics while fitting the existing Rust tool-context plumbing. Do not rely on re-parsing the transcript for dedup.
- The read tool records the instruction file paths it attaches (already surfaced as `loaded` metadata in `read.rs:423-447`) so the session can accumulate them.
- Preserve the legitimate first-time injection behavior; do not drop instructions entirely.

## Non-goals

- Reworking the broader instruction-loading/glob/URL pipeline.
- Changing the system-prompt instruction assembly (this is specifically the per-read tool injection).

## Done when

- A multi-read session attaches each instruction file once, not once per read.
- A regression test covers multiple reads under one instruction file and asserts a single injection.
- Instruction content that changes between reads is still surfaced.

## Recommended verification

- `cargo test -p opencode-tool read`
- Live: fresh server, read several files under this repo and confirm `AGENTS.md` appears once, not per read.

## Related Items

- `BUG-004` Coding sessions run as bare chat
- `PHASE-001` V1 daily-driver hardening

## Notes

- Surfaced during BUG-006 QA; see `summarize-workspace-files.md`.
- This is a context-efficiency/token-budget issue, not a functional failure; classify accordingly if scope is questioned.
