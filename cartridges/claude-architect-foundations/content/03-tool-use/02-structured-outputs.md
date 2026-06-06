# Structured Outputs

A common task is to extract from a model a result in a specific shape — a JSON object, a list of objects, a record matching a schema — that downstream code can process programmatically. There are several techniques for this with Claude. The architect should know all of them and choose by context.

## Tool-call structured output

The most reliable technique is to **define a tool whose input schema is the desired output shape** and force the model to call it via `tool_choice: { type: "tool", name: "..." }`.

The model is constrained by the API layer to produce a JSON object that validates against the schema. The application reads the result from the `input` field of the tool_use block — no parsing of free text, no risk of trailing prose, no missing fields.

This pattern is so common that it has a name: **forced-tool structured output**. It is the recommended technique for any production application that needs reliable structured data from Claude.

A schema for extracting a person record might look like:

```json
{
  "type": "object",
  "properties": {
    "name": {"type": "string", "description": "Full legal name"},
    "age": {"type": "integer", "minimum": 0},
    "occupation": {"type": "string"}
  },
  "required": ["name", "age"]
}
```

## Prefilling the assistant turn

A lighter-weight technique that avoids tool definitions: include an `assistant` message in the conversation whose content begins with `{` (for a JSON object) or `[` (for an array). The model is then constrained to continue from that opening character. With a small instruction in the user message ("Respond with a JSON object matching this schema..."), this usually produces clean JSON.

The trade-off versus tool calls: the prefill technique does not validate the output against a schema. The application must parse and validate manually. For simple shapes this is fine; for nested or complex schemas, the tool-call approach is more reliable.

## XML output

For cases where free text and structured fields need to interleave, **XML tags** are a reliable middle ground. The system prompt instructs the model to wrap the structured portions of the response in named tags:

> "When listing items, wrap each item's name in `<item>` tags and each item's description in `<description>` tags."

XML output is parseable with standard libraries and is robust against the model emitting surrounding prose. It also allows mixed content (narrative paragraphs interleaved with structured items) more naturally than JSON.

## Extended thinking and structured output

When **extended thinking** is enabled, the model produces a thinking block before its visible response. Structured output techniques work normally — the structured response follows the thinking block — but the application should retain the thinking block in the conversation history if it intends to make subsequent calls that depend on prior reasoning. The thinking is not counted toward the visible response.

## Schema design tips

Whatever the technique, the schema (or instructions) should be designed for the model's strengths:

- **Use descriptions liberally.** Every field's `description` is read by the model. A field named `urgency` with no description is a guess; a field named `urgency` described as `"How time-sensitive the request is. 'low' = within a week, 'high' = within an hour."` is far more reliable.
- **Use enums where applicable.** `{"type": "string", "enum": ["low", "medium", "high"]}` is far more reliable than a free-text field. The model is constrained at the schema layer.
- **Mark required fields.** The `required` array forces the model to include fields that the application's downstream code depends on.
- **Avoid deeply nested schemas.** Five levels of nesting is harder for the model than two. If the natural shape is deep, consider flattening or splitting into multiple calls.

The goal is to give the model the maximum opportunity to succeed by removing ambiguity from the specification.
