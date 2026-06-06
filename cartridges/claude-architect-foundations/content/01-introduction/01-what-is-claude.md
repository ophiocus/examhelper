# What Is Claude?

**Claude** is a family of large language models developed by **Anthropic**, a San Francisco-based AI safety company founded in 2021. The first Claude model was released in March 2023; the family has since grown through several major generations (Claude 1, Claude 2, Claude 3, Claude 3.5, Claude Sonnet 4 and beyond), with each generation introducing variants tuned for different cost-and-capability tradeoffs.

## The Claude model family

A given Claude generation typically offers three variants distinguished by a name token: **Haiku**, **Sonnet**, and **Opus**.

- **Haiku** is the fastest and least expensive variant. It is intended for high-throughput, latency-sensitive tasks such as classification, simple extraction, content moderation, and embedded assistants.
- **Sonnet** is the balanced variant. It is the default choice for most production work — coding, agentic tasks, retrieval-augmented generation, complex tool use — when the cost or speed of Opus is not justified.
- **Opus** is the most capable variant. It is reserved for the hardest reasoning, longest-context analysis, agentic work over extended horizons, and tasks where quality matters more than throughput or cost.

The exact pricing, context window, and benchmark performance differ by generation. The Foundations exam expects familiarity with the **tiering principle** (Haiku → Sonnet → Opus, fast/cheap → balanced → most capable) rather than memorization of any single generation's numbers.

## What Claude is good at

Claude is a general-purpose conversational model with several emphases that distinguish it from other LLMs:

- **Long-context reasoning.** Claude models support large context windows (200K tokens is common; some configurations extend to 1M). This is useful for whole-codebase analysis, large-document QA, and long agent sessions.
- **Tool use and agentic work.** Claude models are trained to call tools, reason about tool results, and chain tool calls together. The Agent SDK and Claude Code are built on this capability.
- **Coding.** Claude has been benchmarked at the top of SWE-bench and similar coding evaluations across multiple generations. Claude Code, Anthropic's CLI coding tool, is the canonical product instance.
- **Faithful, structured, citable output.** Claude tends to follow output schemas precisely and to cite passages from supplied context rather than confabulating.
- **Constitutional AI training.** Claude is trained using a method called *Constitutional AI* — see the safety section — that yields a model that refuses harmful requests in a principled way rather than through brittle keyword filters.

## How Claude is deployed

Claude is accessed primarily through three surfaces:

1. **The Anthropic API.** A REST API with a single primary endpoint (`/v1/messages`) and SDKs for Python, TypeScript, Java, Go, and other languages. This is the surface that the Foundations exam concentrates on.
2. **Cloud-provider partnerships.** Claude is also available through **Amazon Bedrock** and **Google Cloud Vertex AI**, with the same model behavior and similar request formats.
3. **First-party products.** Claude.ai (the consumer chat product), Claude Code (the CLI), and Claude in Chrome (the browser extension) are first-party Anthropic products built on the same underlying models.

The Foundations exam concentrates on the API surface and on the architectural patterns used to build production applications on it.
