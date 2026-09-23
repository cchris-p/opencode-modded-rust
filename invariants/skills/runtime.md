# Skills Runtime Invariants

- The runtime exposes skills through the `skill` tool as a permission-gated context-loading operation.
- Executing the `skill` tool requires a known discovered skill name; unknown names are rejected at argument validation time.
- Loading a skill returns the skill markdown content together with the skill base directory.
- Relative resource references inside a skill are interpreted from the directory that contains that skill's `SKILL.md`.
- Skill loads include a sampled file listing from the skill directory tree in addition to the main `SKILL.md` content.
- Skill metadata emitted by the tool includes the skill name, base directory, and source file location.
- Session compaction must preserve `skill` tool calls as protected tool history.
- CLI debug listing for skills is backed by the same local discovery path as the `skill` tool.
- The server `/skill` endpoint is an authoritative discovery surface for local skills and returns the same discovered skill names and descriptions the runtime can load.
- TUI skill browsing and prompt autocomplete must refresh from the server `/skill` endpoint so user-facing selection surfaces match runtime-discoverable skills.
- The `skill` tool accepts both `skill_name` and the reference-compatible `name` input key for selecting a discovered skill.
- The `skill` tool exposes the skill name as a free-form string rather than enumerating discovered names in its JSON schema.
- The `skill` tool description directs the model to the skills listed in the system prompt.
- The runtime appends an available-skills block to the system prompt on every model call, matching the reference `SystemPrompt.skills(agent)` shape: a two-line preamble followed by verbose `<available_skills>` XML carrying `name`, `description`, and `location` per skill.
- The available-skills block is built from the session workspace and filtered by the agent `skill` permission: skills without a description and skills denied by name are omitted, and the block is omitted entirely when the agent blanket-denies the `skill` permission.
