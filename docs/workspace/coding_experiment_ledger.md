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
