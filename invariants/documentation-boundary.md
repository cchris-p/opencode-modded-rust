# Documentation Boundary Invariants

- `opencode-modded-rust` owns its own `docs/`, `wiki/`, `invariants/`, and `boards/` artifacts.
- Files in `$HOME/repos/opencode-modded` may be referenced only by explicit path.
- Cross-repo references provide context, comparison, or planning traceability; they do not create shared ownership.
- Reference by path, not by inheritance: a document in `$HOME/repos/opencode-modded` does not become binding Rust-product policy automatically.
- Any rule, invariant, or plan that should constrain `opencode-modded-rust` must be restated in this repository.
