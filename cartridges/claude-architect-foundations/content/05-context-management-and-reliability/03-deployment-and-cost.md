# Deployment Architecture and Cost

Building a Claude application that works in a laptop demo differs from operating one that serves users at scale. The architect's deployment choices affect every dimension that matters: cost, latency, reliability, and operability.

## Model selection by tier

The Haiku / Sonnet / Opus tiering is not merely a pricing matter. The architect's task is to pick the cheapest tier that satisfies the quality target for each use case.

- **Haiku** — high-volume classification, routing, simple extraction. The cost difference at scale is large.
- **Sonnet** — general-purpose agent work, coding, RAG, structured output. The default.
- **Opus** — hard reasoning, long-horizon agentic work, novel problem solving. When quality justifies the cost.

A common production pattern is **model routing**: a cheap classifier (Haiku) decides which subsequent model to invoke. Easy queries handled end-to-end by Haiku; hard queries escalated to Sonnet or Opus. Substantial savings; latency improves too.

## Cost optimization

The architect's cost levers, in approximate order of impact:

1. **Prompt caching.** Typically the largest single optimization.
2. **Right-sizing the model.** Using Opus where Sonnet would do is the second-largest avoidable cost.
3. **Capping `max_tokens`.** A request with `max_tokens: 32000` and a 200-token actual response is billed for the 200; a runaway response that fills the budget is expensive. Cap honestly.
4. **Compaction.** Long agentic conversations grow until compacted. Summarize earlier turns to trade fidelity for cost.
5. **Batch processing.** When latency is not critical, the **Message Batches API** processes large request volumes asynchronously at approximately half the standard rates.
6. **Stop sequences.** When the application can predict where the model's output should end, a `stop_sequence` prevents over-generation.

## Streaming

For interactive applications, the Messages API supports **streaming** by setting `stream: true`. The response arrives as Server-Sent Events. Streaming improves perceived latency dramatically — the user sees the first tokens within hundreds of milliseconds rather than waiting for the full response.

Essential for chat UIs. Less useful for batch processing or for agent loops that wait for the full response before deciding the next step.

## Latency budgets

A request's wall-clock latency is dominated by:

- **Time to first token (TTFT).** Affected by input length, model size, current backend load.
- **Output generation rate.** Tokens per second. Smaller models stream faster.
- **Tool execution time.** In agent loops, time in tools dominates loop wall-clock when tools are slow.
- **Network round-trips.** Many short tool calls in sequence accumulate round-trip cost.

Benchmark realistic TTFT and throughput at production prompt size before committing to a tier. The same prompt on Haiku and Opus has materially different latency.

## Rate limits and retries

The API enforces rate limits per workspace on request frequency and tokens per minute. Limits scale with usage tier. Production applications handle `429 Too Many Requests` with exponential backoff. Official SDKs implement this automatically; bespoke clients must implement it explicitly.

Beyond rate-limit handling, idempotency matters: a retried request should not produce a duplicate side effect. For tool-calling agents this is usually the application's responsibility (tools should be designed idempotent where possible) rather than the API's.

## Multi-region considerations

For applications with global users, latency to the API matters. The Anthropic API is hosted in specific regions; **Bedrock** and **Vertex AI** offer Claude in additional regions and can be the right choice when users concentrate outside the Anthropic-hosted regions. The model behavior is the same; the request format is similar but not identical.

The architect should know all three deployment surfaces — Anthropic API, Bedrock, Vertex AI — and their region maps.

## Capacity planning

Several questions to answer before launch:

- **Peak QPS?** What rate-limit tier is needed.
- **Average tokens in / out per request?** What the cost per request will be.
- **Cache hit rate target?** Affects effective per-request cost.
- **Tail latency target?** Affects model and streaming choice.
- **Acceptable error rate?** Affects retry policy.

Operating without these numbers is operating blind. The architect's design includes capacity planning, not just functional correctness.
