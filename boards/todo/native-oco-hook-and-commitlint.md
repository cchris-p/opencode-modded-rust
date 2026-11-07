---
id: "CLI-016"
title: "Native oco hook and commitlint subcommands"
priority: "P1"
type: "feature"
area: "CLI"
spec: "docs/opencode-cli.md; ~/standards/handoffs/H-014-ort-native-oco-handoff.md"
status: "todo"
created: "2026-11-07"
updated: "2026-11-07"
predecessor: "CLI-014"
---

# Native oco hook and commitlint subcommands

## Summary

Port the two remaining OpenCommit subcommands that the owner selected for parity:
the `prepare-commit-msg` git hook and the `@commitlint` prompt module.

## Hook Scope

- `oco hook set` installs a `prepare-commit-msg` hook in the current repo.
- `oco hook unset` removes only the OpenCommit-managed hook; never delete an
  unrelated hook.
- Respect `core.hooksPath` when resolving the hook directory.
- When invoked as a hook (git passes the commit-message file path), read the
  message file, generate a replacement, and write it back.
- Honor `OCO_HOOK_AUTO_UNCOMMENT`.
- The hook must call a stable executable path. It should not depend on an
  interactive shell function; it may call `ort oco` or the installed `oco`
  entry point.
- Idempotency: setting twice must not duplicate hook content; unsetting an
  absent hook is a no-op with a clear message.

## Commitlint Scope

- `oco commitlint force` writes/refreshes the local commitlint integration.
- `oco commitlint get` prints the generated config.
- Create `.opencommit-commitlint` from the local commitlint config, matching
  upstream's `@commitlint` prompt module behavior.
- `OCO_PROMPT_MODULE=@commitlint` selects the commitlint prompt path in
  `CLI-014`; this card supplies the generated prompt file.
- If no local commitlint config exists, fail with the same actionable guidance
  upstream gives.

## Acceptance Criteria

- [ ] In a temp repo, `oco hook set` then `git commit` generates and commits a
      message without an editor; `oco hook unset` restores normal behavior.
- [ ] A pre-existing `prepare-commit-msg` hook is preserved and reported.
- [ ] `core.hooksPath` is honored in a test repo.
- [ ] `oco commitlint force` creates `.opencommit-commitlint`; `get` prints it.
- [ ] `OCO_PROMPT_MODULE=@commitlint` uses the generated file and produces a
      commitlint-shaped message in a snapshot test.

## Related

- `CLI-014` — core config/commit/prompt engine.
- `CLI-015` — providers.
- `INFRA-016` — artifact packaging.
- Standards `H-014` / `STD-155`.
