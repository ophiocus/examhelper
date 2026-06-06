# What Is MCP?

The **Model Context Protocol (MCP)** is an open protocol developed by Anthropic for connecting language models to external tools, data sources, and services. MCP was announced in November 2024 and has since been adopted across the model and tool ecosystem.

The problem MCP solves: every tool integration with a model is normally an ad-hoc piece of code. A weather API for Claude needs a Claude-specific wrapper; the same API for another model needs a different wrapper; a different tool for Claude needs yet another wrapper. The combinatorial explosion makes the tools-and-models ecosystem fragmented and brittle.

MCP standardizes the wire format between models (or model-using applications) and tools so that any MCP-compatible tool can be used by any MCP-compatible client. The architecture is modeled on **LSP**, the Language Server Protocol for editors and language tooling.

## The three actors

- **The host.** The application that uses the model — Claude Code, Claude.ai, or a custom agent on the SDK. The host coordinates the model and the MCP servers.
- **The client.** A library embedded in the host that speaks MCP. Typically one client per server connection.
- **The server.** A process that exposes tools, resources, or prompts via MCP. Local (a process on the user's machine) or remote (an HTTP-reachable service).

The flow: the host loads the server, the server advertises its capabilities, the host registers those with the model, the model calls them as needed, the host routes calls to the server, and results return to the model.

## What an MCP server provides

A server can expose three kinds of capability:

- **Tools.** Functions the model can call. Each tool has a name, description, and JSON Schema input — the same shape as Claude's native tool definitions. The server implements the tool.
- **Resources.** Read-only content the host can supply to the model as context — files, database rows, API responses, anything addressable by URI. Pull-based: the model or host requests them; the server returns them.
- **Prompts.** Parameterized prompt templates the user (or host) can invoke. Useful for common workflows the server's authors know are best expressed as a particular prompt structure.

Most production servers expose only tools.

## Transports

MCP defines two transports:

- **stdio.** The host spawns the server as a subprocess and communicates over stdin/stdout. Canonical for local servers that wrap CLI tools, manipulate files on the user's machine, or hold local credentials.
- **HTTP with Server-Sent Events.** The host connects to a remote URL; requests are POSTed, responses stream as SSE. The transport for hosted MCP servers — a company's API exposed through MCP.

Both speak the same MCP message format; the difference is purely in delivery.

## Authentication

Servers handling privileged operations need authentication.

- **Local stdio servers** inherit the user's credentials — the server reads `~/.config/...` or environment variables.
- **Remote HTTP servers** use **OAuth 2.0** in a flow defined by the MCP specification. The host directs the user to authenticate with the server's provider, the server returns a token, subsequent MCP calls carry the token.

This standardization is one of MCP's chief practical advantages. Without it, every remote tool requires its own auth dance.

## Why MCP matters

Five years ago, adding email-sending or calendar-reading to an agent meant writing custom tool code. Today, an MCP server for Gmail or Google Calendar exists; attaching it is a configuration change.

The architect's question shifts from *how do I implement this tool* to *which MCP server do I attach and how do I scope its access*.

The architect should be familiar with:

- Listing and configuring MCP servers in Claude Code and other hosts.
- Evaluating an MCP server's security posture (scopes claimed vs. access actually needed).
- Writing a minimal MCP server when a needed integration doesn't exist.
- Choosing between stdio and HTTP transport for a given deployment.

## MCP and the cert

MCP is **18% of the CCA-F exam** (combined with tool design). Expect questions on the actors, the transports, the auth model, tool/resource/prompt distinctions, and architectural trade-offs.
