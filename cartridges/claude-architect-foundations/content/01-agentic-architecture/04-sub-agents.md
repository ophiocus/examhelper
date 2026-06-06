# Sub-agents and Delegation Patterns

A **sub-agent** is a child agent invoked by a parent agent for a specific task. The sub-agent has its own system prompt, its own tool set, its own conversation history, and returns a final result that the parent receives and incorporates.

Sub-agents are the architectural answer to two recurring problems: **context exhaustion** and **specialization**.

## Why sub-agents

**Context exhaustion.** A long-running agent accumulates conversation history that eventually exceeds the context window. Compaction (summarizing earlier turns) trades fidelity for budget. The alternative is delegation: spawn a sub-agent for a self-contained task, let it work in its own fresh context, return only its final report. The sub-agent's verbose intermediate work — file reads, tool calls, exploration — never enters the parent's context.

**Specialization.** A monolithic agent with twenty tools and three personas is harder to prompt well than a focused agent with five tools and one persona. Sub-agents let an architect decompose a complex problem into smaller, focused, independently testable units. Each sub-agent's prompt can be tuned without affecting the others.

## Sub-agent shape

Each sub-agent declares:

- **A system prompt** — often more specialized than the parent's.
- **A tool set** — may be narrower or broader than the parent's. A search-only sub-agent might have only read tools; a build sub-agent might have shell access the parent does not.
- **A return contract** — what the parent expects back, typically a structured object or a summary string.
- **Optional isolation** — for sub-agents that mutate state (filesystem, database), running each in a fresh worktree or sandbox prevents interference.

## Common delegation patterns

### Explore-then-act

A general-purpose agent often spawns a focused **search sub-agent** to find relevant context, then incorporates the search results into its own decision-making. This keeps the parent's context lean — only the final search results enter it, not every grep result and file read.

Claude Code's built-in "Explore" agent is the canonical instance.

### Fan-out / map

For a list of N items to process, the parent spawns N parallel sub-agents, each handling one item, then collects their results. This is the standard pattern for batch processing, parallel review across a codebase, or running the same analysis on many inputs.

### Pipeline

For multi-stage work where each stage depends on the previous, the parent runs each item through a sequence of sub-agent stages. A pipeline differs from a barrier-style fan-out: items can move through the stages independently, so the slowest item does not block faster ones.

### Adversarial verification

A finding produced by one sub-agent is verified by N independent sub-agents, each prompted to refute it. Only findings that survive a majority of refutation attempts are reported. This pattern reduces false positives in research, code review, and audit work.

### Tournament / judge panel

For tasks with no single correct answer (design proposals, written content), spawn N independent sub-agents to produce variants, then a judge sub-agent to score and select. The architect designs the scoring rubric; the judge applies it.

## Sub-agent budget control

Sub-agents inherit budget concerns from the parent. A sub-agent that loops forever takes the parent down with it. Production deployments enforce:

- **Per-sub-agent turn cap.**
- **Shared token budget pool.** All sub-agents draw from a single pool; one greedy sub-agent doesn't starve the others, but the pool as a whole is bounded.
- **Concurrency caps.** Spawning a hundred sub-agents at once may exhaust local CPU, API rate limits, or downstream services.
- **Timeout per sub-agent.**

## Worktree isolation

For sub-agents that **mutate the filesystem** — write code, edit configs, generate artifacts — running them in parallel against the same checkout produces merge conflicts and lost work. The standard mitigation is **per-sub-agent worktrees**: each sub-agent gets a fresh `git worktree`, runs its work in isolation, and the parent merges or selects the final result.

The Agent SDK supports this as a configuration option (`isolation: "worktree"` in workflow scripts). It costs a few hundred milliseconds of setup per sub-agent — a small price when the alternative is corrupted state.
