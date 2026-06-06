# MCP Servers in Practice

The MCP server ecosystem grew rapidly after the protocol's late-2024 release. By 2026 there are servers for most common SaaS products, several major databases, file systems, version control systems, browser automation, and many specialized internal tools. An architect's working knowledge of the ecosystem is part of the foundations.

## Common server categories

- **Code and version control.** GitHub, GitLab, Bitbucket. These servers expose tools for reading issues and pull requests, posting comments, creating branches, reviewing diffs, and so on. The line between "GitHub via MCP" and "GitHub via the gh CLI" is thin; MCP wins when the host already has the protocol wired up.
- **File systems.** Servers that expose read/write access to a directory tree. Filesystem MCP is the canonical example in the MCP documentation and is often the first server a new user installs.
- **Databases.** PostgreSQL, MySQL, MongoDB, and most major databases have community or official MCP servers. They typically expose query and schema-introspection tools.
- **Web fetching.** Servers that wrap `fetch`, render JavaScript, extract structured data, and download resources. Some are heavyweight headless-browser-based; some are simple HTTP fetchers.
- **Communication.** Slack, Discord, email. These servers handle message reading and posting; OAuth is essential.
- **Productivity.** Calendar, Notion, Linear, Jira, Confluence. Same OAuth pattern.
- **Browser automation.** "Claude in Chrome", browser-control servers, Playwright wrappers. These let an agent operate a real browser session.

## Choosing an MCP server

For any given capability there is often more than one server available. The architect's evaluation criteria:

- **Maintained by whom.** First-party (the vendor's own server) is usually the safest choice. Third-party servers vary widely in quality.
- **Auth scope.** A Slack server that asks for `chat:write` is appropriate for a posting agent. A Slack server that asks for full workspace admin is not. The principle of least privilege applies; OAuth scopes are the operative mechanism.
- **Tool surface.** A server with five focused tools is usually more usable than one with fifty under-described tools. Claude can be overwhelmed by large undisciplined tool catalogs.
- **Transport.** Local stdio for sensitive operations; remote HTTP for hosted SaaS where the server vendor handles auth.
- **Performance.** A slow server bottlenecks the agent loop. Latency budgets matter, especially in interactive agents.

## Configuring MCP in Claude Code

Claude Code (the Anthropic CLI) reads MCP server configuration from `~/.claude.json` (user-level) and `.mcp.json` (project-level). The configuration declares server name, command (for stdio) or URL (for HTTP), arguments, and environment variables.

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

Once configured, the server's tools appear to Claude under a namespaced prefix (e.g. `mcp__filesystem__read_file`) and can be invoked like any other tool.

## Writing a minimal MCP server

An MCP server in Python or TypeScript is small. Using the reference SDK, a stdio server that exposes one tool is on the order of twenty lines:

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

The architect's takeaway: when no off-the-shelf MCP server exists for a needed integration, building one is a few hours' work, not a major project. This shifts the make-or-buy calculation in MCP's favor.

## Security considerations

MCP gives agents real access to real systems. Treating MCP servers as trusted code that the agent runs is the right mental model. The architect's checklist:

- **Sandbox where possible.** Local stdio servers run with the user's privileges; consider container or restricted-user execution for production deployments.
- **Audit tool calls.** A pre-tool hook can log every MCP invocation, providing audit trail and runtime visibility.
- **Scope OAuth narrowly.** When delegating server access, grant only the minimum scopes needed.
- **Vet the supply chain.** A malicious MCP server can exfiltrate data, modify files, or send messages on the user's behalf. Treat MCP installs like dependency installs: review the source, check the publisher, watch for known-good signatures.
- **Disable when unused.** Servers configured but unused still appear in the model's tool list, can confuse selection, and represent attack surface.
