# Prompt Engineering Techniques

Prompt engineering is the discipline of structuring inputs to a language model so that it produces the desired output reliably. Anthropic's documentation collects several techniques that consistently improve Claude's performance on complex tasks.

## Be clear, direct, and detailed

The single most reliable improvement is to **state the task explicitly**. Vague prompts produce vague outputs. A prompt that says "Summarize this" gives the model wide latitude; a prompt that says "Summarize this in three bullet points, each beginning with a verb, totaling no more than 60 words" pins down the shape of the result.

Direct instructions outperform polite indirection. "Output only valid JSON" works; "It would be nice if you could maybe format the output as JSON" works less well.

## Use examples (few-shot prompting)

**Few-shot prompting** is the technique of including one or more worked examples in the prompt before the actual question. Each example shows the model the form of input and the form of expected output. For classification, extraction, or formatting tasks, a handful of well-chosen examples can outperform paragraphs of natural-language description.

The convention in Anthropic prompts is to wrap examples in XML tags:

```
<example>
<input>...</input>
<output>...</output>
</example>
```

The tags make the structure explicit to the model and easy to parse for the application.

## Let Claude think step by step

For tasks that require reasoning — math, multi-step inference, complex planning — instructing Claude to **think out loud before answering** improves accuracy. This is the chain-of-thought technique:

> "Before giving your final answer, think through the steps in `<thinking>` tags."

Recent Claude models support **extended thinking** as a first-class API feature: setting `thinking: { type: "enabled", budget_tokens: N }` in the request lets the model produce a dedicated thinking block before the visible response. The thinking is counted separately in usage and can be retained or discarded by the application.

## Use XML tags to structure the prompt

Claude is trained to recognize **XML-tag-delimited structure** in prompts. Wrapping different parts of the prompt in tags clarifies intent:

```
<context>The user is a hospital administrator...</context>
<task>Draft a memo summarizing...</task>
<constraints>
- Under 200 words
- No medical advice
</constraints>
```

This is more reliable than relying on whitespace, headings, or pure prose to delimit sections.

## Prefill the assistant turn

A subtle but powerful technique is to start the assistant turn yourself. By including an assistant message in the `messages` array that contains only the beginning of the desired response (for example, `{` to start a JSON output, or `Step 1:` to start an enumerated answer), you constrain the model's next tokens to continue that exact pattern. The model cannot refuse, cannot apologize, cannot say "Sure, here's..." — it must continue the text you started.

This is the canonical way to force JSON output, structured responses, or specific tones.

## Provide the document, then the question

For document-grounded tasks (QA over a long document, summarization, extraction), put the **document first**, the **question last**. Claude's attention works better when the question can be answered with reference to recently-seen tokens. This pattern also pairs well with prompt caching, since the document portion is reused across many questions.

## Iterate on the prompt with measurement

Prompt engineering is empirical. Two prompts that look equally reasonable can differ significantly in output quality. The architect's discipline is to:

1. Define an **eval set** — a fixed list of test inputs with known good outputs.
2. Try variations of the prompt.
3. Score the outputs.
4. Keep the best.

Without measurement, prompt engineering devolves into superstition. The Anthropic Console includes a prompt-evaluation tool for this workflow.
