# Tool Use Basics

**Tool use** (also called *function calling*) is the API feature that lets Claude invoke functions defined by the application. Tools are the foundation of agentic work, structured output, retrieval-augmented generation, and any application that needs to interact with external systems.

## Defining a tool

A tool is defined as a JSON object with three fields:

- **`name`** — a stable identifier, typically lowercase with underscores. For example, `get_weather`, `search_codebase`, `send_email`.
- **`description`** — a natural-language explanation of what the tool does, when to use it, and what its arguments mean. Claude reads this to decide *whether* to call the tool. A vague description produces unreliable tool selection.
- **`input_schema`** — a JSON Schema object describing the tool's arguments. Each argument has a name, a type, and (importantly) a description explaining what it represents.

The tools are passed in the `tools` field of the Messages API request as an array.

## The tool-use turn

When Claude decides to call a tool, the API response has `stop_reason: "tool_use"` and the assistant message's content array includes one or more `tool_use` blocks. Each block has:

- **`type: "tool_use"`**
- **`id`** — a unique identifier for this specific call (e.g. `toolu_01ABC...`)
- **`name`** — the name of the tool being called
- **`input`** — the arguments the model is passing, validated against the input schema

The application receives this response, executes the tool with the given input, and returns the result by appending a **user-role message** whose content includes a `tool_result` block:

```
{
  "type": "tool_result",
  "tool_use_id": "toolu_01ABC...",
  "content": "The current temperature in Bogotá is 16°C."
}
```

The `tool_use_id` matches the id of the corresponding tool_use block. The application then calls the Messages API again with the updated conversation; Claude reads the tool result and decides on the next step — either calling another tool, or producing a final text response.

## Parallel tool calls

A single assistant turn may contain **multiple `tool_use` blocks** if Claude decides several tools should be called at once. The application is expected to execute them in parallel and return all results in a single user message that contains one `tool_result` block per call. This is the canonical pattern for batched lookups, multi-source retrieval, and "fan-out" steps in an agent.

To encourage parallel calls, the system prompt can explicitly instruct Claude to issue multiple tool calls when the steps are independent. By default, recent Claude models will issue parallel calls when the work is clearly parallelizable.

## Tool choice control

The `tool_choice` field in the request controls Claude's tool-use behavior:

- **`{ "type": "auto" }`** (default) — Claude decides whether to call a tool or respond with text.
- **`{ "type": "any" }`** — Claude must call some tool, but may pick which one.
- **`{ "type": "tool", "name": "X" }`** — Claude must call tool `X` next.
- **`{ "type": "none" }`** — Claude may not call any tool; it must respond with text.

Forcing a specific tool is the standard pattern for **structured output**: define a tool whose input schema is the desired output shape, force the model to call it, and read the validated JSON from the tool input. This guarantees a parseable response even when the model has nothing more to add.

## The agent loop

The basic agent loop is:

1. Send the conversation (system prompt, tool definitions, message history) to the API.
2. Receive the response.
3. If `stop_reason` is `end_turn`, the agent has finished; return the assistant's text to the user.
4. If `stop_reason` is `tool_use`, execute each `tool_use` block, append a user message with corresponding `tool_result` blocks, and loop back to step 1.

This loop is the heart of every Claude-based agent. The Agent SDK wraps this loop for the common cases; the underlying API behavior is the same.
