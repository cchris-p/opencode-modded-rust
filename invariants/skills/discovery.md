# Skills Discovery Invariants

- A discoverable skill is a filesystem directory tree entry whose defining file is named `SKILL.md`.
- A valid `SKILL.md` must begin with frontmatter and include both `name` and `description` fields.
- Skill body content is the markdown that follows the closing frontmatter delimiter.
- Skill discovery is filesystem-first; the runtime scans known local roots rather than fetching remote skill definitions.
- Skill roots may come from built-in home/config/project locations and from explicit `skills.paths` entries in config.
- Project-local roots are discovered from the active directory upward to the git worktree root so ancestor `.opencode`, `.claude`, and `.agents` skill directories remain visible from nested working directories.
- Relative configured skill paths resolve from the active workspace base directory; `~/` paths resolve from the user home directory.
- Skill identity is keyed by the frontmatter `name`, not by directory name or file path.
- When duplicate skill names are discovered, only one definition survives in the discovered set, with later and more local sources overriding earlier ones.
- The discovered skill set is returned in sorted name order.
- `skills.urls` exists in config schema but is not part of active runtime discovery in the current implementation.
