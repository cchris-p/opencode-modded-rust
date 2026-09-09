# Coding-Session Behavior Invariants

- An agentic coding session must behave like the reference OpenCode coding agent, not like a plain chat completion.
- Every agentic model request in a coding session must carry all of: the resolved agent identity (defaulting to `build`), the model-appropriate agent system prompt, the workspace/environment context block, and the agent's permission-filtered tool set.
- A session request that omits the system prompt, the environment block, or the tool set is a defect, never a valid coding-session state.
- System prompt, environment, and tool resolution must happen per request from durable session state and repository information, mirroring the reference resolution path, rather than being left to caller-supplied per-message extras.
- The TUI/server prompt path and the CLI prompt path must both attach the agent system prompt, environment context, and tools; neither path may silently degrade into a bare chat request.
- The runtime must not rely on a provider auto-injecting tools or a default system prompt; tool and system attachment is the session layer's responsibility for every provider.
- Session persistence must round-trip the resolved agent, model, and tool context so that a resumed session reconstructs the same agentic behavior instead of losing context.
- A model that is not capable of tool calling must be gated explicitly and visibly; it must not cause a coding session to silently run without tools or environment context.
- Tool-call execution must remain reachable end-to-end: once tools are declared on a request, the registry must be able to execute them and return results into the same session.
