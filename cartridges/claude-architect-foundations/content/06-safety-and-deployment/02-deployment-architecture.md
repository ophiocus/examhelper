# Deployment Architecture

Building an application on Claude that works in a laptop demo is different from operating one that serves users at scale. The foundations exam expects familiarity with the architectural choices that matter once Claude leaves development.

## Model selection by tier

The Haiku / Sonnet / Opus tiering is not merely a pricing matter. The architect's task is to pick the cheapest tier that satisfies the quality target for each use case.

- **High-volume classification, routing, simple extraction** — Haiku is almost always sufficient. The cost difference at scale is large.
- **General-purpose agent work, coding, retrieval-augmented generation, structured output** — Sonnet is the default.
- **Hard reasoning, long-horizon agentic work, novel problem solving** — Opus when quality justifies the cost; Sonnet when the budget does not stretch.

A common production pattern is **model routing**: a cheap classifier (Haiku) decides which subsequent model to invoke. Easy queries are handled by Haiku end-to-end; hard queries are escalated to Sonnet or Opus. The savings are substantial; the latency improvement is also real.

## Cost optimization

The architect's cost levers, in approximate order of impact:

1. **Prompt caching.** As discussed in the prompting section, this is typically the largest single optimization.
2. **Right-sizing the model.** Using Opus where Sonnet would do is the second-largest avoidable cost.
3. **Capping `max_tokens`.** A request with `max_tokens: 32000` and an actual response of 200 tokens is billed only for the 200; but a runaway response that fills the budget can be expensive. Cap honestly.
4. **Compaction.** Long agentic conversations grow until compacted. Strategically summarizing earlier turns trades fidelity for cost.
5. **Batch processing.** When latency is not critical, the **Message Batches API** processes large numbers of requests asynchronously at approximately half the standard input/output rates.
6. **Stop sequences.** When the application can predict where the model's output should end (a closing tag, a sentinel string), supplying it as a `stop_sequence` prevents over-generation.

## Streaming

For interactive applications, the Messages API supports **streaming** by setting `stream: true`. The response arrives as Server-Sent Events: a sequence of `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, and `message_stop` events. Streaming improves perceived latency dramatically — the user sees the first tokens within hundreds of milliseconds rather than waiting for the full response.

Streaming is essential for chat UIs. It is less useful for batch processing or for agent loops where the application waits for the full response before deciding the next step.

## Latency budgets

A request's wall-clock latency is dominated by:

- **Time to first token (TTFT).** Affected by input length, model size, and current backend load.
- **Output generation rate.** Tokens per second. Smaller models stream faster.
- **Tool execution time.** In agent loops, the time spent in tools dominates the loop wall-clock when tools are slow.
- **Network round-trips.** Many short tool calls in sequence accumulate round-trip cost.

An architect should benchmark the realistic TTFT and throughput of each model at the production prompt size before committing to a tier. The same prompt on Haiku and on Opus has materially different latency.

## Rate limits and retries

The API enforces rate limits per workspace, both on request frequency and on tokens per minute. Limits scale with usage tier. A production application handles `429 Too Many Requests` responses with exponential backoff. The standard SDKs implement this automatically; bespoke clients must implement it explicitly.

Beyond rate-limit handling, idempotency matters: a retried request should not produce a duplicate side effect. For tool-calling agents this is usually the application's responsibility (tools should be designed idempotent where possible) rather than the API's.

## Observability

A production Claude deployment generates:

- **Request and response logs.** Every API call with its inputs, outputs, usage, and stop reason.
- **Cost metrics.** Tokens in, tokens out, cache reads, cache writes — converted to dollars.
- **Latency metrics.** TTFT and total time per request, per tier, per use case.
- **Tool-call metrics.** Frequency, latency, error rate, and outcome per tool.
- **Refusal metrics.** As discussed, refusals are a useful signal.

Operating without these metrics is operating blind. The architect's design should include observability from the start; retrofitting it under load is much harder.

## Multi-region considerations

For applications with global users, latency to the API matters. The Anthropic API is hosted in specific regions; **Bedrock** and **Vertex AI** offer Claude in additional regions and can be the right choice when the application's users are concentrated outside the Anthropic-hosted regions. The model behavior is the same; the request format is similar but not identical. An architect should know all three deployment surfaces and their region maps.
