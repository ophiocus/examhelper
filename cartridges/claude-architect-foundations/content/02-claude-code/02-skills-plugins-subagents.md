# Skills, Plugins, and Sub-agents in Claude Code

Three extension mechanisms turn Claude Code from a fixed agent into a configurable platform: **Skills**, **plugins**, and **sub-agents**. An architect uses these to package institutional knowledge, share team standards, and decompose complex workflows.

## Skills

A **Skill** is a reusable, named bundle of instructions and (optionally) supporting files that Claude Code can load on demand. A skill lives in a directory under `~/.claude/skills/<skill-name>/` (user-level) or `.claude/skills/<skill-name>/` (project-level), with a `SKILL.md` file as its entry point and arbitrary supporting files alongside.

`SKILL.md` declares, in frontmatter, the skill's name, description, and trigger conditions, and then provides the instructions Claude Code follows when the skill is invoked. Skills are addressable by name and can also be auto-triggered by keyword or context.

### What skills are good for

- **Workflow templates.** "Set up a new Drupal module", "scaffold a Rust app from skeleton", "open a PR with our standard template."
- **Domain knowledge.** "Format a financial report this way", "use these specific conventions for medical content."
- **Tool wrappers.** "Use this CLI tool with these flags for this kind of task."
- **Persona shifts.** "Act as the code reviewer for this codebase."

The architectural value: a skill encapsulates a *recipe* that the team agrees on once and reuses everywhere. The recipe lives in code, is versioned, and is invokable by name.

## Plugins

A **plugin** is a packaged unit that bundles skills, MCP servers, hooks, and configuration into a single installable artifact. Plugins are how the Claude Code ecosystem distributes capability: an organization, vendor, or community publishes a plugin; users install it; Claude Code is now configured to do something new.

Plugins are installed and managed via the Claude Code plugin system. A plugin manifest declares what it contributes:

- Skills it ships.
- MCP servers it provides or wires up.
- Hooks it registers.
- Default settings it sets.

For an architect, plugins are the unit at which capability is **packaged for distribution**. A team's Claude Code standards — internal MCP servers, security hooks, project skills — are bundled as a plugin and installed once per developer machine.

## Sub-agents in Claude Code

Claude Code supports first-class sub-agents. A sub-agent is declared in a markdown file under `~/.claude/agents/<name>.md` (user-level) or `.claude/agents/<name>.md` (project-level), with frontmatter declaring its name, description, system prompt, allowed tools, and model.

When the main Claude Code session invokes the `Agent` tool, it can pass a `subagent_type` parameter to select a specific sub-agent. The selected sub-agent runs with its own configured prompt, tool set, and (potentially) model, returns a final message, and the main session continues.

### Well-known built-in sub-agents

Claude Code ships with several:

- **Explore** — a fast, read-only search agent optimized for finding files and references.
- **general-purpose** — the default, used when no specific subagent_type is given.
- **Plan** — designs implementation plans, returns a structured step-by-step.

### Custom sub-agents

Architects write custom sub-agents for recurring delegated tasks: a code-reviewer agent, a test-writer agent, a documentation agent. Each has its own prompt tuned for its job and a narrower tool set that reduces selection errors.

## Composition

Skills, plugins, sub-agents, hooks, and MCP servers compose. A team's plugin can ship:

- A skill that defines the team's PR workflow.
- A custom sub-agent for code review.
- An MCP server wired up to the team's internal issue tracker.
- A pre-commit hook that runs the team's linter.

The architect's task is to identify what should be shared (plugin), what should be per-project (`.claude/`), and what should be per-developer (`~/.claude/`). Misplacing these — putting team standards in user-level config, or personal preferences in project config — leads to inconsistency that compounds.
