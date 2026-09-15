---
id: "H-002"
title: "ort workspace targeting + product default model - Session Status"
status: "open"
created: "2026-09-15"
updated: "2026-09-15"
owner: ""
target: ""
blocked_reason: ""
needs_human: ""
items: ["BUG-011", "FEAT-016"]
---

# ort workspace targeting + product default model - Session Status

## Objective

Make `ort` operate on the directory it is activated from and default to the
deepseek-direct provider, instead of silently targeting the rust repo workspace and
inheriting the shared vanilla global model. Secondary: remove local TUI server reuse
entirely (user directive).

## The reported problem

- `ort` launched from a non-repo directory showed the rust repo as the workspace.
- The default model/provider was wrong outside the repo (`openrouter/...`, reported as
  "deepseek by openrouter"; later "deepseek by opencode" on a different machine).

## Root cause

Two independent mechanisms:

1. **Workspace force-retarget (machine-local launcher).** `~/standards/opencode-config`
   `_opencode_rust_exec` ran `cd "$OPENCODE_RUST_REPO"`, so the launcher (and therefore the
   TUI/server cwd, config search root, and displayed workspace) was always the rust repo.
2. **Default provider (product).** The Rust product had no built-in default model, so with no
   workspace config it inherited the shared global `~/.config/opencode/opencode.json` model,
   which the user's shell generates for vanilla opencode.

Important: the original bug report was gathered while testing on the **wrong machine**. The
observed symptoms were real, but the server-reuse behavior was never the cause. See
"Open decision" below.

## What changed this session

Repo (this PR / `development`):

- **BUG-011** `crates/opencode-config/src/loader.rs`: added `pub const DEFAULT_MODEL =
  "deepseek/deepseek-v4-flash"`, applied in `ConfigLoader::load_all` immediately after
  `load_global()`. The product now owns its default; global config no longer dictates it,
  while `OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`, project config, `.opencode`, and
  managed config still override it (BUG-010 precedence preserved).
- **FEAT-016** `crates/opencode-cli/src/main.rs`: removed the recorded-server reuse path
  (record load/store, rotation, terminate, `server_is_ready`) and now always starts a fresh
  detached server in the activated cwd, choosing the first free port via a new
  `find_available_port`. Legacy `tui-servers/*.json` records are inert.
- `AGENTS.md`: launcher section updated.
- Boards: `BUG-011` and `FEAT-016` created and moved to `qa`; follow-up note added to
  `FEAT-014`.

Machine-local (NOT in the repo, NOT yet pushed at time of writing):

- `~/standards/opencode-config` `_opencode_rust_exec` no longer `cd`s to the repo; it runs
  the binary in the current directory and unsets `OPENCODE_CONFIG_CONTENT` /
  `OPENCODE_CONFIG_DIR`. This is required for the workspace fix on any machine. It currently
  exists as an uncommitted edit on `matrillosub1`.

Follow-up on 2026-09-15: the same machine-local launcher fix was applied on this machine.
The active `_opencode_rust_exec` now runs `env -u OPENCODE_CONFIG_CONTENT -u
OPENCODE_CONFIG_DIR "$binary" "$@"` without changing directory first.

Second follow-up on 2026-09-15: the prior repo fix still missed one product-side path.
`OPENCODE_CONFIG_DIR=$HOME/.config/opencode` and model-only `OPENCODE_CONFIG_CONTENT`
from `opencode-use-auto` could still override the product default when the binary was run
directly or from a shell path that did not clean those variables. `opencode-config` now
protects the platform global config dir and `$HOME/.config/opencode` from the late config-dir
scan, and ignores model-only inline content attached to that shared global config. Richer
inline config content and workspace config still apply.

## Merge status

- Merged into `development` on 2026-09-15 via PR #33 (merge commit `9ef8b47`); feature branch
  `feature/FEAT-016-ort-fresh-server-and-default-model` deleted (remote + local).
- `BUG-011` and `FEAT-016` remain in `qa` pending post-merge user verification.

## Verification performed

- `cargo test -p opencode-config` -> 53 passed (new: `product_default_model_applies_without_workspace_config`,
  `workspace_config_overrides_product_default_model`).
- `cargo test -p opencode-cli` -> 2 passed (new `find_available_port` tests).
- `cargo check -p opencode-cli -p opencode-config` clean.
- Live: fresh server from a non-repo workspace -> `/config/providers` `effective_model:
  deepseek/deepseek-v4-flash`; no new server record written; workspace = launch dir.
- Follow-up verification: `cargo test -p opencode-config` -> 56 passed; `cargo build -p
  opencode-cli` passed; direct `target/debug/opencode config` and fresh-shell `opencode-rust
  config` from a temp non-repo workspace both reported working directory = that workspace and
  default model = `deepseek/deepseek-v4-flash` under the normal shell environment.
- `cargo test -p opencode-server`: one pre-existing environment-dependent failure
  (`skill_route`, counts the 34 global skills under `~/.config/opencode/skills`); passes with
  an isolated `HOME`/`XDG_CONFIG_HOME`.

## Current board state (2026-09-15)

- `todo` (16): `PHASE-001..004`, `FEAT-013`, `FEAT-015`, tool/provider parity items, etc.
- `hold` (6): the evaluation-harness set (`START-020..024`) plus `FEAT-005`.
- `refinement` (0).
- `doing` (3): `SKILL-001`, `SKILLS-001`, `SKILLS-002`.
- `qa` (4): `BUG-007` batch tool, `BUG-007` `ls` listing, plus this session's `BUG-011` and
  `FEAT-016`.
- `done`: `BUG-010` (config precedence), `FEAT-014` (single local TUI server), and 26 others.

## What to do next

1. In any already-open shell, re-source `~/standards/opencode-config` or open a fresh shell,
   then run `ort` from `~/apps/tss-notes` (or equivalent) and confirm the workspace matches.
2. Build the merged work there if needed: `git fetch origin && git checkout development &&
   git pull && ort-build`, then confirm the model is `deepseek/deepseek-v4-flash`.
3. **Open decision - FEAT-016**: re-evaluate whether removing reusable server state is
   worth it now that the wrong-machine misdiagnosis is known.
   - Keep: zero reuse state, but never stops prior servers (they accumulate).
   - Discard: revert the `opencode-cli` change and keep FEAT-014's stop-prior + next-port
     behavior.
   - Middle: fresh server + terminate it on TUI exit (no accumulation, no reuse state).
4. Post-merge QA for `BUG-011` / `FEAT-016`; move them from `qa` to `done` only after user
   verification on `development`.
5. Durable follow-up (optional): move `ort`/`ort-build` into the repo as versioned scripts so
   the workspace behavior travels with the repo instead of living in `~/standards`, which
   diverges across machines.

## Relationship to other work

- `FEAT-014`: kept the single-server/rotation model; `FEAT-016` supersedes its recorded-state
  part. Reverting `FEAT-016` restores `FEAT-014` semantics.
- `BUG-010`: config precedence fix this session must not regress (it does not).
- `FEAT-015`: "preserve manually selected model/provider across sessions" is the eventual
  place for user model selection; the product default here only changes the *unconfigured*
  default.
