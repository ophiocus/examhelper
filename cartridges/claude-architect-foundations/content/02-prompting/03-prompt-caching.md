# Prompt Caching

**Prompt caching** is an API feature that lets Claude reuse the computational work of processing repeated prompt prefixes. When a request reuses a prefix the model has already seen, the cached portion is read from storage at a fraction of the cost and latency of fresh processing.

For an architect, prompt caching is one of the most impactful optimizations available — both for cost and for response time. Production agentic applications without caching are leaving large amounts of money on the table.

## How it works

The caller marks one or more **cache breakpoints** in the request by attaching `cache_control: { type: "ephemeral" }` to a content block. When Anthropic's servers receive the request, they hash the prompt prefix up to each breakpoint and compare it against the workspace's prompt cache. A match is a **cache read**; the matched tokens are charged at a reduced rate. A miss is a **cache write** — the new prefix is stored — and is charged at a slightly higher rate than uncached input.

The cache has a **time-to-live**. Standard prompt caching is five minutes; **extended caching** is one hour. The TTL is refreshed on each cache hit, so frequently-reused prefixes stay warm.

## Pricing

The three rates that matter:

- **Cache write** — roughly **1.25×** the base input token price. Pay this once when a new prefix is first stored.
- **Cache read** — roughly **0.1×** the base input token price. Pay this on every subsequent request that hits the cache.
- **Extended (1h) cache write** — roughly **2×** the base input token price. Pay this once for the longer TTL.

The economics: if the same prefix is reused even a few times, caching is a large net win. A 10,000-token system prompt that would cost X per request uncached costs about 1.25X to first cache and then about 0.1X per subsequent request — an order-of-magnitude reduction.

## Where to place breakpoints

A request may include up to **four cache breakpoints**. Common placements:

1. **End of system prompt.** Cache the standing instructions, persona, and operating rules.
2. **End of tool definitions.** When using tool use, cache the full tool definitions array.
3. **End of long context.** When supplying a large document for QA, cache the document.
4. **End of a few-shot examples block.** Cache the worked examples.

The principle: cache **everything that does not change** between adjacent requests, and leave the changing portion (the user's actual question, the latest agent step) uncached.

## What invalidates the cache

Any change in tokens **before or at** a breakpoint invalidates that breakpoint. The cache is prefix-based: the model can only resume from a point if the entire prefix up to that point is bit-identical to a cached entry.

Common mistakes that silently disable caching:

- Including a timestamp in the system prompt that changes every call.
- Including the user's name early in the prompt; the prefix differs per user.
- Adding tool definitions in a different order; tool definitions are part of the cached prefix.

The architect's discipline: keep dynamic content at the **tail** of the request, after all breakpoints.

## Agentic loops and caching

In a long agentic conversation, each turn adds new content to the end of `messages`. If the system prompt and tool definitions are cached, every turn of the loop hits the cache for the system+tools prefix and only the growing conversation history is processed fresh. By placing a cache breakpoint at the end of the latest assistant message each turn, the conversation history itself accumulates in the cache, so each new step pays cache-read rates for almost the entire context.

This is why agentic applications benefit disproportionately from caching, and why every production Claude agent should be designed with caching in mind from the start.
