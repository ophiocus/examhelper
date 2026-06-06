# Constitutional AI and Safety

**Constitutional AI** (CAI) is the training method Anthropic developed and uses for Claude. It is the central reason Claude's safety behavior — what it refuses, how it refuses, what it discloses — differs from models trained on more traditional RLHF.

## The basic idea

Traditional RLHF trains a reward model on human preference labels and then optimizes the language model against that reward. The quality of the result is bounded by the quality, consistency, and coverage of the human labels.

Constitutional AI replaces a large portion of those human labels with **model-generated labels guided by a written constitution**. The constitution is a set of principles (drawn from sources including the UN Declaration of Human Rights, Apple's terms of service, and other broad ethical and operational rules) that the model is instructed to evaluate its outputs against. The training loop:

1. The model produces an output.
2. The model (or a separate model) critiques that output against the constitution.
3. The model produces a revised output.
4. The model is trained to prefer the revised output.

This **AI-feedback** loop (sometimes called RLAIF, reinforcement learning from AI feedback) scales further than purely human-labeled RLHF and produces a more consistent application of stated principles.

## What Claude's constitution covers

The principles emphasize:

- **Helpfulness** — Claude should be genuinely useful, not merely safe-by-refusal.
- **Honesty** — Claude should not deceive, should acknowledge uncertainty, should not pretend to have capabilities it lacks.
- **Harmlessness** — Claude should refuse to help with material harm (violence, weapons, fraud, exploitation), but should refuse precisely, not by blanket avoidance of any tangentially related topic.

The triad **helpful, honest, harmless** is sometimes abbreviated *HHH*.

## Implications for application design

CAI training has practical consequences.

- **Refusals are principled, not keyword-based.** Claude does not refuse a request because it contains the word "weapon"; it refuses because the request is for material harm. Legitimate adjacent requests (a novelist describing a fight scene, a security researcher discussing exploit classes) usually succeed. Conversely, harmful requests in disguise are harder to slip past — the model evaluates intent, not surface features.
- **The model can be reasoned with about refusals.** If Claude declines, supplying additional context, clarifying legitimate purpose, or adjusting framing often produces a useful response on the next turn.
- **The model is not infallible.** CAI is a training method, not a guarantee. Edge cases, adversarial prompts, and novel attack patterns can still produce undesirable outputs.

## Defense in depth at the application layer

Trusting the model's safety training alone is a mistake. Production architectures include layers above and below the model.

- **Input filtering.** Before sending user input to Claude, run cheap classifiers, content filters, or rule-based checks for known-bad patterns.
- **Output filtering.** Before returning Claude's response, scan for prohibited content, leaked secrets, or off-policy outputs.
- **Tool-call gating.** A pre-tool hook can refuse or modify tool calls. Even if Claude calls `delete_all_records`, the hook can require confirmation or block the action.
- **Audit logging.** All inputs, outputs, and tool calls are logged for post-hoc analysis.
- **Human-in-the-loop.** For high-stakes irreversible actions (sending external communications, executing payments, modifying production systems), a human approval step is the most reliable defense.

The architect's responsibility is not to make Claude impossible to abuse — that is the model trainer's responsibility — but to layer the application so that the consequences of a model failure are bounded.

## Refusals as a deployment signal

When Claude refuses a request in production, that refusal is a signal worth instrumenting. A spike in refusals for a feature may indicate legitimate users are being blocked (a precision problem), or that the feature is attracting misuse (a real signal). Tracking refusal rates and their reasons is part of operating a Claude-based product responsibly.

## Responsible disclosure and reporting

When an architect discovers a model failure mode — a prompt injection that bypasses constraints, a jailbreak that produces harmful content, a privacy leak — Anthropic operates a responsible disclosure program. Reporting findings privately helps Anthropic improve the model rather than leaving the issue exposed.

For production deployments, the equivalent internal discipline: bug reports about Claude behavior get triaged like any other product bug. The model is a component; its failures are product failures; the team owns the response.
