# ScopeMux Integration Scope Invariants

- Integration work includes the `scopemux` development it depends on.
- The ScopeMux integration program owns the required `scopemux-core` engine changes and the `scopemux-notes` planning and board work that gates them.
- Upstream `FIX-*` and `WI-*` items that gate integration are in-scope deliverables of that program, not external prerequisites owned by someone else.
- Upstream artifacts are edited in their own repository (`$HOME/apps/scopemux-core`, `$HOME/apps/scopemux-notes`) per `invariants/documentation-boundary.md`; this convention does not import upstream ownership into this repo.
- This convention does not make `scopemux` a V1 hard dependency: the generic provider remains the default and fallback, and runtime authority over task state, lifecycle, verification, and review is unchanged.
