# Product Boundary

## Boundary

`scopemux-code` is the main product. `opencode-modded` remains the long-term reference and planning repository.

## Naming

- `scopemux-code` is the canonical product name.
- `opencode` remains the CLI command name only where compatibility is useful.
- `opencode-modded-rust` and `opencode-*` names are retained as repository and implementation identifiers, not product branding.

## Documentation ownership

- `opencode-modded` retains its own `docs/`, `wiki/`, `invariants/`, and `boards/` artifacts.
- `scopemux-code` must maintain its own `docs/`, `wiki/`, `invariants/`, and `boards/` artifacts.
- The Rust repo may reference files in `opencode-modded` by explicit path.
- Reference by path, not by inheritance: a document in `opencode-modded` does not become Rust-product policy unless it is adopted in this repo.
- `invariants/documentation-boundary.md` is the canonical invariant for this documentation-ownership rule.

## Current scope

- Personal use first.
- TUI-first operation.
- Local-model-first runtime design.
- Strong architectural control over task state, context construction, and verification.

## Reference line freeze

- The initial TypeScript reference line is the current `opencode-modded` state at planning adoption time.
- The current recorded reference commit is `e62912b5d18b73316c7bfd6e894b040698f6c880` in `$HOME/repos/opencode-modded`.
- Future upstream feature review is separate work and should be introduced only through explicit board items.

## Explicit exclusions

- No requirement to preserve exact implementation parity with TypeScript internals.
- No requirement to track upstream bug fixes.
- No requirement to sync with upstream during the core build-out.

## Reference line

- The current OpenCode state is the reference line for useful capabilities and workflows.
- Feature uptake from newer upstream versions is deferred and selective.
- Any future sync review should be treated as explicit product triage, not routine maintenance.
