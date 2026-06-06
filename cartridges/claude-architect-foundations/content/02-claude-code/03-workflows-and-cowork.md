# Workflows, Cowork, and Headless Operation

Beyond interactive use, Claude Code supports three operating modes that an architect should understand: **workflows**, **Claude Cowork**, and **headless / scheduled** operation.

## Workflows

A **workflow** is a deterministic JavaScript script that orchestrates many Claude Code sub-agents in a structured way. The workflow author writes plain control flow (loops, conditionals, fan-out, pipeline stages) and calls `agent(prompt, opts)` to spawn each sub-agent.

Why a workflow rather than a single agent: when the structure of the work is known and the orchestration should be deterministic, a workflow gives the architect explicit control. Examples:

- **Codebase-wide review.** Fan out one sub-agent per file or per dimension, gather findings, dedup, verify each finding with adversarial sub-agents, synthesize a report.
- **Migration.** Discover every call site of a deprecated API, transform each in an isolated worktree, verify the changes compile and pass tests, merge the survivors.
- **Multi-stage research.** Parallel readers over relevant subsystems → structured map → judge panel → synthesis.

Workflows compose with all Claude Code primitives: each `agent()` call can pass a `subagent_type`, can request worktree isolation, can be given a structured-output schema, can attach to specific MCP servers.

The trade-off: writing a workflow is more work than just prompting Claude Code conversationally. The architect's call is when the work shape is fixed enough to be worth scripting versus when conversational guidance is sufficient.

## Claude Cowork

**Claude Cowork** is the more recent extension of Claude Code that focuses on working **alongside the user on real files and projects** rather than as a pure CLI agent. It introduces:

- **Task loops** — the agent works on a task, the user reviews, the agent iterates.
- **Plugins** — first-class plugin discovery and installation (Cowork generalized the plugin system).
- **Skills** — Cowork is the surface where Skills are most prominently exposed.
- **File workflows** — collaborative editing patterns that interleave user edits and agent edits with awareness of both.

For the CCA-F architect, Cowork is the answer to "how does Claude Code scale to a team where multiple humans and one or more Claude instances are touching the same project at the same time."

## Headless and scheduled operation

Claude Code can run **headlessly** — without an interactive terminal — via:

- **`claude -p "prompt"`** — non-interactive: send a prompt, get a response, exit. Useful in scripts, CI, and one-shot automations.
- **Scheduled tasks / cron** — Claude Code can be invoked from cron, GitHub Actions, or any scheduler. The MCP `mcp__scheduled-tasks` server formalizes this with manage-able scheduled jobs.
- **Remote triggers** — webhooks, Slack mentions, or other external events can trigger Claude Code runs.

Headless operation is what turns Claude Code from a personal tool into infrastructure. An architect designing automation around it considers:

- **Authentication.** API keys must be accessible in the headless environment.
- **Permission boundaries.** What can the headless agent do? `--dangerously-skip-permissions` exists but should be used judiciously; better to scope tools and MCP servers explicitly.
- **Observability.** Transcripts, exit codes, audit logs of headless runs.
- **Idempotency.** A scheduled job that runs every hour must not produce duplicate side effects.

## IDE integrations

Claude Code integrates with major IDEs (VS Code, JetBrains, Cursor) through extensions that surface the agent inside the editor: file context flows automatically, edits appear in diffs, the agent's terminal output is visible in a panel. From an architectural standpoint, the underlying CLI is unchanged — the IDE is a UI layer over the same `claude` process.

The architect's relevant decision is whether to **standardize the team on a particular surface** (everyone uses the same IDE integration; configurations and skills are shared) or to leave it heterogeneous (each developer picks their interface). The Claude Code engine works the same way regardless.
