# Observability and Reliability

A production Claude deployment without observability is a deployment running blind. The architect designs the observability surface alongside the application; retrofitting it under load is much harder.

## What to measure

A production Claude system generates many signals worth capturing.

- **Request and response logs.** Every API call with its inputs, outputs, usage, stop reason, and latency. The raw transcript.
- **Token metrics.** `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens` per request. Aggregated into per-user, per-endpoint, per-feature views.
- **Cost metrics.** Tokens translated to dollars; rolled up by use case; tracked against budgets.
- **Latency metrics.** TTFT and total time per request. Per model tier, per use case. Percentiles, not just averages.
- **Cache hit rate.** Cache reads / (reads + writes). A leading indicator of cost efficiency.
- **Tool-call metrics.** Frequency, latency, error rate, outcome per tool. The agent's footprint on the outside world.
- **Refusal rate.** Number of `stop_reason: refusal` responses per use case. A safety and product signal.
- **Stop reason distribution.** Healthy: mostly `end_turn`. Concerning: high rates of `max_tokens` (budgets too tight or outputs too verbose) or `tool_use` loops that never terminate.
- **Agent loop length.** Average and tail iterations per agent run.

## Tracing

Each user-visible request often expands into multiple API calls (an agent loop) and tool invocations. **Distributed tracing** with a trace ID per user request and spans for each API call and each tool call lets the architect:

- See the full sequence that produced any given outcome.
- Identify which step was the bottleneck.
- Correlate user-reported issues with system behavior.
- Debug agents that fail intermittently.

Without tracing, an intermittent agent failure is a needle in a haystack of disconnected logs.

## Eval-based monitoring

For production Claude applications, the architect maintains an **eval set** — a fixed list of test inputs with known good outputs — and runs it continuously against production.

- **Pre-release eval.** Every prompt change, model swap, or major code update runs against the eval set before shipping.
- **Production canary eval.** A subset of the eval runs against the live system on a schedule. Drift in scores signals a regression even when nothing has been deployed.
- **Drift dashboards.** Quality metrics, refusal rates, tool-call patterns plotted over time. Drift is information.

The Anthropic Console includes prompt evaluation tooling; bespoke deployments can build their own.

## Reliability patterns

Beyond observation, reliability requires explicit architecture.

### Idempotency

Tools that mutate state (send email, charge a card, create a record) must be safe to retry. Idempotency keys on the request, deduplication at the tool layer, and explicit "this was already done" returns are the standard mechanisms.

### Circuit breakers

When a tool fails repeatedly in a short window, opening a circuit breaker prevents the agent from hammering a failing dependency. The model is informed the tool is unavailable; the loop continues without it. Reset after a cooldown.

### Timeouts at every layer

Per-API-call timeout; per-tool timeout; per-agent-loop timeout; per-user-request timeout. Each layer's timeout is shorter than its caller's. A tool that takes 30 seconds inside an API call with a 10-second client timeout is a 100% failure even though the tool works.

### Graceful degradation

When a critical sub-system fails — an MCP server, a database, a downstream API — the agent ideally degrades to reduced functionality rather than complete failure. "I cannot access the calendar right now; based on what you told me earlier, I think you have a meeting at 3pm" is a better failure mode than crashing.

### Replay and re-execution

Storing transcripts of agent runs lets the architect re-execute them against new models, new prompts, or new tools. This is invaluable for debugging, regression testing, and improving prompts based on real-world failures.

## SLAs and SLOs

Production Claude systems usually expose service-level objectives:

- **Latency SLO.** P95 response time under N seconds.
- **Availability SLO.** Successful response rate over a window.
- **Quality SLO.** Eval-set score above a threshold.

These are not Claude-specific; they are standard service reliability targets applied to a Claude-backed system. The architect designs against them, monitors them, and budgets remediation work against them.

## The reliability discipline

Reliable Claude applications are not built by hoping the model behaves well. They are built by **bounding the consequences** of model misbehavior, **measuring** every dimension that matters, and **responding** when measurements drift. The model is one component in a system designed to be operable.

The CCA-F architect understands that the certification's "Reliability" domain is not a vague safety appeal; it is the discipline of building systems that work in production over time.
