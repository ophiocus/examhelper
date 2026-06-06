# The Messages API

The Anthropic API's primary endpoint is **`POST /v1/messages`**. Every Claude interaction — single-turn classification, long agentic loop, tool-using conversation — is built on this one endpoint.

## Request structure

A messages request contains a small set of required fields and a larger set of optional ones.

The **`model`** field names the Claude model variant being called, for example `claude-sonnet-4-5-20250929` or `claude-opus-4-5-20250929`. The exact model identifier strings encode the variant and a release date.

The **`max_tokens`** field caps the number of tokens the model may produce in its response. Required. A typical value is between 1024 and 8192; large agent runs may need more.

The **`messages`** field is an array of message objects, each with a **`role`** (`user` or `assistant`) and a **`content`** field. The conversation alternates user and assistant turns. The model's reply will be a new assistant message that should be appended to the array on the next call.

The **`system`** field (optional but commonly used) holds the system prompt — the model's standing instructions, persona, rules, and operating context. The system prompt is not a message; it lives outside the `messages` array.

Other commonly used optional fields include **`temperature`** (sampling randomness, 0.0 to 1.0), **`top_p`** and **`top_k`** (alternative sampling controls), **`stop_sequences`** (strings that halt generation), and **`stream`** (whether to stream the response as Server-Sent Events).

## Response structure

The response is a JSON object with several useful fields.

The **`id`** is a unique message identifier. The **`type`** is always `"message"` for messages-endpoint responses. The **`role`** is always `"assistant"`. The **`content`** is an array of content blocks — typically text blocks, but also tool-use blocks, thinking blocks, and others, depending on the request.

The **`stop_reason`** explains why generation halted: `"end_turn"` (the model finished naturally), `"max_tokens"` (the cap was hit), `"stop_sequence"` (a stop sequence was emitted), `"tool_use"` (the model wants to call a tool), or `"refusal"` (the model declined the request).

The **`usage`** object reports token counts: `input_tokens`, `output_tokens`, and, when caching is in use, `cache_creation_input_tokens` and `cache_read_input_tokens`. These are the basis for billing.

## Authentication

Authentication is by **API key**, supplied as the `x-api-key` header. Keys are obtained from the Anthropic Console and are bound to a workspace; usage is billed against the workspace's payment method.

Additional required headers are **`anthropic-version`** (which pins the request to a specific API version, e.g. `2023-06-01`) and **`content-type: application/json`**.

## SDKs

Official SDKs wrap the REST API in idiomatic libraries:

- **Python:** `pip install anthropic`, used as `client.messages.create(...)`.
- **TypeScript / JavaScript:** `npm install @anthropic-ai/sdk`.
- **Java, Go, Ruby:** also officially supported.

The SDKs translate the same request fields and return objects with the same field structure. An architect should be comfortable reading and writing the raw JSON request body as well, because tool definitions, streaming events, and edge cases are sometimes more transparent at that level.

## Versioning

Claude API versions are pinned by the `anthropic-version` header. New versions are released alongside model-behavior changes; the header allows applications to opt in to a known version rather than break silently when defaults change. Production architects should pin the version explicitly.
