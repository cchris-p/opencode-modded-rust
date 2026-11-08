# Vision

`opencode-modded-rust` is a new Rust-native coding-agent product optimized for local models and personal daily use.

## Intent

- Build a serious personal daily-driver for a narrow workflow before pursuing broad product scope.
- Optimize for local-model reliability through stronger orchestration, structured state, bounded tasks, and explicit verification.
- Treat long conversational history as an implementation detail rather than the center of the system.
- Prefer deterministic runtime behavior and system-owned lifecycle control over open-ended autonomous chat loops.

## Product posture

- This is a new product, not a long-term TypeScript customization effort.
- The current OpenCode implementation is a reference line, not the primary execution target.
- General-purpose distribution is a much later goal.

## Primary goals

- Make local coding workflows reliable enough for real daily use.
- Preserve a clean runtime architecture that can grow without inheriting all historical OpenCode constraints.
- Introduce higher-quality context construction as a first-class system capability.
- Separate implementation from review so the runtime does not trust a single reasoning trajectory.

## Absorption-resistant substrate

The product is designed to stay useful as models absorb harness features. The
durable value is not a better coding prompt; it is the layer the model cannot
absorb:

- a small, stable runtime core with deterministic task state and authority,
- a volatile policy layer that can be versioned, canaried, and deleted cheaply,
- an engine-neutral System-1 decision boundary (Jev, Laya, deterministic rules),
- owned, provenance-bearing data from decisions and outcomes,
- measured efficiency and trust.

Model-facing scaffolding is allowed when it compensates for a specific local model,
but it is policy, not core, and it must be removable when the model absorbs it.
`wiki/harness-substrate.md` and `invariants/future-proofing.md` define the rules.
The decision layer is planned in `wiki/decision-layer.md`.

## Non-goals for now

- Full feature parity with OpenCode in the first versions.
- Desktop app support.
- MCP, multi-user server concerns, or broad ecosystem compatibility.
- Continuous upstream sync work while the core product is still being formed.
