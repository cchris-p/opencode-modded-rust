# Provider-Side Visibility of Tool and Model Usage

This document evaluates what the product actually sends to a model provider when it runs a coding session with tools, and classifies that payload by sensitivity. It is an internal reference: it states plainly what leaves the machine, what a provider (and anything behind it) can observe or infer, and where exposure deserves a second look.

It is descriptive. It does not propose or implement redaction, and it does not make privacy guarantees.

## Purpose

- Inventory the request payload a provider receives for a coding-session prompt.
- Separate **provider-visible** data (sent in the request) from **provider-inferable** data (derivable from what was sent).
- Name the sensitive categories that can appear in that payload with concrete examples and code references.
- Document existing exposure-limiting controls and state their limits explicitly.
- Record provider-path differences in payload shape, including where a path currently drops data.
- List follow-up candidates only where a real gap or ambiguity exists.

## Scope and non-goals

In scope:

- The Rust product's provider request assembly and the per-provider wire shapes.
- Tool definitions, tool calls, and tool results as they enter provider requests.
- The env/system prompt and instruction-file injection.

Out of scope:

- Provider terms of service, retention policies, and legal/compliance analysis.
- Changing what is sent, adding redaction, or altering permissions.
- Auditing provider transport beyond noting where secrets travel.

## Vocabulary

- **Provider-visible**: present in the bytes sent to the provider endpoint (headers or body).
- **Provider-inferable**: not sent verbatim, but derivable from what was sent. For example, tool schemas reveal which capabilities the agent has even when no tool is called; a working directory can reveal a username or organization.
- **Sensitive**: data whose disclosure could expose source, secrets, identities, or infrastructure. This is a judgment about content, not a binary property of a field.

## How a coding-session request is assembled

The server resolves an agentic context per prompt: the agent prompt (or the model default prompt), the environment block, and a permission-filtered tool set (`crates/opencode-server/src/agentic.rs:39-73`). The system prompt is the agent/model prompt followed by the environment block (`crates/opencode-server/src/agentic.rs:104-119`).

At each step of the prompt loop the product builds chat messages, merges tool definitions, and issues a `ChatRequest` (`crates/opencode-session/src/prompt.rs:1069-1091`). `build_chat_messages` lifts the system prompt into a leading `role: system` message and converts each stored session message into a provider message (`crates/opencode-session/src/prompt.rs:1852-1880`). `parts_to_content` maps stored parts to text, `tool_use`, `tool_result`, and `reasoning` content parts (`crates/opencode-session/src/prompt.rs:1882-1963`).

Concretely, a request can carry all of the following.

### System prompt

- The selected model prompt, e.g. `gpt-5*` → codex, `gpt-*`/`o1`/`o3` → beast, `gemini-*` → gemini, `claude*` → anthropic, otherwise qwen (`crates/opencode-session/src/system.rs:52-72`).
- A custom agent system prompt when the agent defines one; otherwise the model default (`crates/opencode-server/src/agentic.rs:110-113`). Builtin agents set their own prompt text (`crates/opencode-agent/src/agent.rs:200-374`).
- The environment block (below).

### Environment block

Built by `SystemPrompt::environment` and included verbatim (`crates/opencode-session/src/system.rs:88-112`):

- The active model API id, provider id, and model name.
- `Working directory` — an absolute path, which often contains a username or organization.
- `Is directory a git repo` — yes/no.
- `Platform` — OS.
- `Today's date`.

### Conversation history and attachments

- All non-compacted session messages, converted to provider messages (`crates/opencode-session/src/prompt.rs:981-1070`).
- User text, assistant text, and assistant reasoning parts.
- File attachments: text files are inlined as text; binary/asset MIME types are base64 data URLs (`crates/opencode-session/src/prompt.rs:409-511`). Base64 image/PDF bytes therefore leave the machine when attached.

### Tool schemas

- Each declared tool contributes `{ name, description, parameters }` (`crates/opencode-provider/src/message.rs:139-143`).
- The default tool set is the builtin registry: read, write, edit, bash, glob, grep, ls, task, question, webfetch, websearch, todo, multiedit, apply_patch, skill, lsp, codesearch, plan, plus `invalid` which is filtered out (`crates/opencode-tool/src/registry.rs:222-253`, `crates/opencode-server/src/agentic.rs:127-145`).
- Tools the agent denies are excluded; `allow`/`ask` tools are declared (`crates/opencode-server/src/agentic.rs:127-145`).
- MCP tools recorded in session metadata are merged in as additional definitions (`crates/opencode-session/src/prompt.rs:1073-1074`, `crates/opencode-session/src/prompt.rs:1659-1680`).

### Tool calls and tool results

- Assistant `tool_use` parts are serialized as provider tool calls (`crates/opencode-session/src/prompt.rs:1913-1927`).
- Executed tool output is stored as `tool_result` content and fed back on the next step (`crates/opencode-session/src/prompt.rs:1629-1651`). This is the main mechanism by which real file bodies, grep/glob matches, and command output become provider input.
- The `read` tool additionally injects nearby instruction files (`AGENTS.md`, `CLAUDE.md`, etc.) into the conversation as a `<system-reminder>` block when a file is read (`crates/opencode-session/src/prompt.rs:504-532`, `crates/opencode-session/src/instruction.rs:10-27`). Global instruction files (`~/.config/opencode/AGENTS.md`, `~/.claude/CLAUDE.md`) are also probed (`crates/opencode-session/src/instruction.rs:62-84`).

### Metadata, options, and plugin transforms

- `ChatRequest` metadata sent in the body: model, messages, max_tokens, temperature, top_p, tools, stream (`crates/opencode-provider/src/message.rs:5-21`).
- Plugin hooks can rewrite the message list before it is sent (`crates/opencode-session/src/prompt.rs:1053-1061`), and `provider_options` are merged into the top-level request body on the OpenAI-compatible path (`crates/opencode-provider/src/openai.rs:515-523`). Anything a plugin adds here is provider-visible.

## Per-provider payload shapes

The product uses a provider-neutral `ChatRequest` and converts it per provider. The shapes differ materially.

### OpenAI-compatible (`/chat/completions`)

Used by OpenAI, DeepSeek, OpenRouter, Groq, Mistral, Ollama, and other compatible endpoints. The body is the serialized request with tools wrapped as `{ type: "function", function: { name, description, parameters } }`, assistant tool calls as `tool_calls`, and tool results as separate `{ role: "tool", tool_call_id, content }` messages (`crates/opencode-provider/src/openai_chat.rs:22-140`). Reasoning is echoed as `reasoning_content`. This path sends the **full** payload: system prompt, history, tool schemas, tool calls, and tool results.

### Anthropic

`convert_request` lifts system text into the `system` field and sends only user/assistant **text** parts (`crates/opencode-provider/src/anthropic.rs:99-160`). The `AnthropicRequest` struct has no `tools` field and no tool-use/tool-result content variants (`crates/opencode-provider/src/anthropic.rs:240-271`). As a result, on this path the provider currently receives the system prompt (including the environment block) and text history, but **tool schemas, tool calls, and tool results are not sent**. This matches the open `FEAT-010` Anthropic tool-transport parity item.

### Google

`convert_request` lifts system text into `system_instruction` and sends user/assistant text only; `Role::Tool` messages are dropped and assistant content keeps only text parts (`crates/opencode-provider/src/google.rs:99-162`). `GoogleRequest` also has no `tools` field (`crates/opencode-provider/src/google.rs:239-248`). The provider therefore receives system text and text history, but not tool schemas or tool results.

### Local models (Ollama)

Ollama is registered as an OpenAI-compatible provider pointing at `http://127.0.0.1:11434/v1` by default (`crates/opencode-provider/src/bootstrap.rs:91-122`). When the endpoint is a local model, the same full payload shape applies but no third-party provider receives it. This is the only configured path that keeps payload on the machine; it is a routing choice, not a redaction control.

### Transport and auth

- OpenAI-compatible sends the key as `Authorization: Bearer <key>` (`crates/opencode-provider/src/openai.rs:646-658`, `crates/opencode-provider/src/openai.rs:732`).
- Anthropic sends `x-api-key` (`crates/opencode-provider/src/anthropic.rs:179`).
- Google places the key in the request URL query string (`crates/opencode-provider/src/google.rs:178`).

These are transport details about the product's own credentials, not about user content, and are noted only for completeness.

## Sensitivity classification

### Provider-visible with concrete examples

| Category | How it enters the payload | Reference |
| --- | --- | --- |
| Source code | `read`/`grep`/`glob` output stored as tool results, replayed each turn | `crates/opencode-session/src/prompt.rs:1629-1651`, `crates/opencode-session/src/prompt.rs:1928-1946` |
| File paths and repo layout | `read`/`glob`/`ls` output, working directory, tool-call inputs | `crates/opencode-session/src/system.rs:99`, tool schemas |
| Instruction files | `AGENTS.md`/`CLAUDE.md` content injected on read and at session start | `crates/opencode-session/src/prompt.rs:513-532` |
| Command output | `bash` stdout/stderr stored as tool results | `crates/opencode-tool/src/bash.rs:210-310` |
| Attachments | Text inlined; binary/image/PDF base64 data URLs | `crates/opencode-session/src/prompt.rs:476-510` |
| Secrets in files or env surfaced by tools | If a tool reads a `.env` or a file containing credentials, the body is replayed | tool-result path above |
| Conversation content | User prompts and assistant replies, including reasoning | `crates/opencode-session/src/prompt.rs:1858-1877` |
| Provider/infra metadata | Model id, provider id, agent name, token limits | `crates/opencode-session/src/system.rs:91-94`, `crates/opencode-session/src/prompt.rs:1107-1116` |

### Provider-inferable

| Inference | Basis |
| --- | --- |
| Tool capabilities and absence of tools | The serialized tool schema list (`crates/opencode-provider/src/openai_chat.rs:53-61`) |
| Workspace identity, username, org | Working directory path and repo names in tool output |
| Repository contents beyond what was read | Grep/glob patterns and matches reveal structure and naming |
| Development environment | Platform, git status, model selection |
| Behavior across sessions | Whatever a provider retains from prior requests; the product does not control retention |

### Path-specific exposure

Because tool schemas and tool results are only transmitted on the OpenAI-compatible path, the **breadth** of provider-visible data differs by provider: OpenAI-compatible endpoints see file bodies and command output; Anthropic and Google currently see system text and conversation text only. This is an implementation gap (`FEAT-010`), not a deliberate privacy boundary, and it will change when tool transport lands.

## Exposure-limiting controls and their limits

These reduce volume or gate execution. None of them redact content.

- **Permission filtering / tool disabling**: tools denied by the agent are removed from the declared tool set (`crates/opencode-server/src/agentic.rs:127-145`). This limits what can be executed, but `allow`/`ask` tools that do run still return their full output.
- **Output truncation (bash)**: `bash` caps output at 50 KiB (`crates/opencode-tool/src/bash.rs:14`, `crates/opencode-tool/src/bash.rs:230-297`). Truncation limits size, not sensitivity; the retained prefix still leaves the machine.
- **Generic truncation**: large tool output is reduced to 2000 lines / 50 KiB, keeping the most recent portion and saving the full text locally (`crates/opencode-tool/src/truncation.rs:7-58`).
- **Result caps**: glob shows at most 100 results (`crates/opencode-tool/src/glob_tool.rs:127-140`); grep caps at 100 matches (`crates/opencode-tool/src/grep_tool.rs:142-238`); read defaults to 2000 lines and 50 KiB per file (`crates/opencode-tool/src/read.rs:11-13`).
- **Context compaction**: when context overflows, older messages are replaced by a generated summary that is itself sent to a provider (`crates/opencode-session/src/prompt.rs:1027-1049`, `crates/opencode-session/src/compaction.rs:237-266`). Compaction can resurface or paraphrase sensitive content rather than remove it.
- **Local routing**: selecting a local Ollama endpoint keeps the payload on the machine, but the payload shape is unchanged and nothing prevents later switching back.

There is **no** content redaction, secret scanning, or path anonymization on any path. Truncation and caps are size limits, not privacy guarantees.

## Gaps and follow-up candidates

1. **Anthropic and Google drop tools and tool results** (`crates/opencode-provider/src/anthropic.rs:240-271`, `crates/opencode-provider/src/google.rs:239-248`). Already tracked by `FEAT-010`. When fixed, the sensitive-data surface on those providers expands to match the OpenAI-compatible path, so this document must be revisited.
2. **No secret redaction in tool output.** Any file or command output the agent reads is replayed verbatim. A follow-up item could evaluate a redaction or secret-detection pass before tool results enter the conversation.
3. **Environment block exposes absolute paths.** The working directory is always sent. A follow-up could evaluate sending a workspace-relative or basename form.
4. **External tool egress is separate from provider egress.** `websearch` and `codesearch` call `https://mcp.exa.ai` (`crates/opencode-tool/src/websearch.rs:7`, `crates/opencode-tool/src/codesearch.rs:7`) with query text, and `webfetch` fetches arbitrary URLs (`crates/opencode-tool/src/webfetch.rs:83-85`). These leak data through a different channel than the model provider and are not covered by provider-side analysis alone.
5. **Plugin and provider-option transforms are unaudited.** Hooks can rewrite messages (`crates/opencode-session/src/prompt.rs:1053-1061`) and provider options are spread into the body (`crates/opencode-provider/src/openai.rs:515-523`). There is no review of what a third-party plugin can add.

Splitting these into board items is deferred to the human; items 2, 3, and 5 are candidates. Item 1 already exists as `FEAT-010`. Item 4 may warrant its own document.

## Open questions

- Does any provider path serialize the `system` field of `ChatRequest` in addition to the leading system message? On the paths reviewed, the system prompt is the leading message and `request.system` is `None` (`crates/opencode-session/src/prompt.rs:1081`), so it does not appear to double-send, but this was not confirmed against a live trace.
- Are reasoning parts ever forwarded on paths other than OpenAI-compatible? Reasoning is dropped on Anthropic and Google.
- What does the concrete OpenAI-compatible body look like end to end for a real coding session? Recommended verification is a debug-logged or proxied request body.
- Do hosted gateways (OpenRouter, Bedrock, Vercel, internal gateways) add their own logging or routing that differs from the direct endpoint? Not assessed here.

## Evidence and references

Primary code paths verified for this document:

- Request assembly: `crates/opencode-session/src/prompt.rs:1069-1091`, `crates/opencode-session/src/prompt.rs:1852-1963`, `crates/opencode-server/src/agentic.rs:39-145`.
- System prompt and environment: `crates/opencode-session/src/system.rs:52-142`.
- Instruction injection: `crates/opencode-session/src/prompt.rs:504-532`, `crates/opencode-session/src/instruction.rs:10-84`.
- Provider wire shapes: `crates/opencode-provider/src/openai_chat.rs:22-140`, `crates/opencode-provider/src/anthropic.rs:99-271`, `crates/opencode-provider/src/google.rs:99-248`, `crates/opencode-provider/src/openai.rs:507-523`.
- Local routing: `crates/opencode-provider/src/bootstrap.rs:91-122`.
- Tool limits: `crates/opencode-tool/src/bash.rs:14`, `crates/opencode-tool/src/truncation.rs:7-58`, `crates/opencode-tool/src/glob_tool.rs:127-140`, `crates/opencode-tool/src/grep_tool.rs:142-238`, `crates/opencode-tool/src/read.rs:11-13`.

Line numbers reflect the codebase at the time of writing and will drift.

## Related items

- `BUG-004` Coding sessions run as bare chat: the change that began attaching prompt, environment, and tools.
- `FEAT-010` Anthropic provider tool transport parity: whether tools reach Anthropic models at all.
- `START-025` Retrieval-provider boundary for task context assembly: a future source of additional provider-visible context.
- `START-001` Cross-repo documentation boundary: establishes that this repo owns its own `wiki/`.
