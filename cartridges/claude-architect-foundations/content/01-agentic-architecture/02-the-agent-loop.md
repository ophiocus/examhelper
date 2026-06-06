# The Agent Loop

An **agent** is an application that uses Claude to choose and chain actions toward a goal. The defining feature is the **loop**: Claude is called repeatedly, and on each call it may either produce a final response or request another tool call.

## The canonical loop

In pseudocode:

```
messages = [user_request]
while True:
    response = claude.messages.create(
        model=model, system=system_prompt,
        tools=tool_definitions, messages=messages,
    )
    messages.append(response.message)
    if response.stop_reason == "end_turn":
        return response.message.text
    if response.stop_reason == "tool_use":
        tool_results = []
        for call in response.message.tool_uses:
            result = execute_tool(call.name, call.input)
            tool_results.append({
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": result,
            })
        messages.append({"role": "user", "content": tool_results})
```

The loop runs until Claude returns `stop_reason: "end_turn"`. The application is responsible for executing tools, returning results, and enforcing budgets.

## Loop control

A naive loop can run indefinitely. Production agents enforce explicit limits:

- **Max turn count.** After N iterations, terminate and return what has accumulated. 10–50 is typical for short agents; longer for deep work.
- **Token budget.** Track cumulative output tokens across the loop and stop when a budget is exhausted.
- **Wall-clock timeout.** Some tasks have a real-time SLA.
- **Repetition detection.** If the model calls the same tool with the same arguments in successive turns, it is likely stuck; intervene or terminate.
- **Cost budget.** Translate tokens to dollars and refuse to exceed a per-run cap.

These limits are architectural constants, not afterthoughts. An agent without them is not production-ready.

## Parallel tool calls

A single assistant turn may emit multiple `tool_use` blocks. The application executes them in parallel and returns all results in a single user message with one `tool_result` per call. This is the canonical pattern for fan-out steps (multiple independent lookups, batched retrieval, parallel API calls).

Pushing the model to use parallel calls when steps are independent is a meaningful latency win. Recent Claude models do this by default; older models often need an explicit nudge in the system prompt.

## Hooks: the architectural seams

Production agent loops have several **hook points** where the application can inspect, modify, or veto behavior:

- **Pre-call hook** — before each API call. Modify messages, add cache breakpoints, log.
- **Post-call hook** — after each API response. Inspect the response, gather metrics, detect refusals.
- **Pre-tool hook** — before executing a tool the model has called. Veto, require human approval, sanitize arguments.
- **Post-tool hook** — after a tool returns but before returning to the model. Sanitize results, redact secrets, log.

Hooks are the seam at which policy, audit, observability, and safety live. The Agent SDK exposes them as callbacks; Claude Code exposes them as configurable shell hooks in `settings.json`. Either way, an architect designs the hook surface alongside the agent itself.

## Failure modes

The canonical failure modes of agent loops are:

- **Infinite tool calls.** Mitigated by max-turn limit.
- **Context exhaustion.** Long conversations exceed the context window. Mitigated by compaction or sub-agent delegation.
- **Hallucinated tool calls.** The model invents tool names or arguments. Mitigated by tight tool definitions, schemas, and post-tool validation.
- **Silent failure.** The tool fails but returns a string that the model treats as success. Mitigated by structured error returns and explicit error-handling instructions in the system prompt.
- **Reward hacking.** The model finds a shortcut that satisfies the literal instruction but not the intent. Mitigated by careful prompt design and well-chosen termination conditions.

An architect explicitly designs around each of these. Naive implementations will encounter all of them in production.
