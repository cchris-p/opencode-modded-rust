---
id: "CLI-011"
title: "Watchdog-compatible local web server launch contract"
priority: "P2"
type: "feature"
area: "CLI"
spec: "AGENTS.md"
status: "todo"
created: "2026-10-03"
---

# Watchdog-compatible local web server launch contract

## Summary

Define and, if necessary, implement the `opencode-modded-rust` CLI/server contract that lets Watchdog start a local ORT web server on a registered system, discover its readiness, and stop it without relying on undocumented TUI implementation details.

This card supports Watchdog backend orchestration. Watchdog owns when to instantiate servers, max-instance limits, remote routing, and product frontend UX; ORT owns the local web server behavior and launch/readiness surface.

## Scope

- Provide a documented command or API shape for launching a local ORT web server for a specific workspace/root
- Make bind host, port selection, workspace/root, and machine-readable startup output explicit
- Provide a health/readiness endpoint or command output that Watchdog can poll before exposing the server to users
- Provide a supported stop/termination contract for a server instance started by an external supervisor
- Preserve the existing no-implicit-reuse behavior: Watchdog may start multiple fresh instances up to its own hard limit, but ORT should not silently attach to an unrelated stale server
- Document how this launch path relates to `ort`, `opencode attach <url>`, and vanilla `opencode-modded` web server behavior

## Non-Goals

- Implementing Watchdog backend models, runner messages, or APIs
- Implementing Watchdog frontend parity with the vanilla web UI
- Adding Watchdog-specific access control inside ORT beyond exposing enough process metadata for an external supervisor
- Remote/multi-machine orchestration inside ORT

## Done When

- Watchdog can launch ORT's local web server with documented inputs and parse documented outputs
- The launched server reports readiness through a stable route or machine-readable status output
- The launch path supports external supervision without relying on TUI-only state files or stale-server reuse
- The stop contract is documented and verified locally
- The behavior is covered by appropriate CLI/server tests or documented manual verification

## Recommended Verification

- Run the relevant cargo tests for CLI/server launch behavior
- Launch two fresh ORT web servers for distinct workspaces and confirm they do not reuse stale state
- Confirm a supervisor can discover each instance's URL/port and readiness state
- Confirm stopping one supervised instance does not stop an unrelated instance

## Related Items

- Watchdog `CP-017` - Watchdog-owned OpenCode web server instantiation (`~/apps/watchdog-dev/boards/todo/Watchdog-owned OpenCode web server instantiation.md`)
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers
- `CLI-005` Decide whether same-workspace server attach or reuse should exist
