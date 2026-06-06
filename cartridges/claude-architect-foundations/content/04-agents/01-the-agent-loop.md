# The Agent Loop

An **agent**, in the Claude context, is an application that uses Claude to choose and chain actions toward a goal. The defining feature of an agent is the **loop**: Claude is called repeatedly, and on each call it may either produce a final response or request another tool call.

## The canonical loop

The basic structure, in pseudocode:

```
messages = [user_request]
while True:
    response = claude.messages.create(
        system=system_prompt,
        tools=tool_definitions,
        messages=messages,
    )
    messages.append(response.message)
    if response.stop_reason == "end_turn":
        return response.message.text
    if response.stop_reason == "tool_use":
        tool_results = []
        for tool_call in response.message.tool_uses:
            result = execute_tool(tool_call.name, tool_call.input)
            tool_results.append({
                "type": "tool_result",
                "tool_use_id": tool_call.id,
                "content": result,
            })
        messages.append({"role": "user", "content": tool_results})
```

The loop runs until Claude decides it has nothing more to do (`stop_reason: "end_turn"`). The application is responsible for:

- Executing the requested tools.
- Returning their results in the correct format.
- Detecting failure modes (infinite loops, repeated tool calls, refused tasks).
- Enforcing budgets (max turns, max tokens, max wall clock).

## Loop control

A naive loop can run indefinitely if Claude keeps requesting tool calls. Production agents enforce explicit limits:

- **Max turn count.** After N iterations of the loop, stop and return what has accumulated. 10–50 is typical for short agents; longer for deep work.
- **Token budget.** Track cumulative output tokens across the loop and stop when a budget is exhausted.
- **Wall-clock timeout.** Some tasks have a real-time SLA; the loop should be abortable.
- **Repetition detection.** If the model calls the same tool with the same arguments in successive turns, it may be stuck. The application can intervene with a clarifying user message or terminate.

## Compaction

Long-running agents accumulate large conversation histories that eventually exceed the context window. The standard mitigation is **compaction**: at some threshold, the application summarizes the early portion of the conversation, replaces it with the summary, and continues. Compaction trades fidelity for context budget. The Agent SDK provides hooks for this.

An alternative is **subagent delegation**: spawn a fresh subagent for a self-contained subtask, let it work in its own clean context, return only its final report to the parent. This bounds the parent's context growth without sacrificing the subtask's working detail.

## Sub-agents

A **sub-agent** is a child agent invoked by a parent agent for a specific task. Sub-agents have:

- Their own **system prompt**, often more specialized than the parent's.
- Their own **tool set**, which may be narrower or broader than the parent's.
- Their own **conversation history**, isolated from the parent's.
- A **return value** that the parent receives and incorporates into its own context.

Sub-agents are the architectural answer to context blowup and to specialization. They let an architect decompose a complex agent into smaller, focused, independently testable units. The Agent SDK supports sub-agents as first-class objects.

## Hooks and instrumentation

Production agent loops need observability. Common instrumentation points:

- **Pre-call hook.** Inspect the message being sent; log it; modify it.
- **Post-call hook.** Inspect the response; log it; gather metrics.
- **Pre-tool hook.** Inspect a tool call before execution; veto, modify, or short-circuit.
- **Post-tool hook.** Inspect the tool result before returning to the model; sanitize, transform, or log.

The Agent SDK exposes these as callbacks; Claude Code exposes them as configurable shell hooks. Either way, hooks are how a production agent gets policy, audit, and adaptation without modifying the model itself.

## Why this matters

The agent loop is the central architecture of every Claude-based application beyond single-shot Q&A. An architect who understands the loop — its control flow, its failure modes, its instrumentation surfaces, and its caching opportunities — can design systems that are reliable, observable, and economical. An architect who treats the loop as a black box is at the mercy of whatever the SDK happens to do.
