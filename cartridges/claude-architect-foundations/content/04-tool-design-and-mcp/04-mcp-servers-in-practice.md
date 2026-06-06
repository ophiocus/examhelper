# MCP Servers in Practice

The MCP ecosystem grew rapidly after the protocol's late-2024 release. By 2026 there are servers for most common SaaS products, several major databases, file systems, version control systems, browser automation, and many specialized internal tools. Working knowledge of the ecosystem is part of the architect's foundations.

## Common categories

- **Code and version control.** GitHub, GitLab, Bitbucket. Tools for reading issues and PRs, posting comments, creating branches, reviewing diffs.
- **File systems.** Servers exposing read/write access to a directory tree. The canonical example in the MCP documentation.
- **Databases.** PostgreSQL, MySQL, MongoDB. Query and schema-introspection tools.
- **Web fetching.** Servers wrapping `fetch`, rendering JavaScript, extracting structured data.
- **Communication.** Slack, Discord, email. OAuth is essential.
- **Productivity.** Calendar, Notion, Linear, Jira, Confluence. OAuth flows.
- **Browser automation.** Claude in Chrome, Playwright wrappers. Let an agent operate a real browser.

## Choosing an MCP server

For any capability there is often more than one server. The architect's criteria:

- **Maintained by whom.** First-party (the vendor's own) is usually safest. Third-party quality varies widely.
- **Auth scope.** A Slack server asking for `chat:write` is appropriate for a posting agent. A Slack server asking for full workspace admin is not. Least privilege; OAuth scopes are the mechanism.
- **Tool surface.** Five focused tools is usually more usable than fifty under-described tools. Claude can be overwhelmed by large undisciplined catalogs.
- **Transport.** Local stdio for sensitive operations; remote HTTP for hosted SaaS where the vendor handles auth.
- **Performance.** A slow server bottlenecks the agent loop. Latency budgets matter in interactive agents.

## Configuring MCP in Claude Code

Claude Code reads MCP configuration from `~/.claude.json` (user-level) and `.mcp.json` (project-level). The configuration declares server name, command (for stdio) or URL (for HTTP), arguments, environment variables.

Example user-level entry for a filesystem server:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/me/code"]
    }
  }
}
```

Once configured, the server's tools appear to Claude under a namespaced prefix (`mcp__filesystem__read_file`) and are invoked like any other tool.

## Writing a minimal MCP server

An MCP server in Python or TypeScript is small. Using the reference SDK, a stdio server with one tool is about twenty lines:

```python
from mcp.server.fastmcp import FastMCP

mcp = FastMCP("greeter")

@mcp.tool()
def greet(name: str) -> str:
    """Return a greeting for the given name."""
    return f"Hello, {name}!"

if __name__ == "__main__":
    mcp.run()
```

The architect's takeaway: when no off-the-shelf server exists, building one is a few hours' work, not a major project. The make-or-buy calculation shifts in MCP's favor.

## Security

MCP gives agents real access to real systems. Treat MCP servers as trusted code the agent runs.

- **Sandbox where possible.** Local stdio servers run with the user's privileges; consider container or restricted-user execution in production.
- **Audit tool calls.** A pre-tool hook can log every MCP invocation, providing audit trail and runtime visibility.
- **Scope OAuth narrowly.** When delegating server access, grant only the minimum scopes.
- **Vet the supply chain.** A malicious MCP server can exfiltrate data, modify files, or send messages on the user's behalf. Treat MCP installs like dependency installs — review source, check publisher, watch for signatures.
- **Disable when unused.** Servers configured but unused still appear in the model's tool list, can confuse selection, and represent attack surface.

## MCP in the wider architecture

For a CCA-F architect, MCP is most useful as a **decoupling layer**:

- The model (Claude) talks MCP.
- The host (Claude Code, an SDK-based agent, or a custom application) wires up MCP servers.
- Tools and data sources live behind MCP servers.

This decoupling means tools can be swapped or upgraded without changing the model or the host. It means the same toolset can be reused across multiple agents. It means tool vendors can ship their own servers and have them work everywhere.

The architectural lesson: when designing a new agent capability, **first ask whether an MCP server is the right shape for it**. If yes, the capability becomes reusable. If no — because it is specific to one agent's internal logic — then a native tool is fine.
