# Context Windows and Compaction

The **context window** is the total token budget available for a single API call: system prompt, tools, message history, and the response itself all share it. Once the window is exhausted, the application must do something. The strategies for *what* it does are the architecture of long-running Claude work.

## Context windows in practice

Recent Claude models have context windows of **200,000 tokens** by default, with some configurations available at **1,000,000 tokens**. A token is roughly 4 characters of English text. 200K tokens is about 500 pages of dense prose.

Even with 200K, real applications hit the wall:

- A long agentic loop accumulates tool calls and results.
- A document-analysis task wants to load a whole codebase or knowledge base.
- A multi-day conversation accumulates history.
- Sub-agent outputs flow back into the parent's context.

The architect's job is to manage this budget explicitly. A naive long-running agent will fail when it hits the limit; a well-architected one will keep working.

## The token accounting

Every API call's `usage` field reports:

- **`input_tokens`** — tokens in the request (system + tools + history + current user message).
- **`output_tokens`** — tokens in the response.
- **`cache_creation_input_tokens`** — input tokens stored in the cache this call.
- **`cache_read_input_tokens`** — input tokens served from the cache this call.

Sum input + cache_read across a session and the architect sees the total context budget the agent is consuming.

## Strategies for the long run

### Compaction

The standard technique for in-place mitigation: at some threshold (say, 80% of the window), the application summarizes the early portion of the conversation, replaces it with the summary, and continues. The summary is shorter than what it replaces; the agent has reclaimed budget.

Claude Code performs compaction automatically when the user invokes `/compact`. The Agent SDK exposes compaction hooks for custom strategies.

Trade-offs: compaction loses detail. The summary captures the high-level shape; specific tool results, intermediate code, partial findings disappear. For tasks where the model needs verbatim recall of early work, compaction is the wrong choice.

### Sub-agent delegation

The alternative: spawn a sub-agent for a self-contained task, let it work in its own fresh context, return only its final report. The sub-agent's verbose intermediate work — file reads, tool calls, exploration — never enters the parent's context.

For tasks that can be cleanly bounded ("read these files and summarize", "search for X and report"), delegation is cleaner than compaction. The parent never grows; the sub-agent dies with its bloat.

### Context pruning

A coarser approach: drop earlier turns wholesale once they're no longer relevant. Works for stateless tool-using agents where each step is independent. Doesn't work when later steps need context from earlier ones.

### Window stretching

Anthropic's 1M-token context configurations let some applications avoid these strategies entirely — just keep growing the conversation. This is the right answer for some workloads (whole-codebase analysis where the model needs random access to everything). It costs more per call and runs slower; it doesn't replace compaction, but it raises the threshold.

## Designing for context budget

Architectural decisions that affect context consumption:

- **System prompt size.** Every token spent on standing instructions is a token not available for work. Trim aggressively.
- **Tool definitions.** Each tool's description and schema is in the context. Removing unused tools wins context.
- **Document attachment.** A 50K-token document attached to every call is 50K of context spent. Consider retrieval over attachment for large knowledge bases.
- **History granularity.** Every assistant turn includes the model's verbose explanations. A custom agent can strip these in the history it sends back, retaining only the essential decisions.
- **Sub-agent return shapes.** A sub-agent returning a 10K-token report consumes 10K of the parent's context. Structured, minimal returns are cheaper.

## Reliability follows from context management

A reliable long-running agent is one that knows what is in its context, manages the budget explicitly, and degrades gracefully when limits approach. An unreliable agent treats the context as unlimited until it isn't.

The architect's discipline: instrument context usage from day one, set budget thresholds, choose a compaction or delegation strategy explicitly, and test the agent under conditions that exercise long-running behavior. An agent that works fine for ten turns and falls over at fifty has not been architected; it has been hoped.
