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

## Planning Direction

- `wiki/` contains architecture and version-direction documents.
- `invariants/` contains absolute truths for the final desired system.
- `boards/` tracks execution work, deferred work, and future feature triage.
- `docs/archive/session.md` is archived brainstorming source material, not the authoritative plan.
- The frozen TypeScript reference line is `$HOME/repos/opencode-modded` at commit `e62912b5d18b73316c7bfd6e894b040698f6c880` until a later board item explicitly changes it.

## Current Product Stance

- Personal daily-driver on a narrow workflow is the immediate target.
- TUI is part of V1.
- Full parity with OpenCode is deferred.
- Upstream sync is optional and selective, not a standing maintenance obligation.
