# Coding Runtime Behavioral Model v2.1 Implementation Blueprint

Date: 2026-05-20

Status: implementation blueprint

Parent model:

`docs/workspace/coding_runtime_behavioral_model_v2.md`

## Goal

Build the missing runtime topology from the behavioral model.

The target is not simply:

`make Level 1 pass`

The target is:

`make the cheapest safe coding lane fire first, emit receipts/traces, and
escalate only when the task shape requires more machinery.`

## Non-negotiable constraints

- No hardcoded eval levels.
- No hardcoded `hello.py`.
- No hardcoded `greet`.
- No fixture-path logic outside evals.
- Lower-tier changes must not regress lower-tier evals.
- Final answers must not claim mutations without mutation receipts.
- Validation claims must not be made without validation receipts.
- Higher-tier workflows must be built from lower-tier primitives.

## Runtime ownership map

| Concern | Owner | Reason |
| --- | --- | --- |
| Task text and workflow policy | Workflow CD | User-facing routing and contracts must stay configurable. |
| Execution-shape gate contract | Workflow CD plus orchestration validator | CD declares decision fields; orchestration enforces shape. |
| Direct local file mutation | Native local coding capability pack | File mutation is an execution primitive, not prompt behavior. |
| Permission checks | Kernel/capability policy | Permissions must fail closed below workflow logic. |
| Receipts | Native tool/runtime layer | Receipts must be tied to actual side effects. |
| Trace event envelope | Orchestration runtime | All lanes need comparable traces. |
| Final answer synthesis | Workflow CD plus orchestration runtime | CD defines required fields; runtime binds receipts. |
| Eval fixtures | Observability/eval module | Specific levels, prompts, and expected files belong only in evals. |

## Build sequence

### Phase 1: lane trace envelope

Add a generic coding lane trace envelope.

Required fields:

```json
{
  "task_id": "string",
  "workflow_id": "string",
  "lane": "direct_mutation | deterministic_local_loop | model_tool_loop | validation_repair_loop | project_operator_loop | structured_blocker",
  "lane_reason": "string",
  "escalation_reason": "string | null",
  "requires_model": "boolean",
  "requires_discovery": "boolean",
  "requires_validation": "boolean",
  "target_scope": "new_file | known_file | existing_project | unknown",
  "mutation_safety": "safe_new | safe_patch | unsafe_overwrite | ambiguous | not_applicable",
  "phase_latency_ms": {
    "workflow_load": "number",
    "execution_shape_gate": "number",
    "provider_start": "number",
    "model_call": "number",
    "tool_dispatch": "number",
    "mutation": "number",
    "validation": "number",
    "repair": "number",
    "final_synthesis": "number",
    "total": "number"
  },
  "receipt_refs": ["string"]
}
```

This can be introduced without changing coding behavior.

### Phase 2: execution-shape gate

Add a gate that classifies coding tasks before provider invocation.

The gate must produce:

```json
{
  "lane": "direct_mutation",
  "confidence": 0.0,
  "requires_model": false,
  "requires_discovery": false,
  "requires_validation": false,
  "target_scope": "new_file",
  "mutation_safety": "safe_new",
  "escalation_reason": null,
  "direct_mutation_candidate": {
    "mutation_kind": "create_file",
    "target_path": "src/example.py",
    "content_source": "explicit | generated_from_small_spec | unavailable"
  }
}
```

The first implementation may be conservative. It is better to refuse and
escalate than to guess.

### Phase 3: direct mutation primitive

Add `single_mutation_execution` as a native local coding primitive.

Inputs:

```json
{
  "workspace_root": "string",
  "mutation_kind": "create_file | overwrite_file | patch_file | append_file",
  "target_path": "string",
  "content": "string | null",
  "patch": "string | null",
  "permission_receipt": "string",
  "trace_parent": "string"
}
```

Outputs:

```json
{
  "ok": true,
  "mutation_receipt": {
    "receipt_id": "string",
    "tool": "single_mutation_execution",
    "mutation_kind": "create_file",
    "path": "string",
    "bytes_written": 0,
    "created": true,
    "overwrote_existing": false,
    "content_hash": "string"
  },
  "changed_file_summary": [
    {
      "path": "string",
      "operation": "created",
      "bytes_written": 0
    }
  ],
  "trace_events": []
}
```

Structured failure:

```json
{
  "ok": false,
  "failure_code": "unsafe_overwrite | path_escape | missing_content | ambiguous_target | permission_denied | patch_target_not_found",
  "failure_message": "string",
  "needed_input": "string | null"
}
```

### Phase 4: workflow CD wiring

Add or update a primitive workflow:

`coding_execution_shape_gate.workflow.json`

Purpose:

- classify task shape
- decide whether Tier 0 can fire
- emit gate artifact

Add or update a primitive workflow:

`single_mutation_execution.workflow.json`

Purpose:

- invoke native direct mutation primitive
- emit mutation receipt artifact
- produce structured failure on refusal

Update the higher coding workflow:

- call execution-shape gate before model/provider path
- if `lane = direct_mutation`, call `single_mutation_execution`
- if direct mutation refuses, escalate with reason
- preserve normal model/tool loop for non-direct tasks
- final synthesis must include lane, receipts, and validation status

### Phase 5: eval proof

Add evals that prove topology, not only output.

Required Level 1 evals:

- varied file names
- varied small content shapes
- nested safe paths
- no validation requested
- no existing project discovery required

Pass criteria:

- file mutation succeeds
- mutation receipt exists
- final answer references receipt
- lane trace says `direct_mutation`
- `requires_model = false`
- `provider_start_ms = 0`
- `model_call_ms = 0`
- no level-specific strings in runtime code

Required refusal evals:

- missing target path
- unsafe overwrite
- path escape
- ambiguous content
- validation requested without validation lane

Pass criteria:

- no mutation performed
- structured failure returned
- needed input or escalation reason present
- no fake success

## Execution-shape decision table

| Condition | Lane | Reason |
| --- | --- | --- |
| New file path is explicit, content is explicit, write is safe, no validation requested | `direct_mutation` | Cheapest safe lane. |
| New file path is explicit, content is small and mechanically derivable, no discovery needed | `direct_mutation` or `deterministic_local_loop` | Direct if deterministic generator exists; otherwise local loop. |
| Known file path plus exact patch is explicit | `direct_mutation` | Patch can be applied without model inference. |
| Existing project with already-selected compact file context and natural-language edit intent | `small_scoped_edit_artifact` | Cheapest safe model-mediated path: file context in, edit artifact out, deterministic patch receipts. |
| Known file path but edit intent is natural language | `model_tool_loop` | Model must infer concrete patch. |
| Existing project but target file unknown | `model_tool_loop` | Discovery required. |
| Validation requested after mutation | `validation_repair_loop` | Mutation must be followed by command/validation receipts. |
| Multi-file feature with architecture choices | `project_operator_loop` | Requires planning/checkpoints. |
| Target path escapes root | `structured_blocker` | Unsafe path. |
| Existing file would be overwritten without explicit overwrite intent | `structured_blocker` | Unsafe overwrite. |
| Prompt lacks enough content to create requested file safely | `structured_blocker` or `model_tool_loop` | Ask for input if content is required; use model only if generation is appropriate. |

## Direct mutation eligibility rules

A task is eligible for `direct_mutation` only when all are true:

- local workspace root is known
- target path is explicit
- target path is inside allowed root
- mutation kind is known
- required content or exact patch is available
- no project discovery is needed

## Small scoped edit artifact profile

Primitive:

`small_scoped_edit_artifact`

Tier:

`Tier 2a`, a model-mediated artifact profile inside the bounded edit family.

It exists to assimilate the useful part of Aider's fast path without importing
Aider's product-specific behavior or bypassing Infring receipts.

Runtime shape:

```text
selected local file context
-> one compact model call for SEARCH/REPLACE artifact
-> safe_file_patch application
-> mutation receipts
-> parent validation/final synthesis
```

Eligibility:

- selected files are known before provider invocation
- selected file count is within the profile budget
- selected context bytes are within the profile budget
- the task is a bounded local edit
- broad discovery, architecture, checkpoint planning, and repair are not needed
- phase timing is required for context read, file-context assembly, model call,
  patch application, validation, and total lane time
- the profile uses the caller-selected model for now; model routing is explicitly
  omitted until same-model behavior is stable

Non-goals:

- no project discovery
- no validation execution
- no repair loop
- no architecture choice
- no level-specific or fixture-specific logic
- no hidden model substitution

Implementation rule:

The model may only generate an edit artifact. It does not apply edits and it
does not claim completion. Mutation authority remains with `safe_file_patch`
receipts, and parent workflows remain responsible for validation, evidence
coverage, repair, and final response synthesis.

Optimization rule:

Improve this primitive in this order:

1. Measure phase latency.
2. Reduce prompt and context payload.
3. Keep SEARCH/REPLACE as the single fast-profile grammar.
4. Reduce runtime startup or workflow-loading overhead when traces prove it.
5. Revisit model routing only after same-model parity data is clean.
- no architecture decision is needed
- no validation was requested
- permission policy grants the mutation

If any condition is false, the gate must not silently guess.

## Integration with native file tools

`single_mutation_execution` should be a thin primitive over existing native file
tool capability, not a second competing file system abstraction.

Preferred implementation shape:

```text
single_mutation_execution
-> validate path and safety
-> call existing write/patch primitive internally
-> convert native tool result into canonical mutation receipt
-> emit lane trace event
```

This keeps existing file tooling as the substrate while adding the missing
routing topology.

## Integration with final synthesis

Direct mutation final synthesis should not require a model by default.

Template output is acceptable when:

- mutation succeeded
- changed-file summary exists
- no validation was requested
- no ambiguity remains

Template fields:

- lane used
- files changed
- validation status
- receipt IDs

Model synthesis remains available for complex or user-facing explanatory tasks,
but must not be required for Level 1 mutation success.

## How to avoid another hardcoded patch cycle

Every implementation PR/change should answer:

- Which lane does this modify?
- Which lower-tier evals must remain green?
- Which reference behavior is this assimilating?
- Which primitive contract changed?
- Which hardcoded fixture strings are forbidden in runtime code?

If a change fixes only one eval prompt, it belongs in the eval fixture or should
be rejected.

## Immediate implementation checklist

1. Add the trace envelope type.
2. Add `execution_shape_gate` with conservative direct-mutation detection.
3. Add `single_mutation_execution` using existing native file tooling.
4. Add workflow CD entries for the gate and primitive.
5. Wire the higher coding workflow to try Tier 0 first.
6. Add Level 1 topology evals.
7. Run Level 1 for reliability and timing.
8. Only then resume Level 2+.

## Expected result

After implementation, a fully specified one-file creation task should produce:

```text
lane: direct_mutation
requires_model: false
provider_start_ms: 0
model_call_ms: 0
mutation_receipt: present
final answer: receipt-backed
```

That is the minimum proof that the reference runtime behavior was integrated,
not merely documented.
