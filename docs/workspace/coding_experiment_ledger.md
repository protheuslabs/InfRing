# Coding Workflow Experiment Ledger

Purpose: Preserve coding workflow experiment decisions independent of chat context. This ledger tracks hypotheses, changes, test evidence, findings, and decisions so unstable ideas are not accidentally retried or promoted.

Status values:
- `promoted`: active and supported by enough evidence to keep using.
- `active_trial`: active temporarily, still under evaluation.
- `dormant`: code or workflow shape exists but is disabled.
- `failed`: should not be retried in the same shape.
- `reverted`: removed or disabled after regression.

## EXP-CODING-001: Native mutation batch scheduler

Status: `promoted`

Hypothesis: When the model emits a batch containing product/source mutations, export-surface mutations, and commands, runtime should order the batch so implementation/source changes occur before export surfaces and validation.

Change: Added a native mutation batch scheduler that sorts implementation source mutations before export-surface mutations and commands. Added a guard that blocks export-surface-only progress when product/source mutation is still required.

Tests:
- Level 7 smoke after commit: `1/1` pass, wall `20.1s`, first mutation `19.9s`.
- Later Level 7 batch: `5/5` pass, median wall about `54.6s`.

Finding: Correctness improved for dependent multi-file edits. Speed remains provider-bound, but scheduler gives useful structural reliability.

Decision: Keep active.

Next: Preserve as controller-level primitive. Do not replace it with prompt-only ordering instructions.

## EXP-CODING-002: Failed-validation mutation-first repair guard

Status: `promoted`

Hypothesis: Validation failures after mutation should force mutation-first repair rather than allowing prose, broad rediscovery, or non-mutating loops.

Change: Added failed-validation repair contract and narrowed failed-validation repair tools to file read/write/patch and command run.

Tests:
- Level 4 pre-patch batch exposed one real failure: mutation happened, validation failed, repair did not produce a useful second mutation.
- Level 4 post-patch batch: `5/5` pass.

Finding: The failure mode was general, not slug-specific. A small repair-lane contract improved reliability without eval-specific hardcoding.

Decision: Keep active.

Next: If validation repair fails again, inspect whether the model had enough current file content and failure output, not whether a case-specific rule is missing.

## EXP-CODING-003: First-edit batch prompt contract

Status: `failed`

Hypothesis: A prompt-level first-edit batch contract could push the model to mutate earlier and reduce latency.

Change: Added prompt/tool-menu pressure for the first edit batch.

Tests:
- Level 7 failed with longer wall time and poor first mutation behavior.

Finding: Prompt-only pressure worsened reliability and latency.

Decision: Do not reactivate in the same shape.

Next: Use controller-level shaping or actual tool dispatch constraints instead of more prompt text.

## EXP-CODING-004: Incremental edit loop prompt path

Status: `failed`

Hypothesis: A prompt-level incremental edit loop would make the agent mutate sooner and repair in smaller chunks.

Change: Added an incremental edit loop prompt path, left dormant after testing.

Tests:
- Level 7 failed, wall about `139s`, first mutation about `111s`.
- It mutated export surface without completing product source behavior.

Finding: Prompt-only incremental looping does not reliably preserve source/export dependency structure.

Decision: Keep dormant/off.

Next: If revisited, it needs controller-owned tool/menu/dispatch boundaries, not just instructions.

## EXP-CODING-005: Compact bounded-edit bootstrap receipt-preview

Status: `failed`

Hypothesis: Compacting model-facing receipt context would reduce latency while keeping full receipts internally.

Change: Added compact bounded-edit bootstrap that sent summarized receipt previews instead of full receipt JSON.

Tests:
- Level 4 compact smoke passed but did not show a speed win.
- Level 7 compact smoke failed because the preview removed too much multi-file dependency/export context.

Finding: Truncated receipt previews are the wrong abstraction. They reduce structure along with size.

Decision: Keep workflow flag off. Do not promote.

Next: Use semantic context capsules instead of receipt-preview truncation.

## EXP-CODING-006: Local context pack builder v1

Status: `dormant`

Hypothesis: A real local context pack with file and validation capsules could reduce prompt size while preserving semantic context better than receipt-preview truncation.

Change: Added `local_context_pack_v1` model-facing capsules with file path, role, line range, content excerpt, and command stdout/stderr excerpts. Full receipts remain internal.

Tests:
- Initial smoke Level 4: pass, prompt chars about `5057` vs prior about `6124`.
- Initial smoke Level 7: pass, prompt chars about `8206` vs prior about `10007`.
- Retry batch: Level 4 `2/3`, Level 7 had `no_successful_mutation` before remaining run was stopped.

Finding: The abstraction is better than receipt-preview compaction, but v1 is not reliable enough as an active path. It can still fail to produce mutation.

Decision: Keep code as dormant scaffold. Workflow flags off.

Next: Do not reactivate without controller-level mutation shaping or a substantially different context-pack design.

## EXP-CODING-007: Local context pack builder v2 mutation-required prompt contract

Status: `failed`

Hypothesis: Adding an explicit mutation-required contract and target candidates inside the context pack would fix the v1 no-mutation failures.

Change: Added mutation-first required action, forbidden discovery/read tools before mutation, allowed first tools, and target candidates to the context-pack prompt/payload.

Tests:
- First Level 4 run failed with `no_successful_mutation` after about `152s`.
- Prompt chars increased to about `6128`, erasing much of v1's size benefit.
- Remaining queued runs were stopped after the regression was clear.

Finding: More prompt contract text made the prompt heavier and did not solve no-mutation failures.

Decision: Reverted to dormant v1 shape with workflow flags off. Do not retry v2 as prompt text.

Next: Move to controller-level shaping: mutation-only first turn, restricted first-turn tool menu, or dispatch-level mutation lane.

## EXP-CODING-008: Claude Code comparison baseline

Status: `reference`

Hypothesis: Faster frameworks may reveal useful runtime patterns for Infring without copying test-specific behavior.

Observed Claude Code Level 4 baseline:
- `5/5` pass.
- Median wall about `32.9s`.
- Median first mutation about `26.0s`.
- Visible prompt about `810` chars in one inspected run.
- Tool pattern: smaller incremental steps using Bash, Read, Edit, Bash.

Observed Infring Level 4 baseline:
- Recent `4/4` pass sample.
- Median wall about `38.6s`.
- Prompt/system/observation substantially larger than Claude Code.
- Tool pattern: bounded context preload, one larger structured JSON tool batch, then fast local execution.

Finding: Provider latency explains variance but not the full median gap. Infring's safety/receipt architecture is useful, but model-facing contract size and one-shot batch complexity amplify latency.

Decision: Keep receipts/gates. Optimize model-facing context and tool-loop shape without weakening audit or validation.

Next: Compare against controller-level mutation-only lanes rather than adding more prompt instructions.

## Current active coding lane summary

Active:
- Native mutation batch scheduler.
- Failed-validation mutation-first repair guard.
- Native receipts and validation/semantic probe gates.

Dormant/off:
- Compact bounded-edit bootstrap.
- Local context pack builder.
- Incremental edit loop.
- First-edit batch contract.

Current bottleneck:
- First provider turn latency before mutation.
- One-shot structured batch complexity.
- Prompt/system/observation size.

Current recommended next experiment:
- Controller-level first-mutation tool shaping.
- Keep full receipts internally.
- Do not add more prompt contract text.
- Evaluate with Level 4 and Level 7 before promotion.
