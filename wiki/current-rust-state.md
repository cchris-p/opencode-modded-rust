# Current Rust State Assessment

## Purpose

Assess whether the current Rust codebase is a strong foundation for the V1 product goal in `wiki/v1.md`, and distinguish what should be treated as product-core foundation versus reference-only carryover.

## Assessment Basis

- V1 target: `wiki/v1.md`
- Product boundary: `wiki/product-boundary.md`
- Reference strategy: `wiki/reference-strategy.md`
- Runtime and task invariants: `invariants/runtime-lifecycle.md`, `invariants/task-state.md`, `invariants/context.md`, `invariants/verification.md`
- Reference context only: `$HOME/repos/opencode-modded` at commit `e62912b5d18b73316c7bfd6e894b040698f6c880`

## Conclusion

The current Rust repo is a strong implementation base, but it is not yet a V1-ready architecture.

The code already contains a substantial TUI, session, storage, provider, and tool stack that is worth preserving. The main weakness is architectural: the runtime still behaves like a broad chat-and-tool loop, while V1 requires explicit runtime-owned task stages, durable task state outside transcript history, and verification before completion.

## Strong Foundations Worth Preserving

### TUI foundation

The product already has a serious interactive terminal surface with session and model workflows.

- Entry and bootstrap: `crates/opencode-tui/src/lib.rs`
- Main app flow: `crates/opencode-tui/src/app/app.rs`
- TUI launch from CLI/server bootstrap: `crates/opencode-cli/src/main.rs`

This is aligned with the V1 requirement that the TUI remain the main interface.

### Durable session and storage foundation

The repo already persists session state through the storage layer and has working session continuity concepts.

- Session domain: `crates/opencode-session/src/session.rs`
- Database bootstrap: `crates/opencode-storage/src/database.rs`
- Persistent schema: `crates/opencode-storage/src/schema.rs`

This is a strong base for V1 session continuity, but it is still more session-centric than task-centric.

### Core repository toolchain

The repo already has the minimum tool surface needed for the narrow coding workflow.

- Tool registry: `crates/opencode-tool/src/registry.rs`
- File read: `crates/opencode-tool/src/read.rs`
- File edit: `crates/opencode-tool/src/edit/tool.rs`
- File write: `crates/opencode-tool/src/write.rs`
- Shell execution: `crates/opencode-tool/src/bash.rs`

This part already matches the V1 requirement for repository reads, file editing, and shell execution.

### Provider abstraction and model streaming

The provider layer is already substantial and reusable.

- Provider surface: `crates/opencode-provider/src/lib.rs`
- Bootstrap catalog: `crates/opencode-provider/src/bootstrap.rs`
- OpenAI-compatible adapter path: `crates/opencode-provider/src/openai.rs`

This gives V1 a useful base, but the current product shape is not clearly local-model-first yet.

## Major Gaps Relative to V1

### 1. Runtime ownership is still a chat loop, not explicit task stages

V1 requires runtime-owned lifecycle stages and explicit verification before completion.

- Current runtime loop: `crates/opencode-session/src/prompt.rs`
- Required lifecycle invariant: `invariants/runtime-lifecycle.md`
- Existing planning card: `boards/todo/define-v1-runtime-loop.md`

The current loop is good reference implementation material, but not yet the V1 architecture of record.

### 2. Structured task state is too thin

The repo has durable sessions and a todo list, but it does not yet model a bounded task with authoritative objective, criteria, stage, and completion state.

- Task-state invariant: `invariants/task-state.md`
- Current todo model: `crates/opencode-session/src/todo.rs`
- Current persistent schema: `crates/opencode-storage/src/schema.rs`

This is a direct gap against V1's requirement for durable task state outside transcript history.

### 3. Review and verification are not enforced by the runtime

V1 requires review and verification as separate stages, and completion only after explicit verification.

- Verification invariant: `invariants/verification.md`
- Runtime completion path: `crates/opencode-session/src/session.rs`
- Current review/test command docs: `crates/opencode-command/commands/review.md`, `crates/opencode-command/commands/test.md`

The current repo has helpful workflow pieces, but they are not yet strong runtime gates.

### 4. TUI approval and question handling are incomplete

The TUI still contains visible placeholders in the approval flow.

- Pending approval/question UI work: `crates/opencode-tui/src/app/app.rs`
- Missing or incomplete client support: `crates/opencode-tui/src/api.rs`

That makes the TUI foundation real, but not complete enough for the V1 workflow loop.

### 5. Context construction is not yet a dedicated bounded-task subsystem

The runtime currently assembles prompt context primarily from transcript and instruction state. That is useful, but it is not yet the focused task-context construction model described by the V1 docs and invariants.

- Prompt assembly path: `crates/opencode-session/src/prompt.rs`
- Instruction loading: `crates/opencode-session/src/instruction.rs`
- Context invariant: `invariants/context.md`
- Deferred retrieval planning: `boards/todo/plan-scopemux-integration.md`

This is partly planned already, but it is not yet implemented as a first-class runtime capability.

### 6. The visible product surface is broader than V1 needs

The current CLI and server expose a much wider feature set than the immediate V1 target.

- Broad CLI surface: `crates/opencode-cli/src/main.rs`
- Broad server route surface: `crates/opencode-server/src/routes.rs`
- Non-goal surfaces still present in the repo: `crates/opencode-plugin/src/lib.rs`, `crates/opencode-mcp/src/lib.rs`

These areas are not automatically wrong, but they should mostly be treated as reference-only or de-emphasized until the V1 core loop is solid.

## Subsystem Disposition

### Preserve for V1

- TUI shell and interaction foundation
- Durable session and storage infrastructure
- Core file and shell tools
- Provider abstraction and streaming core

### Preserve, but reshape

- Session runtime loop
- Task/todo persistence model
- Context construction path
- Completion and verification flow

### Treat mostly as reference-only for now

- Broad parity-oriented CLI/server surfaces outside the narrow workflow
- Plugin and MCP breadth not required for V1
- Other non-core compatibility surfaces that do not directly improve the daily-driver loop

## Reference-Only Comparison Notes

The TypeScript reference repo is still useful for workflow ideas and implementation patterns, but it does not change product policy in this repo.

Relevant reference observations:

- It separates current implementation behavior from future architecture planning in `specs/v2/`.
- It has layered context assembly in `packages/opencode/src/session/system.ts` and `packages/opencode/src/session/instruction.ts`.
- It models task execution and resumability through the task tool in `packages/opencode/src/tool/task.ts`.
- It treats review as a first-class product concept in `packages/app/src/pages/session/review-tab.tsx`.

Those observations are useful as design context only. This Rust repo remains the authority for the delivered V1 architecture.

## Follow-Up Map

Existing follow-up items already covering part of the gap:

- `START-005` Define V1 runtime loop
- `START-007` Plan ScopeMux integration

New follow-up items created from this assessment:

- `START-016` Define structured task state for V1
- `START-017` Enforce verification and review stages in runtime
- `START-018` Complete TUI approval and question handling
- `START-019` Define local-model-first provider path
- `START-020` Constrain primary product surface to the V1 workflow

## Bottom Line

The Rust repo should not be treated as reference-only. It is already the right product foundation for V1.

What should be treated as reference-only are the broad parity surfaces and the current chat-loop architecture where they conflict with the narrower V1 direction. The right path is to preserve the working TUI/session/tool/provider base, then reshape the runtime around explicit task stages, durable task state, and verification-driven completion.
