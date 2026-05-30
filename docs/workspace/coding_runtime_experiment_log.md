# Coding runtime experiment log

This log captures coding-runtime experiments that affect primitive workflow design.

## 2026-05-26 - Post-validation closure controller trial

Experiment:

- Added a config-gated post-validation closure controller to the model-manifest runtime lane.
- Enabled it only in `coding_project_operator`.
- Reused the generic handoff/memory artifact synthesizer instead of adding Level 8 fixture-specific behavior.

Motivation:

- Native Level 8 showed Infring could produce real subsystem code and pass validation, but failed checkpoint receipt and memory-row closure.
- Claude Code and Codex passed Level 8 by completing code, validation, checkpoint receipt, and memory row.

Expected behavior:

- After successful mutation plus validation, the runtime may synthesize declared checkpoint/handoff artifacts and expected memory rows from receipts when workflow CD policy enables the primitive.
- Lower-level mutation spines remain unaffected unless they opt in.

## 2026-05-26 - Capability evidence gate trial

Experiment:

- Added a pre-dispatch capability-evidence gate to the model-manifest planner.
- Enabled it in `coding_project_operator`.
- The gate activates only when the user task itself implies durable/persistent/checkpoint/report/import-export capability.
- It checks generic planned mutation categories rather than fixture names or eval slugs.

Motivation:

- Post-closure Level 8 reruns failed before terminal bookkeeping: the model sometimes omitted the actual product slice, such as persistent storage, service integration, CLI/report surface, import/export roundtrip, checkpoint receipt, or memory row.
- Claude Code and Codex passes showed a clear runtime pattern: build a coherent vertical slice first, then validate, then close checkpoint/memory artifacts.

Expected behavior:

- Bad manifests should be rejected before tool dispatch and repaired once with explicit missing category feedback.
- If repair still lacks required categories, the runtime should fail closed with structured `missing_capability_evidence` instead of spending a full run on an under-scoped implementation.
- The gate should not affect simple local edits because it is activated by durable capability language in the prompt, not by test level names.

## 2026-05-27 - Checkpointed project-operator tool-loop routing

Experiment:

- Added `checkpointed_project_operator_tool_loop_enabled` to `coding_project_operator`.
- The runtime now skips `model_manifest_planner` for prompts that combine checkpoint/resume/memory, local project, and durable vertical-slice signals.
- Skipped tasks fall through to the native agent tool loop instead of requiring one giant pre-tool deterministic manifest.
- The tool-loop prompt keeps Infring-specific receipt discipline: source/test/receipt/local artifact mutations should use native `file_write`/`file_patch`; `command_run` is for validation and required project or memory CLI commands.

Evidence source:

- Claude Code and Codex Level 8 passes used an incremental loop: list/read project context, retrieve checkpoint memory, edit source/test/operator files, run validation, write checkpoint receipt, then write memory row.
- The failing Infring Level 8 run timed out before mutation inside the initial `model_manifest_planner` call, so the capability-evidence gate never got a chance to operate.

Design implication:

- Deterministic manifests remain useful for bounded slices.
- Checkpointed project-operator work needs an open native tool loop because the task includes discovery, memory recall, multi-file edits, validation, and terminal artifact closure.
- The selector is prompt/category based, not eval-level based, to preserve primitive-first doctrine.

## 2026-05-27 - Durable vertical-slice sufficiency gate

Experiment:

- Added prompt-derived durable vertical-slice evidence categories to the native product-slice checker.
- Categories include domain record/model, persistence store, service integration, operator/report surface, report summary, and import/export or round-trip behavior when the prompt requests them.
- Wired the same evidence into live staged gating, bounded-direct final evidence, and repair hints.

Motivation:

- The first checkpointed tool-loop Level 8 smoke reached native tools and validation, but under-scoped the product slice: it changed only repository and tests, then failed receipt/memory and judge checks for model, report, CLI import/export, and memory row.
- Claude Code and Codex passed by completing the vertical slice before closure, not by validating a shallow source fragment.

Expected behavior:

- The runtime should keep durable project-operator tasks in source/product mutation while requested vertical-slice categories are missing.
- Repair prompts should tell the model which generic categories are missing instead of only requesting a checkpoint receipt path.
- Closure should remain blocked until source/test/validation evidence is strong enough for post-validation receipt/memory synthesis or explicit tool closure.

## 2026-05-27 - Checkpointed tool-loop provider timeout

Experiment:

- Added `checkpointed_tool_loop_provider_timeout_seconds` to the coding workflow CD.
- Native tool-loop provider requests now get a top-level `provider_timeout_seconds` when checkpointed project-operator routing is active.

Motivation:

- After adding the durable vertical-slice sufficiency gate, a Level 8 smoke exceeded ten minutes with no worker output.
- The previous patch improved correctness pressure, but exposed that a single open-loop provider turn could outlive the intended wall budget because timeout policy was not presented at the provider-call layer.

Expected behavior:

- Checkpointed project-operator turns are bounded to 120 seconds by default.
- Slow source-repair turns should return structured timeout or partial progress instead of consuming the full worker timeout.
- This keeps the Claude/Codex-inspired open-loop shape while preserving Infring's controlled execution contract.

## 2026-05-27 - Memory recall versus closure gating

Experiment:

- Split memory CLI command handling into recall/discovery versus closure/write semantics.
- The staged controller now treats memory write, ingest, update, append, put, or checkpoint write commands as memory closure.
- Memory get/read/recall commands are allowed before product mutation.

Motivation:

- The Level 8 checkpointed tool-loop smoke showed `staged_controller_requires_product_source_before_memory_closure` on an early memory command.
- Passing Claude Code and Codex traces retrieve checkpoint memory before editing, then write the new checkpoint memory row after validation and receipt closure.

Expected behavior:

- Agents can retrieve checkpoint memory early without being blocked.
- Premature memory-row writes remain blocked until source/test/validation/checkpoint evidence is complete.

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

## Local context pack builder v1 batch retry

Date: 2026-05-26

Status: Dormant. The workflow flags were turned off after batch retry.

Batch result with `local_context_pack_builder` active:
- Level 4: 2/3 pass. One failure class: `no_successful_mutation` after a long provider turn.
- Level 7: 1 completed pass plus one `no_successful_mutation` failure before the remaining run was interrupted after the regression was clear.

Conclusion: `local_context_pack_builder` is a better abstraction than compact receipt-preview truncation, but v1 is not reliable enough to stay active. Keep the code as a dormant primitive scaffold. Next iteration should preserve dependency/export structure more explicitly and add a mutation-required micro-contract inside the context pack, without reintroducing full receipt JSON.

## Local context pack builder v2 mutation-required contract trial

Date: 2026-05-26

Status: Reverted to dormant.

Change attempted: Added an explicit mutation-required contract to the context pack prompt and JSON payload, including mutation-first action, forbidden discovery/read tools before first mutation, and target mutation candidates.

Observed result:
- First Level 4 run failed with `no_successful_mutation` after a long provider turn.
- Prompt chars increased to 6128, erasing much of the v1 prompt-size benefit.
- Remaining queued runs were interrupted once the same failure class appeared.

Conclusion: Adding a heavier mutation-required contract inside the context pack is not the right repair. It increases model-facing complexity and still does not reliably produce mutation. Keep `local_context_pack_builder` dormant. The next viable path should likely be controller-level tool-menu/dispatch shaping or a real streaming/incremental edit lane, not more prompt contract text.

## First-mutation tool shaping controller trial

Date: 2026-05-26

Status: Reverted to dormant/off.

Change attempted: Added `first_mutation_tool_shaping`, reusing the staged-edit controller to restrict the first post-bootstrap bounded-edit turn to mutation tools.

Observed result:
- Level 4 passed, but wall time was 134.8s and first mutation was 87.8s. The staged tool menu reduced system prompt size but introduced extra provider-loop cost.
- Level 7 failed validation, wall time 232.8s and first mutation 203.8s.

Conclusion: Naively restricting the first post-bootstrap turn to mutation tools is not a viable controller-level optimization. It can split the work into a slower multi-turn path and degrade multi-file/export-surface correctness.

Decision: Keep `first_mutation_tool_shaping` off. Do not retry through the staged-edit controller. A future controller-level attempt needs a different shape, likely streaming first usable tool calls from the normal full tool batch or executing partial valid mutation calls without forcing a separate staged turn.

## Provider stream until first mutation call trial

Date: 2026-05-26

Status: Reverted to dormant/off.

Change attempted: Added `provider_stream_until_first_mutation_call`, allowing the Ollama streaming provider to stop as soon as the first complete mutation tool-call object appears inside the normal full tool batch. Unlike staged-edit shaping, this used the normal full prompt and full tool menu.

Observed result:
- Level 4 passed: wall 39.0s, first mutation 38.9s. The first mutation did not appear materially earlier than the balanced tool-call JSON in this sample.
- Level 7 failed validation: wall 170.5s, first mutation 81.4s, changed only one file, then wandered through many read/list calls.

Conclusion: Executing only the first streamed mutation call amputates later planned calls from the model's intended multi-file batch. This can harm multi-file/export-surface correctness and trigger slow repair/discovery loops.

Decision: Keep `provider_stream_until_first_mutation_call` off. Do not promote first-call partial execution.

Next: If streaming is revisited, stream until a safe mutation prefix/batch is complete, not just the first mutation call. The prefix must preserve dependent multi-file edits or wait for the full balanced batch.

## Provider stream until mutation prefix trial

Date: 2026-05-26

Status: Dormant/off after smoke.

Change attempted: Added `provider_stream_until_mutation_prefix`, a safer streaming variant that waits for a prefix of at least two complete mutation tool-call objects before stopping the provider stream. This avoids the first-call trial's worst behavior of amputating source/export dependent edits after only one mutation.

Observed result:
- Level 4 passed: wall 25.0s, first mutation 24.8s.
- Level 7 passed: wall 97.9s, first mutation 72.4s, changed two files.
- Level 7 still required a slower follow-up path with extra reads before the second mutation, so the speed profile was worse than the stable full-batch median.

Conclusion: Mutation-prefix streaming is safer than first-call streaming, but v1 is not an active improvement. It can preserve correctness in the smoke run, but does not reliably beat the stable full balanced-batch path.

Decision: Keep `provider_stream_until_mutation_prefix` off. Preserve the dormant provider hook for future experiments.

Next: If revisited, the prefix parser needs to stop only when the prefix contains all source/export/test mutations needed for the slice, or it should not stop before the full balanced batch.

## Provider stream until safe mutation prefix trial

Date: 2026-05-26

Status: Dormant/off after smoke.

Change attempted: Tightened mutation-prefix streaming so it only stops when the streamed prefix contains a safe-looking multi-mutation product pair, such as source plus export surface or source plus test. One-file tasks should fall back to full balanced-batch behavior.

Observed result:
- Level 4 passed: wall 75.2s, first mutation 75.1s. This effectively fell back to full-batch behavior.
- Level 7 failed with `no_successful_mutation`: wall 169.7s, no mutation. The model emitted a balanced tool-calls object early, but the stricter prefix did not accept a safe prefix and the run did not recover into mutation.

Conclusion: Safe-prefix parsing is too brittle at the provider layer with only streamed JSON shape/path heuristics. It can reject useful batches and leave the runtime without mutation.

Decision: Keep `provider_stream_until_mutation_prefix` off. Do not promote provider-only prefix gating.

Next: Prefer the existing full balanced-batch path plus runtime batch scheduling. Further speed work should focus on provider/model latency, smaller but semantically complete prompts, or better model/tool-call generation, not provider-side partial execution.

## Split checkpointed project-operator timeout budget

Date: 2026-05-27

Status: Partial; kept as a primitive but not sufficient.

Change attempted: Kept the checkpointed project-operator tool loop on, but declared a larger first-turn timeout and a shorter repair-turn timeout in the workflow CD.

Reasoning: The previous Level 8 run proved that a 120s cap can stop the first provider turn before any mutation. Fast reference frameworks appear to preserve one coherent initial edit batch, then bound repair/validation. This change matches that runtime shape without adding test-case-specific logic.

Observed result: Level 8 completed in about 200s with an infrastructure-classified failure. The first provider turn used the declared 180s initial budget, but attempted `command_run` before any product mutation. The staged controller blocked the command, then the 120s repair turn timed out without a successful mutation.

Conclusion: Split budgets prevent unbounded stalls and expose the next real blocker, but do not by themselves make the project-operator loop pass. The next patch should make the post-blocker repair contract unambiguously mutation-only until a product source mutation receipt exists.

## Premature validation recovery gate

Date: 2026-05-27

Status: Partial.

Change attempted: Added a workflow-gated native recovery primitive that activates after `command_run` is blocked before required product mutation. The next provider turn gets a compact write/patch-only prompt and a mutation-only tool menu.

Reasoning: Claude Code and Codex traces that pass Level 8 preserve the sequence `context -> product mutation -> validation -> repair/closure`. This patch copies that sequencing behavior without weakening receipts, validation, or product-slice gates.

Expected result: The previous Level 8 blocker should recover into source/operator/test mutation instead of spending the repair turn on another broad provider attempt.

## First-timeout mutation-only recovery

Date: 2026-05-27

Status: Partial.

Observed trigger: After adding premature-validation recovery, Level 8 no longer reached a command blocker. It timed out before any mutation or blocker receipt, then the broad timeout-recovery turn also timed out without mutation.

Change attempted: Generalized the controller state into `mutation_only_recovery_gate_enabled` and routed first-turn timeout recovery into the same write/patch-only tool menu used for premature validation recovery.

Observed result: Level 8 produced blocker receipts for premature `command_run` and repeated `file_read` attempts before mutation. The runtime then routed the next provider turn through mutation-only recovery, but the provider timed out at 120s without emitting a write/patch mutation.

Conclusion: The controller sequencing is moving in the right direction, but the recovery prompt is still too receipt-heavy or too ambiguous. The next improvement should use a compact mutation-entry packet instead of passing full native observations into the recovery prompt.

## Compact mutation-entry packet

Date: 2026-05-27

Status: Partial.

Change attempted: Added a CD-gated compact mutation-entry packet for mutation-only recovery turns. The packet includes the project root, exact allowed/forbidden next tools, prioritized candidate paths, and observed source/test contents.

Reasoning: Codex/Claude-style fast paths keep the edit-entry context small and action-shaped. Infring was handing the recovery turn a large native observation payload, which preserved evidence but made the next action harder for the model.

Observed result: Level 8 still failed as `provider_timeout_or_spawn_failure`, but wall time dropped to about 143s. The first turn attempted `command_run`, the controller blocked it, and the compact mutation-entry recovery turn timed out at 120s without mutation.

Conclusion: The compact packet is cleaner and faster, but the model still needs a more executable mutation-entry contract for Level 8. The next candidate is a small required batch shape that identifies source/operator/test roles from observed paths and asks for one coherent write/patch batch.

## Executable mutation-entry batch shape

Date: 2026-05-27

Status: Active for next Level 8 run.

Change attempted: Added generic target-role derivation to the compact mutation-entry packet and included an explicit JSON `tool_calls` batch skeleton. Roles are inferred from observed paths and prompt demands, not from eval fixture names.

Reasoning: Aider/Codex/Claude-style edit entry is action-shaped: pick target files, produce edits, validate after. The previous packet reduced context but still made the model infer the actual edit batch shape.

Observed result: Level 8 still failed as `provider_timeout_or_spawn_failure` after about 189s. The first provider turn attempted `command_run`, the staged controller blocked it, and the mutation-only recovery turn timed out at 120s without emitting `file_write` or `file_patch`.

Conclusion: The recovery packet is more executable, but the current provider/model path still does not reliably enter edit mode at Level 8. The next investigation should compare the actual recovery prompt/timing probe with Codex/Claude edit-entry traces before adding more prompt text.

## Controlled shell edit batch

Date: 2026-05-27

Status: Active for next Level 8 run.

Trace finding: Codex and Claude Code used the same Kimi-class model successfully by entering shell/edit-batch mode: read context, write whole files with shell commands, validate, then write receipt and memory. Infring's recovery prompt asked for native JSON file writes and timed out.

Change attempted: Added a controlled shell edit batch primitive. Pre-mutation `command_run` remains blocked unless it contains explicit project-file write targets. When accepted and successful, runtime scans those target files and synthesizes `file_write` mutation receipts with content hashes.

Expected result: Level 8 should be able to mutate through a framework-like shell edit batch while preserving Infring receipt semantics.

## Enable first-turn timeout recovery for project operator

Date: 2026-05-27

Status: Active for next Level 8 run.

Observed trigger: After adding controlled shell edit batch, the Level 8 run timed out on the first provider turn at 180s with only bootstrap receipts. The Rust recovery path existed, but the workflow CD did not enable `first_turn_timeout_recovery`.

Change attempted: Enabled `first_turn_timeout_recovery` in `coding_project_operator.workflow.json`.

Expected result: First-turn timeout should proceed into compact mutation-only recovery instead of fail-closing immediately.
