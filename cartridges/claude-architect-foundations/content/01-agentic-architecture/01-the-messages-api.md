# The Messages API and the Architecture Surface

Every Claude-based architecture is built on a single API endpoint: **`POST /v1/messages`**. Single-turn classification, long-running agent, multi-modal application — all are the same endpoint with different message contents. An architect's job begins with mastering this surface.

## Anatomy of a request

A messages request carries:

- **`model`** — the variant being called, e.g. `claude-sonnet-4-5-20250929`. The exact identifier encodes the model family and a release date.
- **`max_tokens`** — required cap on response length.
- **`messages`** — the conversation array, alternating `user` and `assistant` roles.
- **`system`** — standing instructions, separate from the messages array.
- **`tools`** — optional array of tool definitions the model may call.
- **`tool_choice`** — control over whether/which tool the model uses next.
- Optional sampling controls: `temperature`, `top_p`, `top_k`, `stop_sequences`, `stream`.

## Anatomy of a response

The response object carries:

- **`id`** — unique message identifier.
- **`content`** — an array of **content blocks**: `text`, `tool_use`, `thinking`, and so on.
- **`stop_reason`** — why generation stopped: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `refusal`, `pause_turn`.
- **`usage`** — `input_tokens`, `output_tokens`, and cache counters.

The architect treats `stop_reason` as the control-flow signal of the entire system: `tool_use` means another loop iteration; `end_turn` means the agent has finished.

## Statelessness as architectural fact

The API is **stateless**. The model has no memory between calls. The conversation state lives entirely in the application's `messages` array, which the application must send in full on every call. Persistence, branching, undo, fork, replay — all of these are application responsibilities.

This is liberating for architecture. The same conversation can be paused and resumed by storing the array. It can be forked by copying it and continuing each branch independently. It can be audited by treating each step as an immutable record. The architect designs around this property; the API does not.

## Versioning and pinning

The `anthropic-version` header pins a request to a specific API version. Model behavior, response field structure, and edge-case handling can change between versions. **Production architectures pin the version explicitly** and roll forward deliberately rather than allowing the default to drift under them.

## Where Claude is deployed

Claude is available through three primary surfaces, all speaking near-identical Messages API:

- **Anthropic API** — `api.anthropic.com`. Direct, simplest pricing, fastest access to new models.
- **Amazon Bedrock** — Claude as a managed Bedrock model. Useful for AWS-native architectures with VPC, IAM, and CloudWatch integration.
- **Google Cloud Vertex AI** — same idea, GCP-native.

The architect's choice between these is rarely about model quality (it's the same model) and usually about data residency, IAM integration, billing consolidation, and regional latency.
