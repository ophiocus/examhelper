# What Is Claude Code?

**Claude Code** is Anthropic's official command-line agent for software engineering work. It is the canonical production deployment of the Claude Agent SDK — a long-context system prompt, a curated tool set, hooks, sub-agents, and MCP integration, wrapped in a CLI that integrates with editors, terminals, and version control.

For the CCA-F architect, Claude Code is both a daily tool and the reference implementation of Claude-based agent architecture. Many of the patterns the exam tests originated here.

## Installation and invocation

Claude Code installs as an npm-based CLI:

```
npm install -g @anthropic-ai/claude-code
```

It is then invoked as `claude` in any terminal. Run inside a project directory; the CLI takes that directory as the working tree.

A first run prompts for an API key (stored in the OS keychain or `~/.claude/credentials`) and a session begins. The user types prompts; Claude responds, calls tools, edits files, runs commands. The session can be exited and resumed; transcripts are stored in `~/.claude/projects/<slug>/`.

## The default tool set

Out of the box, Claude Code has tools for:

- **File operations** — `Read`, `Write`, `Edit`, `Glob`, `Grep`.
- **Shell** — `Bash` (or `PowerShell` on Windows). Sandboxed by default.
- **Web** — `WebFetch`, `WebSearch`.
- **Task management** — `TodoWrite`, `TaskCreate`, `TaskUpdate`.
- **Sub-agents** — `Agent` (spawns a sub-agent with a chosen subagent type).
- **Plan mode** — `EnterPlanMode` / `ExitPlanMode` (read-only planning).
- **MCP tools** — any MCP server configured for the session.

The architect's first move on a new Claude Code project is often pruning or augmenting this set via configuration.

## settings.json — the configuration surface

Claude Code's behavior is configured via `settings.json` files at several scopes:

- **User-level**: `~/.claude/settings.json` — applies to every project.
- **Project-level**: `.claude/settings.json` — checked into the repo, shared across the team.
- **Personal project-level**: `.claude/settings.local.json` — local overrides, gitignored.

`settings.json` controls model selection, default system-prompt additions, allowed tools, MCP server configuration, hooks, keybindings, and several behavioral flags. An architect deploying Claude Code as a team tool uses the project-level settings to enforce standards (which tools allowed, which MCP servers wired in, which hooks run).

## Hooks: shell scripts as policy

Claude Code's hook system runs shell commands at lifecycle events:

- **PreToolUse** / **PostToolUse** — before/after tool execution.
- **UserPromptSubmit** / **Stop** — before user input is sent, after a turn ends.
- **SessionStart** / **SessionEnd** — at the boundaries of a session.

Hooks can inspect tool arguments, veto a tool call, modify the conversation, run external policies (lint, security scan, secret detection), and log to external systems. Because hooks are shell scripts, anything the OS can run is available.

This is how Claude Code expresses policy that the model itself cannot enforce: a hook can refuse `git push --force` to main, run `secret-scan` on every file write, or send each tool call to an audit log.

## CLAUDE.md — project-level standing instructions

A file named `CLAUDE.md` at the root of a project is read automatically into Claude Code's system prompt at session start. This is where project-specific context lives: build commands, test commands, code conventions, the user's preferred style, key files, gotchas. User-level `~/.claude/CLAUDE.md` applies across every project.

Well-tended `CLAUDE.md` files are one of the strongest leverage points for getting consistent, project-aware behavior from Claude Code. They are the project's institutional memory delivered to the model on every session.

## When to use Claude Code

Claude Code is a development tool, not a hosted product. It runs on the user's machine, with the user's credentials, against the user's code. Architectures that use Claude Code:

- **Internal developer tools.** Pair-programming, refactoring, codebase exploration.
- **Repo-scoped automation.** Triage, diff review, test generation, documentation.
- **Bootstrap and scaffolding.** Generating starter projects, applying templates.
- **Audit and review.** Running a sub-agent fleet over a codebase looking for issues.

Architectures that should not use Claude Code: hosted SaaS products (build directly on the SDK or API instead), serverless inference endpoints, anything where the user does not control the machine.
