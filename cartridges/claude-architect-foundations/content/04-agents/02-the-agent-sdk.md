# The Claude Agent SDK

The **Claude Agent SDK** is Anthropic's official library for building agentic applications on top of the Messages API. It wraps the basic agent loop, conversation management, tool execution, sub-agent spawning, and caching, and exposes a high-level interface in Python and TypeScript.

The SDK was previously known as the **Claude Code SDK** before being generalized in 2025. The same library now powers both Claude Code (Anthropic's CLI coding tool) and arbitrary user-built agents.

## What the SDK provides

The SDK's value is in handling the parts of agent construction that are mechanical and repetitive, so the application code can focus on the parts that are domain-specific.

- **The agent loop.** The SDK calls Claude, executes tools, appends results, and loops until the agent finishes or hits a configured limit.
- **Tool registration.** Tools are registered as Python or TypeScript functions; the SDK extracts the schema from type hints and docstrings, builds the API tool definition, and dispatches calls automatically.
- **Sub-agent spawning.** Sub-agents are first-class. The parent agent can spawn a sub-agent with its own system prompt, tool set, and lifecycle, and receive back the sub-agent's final result.
- **Built-in tools.** The SDK ships a set of general-purpose tools — file read/write, shell execution, search — that an agent can use immediately. These are the same tools Claude Code uses.
- **MCP integration.** Model Context Protocol servers (see the MCP section) can be attached to an agent and their tools become available alongside locally-defined tools.
- **Hooks.** Pre-call, post-call, pre-tool, post-tool hooks for instrumentation, audit, modification, and policy enforcement.
- **Conversation management.** Saving, resuming, and forking conversations; truncating or summarizing history to fit the context window.
- **Caching.** The SDK applies prompt caching breakpoints by default at the points known to be most effective (end of system prompt, end of tools, end of growing history).

## A minimal agent

```python
from anthropic import Anthropic
from claude_agent_sdk import Agent

agent = Agent(
    model="claude-sonnet-4-5",
    system_prompt="You are a research assistant. Use the search tool to find information; cite your sources.",
    tools=[search_web, read_url],
)

result = agent.run("What were the main findings of the latest IPCC report?")
print(result.text)
```

The SDK handles authentication, the loop, the tool dispatch, and the conversation. The application code is twenty lines for what would otherwise be hundreds.

## Where the SDK is the wrong choice

The SDK is opinionated. It assumes a single primary agent with optional sub-agents, a Pythonic tool-registration style, and a particular flow for hooks. Some architectures don't fit:

- **Highly customized agent topologies** — for example, a fixed pipeline of dozens of agents with rigid handoff rules — may be easier to express directly on the API.
- **Stateless serverless endpoints** that complete in a single Claude call don't need an agent framework.
- **Strict control-flow requirements** (every step must be auditable in a specific format, must be checkpointed in a specific database) sometimes benefit from a hand-written loop.

For the majority of agentic applications, the SDK is the right starting point. The architect's call is when to drop down to the raw API.

## Relationship to Claude Code

**Claude Code** is the canonical agent built on the SDK. It is a CLI coding tool: an agent with file-system tools, shell access, version-control awareness, and a long-context system prompt. Studying Claude Code's open-source SDK source is a fast way to learn idiomatic Agent SDK patterns — its tool definitions, its hook structure, its sub-agent organization (for example, the dedicated "Explore" agent for searching code), and its compaction strategy.

Many architectural patterns in production Claude agents — checkpoint files, transcript jsonl files, hook-based policy — originated in Claude Code and were generalized into the SDK.
