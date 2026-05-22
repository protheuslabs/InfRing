# Coding Primitive Spine Reset

Status: active

Purpose: reset native coding workflow development around a small deterministic primitive spine instead of continuing to patch the large composite coding workflow.

## Decision

The official coding workflow remains useful as a composite shell, but it must stop absorbing new behavioral rules directly.

New coding behavior should be added only as:

- a primitive contract
- a tool contract
- a typed runtime gate
- a language/profile adapter
- a composite wiring rule
- an eval-only fixture or judge

If a Level N eval failure requires changing Level 1 behavior, the patch is invalid unless it improves a lower-level primitive monotonically.

## Minimal coding spine

The native coding workflow should rebuild upward from this spine:

1. `coding_task_contract`
2. `implementation_entry_gate`
3. `context_pack_builder`
4. `file_mutation_executor`
5. `public_interface_verifier`
6. `validation_runner`
7. `bounded_repair_loop`
8. `final_receipt_synthesis`

Each primitive must have explicit inputs, outputs, receipts, failure modes, and non-goals.

## Controller-owned invariants

These are not prompt suggestions. They are runtime/controller facts:

- If implementation is required and no `file_write` or `file_patch` receipt exists, the task is not complete.
- If validation passes before mutation, it is baseline evidence only.
- If validation passes but the requested public surface is missing, repair the implementation/export surface.
- If the model produces no tool calls for a mutation task, retry through the implementation-entry gate before finalization.
- If repair budget is exhausted without mutation, return structured partial/blocker evidence instead of success.

## Current first primitive patch

The native runtime now synthesizes `missing_product_mutation_receipt` as a repair reason whenever:

- native success criteria require mutation,
- the prompt implies product/software mutation,
- no successful mutation receipt exists.

This is intentionally independent of the large workflow artifact contract. It makes implementation entry a primitive runtime gate instead of a prompt-heavy workflow rule.

## Non-goals

- Do not hardcode Level 1 task names, packages, symbols, or fixture paths.
- Do not make Python-only behavior a global primitive without a language/profile boundary.
- Do not add more case-specific prose to the big coding workflow as the primary fix.
- Do not promote higher-level evals while lower-level mutation entry is unstable.
