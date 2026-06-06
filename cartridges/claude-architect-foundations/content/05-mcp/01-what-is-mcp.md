# What Is MCP?

The **Model Context Protocol (MCP)** is an open protocol developed by Anthropic for connecting language models to external tools, data sources, and services. MCP was announced in November 2024 and has since been adopted by other model vendors and tool ecosystems.

The problem MCP solves: every tool integration with a model is normally an ad-hoc piece of code. A weather API for Claude requires a Claude-specific wrapper; the same API for another model requires a different wrapper; a different tool for Claude requires yet another wrapper. The combinatorial explosion makes the tools-and-models ecosystem fragmented and brittle.

MCP standardizes the wire format between models (or model-using applications) and tools, so that any MCP-compatible tool can be used by any MCP-compatible client. The architecture is analogous to LSP (the Language Server Protocol) for IDEs and language tooling, and MCP was deliberately modeled on that precedent.

## The MCP architecture

MCP has three actors:

- **The host.** The application that uses the model — for example, Claude Code, Claude.ai, or a custom agent built on the Agent SDK. The host coordinates the model and the MCP servers.
- **The client.** A library embedded in the host that speaks the MCP protocol. There is typically one client per MCP server connection.
- **The server.** A process that exposes tools, resources, or prompts via MCP. Servers can be local (a process running on the user's machine) or remote (an HTTP-reachable service).

The flow: the host loads the MCP server (spawning it or connecting to it), the server advertises its capabilities (tools, resources, prompts), the host registers these with the model, the model calls them as needed, the host routes the calls to the appropriate server, and results return to the model.

## What an MCP server provides

A server can expose three kinds of capability:

- **Tools.** Functions the model can call. Each tool has a name, description, and input schema — the same shape as Claude's native tool definitions. The MCP server is what implements the tool.
- **Resources.** Read-only content that the host can supply to the model as context — files, database rows, API responses, anything addressable by a URI. Resources are pull-based: the model or host requests them; the server returns them.
- **Prompts.** Parameterized prompt templates that the user (or host) can invoke. Useful for common workflows that the server's authors know are best expressed as a particular prompt structure.

Most production MCP servers expose only tools; resources and prompts are used by a smaller set of integrations.

## Transports

MCP defines two transports:

- **stdio.** The host spawns the server as a subprocess and communicates over stdin/stdout. This is the canonical transport for local servers — a server that wraps a CLI tool, manipulates files on the user's machine, or holds local credentials.
- **HTTP with Server-Sent Events.** The host connects to a remote URL; requests are POSTed, responses stream as SSE. This is the transport for hosted MCP servers — a company's API exposed through MCP, accessible to any client.

Both transports speak the same MCP message format; the difference is purely in delivery.

## Authentication

MCP servers that handle privileged operations need authentication. Local stdio servers typically inherit the user's credentials (the server reads `~/.config/...` or environment variables). Remote HTTP servers use **OAuth 2.0** in a flow defined by the MCP specification: the host directs the user to authenticate with the server's provider, the server returns a token, and subsequent MCP calls carry the token. This standardization is one of MCP's chief practical advantages — without it, every remote tool requires its own auth dance.

## Why MCP matters to an architect

MCP changes the build-vs-integrate decision for many capabilities. Five years ago, adding email-sending or calendar-reading to an agent meant writing custom tool code. Today, an MCP server for Gmail or Google Calendar exists; attaching it to a Claude agent is a configuration change. The architect's question shifts from *how do I implement this tool* to *which MCP server do I attach and how do I scope its access*.

The architect should be familiar with:

- How to list and configure MCP servers in Claude Code and other hosts.
- How to evaluate an MCP server's security posture (what scopes does it claim, what does it actually access).
- How to write a minimal MCP server when a needed integration doesn't exist.
- The trade-offs of stdio versus HTTP transport for a given deployment.
