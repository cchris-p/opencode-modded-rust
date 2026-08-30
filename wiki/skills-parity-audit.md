# Skills Parity Audit

This document audits the current Rust `skills` feature against the frozen TypeScript/OpenCode reference line at `$HOME/repos/opencode-modded` commit `e62912b5d18b73316c7bfd6e894b040698f6c880`.

## Purpose

- capture what is already implemented in this repo
- identify concrete parity gaps against the reference behavior
- give `SKILLS-001` an evidence-backed implementation target

## Current Rust behavior

Confirmed in this repo today:

- Local filesystem skill discovery exists.
- Discoverable skills are `SKILL.md` files with `name` and `description` frontmatter.
- Discovery searches built-in home, config, and workspace-local roots plus configured `skills.paths` entries.
- Duplicate skill names collapse to one surviving definition.
- The `skill` tool permission-gates loading and returns skill content, base directory guidance, sampled files, and metadata.
- CLI debug listing exists through `opencode debug skill`.
- Session compaction protects `skill` tool calls.

## Reference behavior confirmed

Confirmed from the frozen reference line:

- Skills are folder-based and transferable via `SKILL.md` plus relative resources.
- Project, global, Claude-compatible, and agent-compatible roots are part of the supported contract.
- Duplicate names resolve by later source precedence.
- The skill tool returns structured skill content and relative-path guidance.
- A real server listing endpoint exists.
- TUI skill browsing uses server-provided skill data and includes descriptions.
- Discovered skills are also exposed as user-facing slash commands in the reference command layer.

## Parity gaps

The most important current gaps are:

1. Project discovery breadth

- Closed by `SKILLS-001`: Rust project skill discovery now walks upward from the active directory to the git worktree root for `.opencode`, `.claude`, and `.agents` skill roots.

2. Server-backed listing

- Closed by `SKILLS-001`: the Rust server route `GET /skill` now returns the discovered local skills set with names and descriptions.
- The TUI refresh path now consumes that server list instead of depending on a stubbed empty response.

3. TUI parity

- Closed by `SKILLS-001` for the current local-filesystem scope: the Rust skills dialog now consumes skill names plus descriptions and filters on both fields.

4. Config compatibility

- Rust supports `skills.paths` in active discovery.
- Rust also defines `skills.urls` in schema, but does not use it at runtime.
- The reference line supports URL-backed skill sources and also contains a newer flat `skills` source-list shape.

5. User-facing command parity

- The Rust repo exposes a static `/skills` browser command.
- Prompt skill autocomplete now refreshes from the same discovered server-backed skill list as the TUI browser.

6. Tool argument compatibility

- Closed by `SKILLS-001`: the Rust skill tool now accepts the reference-compatible `name` key as an alias for `skill_name`.

7. Test coverage

- Improved by `SKILLS-001`: Rust tests now cover upward project discovery, duplicate precedence for nearer project roots, reference-compatible tool argument naming, and TUI-side filtering by skill metadata.

## Observed mismatches that should be handled carefully

- Reference docs describe some stricter rules than the code appears to enforce, especially around required descriptions and directory-name matching.
- Both legacy and newer config shapes exist in the reference tree.

These should be treated as explicit alignment decisions during implementation rather than assumed truths.

## Recommended scope for SKILLS-001

`SKILLS-001` should establish parity for the transferable local skills contract first:

- local/project/global discovery behavior
- duplicate resolution behavior
- server and TUI listing parity for discovered skills
- runtime load behavior and metadata parity
- compatibility with representative real skills from the reference repo
- focused tests for discovery, loading, and listing

Remote URL-backed skills and broader post-parity expansion may need follow-up items if they are too large for the first parity pass.
