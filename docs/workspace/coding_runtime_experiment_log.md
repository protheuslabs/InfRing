# Coding runtime experiment log

This log captures coding-runtime experiments that affect primitive workflow design.

## 2026-05-26 - Level 7 dependent edit stabilization

Stable checkpoint:

- Commit: `f72d44d69`
- Result: native Infring Level 7 passed `5/5`.
- Median wall time: about `57.8s`.
- Median first mutation: about `56.9s`.
- Key primitive: `atomic_dependent_edit_guard`.

The guard prevents dependent export-surface edits, such as `__init__.py`, `index.ts`, or `mod.rs`, from landing after a blocked product-source edit. This preserves atomicity across related source/export edits without hardcoding a specific eval case.

## 2026-05-26 - Failed `first_edit_batch_contract` speed experiment

Experiment:

- Enabled `first_edit_batch_contract` in the coding workflow CD.
- Ran native Infring Level 7 using `kimi-k2.6:cloud`.

Observed result:

- Level 7 failed.
- Wall time: about `113.5s`.
- First mutation: about `93.0s`.
- Validation and semantic probe failed.
- Prompt chars dropped slightly, system/tool surface dropped sharply, but provider latency and output quality got worse.

Conclusion:

- Compact prompt and restricted tool menu alone are not the Claude Code advantage.
- This experiment should not be treated as a reusable improvement.
- The next primitive should be an `incremental_edit_loop` that keeps rich success/receipt semantics while changing execution sequencing.

Design implication:

- Prefer `bounded context -> normal tool menu -> early useful mutation -> automatic validation -> repair only if needed`.
- Avoid `tiny system -> mutation-only menu -> hope the model infers all task semantics`.
- Any early-edit primitive must remain config-gated and must prove no lower-level regression before promotion.

## 2026-05-26 - Failed `incremental_edit_loop` first attempt

Experiment:

- Added a distinct `incremental_edit_loop` primitive.
- Kept the normal native tool menu.
- Used a smaller first-turn system prompt focused on bounded-context editing.
- Enabled the primitive in the coding workflow CD for one Level 7 run.

Observed result:

- Level 7 failed.
- Wall time: about `139.3s`.
- First mutation: about `111.2s`.
- Validation and semantic probe failed.
- The first provider turn was marked `incremental_edit_turn: true`.
- The model mutated only the export surface while leaving product source incomplete.

Conclusion:

- A shorter first-turn system plus normal tool menu is still not sufficient.
- Claude Code's advantage is likely not merely first-turn wording.
- The missing primitive appears to be controller-level sequencing: small read batches, edit application as soon as a valid edit appears, then immediate validation/repair.

Design implication:

- Keep `incremental_edit_loop` dormant until the controller can stream or split execution at the tool-call boundary.
- Do not promote prompt-only early-edit experiments without a passing lower-level monotonic batch.

## 2026-05-26 - `mutation_batch_scheduler` controller primitive

Experiment:

- Added `mutation_batch_scheduler` as a controller-level primitive.
- Enabled it in the coding workflow CD.
- Ran native Infring Level 7 five times with `kimi-k2.6:cloud`.

Observed result:

- Level 7 passed `5/5`.
- Wall times: `37.6s`, `61.8s`, `47.0s`, `75.2s`, `52.9s`.
- Median wall time: about `52.9s`.
- Median first mutation: about `52.7s`.
- The scheduler reordered source/export edits when needed. In the smoke run, the model requested the export edit before the source edit, and the runtime executed the implementation source edit first.

Conclusion:

- This is a real primitive improvement: it changes controller behavior without hardcoding the eval.
- It improves atomicity and modestly improves median performance versus the prior native Infring Level 7 median of about `57.8s`.
- It does not solve the remaining speed gap against Claude Code, because most wall time is still provider latency before usable tool calls appear.

Design implication:

- Keep `mutation_batch_scheduler` as a promoted bounded-direct-edit primitive if lower levels remain green.
- The next speed primitive should target provider/tool-call streaming or split execution, not prompt compression.

## Dormant compact bounded-edit bootstrap experiment

Date: 2026-05-26

Status: Dormant. The workflow flags are off.

Purpose: Test whether reducing model-facing receipt context could close the Level 4 speed gap against Claude Code while preserving native receipt gates.

Change shape: Added a compact bounded-edit bootstrap prompt path that keeps full receipts internally but sends summarized receipt previews to the model.

Observed result:
- Level 4 compact smoke passed, but prompt size only dropped modestly and did not prove a speed win.
- Level 7 compact smoke regressed because the compact preview removed too much multi-file dependency/export context.
- Disabling the flag restored the workflow shape, but the follow-up Level 7 restore smoke hit provider timeout, so that timeout is not counted as evidence for the compact path.

Conclusion: Receipt-preview compaction is the wrong primitive. Preserve this only as a dormant scaffold. The next primitive should be `local_context_pack_builder`, which emits explicit file/context capsules rather than truncated receipt JSON.

## Local context pack builder v1

Date: 2026-05-26

Status: Active in the native coding workflow.

Purpose: Reduce model-facing bounded-edit context without weakening native receipts, mutation gates, validation gates, or audit evidence.

Primitive shape:
- Runtime retains full native tool receipts internally.
- Model receives `local_context_pack_v1` capsules instead of full receipt JSON during bounded direct-edit bootstrap.
- File capsules include path, role, line range, total lines, and bounded content excerpts.
- Command capsules include cwd, command, success, exit code, timeout flag, and bounded stdout/stderr excerpts.

Initial smoke results:
- Level 4: pass, prompt chars dropped to 5057 from the prior approximate 6124 full-receipt prompt.
- Level 7: pass, prompt chars dropped to 8206 from the prior approximate 10007 full-receipt prompt.

Conclusion: This is the first viable version of the model-facing context optimization. It is materially better than compact receipt-preview truncation because it preserves file and validation semantics for multi-file slices.
