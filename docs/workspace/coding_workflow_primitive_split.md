# Coding Workflow Primitive Split

Status: active foundation split  
Owner lane: coding workflow / tooling  
Parent doctrine: `docs/workspace/primitive_workflow_composition_doctrine.md`

## Why this exists

The `coding_project_operator` workflow has grown large enough that routing,
bootstrap, memory, mutation, validation, repair, closure, and final synthesis
were being maintained as one tangled behavior surface. Level 8 testing exposed
the risk: each patch moved the bottleneck but also made the workflow harder to
reason about.

The repair is to split the coding operator into primitive workflows with strict
input and output artifacts. Higher-level coding workflows should compose these
parts rather than embedding all behavior into one prompt or runtime lane.

## Active primitive tranche

| Workflow ID | Level | Owns | Must not own |
| --- | ---: | --- | --- |
| `coding_execution_spine_v1` | 0 | Deterministic coding state transitions from normalized evidence. | Provider calls, file/command execution, language-specific symbol detection, eval fixture scoring, final user projection. |
| `first_mutation_artifact_lane_v1` | 0 | First mutation action shape from supplied context for low-complexity existing-project edits. | Discovery, validation, repair, checkpoint/memory closure, final user projection. |
| `local_project_bootstrap` | 0 | Bounded file discovery/read context for current local project files. | Mutation, validation, memory writes, architecture decisions. |
| `checkpoint_memory_bootstrap` | 0 | Bounded checkpoint-memory retrieval as advisory context. | Project mutation, checkpoint closure, treating memory as authoritative. |
| `first_mutation_executor` | 1 | First bounded source/test/operator mutation from supplied context, delegating actual writes/patches to `file_mutation_executor`. | Discovery, validation, checkpoint receipt, memory write, final success claims. |

The active CDs live under `orchestration/src/control_plane/workflows/lab/primitives/coding_runtime/` and are registered as lab, non-runtime-selectable workflows. `coding_project_operator` remains the official runtime-selectable composer.

These are now declared as official workflow CDs under
`orchestration/src/control_plane/workflows/official/`.

## Composite owner

`coding_project_operator` remains the composite owner. It now declares
`coding_stage_pipeline_contract_v1` and composes the active primitives before
the broader `local_coding_program_builder` responsibility.

Current level remains `4` because this is still the high-level coding operator;
adding Level 0 primitives does not reduce its orchestration responsibility.

## Stage boundaries

### `local_project_bootstrap`

Input:

- `project_root`
- `task_goal`
- `selection_policy`
- optional known relevant/excluded paths and budget limits

Output:

- `local_project_bootstrap_artifact_v1`
- file read/list receipts
- selected files
- excluded files
- bounded context packet
- validation command candidates

Invariant:

Current local files are authoritative.

### `checkpoint_memory_bootstrap`

Input:

- `project_root`
- `memory_db_path`
- `memory_cli_command_pattern`
- optional resume token / prior memory row id

Output:

- `checkpoint_memory_bootstrap_artifact_v1`
- command receipts
- memory context packet or unavailable artifact

Invariant:

Memory is a hint only. It never overrides current files.

### `first_mutation_executor`

Input:

- `project_root`
- `task_contract`
- `local_project_bootstrap_artifact_v1`
- optional `checkpoint_memory_bootstrap_artifact_v1`
- mutation scope and target path constraints

Output:

- `first_mutation_executor_artifact_v1`
- mutation receipts
- changed files
- implemented generic capability categories
- blocked reason when mutation is unsafe

Invariant:

This primitive cannot read more files, run validation, write checkpoint
receipts, write memory rows, or claim the whole task is complete.

## Placeholder stages to extract next

The master workflow declares placeholders for stages we should split next:

- `vertical_slice_completion_gate`
- `validation_runner`
- `repair_loop`
- `checkpoint_closure_writer`
- `final_response_synthesizer`

Those placeholders are not active child workflows yet. They document the next
composition seams and prevent us from hiding that behavior inside the first
three primitives.

## Runtime migration rule

Runtime code may temporarily implement stage behavior directly while we migrate,
but new runtime behavior must map to one of these stage IDs or be documented as
migration debt.

As of the semi-aggressive rebuild, controller authority belongs to
`coding_execution_spine_v1`. Legacy coding runtime controllers may remain during
migration only if they act as evidence producers, evidence adapters, or policy
inputs. They must not independently decide to continue, repair, finalize, seed,
or timeout around the spine.

Initial routing is intentionally limited to low-complexity lanes:
`new_file_fast_path`, `micro_direct_mutation`, `existing_project_patch`,
`bounded_existing_project_edit`, and `implementation_slice`. Higher project,
operator, checkpoint, memory, and multi-file slice lanes remain on the legacy
controller until Level 1-3 behavior is monotonic.

The first mutation artifact lane is narrower than the spine routing surface:
it is initially enabled only for `existing_project_patch`. It must emit mutation
receipts into `coding_execution_spine_v1`; it cannot decide validation, repair,
or final success itself.

If a Level 8 patch needs more than first mutation, it should usually become one
of:

- `vertical_slice_completion_gate`
- `checkpoint_closure_writer`
- `repair_loop`

It should not be added to `first_mutation_executor`.

## Success criteria for this split

- Simple coding tasks can skip checkpoint/memory stages.
- Existing-project tasks get local context before mutation.
- Checkpointed project tasks get memory context without treating memory as truth.
- First mutation is fast and receipt-backed.
- Later stages can reject incomplete slices without bloating the mutation
  primitive.
- Lower coding levels do not regress when checkpointed project policy changes.
