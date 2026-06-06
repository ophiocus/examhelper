# The Claude Agent SDK

The **Claude Agent SDK** is Anthropic's official library for building agentic applications on top of the Messages API. It wraps the basic loop, conversation management, tool execution, sub-agent spawning, and caching, exposed in Python and TypeScript.

The SDK was previously called the **Claude Code SDK** before being generalized in 2025. The same library now powers Claude Code and arbitrary user-built agents.

## What the SDK provides

- **The agent loop.** Calls Claude, executes tools, appends results, loops until termination.
- **Tool registration.** Tools are registered as Python or TypeScript functions; the SDK extracts JSON Schema from type hints and docstrings.
- **Sub-agent spawning.** Sub-agents are first-class.
- **Built-in tools.** File read/write, shell execution, search — the same tools Claude Code uses.
- **MCP integration.** MCP servers attach as tool providers.
- **Hooks.** Pre-call, post-call, pre-tool, post-tool callbacks for instrumentation, audit, modification, and policy.
- **Conversation management.** Saving, resuming, forking; truncation or compaction to fit the context window.
- **Caching.** Default cache breakpoints at the proven-effective locations.

## A minimal agent

```python
from claude_agent_sdk import Agent

agent = Agent(
    model="claude-sonnet-4-5",
    system_prompt="You are a research assistant. Use the search tool; cite your sources.",
    tools=[search_web, read_url],
)

result = agent.run("What were the main findings of the latest IPCC report?")
print(result.text)
```

Twenty lines for what would otherwise be hundreds.

## When the SDK fits

The SDK fits the vast majority of agent shapes:

- Single agent with a tool set.
- Agent with sub-agents for delegation.
- Agent driven by a conversational user.
- Agent driven by a programmatic caller.
- Agent that needs caching, hooks, and lifecycle management.

## When to drop down to the raw API

The SDK is opinionated. Some architectures don't fit:

- **Highly customized agent topologies.** Fixed pipelines, tournament structures, mesh networks of dozens of agents with strict handoff rules sometimes need raw API control.
- **Strict control-flow requirements.** Every step audited in a specific format, every transition checkpointed in a specific database.
- **Stateless serverless endpoints.** A single Claude call in a Lambda doesn't need an agent framework.
- **Existing in-house frameworks.** Migrating to the SDK is rarely worth the cost when an internal equivalent works.

The architect's call is when the SDK helps and when it gets in the way.

## Relationship to Claude Code

**Claude Code** is the canonical Agent SDK consumer. Reading the open-source SDK source — its built-in tools, its hook layout, its sub-agent organization, its compaction strategy — is a fast way to learn idiomatic SDK patterns.

Many production patterns originated in Claude Code and were generalized into the SDK: transcript jsonl files, hook-based policy enforcement, the dedicated "Explore" agent for searches, the compaction protocol. An architect who knows Claude Code's source code has learned half the SDK by osmosis.
