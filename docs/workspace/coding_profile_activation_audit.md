# Coding Profile Activation Audit

Date: 2026-05-29

Status: active corrective model

Purpose: restore the unified coding model after lower-level eval regressions showed
that valid higher-level primitives were leaking into lower-level execution.

## Finding

The unified model is conceptually sound, but runtime activation is corrupted.

The failure pattern is:

```text
higher-level primitive added for Level N
-> primitive lands in shared native loop or global prompt policy
-> lower-level task encounters the primitive
-> lower-level task stalls, slows, or fails
```

This violates the primitive-first doctrine. A higher-level primitive may compose
lower primitives, but it must not globally alter, slow, or narrow them.

## Profiles

Profiles classify runtime execution shape. They are not eval levels.

The test gauntlet may name scenarios as Level 1, Level 2, and so on, but the
runtime must not select behavior from those labels. It must classify the task
the way successful coding agents do: direct write, bounded existing-project
edit, validation-guided repair, multi-file slice, checkpointed project
operation, or long-horizon project work.

This distinction is mandatory:

```text
eval level = measurement fixture
runtime profile = reusable execution shape
```

| Profile | Name | Purpose |
|---:|---|---|
| 0 | `micro_direct_mutation` | Fully specified create/write/patch with no discovery or validation. |
| 1 | `deterministic_local_action` | Deterministic local action sequence without live provider reasoning. |
| 2 | `bounded_existing_project_edit` | Small existing-project edit with bounded context and owner-source mutation. |
| 3 | `validated_repair_edit` | Profile 2 plus validation, failure diagnosis, and bounded repair. |
| 4 | `project_slice` | Multi-file source/test/operator slice with staged completion evidence. |
| 5 | `checkpointed_project_operator` | Project-operator slice with checkpoint/context memory and closure artifacts. |
| 6 | `long_horizon_project` | Multi-slice roadmap, architecture, memory retrieval, and resumable operation. |

## Activation matrix

| Primitive or guard | Profile min | May block lower profiles | Activation boundary |
|---|---:|---|---|
| `coding_task_contract` | 0 | no | Always classify; never mutate or block by itself. |
| `implementation_entry_gate` | 0 | yes, only success/finalization | Mutation-required tasks cannot finalize without mutation receipts. |
| `file_mutation_executor` | 0 | no | Only executes requested file write/patch and emits receipts. |
| `receipt_journal` | 0 | no | Records receipts; must remain cheap. |
| `final_receipt_synthesis` | 0 | no | Summarizes receipts; no hidden success claims. |
| `context_pack_builder` | 2 | no | Only when existing project context is required. |
| `bounded_direct_edit_lane` | 2 | no | Bounded file context, small edit, no broad stage controller. |
| `public_interface_verifier` | 2 | no for profiles 0-1 | Only when prompt, tests, or local probe define public surface requirements. |
| `source_owner_resolution` | 2 | no for profiles 0-1 | Only when an import/probe maps a public symbol to an owner file. |
| `preserved_api_guard` | 2 | no for profiles 0-1 | Only when preserving existing public behavior is part of the task or owner-source evidence. |
| `validation_runner` | 3 | no for profiles 0-2 unless explicitly requested | Only after mutation, except explicit pre-mutation validation bootstrap. |
| `failure_diagnosis` | 3 | no for profiles 0-2 | Only from failed receipts. |
| `seeded_repair_controller` | 3 | no for profiles 0-2 | Only after concrete failed validation/import-surface evidence or a deterministic seed receipt; converts evidence into bounded owner/export mutations, then validation. |
| `bounded_repair_loop` | 3 | no for profiles 0-2 | Only after concrete failed tool/validation/interface evidence. |
| `tool_retry_reflection` | 3 | no for profiles 0-2 | Tool schema repair only; must not broaden task scope. |
| `controlled_shell_edit_batch` | 4 | no for profiles 0-3 by default | Only for multi-file project slices or explicit profile opt-in. |
| `completion_evidence_gate` | 4 | no for profiles 0-3 | Only for multi-requirement/project-slice tasks. |
| `staged_execution_controller` | 4 | no for profiles 0-3 | Only project slices with source/test/operator stage requirements. |
| `semantic_closeout_probe` | 4 | no for profiles 0-3 | Only operator/public behavior closeout after validation. |
| `capability_evidence_gate` | 5 | no for profiles 0-4 | Only durable/checkpoint/operator tasks with declared capability categories. |
| `checkpoint_handoff` | 5 | no for profiles 0-4 | Closure only after mutation and validation evidence. |
| `checkpoint_memory_bootstrap` | 5 | no for profiles 0-4 | Context only when prompt/session declares checkpoint/resume memory. |
| `checkpoint_memory_write` | 5 | no for profiles 0-4 | Closure only after requested checkpoint evidence exists. |
| `architecture_bootstrap` | 6 | no for profiles 0-5 | Only when project initialization or architecture decisions are required. |
| `long_horizon_planner` | 6 | no for profiles 0-5 | Only roadmap/multi-slice tasks; never before lower-profile edit execution. |

## Current runtime violations

These are model violations observed during the Level 2 regression investigation:

| Violation | Why it is wrong | Required correction |
|---|---|---|
| `model_manifest_planner` intercepted bounded existing-project tasks. | Profile 4/5-style manifest planning blocked Profile 2 mutation. | Planner activation must be profile/lane-scoped. |
| `capability_evidence_gate` required durable/operator evidence for Level 2 work. | Profile 5 capability categories leaked into Profile 2. | Capability evidence gate must activate only for checkpointed/operator profiles. |
| `controlled_shell_edit_batch` shaped low-level repair behavior. | Shell batch is a Profile 4+ optimization; Profile 2 should prefer native file tools. | Shell edit batch must be profile-gated and dormant for lower profiles by default. |
| `staged_execution_controller` blocked simple owner-source repair with test/export sequencing. | Project-slice stage order leaked into bounded edit. | Stage controller must require Profile 4+. |
| `completion_evidence` and owner-source repair prompts accumulated contradictory constraints. | The model received too many active controllers for a small task. | Only the earliest active primitive may issue a blocking repair instruction. |
| Tool schema aliases such as `target_file_path` and `patch_content` were unsupported. | Tooling primitive was too narrow for common coding-agent edit forms. | Tool compatibility belongs in `file_mutation_executor`, not workflow prompts. |
| Final artifact sometimes reported zero tool calls despite internal journal mutations. | Finalization/accounting hid partial progress. | `receipt_journal` and `final_receipt_synthesis` must flush partial progress before timeout. |

## Runtime task-shape classifier

The classifier should mirror the practical distinctions used by Codex, Claude
Code, Aider, and SWE-agent, not the eval ladder:

| Shape | Runtime lane | Primary question | Controller owner |
|---|---|---|---|
| `direct_create_or_write` | `new_file_fast_path` | Can this be safely written without discovery? | `file_mutation_executor` |
| `bounded_existing_project_edit` | `existing_project_patch` | Which existing files are in edit scope? | `bounded_direct_edit_or_patch_artifact` |
| `validation_guided_repair` | `validation_repair` | What failed, and which owner/export surface must mutate? | `validation_repair_controller` |
| `multi_file_project_slice` | `multi_file_slice` | What source/test/operator slice satisfies the user goal? | `staged_execution_controller` |
| `checkpointed_project_operation` | `long_run_project_operator` | What context, checkpoint, memory, and closure artifacts are required? | `checkpointed_project_operator` |
| `general_native_tool_task` | `general_native_tool_task` | What minimal local context is needed before mutation or blocker? | `native_tool_loop_with_structured_blockers` |

Forbidden classifier inputs:

- eval level number
- fixture name
- test case id
- expected marker string
- one-off prompt phrase from a specific harness task

Required classifier outputs:

- selected runtime lane
- selected profile
- owning controller
- required context/discovery flag
- validation requirement flag
- escalation reason when choosing a higher profile over a lower one

## Current model amendment: repair must be evidence-to-mutation, not retry-first

The EXP-CODING-052 compact first-mutation retry was rejected because it did not
improve the dominant failure classes:

```text
Level 2 -> no_successful_mutation
Level 5+ -> seeded_repair_timeout / import_surface_missing
```

This means the next primitive must not be another generic retry wrapper. The
unified model now treats repair as a Profile 3 evidence-to-mutation controller:

```text
failed validation/import evidence
-> diagnose owner/export gap
-> emit bounded source/export mutation
-> run validation/probe
-> close with receipt-backed success or structured blocker
```

Activation constraints:

- It must activate only after concrete failed validation, failed import, failed
  probe, or deterministic seed evidence exists.
- It must not activate for Profile 0-2 direct mutation tasks unless the prompt
  explicitly requests validation or the runtime has already produced failed
  validation/import evidence.
- It must own no eval-level names, fixture paths, symbol names, or task-specific
  shortcuts.
- It must preserve the smaller primitive stack: file mutation, receipt journal,
  validation runner, failure diagnosis, final synthesis.
- It must emit a structured blocker quickly when evidence is insufficient
  instead of widening the context or entering an opaque provider stall.

Patch rule:

Any implementation of this controller must update the experiment ledger with
its declared profile, activation evidence, rollback condition, and lower-profile
smoke results. A Profile 3 repair patch is invalid if Profile 1 or Profile 2
regresses.

## Enforcement rule

Every coding runtime change must declare the highest profile it touches.

Required smoke before accepting the patch:

```text
for each profile P lower than or equal to touched_profile:
  run at least one smoke attempt for P
  fail the patch if any lower profile regresses
```

Promotion requires five attempts for affected and lower profiles.

Reliability claims require twenty attempts.

If a Profile 5 or Profile 6 change breaks Profile 1 or Profile 2, the higher
profile change is invalid unless the patch fixes a shared primitive in a way
that improves or preserves lower profiles.

## Next implementation move

Do not continue Level 2 by adding more local guards to the shared loop.

Patch activation boundaries first:

1. Add a controller-visible selected profile to every native coding run.
2. Disable Profile 4+ primitives when selected profile is below 4.
3. Disable Profile 5+ primitives when selected profile is below 5.
4. Ensure lower-profile finalization can emit receipt-backed partial progress.
5. Rerun the smoke ladder from Profile 0 upward before returning to Level 8.
