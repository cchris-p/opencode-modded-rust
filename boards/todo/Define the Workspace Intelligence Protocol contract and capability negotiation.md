---
id: "FUTURE-PROOFING-014"
title: "Define the Workspace Intelligence Protocol contract and capability negotiation"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Define the Workspace Intelligence Protocol contract and capability negotiation

## Summary

Formalize the Workspace Intelligence Protocol (WIP): the versioned contract between the
harness and a workspace-intelligence provider.

This turns the current in-process `START-025` retrieval trait into an explicit protocol
with capability negotiation, health, schema versions, and error/fallback semantics.

## Why this exists

The current boundary is a Rust trait. That is a good first implementation, but it does not
give the harness a way to:

- ask what a provider can do,
- reject an incompatible provider cleanly,
- version the map/request/response schema,
- run the provider out of process,
- test a stub, generic, and native provider against one contract.

Without that, ScopeMux is a single implementation, not a substrate capability.

## Scope

- Define the protocol version and the provider handshake.
- Define capability advertisement:
  - locate, slice, project, reconcile,
  - search, delta, observability, duplicates, impact, review,
  - supported languages,
  - map schema version,
  - health/status.
- Define request/response schemas and versioning for each capability.
- Define error semantics: unsupported capability, unsupported language, stale map,
  provider failure, budget exhaustion.
- Define fallback: missing capability -> next rung -> generic provider.
- Define provenance and confidence fields as first-class.
- Define the compatibility rule: the harness only asks for advertised capabilities.
- Provide a deterministic stub provider that implements the full contract for tests.
- Test the stub, the generic provider, and the native provider against the same contract.
- Keep the in-process transport as the first implementation; design for an out-of-process
  transport (`SCOPE-011`).
- Document the contract in `wiki/workspace-intelligence.md` and the code.

## Acceptance criteria

- A provider advertises protocol version, capabilities, languages, map schema, and health.
- The harness refuses an incompatible schema with a clear reason and falls back.
- A missing capability degrades to the next rung, not a task failure.
- The stub provider passes the full contract test.
- The native provider passes the same contract test.
- The generic provider passes the subset it supports.
- No provider-specific type leaks into the core.
- The contract is documented and versioned.

## Non-goals

- Implementing the out-of-process transport (that is `SCOPE-011`).
- Changing task authority.
- Defining every future capability.
- Replacing `opencode-lsp`.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-015`
- `FUTURE-PROOFING-016`
- `SCOPE-011`
- `START-025` retrieval boundary
- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`
