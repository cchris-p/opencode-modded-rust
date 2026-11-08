---
id: "SCOPE-011"
title: "Add out-of-process Workspace Intelligence Protocol transport"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Add out-of-process Workspace Intelligence Protocol transport

## Summary

Add an out-of-process transport for the Workspace Intelligence Protocol so the provider can
run as a separate service/process, with the in-process Rust path as the default and
fallback.

This is the LSP-like part of WIP: the contract survives a process boundary, and the
provider can be tested, versioned, and replaced independently.

## Why this exists

The current provider is in-process FFI. That is fast and simple, but it couples the
harness to the native build, makes crash isolation hard, and limits reuse. An
out-of-process transport gives:

- isolation from native crashes,
- independent provider lifecycle/versioning,
- easier contract testing,
- the option to run the map on another host later,
- a clean path for non-Rust consumers.

## Scope

- Define the transport: stdio/JSON-RPC or an equivalent framed protocol.
- Implement handshake, capability negotiation, health, and version checks.
- Map the WIP request/response schemas over the transport.
- Add timeouts, cancellation, and backpressure.
- Add crash/restart handling and fallback to in-process/generic.
- Define security: local-only by default, no network listener without explicit config,
  no secrets in requests.
- Keep the in-process path as the default; out-of-process is opt-in/profile-driven.
- Add a conformance test that runs the same contract against in-process and out-of-process
  providers.
- Document the transport in `wiki/workspace-intelligence.md`.

## Acceptance criteria

- The same contract test passes for in-process and out-of-process providers.
- A provider crash degrades to in-process/generic, not a session crash.
- Timeouts and cancellation work.
- Capability negotiation and version checks work over the transport.
- No network listener is opened by default.
- The in-process path remains the default.

## Non-goals

- A remote multi-tenant service.
- Replacing the in-process provider.
- A general RPC framework.
- Cross-machine map sharing in the first slice.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-014`
- `FUTURE-PROOFING-016`
- `SCOPE-009` freshness/staleness
- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`
