---
id: "FUTURE-PROOFING-011"
title: "Future-proof the durable and cloud runtime"
priority: "P2"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Future-proof the durable and cloud runtime

## Summary

Prepare the runtime for long-running and off-laptop work: durable execution, resume,
artifact-first distribution, and a clear local/remote boundary.

The lecture's prediction is that within six months most agent work runs off the laptop.
Whether or not that timing is right, the substrate must be able to move a session without
a rewrite.

## Why this exists

The current runtime is local-first and TUI-launched. Long tasks, coordinator/worker
fleets, and software-factory loops cannot depend on a laptop that sleeps. `INFRA-007`
already distributes a published binary; the missing piece is durable execution.

## Scope

- Define a durable run record: task state, policy version, decision log link, artifacts,
  and resume point.
- Define a remote execution boundary: what may run remotely, what must stay local,
  and how secrets are scoped.
- Add resume after process exit and after host migration.
- Reuse the published binary and artifact-first install path (`INFRA-007`).
- Define the local/remote handoff contract for workspace, permissions, and evidence.
- Add a no-hosted-dependency path for the control plane.
- Document the threat model for remote execution.

## Acceptance criteria

- A session can resume from a durable run record after a restart.
- A run can move hosts without losing task state or decision provenance.
- Remote execution is opt-in and scoped.
- Secrets are not copied to a remote host by default.
- The binary-only path works without a repo checkout.
- The control plane works with no hosted decision engine.

## Non-goals

- Building a hosted SaaS.
- Multi-tenant isolation in the first slice.
- Running the GPU transcription service here.
- Replacing the local TUI workflow.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-012`
- `INFRA-007` consume the published ort CLI binary
- `invariants/runtime-lifecycle.md`
- `invariants/session-durability.md`
- `wiki/post-jev-roadmap.md` (next frontier)
