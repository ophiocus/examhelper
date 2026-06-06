# Structured Outputs

Many applications need Claude's output in a specific shape — a JSON object, a list of records, a schema-conforming response — that downstream code can process. There are several techniques. The architect should know all of them.

## Tool-call structured output

The most reliable technique: **define a tool whose input schema is the desired output shape** and force it via `tool_choice: { type: "tool", name: "..." }`.

The model is constrained at the API layer to produce a JSON object validating against the schema. The application reads the result from the `input` field of the `tool_use` block — no parsing of free text, no risk of trailing prose, no missing fields.

This pattern is so common it has a name: **forced-tool structured output**. It is the recommended technique for any production application that needs reliable structured data.

Example schema for extracting a person record:

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

## Prefill the assistant turn

A lighter-weight technique: include an `assistant` message whose content begins with `{` (for a JSON object) or `[` (for an array). The model is constrained to continue from that opening character. With a small instruction in the user message, this usually produces clean JSON.

Trade-off vs. tool calls: prefill does **not** validate against a schema. The application parses and validates manually. Fine for simple shapes; less robust for nested or complex schemas.

## XML output

For cases where free text and structured fields need to interleave, **XML tags** are a reliable middle ground:

> "When listing items, wrap each item's name in `<item>` tags and each item's description in `<description>` tags."

XML output is parseable with standard libraries, robust against the model emitting surrounding prose, and allows mixed content (narrative paragraphs interleaved with structured items) more naturally than JSON.

## Extended thinking and structured output

When **extended thinking** is enabled, the model produces a thinking block before its visible response. Structured output techniques still work — the structured response follows the thinking block — but the application should **retain the thinking block in the conversation history** if subsequent calls depend on prior reasoning. The thinking is not counted toward the visible response.

## Schema design for the model

Whatever the technique, schemas should be designed for the model's strengths:

- **Descriptions are read by the model.** Every field's `description` is a prompt fragment. Use it.
- **Enums constrain at the schema layer.** Vastly more reliable than free-text fields with hopeful instructions.
- **Required vs. optional.** Mark fields the application depends on as required; leave the rest optional.
- **Avoid deeply nested schemas.** Two levels usually fine; five levels is a problem.
- **Use format hints.** `"format": "date"`, `"format": "email"`, `"pattern": "^[A-Z]{3}$"` — Claude respects them.

The goal: maximize the model's opportunity to succeed by removing ambiguity from the specification.

## Validation as policy

Schema validation at the API layer catches obvious shape errors but not semantic errors. A schema that says "age is an integer 0–150" does not prevent the model from emitting `age: 42` for a person who is clearly described as 7. Post-output validation belongs in the application:

- **Cross-field consistency checks.** Does the model's data match what was in the input?
- **Sanity bounds.** Are the numbers in plausible ranges?
- **Source citations.** Did the model cite a source for each claim? Are the citations real?
- **Schema-level enforcement.** Even though the API enforces the schema, an extra validation pass at the application layer is cheap insurance.

Structured output is reliable; semantically correct output requires more than schema.
