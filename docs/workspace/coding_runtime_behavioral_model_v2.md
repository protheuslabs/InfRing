# Coding Runtime Behavioral Model v2

Date: 2026-05-20

Status: active design model

## Purpose

This model exists because the first assimilation pass captured many correct
coding-agent behaviors, but did not capture enough runtime topology.

The failure signal was clear:

- Reference Level 1 mutation paths complete in roughly `49 ms` to `195 ms`.
- Native Infring Level 1 passed reliability checks, but took roughly `21 s` to
  `123 s` per run.

That is not a small tuning gap. It is a runtime-shape mismatch.

The doctrine for this model:

`Behavior is not assimilated until Infring can reproduce the runtime path that
made the behavior cheap, reliable, and observable.`

## Core distinction

Surface behavior:

`The system can create a file.`

Runtime behavior:

`The system routed a fully specified file-creation request into the cheapest
safe mutation lane, produced a mutation receipt, emitted a trace explaining why
that lane fired, and avoided unnecessary model-loop cost.`

The second statement is the target.

## Reference trace basis

Tracked summary:

`docs/workspace/coding_level1_reference_trace_observations.md`

Ignored trace artifact:

`references/coding-agent-systems/level1_reference_runtime_trace_observations.json`

Level 1 task:

`Create one file named hello.py containing a greet(name) function.`

Observed runtime surfaces:

| System | Runtime surface observed | Level 1 mutation | Latency | Primary lesson |
| --- | --- | ---: | ---: | --- |
| mini-swe-agent | provider-free agent loop | yes | 195 ms | Small deterministic loop is easy to trace and avoids live provider cost. |
| swe-agent | direct editor primitive | yes | 49 ms | Fully specified edits can route directly to a mutation tool. |
| swe-rex | metadata/runtime surface | no | 33 ms | Useful substrate evidence, but not a local writer in this probe. |
| aider | metadata/runtime surface | no | 199 ms | Pattern source, not direct Level 1 proof in this checkout. |
| openhands | metadata/runtime surface | no | 38 ms | Pattern source, not direct Level 1 proof in this checkout. |
| cline | editor/tool surface | no | 32 ms | Extension/editor primitives should inform tool routing. |
| continue | editor/context surface | no | 57 ms | Context/tool registry patterns should inform routing. |
| goose | workspace/runtime surface | no | 0 ms | Rust workspace surface, not direct mutation proof in this probe. |
| roo-code | editor/tool surface | no | 81 ms | Tool registry patterns should inform routing. |
| forgecode | benchmark/runtime surface | no | 83 ms | Strong source for command execution, validation, retry, and loop guards. |

## Runtime topology tiers

Coding tasks should not all enter the same execution path.

### Tier 0: direct mutation primitive

Use when the task contract proves the change is fully specified and local.

Examples:

- create a new file with supplied or trivially derivable content
- replace an exact known string in a known file
- append a small declared snippet to a known file

Required properties:

- no repo discovery needed
- no architecture decision needed
- no ambiguous target
- no unsafe overwrite
- no hidden dependency on model reasoning
- mutation permission is granted

Runtime shape:

`task contract -> direct mutation executor -> receipt -> final synthesis`

This is the lane that should compete with SWE-agent's direct editor path.

### Tier 1: deterministic local agent loop

Use when a tiny local action loop is useful, but a live provider is not.

Examples:

- scripted edit plus command observation
- generated fixture operation
- reproducible eval action sequence

Runtime shape:

`task contract -> deterministic planner/action source -> local environment -> receipts -> trajectory`

This is the lane that should compete with mini-swe-agent's deterministic loop.

### Tier 2: model-mediated tool loop

Use when the model must infer files, content, or edit shape, but the task is
still bounded.

Examples:

- modify an existing project file based on natural language
- locate the relevant file before editing
- implement a small feature across a few files

Runtime shape:

`task contract -> context assessment -> model tool loop -> file tools -> receipts -> synthesis`

### Tier 2a: small scoped edit artifact

Use when the model must infer the concrete edit, but the runtime already has a
small, confident file context.

Examples:

- patch one or a few known existing source/test files
- update a small public API plus its directly related test
- apply a bounded local behavior change where broad discovery is unnecessary

Required properties:

- selected file context is already known
- selected file count is within the profile budget
- selected context bytes are within the profile budget
- allowed edit paths are explicitly bounded
- no broad architecture decision is needed
- no model-side claim of mutation is trusted
- phase timing is emitted so speed regressions can be assigned to context
  selection, model artifact generation, deterministic patching, validation, or
  runtime wrapper overhead
- model routing is intentionally omitted for now; this profile uses the caller's
  selected model so eval comparisons remain honest

Runtime shape:

`task contract -> selected file context -> edit artifact -> deterministic safe patch -> receipts -> validation/synthesis`

Reference basis:

- Aider's fast existing-project path succeeds when target files are already in
  chat, then emits compact SEARCH/REPLACE edits for deterministic application.
- Infring keeps the portable mechanic, but safe file patch tooling remains the
  authority for mutation receipts.

This is a Tier 2 profile, not a replacement for Tier 0 direct mutation or the
general Tier 2 model tool loop.

Optimization order:

1. Shrink selected context without hiding required evidence.
2. Keep the artifact grammar singular and deterministic.
3. Measure phase latency before changing behavior.
4. Cache/compile runtime and workflow startup paths when timing proves wrapper
   overhead dominates.
5. Consider model routing only after same-model comparisons are stable.

Fallback ladder:

1. Try the small scoped SEARCH/REPLACE artifact profile.
2. If that profile times out or returns an invalid artifact, retry the general
   bounded patch artifact profile with the same already-selected context.
3. Enter the full native tool loop only after both artifact profiles fail.

### Tier 3: validation and repair loop

Use when validation is requested, expected, or necessary for confidence.

Examples:

- run tests after mutation
- repair compiler or test failure
- stop with structured blocker after bounded attempts

Runtime shape:

`mutation receipts -> validation command -> failure diagnosis -> bounded repair -> final receipt synthesis`

ForgeCode is the primary reference for this tier because its benchmark artifacts
emphasize command execution, validation callbacks, retry reflection, and loop
guards.

### Tier 4: project operator loop

Use when the task is a larger project slice.

Examples:

- new project initialization
- multi-file feature across architecture boundaries
- persistent project context capture
- checkpointed long-horizon work

Runtime shape:

`project goal -> architecture/context gate -> slice plan -> repeated lower-tier execution -> checkpoint handoff`

This tier must be built from lower tiers. It must not special-case lower-tier
behavior.

## Execution shape gate

Every coding task should first produce an execution-shape verdict.

Required verdict fields:

- `lane`: one of `direct_mutation`, `deterministic_local_loop`,
  `small_scoped_edit_artifact`, `model_tool_loop`, `validation_repair_loop`,
  `project_operator_loop`, `structured_blocker`
- `confidence`: numeric confidence that the selected lane is appropriate
- `requires_model`: boolean
- `requires_discovery`: boolean
- `requires_validation`: boolean
- `target_scope`: `new_file`, `known_file`, `existing_project`, `unknown`
- `mutation_safety`: `safe_new`, `safe_patch`, `unsafe_overwrite`,
  `ambiguous`, `not_applicable`
- `escalation_reason`: required when choosing a higher tier over a lower tier

The gate may not mention eval levels, fixture names, or hardcoded task examples.

## Direct mutation primitive contract

Primitive name:

`single_mutation_execution`

Layer intent:

General local coding primitive. It is not an eval shortcut.

Inputs:

- `task_id`
- `workspace_root`
- `mutation_kind`: `create_file`, `overwrite_file`, `patch_file`,
  `append_file`
- `target_path`
- `content` or `patch`
- `permission_receipt`
- `safety_policy`
- `trace_parent`

Outputs:

- `mutation_receipt`
- `changed_file_summary`
- `trace_events`
- `structured_failure`, if refused

Refusal conditions:

- target path escapes workspace or allowed root
- requested overwrite is unsafe or undeclared
- content is missing for create/overwrite
- exact patch target cannot be proven
- permissions do not include the required mutation operation
- validation was requested but no validation lane is available

Non-goals:

- infer architecture
- discover project context
- invent large content from vague prompts
- run long repair loops
- satisfy eval-level-specific cases

## Latency anatomy

The runtime trace must expose latency by phase, not only total duration.

Minimum phase fields:

- `workflow_load_ms`
- `execution_shape_gate_ms`
- `provider_start_ms`
- `model_call_ms`
- `tool_dispatch_ms`
- `mutation_ms`
- `validation_ms`
- `repair_ms`
- `final_synthesis_ms`
- `total_ms`

Expected Level 1 shape:

Direct mutation:

`provider_start_ms = 0`

`model_call_ms = 0`

`mutation_ms` dominates total runtime.

Current native path:

`provider_start_ms + model_call_ms + workflow preamble` dominate total runtime.

That is the mismatch to eliminate.

## Trace semantics

Every coding run should record both task outcome and lane outcome.

Task outcome asks:

`Was the requested software change completed?`

Lane outcome asks:

`Did the runtime choose the cheapest safe lane, and if not, why not?`

Required trace events:

- `coding.task_contract.created`
- `coding.execution_shape.selected`
- `coding.execution_shape.escalated`, if applicable
- `coding.mutation.requested`
- `coding.mutation.applied`
- `coding.validation.requested`, if applicable
- `coding.validation.completed`, if applicable
- `coding.repair.attempted`, if applicable
- `coding.final_synthesis.completed`

Required receipt linkage:

- final answer must reference mutation receipts for mutation tasks
- validation claims must reference validation receipts
- blocker responses must include the lane, failed precondition, and next needed
  input

## Anti-hardcoding rule

The implementation may not special-case:

- eval level names
- `hello.py`
- `greet`
- specific fixture paths
- benchmark IDs
- canned prompts

Allowed specific cases:

- eval fixtures
- test expectations
- docs examples

Runtime behavior must be expressed as:

- primitive contracts
- lane gates
- safety policies
- tool capabilities
- workflow CD configuration
- generic validators

## Integration gap diagnosis

Current Infring native Level 1 success proved:

- native agent can use file tools
- mutation receipts can be produced
- the workflow can complete end-to-end

It did not prove:

- Level 1 is routed to the cheapest safe lane
- trivial mutation avoids provider startup
- latency is competitive with reference systems
- the execution topology matches assimilated systems

Therefore the next implementation should not add another prompt patch. It
should add the missing topology.

## Implementation target

The next implementation wave should build:

1. `execution_shape_gate`
2. `single_mutation_execution`
3. shared trace fields for lane and phase latency
4. workflow wiring that tries lower tiers before higher tiers
5. a Level 1 timing eval that fails if direct-mutation-eligible work enters the
   provider loop without an escalation reason

## Promotion criteria

The primitive foundation should not be considered restored until:

- Level 1 direct-mutation-eligible tasks complete without provider startup
- Level 1 remains reliable across varied file names and small content shapes
- the final answer still includes receipts
- lane traces explain why the fast path fired
- lower-level gates remain green when higher tiers are modified

## Open questions

- Should direct mutation live in orchestration as a workflow primitive, or in a
  kernel-adjacent local execution service with orchestration as coordinator?
- Should final synthesis for direct mutations be template-based, model-based, or
  configurable by workflow CD?
- Should deterministic local loops be first-class production lanes or eval-only
  scaffolding?
- Which reference system should be the primary parity target for Tier 3 repair:
  ForgeCode benchmark runtime, SWE-agent edit loop, or a hybrid?
