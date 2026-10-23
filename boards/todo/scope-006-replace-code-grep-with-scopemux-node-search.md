---
id: "SCOPE-006"
title: "Replace the code grep tool with ScopeMux node search, keeping grep as fallback"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/scopemux-map-integration.md"
status: "todo"
created: "2026-10-23"
request_id: "191c814a-1e0b-45f6-8ad0-20db0d46f3eb"
---

# Replace the code grep tool with ScopeMux node search, keeping grep as fallback

## Summary

Add an agent tool call that searches the workspace through the ScopeMux map instead of walking the filesystem. The call queries the registered node set (symbols, InfoBlocks, structural IR) across the whole tree in one shot, so the agent "transcends" the file/directory-tree walk that `grep` does today and finds what it needs faster and cheaper.

All nodes must always be registered and periodically updated. The operator believes this is already implemented (the `scopemux-core` incremental index/watcher, `WI-018`, plus the provider integration behind `START-025`); the first step of this card is to test node registration and periodic update on a real codebase and confirm or correct that belief before building the tool on top of it.

Implement the grep replacement with ScopeMux after that verification. Expected efficiency wins: the agent stops paying full-tree filesystem walks to locate code, and the small local Qwen model benefits further from token-budgeted, structural results.

`grep` stays as a fallback: when the ScopeMux provider is absent, the workspace language is unsupported, or the node search fails, the tool falls back to the existing bounded `grep` (per `BUG-040` guardrails). The tool surface distinguishes two search modes: one for markdown/prose files (grep-based, since ScopeMux parses code, not prose) and one for code bases (ScopeMux-backed with grep fallback).

## Acceptance Criteria

- Node registration and periodic update are verified empirically on a real code base (this repository is the natural first target once `WI-030` Rust grammar lands; otherwise a supported-language repo): nodes for existing symbols exist, edits produce updated nodes within the periodic update cycle, and stale review is not required to rebuild from scratch. Record the finding; if the behavior is not actually implemented, surface that as a prerequisite instead of building on it.
- A code-search tool call exists that answers "where is X" from the ScopeMux node registry in a single call, without a filesystem walk, and returns candidates with provenance (file, symbol, block id, confidence) per `invariants/retrieval.md`.
- `grep` remains available and is used as the fallback whenever the ScopeMux path cannot answer (provider absent, unsupported language, empty/indexing result used as genuine no-match only with care).
- The tool surface separates markdown/prose search (grep-based) from code search (ScopeMux-backed); the agent does not run code-grammar search over prose or vice versa.
- Runtime authority is unchanged: the search tool is evidence-only; no task state, lifecycle, verification, or review moves into `scopemux` (`invariants/task-state.md`, `invariants/integration-scope.md`).
- Activation follows the provider scope rule: the ScopeMux-backed path activates for the local Qwen-via-Ollama model and every other model/workspace uses the generic path (`invariants/providers.md`, `SCOPE-005`).

## Dev Notes

- The ScopeMux provider and its Qwen-only activation already exist (`SCOPE-002`, `SCOPE-005`); reuse `opencode-scopemux` and the `START-025` retrieval boundary rather than adding a second integration path.
- The existing `grep` tool already carries blocking-walk fixes (`spawn_blocking`, build/vendor exclusions, bounded scans; `BUG-040`, commit `1fb80d0`). The fallback mode must inherit those bounds, not regress them.
- The markdown/code split mirrors how the runtime treats code vs docs contexts; keep the split at the tool-argument/dispatch level so prompt-facing surface stays one tool name with a mode, unless implementation shows two distinct tool names are clearer for the agent.
- Respect the Phase-1 dependency reality: on Rust workspaces the node search currently cannot work until `WI-030` (upstream, in-scope per `invariants/integration-scope.md`) lands; until then the code-search mode on a Rust workspace is the grep fallback with explicit provenance.
- Do not break the "replacing direct file reads for small, obvious tasks" non-goal from `SCOPE-001`; the tool complements reads, it does not forbid them.

## Related Items

- `SCOPE-001` Idealized scopemux map integration - the locate/slice model this tool instantiates.
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary - provider plumbing to reuse.
- `SCOPE-005` Activate the ScopeMux provider only for the local Qwen-via-Ollama model - activation scope.
- `BUG-040` Grep tool blocks the async runtime - the bounded grep behavior the fallback must preserve.
- `WI-018` incremental index/watcher and `WI-030` Rust grammar (upstream `$HOME/apps/scopemux-notes`, in-scope per `invariants/integration-scope.md`).

## Notes

Captured from the 2026-10-23 operator voice request (`request_id: 191c814a-1e0b-45f6-8ad0-20db0d46f3eb`). The operator's words, condensed: "ScopeMux needs to replace the grep functions... a tool call in ORT where the tool call transcends the tree and file directory trees in order for it to quickly find what it needs. All nodes must always be registered and periodically updated - I think this feature is already implemented; we just need to test it out on a code base and implement the grep replacement with ScopeMux... grep should be the fallback. ORT will need two separate greps, one for markdown and one for code bases."
