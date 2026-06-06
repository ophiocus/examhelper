# System Prompts and Message Structure

Every Claude conversation has two kinds of text: the **system prompt** and the **messages**. The architect's first task in any new application is deciding what goes in each.

## The system prompt

The **system prompt** is a single string passed in the `system` field. It sits outside the `messages` array. It is for **standing instructions**: rules, persona, output format, operating context, definitions of terms, constraints that apply to the entire conversation.

A good system prompt is durable across many user turns. It does not contain the specific question being asked; it contains the framing within which any question will be answered. Examples:

- "You are a customer service agent for Acme Corp. Always greet the customer by name. Never disclose internal SKU codes."
- "You are an expert SQL query generator. Output only the SQL query, with no surrounding explanation."
- "The user is a junior developer learning Rust. Explain concepts using small, runnable examples."

System prompts are typically several hundred to several thousand tokens in production. They are a primary candidate for **prompt caching** because they are reused unchanged.

## The messages array

The **`messages`** field is an array of message objects, each with `role` (`user` or `assistant`) and `content`. The conversation alternates strictly:

- **`user`** — input from the application's user, the orchestrating program, or tool results.
- **`assistant`** — output produced by Claude in previous turns.

There is no `system` role in the messages array. The conversation must start with `user`, alternate, and continue with `user` after every assistant turn (typically containing the user's next question or tool results).

## Content blocks

Each message's `content` is either a plain string or an array of **content blocks**:

- **`text`** — a string of text.
- **`image`** — base64-encoded or URL-referenced image. Used for vision input.
- **`tool_use`** — produced by the assistant when it calls a tool.
- **`tool_result`** — produced by the application in a user message to return a tool's output.
- **`thinking`** — extended reasoning, produced when extended thinking is enabled.

The block-array structure lets a single message interleave text, images, and tool interactions naturally.

## Architectural placement decisions

For each piece of context an application has to give the model, the architect decides:

- **System prompt or first user message?** Standing instructions in system; specific question in user.
- **Per-turn or once?** Content that never changes belongs in system. Per-turn changing content belongs in the user message.
- **Cacheable?** If many users share the same prompt prefix (same system, same tools), put their shared content first and cache it; put per-user content at the tail.
- **Sensitive?** API keys, secrets, raw PII don't belong anywhere in the prompt. If a tool needs them, retrieve them inside the tool, not in the prompt.

These decisions, made well, produce systems that are cheap, fast, and consistent. Made badly, they produce caching no-ops, inconsistent personas, and accidentally-leaked secrets.

## Extended thinking

Recent Claude models support **extended thinking** as a first-class request parameter: `thinking: { type: "enabled", budget_tokens: N }` enables a dedicated thinking block before the visible response. The thinking is counted separately in usage and can be retained or discarded by the application.

Extended thinking is most valuable for hard reasoning tasks (math, multi-step planning, complex inference). It is unnecessary for classification, simple extraction, or formatting tasks. The architect's call is when the budget for thinking is worth the latency and cost it adds.
