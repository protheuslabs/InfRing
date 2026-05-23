# Coding Runtime Model v1

Status: draft runtime model
Behavioral assimilation package: `docs/workspace/coding_behavioral_assimilation_v1.md`
Implementation spec: `docs/workspace/coding_runtime_implementation_spec_v1.md`

Purpose: define a cohesive coding-agent runtime model from reference-system mechanics before rebuilding Infring coding as a primitive-first execution spine.

## Thesis

The coding workflow must be a runtime state machine, not a large prompt with many repair clauses.

The model owns:

- state
- allowed next actions
- tool execution
- observation capture
- receipt/evidence update
- repair or finalization gates

The model does not own:

- benchmark-specific fixture logic
- language-specific behavior without an adapter boundary
- hidden prompt patches for specific task shapes
- final success claims without receipts

## Core loop

```text
task_contract
-> context_state
-> executable_manifest_or_patch_artifact
-> allowed_action_decision
-> tool_execution
-> observation_receipt
-> progress_gate
-> repair_or_finalize
```

The model repeats until terminal state:

- `success`
- `partial_blocked`
- `failed_budget`
- `failed_tooling`
- `needs_user_input`

## Canonical coding execution spine

The default coding runtime should optimize for fast, receipt-backed progress before heavyweight reasoning.

```text
fast_task_contract
-> compact_executable_manifest
-> deterministic_native_tool_execution
-> receipt_and_evidence_capture
-> validation_or_semantic_completion_check
-> bounded_strong_repair_if_evidence_requires_it
-> final_receipt_synthesis
```

This spine is deliberately primitive-first:

- the fast path handles simple/context-free tasks without existing-project discovery
- the normal path builds enough local context before mutation
- native tools own file writes, patches, command execution, and receipts
- validation and semantic probes are evidence sources, not success substitutes
- stronger models are used after concrete failure evidence exists, not as a blanket first step

Strong-model escalation is allowed when evidence shows:

- the manifest is malformed, empty, or non-executable
- no mutation receipt exists for a mutation-required task
- a file write or patch failed
- validation failed after mutation
- the requested public interface is missing after mutation
- the runtime detects repeated no-progress behavior
- the task contract has user-owned ambiguity that cannot be resolved locally

Strong-model escalation should produce a smaller corrected manifest, repair plan, or user-facing blocker. It must not bypass deterministic tool execution, receipt journaling, validation gates, or final synthesis rules.

## Runtime state

### `task_contract`

Defines what the task requires.

Fields:

- `task_kind`: create_file | existing_project_patch | debug_repair | refactor | project_slice
- `requires_context`: boolean
- `requires_mutation`: boolean
- `requires_tests`: boolean
- `requires_validation`: boolean
- `public_surface_requirements`: functions, classes, attrs, modules, constructors, return fields
- `allowed_write_scope`: roots and paths
- `stop_conditions`: success, blocker, budget, missing user info

### `context_state`

Tracks whether the runtime has enough local context to mutate safely.

Allowed actions:

- `file_list`
- `file_stat`
- `file_read`
- `file_read_many`

Exit conditions:

- enough context selected
- create-file fast path selected
- blocked by missing local path or permissions

### `executable_manifest_or_patch_artifact`

Converts the task contract and selected context into a compact executable artifact.

Output:

- proposed file writes or patches
- validation commands to run, when known
- public interface expectations to verify
- required receipts before success
- blocker reason when no safe mutation can be proposed

Rules:

- Prefer the smallest executable artifact that can make progress.
- Do not include benchmark names, fixture paths, or eval-specific symbols as production behavior.
- If the artifact is malformed or non-executable, repair the artifact shape before broadening task scope.
- If a stronger model is needed, ask it to repair or regenerate the artifact from evidence, not to replace the whole runtime loop.

### `implementation_entry_gate`

Hard runtime invariant:

```text
If requires_mutation is true and no successful file_write/file_patch receipt exists,
the task cannot finalize.
```

Allowed actions:

- `file_write`
- `file_patch`

Disallowed actions:

- final answer
- success synthesis
- validation-as-success
- checkpoint closure
- memory closure

### `file_mutation_executor`

Owns concrete source/test/doc/checkpoint mutations.

Receipt requirements:

- tool name
- status
- path
- old/new hash when available
- scope decision
- error envelope on failure

### `public_interface_verifier`

Checks requested or test-imported public surfaces after mutation.

Examples:

- changed tests import `from package.module import Symbol`
- prompt asks for `ClassName(arg=...)`
- prompt asks for object attrs like `result.invalid_rows`

This primitive may use language/profile adapters, but the controller-facing output is language-neutral:

- `missing_public_interface`
- `missing_constructor_shape`
- `missing_return_field`
- `wrong_module_surface`

### `validation_runner`

Runs declared validation commands after mutation when requested or required.

Validation before mutation is baseline only.

Output:

- command
- exit code
- stdout/stderr summary
- success boolean
- receipt ref

### `tool_retry_reflection`

Classifies failed tool calls before retrying.

Output:

- failed tool name
- failure kind
- schema or parameter mismatch
- corrected next action
- attempts remaining

This is not a prompt-only reminder. It is a controller-visible repair state for tool-call failure.

### `doom_loop_interrupt`

Detects repeated no-progress behavior.

Output:

- repeated action signature
- consecutive count
- redirected action class
- terminal blocker when no safe alternative exists

This prevents higher-level workflows from looping forever while still allowing legitimate retry.

### `bounded_repair_loop`

Consumes structured failure reasons and chooses the next allowed action.

Repair reasons include:

- `missing_product_mutation_receipt`
- `missing_test_change_receipt`
- `missing_public_interface`
- `failed_validation`
- `tool_error`
- `budget_exceeded`
- `missing_user_input`

The repair loop must not broaden scope unless the task contract allows it.

The repair loop chooses the earliest failed primitive to revisit. It should not restart the whole workflow when a narrower repair is available.

Examples:

- malformed manifest -> repair manifest shape
- missing mutation receipt -> return to implementation entry and mutation executor
- failed validation -> repair changed source/tests from command evidence
- missing public interface -> repair export/module/constructor/return surface
- repeated no-progress -> redirect to a different allowed action or terminal blocker

### `final_receipt_synthesis`

Final response is derived from receipts, not model optimism.

Required fields:

- status
- changed files
- validation run/not run
- unresolved blockers
- receipt refs
- caveats

## Reference-system mechanics to assimilate

The runtime trace harness records these mechanics from downloaded coding-agent systems:

- mini-SWE-agent: step loop, observation append, trajectory save, step/cost budgets
- SWE-agent: explicit search/edit/submit tools, exact edit contracts
- SWE-ReX: runtime boundary with command output, exit code, failure reason, session state
- Aider: context/repo-map, diff discipline, lint/test repair loop
- OpenHands: event/action/observation/state model, replay and stuck detection
- Cline: tool lifecycle, permission policy, session runtime, edit/diff projection
- ForgeCode: multi-mode coding runtime, detailed tool contracts, runtime limits, benchmark validation, tool-error reflection, doom-loop interruption, pending-todo finalization gates

Level 3/4 live comparison added these portable mechanics:

- Aider: selected source/test context before edit, surgical patch application, static lint feedback before final success
- mini-SWE-agent: simple action/observation loop that reads source and test files, mutates the owner source before or with tests, then validates from command evidence
- Infring target: native runtime should infer likely source/test pairs for small existing projects, verify imported public APIs against the source owner module, and return recoverable partial progress when a provider times out after mutations

These are primitives, not eval cases. They must work for flat Python modules, `src/` Python layouts, and analogous source/test structures without naming benchmark fixtures.

## Infring rebuild target

```text
coding_project_operator
  -> task_contract
  -> context_pack_builder
  -> source_test_pair_context_selection
  -> executable_manifest_or_patch_artifact
  -> implementation_entry_gate
  -> file_mutation_executor
  -> owner_module_completion_gate
  -> public_interface_verifier
  -> validation_runner
  -> tool_retry_reflection
  -> doom_loop_interrupt
  -> bounded_repair_loop
  -> final_receipt_synthesis
```

The high-level workflow composes primitives. It should not inline broad behavior rules.

## Promotion rule

A coding workflow change is valid only if:

- it improves or preserves lower-level primitive behavior,
- it does not add fixture-specific production logic,
- it has receipt-backed eval evidence,
- it can be explained as a primitive, adapter, contract, policy, or composite wiring change.
