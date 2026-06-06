# Prompt Engineering Techniques

Anthropic's documentation collects several techniques that consistently improve Claude's performance. An architect should know each by name and use each by reflex.

## Be clear, direct, and detailed

The single most reliable improvement is stating the task explicitly. Vague prompts produce vague outputs. "Summarize this" gives wide latitude; "Summarize in three bullet points, each beginning with a verb, totaling no more than 60 words" pins the result.

Direct instructions outperform polite indirection. "Output only valid JSON" works; "It would be nice if you could maybe format as JSON" works less well.

## Use examples (few-shot)

**Few-shot prompting** includes one or more worked examples in the prompt before the actual question. Each example shows the form of input and the form of expected output. For classification, extraction, or formatting tasks, a handful of well-chosen examples can outperform paragraphs of natural-language description.

The Anthropic convention is to wrap examples in XML tags:

```
<example>
<input>...</input>
<output>...</output>
</example>
```

## Let Claude think step by step (chain-of-thought)

For tasks that require reasoning, instruct Claude to **think out loud before answering**. Wrap thinking in `<thinking>` tags so the application can parse or strip it; or use the first-class **extended thinking** parameter.

The improvement on math, multi-step inference, and complex planning tasks is significant.

## Use XML tags to structure the prompt

Claude is trained to recognize **XML-tag-delimited structure** in prompts. Wrapping different parts of the prompt in named tags clarifies intent:

```
<context>The user is a hospital administrator...</context>
<task>Draft a memo summarizing...</task>
<constraints>
- Under 200 words
- No medical advice
</constraints>
```

This is more reliable than whitespace, headings, or pure prose.

## Prefill the assistant turn

Include an `assistant` message containing only the beginning of the desired response — `{` for JSON, `Step 1:` for an enumerated answer, the opening of a song lyric, anything. The model is constrained to continue that exact pattern. It cannot refuse, cannot apologize, cannot say "Sure, here's..." — it must continue the text you started.

This is the canonical way to force JSON output, structured responses, specific tones, or to bypass the model's tendency to add unwanted prose.

## Provide the document, then the question

For document-grounded tasks, put the **document first**, the **question last**. Claude's attention works better when the question can be answered with reference to recently-seen tokens. This pattern also pairs well with prompt caching — the document portion is cacheable across many questions.

## Role assignment

Assigning Claude a specific role in the system prompt ("You are a senior staff engineer reviewing this code…") shifts the response toward that role's typical content, vocabulary, and standards. Combined with explicit constraints, this is a reliable way to get domain-appropriate output.

## Define success criteria

Where measurable, define what success looks like in the prompt itself: "A correct answer satisfies these checks: (1) cites the source paragraph; (2) gives a numerical estimate; (3) flags uncertainty if the data is ambiguous." The model is more likely to produce a result that satisfies criteria it has been shown.

## Iterate on the prompt with measurement

Prompt engineering is empirical. The architect's discipline:

1. Define an **eval set** — fixed test inputs with known good outputs.
2. Try prompt variations.
3. Score outputs.
4. Keep the best.

The Anthropic Console includes a prompt-evaluation tool. Without measurement, prompt engineering devolves into superstition.

## Anti-patterns

- **Adding "please".** Politeness does not improve outputs and wastes tokens.
- **Repeating instructions.** Saying "Important: don't include preamble" three times in slightly different ways often makes Claude include preamble.
- **Negative framing.** "Don't be verbose" works less well than "Be terse." The model attends to what is described, even when negated.
- **Embedded fragility.** A timestamp or session ID at the top of the system prompt invalidates the cache on every call and serves no purpose.
- **Cargo-cult prompting.** Copying a prompt that worked on another model without testing it on Claude. Different models respond to different cues.

The architect distinguishes folk technique from measured technique. The latter is the discipline.
