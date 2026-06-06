# Tool Use Basics

**Tool use** (or *function calling*) is the API feature that lets Claude invoke functions defined by the application. Tools are the foundation of agentic work, retrieval-augmented generation, structured output, and any application that needs to interact with external systems.

## Defining a tool

A tool is a JSON object with three required fields:

- **`name`** — a stable identifier, lowercase with underscores. For example `get_weather`, `search_codebase`, `send_email`.
- **`description`** — a natural-language explanation of what the tool does, when to use it, and what its arguments mean. Claude reads this to decide *whether* to call the tool. Vague descriptions produce unreliable selection.
- **`input_schema`** — a JSON Schema object describing the tool's arguments. Each argument has a name, a type, and ideally a description.

Tools are passed in the `tools` array of the Messages API request.

## The tool-use turn

When Claude decides to call a tool, the response has `stop_reason: "tool_use"` and the assistant content includes one or more `tool_use` blocks. Each block has:

- **`type: "tool_use"`**
- **`id`** — a unique identifier for this call (e.g. `toolu_01ABC...`)
- **`name`** — the tool name
- **`input`** — arguments, validated against the input schema

The application executes the tool, then returns the result in a **user-role message** containing a `tool_result` block:

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01ABC...",
  "content": "The current temperature in Bogotá is 16°C."
}
```

The `tool_use_id` correlates the result with the call. The application then calls the API again with the updated conversation.

## Parallel tool calls

A single assistant turn may contain **multiple `tool_use` blocks**. The application executes them in parallel and returns all results in a single user message with one `tool_result` per call. This is the canonical pattern for fan-out steps and is essential for agent latency.

## Tool choice control

The `tool_choice` field controls Claude's behavior:

- **`{ type: "auto" }`** (default) — Claude decides whether to call a tool.
- **`{ type: "any" }`** — Claude must call some tool.
- **`{ type: "tool", name: "X" }`** — Claude must call tool X.
- **`{ type: "none" }`** — Claude may not call any tool.

Forcing a specific tool is the canonical pattern for **structured output**.

## Designing the tool description

The description is the single most important field. The architect's checklist for a good description:

- **What does the tool do?** State the action plainly.
- **When should it be called?** Describe the condition for use.
- **What does it return?** Describe the result shape.
- **What does it cost?** Note slow, expensive, or irreversible tools so Claude is cautious.

A vague tool description ("Does some search.") produces selection errors. A good description ("Searches the company's product catalog by SKU or product name; returns a list of matching products with name, price, and stock; takes ~200ms; safe to call repeatedly.") gives Claude enough information to use the tool well.

## Designing the input schema

Schema design is design work. The architect's principles:

- **Describe every field.** Each property's `description` is read by the model. A field with no description is a guess.
- **Use enums for closed sets.** `{"type": "string", "enum": ["low", "medium", "high"]}` is far more reliable than free-text urgency.
- **Mark required fields.** The `required` array forces inclusion.
- **Use minimum/maximum on numbers.** Tighter ranges prevent absurd values.
- **Avoid deep nesting.** Two levels usually fine; five levels is a problem. Flatten or split.
- **Avoid optional kitchen-sink fields.** A schema with twenty optional fields is harder for the model than one with five carefully chosen required fields.

## Tool errors

Tools fail. Network drops, validation errors, rate limits, unexpected nulls. The architect designs the tool's error path explicitly:

- **Return errors as `tool_result` with `is_error: true`.** The model recognizes this and can adapt.
- **Include the error message verbatim.** "Connection refused" tells the model something different than "Invalid argument."
- **Decide retry policy at the tool layer.** Some errors warrant the tool retrying internally; others should be returned for the model to decide.
- **Bound the model's retry behavior.** Without limits, the model may call a failing tool repeatedly. A pre-tool hook or post-tool circuit-breaker is the standard mitigation.

## Tool security

Tools that mutate state, send communication, or spend money are **privileged actions**. Production architectures wrap them:

- **Pre-tool hooks** for approval, audit, sanitization.
- **Allow lists** for arguments (no shell metacharacters; no arbitrary file paths).
- **Rate limiting** at the tool layer, independent of API limits.
- **Read-only modes** for development.

Trusting the model not to misuse a tool is a poor security posture. Wrap the tool with the controls the threat model demands.
