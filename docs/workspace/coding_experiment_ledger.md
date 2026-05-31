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

Status: `patched_pending_measurement`

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

Status: `patched_pending_measurement`

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

Status: `patched_pending_measurement`

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

Status: `patched_pending_measurement`

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

Status: `patched_pending_measurement`

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

Status: `patched_pending_measurement`

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
