- The default branch in this repo is whatever the repo currently uses locally; verify before branching or diffing.

## Repository Role

- This repository is the main product.
- `$HOME/repos/opencode-modded` is a reference and planning repo, not the product core.

## Cross-Repo Documentation Boundary

- This repository owns its own `docs/`, `wiki/`, `invariants/`, and `boards/` artifacts.
- Files in `$HOME/repos/opencode-modded` may be referenced by explicit path for context, comparison, or planning history.
- Reference by path, not by inheritance: documents in `$HOME/repos/opencode-modded` are not automatically authoritative in this repo.
- If a rule, invariant, or plan must constrain this product, it must be restated in this repo.
- Cross-repo references are for context and traceability, not shared ownership.

## Reference Repo Convention

- The vanilla/reference codebase is `$HOME/repos/opencode-modded`, a mirror of `anomalyco/opencode` (`https://github.com/cchris-p/opencode-modded`).
- Reference vanilla behavior from the `dev` branch, and fetch the latest `dev` before comparing (`git -C "$HOME/repos/opencode-modded" fetch origin dev`). Do not compare against a stale local checkout or an old pinned commit.
- The recorded reference commit is pinned for reproducibility: `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (branch `dev`, package `1.18.31`, 2026-09-21), re-pinned by `GATE-004`.
- The previous pin `e62912b5d18b73316c7bfd6e894b040698f6c880` is unreachable on the remote (force-pushed) and is superseded; do not cite it as current.

## Planning Direction

- `wiki/` contains architecture and version-direction documents.
- `invariants/` contains absolute truths for the final desired system.
- `boards/` tracks execution work, deferred work, and future feature triage.
- Keep board lane directories flat: do not create subdirectories under `boards/<lane>/`; place board item markdown files directly in the lane directory.
- `docs/archive/session.md` is archived brainstorming source material, not the authoritative plan.
- The TypeScript reference line and the `dev`-branch convention are defined under "Reference Repo Convention" above.

## Current Product Stance

- Personal daily-driver on a narrow workflow is the immediate target.
- TUI is part of V1.
- Full parity with OpenCode is deferred.
- Upstream sync is optional and selective, not a standing maintenance obligation.

## Storage Paths

- The Rust product resolves its SQLite database as `dirs::data_local_dir()/opencode/opencode.db` (`crates/opencode-storage/src/database.rs:138`).
- macOS: `~/Library/Application Support/opencode/opencode.db`.
- Linux: `~/.local/share/opencode/opencode.db` (or `$XDG_DATA_HOME/opencode/opencode.db`).
- This is the Rust product's database only. Do not confuse it with vanilla OpenCode, whose `xdg-basedir` storage resolves to `~/.local/share/opencode/opencode.db` even on macOS.

## Local Launchers

- `ort-build` builds the Rust TUI/CLI binary from `$HOME/repos/opencode-modded-rust`.
- `ort` launches the most recently built Rust TUI binary without rebuilding first.
- `ort` runs the TUI in the directory it was activated from, so that directory is the workspace (config search root and displayed working directory).
- `ort` always starts a fresh local TUI server for that workspace. It never reuses, rotates, or attaches to a previously recorded server, so a stale or other-workspace server can never serve the TUI. Use `opencode attach <url>` for intentional re-attachment.
- The Rust product owns its default model (`deepseek/deepseek-v4-flash`); the shared vanilla `~/.config/opencode/opencode.json` model does not dictate this product's default, and a workspace `opencode.json{,c}` still overrides it.
- When testing code changes, run `ort-build` before `ort` so the freshly started server runs the latest binary.

## Git Hooks

- The repo ships `.githooks/pre-commit`: when a commit stages Rust files it runs `cargo fmt --all` and re-stages the formatted result. It no-ops when `cargo` is unavailable or no Rust files are staged.
- Install per clone with `./scripts/install-git-hooks.sh` (sets `core.hooksPath=.githooks`). Worktrees of the clone share the setting.
- Bypass a commit with `git commit --no-verify`, or skip one hook run with `OPENCODE_SKIP_FMT_HOOK=1`.
- Staged files that also have unstaged edits are intentionally not re-staged; the hook warns instead so unstaged work is never captured.
