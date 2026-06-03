# Agent Runtime Engine Strategy

InfRing should treat agent runtimes as swappable engines connected through Gateway, not as Kernel authority and not as Shell-owned behavior.

The near-term goal is to let InfRing use strong external frameworks while preserving InfRing's own substrate: Kernel authority, Gateway boundaries, Memory context, Observability traces, Validation checks, and Governance policy.

## Engine role

External frameworks such as Codex CLI, Claude Code, Grok Code, OpenHands, OpenClaw, OpenFang, and future socket engines are work runtimes. They may perform reasoning, coding, and task execution inside bounded scopes, but they do not own durable truth.

```text
Shell selects engine_id
  -> Gateway validates and routes
  -> selected runtime performs work
  -> Gateway normalizes events/proposals
  -> Kernel/Memory/Artifact authority handles durable effects
  -> Observability and Validation record evidence
```

## Current compatibility layer

Most current CLI frameworks only consume prompt text. InfRing therefore uses bounded context packs rendered into prompt preambles as the compatibility layer.

This is intentionally transitional. It proves semantics and continuity before attempting lower-token transports.

Current path:

```text
Kernel Layer-2 context topology
  -> AgentRuntimeContextPack
  -> Gateway universal tool grants
  -> adapter prompt preamble
  -> external runtime turn
```

## Target transport abstraction

Eventually each engine should advertise its strongest supported context transport mode.

```text
prompt_text              universal fallback
structured_json          typed context object sent through socket/API
context_refs             engine receives refs and can request details through Gateway
memory_refs              engine receives memory refs and bounded summaries
embedding_refs           engine can use shared or projected embedding context
same_model_kv_cache      same model/runtime cache reuse
family_kv_cache_adapter  compatible model-family cache adaptation
learned_context_transcoder future research track
native_infring_context   InfRing-native engine consumes context directly
```

Gateway chooses the strongest safe mode and falls back toward `prompt_text`.

## Universal tools before native workflow exposure

Only a tiny universal core-tool surface should cross engine boundaries at first:

```text
conversation.read
memory.read
memory.write_propose
artifact.read
artifact.create_propose
permission.request
```

These are proposal/read surfaces, not direct authority. Engines may propose calls. Gateway validates. Kernel/Memory/Artifact authority performs durable effects and emits receipts.

Native workflow tools, web research tools, terminal execution, direct file writes, and orchestration-specific tools should not be exposed wholesale to external engines until the universal boundary is proven.

## Why LLM manipulation waits

InfRing will eventually need model-manipulation capabilities: local specialist models, quantization, fine-tuning, context transcoding, and possibly KV-cache or latent-state transport.

That work is mid/long-term. It should wait until external framework integration works well enough that InfRing is practically useful.

Premature LLM manipulation risks building advanced infrastructure before the product can reliably do real work.

## Practical milestone order

1. Reliable framework selection in the dashboard.
2. Context continuity across Codex, Claude Code, Grok Code, native, and socket engines.
3. Universal core-tool proposals across engines.
4. Real coding/task workflows through external frameworks inside InfRing.
5. Structured JSON context transport for socket-capable engines.
6. Ref-based memory/artifact access through Gateway.
7. InfRing-native orchestration improvements.
8. LLM manipulation and specialist model training.
9. Token/embedding/KV/context transcoding primitives.

## Operating rule

InfRing should first become useful with other brains, then become smarter, then learn to grow its own brains.
