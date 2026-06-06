# Prompt Caching

**Prompt caching** is the architectural lever with the largest cost and latency impact in most Claude applications. An architect who is not using prompt caching well is leaving substantial money and performance on the table.

## How it works

The caller marks one or more **cache breakpoints** in the request by attaching `cache_control: { type: "ephemeral" }` to a content block. Anthropic's servers hash the prompt prefix up to each breakpoint and compare against the workspace's cache. A match is a **cache read** (charged at a reduced rate). A miss is a **cache write** (the new prefix is stored, charged slightly above the base rate).

The cache has a **time-to-live**: standard caching is **5 minutes**; **extended caching** is **1 hour**. The TTL resets on each cache hit, so frequently-reused prefixes stay warm indefinitely.

## Pricing

The three rates that matter:

- **Cache write** — about **1.25×** the base input token price. Paid once per new prefix.
- **Cache read** — about **0.1×** the base input price. Paid on every subsequent request that hits.
- **Extended (1-hour) cache write** — about **2×** the base input price. Paid once for the longer TTL.

The economics: a 10,000-token system prompt costs roughly 1.25× input to first cache and roughly 0.1× input on every subsequent call. After three or four hits, the average cost per call is an order of magnitude below uncached.

## Where to place breakpoints

A request may include up to **four cache breakpoints**. Standard placements, in priority order:

1. **End of tool definitions.** Tool definitions are part of the cached prefix; cache them.
2. **End of system prompt.** Standing instructions, persona, rules.
3. **End of long context.** A supplied document for QA, a knowledge base, an attached codebase summary.
4. **End of the latest assistant turn in a growing conversation.** This keeps the conversation history itself cached as the loop grows.

The principle: **cache what does not change between adjacent requests; leave dynamic content at the tail.**

## What invalidates the cache

Any change in tokens **before or at** a breakpoint invalidates that breakpoint. The cache is prefix-based; resumption requires the entire prefix up to the resume point be bit-identical.

Common silent cache disablers:

- **Timestamps in the system prompt.** Different on every call.
- **User names early in the prompt.** Different per user.
- **Tool definitions in changing order.** A reordered tool array is a different prefix.
- **Randomized session IDs in the prefix.**
- **Adding or removing tools dynamically per-request.**

The architect's discipline: keep dynamic content at the **tail**, after all breakpoints.

## Caching in agentic loops

In a long agentic conversation, each turn appends new content to the end of `messages`. If the system prompt and tools are cached, every turn hits cache for the system+tools prefix. By placing a cache breakpoint at the end of each assistant turn, the growing conversation history itself accumulates in the cache.

Net effect: each new step pays cache-read rates for nearly all the context, plus base rates only for the few hundred tokens of the latest user message. This is why agentic applications benefit disproportionately from caching, and why every production Claude agent should be designed with caching from the start.

## Caching across users

If multiple users share the same system prompt and tool definitions (a common pattern in product applications), they share the cached prefix. The cache lives at the workspace level; user A's cache write is user B's cache read.

This makes **broad-stroke architectural decisions** (one shared system prompt across all users? per-user personalization?) into cost decisions. A per-user system prompt fragments the cache and multiplies the write cost; a shared system prompt with per-user content at the tail consolidates the cache.

## Diagnosing cache effectiveness

The `usage` object in every response reports `cache_creation_input_tokens` and `cache_read_input_tokens`. An architect monitors:

- **Cache hit rate** — reads / (reads + writes). High is good.
- **Average tokens cached per request** — read counts indicate effective prefix length.
- **Tail length** — uncached input tokens per request. Should be small for established workloads.

A workload with low hit rate has a bug somewhere — usually dynamic content sneaking into the cached prefix. Finding and fixing it is one of the highest-leverage optimizations available.
