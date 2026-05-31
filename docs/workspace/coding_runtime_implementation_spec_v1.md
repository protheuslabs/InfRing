# Coding Runtime Implementation Spec v1

Status: build contract draft

Behavioral source model: `docs/workspace/coding_runtime_model_v1.md`

Assimilation package: `docs/workspace/coding_behavioral_assimilation_v1.md`

Level 1 reference trace: `docs/workspace/coding_level1_reference_trace_observations.md`

Purpose: define the exact implementation contract for rebuilding Infring coding as a primitive-first runtime spine. This document converts the behavioral model into buildable primitives, schemas, transitions, ownership boundaries, and eval promotion rules.

## Reset decision: semi-aggressive controller rebuild

Status: active as of 2026-05-30

The current coding runtime contains useful primitives, but the controlling shape
has become a set of overlapping controllers. That shape is now migration debt.
The rebuild rule is:

```text
Keep primitives. Scrap competing controllers. Route behavior through one spine.
```

Keep:

- native file tools
- command execution tools
- safe shell edit parsing
- receipts and journals
- timing probes
- validation/probe harnesses
- Workflow CD contracts
- reference-framework trace corpus

Quarantine unless reintroduced as evidence adapters:

- layered planning controllers
- seeded import-surface repair branches
- mutation-only recovery branches
- completion evidence repair loops
- profile-specific controller behavior
- staged controller branches
- broad timeout recovery branches
- eval-level-derived behavior

Authoritative rebuild target:

```text
coding_execution_spine_v1
```

The spine owns deterministic state transition. It does not own provider calls,
tool execution, language-specific symbol detection, fixture scoring, or final
user projection. Those systems must feed normalized evidence into the spine and
accept the spine decision.

Initial routing scope:

- Low-complexity lanes that correspond to Level 1 through Level 3 behavior:
  `new_file_fast_path`, `micro_direct_mutation`, `existing_project_patch`,
  `bounded_existing_project_edit`, and `implementation_slice`.
- No Level 4+ optimization until Level 1-3 are monotonic.
- Any higher-level patch must run at least one lower-level smoke before
  promotion.

## Design goal

The coding runtime must be cohesive enough to handle arbitrary local coding tasks without level-specific production logic.

The implementation must be built from monotonic primitives:

- lower primitives must stay valid as higher workflows are added
- higher workflows may compose primitives, but must not alter their basic meaning
- eval fixtures may be specific, but production runtime behavior must stay primitive-first
- no success claim may bypass receipts
- every primitive or guard must declare `profile_min`
- primitives with `profile_min` above the selected profile must be invisible to that run
- higher-profile work must run lower-profile smoke before promotion

Activation audit:

`docs/workspace/coding_profile_activation_audit.md`

Machine-readable activation matrix:

`orchestration/src/control_plane/workflows/lab/composites/coding/coding_profile_activation_matrix.json`

## Runtime spine

Canonical order:

```text
coding_task_contract
-> context_pack_builder
-> project_operator_context_packet
-> implementation_entry_gate
-> tool_progress_watchdog
-> first_mutation_artifact_lane
-> file_mutation_executor
-> receipt_journal
-> public_interface_verifier
-> validation_runner
-> semantic_closeout_probe
-> failure_diagnosis
-> tool_retry_reflection
-> doom_loop_interrupt
-> bounded_repair_loop
-> checkpoint_handoff
-> final_receipt_synthesis
```

Allowed fast path for new single-file creation:

```text
coding_task_contract
-> implementation_entry_gate
-> file_mutation_executor
-> receipt_journal
-> final_receipt_synthesis
```

The fast path is allowed only when the task contract says local context is unnecessary.

## Ownership

| Primitive | Primary owner | Reason |
|---|---|---|
| `coding_task_contract` | Orchestration | Interprets user intent and maps it to runtime requirements. |
| `context_pack_builder` | Orchestration | Selects relevant local context through tool-backed providers. |
| `project_operator_context_packet` | Orchestration | Compresses project/checkpoint context into an action-oriented packet before the first model edit turn. |
| `implementation_entry_gate` | Orchestration/Core boundary | Enforces mutation-before-success invariants. |
| `tool_progress_watchdog` | Orchestration/Observability boundary | Detects provider turns that produce no tool receipts and routes to compact recovery or runtime failure. |
| `first_mutation_artifact_lane` | Orchestration/Core boundary | Converts compact project context into one constrained source/test/operator mutation batch before the open tool loop. |
| `file_mutation_executor` | Core tool substrate | Owns local file mutation and mutation receipts. |
| `receipt_journal` | Core/Observability boundary | Records authoritative execution evidence. |
| `public_interface_verifier` | Orchestration with adapters | Checks task-facing API shape without hardcoding languages globally. |
| `validation_runner` | Validation | Runs declared checks and returns command receipts. |
| `semantic_closeout_probe` | Orchestration with validation/tool adapters | Runs product/operator evidence probes after validation when the task requires public or operator-facing behavior. |
| `failure_diagnosis` | Orchestration | Converts failed receipts into structured repair reasons. |
| `tool_retry_reflection` | Orchestration | Repairs malformed or incorrect tool calls. |
| `doom_loop_interrupt` | Orchestration/Observability boundary | Detects repeated no-progress loops from event history. |
| `bounded_repair_loop` | Orchestration | Selects the next earliest safe repair action. |
| `checkpoint_handoff` | Orchestration/Memory boundary | Captures resumable project context after useful progress. |
| `final_receipt_synthesis` | Orchestration/Shell projection | Produces user-visible summary from receipts and blockers. |

Layer rule:

Core owns irreversible local effects and authoritative receipts. Orchestration owns sequencing and gates. Validation owns command proof. Observability owns trace/journal projection. Shell owns user-facing projection only.

## Primitive interfaces

### `coding_task_contract`

Input:

```json
{
  "user_prompt": "string",
  "workspace_root": "string",
  "allowed_write_roots": ["string"],
  "permissions": ["file.read", "file.write", "file.patch", "command.run"],
  "workflow_profile": "coding"
}
```

### `tool_progress_watchdog`

Input:

```json
{
  "task_id": "string",
  "turn_index": 0,
  "receipt_count": 0,
  "has_successful_mutation": false,
  "provider_timeout_seconds": 240,
  "first_receipt_deadline_seconds": 45
}
```

Output:

```json
{
  "action": "continue | compact_recovery | structured_runtime_failure",
  "reason": "string",
  "next_provider_timeout_seconds": 45
}
```

Rules:

- A mutation-required task must not spend an entire long provider turn with zero
  mutation receipts before recovery.
- Streaming provider adapters must enforce this with interruptible I/O polling,
  not by checking a deadline only before a blocking stdout read.
- Recovery must be primitive, not fixture-specific: compact the prompt, preserve
  observed public API shapes, prefer additive source/test changes, and ask for
  the smallest receipt-producing tool batch.
- If recovery still produces no tool receipts, classify the run as a
  runtime/controller progress failure rather than a completed coding attempt.

### `project_operator_context_packet`

Input:

```json
{
  "task_id": "string",
  "project_root": "string",
  "context_receipts": ["receipt_ref"],
  "checkpoint_memory_receipt": "receipt_ref | null",
  "validation_command": "string",
  "handoff_paths": ["string"],
  "memory_row_id": "string"
}
```

Output:

```json
{
  "type": "project_operator_context_packet_v1",
  "active_stage": "product_mutation | operator_surface_mutation | test_mutation | validation | semantic_closeout | checkpoint_handoff | memory_closure",
  "allowed_next_tools": "string",
  "stage_goal": "string",
  "stage_stop_condition": "string",
  "context_receipts": [],
  "public_shape_lines_to_preserve": [],
  "product_slice_gaps": [],
  "compact_observation": "string"
}
```

Rules:

- The first model edit turn for checkpointed project/operator work should see a
  compact packet, not a full workflow dump.
- The packet must preserve baseline API/CLI shapes as evidence, not as
  fixture-specific hardcoding.
- The packet should route the model to the next smallest stage action and keep
  handoff/memory closure behind validation and semantic closeout evidence.

### `first_mutation_artifact_lane`

Input:

```json
{
  "type": "project_operator_context_packet_v1",
  "active_stage": "product_mutation | operator_surface_mutation | test_mutation",
  "allowed_next_tools": "file_write | file_patch | controlled command_run edit batch",
  "provider_timeout_seconds": 75
}
```

Output:

```json
{
  "type": "first_mutation_artifact_lane_receipt_v1",
  "tool_receipts": [],
  "successful_mutation": true
}
```

Rules:

- This lane exists to avoid sending checkpointed project work directly into a
  broad open native tool loop for the first mutation.
- The model receives a minimal edit-engine system prompt and must return only
  `{"tool_calls":[...]}`.
- Reads, validation, handoff, and memory closure are forbidden in this lane.
- If no safe mutation is possible from the packet, the lane returns no mutation
  and the controller may fall back to the normal staged loop.
- Tool-call normalization may skip schema examples and placeholder calls before
  choosing an executable candidate.
- Tool-call normalization must not turn arbitrary incomplete/broken outer
  envelopes into trusted broad edit batches; salvage belongs to a bounded
  retry/reflection primitive, not the parser boundary.

### `validation_guided_repair_turn`

Input:

```json
{
  "task_contract_ref": "receipt-ref",
  "context_pack_ref": "receipt-ref",
  "failed_validation_receipt_ref": "receipt-ref",
  "allowed_write_scope": ["string"],
  "max_repair_mutation_calls": 4,
  "first_repair_receipt_deadline_seconds": 20
}
```

Output:

```json
{
  "type": "validation_guided_repair_turn_receipt_v1",
  "repair_mutation_receipts": [],
  "post_mutation_validation_receipt": "receipt-ref | null",
  "status": "repaired | blocked | no_progress | validation_still_failing",
  "reason": "string"
}
```

Rules:

- This primitive only runs after concrete validation evidence exists.
- The repair packet must contain compact failing assertions, command status,
  selected source/test file context, and allowed mutation tools.
- The repair turn may mutate source or tests only when the task contract allows
  that artifact role.
- The repair turn must not run broad discovery, memory handoff, checkpointing,
  or final synthesis.
- If no mutation receipt appears by the deadline, return `no_progress` with the
  failed validation receipt and selected context refs.
- Final success remains owned by `final_receipt_synthesis`, not by this repair
  primitive.

Output:

```json
{
  "task_id": "string",
  "task_kind": "create_file | existing_project_patch | debug_repair | refactor | project_slice | explanation_only",
  "requires_context": true,
  "requires_mutation": true,
  "requires_validation": false,
  "requires_public_interface_check": false,
  "allowed_write_scope": ["string"],
  "target_artifacts": ["source | test | doc | config | checkpoint"],
  "public_surface_requirements": [],
  "stop_conditions": ["success", "blocker", "budget", "needs_user_input"],
  "confidence": 0.0,
  "open_questions": []
}
```

Receipt:

```json
{
  "type": "task_contract_receipt_v1",
  "task_id": "string",
  "status": "created | needs_user_input | unsupported",
  "reason": "string"
}
```

Rules:

- Do not inspect large files.
- Do not choose detailed architecture for multi-slice projects.
- If confidence is below the configured planning threshold and missing facts are user-owned, return `needs_user_input`.
- If confidence is below threshold but facts are discoverable locally, continue to `context_pack_builder`.

### `context_pack_builder`

Input:

```json
{
  "task_contract_ref": "receipt-ref",
  "candidate_paths": ["string"],
  "search_terms": ["string"],
  "max_files": 12,
  "max_bytes": 120000
}
```

Output:

```json
{
  "selected_files": [
    {
      "path": "string",
      "reason": "string",
      "role": "source | test | config | docs | unknown"
    }
  ],
  "excluded_paths": [
    {
      "path": "string",
      "reason": "string"
    }
  ],
  "likely_validation_commands": ["string"],
  "public_api_owners": ["string"],
  "confidence": 0.0,
  "open_questions": []
}
```

Receipt:

```json
{
  "type": "context_pack_receipt_v1",
  "status": "selected | skipped_fast_path | blocked",
  "selected_files": ["string"],
  "tool_receipts": ["receipt-ref"]
}
```

Rules:

- Existing-project mutation requires context unless the contract explicitly marks the task as context-free.
- New single-file tasks may skip context if no local dependency is implied.
- Context selection must use file tools or registered context providers, not invisible prompt assumptions.

### `implementation_entry_gate`

Input:

```json
{
  "task_contract_ref": "receipt-ref",
  "context_pack_ref": "receipt-ref | null",
  "receipt_journal_ref": "receipt-ref",
  "proposed_action": "file_write | file_patch | validate | finalize | ask_user"
}
```

Output:

```json
{
  "allowed": true,
  "allowed_actions": ["file_write", "file_patch"],
  "blocked_reason": "string | null",
  "repair_reason": "string | null"
}
```

Rules:

- If `requires_mutation=true` and no successful mutation receipt exists, finalization is forbidden.
- If existing-project mutation lacks sufficient context, mutation is forbidden unless the task contract explicitly allows blind create.
- Validation before mutation is baseline evidence only and cannot satisfy implementation.

### `file_mutation_executor`

Input:

```json
{
  "operation": "write | patch",
  "path": "string",
  "content": "string | null",
  "old_text": "string | null",
  "new_text": "string | null",
  "allowed_write_scope": ["string"]
}
```

Output receipt:

```json
{
  "type": "file_mutation_receipt_v1",
  "tool": "file_write | file_patch",
  "status": "success | failed",
  "path": "string",
  "operation": "write | patch",
  "old_hash": "string | null",
  "new_hash": "string | null",
  "bytes_written": 0,
  "scope_decision": "allowed | denied",
  "error": {
    "kind": "path_out_of_scope | patch_context_not_unique | file_missing_for_patch | permission_denied | io_error",
    "message": "string"
  }
}
```

Rules:

- Mutation receipt is the only source of truth for whether code changed.
- Tool success does not imply task success.
- Failed mutation must flow to `failure_diagnosis`.

### `receipt_journal`

Input:

```json
{
  "task_id": "string",
  "event": "state_transition | tool_call | tool_result | validation_result | blocker | final",
  "receipt": {}
}
```

Output:

```json
{
  "journal_ref": "string",
  "latest_state": "string",
  "mutation_receipts": ["receipt-ref"],
  "validation_receipts": ["receipt-ref"],
  "blockers": ["receipt-ref"],
  "event_count": 0
}
```

Rules:

- Final synthesis reads from the journal, not from model memory.
- Doom-loop detection reads repeated action signatures from the journal.
- Receipts must be append-only for a run.

### `public_interface_verifier`

Input:

```json
{
  "task_contract_ref": "receipt-ref",
  "changed_paths": ["string"],
  "language_profile": "python | javascript | typescript | rust | generic",
  "public_surface_requirements": []
}
```

Output receipt:

```json
{
  "type": "public_interface_verification_receipt_v1",
  "status": "verified | gap | skipped",
  "checked_paths": ["string"],
  "missing": [
    {
      "kind": "missing_symbol | wrong_constructor | missing_return_field | wrong_module_surface",
      "name": "string",
      "path": "string | null"
    }
  ]
}
```

Rules:

- Language-specific checks must live behind adapters.
- This primitive should start generic and only add adapters when evals require them.
- Missing public interface must flow to repair, not final success.

### `validation_runner`

Input:

```json
{
  "commands": ["string"],
  "cwd": "string",
  "timeout_seconds": 60,
  "after_mutation_receipts": ["receipt-ref"]
}
```

Output receipt:

```json
{
  "type": "validation_receipt_v1",
  "status": "passed | failed | skipped | blocked",
  "command": "string",
  "exit_code": 0,
  "stdout_excerpt": "string",
  "stderr_excerpt": "string",
  "duration_ms": 0,
  "after_mutation": true
}
```

Rules:

- Validation requested by the user must run after mutation or be reported as not run with a blocker reason.
- Exit code zero is validation evidence, not full task success by itself.
- Timeout is a structured validation failure.

### `semantic_closeout_probe`

Input:

```json
{
  "task_contract_ref": "receipt-ref",
  "journal_ref": "receipt-ref",
  "validated_after_mutation": true,
  "operator_surface_requirements": ["cli | report | import | export | round_trip | public_api"],
  "project_root": "string"
}
```

Output receipt:

```json
{
  "type": "semantic_closeout_probe_receipt_v1",
  "status": "passed | failed | skipped | blocked",
  "commands": ["string"],
  "covered_requirements": ["string"],
  "missing_requirements": ["string"],
  "after_validation": true
}
```

Rules:

- Run only after post-mutation validation has passed.
- Required when a project/operator task asks for CLI, report, import, export,
  round-trip, or public API behavior.
- Probes are product evidence, not hidden judge fixtures. They should exercise
  the public/operator surface that the task itself requested.
- A passing validation command does not satisfy this stage unless it also
  explicitly exercises the requested operator/public behavior.
- Failed probes route to `failure_diagnosis` and then the earliest safe
  product/operator/test repair state.
- Handoff and memory closure are forbidden while required semantic closeout
  probes are missing.

### `failure_diagnosis`

Input:

```json
{
  "latest_failed_receipt": "receipt-ref",
  "task_contract_ref": "receipt-ref",
  "journal_ref": "receipt-ref"
}
```

Output:

```json
{
  "repair_reason": "missing_context | failed_validation | malformed_tool_call | mutation_failed | missing_public_interface | repeated_no_progress | needs_user_input | unsupported",
  "earliest_failed_state": "string",
  "safe_next_actions": ["string"],
  "terminal_blocker": "string | null"
}
```

Rules:

- Repair targets the earliest unsatisfied state.
- Do not broaden write scope during diagnosis.
- User-facing blocker is required when missing information is not locally discoverable.

### `tool_retry_reflection`

Input:

```json
{
  "failed_tool_receipt": "receipt-ref",
  "attempts_used": 0,
  "attempts_left": 0,
  "tool_schema_ref": "string"
}
```

Output:

```json
{
  "status": "retry | blocked",
  "failure_kind": "wrong_tool | missing_parameter | malformed_arguments | permission_denied | unavailable",
  "corrected_action": {},
  "reflection_summary": "string"
}
```

Rules:

- This primitive repairs tool-call shape, not code behavior.
- Repeating the same failed signature must invoke `doom_loop_interrupt`.
- Permission failures do not retry unless policy changes.

### `doom_loop_interrupt`

Input:

```json
{
  "journal_ref": "receipt-ref",
  "current_action_signature": "string",
  "consecutive_threshold": 3
}
```

Output:

```json
{
  "status": "continue | redirect | blocked",
  "repeated_signature": "string",
  "consecutive_count": 0,
  "redirect_action": "string | null",
  "blocker_reason": "string | null"
}
```

Rules:

- Same failed tool/action signature at threshold cannot run again unchanged.
- Redirect should choose a different primitive or ask for user input.
- This is generic no-progress control, not an eval-specific patch.

### `bounded_repair_loop`

Input:

```json
{
  "repair_reason": "string",
  "repair_budget": {
    "max_attempts": 3,
    "max_validation_runs": 2,
    "max_duration_ms": 300000
  },
  "journal_ref": "receipt-ref"
}
```

Output:

```json
{
  "status": "repaired | retrying | exhausted | blocked",
  "next_state": "string | null",
  "budget_remaining": {},
  "blocker_reason": "string | null"
}
```

Rules:

- Repair loops may only return to the earliest unsatisfied state.
- Repair cannot claim success without new receipts.
- Budget exhaustion becomes `failed_budget` or `partial_blocked`.

### `checkpoint_handoff`

Input:

```json
{
  "task_id": "string",
  "journal_ref": "receipt-ref",
  "changed_paths": ["string"],
  "validation_status": "passed | failed | not_run",
  "open_items": ["string"]
}
```

Output receipt:

```json
{
  "type": "checkpoint_handoff_receipt_v1",
  "status": "written | skipped | blocked",
  "artifact_refs": ["string"],
  "summary": "string"
}
```

Rules:

- Checkpoint handoff is not a substitute for source/test mutation.
- Long-horizon workflows may require handoff before final.
- Simple tasks may skip handoff.

### `final_receipt_synthesis`

Input:

```json
{
  "task_contract_ref": "receipt-ref",
  "journal_ref": "receipt-ref",
  "terminal_state": "success | partial_blocked | failed_budget | failed_tooling | needs_user_input"
}
```

Output:

```json
{
  "status": "success | partial | blocked | failed",
  "changed_files": ["string"],
  "validation": {
    "status": "passed | failed | not_run",
    "commands": ["string"]
  },
  "blockers": ["string"],
  "receipt_refs": ["string"],
  "user_message": "string"
}
```

Rules:

- Final answer must not claim unreceipted file changes.
- Final answer must say when validation was not run.
- Final answer must expose blocker reasons without exposing raw internal traces.

## State transitions

| From | To | Condition |
|---|---|---|
| `start` | `coding_task_contract` | Coding category selected. |
| `coding_task_contract` | `final_receipt_synthesis` | Explanation-only task or no mutation required. |
| `coding_task_contract` | `context_pack_builder` | Existing context is required or locally discoverable. |
| `coding_task_contract` | `implementation_entry_gate` | Context-free creation fast path is valid. |
| `context_pack_builder` | `project_operator_context_packet` | Checkpointed project/operator work has bounded context and needs staged action. |
| `context_pack_builder` | `implementation_entry_gate` | Context selected or fast path confirmed for non-project-operator work. |
| `context_pack_builder` | `partial_blocked` | Required context missing and not inferable. |
| `project_operator_context_packet` | `implementation_entry_gate` | Compact staged action packet created. |
| `implementation_entry_gate` | `tool_progress_watchdog` | Mutation required and safe action exists. |
| `implementation_entry_gate` | `failure_diagnosis` | Proposed action violates gate. |
| `tool_progress_watchdog` | `first_mutation_artifact_lane` | Checkpointed project/operator work needs its first source/test/operator mutation. |
| `tool_progress_watchdog` | `file_mutation_executor` | Provider produced a receipt-bearing tool call or compact recovery is active outside the first-mutation lane. |
| `tool_progress_watchdog` | `failure_diagnosis` | Provider turn exceeded the first-receipt deadline without usable tool receipts. |
| `first_mutation_artifact_lane` | `file_mutation_executor` | Constrained edit artifact produced mutation tool calls. |
| `first_mutation_artifact_lane` | `failure_diagnosis` | Artifact is empty, malformed, unsafe, or only proposes forbidden non-mutation work. |
| `file_mutation_executor` | `receipt_journal` | Mutation receipt produced. |
| `file_mutation_executor` | `failure_diagnosis` | Mutation failed. |
| `receipt_journal` | `public_interface_verifier` | Public surface check required. |
| `receipt_journal` | `validation_runner` | Validation required and public surface has no gap. |
| `receipt_journal` | `final_receipt_synthesis` | Required receipts are satisfied and no validation required. |
| `public_interface_verifier` | `validation_runner` | Surface verified or skipped. |
| `public_interface_verifier` | `failure_diagnosis` | Surface gap found. |
| `validation_runner` | `semantic_closeout_probe` | Validation passed and operator/public behavior evidence is required. |
| `validation_runner` | `final_receipt_synthesis` | Validation passed, no semantic closeout is required, or user only requested run status. |
| `validation_runner` | `failure_diagnosis` | Validation failed or timed out. |
| `semantic_closeout_probe` | `checkpoint_handoff` | Required semantic probes passed and handoff is required. |
| `semantic_closeout_probe` | `final_receipt_synthesis` | Required semantic probes passed and no handoff is required. |
| `semantic_closeout_probe` | `failure_diagnosis` | Probe failed, timed out, or missed a requested surface. |
| `failure_diagnosis` | `tool_retry_reflection` | Failure is malformed or incorrect tool call. |
| `failure_diagnosis` | `doom_loop_interrupt` | Repeated no-progress signature detected. |
| `failure_diagnosis` | `bounded_repair_loop` | Safe repair exists. |
| `failure_diagnosis` | `partial_blocked` | Repair needs user input or unsafe scope expansion. |
| `tool_retry_reflection` | `bounded_repair_loop` | Corrected action exists. |
| `tool_retry_reflection` | `partial_blocked` | No safe retry exists. |
| `doom_loop_interrupt` | `bounded_repair_loop` | Redirect action exists. |
| `doom_loop_interrupt` | `partial_blocked` | No useful progress path remains. |
| `bounded_repair_loop` | earliest failed state | Budget remains. |
| `bounded_repair_loop` | `failed_budget` | Budget exhausted. |
| `checkpoint_handoff` | `final_receipt_synthesis` | Handoff written or skipped. |

Forbidden transitions:

- `implementation_entry_gate -> final_receipt_synthesis` when mutation is required and no mutation receipt exists
- `validation_runner -> final_receipt_synthesis` when validation was run before mutation and no post-mutation validation exists
- `validation_runner -> checkpoint_handoff` when required semantic closeout probes are missing
- `semantic_closeout_probe -> final_receipt_synthesis` when requested operator/public behavior has no probe receipt
- `failure_diagnosis -> final_receipt_synthesis` without repair, blocker, or budget failure receipt
- `bounded_repair_loop -> final_receipt_synthesis` without a new satisfying receipt or terminal blocker

## Failure taxonomy

| Failure reason | Meaning | User input needed? |
|---|---|---|
| `missing_user_input` | The task depends on user-owned facts. | Yes |
| `context_not_found` | Local context search found no relevant files. | Maybe |
| `ambiguous_context` | Multiple likely targets and mutation would be risky. | Maybe |
| `path_out_of_scope` | Requested write target violates allowed scope. | Yes |
| `permission_denied` | Required tool permission is missing. | Yes |
| `malformed_tool_call` | Tool arguments do not satisfy schema. | No |
| `tool_unavailable` | Required tool is missing or failed to load. | Maybe |
| `mutation_failed` | File write or patch did not succeed. | No |
| `missing_product_mutation_receipt` | Runtime tried to finish without mutation evidence. | No |
| `missing_public_interface` | Required public API shape is absent. | No |
| `failed_validation` | Post-mutation command failed. | No |
| `validation_timeout` | Validation exceeded budget. | Maybe |
| `repeated_no_progress` | Same action or failure repeated past threshold. | Maybe |
| `budget_exceeded` | Repair or runtime budget exhausted. | Maybe |

## Tool mapping

| Runtime need | Native tool/capability | Required receipt |
|---|---|---|
| list files | `file_list` | `file_observation_receipt_v1` |
| inspect file metadata | `file_stat` | `file_observation_receipt_v1` |
| read file | `file_read` | `file_observation_receipt_v1` |
| read several files | `file_read_many` | `file_observation_receipt_v1` |
| write new file | `file_write` | `file_mutation_receipt_v1` |
| patch existing file | `file_patch` | `file_mutation_receipt_v1` |
| resolve command/tool executable | `command_resolve` | `command_resolution_receipt_v1` |
| run validation | `command_run` | `validation_receipt_v1` |
| record trace | `receipt_journal_append` | `journal_receipt_v1` |
| capture handoff | memory/project-context writer | `checkpoint_handoff_receipt_v1` |

Tool availability rule:

If a workflow declares a tool in its contract but the native runtime cannot expose it, fail closed with `failed_tooling`.

Command execution mode rule:

Workflow CDs declare tool intent and allowed execution modes; native tooling resolves the concrete executable and emits a receipt. Coding workflows must not hardcode development launchers such as `cargo run` unless they are explicitly dev/debug workflows.

| Execution mode | Meaning | Timing comparable |
|---|---|---|
| `installed_cli_binary` | Use an installed/PATH or configured binary directly. | yes |
| `workspace_cli_binary` | Use an already-built workspace binary directly. | yes |
| `dev_build_then_run` | Build through an admitted build command, then run the produced binary directly. | only after the build step is separately receipted |
| `cargo_run_dev_fallback` | Use `cargo run` as a last-resort development/debug launcher. | no |

Gate rule:

Eval/comparison workflows should allow `installed_cli_binary`, `workspace_cli_binary`, and optionally `dev_build_then_run`; they should forbid `cargo_run_dev_fallback`. Local development workflows may allow all modes, but receipts must mark `cargo_run_dev_fallback` as `timing_comparable: false`.

## Eval-level mapping

| Eval level | Required primitives |
|---|---|
| Level 1 | `coding_task_contract`, `implementation_entry_gate`, `tool_progress_watchdog`, `file_mutation_executor`, `receipt_journal`, `final_receipt_synthesis` |
| Level 2 | Level 1 plus `context_pack_builder` for simple existing-file patches |
| Level 3 | Level 2 plus `validation_runner` |
| Level 4 | Level 3 plus `failure_diagnosis` and basic `bounded_repair_loop` |
| Level 5 | Level 4 plus `public_interface_verifier` adapters |
| Level 6 | Level 5 plus `checkpoint_handoff` and project context capture |
| Level 7 | Level 6 plus stronger context selection and multi-file mutation receipts |
| Level 8 | Level 7 plus `project_operator_context_packet`, `first_mutation_artifact_lane`, `semantic_closeout_probe`, `tool_retry_reflection`, and `doom_loop_interrupt` |
| Level 9+ | Level 8 plus long-horizon planning, architecture checkpoints, memory retrieval, and multi-slice orchestration |

Promotion rule:

A higher-level eval cannot be considered healthy if it regresses lower-level evals. If Level N breaks Level 1 or Level 2, the Level N change is invalid unless it exposes a previously hidden primitive bug and the fix preserves lower-level behavior.

Lower-profile smoke rule:

- After any coding runtime/workflow/tooling patch, run one smoke attempt for every profile lower than or equal to the highest touched profile.
- Before promotion, run five attempts for every affected and lower profile.
- Before reliability claims, run twenty attempts for the claimed profile.
- If a lower profile regresses, stop higher-profile work and patch activation boundaries first.

## Implementation phases

### Phase 1: smallest mutation spine

Build:

- `coding_task_contract`
- `implementation_entry_gate`
- `file_mutation_executor`
- `receipt_journal`
- `final_receipt_synthesis`

Goal:

- Level 1 passes repeatedly without context hacks.

### Phase 2: existing-project context

Build:

- `context_pack_builder`
- context-required gate behavior

Goal:

- Level 2 passes without hardcoded file names or eval-specific read prompts.

### Phase 3: validation and repair

Build:

- `validation_runner`
- `failure_diagnosis`
- `bounded_repair_loop`

Goal:

- Level 3 and Level 4 pass with post-mutation validation and generic repair.

### Phase 4: public interface and project slices

Build:

- `public_interface_verifier`
- language/profile adapters
- multi-file receipt aggregation

Goal:

- Level 5 to Level 7 pass without special-casing task names.

### Phase 5: ForgeCode control assimilation

Build:

- `tool_retry_reflection`
- `doom_loop_interrupt`
- `pending_todo_completion_gate`
- benchmark-style command timeout and early-exit validation controls

Goal:

- Higher-level tasks fail gracefully, avoid infinite loops, and report actionable blockers.

### Phase 6: long-horizon coding workflow

Build:

- checkpoint handoff
- memory capture/retrieval
- architecture slice planning
- composite orchestration around the primitive spine

Goal:

- Long software tasks can resume and scale without corrupting lower primitives.

## Anti-hardcoding enforcement

Production runtime code may not branch on:

- eval level names
- fixture filenames
- exact benchmark task text
- language-specific behavior outside an adapter
- success phrases in model output

Allowed specificity:

- eval harnesses may contain fixture-specific expectations
- adapters may contain language-specific checks
- workflow CDs may configure budgets and profiles
- primitive contracts may define generic task classes

Required justification for new behavior:

Every new coding behavior must declare whether it is a primitive, adapter, policy, workflow composition, tool contract, receipt schema, or eval-only fixture.

## Build readiness checklist

The implementation can begin when these are true:

- this spec is linked from the behavioral assimilation package
- coding eval Level 1 expectations map only to Phase 1 primitives
- native file tools emit mutation receipts compatible with this spec
- final synthesis can read receipt journal data
- lower-level monotonic gate is enforced in eval workflow

Current status:

- model is complete enough to build from
- implementation should begin at Phase 1
- do not resume high-level patching until Phase 1 and Phase 2 are stable
