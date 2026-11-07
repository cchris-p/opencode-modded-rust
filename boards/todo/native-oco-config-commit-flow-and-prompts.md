---
id: "CLI-014"
title: "Native oco core: config engine, commit flow, prompts"
priority: "P1"
type: "feature"
area: "CLI"
spec: "docs/opencode-cli.md; ~/standards/handoffs/H-014-ort-native-oco-handoff.md"
status: "todo"
created: "2026-11-07"
updated: "2026-11-07"
---

# Native oco core: config engine, commit flow, prompts

## Summary

Add a native `oco` implementation to the Rust CLI so OpenCommit no longer needs
the Node `opencommit` package or the `opencommit-modded` fork. This card covers
the parity-critical core: config resolution, staged-diff collection, prompt
construction, message generation, confirmation, and `git commit`.

Parity baseline: upstream `di-sukharev/opencommit` **3.2.19** (the version
currently installed on Jasper). Behavior is defined by that source tree, not by
memory.

## Context

- Today `oco` is `opencommit@3.2.19` from npm; it is unreliable and the fork is
  being retired.
- ORT is the right owner because `oco` then ships with every ORT artifact.
- The standards handoff `H-014` is the umbrella plan, including packaging and
  standards cleanup.

## Scope

### Config engine

- Parse `~/.opencommit` as INI (same key names: `OCO_*`).
- Parse a repo-local `.env` in the current working directory.
- Resolve precedence exactly like upstream: real process env > repo `.env` >
  `~/.opencommit` > defaults.
- Normalize/validate known keys, including boolean strings (`true`/`false`),
  `OCO_TOKENS_MAX_INPUT`, `OCO_TOKENS_MAX_OUTPUT`,
  `OCO_MESSAGE_TEMPLATE_PLACEHOLDER`, `OCO_PROMPT_MODULE`, `OCO_AI_PROVIDER`.
- Implement `oco config get|set|describe` in `CLI-014` or split the read-only
  `describe` if it grows; `set` must write the global file in upstream INI
  format.

### Commit flow

- `assertGitRepo`; detect staged files; auto-add when nothing is staged
  (upstream behavior) — but never overwrite user changes.
- Collect a diff for staged changes only; support staged file list and the
  upstream "changed files" fallback.
- Enforce `OCO_TOKENS_MAX_INPUT` with a token estimate and truncate exactly once
  (record the truncation in debug output).
- Generate the message via `CLI-015` provider layer.
- Show the message and ask for confirmation unless `--yes`/`-y`.
- Apply `OCO_MESSAGE_TEMPLATE_PLACEHOLDER` (`$msg`) when a matching argument is
  present; remove that argument before forwarding to git.
- Forward unknown flags/args to `git commit` (for example `--no-verify`,
  `--amend`, `--signoff`).
- Support `-c/--context` and `--fgm` (full GitMoji set).
- Support the optional push prompt controlled by `OCO_GITPUSH`.

### Prompts

- Port the upstream `conventional-commit` prompt behavior, including
  `OCO_DESCRIPTION`, `OCO_EMOJI`, `OCO_EMOJI_POSITION_BEFORE_DESCRIPTION`,
  `OCO_OMIT_SCOPE`, `OCO_ONE_LINE_COMMIT`, and `OCO_LANGUAGE` (v1: language as a
  prompt instruction; full i18n packs deferred).
- Strip prompts/backticks from model output the way upstream does.
- Add an explicit request timeout and clear error text; the old "Generating the
  commit message" hang must become a diagnosable timeout.

## Out Of Scope

- Provider HTTP adapters and `oco models` (`CLI-015`).
- `hook` and `commitlint` subcommands (`CLI-016`).
- `.opencommitignore` support (deferred by owner decision).
- Interactive `oco setup` wizard (deferred; rely on config/env and fail clearly).
- GitHub Action.

## Acceptance Criteria

- [ ] `cargo test -p <oco crate>` covers config precedence, template
      substitution, unknown-arg passthrough, and prompt selection.
- [ ] In a temp git repo with a mock provider, `oco --yes` creates one commit
      with the generated message and does not create extra commits.
- [ ] `oco -c "context"` changes the prompt payload; `oco '#205: $msg'` becomes
      the final commit message prefix/suffix.
- [ ] `oco --no-verify` reaches `git commit` (verified by a failing pre-commit
      hook that is bypassed).
- [ ] A missing/unreachable provider fails with a bounded, actionable error and
      non-zero exit, never an indefinite hang.
- [ ] `oco config get`, `set`, and `describe` round-trip through
      `~/.opencommit`.

## Related

- `CLI-015` — providers, models, prompt shaping.
- `CLI-016` — hook and commitlint.
- `INFRA-016` — ship `oco` in the ORT artifact.
- Standards `H-014` / `STD-155` — umbrella handoff and fork retirement.
