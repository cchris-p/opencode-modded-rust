# Skills Discovery Invariants

- A discoverable skill is a filesystem directory tree entry whose defining file is named `SKILL.md`.
- A valid `SKILL.md` must begin with frontmatter and include a `name` field; `description` is optional and must not determine whether the skill name is discoverable.
- Skill body content is the markdown that follows the closing frontmatter delimiter.
- Skill discovery is filesystem-first; the runtime scans known local roots rather than fetching remote skill definitions.
- For local filesystem-backed skills, Rust discovery must produce the exact same discovered skill name list as vanilla OpenCode when both run against the same global and project-local environment.
- Skill roots may come from built-in home/config/project locations and from explicit `skills.paths` entries in config.
- Global OpenCode config roots include vanilla-compatible `~/.config/opencode/{skill,skills}` even on macOS, plus the platform config directory roots used by Rust.
- Project-local roots are discovered from the active directory upward to the git worktree root so ancestor `.opencode`, `.claude`, and `.agents` skill directories remain visible from nested working directories.
- Relative configured skill paths resolve from the active workspace base directory; `~/` paths resolve from the user home directory.
- Skill identity is keyed by the frontmatter `name`, not by directory name or file path.
- When duplicate skill names are discovered, only one definition survives in the discovered set, with later and more local sources overriding earlier ones.
- The discovered skill set is returned in sorted name order.
- `skills.urls` exists in config schema but is not part of active runtime discovery in the current implementation.
- Both legacy `skills.paths`/`skills.urls` config and the current vanilla flat `skills` source-list shape are accepted; URL entries remain excluded from active runtime discovery until URL-backed skills are implemented.
