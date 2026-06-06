# System Prompts and Message Roles

Every Claude conversation is structured around two distinct kinds of text: the **system prompt** and the **messages**.

## The system prompt

The **system prompt** is a single string passed in the `system` field of a Messages API request. It sits outside the `messages` array and is delivered to the model with a different framing than user-turn content. The system prompt is for **standing instructions**: rules, persona, output format, operating context, definitions of terms, and constraints that apply to the entire conversation.

A good system prompt is durable across many user turns. It does not contain the specific question being asked; it contains the framing within which any question will be answered. Examples of system prompt content:

- "You are a customer service agent for Acme Corp. Always greet the customer by name. Never disclose internal SKU codes."
- "You are an expert SQL query generator. Output only the SQL query, with no surrounding explanation."
- "The user is a junior developer learning Rust. Explain concepts using small, runnable examples."

System prompts may be quite long — several thousand tokens is normal in production. The system prompt is also a primary candidate for **prompt caching**, because it is reused unchanged across many calls.

## The messages array

The **`messages`** field is an array of message objects, each with a `role` and `content`. The conversation alternates between two roles:

- **`user`** — input from the application's user (or, in agentic settings, from the orchestrating program).
- **`assistant`** — output produced by Claude in previous turns of the same conversation.

A first call typically has a single `user` message. Subsequent calls in the same conversation append the model's previous assistant response and the next user turn, preserving the full conversation history. The model has no memory between API calls; each call must include the relevant history.

There is no `system` role in the messages array. System content belongs in the `system` field, not as a message.

## Content blocks

Each message's `content` is either a plain string (for simple text) or an array of **content blocks**. The block types include:

- **`text`** — a string of text. The most common type.
- **`image`** — a base64-encoded or URL-referenced image. Used for vision input.
- **`tool_use`** — produced by the assistant when it calls a tool. Contains the tool name, an input object, and an id.
- **`tool_result`** — provided by the application in a user-role message to return the result of a tool call referenced by id.
- **`thinking`** — extended reasoning blocks produced when extended thinking is enabled.

The block array structure is what lets the same conversation interleave text, images, and tool calls naturally.

## Turn order

The conversation alternates strictly: the first message must be `user`, the next must be `assistant`, and so on. The model's response is always an assistant message; the application's next call must therefore append at least one user message — typically containing either the user's next question or one or more tool results.

A common architectural mistake is to send two consecutive user messages, or to omit the assistant turns from history. Both produce malformed requests. The conversation history is the **state** of the dialogue; the API itself is stateless.
