# Coding Workflow Experiment Ledger

Purpose: Preserve coding workflow experiment decisions independent of chat context. This ledger tracks hypotheses, changes, test evidence, findings, and decisions so unstable ideas are not accidentally retried or promoted.

Status values:
- `promoted`: active and supported by enough evidence to keep using.
- `active_trial`: active temporarily, still under evaluation.
- `dormant`: code or workflow shape exists but is disabled.
- `failed`: should not be retried in the same shape.
- `reverted`: removed or disabled after regression.

## EXP-CODING-014: Post-validation closure controller

Status: `active_trial`

Hypothesis: Long-horizon coding tasks that already have mutation and validation receipts should not rely on the model to remember final bookkeeping. Runtime should use a generic post-validation closure primitive to synthesize declared checkpoint handoff and memory-row artifacts from receipts when the workflow CD opts in.

Change: Added a config-gated model-manifest runtime-lane closure hook. `coding_project_operator` enables it with `post_validation_closure_controller_enabled`; lower-level mutation spines remain unaffected.

Tests:
- Pending. The motivating Level 8 batch showed Infring could implement and validate the subsystem, but failed checkpoint receipt and memory-row closure.

Finding: This targets the observed Level 8 gap without changing source-edit behavior or adding eval-specific cases.

Decision: Keep as active trial until Level 8 reruns show whether closure reliability improves without regressing lower levels.

## EXP-CODING-015: Capability evidence gate

Status: `active_trial`

Hypothesis: Durable coding tasks should not dispatch a manifest that lacks source-level evidence for the generic capability categories implied by the prompt. Claude Code and Codex Level 8 passes both edited a coherent vertical slice before validation: domain model, store, service, operator/report surface when requested, import/export or roundtrip surface when requested, tests, validation, checkpoint receipt, and memory/project-context artifacts.

Change: Added a reusable capability-evidence admission gate to the model-manifest planner. The gate is not level-specific: it activates only when the prompt itself asks for durable/persistent/checkpoint/report/import-export style capability, then checks planned mutation actions for generic categories before dispatch. Missing categories trigger one manifest repair request; if the repair still lacks evidence, the runtime fails closed with structured missing/present/required category data.

Expected benefit: Prevent expensive Level 8-style runs where the model writes only a manifest-like artifact or partial tests while omitting the real product slice. This should convert bad plans into either a corrected pre-dispatch manifest or an honest structured blocker before mutation.

Next: Rerun Level 8 and compare against the post-validation closure controller trial. If failures remain, inspect missing category reports before adding any new heuristic.

## EXP-CODING-016: Checkpointed project-operator tool loop

Status: `active_trial`

Hypothesis: Complex checkpointed project-operator tasks should not start with a monolithic deterministic manifest request. Passing Claude Code and Codex Level 8 traces used an incremental tool-loop shape: discover current project files, retrieve checkpoint memory, mutate a coherent vertical slice across source/tests/operator surface, run validation, then write receipt and memory row. They did not require the model to emit the whole implementation as one pre-tool JSON manifest.

Change: Added a workflow-gated lane-selection primitive, `checkpointed_project_operator_tool_loop_enabled`. When the prompt itself contains checkpoint/resume/memory signals, local project signals, and durable vertical-slice signals, the runtime skips `model_manifest_planner` and falls through to the native agent tool loop. The skipped path adds compact guidance to use native `file_write`/`file_patch` for mutations and `command_run` for validation or required memory CLI calls, preserving Infring receipts while copying the passing frameworks' incremental shape. Simple edit levels and non-checkpointed tasks still keep the existing deterministic lanes.

Expected benefit: Avoids the observed Level 8 infra failure where Kimi timed out for `180s + 90s` before any mutation or capability gate could run. This aligns Infring with the passing-framework runtime pattern without adding Level 8-specific filenames or fixture logic.

Next: Rerun Level 8. If it reaches native tool calls, evaluate product-slice correctness and closure receipts. If it still times out, inspect the native agent loop prompt/tool surface rather than the manifest planner.

## EXP-CODING-017: Durable vertical-slice sufficiency gate

Status: `active_trial`

Hypothesis: Checkpointed durable project-operator tasks should not treat "some source file plus tests plus validation" as enough product evidence. Claude Code and Codex Level 8 passes both completed a source vertical slice before closure: domain record/model, persistent store, service integration, operator/report surface, import/export or round-trip behavior when requested, regression tests, validation, receipt, and memory row.

Change: Extended the generic product-slice evidence layer to derive durable vertical-slice categories from the prompt and changed source receipts. The staged controller now keeps the run in `product_mutation` while those categories are missing, and bounded-direct final evidence includes the same missing product-slice reasons instead of collapsing to only missing receipt paths. Repair prompts now explain missing durable categories as a cohesive source slice.

Expected benefit: Prevents the native tool loop from validating and attempting checkpoint/memory closure after a shallow model/store-only or repository/test-only mutation. This is framework-aligned but not fixture-specific: categories are prompt-derived and content/path-based.

Next: Rerun a single Level 8 smoke. Desired movement: missing product-slice evidence should drive another source-edit turn before validation/closure, rather than ending with only repository/test changes.

## EXP-CODING-018: Checkpointed tool-loop provider timeout

Status: `active_trial`

Hypothesis: Open checkpointed project-operator loops need bounded provider turns. Claude Code and Codex can spend over a minute on Level 8, but they do not leave a single unbounded model call running until the outer worker timeout. Infring must expose a top-level provider timeout on native tool-loop turns, not only nested workflow policy.

Change: Added `checkpointed_tool_loop_provider_timeout_seconds` and inject a top-level `provider_timeout_seconds` into native provider requests. The checkpointed tool loop defaults to 120 seconds per turn while preserving the existing broader wall timeout and partial-progress behavior.

Expected benefit: A stricter vertical-slice gate should produce a bounded structured timeout or repair opportunity instead of a silent 900-second worker stall.

Next: Rerun Level 8. If the provider times out, inspect the partial-progress report and tighten the repair prompt/tool set; do not loosen the durable slice gate unless it blocks valid source mutations.

## EXP-CODING-019: Memory recall versus memory closure gate

Status: `active_trial`

Hypothesis: Checkpoint memory retrieval is discovery context, while checkpoint memory writes are closure. Claude Code and Codex Level 8 passes retrieved checkpoint memory before editing, then wrote a new memory row after validation. Treating every memory CLI command as closure blocks the correct runtime sequence.

Change: Narrowed `native_tool_call_is_memory_closure` so only memory write/ingest/update-style commands are closure-gated. Memory read/get/recall commands are allowed during discovery and source planning.

Expected benefit: The staged controller should no longer block required checkpoint-memory recall before product mutation, while still preventing premature memory-row writes before product/test/validation/receipt evidence is complete.

Next: Rerun Level 8 and check whether the first turn can read memory, then proceed to source vertical-slice mutation rather than bouncing on `staged_controller_requires_product_source_before_memory_closure`.

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

## EXP-CODING-009: First-mutation tool shaping through staged-edit controller

Status: `failed`

Hypothesis: Restricting the first post-bootstrap bounded-edit turn to mutation tools would produce earlier mutation, closer to Claude Code's quick Edit behavior.

Change: Added `first_mutation_tool_shaping` and routed the first post-bootstrap bounded-edit turn through the staged-edit controller/tool menu.

Tests:
- Level 4: pass, but wall `134.8s`, first mutation `87.8s`.
- Level 7: validation failed, wall `232.8s`, first mutation `203.8s`.

Finding: A separate mutation-only staged turn made latency worse and hurt multi-file/export-surface correctness. Reducing the tool menu alone is not enough; it changes the loop shape badly.

Decision: Keep off. Do not retry through staged-edit controller.

Next: If optimizing first mutation, use streaming/partial execution from the normal full tool batch, not a separate prompt-driven staged turn.

## EXP-CODING-010: Provider stream until first mutation call

Status: `failed`

Hypothesis: Stop the normal provider stream as soon as the first complete mutation call appears, then execute it immediately to reduce first-mutation latency without changing the prompt/tool menu.

Change: Added provider-level parsing for the first complete mutation tool-call object inside `tool_calls`, gated by `provider_stream_until_first_mutation_call`.

Tests:
- Level 4: pass, wall `39.0s`, first mutation `38.9s`.
- Level 7: validation failed, wall `170.5s`, first mutation `81.4s`, only one changed file, many follow-up read/list calls.

Finding: First-call partial execution can amputate the rest of a dependent multi-file batch. It gets some mutation earlier but loses source/export/test coherence.

Decision: Keep off. Do not promote first-call partial execution.

Next: Try only a safe mutation-prefix/batch stream parser, or keep waiting for the full balanced batch. Prefix execution must include dependent source/export edits together.

## EXP-CODING-011: Provider stream until mutation prefix

Status: `dormant`

Hypothesis: Stream until a safe mutation prefix, not just the first mutation call, to preserve dependent multi-file edits while reducing first-mutation latency.

Change: Added provider-level parsing for a prefix of at least two complete mutation tool-call objects, gated by `provider_stream_until_mutation_prefix` and `provider_stream_mutation_prefix_min_calls`.

Tests:
- Level 4: pass, wall `25.0s`, first mutation `24.8s`.
- Level 7: pass, wall `97.9s`, first mutation `72.4s`, changed two files.

Finding: Safer than first-call partial execution, but not clearly faster than the stable full-batch path. Level 7 still fell into a slower follow-up/read path before completing the second mutation.

Decision: Keep dormant/off. Do not promote yet.

Next: Only retry if the prefix parser can prove the prefix contains all required source/export/test mutations for the slice, otherwise wait for the full balanced batch.

## EXP-CODING-012: Provider stream until safe mutation prefix

Status: `failed`

Hypothesis: Only stop early when the streamed mutation prefix proves a safe dependent edit pair, such as source plus export surface or source plus test.

Change: Tightened mutation-prefix parsing to require at least two mutation calls with product roles before stopping early.

Tests:
- Level 4: pass, wall `75.2s`, first mutation `75.1s`; effectively full-batch fallback.
- Level 7: failed with `no_successful_mutation`, wall `169.7s`, no changed files.

Finding: Provider-only safe-prefix inference is brittle. It can reject useful streamed batches and fail to recover into mutation.

Decision: Keep off. Do not promote provider-side partial execution.

Next: Use the full balanced-batch path plus runtime batch scheduling. Future speed work should target provider/model latency or semantically complete prompt/context reduction, not partial provider execution.

## EXP-CODING-020: Split checkpointed project-operator timeout budget

Status: `partial`

Hypothesis: Checkpointed project-operator tasks need one larger first provider turn to produce a coherent durable slice, but follow-up repair turns should stay tightly bounded to prevent stalls.

Change: Declared `checkpointed_tool_loop_initial_provider_timeout_seconds` in the project-operator workflow CD and kept the follow-up `checkpointed_tool_loop_provider_timeout_seconds` budget shorter.

Expected behavior:
- First provider turn may spend up to 180s forming the initial batch.
- Later repair turns stay at 120s.
- The runtime remains a reusable timeout primitive, not a Level 8-specific exception.

Observed result:
- Level 8 split-timeout run completed in about 200s instead of hanging.
- The workflow CD correctly exposed `checkpointed_tool_loop_initial_provider_timeout_seconds=180` and `checkpointed_tool_loop_provider_timeout_seconds=120`.
- The first provider turn still did not produce a successful mutation; it attempted `command_run` before product mutation, the staged controller blocked it, and the follow-up turn timed out at 120s.

Decision: Keep the split timeout as a reusable primitive, but it is not sufficient by itself.

Next: Repair the command-before-mutation blocker so the runtime gives a crisp mutation-only next-tool contract after blocking premature validation.

## EXP-CODING-021: Premature validation recovery gate

Status: `partial`

Hypothesis: When a native coding turn tries to run validation before any required product mutation, the runtime should not reopen the full loop. It should block the command and make exactly the next turn a mutation-only recovery turn.

Change: Added `premature_validation_recovery_gate_enabled` to the project-operator workflow CD. The native loop now detects command-before-mutation blocker receipts and routes the next provider turn through a tiny write/patch-only recovery contract and mutation-only tool menu.

Expected behavior:
- Premature `command_run` remains blocked.
- The next turn can only call `file_write` or `file_patch`.
- Validation still runs downstream through runtime automation after a successful mutation.

Next: Rerun Level 8 and check whether the previous `command_run`-before-mutation failure turns into product-slice mutations.

## EXP-CODING-022: First-timeout mutation-only recovery

Status: `partial`

Hypothesis: The same mutation-only recovery primitive should handle first provider timeouts before mutation, not just blocked premature validation. If bootstrap context already exists and a mutation task times out before writing, the next provider turn should be write/patch-only.

Change: Generalized the recovery state to `mutation_only_recovery_gate_enabled`. First-turn timeout recovery now marks the next turn as mutation-only, while the older `premature_validation_recovery_gate_enabled` remains a compatibility alias for command-before-mutation blockers.

Expected behavior:
- First provider timeout before mutation does not reopen broad planning/discovery.
- The next turn has only file write/patch tools.
- The workflow can still fail closed if the model cannot produce a mutation, but it should fail with clearer evidence.

Observed result:
- Level 8 reached blocker receipts for premature `command_run` and repeated reads before mutation.
- The next provider turn was routed through mutation-only recovery.
- The constrained recovery turn still timed out at 120s without a successful mutation receipt.

Decision: Keep the primitive. It correctly narrows recovery, but the recovery context/prompt still needs to become smaller and more executable.

Next: Build a compact mutation-entry packet for recovery turns: task brief, target file contents, missing product categories, and exact allowed JSON tool-call shape, without full receipt payloads.

## EXP-CODING-023: Compact mutation-entry packet

Status: `partial`

Hypothesis: Mutation-only recovery fails because the recovery prompt receives too much native receipt detail. A compact mutation-entry packet should preserve the executable local context while reducing model-facing ambiguity.

Change: Added `compact_mutation_entry_packet_enabled` to the project-operator workflow CD. Mutation-only recovery turns now receive a compact packet containing allowed/forbidden tools, the project root, candidate paths, and prioritized observed file contents instead of full native observation payloads.

Expected behavior:
- Recovery prompt size drops.
- The next provider turn has enough file content to patch/write without rereading.
- The model should emit `file_write` or `file_patch` instead of timing out or trying more discovery.

Observed result:
- Level 8 completed faster than the previous mutation-only recovery attempt, about 143s wall.
- The first provider turn again attempted premature `command_run`, and the staged controller blocked it.
- The compact recovery turn then timed out at 120s without emitting `file_write` or `file_patch`.

Decision: Keep the compact packet primitive as a cleaner context shape, but it is not sufficient by itself.

Next: The recovery turn likely needs an even more executable edit-entry contract: explicit target file roles and a small required batch shape, not just candidate paths and contents.

## EXP-CODING-024: Executable mutation-entry batch shape

Status: `active`

Hypothesis: The compact mutation-entry packet still leaves too much inference work. Recovery should include generic file roles and an explicit write/patch batch shape so the next model turn can act like a coding framework edit primitive.

Change: Extended the compact mutation-entry packet with derived target roles: product source, operator/export surface, and focused tests. The packet now requires the first tool call to be a product-source `file_write`/`file_patch`, recommends full-file `file_write` for small observed files, and shows the exact JSON `tool_calls` shape.

Expected behavior:
- The recovery provider turn has a concrete batch skeleton.
- The model should choose candidate files from observed paths instead of timing out or asking for more reads.
- Validation remains downstream and runtime-controlled.

Observed result:
- Level 8 still failed as `provider_timeout_or_spawn_failure`.
- The first provider turn again attempted premature `command_run`.
- The staged controller blocked it correctly.
- The mutation-only recovery turn timed out at 120s without a mutation receipt.

Decision: Keep the executable packet as a clearer primitive, but it is still not enough for Level 8 with the current provider/model path.

Next: The remaining gap appears to be not packet clarity but provider/edit-entry behavior. Compare the actual recovery prompt/timing probe against Codex/Claude edit-entry traces, then consider moving edit-entry generation to a deterministic manifest/action planner or a stronger model lane.

## EXP-CODING-025: Controlled shell edit batch

Status: `active`

Hypothesis: Codex and Claude Code pass Level 8 with the same weak model because their edit-entry primitive allows shell-style multi-file write batches. Infring should assimilate that behavior without losing receipts by allowing pre-mutation `command_run` only when it is clearly a shell edit batch, then synthesizing normal mutation receipts from changed target files.

Change: Added `controlled_shell_edit_batch_enabled` to the project-operator workflow CD. Mutation recovery now exposes `command_run` alongside file tools, but validation-like commands remain blocked before mutation. If a `command_run` contains project-file heredoc writes such as `cat > path <<'EOF'`, the runtime executes it and synthesizes `file_write` receipts for changed target files.

Expected behavior:
- Models that naturally emit shell write batches can mutate without fighting native JSON file content.
- Governance still sees receipt-backed mutations.
- Validation commands before mutation remain blocked.

Next: Rerun Level 8 and inspect whether the first successful mutation comes from a synthesized shell-edit receipt.

## EXP-CODING-026: Enable first-turn timeout recovery for project operator

Status: `active`

Hypothesis: The controlled shell-edit lane cannot help when the initial provider turn times out before any tool call and the workflow CD does not opt into first-turn timeout recovery.

Change: Enabled `first_turn_timeout_recovery` in the project-operator workflow CD so first-turn provider timeouts can fall into the existing compact mutation/shell-edit recovery lane instead of fail-closing immediately.

Expected behavior:
- A first-turn provider timeout with bootstrap context should schedule mutation-only recovery.
- The recovery turn can use native file tools or controlled shell edit batch.
- Failures should carry more actionable receipt evidence instead of ending after only bootstrap reads.

Next: Rerun Level 8 and inspect whether the runtime reaches mutation recovery after first-turn timeout.

## EXP-CODING-027: Level 8 staged project-operator model import

Status: `model_updated`

Hypothesis: Level 8-class checkpointed project work should be modeled as a
multi-stage project-operator controller, not as one monolithic product-mutation
provider turn. Passing reference systems make progress by moving through
receipt-backed stages: context, memory recall, product/source edits,
operator/report edits, tests, validation, checkpoint receipt, memory write, and
final synthesis.

Reference run:

- `references/coding-agent-systems/runtime_trace_harness/reports/level8_external_frameworks_20260527_160914.json`

Observed result:

- `claude-code` passed in `284s` with rich tool traces: `6394` stream events
  and `86` tool uses.
- `grok` passed in `196s`, but trace visibility is weaker because the stream is
  mostly text.
- `forgecode` reached a near pass in `146s`, missing only validation-result
  preservation in the memory row.
- `codex` implemented code in `213s`, but missed memory-row closure.
- `aider` did not run because `/tmp/infring-baselines-aider` was missing.
- Native Infring now performs bounded context and checkpoint memory recall
  quickly, but still times out before mutation when asked to generate the whole
  product slice in one turn.

Model change:

- Added the `checkpointed_project_operator_controller` contract to
  `docs/workspace/coding_runtime_behavioral_model_v2.md`.
- The controller stages are generic: `context_snapshot`,
  `checkpoint_memory_recall`, `product_source_slice`,
  `operator_surface_slice`, `test_slice`, `validation`,
  `checkpoint_receipt`, `checkpoint_memory_write`, and `final_synthesis`.
- Each stage must expose a narrow tool menu, compact evidence packet,
  receipt-backed exit criteria, timing, and structured blocker behavior.

Decision:

Use this model as the basis for the next patch. Do not patch by adding more
Level 8-specific prompt text. The implementation should split the project
operator lane into primitive stages while preserving lower-tier contracts.

Next:

Patch the native project-operator runtime so checkpointed work advances through
the staged controller and asks the model for smaller source/operator/test
mutation slices instead of a single all-in-one mutation turn.

## EXP-CODING-028: Profile-scoped activation audit after Level 2 regression

Status: `model_updated`

Hypothesis: The unified model is still correct, but the implementation became
corrupted because higher-profile primitives were added into the shared native
loop without strong activation boundaries. This caused Level 8-oriented
controllers and guards to leak into Level 2 bounded edits.

Observed failure:

- Level 2 bounded existing-project edits were pulled through high-level
  manifest/evidence/repair machinery.
- Valid primitives such as source-owner preservation, shell edit batches,
  staged execution, completion evidence, and capability evidence became active
  together and produced contradictory repair pressure.
- Tool alias gaps such as `target_file_path` and `patch_content` amplified the
  loop, but they were not the root architectural issue.

Model change:

- Added `docs/workspace/coding_profile_activation_audit.md`.
- Added machine-readable activation policy at
  `orchestration/src/control_plane/workflows/lab/composites/coding/coding_profile_activation_matrix.json`.
- Updated the behavioral model and implementation spec to require
  `profile_min` for every primitive/guard.
- Added mandatory lower-profile smoke after higher-profile changes.

Decision:

Stop patching Level 2 locally. The next runtime patch should make selected
profile and active primitive set controller-visible, then make Profile 4+ and
Profile 5+ primitives dormant below their declared profile.

Next:

Patch activation boundaries, rebuild, then run a lower-profile smoke ladder
from Profile 0 upward before returning to Level 8.

## EXP-CODING-029: Compact mutation-entry packet for Level 6/7 no-mutation timeout

Status: `reverted_no_positive_delta`

Hypothesis: Claude Code, Codex, ForgeCode, and Aider traces all point toward a
bounded editable context followed by a compact mutation artifact before
validation. Enabling the existing `compact_mutation_entry_packet_enabled`
primitive in the lab coding spine might reduce the first provider turn enough to
restore mutation on Level 6/7 without adding case-specific behavior.

Reference evidence:

- Level 6 reference run:
  `references/coding-agent-systems/runtime_trace_harness/reports/level6_reference_frameworks_trace_20260525_201057.stdout.json`
- Level 7 reference run:
  `references/coding-agent-systems/runtime_trace_harness/reports/level7_all_frameworks_20260525_233558.stdout.json`
- Aider source pattern:
  `references/coding-agent-systems/aider/aider/coders/base_prompts.py`,
  `references/coding-agent-systems/aider/aider/coders/editblock_prompts.py`,
  `references/coding-agent-systems/aider/aider/coders/editblock_coder.py`

Change attempted:

- Temporarily enabled `compact_mutation_entry_packet_enabled` in
  `local_coding_phase1_mutation_spine.workflow.json`.

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/compact_mutation_packet_l6_l7_20260529.json`
- Level 6 remained `0/1`, failure `no_successful_mutation`, wall `105217ms`.
- Level 7 remained `0/1`, failure `no_successful_mutation`, wall `105388ms`.
- Prompt/system size improved materially:
  Level 7 first turn changed to `prompt_chars=3958`, `system_chars=287`,
  `tool_count=2`.
- But both turns still timed out before mutation:
  first provider turn `60015ms`, retry `45109ms`.

Decision:

Reverted the workflow flag. This was directionally useful as a diagnostic
because it proved context compaction alone is not enough, but it did not produce
an obvious positive capability delta under the current provider/model path.

Next:

Consult the passing framework traces for the missing implementation behavior:
the winning systems do not merely compact context; they also make the first edit
artifact operationally cheap. The next single-change candidate should target the
mutation executor shape itself, most likely a deterministic local edit artifact
or shell-edit batch primitive that applies a model-produced edit payload without
another long generic provider/tool turn.

## EXP-CODING-030: Full mutation-entry artifact trio for Level 6/7 no-mutation timeout

Status: `partial_positive_keep_under_review`

Hypothesis: The compact packet alone failed because it compressed context but
did not activate the full edit-artifact behavior used by faster reference
systems. The official coding operator already carries the full trio:
`mutation_only_recovery_gate_enabled`, `compact_mutation_entry_packet_enabled`,
and `controlled_shell_edit_batch_enabled`. Activating that trio in the lab
phase-1 spine should more closely match Codex-style `command_execution` and
Claude-style fast edit behavior.

Change:

- Enabled `mutation_only_recovery_gate_enabled`.
- Enabled `compact_mutation_entry_packet_enabled`.
- Enabled `controlled_shell_edit_batch_enabled`.
- Enabled `premature_validation_recovery_gate_enabled`.

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/mutation_entry_artifact_trio_l6_l7_20260529.json`
- Level 6 changed from `no_successful_mutation` to a real first mutation:
  `time_to_first_mutation_ms=43824`.
- Level 6 touched the expected source files:
  `orderflow/__init__.py` and `orderflow/attempts.py`.
- Level 6 still failed overall because the implementation was semantically
  wrong and post-mutation repair did not converge before the harness timeout.
- Level 7 remained `no_successful_mutation` with the same 60s/45s timeout
  shape.

Decision:

Keep this as a partial positive only if the next patch targets the newly exposed
failure: after the first mutation, the runtime falls back into the full generic
8-tool loop instead of staying in a narrow validation-repair lane. If that next
patch does not improve Level 6 repair or Level 7 first mutation, revert the trio
with its follow-up patch.

Next:

Consult the reference traces for post-mutation repair behavior. The likely fix
is not more setup; it is a bounded repair controller that keeps post-mutation
turns narrow: changed files, failed validation output, edit-only tools, and a
single validation closeout.

## EXP-CODING-031: Public contract packet from passing Level 6/7 framework traces

Status: `positive_keep`

Reference behavior:

- Claude Code, Codex, and ForgeCode all read tests and semantic probes before
  the first source mutation on Level 6/7.
- The useful primitive was not the fixture domain; it was preserving call-site
  shape from local tests/probes as a public contract. Examples include
  constructor calls, method calls, `__all__` exports, assertion output shapes,
  and semantic-probe API usage.

Patch:

- Added generic `public_contract_packet_enabled` to the lab mutation spine.
- Extended compact mutation-entry packets with observed public/test contract
  lines from read receipts.
- Extended project-operator context packets with
  `public_contract_lines_to_preserve`.
- Extended completion repair prompts so failed-validation repair sees the same
  public/test contract lines instead of relying only on broad observations.

Expected impact:

Level 6 should avoid or repair wrong API-shape mutations such as implementing a
method with arguments that contradict observed tests/probes. Level 7 should get
the same benefit for module/export/reporting surfaces. This is a generic
contract-preservation primitive, not an eval-specific case branch.

Measurement:

- Rebuilt `target/debug/xtask`.
- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/public_contract_packet_rebuilt_l6_l7_20260529.json`
- Level 6 remained `0/1`, failure `no_successful_mutation`, wall `105160ms`.
- Level 7 remained `0/1`, failure `no_successful_mutation`, wall `105150ms`.
- First-turn prompt grew to roughly `6.8k` chars and both runs kept the same
  `60s + 45s` provider timeout shape.

Decision:

Do not use full public/test contract dumps in the first-mutation packet. Keep
the primitive for repair/context packets, where preserving observed call sites
is useful after there is a concrete failed mutation or validation failure to
repair.

## EXP-CODING-032: Defer public contract dump from first mutation and keep tiny import contract

Status: `reverted_no_positive_delta`

Patch:

- Added `public_contract_first_mutation_packet_enabled=false` to keep the
  heavier public-contract dump out of the first mutation path.
- Kept `public_contract_packet_enabled=true` for later repair/context packets.
- Added a tiny generic import/API failure contract to the compact first-mutation
  packet, derived from validation stderr, so missing imported symbols point the
  model toward owner source/export mutation without carrying whole test/probe
  call-site dumps.

Expected impact:

Restore the smaller first-mutation prompt size while preserving the most useful
signal from the passing framework traces: when validation says a public symbol
cannot be imported, the next action should define/export that symbol in the
owner module before validation is rerun.

Measurement:

- Rebuilt `target/debug/xtask`.
- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/deferred_public_contract_import_contract_l6_l7_20260529.json`
- Level 6 remained `0/1`, failure `no_successful_mutation`, wall `105156ms`.
- Level 7 remained `0/1`, failure `no_successful_mutation`, wall `105108ms`.
- First-turn prompts dropped from the failed full-contract experiment to roughly
  `4.2k` chars, but the provider still hit the same `60s + 45s` timeout shape
  with no mutation receipts.

Decision:

Reverted the first-mutation import-contract addition because it did not produce
an obvious positive delta. Keep the public-contract primitive only in
repair/context packets for now. The next candidate should change the execution
shape, not add more prompt content to first mutation.

## EXP-CODING-033: Deterministic Python import-surface seed mutation

Status: `reverted_mixed_negative_delta`

Hypothesis:

The Level 6/7 failure is now earlier than semantic repair: the provider times
out before any file mutation. Passing frameworks all unblock the public import
surface before deeper behavior repair. A primitive runtime seed can make that
first mutation deterministic when validation already proves a missing Python
public import, then leave behavior completion to the normal model/repair loop.

Patch:

- Added `python_import_surface_seed_mutation_enabled=true` to the lab mutation
  spine.
- After bootstrap validation, if stderr contains generic Python
  `cannot import name '<symbol>' from '<module>'` failures and no mutation has
  occurred, the runtime now:
  - locates the module/export file from the project root,
  - chooses an observed package source owner file,
  - appends generic public API stubs for missing imported symbols,
  - exposes those symbols through the package export surface,
  - records normal `file_write` mutation receipts through native tooling.

Non-goal:

This seed does not implement domain behavior or encode eval-specific APIs. It
only converts receipt-backed missing-public-import failures into a concrete
owner-source/export mutation so later repair can operate on real code instead
of timing out before mutation.

Measurement:

- Rebuilt `target/debug/xtask`.
- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/python_import_surface_seed_l6_l7_20260529.json`
- Level 6 passed `1/1`, wall `30790ms`, first mutation `30628ms`.
- Level 7 passed `1/1`, wall `26787ms`, first mutation `25409ms`.
- The old `no_successful_mutation` failure moved to a successful seed mutation
  plus model-completed implementation/validation.
- Level 6 wrote `orderflow/attempts.py` and `orderflow/__init__.py`.
- Level 7 wrote `warehouse/items.py` and `warehouse/__init__.py`.

Decision:

Keep this patch. It is the first recent Level 6/7 change that clearly improves
capability instead of adding prompt weight: the runtime now deterministically
crosses the missing-import first-mutation boundary, then uses the model for
actual behavior completion and validation.

## EXP-CODING-034: Batch public import-surface seed from observed imports

Status: `reverted_negative_delta`

Hypothesis:

The Level 6/7 `4/5` stability result after `EXP-CODING-033` had one failure
where the runtime seeded one missing Python public symbol, then validation later
failed on another symbol from the same `from package import ...` surface. A
more complete primitive might parse observed validation/test/probe import lines
and seed all missing public symbols for a local package in one deterministic
batch.

Patch attempted:

- Extended the Python import-surface seed to collect `from module import A, B, C`
  statements from the prompt, validation details, and receipt payload strings.
- Fed that observed public import contract into the existing seed mutation path.

Measurement:

- Rebuilt `target/debug/xtask` successfully.
- Started a 5x Level 6/7 batch and stopped it after two paired runs because the
  speed regression was already clear.
- Reports:
  `references/coding-agent-systems/runtime_trace_harness/reports/l6_l7_5x_after_public_import_surface_contract_20260529_run1.json`
  `references/coding-agent-systems/runtime_trace_harness/reports/l6_l7_5x_after_public_import_surface_contract_20260529_run2.json`
- Run 1 passed Level 6 and Level 7, but Level 7 took `183840ms` with first
  mutation at `73ms`.
- Run 2 failed Level 6 and Level 7 with `runtime_timeout`; both mutated at
  roughly `71ms`, but behavior remained incomplete and the provider/runtime
  exhausted the harness budget.

Decision:

Reverted this patch. It improved initial import-surface breadth but made the
workflow worse on the architectural metric that matters here: deterministic
stubs happened too early and encouraged a long timeout/repair path instead of a
small model-owned implementation. The next attempt should not broaden generic
stub seeding. It should preserve the `EXP-CODING-033` seed as a narrow unblocker
and improve the post-seed repair handoff or timeout handling as a separate
primitive.

## EXP-CODING-035: Post-seed repair handoff receipt

Status: `reverted_mixed_negative_delta`

Hypothesis:

Keep the narrow `EXP-CODING-033` Python import-surface seed, but add a tiny
post-seed repair handoff receipt so the model treats seeded placeholders as a
repair target rather than final implementation. This should preserve the good
first-mutation unblock while reducing incomplete-stub timeout paths.

Patch attempted:

- When the narrow import seed fired, appended a synthetic runtime handoff receipt
  saying the runtime seeded minimal placeholders and the next model turn should
  implement observed validation/probe behavior in owner source, preserve exports,
  and run validation.

Measurement:

- Rebuilt `target/debug/xtask` successfully.
- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/l6_l7_5x_after_import_seed_repair_handoff_20260529_aggregate.json`
- Level 6 improved to `5/5`, average wall `60243ms`.
- Level 7 regressed to `3/5`, average wall `113878ms`, with one
  `validation_failed` and one `runtime_timeout` failure.

Decision:

Reverted this patch. The handoff helped Level 6 reliability but hurt the more
important scalability signal at Level 7. The next repair should avoid adding
more observation/prompt payload after seed. A better candidate is a controller
primitive that detects seeded-placeholder validation failures and routes one
bounded repair turn with compact validation stderr plus owner file only.

## EXP-CODING-036: Compact post-seed repair context

Status: `reverted_negative_delta`

Hypothesis:

Instead of adding a handoff receipt after the narrow Python import-surface seed,
route the first model repair turn through a compact controller context containing
only failed validation plus the owner/export Python files touched by the seed.
This should reduce post-seed observation bloat while keeping the deterministic
first-mutation unblock.

Patch attempted:

- Tracked whether the Python import-surface seed fired during bootstrap.
- If it fired, replaced the normal all-receipts observation prompt with a compact
  seed repair context built from failed validation details and seed file-write
  receipt paths.
- Added a short seed-specific bootstrap rule telling the model to repair behavior
  in the touched owner/export files before validation.

Measurement:

- Rebuilt `target/debug/xtask` successfully.
- Started Level 6/7 batch:
  `references/coding-agent-systems/runtime_trace_harness/reports/l6_l7_5x_after_import_seed_compact_repair_context_20260529_run1.json`
- Stopped the run before the first paired attempt completed because it exceeded
  roughly five minutes without producing the first summary, a clear regression
  versus the current baseline where paired runs usually complete in about one to
  two minutes.

Decision:

Reverted this patch. Compacting the observation this way appears to remove
useful context or route the weak model into a slow path. The next candidate
should not alter the first post-seed prompt shape. Prefer measuring and fixing
provider/runtime timeout classification or adding a rollback/repair-after-timeout
controller outside the first repair turn.

## EXP-CODING-037: Runtime failure taxonomy for seeded repair failures

Status: `patched_measurement_positive_diagnostic`

Hypothesis:

The biggest current cross-level failure is no longer basic file mutation. The
runtime often crosses the first-mutation boundary, then fails during semantic
repair after a deterministic Python import-surface seed. Treating these as
plain `runtime_timeout` or `validation_failed` hides the actual next primitive
we need to build.

Patch:

- Added `runtime_failure_analysis` to native agent receipts and run journals.
- Added generic validation failure classes:
  `import_surface_missing`, `attribute_missing`, `type_error`,
  `file_not_found`, `assertion_mismatch`, `command_timeout`, `syntax_error`,
  and `unknown_validation_failure`.
- Added seeded-repair classification when a Python import-surface seed receipt
  exists and repair/validation remains unresolved.
- Updated Level 2 and Level 3+ coding harnesses to preserve runtime analysis and
  fall back to check-detail stderr classification.

Measurement:

- Rebuilt `target/debug/xtask` successfully.
- Python-compiled Level 2 and Level 3+ harnesses successfully.
- Smoke report:
  `references/coding-agent-systems/runtime_trace_harness/reports/failure_taxonomy_l6_l7_smoke_20260529.json`
- Level 6 passed `1/1`.
- Level 7 failed `1/1` and now reports `seeded_repair_timeout` instead of a
  generic timeout. The trace shows seed mutation at `81ms`, then provider
  timeout on the first post-seed turn.

Decision:

Keep this diagnostic patch. It does not alter repair behavior, but it gives the
higher workflow and future patches the right target: the next behavior primitive
should handle `seeded_repair_timeout`/`seeded_repair_import_surface_missing`, not
broad `runtime_timeout`.

## EXP-CODING-038: Seeded timeout import-surface recovery after provider timeout

Status: `reverted_not_exercised`

Hypothesis:

After `EXP-CODING-037`, the clearest failure class was
`seeded_repair_timeout`: Python import-surface seed mutation succeeds, then the
post-seed model repair path times out and validation remains incomplete. A small
bounded recovery could run validation after that timeout, seed only the next
concrete missing Python public import, validate again, and return structured
partial progress.

Patch attempted:

- Added a timeout-only recovery hook inside `native_tool_recovery_or_partial_progress`.
- Refactored the Python import-surface seed into a reusable helper so the timeout
  branch could seed from the latest validation details without broadening the
  initial seed.

Measurement:

- Rebuilt `target/debug/xtask` successfully.
- Smoke reports:
  `references/coding-agent-systems/runtime_trace_harness/reports/seeded_timeout_recovery_l6_l7_smoke_20260529.json`
  `references/coding-agent-systems/runtime_trace_harness/reports/seeded_timeout_recovery_l7_resmoke_20260529.json`
- Level 6 passed `1/1` in the combined smoke.
- Level 7 passed `2/2` across the combined and focused smoke runs, but neither
  run emitted the timeout-recovery seed receipt. The attempted branch was not
  actually exercised; the passes came from the normal model/recovery path.

Decision:

Reverted this patch. It did not show an attributable positive delta, and leaving
an unexercised recovery branch in the runtime would violate the current
one-measurable-change rule. The next attempt should target an actually observed
hot path from the traces: the first post-seed provider turn is too large/slow
(`~11k` prompt chars plus `~9k` observation chars) and sometimes reaches the
90s provider timeout before emitting tool calls.

## EXP-CODING-039: Successful evidence closure reconciler

Status: `rejected_after_measurement`

Hypothesis:

The 1-7 sweep after seeded failure taxonomy showed several cases where the
system produced real code, changed files, and passed validation/probes, but the
runtime still returned `partial_blocked` or a generic missing mutation/evidence
status. The immediate blocker is evidence closure, not raw code generation.

Patch:

- Added a bounded runtime success reconciler before `partial_blocked` terminal
  status is returned.
- The reconciler promotes a bounded direct edit to success only when:
  - there is mutation evidence from native file tools or a successful controlled
    shell edit command,
  - there is successful validation after the latest mutation evidence,
  - remaining gaps are receipt-closure gaps such as missing product/test mutation
    receipts or missing changed-path evidence.
- Validation/import/semantic failures are not bypassed.

Expected impact:

Levels that already pass validation/probes but fail terminal status should stop
being reported as generic `partial_blocked`. This specifically targets the
Level 2, Level 3, Level 5, and Level 6 failure shape from the latest sweep.

Decision update for EXP-CODING-039:

Status: `reverted_negative_delta`

Measurement update:

- Reran the 1-7 sweep after the evidence reconciler patch.
- Level 5 improved from fail to pass, but Level 4 regressed from pass to
  `no_successful_mutation`, Level 6 regressed to `runtime_timeout`, and Levels 2,
  3, and 7 were slower or still failing.
- The patch did not satisfy the one-measurable-change keeper rule because it did
  not produce a clear monotonic improvement across lower and higher levels.

Decision:

Reverted the evidence reconciler. The next patch should target the largest
observed trace-backed failure, not broad terminal-status promotion.

## EXP-CODING-040: Existing-project shell edit receipt recognition

Status: `rejected_after_measurement`

Hypothesis:

The largest failure class across the latest 1-7 sweeps is successful external
mutation that does not close as native mutation evidence. Level 3 is the
cleanest trace: `command_run` executes `cat > file <<'EOF'` shell edit batches,
the expected files change, validation passes, semantic probes pass, but the run
fails with `runtime_lane_required_native_mutation_receipt_missing`.

Trace finding:

- The shell-edit path parser already recognizes heredoc writes.
- The receipt synthesis path only activates when
  `native_tool_controlled_shell_edit_batch_paths` recognizes the command.
- That recognizer required profile `4`, while Level 3 existing-project edits run
  under profile `2`.
- Result: profile-2 agents could execute safe shell edit batches, but those
  mutations were invisible to native receipt closure.

Patch:

Lower controlled shell-edit batch recognition from profile `4` to profile `2`,
the first existing-project edit profile. This keeps micro/new-file lanes out of
the shell-edit path while allowing existing-project shell edit batches to
produce synthetic `file_write` receipts.

Expected impact:

Levels that successfully mutate via controlled heredoc shell edit batches should
close with receipt-backed mutation evidence instead of failing as ghost
mutations. This is a primitive receipt-synthesis fix, not a level-specific
exception.

Measurement:

- Rebuilt `target/debug/xtask` before measurement after detecting an initial
  stale-binary run.
- Reran a 1-7 sweep with Infring and `kimi-k2.6:cloud`.
- Level 5 passed, but Levels 2, 3, 4, 6, and 7 still failed.
- Level 3 continued to mutate files and pass validation/probes, but hit native
  tool loop timeout instead of closing cleanly.
- Level 2 and Level 4 showed no-successful-mutation failure shapes.

Decision:

Reverted the profile-threshold change. The trace showed the actual primitive gap
was not the profile threshold alone; safe shell edit commands can execute without
being normalized into native mutation receipts. The next patch should normalize
already-executed safe shell edit commands into receipt evidence rather than
broadly changing which profiles allow shell edit batches.

## EXP-CODING-041: Safe shell edit receipt normalization

Status: `rejected_after_measurement`

Hypothesis:

The largest active failure class is not raw code generation. Several levels show
the agent either mutating successfully but failing to close, or entering long
provider loops after concrete edit/validation evidence exists. The Level 3 trace
is the cleanest case: repeated `command_run` heredoc edit batches changed the
right files and validation/probes passed, but each batch produced only a
`command_run` receipt and no native mutation receipt.

Patch:

- Restored controlled shell-edit profile gating to its previous profile `4`
  behavior.
- Added a separate safe shell-edit receipt-normalization path.
- If a successful `command_run` already executed a safe local shell edit batch
  against project files, synthesize native `file_write` receipts from those
  paths.
- Reused the same safe-path parser and Python/API shape safety checks instead of
  adding level-specific exceptions.

Expected impact:

Existing-project runs that already perform safe shell edit mutations should
close through the existing mutation-plus-validation controller instead of
looping until timeout or failing with missing native mutation evidence.

Patch update:

The first measurement still failed Level 3 with runtime timeout. The trace
showed why: shell edit commands that write test files contain heredoc body text
such as `import unittest`, and the validation-command guard scanned the whole
command including heredoc bodies. That caused a safe write batch to be rejected
as if it were a validation command. The patch now classifies validation using
only shell command control lines outside heredoc bodies.

Measurement update:

Level 3 still failed after the heredoc control-line patch. The run showed real
progress (`successful_mutation_in_batch: true`) but still timed out because the
legacy runtime did not immediately run/record validation and close through one
authoritative controller. This confirms that continuing edge patches inside the
legacy controller maze is lower ROI than rebuilding around a single execution
spine.

Decision:

Do not keep extending this patch path as the main foundation. Preserve the
useful primitive insight (safe shell edit writes can normalize into mutation
evidence), but move control authority to `coding_execution_spine_v1`.

## EXP-CODING-042: Semi-aggressive coding controller rebuild

Status: `started`

Hypothesis:

The current coding runtime has enough useful primitives, but too many competing
controllers. Reference-system traces consistently point to a simpler behavior:
bounded context, mutate once, validate, repair only from concrete evidence, and
close aggressively when receipts satisfy the task contract.

Action:

- Added `coding_execution_spine_v1` as the authoritative reset target.
- Added a Rust spine module with normalized context, mutation, validation,
  public-interface, blocker, and closure evidence.
- Added a lab Workflow CD declaring the spine as a Level 0 primitive.
- Updated the implementation spec and primitive split doc so legacy controllers
  must become evidence adapters or migration debt.
- Added a receipt adapter that projects existing native tool receipts into the
  spine without knowing eval levels or fixture symbols.
- Enabled the spine only for low-complexity lanes corresponding to Levels 1-3.
- Added a runtime hook that lets the spine request auto-validation and close
  success when normalized evidence satisfies the task contract.

## EXP-CODING-043: First mutation artifact lane v1

Status: `measured_partial_progress_pending_runtime_enforcement`

Hypothesis:

After the spine integration, Level 3 failed earlier and more cleanly:
`context_ready -> provider timeout before first mutation`. That means the
closure spine is not enough by itself. The runtime needs a dedicated first
mutation stage that receives already-loaded context and emits only mutation
tool calls before the broad open loop can take over.

Patch:

- Added `first_mutation_artifact_lane_v1` as a Level 0 primitive Workflow CD.
- Added a Rust module for the lane prompt, tool filter, and activation policy.
- Routed only `existing_project_patch` through the lane before the open native
  tool loop.
- The lane allows only `file_write` and `file_patch`.
- Successful lane mutations feed into `coding_execution_spine_v1`, which may
  request validation and close success.
- If the lane times out or emits no mutation, the run closes as a lane failure
  instead of falling into the legacy open loop.

Expected impact:

Level 3 should fail faster and more legibly if the first mutation model turn is
still weak, or pass through the spine if the lane emits a valid mutation.

Measurement update:

Five Level 3 runs produced `3/5` pass. The pass path was clean: context
bootstrap, first mutation lane, source/test patch receipts, auto-validation, and
spine closure. The failures were owned by the new stage:

- one first-mutation lane timeout with no mutation,
- one incomplete mutation slice that changed source only while the task required
  a test change.

Patch update:

Added generic target-artifact role coverage to `coding_execution_spine_v1`.
Mutation evidence now carries artifact roles such as `source`, `test`, and
`doc`; the task contract requires `test` when the prompt requests test changes.
The spine now returns `repair` instead of `close_success` when required artifact
roles are missing.

Promotion rule:

Route only Levels 1-3 through this spine first. Do not optimize Level 4+ until
Level 1-3 are stable and monotonic.

## EXP-CODING-044: First mutation format retry

Status: `rejected_after_measurement`

Hypothesis:

The remaining Level 3 failure is now stage-owned: the first mutation artifact
lane sometimes returns no successful mutation after context is already loaded.
Reference systems solve this class with a narrow edit-format retry rather than
falling into a broad planning loop.

Patch:

- Added one bounded retry inside `first_mutation_artifact_lane_v1`.
- The retry is activated only when the initial lane response produces no
  successful mutation receipt.
- The retry keeps the same lane ownership and allowed tools: `file_write` and
  `file_patch` only.
- The retry has its own shorter provider timeout and timeline events.
- The broad native tool loop remains bypassed for this failure class.

Measurement:

Five Level 3 runs produced `4/5` pass, but the only no-mutation case exercised
the retry and still failed: the first lane returned no mutation around `26.8s`,
the retry returned no mutation around `56.8s`, and the run remained blocked.

Decision:

Reverted the retry behavior because it added wall time to the failure path
without repairing the failure it owned. Keep the lesson: the next primitive
should improve first-pass mutation emission or JSON/tool-call extraction, not
add another model turn after a no-output lane response.

## EXP-CODING-045: First mutation compact context packet

Status: `rejected_after_measurement`

Hypothesis:

The no-mutation failure is an output-contract failure, not a file-tool failure.
The model received enough context and reasoned out the correct edits, but its
visible output began with `Thinking...` and never produced parsed tool calls.
The first-mutation lane should avoid generic observation wording that permits
final-answer behavior and should present a tiny edit packet with a strict
visible-output contract.

Patch:

- Replaced the generic native observation packet inside
  `first_mutation_artifact_lane_v1` with a lane-specific compact edit-context
  packet.
- Removed inherited wording that said the model could provide a final answer.
- Hardened the lane system/prompt contract: the first visible byte must be `{`
  and the output must be only `{"tool_calls":[...]}`.
- Kept the lane primitive narrow: no retries, no broader controller fallback,
  no eval-specific symbols.

Expected impact:

Level 3 should convert more first-mutation turns into immediate parsed
`file_patch` or `file_write` calls without adding another model turn.

## EXP-CODING-046: Native tool-call parser balanced-object scan

Status: `rejected_after_measurement`

Hypothesis:

Some constrained tool lanes still produce visible reasoning before the final
JSON object. The native parser previously considered fenced blocks, the first
balanced JSON object, and the full raw output. That can miss a later valid
`{"tool_calls":[...]}` object when an earlier prose example or placeholder
object appears first.

Patch:

- Changed native tool-call parsing to scan all balanced JSON objects in the
  response, bounded to 64 objects.
- Kept the behavior generic across native tool lanes; this is not a Level 3 or
  first-mutation special case.
- Removed the leftover mutable binding from the reverted first-mutation retry.

Expected impact:

If the provider output contains a later valid `tool_calls` object after
reasoning text, the runtime should materialize it instead of reporting
`no_successful_mutation`.

## EXP-CODING-047: Receipt-satisfied closeout

Status: `measured_partial_success_kept`

Hypothesis:

Level 2 produced real source/test mutations, passed validation, passed the
semantic probe, and exposed expected symbols, but the runtime still ended
`partial_blocked`. The closeout primitive was over-trusting prompt evidence-gap
strings and under-trusting concrete receipts.

Patch:

- Added a receipt-satisfied closeout gate after runtime repair-reason
  computation.
- The gate closes only when successful mutation receipts exist, validation
  passed after the latest mutation, no checkpointed project stage is live,
  required test mutations exist when tests were requested, and semantic probe
  success exists when the prompt requested a semantic probe.
- The gate emits a bounded-direct-edit success marker and synthetic completion
  evidence instead of falling through to `partial_blocked`.

Expected impact:

Level 2 should stop failing closeout after concrete validation/semantic evidence
has already satisfied the task contract.

Measurement:

The follow-up Level 2 run did not reach closeout at all: it timed out around
`240s` with zero native receipts and no mutation. A Level 3 sanity run still
passed, but regressed to around `114s` because the first mutation lane emitted
an incomplete source-only mutation and the runtime needed a second provider
turn to finish tests and validation.

Decision:

Reverted the closeout patch. The current largest failure is not closeout; it is
first-mutation slice completeness and pre-receipt/provider stall behavior.

## EXP-CODING-048: First mutation required artifact roles

Status: `rejected_after_measurement`

Hypothesis:

Slow Level 3 passes are often caused by an incomplete first mutation slice: the
first lane mutates source only, auto-validation cannot satisfy the task, and
the runtime falls into a second long provider turn to add tests and semantic
proof. The first mutation primitive should express required artifact roles and
fail fast when the lane emits an incomplete vertical slice.

Patch:

- Added required artifact roles to the first-mutation context packet.
- Required roles are generic: `source` is always required; `test` is required
  only when the prompt explicitly asks to add tests, unittest coverage, test
  changes, or regression tests.
- If a first-mutation lane batch mutates but omits a required role, the runtime
  returns a structured incomplete-role block instead of falling into a broad
  second provider turn.

Expected impact:

Level 3 should either complete source+test in the first lane or fail fast with
`first_mutation_artifact_lane_v1_incomplete_required_roles`, avoiding slow
two-turn completions.

Measurement:

One Level 3 run completed successfully with source and test mutations in the
first lane, but wall time was still around `36s`. The patch did not restore the
best compact-packet timings, and the extra role machinery risks turning a
prompt-shaping issue into another controller branch.

Decision:

Reverted the runtime role enforcement. Keep the finding in the ledger, but use
the compact context packet plus parser scan as the recoverable baseline before
trying another first-mutation prompt simplification.

## EXP-CODING-049: Executable tool-call candidate selection

Status: `patched_refined_pending_measurement`

Hypothesis:

Some first-mutation failures are not missing edit intent. The provider may
think aloud, echo the schema example with placeholder arguments, then emit the
real patch JSON later. The parser should not stop at the first parseable
placeholder/example call, and it should recover complete nested tool-call
objects even when an outer JSON envelope is incomplete.

Initial patch:

- Keep scanning JSON candidates after placeholder-only tool calls such as
  `/absolute/path`, `exact observed text`, or `replacement text`.
- Prefer the first executable candidate instead of the first merely parseable
  candidate.
- Continue balanced-object scanning past incomplete outer objects so nested
  complete tool-call objects can still be recovered.

Expected impact:

Level 3 no-mutation failures caused by echoed examples or broken outer
`tool_calls` envelopes should become real mutation receipts without adding a
second model turn or level-specific logic.

Measurement:

A five-run Level 3 batch improved the original no-mutation failure class but
was not clean enough to promote: `4/5` passed, while one run spent about `88s`,
fell into a broad second provider turn, and left a broken validation artifact.
A one-pass Level 1-5 ladder then showed the broader failure map:

- Level 1: `5/5` passed through deterministic file-write receipts.
- Level 2: produced valid artifacts, passing validation, semantic probe, and
  symbol checks, but failed latency at about `240s`.
- Level 3: passed, but took about `39s`.
- Level 4: failed with no successful mutation after pre-mutation validation
  evidence.
- Level 5: passed, but took about `88s`.

Decision:

Keep executable candidate selection and placeholder/example skipping as part of
the reusable `tool_call_normalization` primitive. Revert the permissive
broken-envelope nested-object recovery because it crosses from normalization
into noisy salvage and can extract intermediate patch attempts that should not
be trusted as a clean tool-call batch.

## EXP-CODING-050: Workflow-CD tool progress watchdog budget

Status: `rejected_after_measurement`

Patch packet:

```text
Primitive: tool_progress_watchdog
Invariant: mutation-required tasks must produce useful receipts or structured no-progress before long provider/controller wall-time budgets are consumed.
Expected measurable delta: Level 2 and Level 5 should stop ending as opaque 200s+ stalls; Level 1 remains deterministic; Level 3/4 should not regress.
Rollback condition: Level 1-4 regress, or Level 2/5 still burn long wall-time without clearer structured failure.
```

Hypothesis:

The current largest failure is not local file tooling or parser capability. The
runtime allows broad provider/recovery paths to consume `90s` to `240s` after
low/mid-profile tasks should already have either produced receipts or returned
a structured no-progress result.

Patch:

- Lowered the broad provider timeout to `30s`.
- Lowered broad recovery timeout to `45s` where it was still longer.
- Lowered broad native wall timeout to `90s`.
- Capped partial recovery to one turn in the lab mutation spine.
- Applied the same broad budget to the lab mutation spine and official coding
  project operator so the policy remains a workflow primitive rather than a
  harness-only tweak.

Expected impact:

This should not make Level 2/5 magically more capable, but it should prevent
them from hiding capability failures behind multi-minute stalls. If the run has
already produced sufficient receipts, later patches can close from evidence;
if it has not, this watchdog budget should return a faster structured failure.

Measurement:

One Level 1-5 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass, unchanged.
- Level 2: failed `no_successful_mutation`, but wall time improved from about
  `240s` to about `100s`; still missed the `90s` fast budget and produced no
  mutation receipt.
- Level 3: passed at about `33s`; slower than the prior `24s` sample.
- Level 4: passed at about `34s`; functionally healthy but slower than the
  prior `3.5s` sample.
- Level 5: failed `seeded_repair_timeout`, but wall time improved from about
  `205s` to about `36s`; the run produced seeded mutation receipts but did not
  repair behavior before timeout.

Decision:

Keep this as evidence that workflow-CD budgets can cap the worst Level 5 stall,
but do not treat it as the complete `tool_progress_watchdog` implementation.
The next patch must enforce runtime first-useful-receipt behavior, not only
lower metadata timeouts.

## EXP-CODING-051: First-mutation lane obeys CD provider budget

Status: `rejected_after_measurement`

Patch packet:

```text
Primitive: tool_progress_watchdog
Invariant: first mutation artifact lanes must obey the workflow-CD provider budget instead of overriding it with a hardcoded longer timeout.
Expected measurable delta: Level 2/3/4 first-mutation stalls should be capped by the configured provider budget; Level 1 remains unchanged; Level 5 should not regress.
Rollback condition: Level 1-4 regress materially, or broad Level 1-7 still shows long no-mutation stalls with no clearer failure.
```

Hypothesis:

The previous CD budget patch reduced some broad-loop stalls, but the
first-mutation artifact lane still inserted a hardcoded `45s` provider timeout.
That means workflow-level budget changes could not fully control
first-useful-receipt timing for low/mid-profile edit lanes.

Patch:

- The first-mutation lane now computes its provider timeout from the workflow
  metadata, honoring the smallest configured provider budget while preserving a
  `45s` maximum fallback.
- The patch remains generic: no eval level names, fixture paths, or task
  symbols are referenced.

Expected impact:

This should make timeout policy genuinely CD-driven for the first mutation
artifact lane. If the broad 1-7 ladder still shows slow no-progress behavior,
the missing primitive is an interruptible provider watchdog rather than another
timeout value.

Measurement:

One Level 1-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 2: failed `no_successful_mutation` at about `100s`, unchanged from the
  CD-budget-only result.
- Level 3: regressed to `no_successful_mutation` at about `10s`.
- Level 4: regressed to `no_successful_mutation` at about `30s`.
- Level 5: failed `seeded_repair_timeout` at about `42s`.
- Level 6: failed `seeded_repair_timeout` at about `34s`.
- Level 7: failed `seeded_repair_timeout` at about `36s`.

Decision:

Rejected and reverted the runtime timeout-code change because it triggered the
declared rollback condition: lower/mid levels 3 and 4 regressed materially.
Keep the evidence: the current first-mutation lane needs better output forcing
or a compact retry/reflection primitive, not simply a shorter provider timeout.

## EXP-CODING-052: Compact first-mutation retry/reflection

Status: `reverted`

Patch packet:

```text
Primitive: tool_progress_watchdog
Invariant: when a mutation-required first edit turn returns no mutation receipts, the runtime may make exactly one compact retry using the already-loaded context and previous output, then must fail fast if still no mutation.
Expected measurable delta: Level 3/4 no-mutation failures should recover to mutation receipts or fail with a bounded retry timeout; Level 1 remains unchanged; Level 2/5+ should avoid multi-minute opaque stalls.
Rollback condition: Level 1 regresses, Level 3/4 reliability worsens, or the retry reintroduces 200s-style stalls.
```

Hypothesis:

The first-mutation lane sometimes has enough loaded context but returns
reasoning, examples, or no executable mutation. A shorter timeout worsened this
by denying enough time for useful output. A bounded compact retry should keep
the provider budget explicit while giving the model one chance to convert the
same context into a valid mutation-only batch.

Patch:

- Added a retry-only first-mutation system prompt.
- Added a compact retry prompt with task, loaded context, and prior output
  preview.
- Added one retry call after a zero-mutation first batch.
- Retry is capped at `15s` and uses only mutation tools.
- Timeout returns structured partial progress instead of falling into broad
  unbounded continuation.

Expected impact:

This should recover some Level 3/4 no-mutation failures without changing the
direct Level 1 path. If the weak model still cannot emit usable mutation JSON,
the failure should become bounded and easier to diagnose.

Measurement:

One Level 1-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 2: failed `no_successful_mutation` at about `100s`, unchanged from the
  CD-budget-only result.
- Level 3: passed at about `36s`.
- Level 4: passed at about `22s`.
- Level 5: failed `seeded_repair_timeout` at about `40s`.
- Level 6: failed `seeded_repair_timeout` at about `38s`.
- Level 7: failed `seeded_repair_timeout` at about `39s`.

Decision:

Rejected and reverted the compact retry runtime change because it did not
address the largest current failures. Level 2 still produced no successful
mutation, and Level 5+ still failed in the seeded repair path. The evidence
points away from another first-mutation retry and toward a more primitive
repair-path controller that can convert failed validation/import-surface
evidence into bounded repair mutations.

## EXP-CODING-053: Import-surface evidence expansion for seeded repair

Status: `reverted`

Patch packet:

```text
Primitive: seeded_repair_controller
Profile touched: 3
Activation evidence: failed Python validation/import-surface details that include cannot-import errors or explicit `from module import symbol` lines.
Invariant: a failed import surface should be converted into bounded owner/export mutations for the whole observed public import line, not just the first symbol named by Python's ImportError.
Expected measurable delta: Level 5+ seeded-repair failures caused by repeated missing import symbols should advance to semantic validation/repair instead of timing out after one-at-a-time scaffolding.
Rollback condition: Level 1-2 regress, or Level 5+ still times out with no clearer evidence.
```

Hypothesis:

Python reports only the first missing symbol in a `from package import A, B, C`
statement. The previous seed primitive scaffolded that first symbol, which left
later symbols to fail in follow-up validation and pushed the workflow into
provider-bound seeded repair timeouts. The validation traceback often includes
the original import line, so the primitive can derive the whole requested public
surface from evidence already present.

Patch:

- Added generic parsing for `from module import symbol_a, symbol_b` lines in
  failed validation details.
- Merged those symbols with existing `cannot import name` extraction.
- Kept activation fixture-agnostic: no eval levels, paths, or expected symbol
  names are encoded.
- Kept output bounded to the existing Python import-surface seed mutation path.

Expected impact:

This should make Profile 3 seeded repair more atomic: one evidence packet can
produce one owner/export mutation batch for the full import surface, rather than
forcing repeated model/provider repair turns for each newly exposed missing
symbol.

Measurement:

One Level 1-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 2: failed `no_successful_mutation` at about `100s`.
- Level 3: failed `no_successful_mutation` at about `45s`.
- Level 4: failed `no_successful_mutation` at about `45s`.
- Level 5: failed `seeded_repair_timeout` at about `41s`.
- Level 6: failed `seeded_repair_timeout` at about `39s`.
- Level 7: failed `seeded_repair_timeout` at about `42s`.

Decision:

Rejected and reverted. The patch did not produce an obvious positive delta:
Level 5+ still timed out in seeded repair, and Level 3/4 failed at the
first-mutation boundary. The useful evidence is that the current dominant issue
is not missing import-line breadth alone; the runtime still needs a cleaner
primitive boundary between first mutation, failed-validation evidence, and
bounded repair execution.

## EXP-CODING-054: First-mutation timeout demotion

Status: `promoted_for_next_canary_batch`

Patch packet:

```text
Primitive: tool_progress_watchdog
Profile touched: 2
Activation evidence: bounded first-mutation artifact lane provider timeout before any successful mutation receipt.
Invariant: an optional fast-path mutation lane timeout is local no-progress evidence, not terminal task failure.
Expected measurable delta: Level 2/3/4 no-mutation failures should either produce a mutation through the parent native tool loop or fail with clearer parent-loop timing/evidence instead of terminating at the first-mutation lane.
Rollback condition: Level 1 regresses, Level 2/3/4 wall time increases without mutation, or higher levels lose seeded repair evidence.
```

Hypothesis:

The first-mutation artifact lane is a fast-path primitive. Its timeout should
not terminate the whole task, because the parent native tool loop still has the
loaded context and can attempt the mutation under the normal controller. Treating
the fast-path timeout as terminal collapses Profile 2/3 work before repair or
validation evidence can exist.

Patch:

- Changed first-mutation lane timeout handling from terminal partial-timeout
  return to local demotion evidence.
- The parent prompt receives a tiny no-progress instruction: use already loaded
  context, avoid repeated discovery, and mutate before final output.
- No eval levels, fixture paths, or expected symbols are encoded.

Expected impact:

This should restore the primitive boundary:

```text
fast mutation lane timeout
-> parent native tool loop
-> mutation or structured parent-loop failure
```

The patch is intentionally smaller than a repair controller. It only prevents
an optional fast path from becoming a task-level terminal failure.

Measurement:

One Level 1-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 2: failed `no_successful_mutation` at about `100s`, unchanged.
- Level 3: failed `no_successful_mutation` at about `5.7s`, improved from the
  previous `45s` no-mutation failure but still not a pass.
- Level 4: passed at about `3.4s`.
- Level 5: passed at about `27.9s`.
- Level 6: passed at about `22.8s`.
- Level 7: passed at about `20.6s`.

Decision:

Keep as a partial primitive improvement. It did not fix Level 2 or Level 3, but
it preserved Level 1, converted Level 4 back to a fast pass, and restored
Level 5-7 seeded-repair execution. The remaining lower-profile problem is now
clearer: some existing-project patch cases return from the first-mutation lane
with no mutation and no parent-loop continuation, but they fail quickly enough
to patch in a targeted next step.

## EXP-CODING-055: First-mutation no-op demotion

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: tool_progress_watchdog
Profile touched: 2
Activation evidence: first-mutation artifact lane returns successfully but emits zero successful mutation receipts.
Invariant: a successful-but-empty fast lane is local no-progress evidence, not terminal task failure.
Expected measurable delta: Level 3 no-mutation failure should continue into the parent native tool loop and either mutate or fail with parent-loop evidence; Level 4-7 should remain passing.
Rollback condition: Level 1 regresses, Level 4-7 lose their restored pass path, or Level 3 wall time increases materially without mutation.
```

Hypothesis:

EXP-CODING-054 fixed first-mutation timeout demotion, but the Level 3 trace
showed a second boundary bug: the first-mutation lane could return normally with
zero receipts and the runtime immediately closed as `partial_blocked`. That
prevented the parent native tool loop from attempting the mutation even though
bounded context had already been loaded.

Patch:

- Changed successful-but-empty first-mutation lane handling from terminal
  `partial_blocked` return to local no-op demotion.
- Added a tiny parent-loop rule: use loaded context, do not repeat discovery,
  and emit file mutation before final output.
- Kept the patch profile-scoped and fixture-agnostic.

Expected impact:

This should complete the pair with EXP-CODING-054:

```text
fast mutation lane timeout OR no-op
-> parent native tool loop
-> mutation or structured parent-loop failure
```

Measurement:

Focused Level 1 and Level 3-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 3: passed at about `24s`.
- Level 4: failed `no_successful_mutation` at about `62s`.
- Level 5: failed `seeded_repair_timeout` at about `58s`.
- Level 6: failed `seeded_repair_timeout` at about `54s`.
- Level 7: failed `seeded_repair_timeout` at about `37s`.

Decision:

Rejected and reverted. The patch proved the concept for Level 3, but it broke
the restored Level 4-7 path and added unacceptable latency. The useful evidence
is that no-op demotion cannot be broad. It needs tighter activation, likely
only for bounded existing-project patches with no pre-mutation validation or
seeded import-surface repair in play.

## EXP-CODING-056: Scoped first-mutation no-op demotion

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: tool_progress_watchdog
Profile touched: 2
Activation evidence: first-mutation artifact lane returns zero mutation receipts for an existing-project patch without pre-mutation validation, seeded import-surface repair, failed validation evidence, or checkpoint/project-operator control active.
Invariant: no-op demotion is allowed only for bounded Profile 2 edit tasks; validation/repair/project-slice paths keep their owning controllers.
Expected measurable delta: Level 3 recovers without the Level 4-7 regressions seen in EXP-CODING-055.
Rollback condition: Level 1 regresses, Level 4-7 lose the restored pass path, or Level 3 wall time increases materially without mutation.
```

Hypothesis:

EXP-CODING-055 proved that no-op demotion can recover a simple existing-project
edit, but broad activation stole control from validation and seeded-repair
controllers. The primitive should only activate when the task is still a clean
Profile 2 bounded edit with loaded context and no validation/repair evidence.

Patch:

- Demote zero-receipt first-mutation lane output only for `existing_project_patch`.
- Block demotion when pre-mutation validation ran.
- Block demotion when Python import-surface seed receipts exist.
- Block demotion when failed validation evidence exists.
- Block demotion when checkpoint/project-operator control is active.

Expected impact:

This should preserve the useful Level 3 recovery from EXP-CODING-055 while
keeping Level 4-7 on the stable EXP-CODING-054 path.

Measurement:

Focused Level 1 and Level 3-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 3: passed at about `39s`.
- Level 4: passed at about `34s`.
- Level 5: failed `seeded_repair_timeout` at about `48s`.
- Level 6: failed `seeded_repair_timeout` at about `45s`.
- Level 7: failed `seeded_repair_timeout` at about `46s`.

Decision:

Rejected and reverted. The scoped activation recovered Level 3 and preserved
Level 4, but it still disrupted the Level 5-7 seeded-repair path. The failure
suggests the parent-loop continuation itself is too broad/heavy for this
runtime location. A future version should not reuse the full parent loop after a
fast-lane no-op; it needs a smaller deterministic/local mutation fallback or a
separate Profile 2-only lane that cannot affect seeded-repair execution.

## EXP-CODING-057: Profile 2 direct-edit fallback after first-mutation no-op

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: direct_edit_fallback
Profile touched: 2
Activation evidence: first-mutation artifact lane returns zero mutation receipts for an existing-project patch with already-loaded context and no validation/seeded-repair/project-operator evidence active.
Reference pattern: Claude Code/Codex successful traces use exact file context plus direct Edit/Write, then validation, instead of broad parent-loop fallback.
Invariant: a Profile 2 no-op recovery may run one tiny mutation-only model call over already-loaded context; it must not call read/list/command tools or steal control from validation/repair profiles.
Expected measurable delta: Level 3 recovers without Level 4-7 seeded-repair regressions.
Rollback condition: Level 1 regresses, Level 4-7 lose the restored EXP-CODING-054 pass path, or Level 3 recovers only with worse latency than the broad fallback.
```

Hypothesis:

The reference systems that pass Level 3 and higher levels do not recover no-op
edits by falling into a broad parent loop. They use a bounded direct edit step:
read/observe exact files, edit exact files, then validate. Infring already has
the read context before the first-mutation no-op, so the missing primitive is a
single direct-edit fallback constrained to `file_patch`/`file_write`.

Patch:

- Added one Profile 2-only direct-edit fallback after a zero-receipt
  first-mutation artifact lane.
- Fallback is inactive when pre-mutation validation, failed validation evidence,
  Python import-surface seed receipts, or checkpoint/project-operator control is
  active.
- Fallback uses only `file_patch`/`file_write` tools and already-loaded context.
- On successful mutation it runs existing auto-validation and closes through the
  coding execution spine when evidence is satisfied.

Expected impact:

This should preserve the successful reference pattern without reintroducing the
broad parent-loop leakage that broke Level 5-7 in EXP-CODING-055 and
EXP-CODING-056.

Measurement:

Focused Level 1 and Level 3-7 ladder with `kimi-k2.6:cloud`:

- Level 1: `5/5` pass.
- Level 3: failed `no_successful_mutation` at about `52s`.
- Level 4: passed at about `30s`.
- Level 5: failed `seeded_repair_timeout` at about `39s`.
- Level 6: failed `seeded_repair_timeout` at about `38s`.
- Level 7: failed `seeded_repair_timeout` at about `64s`.

Decision:

Rejected and reverted. The patch followed the reference pattern structurally,
but the runtime placement was still wrong. It introduced an extra provider call
that timed out on Level 3 and still failed to preserve Level 5-7. The reference
lesson remains useful: winning systems perform direct edits from exact file
context, but Infring should not add that as an after-the-fact fallback inside
the first-mutation no-op branch. The direct-edit shape needs to be the primary
Profile 2 lane or a deterministic local lane, not a second provider call after
another model no-op.

## EXP-CODING-058: Profile 2 direct-edit primary lane

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: profile2_direct_edit_primary_lane
Profile touched: 2
Activation evidence: bounded existing-project edit with loaded local context and selected lane `existing_project_patch` or `bounded_existing_project_edit`.
Reference pattern: Claude Code and Codex traces perform exact context read -> direct Edit/Write -> validation, instead of letting an artifact-synthesis lane terminate before mutation.
Invariant: Profile 2 direct edit owns the first mutation slot for small existing-project edits; artifact synthesis must not block that lane.
Rollback condition: Level 1 regresses, Level 3 still fails without mutation, or Level 4-7 lose the EXP-CODING-054 pass path.
```

Hypothesis:

EXP-CODING-057 failed because direct edit was bolted on as a fallback after a
first-mutation no-op. The winning framework traces show direct edit as the
primary small-edit shape, not as a second provider call after another lane has
already failed.

Patch:

- Added CD-gated `profile2_direct_edit_primary_lane`.
- Skips `first_mutation_artifact_lane_v1` only for bounded Profile 2 existing
  project edit lanes.
- Gives the first provider turn a tiny direct-edit prompt with only
  `file_patch`/`file_write` tools.
- Leaves validation repair, multi-file/project slices, and checkpointed project
  operation on their existing controllers.

Expected impact:

```text
bounded context loaded
-> Profile 2 direct edit owns first mutation
-> file_patch/file_write receipt
-> existing validation/spine closure
```

Measurement:

First focused Level 5-7 run:

- Level 5: runtime completed successfully, validation passed, semantic probe
  passed, but harness failed `missing_expected_markers` because the compact
  repair turn rewrote the test file instead of treating tests as immutable
  evidence.
- Level 6: failed `seeded_repair_timeout`; compact turn reduced prompt/system
  and tools, but still timed out at the inherited `30s` provider budget.
- Level 7: failed `seeded_repair_timeout`; compact turn reduced prompt/system
  and tools, but still timed out at the inherited `30s` provider budget.

Follow-up patch:

- Block test-file mutations during seeded import-surface repair; this repair
  primitive owns source/export mutation, not evidence/test rewriting.
- Add CD-controlled `seeded_import_surface_repair_provider_timeout_seconds`
  with a `45s` budget so this compact lane is not bound to the broad tool-loop
  timeout.

### EXP-CODING-058 measurement update

One rebuilt Level 3 run with `kimi-k2.6:cloud`:

- Result: pass.
- Wall time: about `122.7s`.
- Time to first mutation: about `62.6s`.
- Native tool sequence: `file_list`, `file_read_many`, `file_write`, `file_write`, `command_run`, `bounded_direct_edit_lane`.
- Provider timing probe: one recorded direct-edit provider turn, `21.8s` provider latency, prompt about `3249` chars, system about `240` chars, tool count `2`.

Decision:

Rejected and reverted. The patch proved that the primary direct-edit topology can mutate and pass, but it made Level 3 much slower than the previous stable and reference-framework baselines. The useful lesson is narrower: direct edit must also close/finalize without extra heavy controller/finalization latency, or it is not a viable replacement for the artifact lane.

## EXP-CODING-059: Compact seeded import-surface repair turn

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: seeded_import_surface_compact_repair_turn
Profile touched: 3
Activation evidence: deterministic Python import-surface seed receipts exist and validation after the latest mutation is unresolved.
Reference pattern: validation/import failure -> source/export repair mutation -> validation, without broad discovery/planning tools in the repair turn.
Invariant: import-surface seeding is not completion; the next model-owned turn must be a compact file mutation repair, not the broad native tool loop.
Rollback condition: Level 3/4 regress, Level 5-7 remain seeded_repair_timeout with no prompt/tool-size improvement, or the repair turn blocks legitimate non-Python tasks.
```

Hypothesis:

The current 5-7 failures are not first-mutation failures. The runtime writes
seeded source/export files almost immediately, then sends the model into a broad
repair turn with `8k-11k` prompt chars, about `3.6k` system chars, and `8`
tools. Kimi times out before replacing the placeholder seed with real behavior.

Patch:

- Added CD-gated `seeded_import_surface_compact_repair_turn`.
- Activates only after Python import-surface seed receipts and unresolved
  validation after the latest mutation.
- Uses a compact mutation-entry packet and file_write/file_patch tools only.
- Blocks read/list/command calls during that repair turn.

Expected impact:

```text
failed validation/import evidence
-> deterministic source/export seed
-> compact source/export repair mutation
-> runtime validation
```

Measurement:

Initial measurement:

- Build passed with isolated xtask target.
- Level 3/4 smoke passed after parser adapter:
  - Level 3: 1/1 pass, ~11.2s wall, ~11.1s to first mutation.
  - Level 4: 1/1 pass, ~15.5s wall, ~15.4s to first mutation.
- Level 2 canary did not improve sufficiently:
  - 0/3 strict pass, average ~114.7s.
  - Failure classes: `model_lock_violation`, `import_surface_missing`, `no_successful_mutation`.
  - Interpretation: protocol support is safe, but the compact mutation lane still is not explicitly selecting the reference `actions` controller shape.

Decision:

Keep the protocol adapter as a safe primitive. Add a separate, measured controller-surface experiment rather than broadening parser behavior.

### EXP-CODING-082: Compact mutation action-surface prompt

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: compact_mutation_action_surface
Profile touched: mutation-only recovery system contract
Activation evidence: EXP-CODING-081 added safe parsing for mini-SWE-style shell actions, but Level 2 still stayed slow and unreliable, indicating the compact lane was not being asked to use the newly supported action surface.
Reference pattern: mini-SWE-agent exposes a tiny `actions` schema for shell edit batches, with runtime-owned validation and stop rules.
Invariant: expose `actions` only as a generic shell-edit batch surface for mutation-only recovery. Existing `tool_calls`, file_write/file_patch, command_run receipts, shell-edit guards, and validation remain unchanged.
Rollback condition: Level 3/4 regress, action commands bypass mutation receipts, or Level 2 latency worsens without improving mutation/semantic completion.
```

Patch:

- Updated the mutation-only recovery system contract to allow `{"actions":[{"command":"..."}]}` for controlled shell edit batches.
- Kept `tool_calls` as the fallback schema and retained all existing no-read/no-validation/final-answer constraints for that turn.

Measurement:

Initial measurement:

- Build passed with isolated xtask target.
- Level 3/4 smoke:
  - Level 3: 1/1 pass, ~6.3s wall, ~6.2s to first mutation.
  - Level 4: 1/1 pass, ~20.9s wall, ~20.8s to first mutation.
- Level 2 canary:
  - 1/3 strict pass, average ~128.4s.
  - Attempt 1 passed fully in ~19.5s.
  - Attempts 2 and 3 mutated source/tests and passed expected-symbol, validation, and semantic-probe checks, but failed final runtime status/latency.

Interpretation:

The action surface improved actual coding capability: the remaining Level 2 failures shifted from missing mutation/import-surface/semantic behavior to evidence-satisfied closure and latency/status handling. Do not add more task-specific coding hints here; next fixes should target generic closeout/evidence and controller budgets.

### EXP-CODING-083: Public-interface generic token suppression

Status: `reverted`

Patch packet:

```text
Primitive: public_interface_generic_token_suppression
Profile touched: public API evidence extraction and completion gates
Activation evidence: after successful mutation, validation, and semantic probe, runtime blockers still included generic prompt words such as `and`, `prompt`, `run`, `symbol`, and `these` as missing public-interface evidence.
Reference pattern: successful frameworks close on concrete code/test/probe evidence; natural-language scaffolding words from the task contract are not treated as API symbols.
Invariant: suppress only generic instruction/control words from prompt-derived public-interface candidates. Explicit imports, call patterns, class/function names, and underscored API names remain evidence targets.
Rollback condition: genuine public API names stop being required, or semantic-probe failures close as success.
```

Patch:

- Expanded prompt-derived public-interface stopwords for generic instruction words, source/test/file wording, and local task-contract scaffolding.
- Applied suppression to both broad request-surface extraction and action-phrase extraction.

Measurement:

- Build passed with isolated xtask target.
- Level 2 canary regressed:
  - 0/3 strict pass, average ~150.8s.
  - Failure classes: `import_surface_missing`, `latency_budget_exceeded`, `assertion_mismatch`.
  - The previous action-surface run had 1/3 strict pass and all attempts had better evidence shape.

Decision:

Revert the stopword expansion. The underlying finding remains valid, but broadening the generic-token filter is not the right patch shape. Future evidence-closeout fixes should use receipt-backed semantic/validation state directly instead of trying to tune prompt-token heuristics.

### EXP-CODING-084: Semantic probe authorizes public-interface closeout

Status: `reverted`

Patch packet:

```text
Primitive: semantic_probe_public_interface_authority
Profile touched: completion evidence and public-interface verification gates
Activation evidence: Level 2 action-surface attempt 2 mutated source/tests and passed validation, expected-symbol, and semantic-probe checks, but the runtime still returned `partial_blocked` because heuristic public-interface verification treated generic prompt words as missing API symbols.
Reference pattern: mini-SWE-agent stops after tests plus semantic probe pass. The project-local semantic probe is the executable public-interface contract; heuristic prompt-token verification should not override it after mutation.
Invariant: only a successful semantic probe command observed after a successful mutation suppresses heuristic public-interface gaps. Missing mutation, missing probe, or failed probe still blocks normally.
Rollback condition: semantic-probe failures close as success, missing imported public APIs are ignored without a successful post-mutation probe, or Level 3/4 regress.
```

Patch:

- Added an order-aware `semantic_probe_after_mutation` evidence helper.
- Suppressed heuristic public-interface verification gaps when a successful post-mutation semantic probe exists.
- Suppressed prompt-derived public-interface evidence gaps in product-slice checks under the same receipt-backed condition.

Measurement:

- Build passed with isolated xtask target.
- Level 3/4 smoke stayed healthy and fast:
  - Level 3: 1/1 pass, ~6.2s.
  - Level 4: 1/1 pass, ~6.7s.
- Level 2 canary did not improve:
  - 0/3 strict pass, average ~184.2s.
  - Failure classes: `syntax_error`, `import_surface_missing`, `unknown_validation_failure`.
  - None of the failures reached the successful post-mutation semantic-probe condition, so the patch did not address the active failure.

Decision:

Revert. The principle may still be valid, but this was not the next effective primitive. The clearer trace-backed failure is prompt-surface leakage: the model copied literal `<full file>` placeholder content from our command example, while mini-SWE avoids placeholder examples and uses a tiny action schema.

### EXP-CODING-085: Remove placeholder-bearing edit examples

Status: `reverted`

Patch packet:

```text
Primitive: placeholder_free_edit_surface
Profile touched: compact mutation-entry packet
Activation evidence: Level 2 semantic-closeout canary wrote literal placeholder text into a test file, producing `SyntaxError: <full file>`. The placeholder came from Infring's own shell-edit example, not from the user task.
Reference pattern: mini-SWE-agent uses a tiny action schema and says to use heredocs for multi-line writes, but does not provide fake file contents for the model to copy.
Invariant: mutation-entry prompts may describe the JSON/action schema and the requirement to write complete real file contents, but must not include placeholder source/test bodies. This is prompt hygiene, not an eval-specific rejection rule.
Rollback condition: Level 3/4 regress, Level 2 mutation rate drops because the model no longer understands shell-edit batches, or placeholder leakage persists.
```

Patch:

- Removed the concrete shell-edit example containing fake file body placeholders.
- Updated the mutation-entry packet to allow the compact `actions` schema for shell edit batches and `tool_calls` for file_write/file_patch.
- Added a generic instruction to write complete real file contents only.

Measurement:

- Build passed with isolated xtask target.
- Level 3/4 smoke:
  - Level 3: 1/1 pass, but slowed to ~39.1s and closed through terminal receipt synthesis.
  - Level 4: 1/1 pass, ~5.3s.
- Level 2 canary regressed:
  - 0/3 strict pass, average ~155.8s.
  - Failure classes: `no_successful_mutation` x2, `latency_budget_exceeded` x1.

Decision:

Revert. Removing placeholder examples is directionally right for prompt hygiene, but doing it as prompt surgery weakened mutation reliability. The next fix should be controller-level: preserve a small explicit edit affordance while preventing placeholder content from becoming trusted mutation, likely through a bounded mini-SWE-style action controller or tool-level placeholder-content rejection with targeted repair feedback.

### EXP-CODING-086: Compact action controller primitive

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: compact_action_controller
Profile touched: compact bootstrap mutation and mutation-only recovery
Activation evidence: mini-SWE-agent solves Level 2 with the same weak model by using a tiny `actions` shell command controller, while prompt-only changes in Infring oscillated between no mutation, placeholder leakage, and long partial-blocked runs.
Reference pattern: bounded action controller with local context already loaded, one shell edit batch, runtime-owned validation/probe/closeout.
Invariant: this is a controller surface, not a fixture branch. It activates from metadata, only after local context is loaded and a mutation is required, routes through normal `command_run` dispatch and shell-edit guards, and leaves the existing mutation packet path as fallback when disabled.
Rollback condition: Level 3/4 regress, Level 2 mutation rate drops, shell actions bypass receipt synthesis, or latency worsens without semantic improvement.
```

Patch:

- Added `compact_action_controller_enabled` to the official coding workflow CD.
- Added a compact action-controller system prompt and prompt builder that accepts only `actions` shell commands.
- Restricted that controller's visible tool surface to command execution aliases.
- Reused the existing compact mutation packet but strips placeholder-bearing shell-edit examples before handing it to the action controller.

Measurement:

Pending.

### EXP-CODING-088: Make compact action controller dormant after activation regression

Status: `patched_pending_measurement`

Finding: Activating the compact action controller in the Level 3/4 mutation spine changed the failure mode from timeout to pass after EXP-CODING-087, but it was still a net regression. Level 3 passed in about 27.8s instead of the prior small-smoke result near 4.3s, and the trace showed the model did not actually follow the `actions` surface: it emitted file-write tool calls through a one-tool controller turn.

Decision: Keep the protocol/parser/controller code as a dormant primitive for future controlled experiments, but disable `compact_action_controller_enabled` in runtime workflow CDs. This avoids letting a partially-assimilated mini-SWE pattern degrade the stable path.

Next model-guided target: consult the reference traces for why mini-SWE/aider/Codex get compact edits without needing a special action-controller surface, likely focusing on concise prompt packets plus runtime-owned closeout rather than a new controller lane.

### EXP-CODING-087: Action controller disables tool-call-marker streaming

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: action_controller_stream_protocol_alignment
Profile touched: compact action controller provider request
Activation evidence: after EXP-CODING-086 activated the compact action controller in the active mutation spine, Level 3 produced real code and passed validation/probe externally, but the agent run timed out. Trace showed `compact_action_controller_turn: true` with `stream_until_tool_calls: true`; the controller asks for `actions`, not `tool_calls`.
Reference pattern: mini-SWE-style action controllers consume a complete compact JSON action object, then runtime dispatches and closes. They do not use a tool-call-marker streaming shortcut.
Invariant: this only changes the compact action controller protocol mode. Existing `tool_calls` lanes keep their streaming gate.
Rollback condition: provider latency increases materially or action parsing fails to dispatch real command receipts.
```

Patch:

- Force `stream_until_tool_calls = false` for compact action controller turns.

Measurement:

Pending.

### EXP-CODING-086: Compact action controller primitive activation

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: compact_action_controller
Profile touched: compact coding controller and active mutation spine workflow CD
Activation evidence: mini-SWE-agent's weak-model Level 2 traces use a tiny JSON `actions` shell-command surface. Infring had protocol support for this shape and a compact controller branch, but the Level 3/4 harness launches `local_coding_phase1_mutation_spine`, which did not enable the new controller flag.
Reference pattern: compact action controller receives already-loaded context, emits only `{"actions":[{"command":"..."}]}`, uses one controlled shell edit batch, and leaves validation/repair/finalization to runtime.
Invariant: no level, fixture, path, or expected-symbol hardcoding. Activation is workflow-CD controlled and still requires the generic controlled shell-edit primitive.
Rollback condition: lower levels regress, compact controller loops without mutation, or traces show eval-specific commands instead of real local coding edits.
```

Patch:

- Enabled `compact_action_controller_enabled` in the active `local_coding_phase1_mutation_spine` native success criteria.
- Kept the already-added official `coding_project_operator` flag so Level 2 and top-level coding lanes share the same primitive switch.

Measurement:

Pending.

### EXP-CODING-059 follow-up measurement

Second focused Level 5-7 run after adding a test-mutation block and extending
the compact repair timeout to `45s`:

- Level 5 regressed to `partial_blocked` at about `189s`; marker preservation
  improved, but the implementation stayed as the seeded `NotImplementedError`.
- Level 6 still failed `seeded_repair_timeout`, now at about `67s`.
- Level 7 still failed `seeded_repair_timeout`, now at about `56.7s`.

Decision:

Rejected and reverted the follow-up timeout/test-block patch. It was not an
obvious positive delta. Keep only the first compact seeded-repair turn for now,
because that reduced the prompt/system/tool surface and got Level 5 to runtime
success with validation and semantic probes passing, even though the harness
still rejected the test-file rewrite.

### EXP-CODING-060: Seeded import-surface implementation-contract packet

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: seeded_import_surface_contract_packet
Profile touched: 3
Activation evidence: compact seeded import-surface repair turn is active.
Reference pattern: failed import/validation evidence -> owner source/export repair, with tests/probes treated as contract evidence rather than edit targets.
Invariant: the model receives enough behavior evidence to implement the seeded source without reopening broad discovery/planning or mutating tests.
Rollback condition: lower levels regress, Level 5 remains seeded_repair_timeout with no clearer failure, or test/probe mutation increases.
```

Patch:

- Added a compact owner/contract/validation/skeleton packet for seeded
  import-surface repair.
- Reused the existing public-contract line extractor rather than adding
  level-specific rules.
- Kept the repair tool menu narrowed to `file_write`/`file_patch`.
- Added runtime timing flags for seeded repair turns so traces can distinguish
  this lane from generic mutation recovery.

### EXP-CODING-061: First-mutation artifact timeout compact recovery

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: first_mutation_artifact_no_mutation_recovery
Profile touched: 2-3
Activation evidence: first mutation artifact lane timed out or returned no successful mutation after context/validation evidence already exists.
Reference pattern: narrow failed fast-edit attempt -> compact mutation-only recovery, rather than finalizing partial before a second mutation attempt.
Invariant: no eval-level special cases; recovery is keyed only to receipts and the existing mutation-only recovery gate.
Rollback condition: Level 3/5 regress, Level 4 still fails with no mutation, or retry loops add latency without successful mutation.
```

Patch:

- Replaced the immediate `partial_blocked` return after first-mutation artifact
  no-mutation with an armed compact mutation-only recovery path.
- Requires existing context or validation receipts, plus the existing compact
  mutation/recovery gates.
- Leaves the original partial-blocked behavior intact when the compact recovery
  preconditions are absent.

Follow-up patch:

- Moved the first-mutation artifact lane provider timeout out of Rust hardcoding
  and into workflow CD policy.
- Set the bounded first-mutation lane timeout to `15s` so no-mutation demotion
  can reach compact recovery before the run spends most of its wall budget.

### EXP-CODING-062: Validation-guided compact repair turn

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: validation_guided_compact_repair_turn
Profile touched: 2-3
Activation evidence: failed validation receipt exists before required mutation, and the first-mutation lane produced no successful mutation.
Reference pattern: failed validation output + observed source/test context -> narrow product/source patch -> runtime validation.
Invariant: tests/probes are contract evidence by default; mutation targets product/source owner files unless the user explicitly requested test edits.
Rollback condition: Level 3/5 regress, Level 4 remains no-successful-mutation without lower wall time, or validation-guided repair mutates tests instead of source.
```

Patch:

- Added a CD-gated validation-guided compact repair turn.
- Builds a compact packet from failed validation lines, observed product/source
  files, and test/probe contract lines.
- Uses file-only tools and a CD-owned provider timeout.
- Activates from the generic first-mutation no-mutation recovery path when
  failed validation evidence exists.

Follow-up patch:

- Matched the validation-guided compact repair turn to the faster first-mutation
  artifact lane contract: first visible byte must be `{`, JSON tool calls only,
  and thinking flags are omitted for this compact repair turn.

### EXP-CODING-063: Validation-aware first mutation artifact lane

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: first_mutation_artifact_validation_context
Profile touched: 2-3
Activation evidence: failed validation receipts are present before the first mutation artifact lane.
Reference pattern: failed validation output should be first-class context for the first edit attempt, not only for later recovery.
Invariant: validation output is compact evidence; tests/probes remain contract evidence, not default mutation targets.
Rollback condition: Level 3/5 regress, Level 4 remains no-successful-mutation with no wall-time or activation improvement, or first-lane prompt size becomes large enough to hurt fast paths.
```

Patch:

- Added compact failed-validation evidence to the first mutation artifact lane
  context packet.
- Updated the lane mutation rule so failed validation is treated as the primary
  repair contract when present.
- Kept allowed tools limited to `file_patch` and `file_write`.

Decision:

Kept active pending a cleaner replacement. A single Level 4 pass was not
promotion-grade evidence, and the broader Level 3-5 batch was not enough to
prove the lane as a speed optimization. However, rebuilding after removing this
packet caused Level 3 and Level 4 to lose mutation entirely in the current
bounded canary. The reusable lesson is that compact failed-validation evidence
is part of the current mutation entry contract, not a disposable special case.
Future work should gate or reshape it with reference traces, not remove it
blindly.

### EXP-CODING-064: Method lock and external-framework loop

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: experiment_method_guard
Profile touched: all coding profiles
Activation evidence: repeated patch entropy while chasing higher levels.
Reference pattern: successful coding systems are used as runtime behavior probes before primitive assimilation.
Invariant: external frameworks remain in the loop, lower-level canaries remain monotonic, and single lucky passes cannot promote runtime behavior.
Rollback condition: none for the documentation; runtime changes still require measured promotion.
```

Patch:

- Added `docs/workspace/coding_workflow_experiment_method.md`.
- Made `validation_guided_compact_repair_turn` dormant in the official and lab
  coding workflow CDs.
- Attempted to remove unproven failed-validation evidence injection from the
  active first-mutation artifact lane, then restored it after rebuilt-binary
  canaries proved it was part of the current mutation-entry contract.
- Added bounded, incremental reporting to the Level 3/4/5/6/7 reference
  harness so external frameworks can stay in the loop without blocking each
  patch round indefinitely.

Measurement:

Bounded Infring/Codex comparison:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/framework_loop_infring_codex_level3_level4_bounded_20260531.json`
- Infring Level 3: pass, about `30.5s` wall time, about `30.3s` time to first mutation.
- Codex Level 3: pass, about `20.6s` wall time, about `10.9s` time to first mutation.
- Infring Level 4: pass, about `39.1s` wall time, about `39.0s` time to first mutation.
- Codex Level 4: pass, about `33.8s` wall time, about `25.2s` time to first mutation.

Trace interpretation:

Infring correctness recovered, but both Infring runs spent a serial `15s`
timeout in `first_mutation_artifact_lane_v1` before the parent compact mutation
path produced the real edit. Codex did not pay a comparable speculative
provider-lane timeout before mutation. The next primitive patch should remove
that serial failed-turn tax from the default hot path.

Rebuilt-binary follow-up:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/infring_level3_level4_rebuilt_method_lock_20260531.json`
- Removing the first-lane failed-validation/context packet from source caused
  Infring Level 3 and Level 4 to fail with `no_successful_mutation`.
- Decision: restore that packet as active primitive evidence until a cleaner
  reference-backed mutation-entry contract replaces it.

### EXP-CODING-067: Reactivate validation-guided compact repair with real timeout

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: failed_validation_routes_to_compact_repair
Profile touched: 4
Activation evidence: rebuilt Level 4 traces show failed validation exists, but the run falls into generic mutation-only recovery and times out without mutation.
Reference pattern: Codex repairs the owner source file after observing validation failure rather than broad rediscovery or generic mutation recovery.
Invariant: failed validation is compact repair evidence, not a prompt-size expansion excuse or test-case special case.
Rollback condition: Level 4 still fails with no mutation, Level 3 regresses, or the lane mutates tests/probes instead of product/source owner files.
```

Patch:

- Reactivated `validation_guided_compact_repair_turn` in the official and lab
  coding workflow CDs.
- Increased `validation_guided_compact_repair_provider_timeout_seconds` from
  `20` to `45` so the lane is measured as a real repair attempt rather than an
  under-timeout artifact.

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/infring_level3_level4_validation_guided_45s_20260531.json`
- Infring Level 3: pass, about `30.6s` wall time, about `30.5s` time to first mutation.
- Infring Level 4: pass, about `42.2s` wall time, about `42.1s` time to first mutation.
- Level 4 trace confirmed `validation_guided_compact_repair_turn: true`,
  prompt about `2630` chars, system about `290` chars, tool count `2`,
  and a `file_patch` mutation to the product/source owner file.

Decision:

Promoted for the next canary batch, not yet declared broadly stable. The patch
restores rebuilt-binary Level 4 correctness using a cleaner failed-validation
repair lane and does not break Level 3 in the same bounded run. Remaining
latency is still high because the runtime spends about `15s` in the speculative
artifact lane before validation-guided repair; optimize that only after repeated
Level 3/4/5 canaries stay green.

### EXP-CODING-065: Dormant speculative first-mutation artifact lane

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: speculative_lane_promotion_gate
Profile touched: 2-4
Activation evidence: first_mutation_artifact_lane_v1 timed out before mutation on repeated current Level 3/4 traces.
Reference pattern: Codex reaches mutation through its primary tool loop rather than paying a failed speculative provider turn first.
Invariant: speculative provider lanes must be dormant by default unless repeated trace data proves they beat the parent mutation path without lower-level regression.
Rollback condition: disabling the lane regresses correctness or increases time to first mutation across lower/mid canaries.
```

Patch:

- Set `first_mutation_artifact_lane_v1_enabled` to `false` in the official
  coding workflow CD.
- Set `first_mutation_artifact_lane_v1_enabled` to `false` in the local lab
  mutation-spine CD.
- Kept the Rust primitive available for future gated reactivation rather than
  deleting it.

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/infring_level3_level4_no_speculative_first_lane_20260531.json`
- Infring Level 3: failed, `no_successful_mutation`, about `60.1s`, no mutation.
- Infring Level 4: failed, `no_successful_mutation`, about `75.1s`, no mutation.

Decision:

Rejected and reverted. The hypothesis was directionally useful but incomplete:
the first-mutation artifact lane is currently not only a speculative fast path;
its timeout demotion arms the compact mutation-only recovery path. Disabling it
sent the runtime through the broader parent prompt, which timed out twice before
mutation. The next primitive fix should preserve the compact recovery entry
shape while removing or shortening the failed serial artifact attempt.

### EXP-CODING-066: Short speculative artifact timeout before compact recovery

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: bounded_speculative_lane_demote_fast
Profile touched: 2-4
Activation evidence: artifact lane timeout demotion is useful, but the serial 15s wait is a repeated latency tax.
Reference pattern: reference agents reach edit-capable tool use without a long failed speculative pre-turn.
Invariant: keep compact-recovery arming semantics, but speculative provider lanes must fail fast when they are not producing tool calls.
Rollback condition: lower/mid canaries lose mutation or wall time does not improve versus the 15s baseline.
```

Patch:

- Reduced `first_mutation_artifact_lane_v1_provider_timeout_seconds` from `15`
  to the CD/player minimum of `5` in the official coding workflow.
- Reduced the same timeout in the local lab mutation-spine workflow.

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/infring_level3_level4_first_lane_5s_20260531.json`
- Infring Level 3: pass, about `24.6s` wall time, about `24.5s` time to first mutation.
- Infring Level 4: failed, `no_successful_mutation`, about `35.1s`, no mutation.

Decision:

Rejected and reverted. The shorter timeout produced a useful Level 3 speed
improvement, but it lost Level 4 mutation. That violates the monotonic canary
rule. The next patch should not merely tune timeout constants; it should change
the compact recovery prompt/contract for validation-guided tasks so Level 4 can
mutate reliably before optimizing the speculative wait.

### EXP-CODING-068: Level 3/4/5 validation-guided canary

Status: `failure_target_identified`

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/infring_level3_4_5_validation_guided_canary_20260531.json`
- Infring Level 3: pass, about `37.4s` wall time, about `37.1s` time to first mutation.
- Infring Level 4: pass, about `55.7s` wall time, about `55.6s` time to first mutation.
- Infring Level 5: failed, `seeded_repair_timeout`, about `39.9s` wall time, about `0.08s` time to first mutation.

Trace interpretation:

Level 5 is no longer a simple no-mutation failure. The runtime immediately seeds
an import surface by writing `calcpack/arithmetic.py` and `calcpack/__init__.py`,
but the seeded repair turn times out before replacing the placeholder
`NotImplementedError` implementation with real behavior. The failing turn used a
seeded import-surface repair prompt of about `4799` chars, observation context of
about `6222` chars, system prompt of about `241` chars, and tool count `2`, then
timed out at `30s`.

Next patch target:

Consult the reference framework traces for Level 5 and patch the seeded
import-surface repair primitive, not the lower-level validation-guided compact
repair lane. The likely primitive gap is that import-surface seeding creates a
placeholder capability but does not hand the model a compact enough direct
implementation contract to replace the placeholder before timeout.

### EXP-CODING-069: Level 5 reference-framework comparison

Status: `reference_trace_collected_no_patch`

Measurement:

- Report:
  `references/coding-agent-systems/runtime_trace_harness/reports/framework_loop_level5_seeded_repair_20260531.json`
- Infring Level 5: pass, about `17.5s` wall time, about `17.3s` time to first mutation.
- Codex Level 5: pass, about `31.0s` wall time, about `16.5s` time to first mutation.
- Claude Code Level 5: pass, about `24.9s` wall time, about `18.7s` time to first mutation.
- Aider Level 5: not comparable; blocked by missing temp venv at `/tmp/infring-baselines-aider`.
- ForgeCode Level 5: not comparable; blocked by missing comparison binary and cargo-run fallback is forbidden by policy.

Trace interpretation:

The prior Level 5 `seeded_repair_timeout` is stochastic rather than a consistent
structural failure. In the successful Infring run, import-surface seeding happened
immediately, then the seeded repair turn completed in one provider turn and wrote
both `calcpack/arithmetic.py` and `calcpack/__init__.py` with passing validation
and semantic probe. This means the seeded import-surface primitive can work and
should not be patched blindly from one timeout failure.

Decision:

No runtime patch from this run. The next evidence step should be repeated Infring
Level 5 reliability sampling and restoration of unavailable reference runners
(Aider venv, ForgeCode binary) if we want them to remain useful in the loop.


### EXP-CODING-070: Level 5 reliability and Level 2-6 sweep

Status: `failure_target_identified`

Level 5 reliability sample:

- Report directory: `references/coding-agent-systems/runtime_trace_harness/reports/level5_reliability_20260531/`
- Infring Level 5: `5/5` pass.
- Wall time range: about `11.4s` to `22.4s`; average about `16.1s`.
- Time to first mutation range: about `11.3s` to `22.3s`; average about `16.0s`.

Level 2-6 sweep:

- Level 2 report: `references/coding-agent-systems/runtime_trace_harness/reports/infring_level2_sweep_20260531.json`
- Level 3-6 report: `references/coding-agent-systems/runtime_trace_harness/reports/infring_level3_6_sweep_20260531.json`
- Level 2: fail, `no_successful_mutation`, about `100.1s`.
- Level 3: fail, `no_successful_mutation:1`, about `45.0s` average wall time.
- Level 4: fail, `no_successful_mutation:1`, about `60.2s` average wall time.
- Level 5: pass, `pass:1`, about `9.3s` average wall time.
- Level 6: fail, `seeded_repair_timeout:1`, about `52.9s` average wall time.

Interpretation:

Focused Level 5 is currently healthy: the seeded public-interface repair lane passed 5/5 and also passed in the broad sweep. The broad Level 2-6 sweep exposes a separate weakness: Level 2/3/4 are timing out without successful mutation, while Level 6 reaches immediate seeded import-surface mutation but times out before implementing the larger persistence behavior. Do not patch Level 5 from this data. The next primitive target should be the lower existing-project mutation-entry path for Levels 2-4, with Level 6 tracked separately as a larger seeded-repair capacity issue.

### EXP-CODING-068: Direct existing-project mutation entry

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: direct_existing_project_mutation_entry
Profile touched: 2-4
Activation evidence: existing-project patch lane has bounded local read context, no successful mutation yet, compact mutation entry is available.
Reference pattern: Codex and Claude traces enter edit-capable tool use directly after context, instead of depending on a failed speculative first-mutation lane to unlock compact recovery.
Invariant: simple existing-project edit lanes own their mutation entry; speculative artifact lanes must not be a hidden prerequisite for compact recovery.
Rollback condition: Level 1-4 regress, Level 5 seeded repair path regresses, or direct entry mutates without required local context receipts.
```

Patch:

- Added CD-gated `direct_existing_project_mutation_entry_enabled` to the official coding workflow and local mutation-spine workflow.
- When enabled, bounded existing-project patch tasks with loaded context arm `mutation_only_recovery` directly, or `validation_guided_compact_repair` when failed validation evidence exists.
- The speculative `first_mutation_artifact_lane_v1` remains implemented, but this direct lane skips it for the selected simple existing-project mutation entry path.

Expected impact:

```text
bounded context loaded
-> direct compact mutation entry armed
-> file_write/file_patch first mutation
-> existing validation/spine closure
```

Measurement:

Pending. Next canary should run lower-level monotonic checks before returning to higher-level optimization.

### EXP-CODING-069: Direct mutation-entry timeout and owner-path contract

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: direct_mutation_entry_provider_budget_and_owner_path_contract
Profile touched: 2-4
Activation evidence: Level 2 compact mutation entry reached provider work but timed out before mutation; Level 4 direct validation repair emitted a path rejected by the owner-source guard.
Reference pattern: Codex/Claude traces use one edit-capable turn with exact workspace paths from observed context, and do not spend hidden time in thinking/planning before tool-call JSON.
Invariant: compact mutation-entry lanes receive a CD-owned provider budget and exact observed owner paths; they do not rely on level-specific fixtures or absolute-path guessing.
Rollback condition: Level 3 regresses, Level 4 still requires a second broad turn for owner-path repair, or Level 2 still fails without mutation after the larger compact budget.
```

Patch:

- Added CD-owned `mutation_only_recovery_provider_timeout_seconds` for compact mutation-only turns.
- Set compact mutation-only turns to omit Ollama thinking flags just like validation-guided compact repair turns.
- Changed the validation-guided repair packet to require the exact observed owner path listed in the packet instead of an invented absolute path.

Measurement:

Pending.

### EXP-CODING-070: Evidence-satisfied tool-loop closure

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: evidence_satisfied_tool_loop_closure
Profile touched: 2+
Activation evidence: Level 2 attempts produced mutation receipts, passed validation, passed semantic probes, and exposed expected symbols, but runtime continued into partial_blocked.
Reference pattern: mini-SWE-agent stops after validation/probe success; Infring should let the coding execution spine close immediately once receipt-backed evidence is sufficient.
Invariant: after any native tool batch, if the coding execution spine returns close_success and mutation evidence exists, the runtime returns a receipt-backed success instead of spending more provider turns.
Rollback condition: any lower-level run closes success without mutation receipts, validation-required tasks close before validation, or Level 3/4 regress.
```

Patch:

- Added a general post-tool-batch closure check using `coding_execution_spine_decision_from_native_receipts`.
- The check is not level-specific and only closes when successful mutation receipts exist and the spine declares required evidence satisfied.

Measurement:

Pending.

### EXP-CODING-071: Receipt-producing compact mutation entry

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: receipt_producing_compact_mutation_entry
Profile touched: 2-4 compact/direct mutation entry
Activation evidence: Level 2 compact mutation turns used command_run heredoc edits that changed files but were rejected by the shell preserved-API guard, leaving zero successful mutation receipts.
Reference pattern: mini-SWE/aider-style fast paths mutate quickly, but their shell edits only count when the runtime can trust and receipt them. Infring's current trusted primitive is file_write/file_patch, not shell-edit receipt synthesis.
Invariant: compact mutation-entry lanes must use receipt-producing file mutation tools before validation/finalization. command_run remains available for validation and later lanes, but not as the first compact product mutation primitive.
Rollback condition: Level 3/4 regress, multi-file compact edits fail because the model cannot emit multiple file_write/file_patch calls, or a future shell-edit receipt synthesizer safely replaces this constraint.
```

Patch:

- Removed `command_run` from the compact mutation recovery tool menu.
- Updated compact mutation-entry prompts to request only `file_write`/`file_patch` mutation calls, including multi-file batches.

Measurement:

Pending.

### EXP-CODING-072: Receipt-backed completion evidence precedence

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: receipt_backed_completion_evidence_precedence
Profile touched: bounded/direct existing-project completion gates
Activation evidence: Level 2 attempt 3 produced source/test mutations, passed validation, passed the external semantic probe, and exposed expected symbols, but runtime stayed partial_blocked because generic prompt words were interpreted as public API evidence gaps and stale partial/blocked model text overrode receipts.
Reference pattern: Codex/Claude/mini-SWE style loops stop once concrete mutation plus validation evidence satisfies the task slice; stale model self-status does not outweigh executable evidence.
Invariant: completion gates derive public API requirements from explicit imports/calls/action names, not generic requirement nouns, and receipt-backed completion suppresses stale reported-uncovered wording.
Rollback condition: real missing public API/export cases close success, Level 4 owner-source repair regresses, or Level 7/8 closes without required durable evidence.
```

Patch:

- Added generic public-API stopwords for requirement nouns that are not API symbols.
- Changed artifact repair reasons so receipt-backed complete runs are not blocked solely by stale `partial_or_blocked`/uncovered wording.

Measurement:

Pending.

### EXP-CODING-073: Revert file-only compact mutation entry

Status: `reverted`

Finding: EXP-CODING-071 removed `command_run` from compact mutation entry to force receipt-producing file tools. A Level 3/4 smoke stayed healthy, but the next Level 2 canary regressed to provider timeouts/no-mutation before the first edit. This suggests the weak-model compact path still benefits from shell-style batch edit affordances, matching mini-SWE/aider trace patterns.

Decision: Revert the file-only compact mutation-entry constraint. Keep the broader receipt-backed completion evidence patch from EXP-CODING-072. Future shell optimization should improve controlled shell-edit receipt synthesis or recovery, not remove the shell batch affordance from the weak-model fast path.

### EXP-CODING-074: Small-step compact mutation loop

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: small_step_compact_mutation_loop
Profile touched: compact mutation entry and context-to-mutation retry prompts
Activation evidence: mini-SWE-agent solved all three Level 2 tasks with the same weak model in roughly 14-20s by using a tiny JSON action schema, reading context/probe files, then issuing one or two useful shell commands per turn. Infring already reads the same context/probe files, but its compact prompt asks for a single giant multi-file edit batch and shows slower, less reliable semantic completion.
Reference pattern: small sequential mutation turns, each with bounded command count, instead of monolithic multi-file generation.
Invariant: the runtime remains primitive-first: no level names, fixture paths, or task-specific branches. The compact lane now expresses an execution-shape primitive: smallest coherent mutation step, then let runtime continue/validate.
Rollback condition: Level 3/4 regress, Level 2 latency worsens without semantic improvement, or higher-level staged lanes lose necessary atomicity.
```

Patch:

- Replaced “one command_run shell edit batch” compact mutation wording with “one or two useful mutation commands for the next coherent step.”
- Told multi-file compact work to split source/export/test edits across sequential runtime turns rather than one giant batch.

Measurement:

Pending.

### EXP-CODING-075: Revert prompt-only small-step compact loop

Status: `reverted`

Finding: EXP-CODING-074 copied the reference small-step behavior into prompt wording only. It improved one simple Level 2 attempt but did not produce a broad positive delta: attempts 2 and 3 still missed semantic behavior and wall time increased to roughly 155-173s.

Decision: Revert the prompt-only small-step change. Keep the trace insight: mini-SWE's winning behavior appears to come from an actual controller shape with a tiny shell-action schema, one/two commands per turn, observations after each step, validation/probe closeout, and a simple stop rule. That should be implemented as a primitive controller, not as more text inside the current compact mutation prompt.

### EXP-CODING-076: Restore compact shell mutation tool menu

Status: `patched_pending_measurement`

Finding: The intended revert of the file-only compact mutation-entry trial restored shell-oriented prompt text but did not restore `command_run` to the compact mutation recovery tool menu. This left the runtime in an inconsistent state: the model was told command-run shell edits were allowed, while the tool menu exposed only file write/patch tools.

Decision: Restore `command_run` and shell aliases to the compact mutation recovery tool filter before building the next controller primitive. This is a primitive surface repair, not a task-specific workaround.

Measurement:

Pending.

### EXP-CODING-077: Semantic probe as native auto-validation

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: local_semantic_probe_auto_validation
Profile touched: post-mutation validation/repair loop
Activation evidence: mini-SWE-agent Level 2 traces solved all attempts by validating both unit tests and `.infring/semantic_probe.py` before stopping. Infring already bootstraps the semantic probe as context, but can finalize or continue from unit validation without treating the probe as native repair evidence.
Reference pattern: after mutation, run validation plus the local semantic probe; semantic probe failure is actionable command output, not an external harness-only signal.
Invariant: this is a generic local-project primitive. Any workspace-owned `.infring/semantic_probe.py` becomes part of auto-validation after mutation. No level names, fixture paths, or expected symbols are hardcoded.
Rollback condition: lower levels without semantic probes regress, validation command behavior changes unexpectedly, or semantic probes run before mutation.
```

Patch:

- When native auto-validation runs after mutation and `.infring/semantic_probe.py` exists, append the semantic probe to the validation command under `PYTHONPATH=src:.`.
- The combined command succeeds only when both validation and semantic probe pass, creating repair evidence from probe failures.

Measurement:

Pending.

### EXP-CODING-078: Shell-edit guard feedback enters repair loop

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: guarded_shell_edit_repair_feedback
Profile touched: compact shell-edit mutation recovery
Activation evidence: Level 2 attempts used valid fast shell heredoc edits, but the preserved-API guard rejected the batch and the runtime ended with zero trusted mutation receipts. The guard's required next tool was `file_patch_or_file_write...`, while the recovery detector only recognized `file_write_or_file_patch...`.
Reference pattern: fast shell edit attempts are acceptable, but guard feedback must become a targeted follow-up repair turn instead of terminal no-mutation failure.
Invariant: keep the preserved-API guard. Do not globally weaken it. Normalize guard feedback into the existing mutation-recovery loop.
Rollback condition: guard-blocked invalid edits start closing as success, or Level 3/4 regress.
```

Patch:

- Treat preserved shell-edit guard reasons as mutation blocker reasons.
- Accept both `file_write_or_file_patch...` and `file_patch_or_file_write...` required-next-tool forms.

Measurement:

Pending.

### EXP-CODING-079: Placeholder tool calls are non-executable

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: placeholder_tool_call_rejection
Profile touched: native tool-call parsing
Activation evidence: after shell-edit guard feedback, the model emitted a placeholder command `...`; the runtime executed it as shell instead of treating it as an invalid/non-action tool call.
Reference pattern: successful coding frameworks never execute placeholder tool payloads as real actions; invalid tool intent stays in the agent loop as a retry/repair condition.
Invariant: placeholder tool calls are not executable tools. This is parser-level and applies across workflows, not to a specific eval case.
Rollback condition: legitimate tool calls are filtered out, or no-tool retry loops increase without producing concrete repair prompts.
```

Patch:

- Added `...`/ellipsis to placeholder argument detection.
- Removed placeholder fallback from native tool-call parsing so all-placeholder payloads produce no executable tool call.

Measurement:

Pending.

### EXP-CODING-080: Revert parser-level placeholder rejection

Status: `reverted`

Finding: EXP-CODING-079 made all-placeholder tool payloads parse as no executable calls. The Level 2 canary regressed: attempt 2 fell into `native_tool_terminal_tool_calls_after_finalization`, attempt 3 lost semantic success, and average runtime worsened. Parser-level empty-call conversion is too blunt for this controller path.

Decision: Revert the parser-level placeholder rejection. Keep the finding that placeholder payloads need handling, but implement it in a controller/tool-repair lane that can preserve observations and produce a targeted retry, not by changing global parse semantics.

### EXP-CODING-081: Native shell action schema adapter

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: native_shell_action_schema_adapter
Profile touched: native tool-call protocol, compact coding control surface
Activation evidence: mini-SWE-agent solved the Level 2 weak-model tasks with a tiny `actions: [{command: "..."}]` shell-action schema, while Infring only accepted `tool_calls`/single-tool JSON and therefore could not directly reuse that compact controller shape.
Reference pattern: compact shell-action JSON maps to normal shell execution, then the runtime owns observations, receipts, validation, repair, and final synthesis.
Invariant: this is a protocol adapter, not an eval branch. It preserves existing `tool_calls` parsing, translates non-empty action commands into `command_run` receipts, and skips literal placeholder action commands without changing global placeholder fallback behavior.
Rollback condition: Level 3/4 regress, action parsing creates ambiguous tool execution, or Level 2 latency worsens without improving mutation/semantic completion.
```

Patch:

- Added `actions` array parsing to the native tool protocol.
- Translated each non-empty `command`/`cmd` action into a `command_run` call with `["sh", "-lc", command]`.
- Left existing `tool_calls` behavior and placeholder fallback intact to avoid repeating the EXP-CODING-079 regression.

Measurement:

Pending.

### EXP-CODING-089: Dormant action-controller recovery measurement

Status: `measured_no_code_change`

Finding: The compact action controller experiment should remain dormant. With the controller active and protocol-aligned, Level 3 and Level 4 closed successfully but slower than the stable path, and the model did not reliably use the requested `actions` surface. With the controller disabled again, Level 3 passed in about 15.0s and an immediate Level 4 retry passed in about 15.7s. A same-batch Level 4 failure was provider-timeout noise, not an action-controller regression, because the controller was off and the retry passed.

Decision: Do not reactivate the compact action controller until the unified model has a cleaner reason to use it. The next primitive target should be selected from reference traces around concise edit prompts, tool-call streaming latency, and runtime-owned closeout.

### EXP-CODING-090: Iterative public import-surface seed

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: iterative_public_import_surface_seed
Profile touched: public import-surface bootstrap before seeded repair
Activation evidence: Current Level 6 failed after the deterministic import-surface seed created `DeliveryAttemptLedger` but did not expose the next missing public import, `summarize_attempts`. Python import validation stops at the first missing symbol, so one seed pass can leave later public API gaps hidden until the slower model repair turn.
Reference pattern: Claude/Codex/Forge Level 6 traces complete the public API surface before final validation; Infring should hand the model a behavior-repair slice, not an avoidable import-export discovery loop.
Invariant: generic and bounded. The runtime repeats validation -> import-surface seed only while generic Python cannot-import evidence produces new public symbols. No fixture names, expected symbols, or test-level branches are encoded.
Rollback condition: lower levels regress, import seed loops without new receipts, or deterministic placeholder exports hide real behavior failures as success.
```

Patch:

- Added a bounded `python_import_surface_seed_round_limit` with default/clamped range `1..=5`.
- After each successful import-surface seed, rerun the same pre-mutation validation to surface the next missing public import before model repair.
- Enabled a round limit of `3` in `local_coding_phase1_mutation_spine`.

Measurement:

Pending.

### EXP-CODING-091: Scoped seeded-repair provider budget

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: seeded_import_surface_repair_provider_budget
Profile touched: seeded public import-surface behavior repair turn
Activation evidence: EXP-CODING-090 moved Level 6 from missing public imports to behavior-level failures, but the seeded repair turn still timed out at the generic 30s budget with a larger post-seed contract packet. Reference Level 6 traces from Claude/Codex/Forge often reach first mutation around 22-32s, so the generic 30s cap is too tight for this heavier slice.
Reference pattern: keep direct edit loops bounded, but budget heavier public API/persistence repair differently from tiny Level 3 edits.
Invariant: scoped timeout only. This does not raise the global provider timeout and does not add level or fixture branches.
Rollback condition: lower levels slow down, seeded repair loops consume the full budget without higher mutation/pass rate, or provider timeouts shift later without useful edits.
```

Patch:

- Added `seeded_import_surface_repair_provider_timeout_seconds` with default/clamped range `8..=60`.
- Set the active mutation spine value to `45` seconds.

Measurement:

Pending.

### EXP-CODING-092: Seeded repair file-write overwrite slice

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: seeded_repair_overwrite_slice
Profile touched: seeded public import-surface behavior repair turn
Activation evidence: EXP-CODING-091's scoped 45s timeout did not create useful mutation; it only delayed the same timeout. The model also attempted context/tooling instead of a direct source edit, suggesting the seeded repair prompt/tool shape was still too ambiguous and patch-oriented for a weak model.
Reference pattern: successful frameworks make a direct owner-file edit once the relevant context and validation failure are known. For a runtime-created public API scaffold, the safe primitive is complete owner-file overwrite with receipt-backed validation, not another discovery/planning pass.
Invariant: generic seeded-owner overwrite. No eval symbols or fixture paths. Runtime supplies seeded owner paths and immutable contract evidence; the model may only `file_write` complete source/export content.
Rollback condition: lower levels regress, seeded repair overwrites unrelated files, or Level 6 still times out without model mutation.
```

Patch:

- Reverted the active scoped 45s timeout behavior from EXP-CODING-091 after it failed to produce useful mutation.
- Changed the seeded import-surface repair system/rule to `file_write` only with `overwrite=true`.
- Removed the generic mutation-entry packet from seeded repair prompt to avoid duplicated/patch-oriented instructions.
- Expanded seeded source skeleton snippets to include source bodies so complete overwrite is feasible.

Measurement:

Pending.

### EXP-CODING-093: Receipt-backed seeded repair task brief

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: receipt_backed_seeded_repair_brief
Profile touched: seeded public import-surface behavior repair turn
Activation evidence: After EXP-CODING-092, the prompt shrank but the model still tried `file_list` and timed out. The seeded repair prompt still included the raw user prompt, which can contain now-stale operational instructions like inspect files, run validation, or do not commit, conflicting with the runtime-owned repair lane.
Reference pattern: once runtime has loaded source, tests/probes, validation failures, and owner paths, strong coding loops operate from the current receipt/evidence state rather than replaying the original operational prompt.
Invariant: no task-specific content. The model receives a generic task brief plus receipt-backed contract/source evidence only.
Rollback condition: seeded repair loses user intent on tasks where validation/probe evidence is incomplete, or lower seeded-public-API slices regress.
```

Patch:

- Removed raw original prompt text from seeded import-surface repair prompt.
- Replaced it with a generic task brief instructing completion of the seeded public API required by immutable validation/probe evidence.

Measurement:

Pending.

### EXP-CODING-094: Dormant specialized seeded repair lane

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: seeded_state_routes_to_generic_mutation_recovery
Profile touched: post import-surface seed controller selection
Activation evidence: The specialized seeded repair lane kept timing out after multiple prompt simplifications. The useful part was deterministic iterative public import-surface seeding; the brittle part was the special repair controller.
Reference pattern: successful frameworks use a single coherent edit/recovery loop once context and validation evidence are available, rather than switching into a special one-off controller that conflicts with model instincts.
Invariant: no loss of deterministic seeding. Only the specialized seeded repair turn is disabled; normal mutation recovery still sees seeded source/export files, validation failures, and receipt context.
Rollback condition: Level 5 public API repair regresses or Level 6 fails earlier than behavior-repair state.
```

Patch:

- Set `seeded_import_surface_compact_repair_turn` to `false` in the active mutation spine workflow CD.

Measurement:

Pending.

### EXP-CODING-095: Import-surface seed is not product mutation for recovery routing

Status: `patched_pending_measurement`

Patch packet:

```text
Primitive: seed_mutation_excluded_from_product_recovery_state
Profile touched: timeout and recovery routing after deterministic import-surface scaffolds
Activation evidence: Level 6 now reaches behavior-level validation failures, but provider timeout routing treats deterministic seed file writes as successful product mutation. That suppresses first-turn timeout recovery and can keep the runtime in partial timeout instead of retrying from the newly loaded context.
Reference pattern: deterministic scaffolding is setup evidence, not completion evidence. Strong coding loops continue from setup/context into the real product mutation.
Invariant: only `runtime_python_import_surface_seed*` writes are excluded. Any non-seed file_write/file_patch still counts as product mutation.
Rollback condition: seeded scaffolds stop being visible as mutation receipts, or real post-seed product edits are no longer recognized.
```

Patch:

- Excluded import-surface seed writes from product-mutation timeout routing.
- Let mutation-only recovery run after seed-only state.
- Prevent the specialized seeded repair turn from overriding an explicit mutation-recovery retry.

Measurement:

Pending.

### EXP-CODING-096 - Disable pre-model import-surface seeding in phase1 mutation spine

- Date: 2026-06-01
- Change: Set `python_import_surface_seed_round_limit` to `0` in the phase1 mutation spine CD, leaving the seed primitive available but dormant for this lane.
- Rationale: Current Level 6 traces show scaffold-only seed writes are fast but create a partial API shell, then the model times out trying to repair behavior. Reference framework traces and prior passing Infring Level 6 runs use a single implementation pass after context plus validation evidence instead of a deterministic partial scaffold before the model acts.
- Success signal: Level 6 should return to a one-provider-turn implementation path with real source mutation before timeout, without losing lower-level direct edit stability.

### EXP-CODING-097 - Make import-surface seeding genuinely opt-in

- Date: 2026-06-01
- Change: Allow `python_import_surface_seed_round_limit` to be `0` and default the helper to `0`; set the official coding workflow to `0` explicitly.
- Rationale: EXP-CODING-096 exposed that the helper clamped the seed limit to at least one, so workflow CD could not make the primitive dormant. That violated the primitive/config boundary and kept forcing partial scaffold writes before the model implementation turn.
- Success signal: Level 6 should no longer emit `bootstrap_python_import_surface_seed_end`; it should enter the normal bounded direct edit implementation path and either pass or produce a non-seed failure that can be targeted separately.

### EXP-CODING-098 - Restore full bounded-edit surface for first implementation pass

- Date: 2026-06-01
- Change: Disabled `compact_mutation_entry_packet_enabled` in the official coding workflow while keeping import-surface seeding dormant.
- Rationale: Current no-seed Level 6 trace timed out before mutation with a compact first turn (`system_chars` ~362, `tool_count` 3). Prior passing Infring Level 6 traces and successful reference-framework behavior use a richer one-pass implementation surface instead of a tiny recovery-like packet for this complexity tier.
- Success signal: Level 6 should produce a real model mutation in one bounded-edit implementation pass and move from `no_successful_mutation` toward pass or validation-repair failure.

### EXP-CODING-099 - Apply full first-pass surface experiment to active phase1 CD

- Date: 2026-06-01
- Change: Restored the official workflow's compact mutation setting and disabled `compact_mutation_entry_packet_enabled` in the active `local_coding_phase1_mutation_spine` CD.
- Rationale: EXP-CODING-098 had no observable effect because Level 6 was governed by the phase1 mutation spine. The trace still showed the compact 3-tool first turn, matching the active lab CD setting.
- Success signal: First provider turn should stop using the compact bootstrap mutation surface and should expose the full bounded-edit tool surface, matching prior passing traces more closely.

### EXP-CODING-100 - Calibrate phase1 full first-pass timeout to reference traces

- Date: 2026-06-01
- Change: Raised active phase1 `provider_timeout_seconds` from `30` to `60` while keeping compact bootstrap mutation disabled and import-surface seeding dormant.
- Rationale: EXP-CODING-099 restored the prior 8-tool/full-context first-pass shape but it was cut off at 30s. Prior passing Infring traces and Claude/Codex/Forge Level 6 traces show first useful mutations commonly land between ~22s and ~50s on the controlled model.
- Success signal: Level 6 should complete from the first full implementation pass instead of falling into timeout recovery; if it fails, the failure should be validation/semantic, not no-mutation timeout.

### EXP-CODING-101 - Decouple fast-lane timeout fallback from compact packet

- Date: 2026-06-01
- Change: If `first_mutation_artifact_lane_v1` times out or produces no mutation, the runtime now arms the parent native tool loop even when compact mutation packets are disabled.
- Rationale: The Level 3 smoke showed the fast lane logged `demoted_to_parent_runtime_loop` but returned `partial_blocked` because fallback recovery was gated on `compact_mutation_entry_packet_enabled`. That made one Level 6-oriented config change break simpler tasks.
- Success signal: Level 3 should continue into the parent native tool loop and produce a source/test mutation instead of ending after the fast-lane timeout with only read receipts.

### EXP-CODING-102 - Route compact bootstrap by task shape, not globally

- Date: 2026-06-01
- Change: Re-enabled compact mutation packets in the active phase1 CD, but prevented compact bootstrap mutation turns for prompts that require pre-mutation validation.
- Rationale: Level 3 needs the compact packet after a fast-lane timeout, while Level 6 regressed when validation-guided work was forced through the compact first pass. This matches the unified model: simple existing-project edits use compact mutation entry; validation-guided/public-surface tasks use full bounded implementation context.
- Success signal: Level 3 should regain compact mutation behavior, while Level 6 should keep the full 8-tool/60s first implementation pass.

### EXP-CODING-103 - Align validation-guided full-pass timeout with operator budget

- Date: 2026-06-01
- Change: Raised active phase1 `provider_timeout_seconds` from `60` to `75` after routing compact bootstrap away from validation-guided prompts.
- Rationale: EXP-CODING-102 fixed Level 3 via compact routing, but Level 6 remained stochastic: one run passed with first tool calls at ~46s, another timed out at 60s. The official project-operator first-mutation budget is 75s, and reference framework traces show first useful mutations often land in the 22-60s range on the controlled model.
- Success signal: Level 6 should produce first-pass mutations more reliably without affecting Level 3, which uses the compact mutation-only timeout path instead of this full-pass timeout.

### EXP-CODING-104 - Align first-receipt watchdog with full-pass budget

- Date: 2026-06-01
- Change: Added `first_receipt_deadline_seconds: 75` to the active phase1 CD.
- Rationale: EXP-CODING-103 set `provider_timeout_seconds` to `75`, but Level 6 traces still showed `ollama_run_timeout:timeout_seconds=60` because the first-receipt watchdog clamped the first implementation turn. Compact Level 3 remains governed by its mutation-only timeout, so this targets validation-guided full-pass work only.
- Success signal: Level 6 first turn should show `timeout_seconds=75` on timeout, or complete if the model emits tool calls between 60s and 75s.

### EXP-CODING-105 - Require first visible JSON byte for compact mutation turns

- Date: 2026-06-01
- Change: Tightened `native_tool_mutation_only_recovery_system` so mutation-only compact turns require `{` as the first visible byte and exactly one JSON object.
- Rationale: Current Level 3 traces showed the compact path could pass but with high variance: one run timed out at 45s, then a second compact turn succeeded after another ~44s. Stream diagnostics showed early visible output but delayed JSON opening brace, implying the model was spending visible/hidden output before the actual tool call. Successful reference-style agent behavior is tool-call-first for simple edits.
- Success signal: Level 3 compact turns should produce JSON tool calls earlier, reducing first-turn timeout frequency without changing the validation-guided full-pass Level 6 route.

### EXP-CODING-106 - Block non-edit commands before mutation in mutation-only lane

- Date: 2026-06-01
- Change: Added a runtime guard that blocks non-edit `command_run` calls during mutation-only recovery before any successful mutation. Controlled shell edit commands that write project files remain allowed.
- Rationale: EXP-CODING-105 improved JSON-first compact behavior, but a Level 3 sample still spent an extra turn because the first compact response emitted a non-mutating command before the actual edit. The mutation-only lane contract already forbids validation/read/list/prose before mutation; this makes the runtime enforce that primitive instead of trusting prompt text alone.
- Success signal: Level 3 should avoid wasted non-edit command turns and either mutate in the first compact response or receive a structured blocked receipt that keeps the next retry focused on file_write/file_patch/controlled shell edit.

### EXP-CODING-107 - Give validation-guided compact repair the controlled edit surface

- Date: 2026-06-01
- Change: Validation-guided compact repair now uses the mutation recovery tool surface, allowing file_write/file_patch or a controlled shell edit command that writes project files. Non-edit command_run is blocked before mutation in this lane.
- Rationale: The Level 4 smoke passed but spent 45s timing out in a two-tool validation-guided edit-only turn, then succeeded in the mutation-only fallback that had access to the controlled shell edit surface. This patch keeps the validation-guided lane edit-only, but gives it the same atomic writer option that already works in the compact lane.
- Success signal: Level 4 should mutate in the first validation-guided compact repair turn more often and avoid the 45s timeout + fallback pattern.

### EXP-CODING-108 - Re-enter compact repair after required-mutation blockers

- Date: 2026-06-01
- Change: Generalized the no-mutation blocker loop so owner-target blocks and preflight-context blocks re-enter a compact mutation/repair lane instead of falling back to the full parent loop.
- Rationale: A Level 4 trace failed with `product_mutation_must_target_observed_owner_file`, then expanded into discovery/validation/read attempts even though runtime had already loaded context and validation evidence. Reference-framework traces stay narrow after a bad edit target: they retry the smallest edit against the observed owner file.
- Success signal: Level 4 should recover from wrong-owner or extra-context attempts by retrying file_write/file_patch against observed product/source owner files, not by reopening discovery.

### EXP-CODING-109 - Keep validation-guided timeout retries validation-guided

- Date: 2026-06-01
- Change: When the first validation-guided compact repair provider turn times out before mutation, the retry now stays in the validation-guided compact repair lane instead of expanding to mutation-only recovery.
- Rationale: A 5-run Level 3/4 smoke after EXP-CODING-108 showed Level 3 at 5/5 and Level 4 at 4/5; the lone Level 4 failure was two provider timeouts, with the second retry using the broader mutation-only prompt. Codex/reference Level 4 traces keep the retry focused rather than broadening the surface.
- Success signal: Level 4 should convert first-turn provider timeouts into focused validation-guided retries; the immediate 5-run Level 4 batch passed 5/5, including one recovery from a first-turn 45s timeout.

### EXP-CODING-110 - Route import-surface validation by evidence, not level

- Date: 2026-06-01
- Change: Treat failed validation class `import_surface_missing` as an import-surface task shape: skip compact direct-entry repair, allow the import-surface seed primitive even when the workflow CD keeps its default round limit dormant, and keep this gated by receipt evidence rather than eval level.
- Rationale: Current Level 7 failed with two compact 45s validation-guided timeouts before any mutation. Stored passing Level 7 traces used a fuller implementation path, while the immediate validation evidence showed missing Python public imports.
- Result: The seed primitive fired and produced immediate source/export mutations, changing Level 7 from no-mutation failure into seeded repair / behavior-repair failure. This is progress but not yet pass.

### EXP-CODING-111 - Validate after every import-surface seed round

- Date: 2026-06-01
- Change: Moved import-surface seed validation before the loop-limit break so a seed round is always measured immediately, even when only one round is allowed.
- Rationale: The runtime previously seeded source/export files and then asked the model to repair without first checking whether the deterministic seed changed the validation state. Reference-style repair loops validate after mutation before asking for more edits.
- Result: Post-seed validation exposed the next concrete missing import or behavior failure instead of leaving the runtime blind.

### EXP-CODING-112 - Allow iterative import-surface seed rounds from validation evidence

- Date: 2026-06-01
- Change: When validation class is `import_surface_missing`, allow the existing import-surface seed primitive to iterate up to its existing cap instead of stopping after one symbol.
- Rationale: Python unittest import failures expose one missing imported symbol at a time. One seed round added `InventoryCatalog`; validation then exposed `summarize_inventory`, then `write_reorder_report`.
- Result: The runtime progressed past import marker coverage; `expected_markers_present` became true, and remaining failures moved into behavior/validation repair.

### EXP-CODING-113 - Align seeded repair writes with seed-owned paths

- Date: 2026-06-01
- Change: Let seeded import-surface repair overwrite files touched by the seed primitive while preserving existing API write guards everywhere else.
- Rationale: The seeded-repair prompt explicitly asked for overwriting seeded source/export files, but the preserved-API guard blocked those writes. The two primitives contradicted each other.
- Result: Model implementation writes began dispatching after seed. Remaining failures were normal behavior validation failures, not guard mismatches.

### EXP-CODING-114 - Block placeholder mutation paths

- Date: 2026-06-01
- Change: Added a generic placeholder-path guard for mutation calls and replaced the seeded-repair example path with a token that explicitly must be replaced by an owner target.
- Rationale: A repair turn copied `exact seeded owner path` literally and the runtime accepted it as a new file path. Coding runtimes should reject placeholder paths generically instead of silently creating bogus files.
- Result: Placeholder writes are now blocked with a structured receipt. Level 7 still does not pass; after exiting the seed lane it drifted into a long generic loop and hit the harness timeout.

### EXP-CODING-115 - Exit seeded repair after first real implementation mutation

- Date: 2026-06-01
- Change: Seeded import-surface repair now only applies before the first non-seed implementation mutation. After real source/export mutation, follow-up validation failures leave the seed lane.
- Rationale: Once the model has made real implementation changes, remaining failures are normal behavior repair, not import-surface seeding.
- Result: The runtime exited the seed lane as intended, but Level 7 hit the outer harness timeout in a generic repair loop. Keep the evidence, but do not stack more blind patches; next step should be a focused repair-lane design based on reference traces.

### EXP-CODING-116 - Classify control gates from latest validation failure

- Date: 2026-06-01
- Change: Added a latest-failed-validation evidence primitive and switched runtime control classifications to use it instead of aggregate failed-validation history. Aggregate validation details remain available for context and receipts.
- Rationale: The Level 7 import-seed run reached first mutation in 243ms but still entered `seeded_import_surface_repair_turn` and timed out. The reference traces indicate successful systems leave import repair once imports resolve and move to normal implementation/behavior repair. The likely primitive bug was stale aggregate evidence: an old `cannot import name` validation could keep the import-surface lane alive after newer validation had advanced to behavior failures.
- Success signal: Seeded import-surface repair should run only while the latest validation failure is still `import_surface_missing`; once validation progresses to `attribute_missing`, `assertion_mismatch`, or another behavior class, the runtime should route to the normal implementation/repair surface.

### EXP-CODING-117 - Route deterministic seed failures directly to compact repair

- Date: 2026-06-01
- Change: When a runtime deterministic seed has mutated files and the latest validation still fails, arm the validation-guided compact repair lane before the first model implementation turn.
- Rationale: EXP-CODING-116 moved Level 7 from seed-lane timeout to `attribute_missing`, but the trace still spent 75s on a 19k-character full implementation prompt before trying compact repair. Reference traces from faster systems move from known validation evidence to a narrow repair action instead of doing a full rediscovery/implementation prompt.
- Success signal: Level 7 should skip the slow full first-pass prompt after deterministic import seeding and start with the compact validation-guided repair prompt; if it fails, the failure should reflect repair quality rather than full-prompt timeout overhead.
- Result: Failed and reverted. The route skipped the slow first full prompt but fell into a longer generic loop and hit the outer 240s harness timeout. Keep the lesson: compact-first after seed needs a better bounded repair controller, not a simple pending-flag redirect.

### EXP-CODING-118 - Compact seeded validation observations

- Date: 2026-06-01
- Change: After deterministic import-surface seeding, the bootstrap prompt now injects a compact observation containing changed paths, seed paths, failed validation refs, latest validation class, and latest failed validation details instead of raw aggregate receipt history.
- Rationale: Passing Level 7 framework traces and older passing Infring traces use roughly 9-10k prompt contexts for the implementation turn. The seeded path inflated the first implementation prompt to about 19k by serializing all receipt history, causing a 75s provider timeout before repair could begin. Raw receipts remain in receipts/logs; the LLM prompt should receive bounded current evidence.
- Success signal: Level 7 first implementation prompt should shrink toward the passing trace range and avoid the 75s first-turn timeout without changing the workflow into an eval-specific route.
- Result: Mixed/insufficient. The first implementation prompt shrank to about 5.4k, but two consecutive runs still timed out at 75s. This likely over-compressed the packet by dropping useful read context that older passing traces preserved.

### EXP-CODING-119 - Preserve read context in compact seeded observations

- Date: 2026-06-01
- Change: Expanded the seeded compact observation to include bounded context receipts (`file_list`, `file_stat`, `file_read`, `file_read_many`, `command_resolve`), seed mutation receipts, and only the latest failed validation command instead of all validation history.
- Rationale: Passing Infring Level 7 traces used a one-turn implementation context around 9-10k chars, not a tiny context-free repair packet. The right primitive is not "drop receipts"; it is "keep current context and latest failure, drop stale validation history."
- Success signal: First implementation prompt should stay bounded but include enough local context for a one-turn implementation attempt, avoiding both 19k receipt bloat and 5k context starvation.
- Result: Failed. Prompt size rose back to about 16.8k and still hit the 75s first-turn timeout. Raw context receipts are still too bulky.

### EXP-CODING-120 - Reuse mutation-entry packet for seeded context

- Date: 2026-06-01
- Change: Replaced raw context receipt injection in seeded compact observations with the existing `native_tool_mutation_entry_packet` primitive plus latest failed validation evidence.
- Rationale: The mutation-entry packet already provides bounded candidate paths and compact file-content slices. This is closer to older passing Infring traces and avoids inventing a parallel context-packet format.
- Success signal: First implementation prompt should land between the 5k context-starved packet and the 16-19k receipt-bloated packets, while preserving enough file/test surface for the model to emit a mutation.
- Result: Partial. Prompt size landed at about 7.9k, close to old passing traces, but the broad full implementation surface still timed out at 75s.

### EXP-CODING-121 - Treat deterministic seed as scaffolding for staged edit

- Date: 2026-06-01
- Change: The staged edit controller now checks for first non-import-surface implementation mutation instead of any mutation, so deterministic import-surface seed writes do not disqualify the run from the staged edit lane.
- Rationale: Older passing Infring Level 7 traces used a staged controller with a small system prompt and two-tool surface. Seed writes are scaffolding, not completed implementation. Counting them as the first mutation forced the runtime into the broad full implementation surface.
- Success signal: Seed-prepared Level 7 should enter the staged edit controller on the first model turn, reducing tool/system surface while keeping the bounded mutation-entry packet.
- Result: Did not activate. The active workflow metadata has `staged_edit_controller` disabled, so the condition change alone could not route the seed-prepared task.

### EXP-CODING-122 - Enable staged edit for seed-prepared tasks only

- Date: 2026-06-01
- Change: Allow the staged edit controller to activate when runtime has deterministic import-surface seed receipts and no non-seed implementation mutation, even if the general staged controller flag is off.
- Rationale: This keeps lower-level bounded edit behavior untouched while testing the reference-backed path that older Infring used successfully for Level 7: a small staged implementation surface after context is known.
- Success signal: Level 7 trace should show `staged_edit_turn: true` on turn 0 with a reduced tool/system surface. If it improves, the staged controller can later become a declared CD option rather than a broad default.
- Result: Passed. Level 7 completed in 47.6s with `staged_edit_turn: true`, a 2-tool surface, and one successful provider turn. This is competitive with the successful reference framework traces and avoids the previous seed repair / full prompt timeout loops.
- Broad smoke result: Reverted. A 3-7 smoke after this branch passed Levels 3/4 but failed Levels 5/6 and did not reproduce Level 7. The branch is promising but not a monotonic primitive yet; reintroduce only behind a cleaner declared seed-prepared staged controller with lower-level gates.

### EXP-CODING-123 - CD-declared seed-prepared staged controller threshold

- Date: 2026-06-01
- Change: Added CD-controlled `seed_prepared_staged_controller_enabled` and `seed_prepared_staged_controller_min_source_seed_receipts`, plus a staged edit rule that explicitly treats runtime import-surface scaffolds as placeholders to replace. The Rust player now activates staged edit for seed-prepared tasks only when the number of source seed receipts reaches the declared threshold.
- Rationale: EXP-CODING-122 showed the staged path can pass Level 7 quickly, but it contaminated smaller Level 5/6 public-interface repairs. The portable separator is import-surface fanout: one or two missing public symbols should stay on simpler repair paths; larger fanout can justify a staged implementation controller.
- Success signal: Levels 3/4 should remain green, Levels 5/6 should avoid the staged controller, and Level 7 should use staged edit only when the runtime has observed at least the declared seed fanout threshold.
- Initial result: 3-7 smoke passed 5/5, but Level 7 did not activate staged edit because the harness uses `local_coding_phase1_mutation_spine`, not the official wrapper CD. This proved no regression but not the new controller path.
- Follow-up change: Moved the declared knobs onto `local_coding_phase1_mutation_spine` and updated `xtask` to pass `native_runtime_prompt_policy` from workflow CD into runtime metadata, so CD-owned staged prompt text is actually available to the native runtime player.
- Wired result: Mixed. The phase1 CD path activated staged edit for Level 7 and kept Levels 5/6 out of staged edit, but Level 7 emitted read tools despite a two-tool staged menu. This exposed a lower primitive bug: request tool menus were advisory instead of enforced.

### EXP-CODING-124 - Enforce declared per-turn tool surface

- Date: 2026-06-01
- Change: Added a generic declared-tool-surface guard. If a provider response asks for a tool not exposed in the current request tool menu, runtime returns a blocked receipt instead of dispatching the tool.
- Rationale: Staged edit exposed only mutation tools, but the model emitted `file_read_many` and runtime executed it. Successful coding frameworks keep the active tool surface authoritative; otherwise narrow staged controllers leak back into broad loops.
- Success signal: Staged/compact lanes should not execute undeclared read/command tools. Failures should become structured blocked receipts that can feed the retry/repair gate instead of silently expanding scope.
- Follow-up change: Updated pre-mutation blockers and recovery gates to require a non-seed implementation mutation. Runtime import-surface seed receipts are scaffolding and must not disable context/command blockers or required-mutation recovery.
- Follow-up result: The tool leak closed, but staged Level 7 could still return no executable tool calls and then time out the harness.

### EXP-CODING-125 - Bounded no-tool retry after staged edit

- Date: 2026-06-01
- Change: Added CD-controlled `seed_prepared_staged_empty_retry_limit`. When staged edit emits no executable tool calls before a non-seed implementation mutation, runtime enters mutation-only recovery with the compact mutation-entry packet instead of falling through to timeout/finalization.
- Rationale: Reference systems recover from malformed/no-op edit attempts by issuing a smaller edit-only retry. The controller should not let a staged no-tool response terminate or expand scope while implementation mutation is still missing.
- Success signal: A staged no-tool Level 7 attempt should produce a second mutation-only provider turn rather than a 240s runtime timeout with no native tool names.
- Initial result: The second mutation-only retry fired as intended, but hit the existing 45s mutation-only timeout. Added CD-controlled `seed_prepared_staged_retry_provider_timeout_seconds` so only this staged-empty retry can use a reference-matched 60s budget without globally slowing simpler mutation-only lanes.
- Final result: Not promoted. The 60s scoped retry still timed out before a real implementation mutation. `seed_prepared_staged_controller_enabled` is set back to `false` in both the phase1 mutation spine and official wrapper CDs. Keep the declared knobs dormant, plus the CD-player prompt-policy pass-through and declared-tool-surface guard as primitive infrastructure.

### EXP-CODING-126 - Trace provider tool-call parse shape

- Date: 2026-06-01
- Change: Added a lightweight `provider_response_tool_parse` runtime timeline event with parsed tool count, parsed tool names, output length, lane flags, and a short output preview only when no tool calls parse.
- Rationale: Several failures report provider status `ok` but no usable mutation. We need to distinguish no-tool prose, malformed JSON, wrong schema, forbidden tools, and parser misses before adding more behavior patches.
- Success signal: Future Level 7 traces should show whether the provider produced no tool calls, wrong tool calls, or parseable mutation calls that were blocked by runtime policy.
- Initial result: Current Level 7 failure did not reach parse; both provider turns timed out. This points to budget/context shape rather than parser shape.

### EXP-CODING-127 - Fanout-scaled seeded implementation budget

- Date: 2026-06-01
- Change: Added CD-controlled `seed_prepared_high_fanout_provider_timeout_seconds`, applied only when runtime has import-surface seed source receipts at or above the declared fanout threshold, no non-seed implementation mutation exists, and the turn is the broad implementation turn.
- Rationale: Current Level 7 high-fanout seeded implementation uses a 19k prompt and times out at 75s. Successful reference traces for comparable larger slices sit around 40-65s, while older passing Infring traces sometimes approach the 75s edge. This patch gives only high-fanout seeded implementation turns a 90s budget without slowing simpler Levels 3-6.
- Success signal: Level 7 should either complete the broad implementation turn or fail with a later validation/repair class instead of first-turn `ollama_run_timeout:timeout_seconds=75`.
- Result: Positive. Focused Level 7 passed in 81.3s with four parsed tool calls after the larger budget. A follow-up 3-7 smoke passed 5/5: Level 3 5.9s, Level 4 29.0s, Level 5 74.1s, Level 6 66.3s, Level 7 68.1s. This is promoted as the active scoped fix unless a larger reliability batch shows regression.

### EXP-CODING-128 - Owner-target repair receipt made actionable

- Date: 2026-06-01
- Goal: Repair the Level 7 failure where the agent reached validation-guided repair but patched a non-owner/import-surface target and could not recover.
- Change: Enriched the generic `product_mutation_must_target_observed_owner_file` receipt with attempted path, observed owner paths, seed paths, suggested owner path, and the required next action.
- Primitive intent: Keep the owner-target guard reusable while making its failure evidence actionable for any coding task that separates seed/import-surface scaffolding from product owner implementation files.
- Expected effect: Validation repair turns should recover from wrong-target mutations instead of timing out or repeatedly patching seed files.

### EXP-CODING-129 - Compact oversized bootstrap observation packets

- Date: 2026-06-01
- Goal: Address the Level 7 first-turn timeout caused by 19k-character full receipt observations after import-surface seeding.
- Evidence: Current failing Level 7 had `prompt_chars=19117` and timed out before tool parsing; prior passing/framework traces stayed closer to compact first-turn packets around 9k-10k chars.
- Change: Added a reusable bootstrap observation compaction primitive with a CD threshold. Oversized bootstrap prompts now preserve authoritative paths, compact source previews, validation tails, and guard reasons instead of serializing every full receipt.
- Primitive intent: Keep receipt authority while bounding prompt size for seeded multi-file coding tasks; no level-specific fixture behavior.
- Expected effect: Reduce first-turn timeout risk without weakening mutation receipts or owner-path guards.

### EXP-CODING-130 - Tighten compact bootstrap packet toward framework trace band

- Date: 2026-06-01
- Goal: Continue reducing Level 7 first-turn timeout after EXP-CODING-129 lowered prompt size from 19.1k to 14.5k but still timed out before parsed tool calls.
- Evidence: Prior passing framework/Infring traces were closer to 9k-10k first-turn prompts; 14.5k remained too slow for the control model.
- Change: Reduced compact observation receipt count, file count, source previews, and validation tails while preserving exact paths, success/error state, owner-target evidence, and validation output tails.
- Primitive intent: Bound context size for any oversized seeded bootstrap packet without changing task-specific routing or eval fixtures.
- Expected effect: Move first provider turn into the previously successful prompt-size band and recover parsed mutation calls instead of first-turn timeout.

### EXP-CODING-131 - Add CD-controlled recovery wall budget

- Date: 2026-06-01
- Goal: Fix the Level 6 case where a compact repair turn produced a `file_patch` after the first provider timeout, but the normal 90s loop wall budget expired before the run could complete recovery.
- Evidence: Level 6 smoke showed prompt sizes `[9889, 4694]`, provider statuses `[timeout, ok]`, parsed repair calls `[file_patch]`, and terminal `native_tool_loop_wall_timeout:timeout_seconds=90`.
- Change: Added `recovery_wall_timeout_seconds` as a native success-criteria knob and used it only while mutation-only or validation-guided recovery is pending.
- Primitive intent: Preserve fast normal failure bounds while allowing an already-entered recovery lane to finish dispatch/validation. This is a reusable scheduling primitive, not a level-specific timeout bump.
- Expected effect: Level 6 repair actions should execute after a slow first turn instead of being discarded by the normal wall timeout.

### EXP-CODING-132 - Raise compact repair provider cap to lane maximum

- Date: 2026-06-01
- Goal: Fix Level 6 recovery runs where the first provider turn times out and the compact validation-guided repair lane needs slightly more than 45s to emit tool calls with the control model.
- Evidence: After adding recovery wall budget, Level 6 reached the repair lane with `prompt_chars=4694` but timed out at `validation_guided_compact_repair_provider_timeout_seconds=45` before parsing calls.
- Change: Raised the validation-guided compact repair provider cap from 45s to 60s in workflow CD only.
- Primitive intent: Tune the reusable recovery lane budget without changing task semantics, adding case-specific code, or increasing the normal first-turn budget.
- Expected effect: Slow compact repair turns can emit their mutation calls while ordinary lanes keep their existing budgets.

### EXP-CODING-133 - Feed blocked mutation guard receipts into compact repair

- Date: 2026-06-01
- Goal: Fix Level 6 repair loops where a validation-guided repair turn emitted a wrong-owner/placeholder patch, the guard correctly blocked it, and the next repair turn lacked enough actionable blocker evidence to recover.
- Evidence: Combined 5-7 smoke showed Level 6 provider sequence `[timeout, ok, timeout]`; the successful repair turn parsed `file_patch` but produced `product_mutation_must_target_observed_owner_file`, then the next compact repair prompt timed out without resolving the exact target issue.
- Change: Validation-guided compact repair packets now include previous blocked mutation evidence: reason, attempted path, suggested path, observed owner paths, and next action from guard receipts.
- Primitive intent: Make guard receipts first-class repair evidence for any coding task, without special-casing Level 6 or its fixture names.
- Expected effect: Second repair attempts should target concrete owner files instead of repeating placeholder or wrong-target patches.

### EXP-CODING-134 - Let active recovery lane timeout dominate stale next-turn override

- Date: 2026-06-01
- Goal: Ensure the CD-configured compact repair timeout is actually used after first-turn timeout recovery.
- Evidence: Workflow CD set `validation_guided_compact_repair_provider_timeout_seconds=60`, but Level 6 repair still timed out at 45s because `next_provider_timeout_seconds` from bounded-patch timeout recovery overrode the active repair lane budget.
- Change: When validation-guided or mutation-only recovery is active, the provider timeout override now uses the maximum of inherited next-turn timeout and the active lane timeout.
- Primitive intent: Preserve lane-local budget authority and avoid stale controller state overriding the current recovery lane.
- Expected effect: Compact repair turns should receive their declared 60s budget after first-turn timeout recovery.

### EXP-CODING-135 - Narrow seeded implementation first-turn surface

- Date: 2026-06-01
- Goal: Reduce Level 6/7 first-turn timeout risk by matching the faster-framework pattern of a narrow edit/validate surface once local context and seed owner files are already prepared.
- Evidence: Latest failing 5-7 smoke showed Level 7 first turn at `prompt_chars=12016`, `system_chars=3625`, `tool_count=8`, with no parsed calls before timeout. Earlier faster/passing traces reached mutation through narrower edit/validate behavior rather than broad exploration.
- Change: The generic seeded high-fanout implementation turn now uses the compact mutation-only system prompt and mutation recovery tool menu. The timeline records `seed_prepared_high_fanout_implementation_turn` for trace comparison.
- Primitive intent: Convert seeded implementation from broad exploration into a bounded edit/validate action lane whenever receipts already provide the owner files and validation evidence.
- Expected effect: Lower system/tool surface on seeded implementation turns while preserving receipt gates, owner guards, and validation repair.

### EXP-CODING-135 verdict - Reverted

- Date: 2026-06-01
- Result: Reverted.
- Evidence: Focused Level 7 passed with reduced system/tool surface, but the broader 5-7 smoke regressed: Level 5, Level 6, and Level 7 all failed with seeded repair timeout/partial timeout patterns.
- Lesson: Narrowing the seeded implementation tool/system surface globally is too coarse. The winning behavior needs a better action packet or controller distinction, not a blanket tool-menu reduction.
- Follow-up: Consult framework traces for how they preserve enough implementation authority while avoiding broad first-turn latency.

### EXP-CODING-136 - Stop import-surface seeding once failure changes class

- Date: 2026-06-01
- Goal: Reduce duplicate seed scaffolding and prompt inflation by matching the reference-framework pattern: validate/read, then let the model write complete product files once importability is established.
- Evidence: Fresh Level 6 Claude/Codex traces show successful systems run validation, inspect tests/source, and full-write implementation/export files. Infring repeated import-surface seed rounds even after seed files existed, creating duplicate receipts and encouraging weak patch-repair loops.
- Change: The import-surface seed loop now stops after validation failure class changes away from `import_surface_missing`, even if validation still fails. That hands implementation/API failures to the model instead of continuing seed scaffolding.
- Primitive intent: Keep seed as an idempotent importability primitive, not a partial implementation loop.
- Expected effect: Smaller first-turn context, fewer duplicate seed receipts, and more direct complete product mutation behavior.

### EXP-CODING-137 - Seed sibling public import symbols from observed import statements

- Date: 2026-06-01
- Goal: Eliminate repeated one-symbol-at-a-time import-surface seeding.
- Evidence: Fresh Claude/Codex Level 6 traces show winning systems read the test import line and satisfy the whole public API surface together. Infring seeded only the first `cannot import name` symbol, reran validation, then discovered the next missing symbol, producing duplicate seed rounds and repair bloat.
- Change: When validation reports a missing Python import from a module, the seed primitive now scans observed local file/test/probe contents for `from module import ...` lines and adds sibling public imported symbols from that module to the same seed pass.
- Primitive intent: Make import-surface seeding module-wide and evidence-driven, not fixture-specific or one-error-at-a-time.
- Expected effect: Fewer seed rounds, smaller first-turn context, and better direct full implementation behavior.

### EXP-CODING-138 - Keep validation-guided repair active after incomplete mutation

- Date: 2026-06-01
- Goal: Fix Level 5 repair loops where a successful product patch occurred but validation still failed, after which the runtime stopped routing to compact repair and allowed read/command churn.
- Evidence: Level 5 smoke patched `calcpack/__init__.py`, auto-validation still failed with `NotImplementedError` in `calcpack/arithmetic.py`, then the next turn emitted reads/commands instead of another product repair.
- Change: Validation-guided compact repair remains active until there is successful validation after the latest mutation. During validation-guided repair, command calls are blocked until the current turn has made a new successful product mutation.
- Primitive intent: Treat failed validation after mutation as a repair lane, independent of whether some earlier product mutation already exists.
- Expected effect: Multi-step repairs should continue mutating the failing owner file instead of drifting into context/validation-only turns.

### EXP-CODING-139 - Replace repair path placeholder with concrete suggested owner path

- Date: 2026-06-01
- Goal: Prevent compact repair turns from copying placeholder path text instead of mutating an observed owner file.
- Evidence: Level 5 smoke failed after multiple `mutation_path_must_be_concrete_observed_path` receipts where the model copied `exact observed owner path from the list below` from the required response shape.
- Change: Validation-guided compact repair packets now compute a concrete suggested repair path from observed source owner paths, preferring the path named in validation traceback/stderr, and put that path directly in the required JSON shape.
- Primitive intent: Make repair packets executable-by-default while still allowing any observed owner path; no fixture-specific path names.
- Expected effect: Weak models should target the failing source file sooner instead of burning turns on placeholder-path guard failures.

### EXP-CODING-140 - Prefer full owner file writes in compact validation repair

- Date: 2026-06-01
- Goal: Reduce fragile patch loops in compact validation-guided repair.
- Evidence: Fresh Claude/Codex Level 6 traces use shell/full-file writes for small owner files. Infring Level 5/7 failures repeatedly emitted fragile `file_patch` calls, placeholder paths, or partial export-only patches after validation identified a seeded stub or incomplete owner file.
- Change: The compact validation repair packet now gives an executable-by-default `file_write` response shape for the concrete suggested owner path, while still allowing `file_patch` only when exact old text is unambiguous.
- Primitive intent: Make compact repair operate at the same granularity as the winning coding systems for small local owner files, without hardcoding task cases.
- Expected effect: Recovery turns should produce complete owner-file replacements instead of brittle one-line patches.

### EXP-CODING-141 - Block placeholder file-write content before mutation

- Date: 2026-06-01
- Goal: Prevent compact repair prompts from corrupting files when weak models copy schema/example content literally.
- Evidence: Reliability run `infring_levels5_7_reliability_full_write_repair_20260601_run1` wrote `complete replacement file content preserving observed public API` into `calcpack/__init__.py`, causing a SyntaxError and repair timeout.
- Change: Added a generic `placeholder_content_guard` for `file_write` calls. Empty/prose/schema placeholder content is rejected before mutation with an actionable receipt and owner path hints.
- Primitive intent: Enforce concrete source content at the tool boundary, mirroring the existing concrete-path guard and avoiding prompt-copy corruption across all coding tasks.
- Expected effect: Repair turns may still fail, but they should fail safely and retry from guard evidence instead of damaging source files with prompt text.

### EXP-CODING-142 - Separate high-fanout seeded budget threshold from staged-controller threshold

- Date: 2026-06-01
- Goal: Improve reliability after module-wide import seeding reduced duplicate seed rounds to one source-owner seed receipt.
- Evidence: Reliability run `infring_levels5_7_reliability_full_write_repair_20260601_run1` showed Level 6 timing out at the normal 75s provider budget on a seeded implementation first turn. The prior high-fanout 90s budget required three source seed receipts, which is stale after EXP-CODING-137 made seeding module-wide and idempotent.
- Change: Added a separate `seed_prepared_high_fanout_min_source_seed_receipts` CD knob and set it to `1`, while leaving the dormant staged-controller threshold unchanged at `3`.
- Primitive intent: Budget seeded implementation turns based on the presence of any prepared source owner seed, without re-enabling or weakening the staged controller.
- Expected effect: Seeded implementation first turns get the intended 90s budget after one module-wide seed, reducing stochastic Level 6/7 first-turn timeout failures.

### EXP-CODING-143 - Route context-before-mutation blockers to mutation recovery

- Date: 2026-06-01
- Goal: Fix Level 7 failures where the model asked for more reads after bootstrap context was already loaded, then the runtime routed recovery into validation-guided repair and timed out.
- Evidence: Smoke `infring_levels5_7_seeded_budget_threshold_smoke_20260601` showed Level 7 first turn parsed only `file_read` calls, both blocked with `preflight_context_already_loaded_first_mutation_required`; the next validation-guided repair prompt timed out at 60s.
- Change: Context-before-mutation blocker reasons now route to mutation-only recovery even when failed validation evidence exists. Validation-guided repair remains for failed mutations/validation repair, not for read-before-mutate control mistakes.
- Primitive intent: Separate action-control recovery from validation repair recovery.
- Expected effect: After blocked context-only calls, the next prompt should use the compact mutation packet and request concrete writes instead of entering a validation repair timeout path.

### EXP-CODING-144 - Keep import-surface seed primitive dormant by default

- Date: 2026-06-01
- Goal: Match the reference-framework Level 7 behavior more closely by avoiding partial seed-only mutations before the model has produced a complete implementation.
- Evidence: Fresh Claude Code and Codex Level 7 traces both passed by validating, inspecting tests/source/probe context, writing complete product files, and validating again. Infring reliability run `infring_levels5_7_reliability_checkpoint_20260601_run1` mutated quickly through import-surface seed receipts, but then timed out in the real implementation/repair turn.
- Change: Set `python_import_surface_seed_mutation_enabled` to `false` in the coding workflow CDs while keeping the seed primitive available behind the CD flag.
- Primitive intent: Treat import-surface seeding as an optional importability primitive, not the default implementation path. Complete implementation should be driven by observed validation, contract, and source context.
- Expected effect: Level 7 should spend its first model turn on complete implementation rather than recovering from partial seed stubs; lower levels should remain governed by the same mutation/validation receipts.

### EXP-CODING-144 verdict - Reverted

- Date: 2026-06-01
- Result: Reverted.
- Evidence: Smoke `infring_levels5_7_import_seed_dormant_smoke_20260601` passed Level 5 in 12.7s, but Level 6 failed with `no_successful_mutation` after 182.2s and Level 7 failed with `no_successful_mutation` after 75.2s.
- Lesson: The import-surface seed primitive is currently necessary scaffolding for Level 6/7 with the control model. The fix is not to remove seeding; it is to improve the post-seed complete implementation lane so it behaves more like Claude/Codex full-file implementation after context is known.

### EXP-CODING-145 - Add post-seed complete implementation packet

- Date: 2026-06-01
- Goal: Preserve useful import-surface seed scaffolding while preventing the next turn from drifting into broad exploration or timeout-prone repair prompts.
- Evidence: Fresh Claude Code and Codex Level 7 traces pass by using observed validation/test/source context to write complete owner files, while Infring failures after seeding often time out before producing the real implementation mutation.
- Change: Added a CD-gated `seed_prepared_complete_implementation_packet_enabled` primitive. When import-surface seed receipts exist and no non-seed product mutation has occurred, the next provider turn receives a compact implementation packet built from observed validation, tests/probe contract, and owner source files. The packet asks for complete product/source owner writes first and lets runtime validate after mutation.
- Primitive intent: Turn seed receipts into a bridge toward complete implementation, not a partial implementation endpoint. This is general to post-seed coding tasks and does not name any eval level or fixture.
- Expected effect: Level 6/7 should retain fast seed scaffolding but emit complete owner-file mutations more reliably, closer to Claude/Codex behavior.

### EXP-CODING-145 verdict - Reverted

- Date: 2026-06-01
- Result: Reverted.
- Evidence: Smoke `infring_levels5_7_post_seed_complete_impl_packet_smoke_20260601` passed Level 5 in 15.1s, but Level 6 failed with `seeded_repair_timeout` after 143.4s and Level 7 failed with `seeded_repair_timeout` after 157.4s.
- Lesson: A compact post-seed packet alone is not enough. It reduced the post-seed prompt size, but the turn still used the broad 3.6k-char system and 8-tool surface, then timed out before tool calls. The actionable next target is a modular post-seed action lane that changes prompt, system/tool surface, and completion criteria together without applying that narrowing globally.

### EXP-CODING-146 - Modular post-seed action lane

- Date: 2026-06-01
- Goal: Fix the Level 6/7 post-seed timeout mode without disabling useful import-surface seed scaffolding or narrowing unrelated lanes.
- Evidence: EXP-CODING-145 showed prompt compaction alone was insufficient because the post-seed turn still used the broad system/tool surface. Fresh Claude Code and Codex traces show successful systems move from context to complete owner-file writes with a constrained action shape.
- Change: Added a CD-gated `post_seed_action_lane_enabled` primitive. When seed receipts exist and no non-seed mutation has occurred, the turn now uses a compact post-seed prompt, a tiny action-only system contract, mutation-recovery tools only, no stream-until-tool-calls mode, omitted thinking flags, and runtime validation after mutation.
- Primitive intent: Make post-seed implementation a scoped action lane rather than a broad prompt tweak. The lane is driven by generic receipt state and observed owner/contract evidence, not eval level names or fixture paths.
- Expected effect: Level 6/7 should emit complete owner-file mutations from the post-seed state instead of timing out in the broad implementation/repair path, while Level 5 remains a lower-level regression canary.

### EXP-CODING-146 verdict - Reverted

- Date: 2026-06-01
- Result: Reverted.
- Evidence: Smoke `infring_levels5_7_post_seed_action_lane_smoke_20260601` failed Level 5 with `seeded_repair_assertion_mismatch` after 221.8s, failed Level 6 with `attribute_missing` after 107.5s, and failed Level 7 with `seeded_repair_timeout` after 192.1s.
- Lesson: The lane successfully narrowed the first post-seed provider request (`system_chars=397`, `tool_count=3`, `prompt_chars=3557-4687`), but the model still produced incomplete semantic repairs and the activation was too broad because it caught Level 5. The next viable direction is not another post-seed prompt lane; it is to improve the seed primitive itself so it can generate minimal complete behavior for simple public API import/NotImplemented cases, or to route simple seed-derived tasks back through the previously reliable validation-guided repair path.

### EXP-CODING-147 - Prefer seeded source owner in validation repair packets

- Date: 2026-06-01
- Goal: Fix post-seed repair turns that keep mutating package/export shims even though the incomplete behavior lives in the seeded source owner file.
- Evidence: Recent Level 7 seeded-repair failures show validation-guided repair writing `warehouse/__init__.py` while `warehouse/items.py` contains the runtime seed stubs and missing behavior. The repair packet previously selected any path mentioned by validation text, which often favors package export files because import tracebacks mention them.
- Change: Validation-guided compact repair packets now collect source paths from `runtime_python_import_surface_seed_source_*` receipts. When failed validation text indicates incomplete behavior, missing attributes, assertions, or `NotImplementedError`, the suggested repair path prefers a seeded source owner that is also in the observed owner-file list.
- Primitive intent: Improve owner selection using receipt provenance and validation failure class signals, without adding a new lane, changing tool permissions, or naming eval fixtures.
- Expected effect: Post-seed compact repairs should target the source file that owns behavior instead of repeatedly rewriting package/export shims.

### EXP-CODING-147 reliability check - Not committed

- Date: 2026-06-01
- Result: Not reliable enough to commit.
- Evidence: Smoke `infring_levels5_7_seed_source_owner_repair_smoke_20260601` passed Levels 5, 6, and 7 once. The follow-up reliability batch `infring_levels5_7_seed_source_owner_repair_reliability_20260601_run1` failed on Level 6 with `seeded_repair_timeout` after 135.6s.
- Failure detail: Level 6 mutated both `orderflow/__init__.py` and `orderflow/attempts.py`, but final validation still failed because `summarize_attempts` returned `{"total": 2}` without the required `by_status` summary.
- Lesson: Source-owner selection is directionally useful but not sufficient. The next primitive target is semantic completion from test/probe contract lines, not path ownership alone.

### EXP-CODING-148 - Preserve semantic assertion blocks in repair contracts

- Date: 2026-06-01
- Goal: Improve semantic completion when tests/probes express required behavior across multi-line assertions or expected output literals.
- Evidence: EXP-CODING-147 reliability failed Level 6 after mutating the right source owner file, because the generated `summarize_attempts` preserved `total` but missed the multi-line expected `by_status` value from the test contract.
- Change: Validation-guided contract extraction now adds compact semantic assertion blocks from test/probe files. It captures assertion/expectation lines with nearby continuation lines and balanced bracket context, capped to bounded blocks.
- Primitive intent: Preserve behavioral expected-output evidence from local contracts without naming a fixture, test level, function, language feature, or output field.
- Expected effect: Repair prompts should carry enough expected-output structure for the model to implement complete behavior instead of satisfying only the first visible assertion fragment.

### EXP-CODING-149 - Keep recovery wall budget after incomplete repair mutation

- Date: 2026-06-01
- Goal: Prevent the native loop from timing out immediately after an incomplete repair mutation when validation still fails and another compact repair turn is warranted.
- Evidence: EXP-CODING-148 reliability passed the first three Levels 5-7 runs, then run 4 failed Level 7 after the first broad turn timed out and a compact repair turn emitted incomplete source changes. The loop exited on the normal 90s wall timeout even though it had post-mutation failed validation evidence and configured recovery wall budget.
- Change: The active wall-timeout selector now uses `recovery_wall_timeout_seconds` when there is a successful non-seed mutation, a failed validation command, and no successful validation after the latest mutation.
- Primitive intent: Treat failed validation after a real mutation as active recovery work, independent of whether the recovery flag is already pending at the top of the loop.
- Expected effect: Incomplete semantic repairs get one more bounded repair opportunity instead of being truncated by the normal wall timeout.

### EXP-CODING-150 - Align mutation-only recovery timeout with compact repair

- Date: 2026-06-01
- Goal: Reduce failures where context-before-mutation blockers correctly route to mutation-only recovery, but the recovery turn times out before emitting a mutation.
- Evidence: `infring_levels5_7_semantic_blocks_recovery_wall_reliability_20260601_run4` failed Level 6 after the first post-seed provider turn emitted blocked read calls. The next mutation-only recovery turn had a compact prompt and 3 tools, but timed out at 45s before producing source mutations.
- Change: Raised `mutation_only_recovery_provider_timeout_seconds` from 45s to 60s in the coding workflow CDs, matching the existing validation-guided compact repair timeout.
- Primitive intent: Give both compact recovery lanes comparable bounded provider budgets without changing routing, prompts, tool permissions, or task-specific logic.
- Expected effect: Context-blocker recovery should have enough time to emit concrete owner-file mutations instead of failing as a short recovery timeout.

### EXP-CODING-148/149/150 reliability verdict - Passed 15/15

- Date: 2026-06-01
- Result: Passed 5x Levels 5-7 reliability gate.
- Evidence: `infring_levels5_7_semantic_blocks_recovery_timeout60_reliability_20260601_run1` through `run5` passed all 15 attempts.
- Timings: Level 5 average wall time was about 49.8s, Level 6 about 64.7s, and Level 7 about 76.8s. One Level 5 outlier reached 101.1s.
- Interpretation: The current patch stack is reliability-positive for Levels 5-7, including the previous run-4 breakpoint. Speed remains uneven and should be treated as the next optimization lane rather than blocking this correctness improvement.

### EXP-CODING-151 - Prefer edit-only first mutation batches for speed

- Date: 2026-06-01
- Goal: Reduce first-turn provider latency by avoiding model-generated validation/probe command calls in the same batch as source mutations.
- Evidence: The 15/15 reliability run showed the slow tail comes from one broad post-seed implementation turn with `system_chars=3625`, `tool_count=8`, and prompt sizes from about 9.5k to 13.4k. Successful batches often included source writes plus validation/probe commands even though the runtime already validates after successful mutation.
- Change: Updated bounded direct edit prompt contracts to ask for source/test `file_write` or `file_patch` mutations first and let runtime validation/probe checks run after successful mutation. Validation/probe `command_run` calls are discouraged in the first mutation batch unless the user explicitly requested a different command.
- Primitive intent: Keep verification owned by the runtime after mutation rather than making the model spend first-turn tokens generating command calls.
- Expected effect: Lower output size and first-turn latency while preserving receipt-backed validation through runtime checks.

### EXP-CODING-151 verdict - Reverted

- Date: 2026-06-01
- Result: Reverted.
- Evidence: Smoke `infring_levels5_7_edit_only_first_mutation_smoke_20260601` made Level 6 pass in 27.1s and Level 7 pass in 22.5s, but Level 5 failed with `runtime_timeout` after 260.0s.
- Failure detail: Level 5 entered a multi-turn loop after an incomplete mutation produced `return a * b` with undefined variables. The edit-only prompt removed the model-generated validation/probe command shape that had been helping the runtime close or repair simple tasks promptly.
- Lesson: Edit-only first batches can be fast for harder Levels 6/7, but they destabilize simpler public API repair. The next speed primitive should be lane/classification-based or runtime-controlled, not a blanket prompt instruction.

### EXP-CODING-152 - Edit-only first batch for seeded multi-requirement tasks

- Date: 2026-06-02
- Goal: Recover the Level 6/7 speed win from EXP-CODING-151 without applying edit-only first batches to simpler public API repair tasks.
- Evidence: EXP-CODING-151 made Level 6 and Level 7 fast, but regressed Level 5. The successful fast cases were seeded multi-requirement implementation slices, while the failed Level 5 case was a simpler public API repair that benefited from model-emitted validation/probe command calls.
- Change: Added a CD-gated `seeded_multi_requirement_edit_only_first_batch_enabled` primitive. When import-surface seed source receipts exist and the task is classified as multi-requirement validation work, the bootstrap rule asks for source/export file mutations only and leaves validation to runtime. Other bounded direct edit paths keep the reliability-proven edit-plus-validation contract.
- Primitive intent: Make the speed optimization depend on generic runtime state and task shape, not eval level numbers or fixture names.
- Expected effect: Level 6/7 should keep much of the edit-only speed improvement while Level 5 remains on the stable path.

### EXP-CODING-153 - Seed-symbol evidence classifier for edit-only first mutation

- Date: 2026-06-02
- Goal: Make the Level 6/7 speed path depend on receipt evidence instead of brittle prompt-shape classification.
- Evidence: EXP-CODING-152 passed a one-run Level 5/6/7 smoke but trace output suggested the edit-only classifier did not reliably activate; tool batches still included validation/probe commands.
- Change: Import-surface seed receipts now annotate seed module, symbols, symbol count, and receipt kind in the receipt result. The edit-only first mutation path activates only when a successful seeded source owner has at least the CD-configured minimum number of public symbols.
- Primitive intent: Treat seed receipts as reusable runtime planning evidence. One-symbol repair stays on the normal validation-aware path; multi-symbol implementation slices can use the leaner first mutation lane.
- Expected effect: Preserve Level 5 stability while reducing Level 6/7 first-turn command generation overhead.
- Result: 1x Level 5/6/7 trace smoke passed, but threshold `2` activated the edit-only path for Level 5 as well as Levels 6/7. Trace evidence: Level 5 max seed symbol count was 2, Level 6 was 3, and Level 7 was 4.
- Verdict: threshold too broad; raise default/CD threshold to 3.

### EXP-CODING-154 - Raise seeded edit-only threshold to larger implementation surfaces

- Date: 2026-06-02
- Goal: Keep simple import-surface repair on the stable validation-aware first batch while preserving the fast lean first mutation path for larger seeded implementation slices.
- Evidence: EXP-CODING-153 showed a clean receipt-evidence split: simple repair max seed symbols was 2, while larger implementation slices were 3+.
- Change: Raised `seeded_multi_requirement_edit_only_min_seed_symbols` from 2 to 3 in the coding workflow CDs and default helper.
- Primitive intent: Classify by reusable seed-surface size, not eval level, fixture name, file path, or prompt wording.
- Expected effect: Level 5 should report `seeded_multi_requirement_edit_only_first_batch=false`; Levels 6/7 should remain `true`.
- Result: 3-run Levels 5-7 mini-gate passed 9/9. Route split was stable in every run: Level 5 reported edit-only `false` with max/min `2/3`, Level 6 reported `true` with `3/3`, and Level 7 reported `true` with `4/3`.
- Timing: Mini-gate averages were about Level 5 `33.0s`, Level 6 `29.3s`, and Level 7 `29.6s`; first-mutation timing remained provider-variable but correctness and route isolation stayed stable.
- Verdict: keep. This is a primitive-positive speed/routing improvement because it improves higher implementation slices without applying the lean path to the simpler repair lane.
