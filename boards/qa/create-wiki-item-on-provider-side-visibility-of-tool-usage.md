---
id: "START-028"
title: "Create a wiki item on how tool and provider usage appears to API providers"
priority: "P2"
type: "docs"
area: "START"
spec: "wiki/README.md"
status: "qa"
created: "2026-09-21"
---

# Create a wiki item on how tool and provider usage appears to API providers

## Summary

Create one wiki document that evaluates, from the API provider's side, what this product actually sends when it uses a provider model and tools, and then classifies what of that payload is sensitive. The goal is a single, honest reference for what a provider (and anything behind it) can observe about the user's workspace, code, prompts, and tool activity.

## Why this exists

The product now attaches an agent system prompt, environment context, and a permission-filtered tool set to coding-session model requests (`BUG-004`), and tool results routinely carry real file contents and command output back into the conversation. That is powerful but it means provider-visible data is much broader than the user's typed prompt. There is no document that states plainly what leaves the machine, what the provider can infer, and where sensitive information deserves a second look. This item produces that document before more provider or tool surface is added.

## Questions to answer

- What exactly is in the request body a provider receives: system prompt, environment block, tool schemas, conversation history, tool calls, tool results, and any metadata?
- How much of the workspace is exposed indirectly through tool use (Read file bodies, Grep/Glob matches, Bash stdout/stderr, LSP/grep output), and how is it bounded (truncation, path filtering, permissions)?
- What is provider-visible versus provider-inferable? For example, tool schemas reveal capability, but tool results reveal actual code and command output.
- What sensitive categories can appear in that payload: source code, credentials or secrets in files/env, personal paths and usernames, repository names, logs, database contents, and instruction files?
- Which providers see what, and does the OpenAI-compatible, Anthropic, Google, and local-model paths differ in payload shape or retention?
- What existing controls reduce exposure today (permission filtering, tool disabling, output truncation, local-model routing), and where are the gaps?
- What should the product state to users about provider-side visibility, and what follow-up work (if any) is warranted?

## Current understanding (starting point, verify before relying on it)

- Coding-session requests attach an agent system prompt, an environment block, and a permission-filtered tool set; before `BUG-004` sessions were bare chat with none of this (`boards/done/coding-sessions-run-as-bare-chat-no-agent-context.md`).
- Provider requests are assembled in the session prompt path (`crates/opencode-session/src/prompt.rs:1069`), where `build_chat_messages` lifts the system prompt into the provider chat request (`crates/opencode-session/src/prompt.rs:1846`).
- Tools are serialized to providers as `{ name, description, parameters }` (`crates/opencode-provider/src/tools.rs:585`); OpenAI-compatible paths wrap them as input tools (`crates/opencode-provider/src/openai.rs:594`).
- Tool results are fed back into the conversation, so file bodies, grep/glob matches, and shell output become subsequent provider input. Output is bounded, not redacted: Bash truncates at 50 KiB (`crates/opencode-tool/src/bash.rs:14`), and Glob caps at 100 results (`crates/opencode-tool/src/glob_tool.rs:127`).
- Instruction files are injected into the conversation when read (`crates/opencode-session/src/prompt.rs:513`), which means project instruction content is provider-visible.
- No existing wiki, docs, or invariant document discusses provider-side visibility, retention, or sensitive-information exposure (checked `wiki/`, `docs/`, `invariants/`).

## Scope

- Write a wiki document that inventories the request payload a provider receives and classifies it by sensitivity.
- Distinguish clearly between data sent by the product and data the provider can only infer.
- Document bounded-but-unredacted behaviors (truncation, caps, permission filtering) and name them as limits, not as privacy guarantees.
- Cover the OpenAI-compatible, Anthropic, Google, and local-model paths at the level of payload shape and any retention/visibility differences.
- Define the vocabulary (provider-visible, provider-inferable, sensitive) and keep the document descriptive.
- List concrete follow-up items only where a real gap or ambiguity is found; do not implement changes here.

## Non-goals

- Changing what the product sends to providers, adding redaction, or altering permissions.
- Legal, compliance, or contractual analysis of any specific provider.
- Auditing provider terms of service or retention policies beyond stating what the product assumes.
- Rewriting provider transport, auth, or tool schemas.
- Producing marketing or user-facing privacy copy; this is an internal reference doc.

## Done when

- A wiki document exists that lists what is sent to providers and what is inferable from it.
- Sensitive categories that can appear in the payload are named with concrete examples and file references.
- Existing exposure-limiting controls and their limits are stated explicitly.
- Provider-path differences in payload shape are documented or explicitly marked unknown.
- Any real gaps are split into implementation-ready board items, or explicitly declared unnecessary.
- The new document is linked from `wiki/README.md` navigation.

## Recommended verification

- Read `crates/opencode-session/src/prompt.rs` (message assembly, system prompt, instruction injection) and confirm the described payload matches what is actually built.
- Read `crates/opencode-provider/src/openai.rs`, `anthropic.rs`, and `google.rs` and confirm the request shapes and which fields carry history, tools, and tool results.
- Trace a real coding session request body (debug logging or a proxy) on the daily-driver path and confirm the inventory against observed fields.
- Spot-check truncation/caps referenced above (`bash.rs`, `glob_tool.rs`) so limits are stated accurately.
- Confirm the final document is reachable from `wiki/README.md`.

## Related Items

- `BUG-004` Coding sessions run as bare chat: foundation that made providers receive prompt, environment, and tools
- `FEAT-010` Anthropic provider tool transport parity: determines whether tools reach Anthropic models at all
- `START-025` Add retrieval-provider boundary for task context assembly: future source of additional provider-visible context
- `START-001` Adopt cross-repo documentation boundary: establishes that this repo owns its own `wiki/`

## Notes

- Treat this as a documentation and evaluation item; the deliverable is the wiki document plus any split-off gaps.
- Prefer concrete evidence (request traces, file references) over general statements about privacy.
- If a section cannot be grounded in code or an observed request, mark it as an open question rather than asserting it.

## Dev Notes

Delivered `wiki/provider-side-visibility.md` and linked it from `wiki/README.md` navigation.

What the document covers:

- Request assembly: system prompt (agent/model + environment block), conversation history, attachments, tool schemas, tool calls, and tool results, with code references (`crates/opencode-server/src/agentic.rs`, `crates/opencode-session/src/prompt.rs`, `crates/opencode-session/src/system.rs`).
- Provider-path differences: OpenAI-compatible sends the full payload (tools, tool calls, tool results); Anthropic and Google currently send system text plus text history only and drop tools/tool results (`FEAT-010`); Ollama is an OpenAI-compatible local route.
- Sensitivity classification split into provider-visible versus provider-inferable, with concrete examples and file references.
- Exposure-limiting controls (permission filtering, output truncation and caps, compaction, local routing) stated explicitly as size/gating limits, not privacy guarantees.
- Follow-up candidates and open questions, including the absence of secret redaction, absolute-path exposure in the environment block, separate egress from `websearch`/`codesearch`/`webfetch`, and unaudited plugin/provider-option transforms.

Notable findings:

- The broadest exposure (file bodies, command output) currently only reaches OpenAI-compatible providers; Anthropic and Google receive prompt and text history but no tools or tool results. This is a transport gap, not a privacy boundary.
- Instruction files (`AGENTS.md`/`CLAUDE.md`) become provider-visible both via the `read` tool injection path and, for global files, during session assembly.
- There is no content redaction on any path.

Verification performed:

- Read and cross-checked request assembly, system prompt/environment, instruction injection, provider conversion, tool registry, and tool output limits against source.
- Confirmed tool serialization shape in `openai_chat.rs`, and absence of a `tools` field in the Anthropic and Google request structs.
- Confirmed output limits in `bash.rs`, `truncation.rs`, `glob_tool.rs`, `grep_tool.rs`, and `read.rs`.

Deferred (not implemented here, per non-goals):

- Creating board items for the follow-up candidates was intentionally left to the user; item 1 already exists as `FEAT-010`.
- No live request-body trace was captured; that remains a recommended future verification listed as an open question.