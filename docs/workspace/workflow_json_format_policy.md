# Workflow JSON Format Policy

Status: active  
Scope: assistant response workflows loaded by the control-plane workflow reader

## Purpose

Define one canonical, human/LLM-friendly workflow format so workflow definitions are not embedded ad hoc in Rust logic.

Canonical format: JSON (`*.workflow.json`)

## Burnable CD / CD Player Reference

The workflow JSON is the burnable CD. The Rust workflow reader/runtime is the CD player.

That means workflow interaction behavior must be contained in the JSON spec, and the Rust side must load, validate, execute, and trace the selected spec without inventing hidden workflow behavior.

JSON owns:

1. Interaction gates
2. Gate options
3. Gate transitions
4. Tool family menus
5. Tool input schemas
6. Confirmation states
7. Loopbacks
8. Final-output contract
9. Trace/status wording for workflow visibility surfaces

Rust owns:

1. JSON loading
2. Schema validation
3. Deterministic state-transition execution
4. Tool execution handoff
5. Receipt binding
6. Trace export
7. Kernel policy enforcement

Acceptance rule:

If changing a workflow JSON file cannot change the workflow's interaction behavior without editing Rust, that behavior is migration debt unless it is a new primitive/tool implementation, a safety policy, or a schema validator. Rust may reject an invalid CD; it must not secretly write a different CD while playing it.

Primitive composition rule:

Higher-level workflow changes must be monotonic over lower-level primitives. A composite may route to, sequence, and checkpoint child workflows, but it must not force level-specific policy into shared primitive behavior. If a harder workflow level breaks an easier workflow level, repair the composition boundary before adding more runtime special cases. See `docs/workspace/primitive_workflow_composition_doctrine.md`.

Primitive-first anti-hardcoding rule:

Production workflow behavior must follow the repo-wide Primitive-First System Doctrine in `docs/workspace/primitive_first_system_doctrine.md`. It must not be hardcoded to specific benchmark cases, prompt phrases, fixture file names, app shapes, languages, or framework examples. Specific-case hardcoding is allowed only inside eval and test fixtures. Production behavior must be represented as reusable primitives, composite workflow routing, declared lanes, tool contracts, schema validators, safety policies, or extensible data/config contracts.

Registered production Workflow CDs and Tool CDs must declare a primitive-first contract or be covered by the primitive capability registry policy. The minimum contract shape is:

```json
{
  "primitive_first_contract": {
    "case_specific_hardcoding_allowed": false,
    "specificity_owner": "primitive|composite|adapter_profile|policy|schema|test_eval_fixture",
    "extension_surface": "workflow_cd|tool_cd|schema|policy|adapter_metadata|profile_config|composition"
  }
}
```

Rust must not infer a workflow route, research category, or expected answer shape from specific user prompt phrases such as "update", "landscape", "compare", or similar wording. If a workflow needs a raw-message recovery path after the workflow has already selected a tool family/tool, that recovery must be declared by the JSON/CD tool contract and bind the original user message without assuming a domain-specific query type.

Reader implementation: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046a-workflow-reader.rs`

Orchestration template reader implementation:
`orchestration/src/control_plane/templates.rs`

Orchestration workflow contract reader/guard:
`orchestration/src/control_plane/workflow_contracts.rs`

Current workflow spec directory:
`core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows/`

Orchestration workflow template directories:

1. Official/tested workflows: `orchestration/src/control_plane/workflows/official/`
2. Experimental lab workflows: `orchestration/src/control_plane/workflows/lab/`
3. Framework comparison profiles: `orchestration/src/control_plane/workflows/lab/frameworks/`

Workflow registry:
`orchestration/src/control_plane/workflows/workflow_registry.json`

Example template:
`docs/workspace/templates/workflow/workflow_template.workflow.json`

Tool menu interface example template:
`docs/workspace/templates/workflow/tool_menu_interface_v1.workflow.json`

## Format Contract

All workflow specs must be JSON objects.

Required reader-acceptance fields:

1. `name` (string, non-empty after sanitization)
2. `stages` (array of stage strings, at least one non-empty item after sanitization)
3. `workflow_type` (`control_plane_orchestration_workflow` for control-plane workflows; `hard_agent_workflow` for current dashboard assistant-response CDs)
4. `workflow_role` (`assistant_response_workflow` or `assimilation_workflow_template`) for control-plane workflow registries
5. `final_response_policy` (string, non-empty)
6. `gate_contract` (string, non-empty)
7. `workflow_source_of_truth_contract` (object)
8. `tool_menu_interface_contract` (object) for assistant toolbox workflows
9. `gate_6_llm_final_output.final_output_contract` inside `tool_menu_interface_contract` for assistant toolbox workflows
10. `typed_execution_contract` (object, required for control-plane promotion)

If any required field is invalid/empty, the reader rejects that spec.
Assistant toolbox workflows are also rejected unless `tool_menu_interface_contract` declares non-empty `llm_gate_instruction`, `llm_tool_request_instruction`, `tool_request_submission_contract`, `gate_order`, `gate_shapes_allowed`, `terminal_states`, `declared_loopbacks`, `tool_family_menu`, `tool_menu_by_family`, `trace_status_messages`, `diagnostic_markers`, the canonical gate objects, Gate 1 accepted outputs/options, at least one no-tool category, at least one tool category with a matching `tool_menu_by_family` entry, formal confirm/cancel states, and `system_injected_chat_text_allowed: false`. The Rust reader must never synthesize missing menus, prompt text, diagnostic phrase lists, or fallback tool categories from partial workflow JSON.

When a workflow declares a confirmation gate, the default execution mode for that gate must still come from JSON. A workflow may declare `execution_mode: "auto_if_permitted"` plus `confirmation_required_when: "permission_gate_asks"` so the runtime auto-executes ordinary allowed tool calls and only pauses when Kernel permission policy explicitly asks for confirmation. Rust may enforce the permission gate; it must not invent a stricter default confirmation policy than the CD declared.

`trace_status_messages` is the source of truth for status wording shown in workflow visibility surfaces such as the UI thinking bubble and agent-process trace. Rust may produce deterministic status keys and gate resume tokens while executing the workflow, but it must read human/agent-facing status text from the JSON CD. Assistant toolbox CDs must provide both `ui` and `agent_process` text for every runtime-visible status key: `default`, `pending_final_llm`, `synthesized`, `no_post_synthesis_required`, `skipped_missing_model`, `diagnostic_failure_pass_through`, `synthesis_failed`, `guard_violation_pass_through`, and `empty_llm_response`.

Control-plane promotion requires the Rust workflow contract guard to compile each JSON workflow into a typed graph before runtime use. `workflow_source_of_truth_contract` must include:

1. `interaction_source`: `json_workflow_spec`
2. `rust_reader_role`: `validate_execute_trace_only`
3. `hardcoded_interaction_behavior_allowed`: `false`
4. `json_owns` containing `interaction_gates`, `gate_options`, `gate_transitions`, `tool_family_menus`, `tool_input_schemas`, `confirmation_states`, `loopbacks`, `final_output_contract`, `trace_status_messages`, and `diagnostic_markers`
5. `rust_owns` containing `json_loading`, `schema_validation`, `state_transition_execution`, `tool_execution_handoff`, `receipt_binding`, `trace_export`, and `kernel_policy_enforcement`

Example:

```json
{
  "workflow_source_of_truth_contract": {
    "interaction_source": "json_workflow_spec",
    "rust_reader_role": "validate_execute_trace_only",
    "hardcoded_interaction_behavior_allowed": false,
    "json_owns": [
      "interaction_gates",
      "gate_options",
      "gate_transitions",
      "tool_family_menus",
      "tool_input_schemas",
      "confirmation_states",
      "loopbacks",
      "final_output_contract",
      "trace_status_messages",
      "diagnostic_markers"
    ],
    "rust_owns": [
      "json_loading",
      "schema_validation",
      "state_transition_execution",
      "tool_execution_handoff",
      "receipt_binding",
      "trace_export",
      "kernel_policy_enforcement"
    ]
  }
}
```

`typed_execution_contract` must include:

1. `gate_kind` (string)
2. `input_kind` (`multiple_choice`, `text_input`, or `multiple_choice_or_text_input`)
3. `allowed_transitions` (`stage_a->stage_b` strings, with terminal transitions allowed)
4. `timeout_ms` (positive integer)
5. `retry_policy.max_retries` and `retry_policy.on_failure`
6. `terminal_states` containing `completed`, `needs_input`, `blocked`, `failed`, and `aborted`
7. `telemetry_streams` containing `workflow_state`, `agent_internal_notes`, `tool_trace`, `eval_trace`, and `final_answer`
8. `tool_family_contracts` containing `workspace`, `web`, `memory`, `agent`, `shell`, and `browser`
9. `visible_chat_policy` set to `llm_final_only_no_system_injection`
10. `run_budgets` with max stages, model turns, tool calls, token budget, and loop-signature detector

Optional fields:

1. `default` (default: `false`)
2. `description` (default: `""`)

`final_response_policy` and `gate_contract` are required. Unknown extra keys are currently ignored by the reader.

## Composable Workflow CD Contract

Workflow CDs are executable workflow programs, not loose config fragments. Every registered CD must declare `workflow_composition_contract` so primitive and composite workflow boundaries are machine-checkable.

Required fields:

1. `version`: `workflow_composition_contract_v1`
2. `cd_kind`: `primitive` or `composite`
3. `returns_exactly_one_terminal_artifact`: `true`
4. `terminal_artifact_contract.artifact_schema`: `workflow_terminal_artifact_v1`
5. `terminal_artifact_contract.allowed_kinds`: typed terminal artifact kinds such as `completed_final_answer`, `clarification_request`, `structured_failure`, and for composite CDs `delegated_workflow_result`
6. `terminal_artifact_contract.parent_receives`: `terminal_artifact_only_refs_for_internals`
7. `child_workflow_calls`: typed child-CD calls, empty for primitive CDs

Child workflow calls must declare `capability`, `workflow_id`, `input_contract`, `output_artifact`, `return_policy`, and `internal_trace_visibility`. `return_policy` must be `return_single_terminal_artifact_to_parent`; `internal_trace_visibility` must be `refs_only`.

Composition rules:

1. Primitive CDs must not call child workflows.
2. Composite CDs may call primitive or composite CDs through typed child-call envelopes.
3. A child CD returns one terminal artifact to the parent; raw tool payloads, internal traces, and child-local state remain behind refs/receipts.
4. The default/main workflow CD may delegate online research to a research workflow CD, but must not inline research retrieval, evidence packing, or synthesis internals as parent-owned behavior.
5. Full research workflows are composite capability CDs, not primitive CDs; smaller retrieval, evidence-pack, synthesis, and verifier CDs may later become primitives under the research composite.
6. Rust may validate, compile, execute, cache, and fail closed on invalid composition contracts; Rust must not invent undeclared child workflow calls or terminal artifact shapes.

## Workflow Role Rule

The control-plane workflow directory contains both assistant-response workflows and assimilation workflow templates. They must be role-typed so the runtime and guards do not confuse strategy templates with normal chat finalization paths.

Allowed roles:

1. `assistant_response_workflow`: may participate in user-visible assistant response flow and must obey the LLM-final-only policy.
2. `assimilation_workflow_template`: may describe structured assimilation strategy and required signals, but must not be treated as the default user-visible response workflow.

Assimilation workflow templates must declare at least one `subtemplates` row. Each subtemplate must include non-empty `id`, `description`, `required_signals`, `required_gates`, and `source_refs` fields so assimilation can be audited as capability transfer instead of ledger burn-down.

Subtemplate `id` values must be unique within the workflow, no longer than 120 characters, and limited to lowercase ASCII letters, digits, `_`, and `-`. Subtemplate `required_signals`, `required_gates`, and `source_refs` must not contain duplicate values. `source_refs` must be repo-relative or local-assimilation paths under approved roots such as `local/workspace/assimilations/`, `local/workspace/vendor/`, `orchestration/`, `docs/workspace/`, `tests/tooling/`, `validation/`, `core/`, or `adapters/`; absolute paths, URL refs, and `..` traversal are invalid.

Assistant-response workflows must not declare `subtemplates`; if a normal response path needs reusable sequencing, promote it into stages/contracts rather than embedding assimilation doctrine.

## Official vs Lab Workflow Rule

Workflow specs are separated by lifecycle tier so framework experiments cannot leak into the runtime default path.

Official workflows:

1. Live under `orchestration/src/control_plane/workflows/official/`.
2. May be runtime-selectable.
3. Must be listed in `workflow_registry.json` with `tier: "official"`, `runtime_selectable: true`, and `promotion_status: "official"`.
4. Must pass schema validation, no-chat-injection checks, runtime replay, trace stream checks, cancellation behavior, and relevant self-play/eval smoke.

Lab workflows:

1. Live under `orchestration/src/control_plane/workflows/lab/`.
2. Include framework-specific profiles under `orchestration/src/control_plane/workflows/lab/frameworks/<framework>/`.
3. Must be listed in `workflow_registry.json` with `tier: "lab"`, `runtime_selectable: false`, and `promotion_status: "lab"`.
4. May be used for comparison, assimilation study, and scorecards.
5. Must not become runtime defaults until promoted through `lab -> candidate -> official -> default`.

Promotion requires an explicit registry change, passing contract guard output, and an SRS update explaining why the workflow belongs in the supported runtime set.

## Research Workflow Freeze Rule

`research_synthesize_verify` is now treated as a stable official composite workflow CD while web tooling is the active experiment surface.

Frozen workflow surface:

1. Gate order and gate meaning
2. Composition boundary and single terminal artifact return
3. Tool request/observation/final-answer lifecycle semantics
4. Final-output channel separation
5. Low-evidence fallback acceptance semantics
6. Workflow-visible trace/status ownership

Permitted non-freeze lanes:

1. Web retrieval Tool CDs
2. Provider adapters and retrieval broker mechanics
3. Evidence extraction, ranking, coverage, and quality metadata
4. Research eval reporting, measurement split, and rubric diagnostics
5. Golden data maintenance that records real failure shapes without changing workflow behavior

Eval discipline for this frozen workflow:

1. Prompt-pool rows, goldens, and bad-prompt banks are probes for failure classes, not behavior specs for the production workflow.
2. A workflow/tooling patch must be justified in generic terms such as answer-unit contamination, traceability drift, undercoverage honesty, or source-quality gating, not "make case X pass."
3. After a patch, success must be re-checked on diverse or random prompts so the system proves generalization rather than prompt memorization.

Freeze exception rule:

Changing the frozen workflow surface requires an explicit freeze exception note in the workflow CD, a passing workflow contract guard, a passing research-golden regression run, and an operator-facing reason explaining why the issue cannot be solved in tooling/evidence/rubric space. Improving retrieval quality, candidate quality, provider resilience, evidence-pack shape, or excellent scoring must not change the research workflow gate semantics.

## Sanitization + Length Limits

The reader sanitizes/truncates text values via `clean_text(...)`.

Current max lengths:

1. `name`: 80
2. `workflow_type`: 80
3. `description`: 600
4. `stage` item: 120
5. `final_response_policy`: 120
6. `gate_contract`: 80

## Default Workflow Rule

The library must resolve to exactly one JSON-declared default workflow.

Fail-closed behavior:

1. If no spec is marked `default: true`, the reader emits a fail-closed loader diagnostic.
2. If multiple specs are marked `default: true`, the reader emits a fail-closed loader diagnostic.
3. If no valid specs load, the reader emits a fail-closed loader diagnostic, not a substitute workflow.
4. The loader diagnostic may expose trace/debug data, but it must not define interaction gates, tool menus, transitions, prompt text, or final-answer wording.
5. Runtime mode strings such as direct-answer/tool-execution telemetry must not be mapped to workflow names. Workflow selection is either the one JSON-declared default or an explicit `workflow=<name>` hint.
6. If an explicit `workflow=<name>` hint names a missing workflow, the reader must emit an explicit not-found diagnostic and must not silently replace it with the default workflow.

## Interface-Only Workflow Rule

Assistant chat workflows are tool access interfaces, not tool-picking advisors.

Allowed gate shapes:

1. Multiple choice menu
2. Text input / request payload field
3. Tool execution handoff using the submitted payload
4. Telemetry-only status export
5. LLM-authored final output

Disallowed workflow behavior:

1. Recommending a tool family
2. Inferring whether a tool is needed
3. Classifying the user turn as task/info
4. Choosing a tool automatically
5. Injecting fallback text into the visible chat response
6. Adding "next actions" or system-authored diagnostic prose to the final answer
7. Rendering workflow state, agent-internal notes, prompt analysis, tool traces, or eval traces as final chat text

No-injection invariant:

1. System-authored fallback text must never be inserted into visible chat.
2. Failure/finalization diagnostics go to telemetry, attention queues, or UI diagnostic streams only.
3. Visible chat text is emitted only by the LLM final output stage.
4. `visible_chat_policy` must remain `llm_final_only_no_system_injection`.
5. `gate_6_llm_final_output.final_output_contract` must declare the visible chat source and the internal streams excluded from chat.

Tool-completion invariant:

1. A workflow that emits a tool request or records a tool observation must not silently terminate as `completed` without an LLM-authored final answer.
2. If the runtime reaches the end of a tool-bearing path without synthesized final output, it must fail closed with a telemetry-only `structured_failure` terminal artifact.
3. Rust may enforce this invariant generically as part of deterministic workflow execution; it must not satisfy it by injecting visible fallback chat text.

Canonical stage vocabulary:

1. `gate_1_work_category_menu`
2. `gate_2_tool_family_menu`
3. `gate_3_tool_menu`
4. `gate_4_request_payload_input`
5. `gate_4b_tool_confirmation_menu`
6. `gate_5_post_tool_menu`
7. `gate_6_llm_final_output`

Direct response is represented by the private `Respond directly` category at `gate_1_work_category_menu`, not by a separate automatic bypass classifier.

## Tool Menu Interface Contract

Assistant-response workflows using `gate_contract: "tool_menu_interface_v1"` must declare a `tool_menu_interface_contract` object. This keeps the human/LLM-readable JSON as the source of workflow shape instead of leaving important behavior implicit in Rust.

Required fields:

1. `version`: `tool_menu_interface_v1`
2. `visible_chat_policy`: `llm_final_only_no_system_injection`
3. `system_injected_chat_text_allowed`: `false`
4. `gate_shapes_allowed`: only `multiple_choice` and `text_input`
5. `gate_order`: ordered gate ids for the workflow
6. `gates`: gate definitions keyed by canonical gate id
7. `private_tokens`: private menu tokens that must never be emitted as visible chat
8. `terminal_states`: terminal state names
9. `declared_loopbacks`: explicit loopback transitions
10. `gates.gate_6_llm_final_output.final_output_contract`: final visible answer contract
11. `gates.gate_1_work_category_menu.submission_contract`: private Gate 1 submission contract
12. `gates.gate_1_work_category_menu.submission_contract.structured_token_fields`: JSON-declared field names the reader may inspect when the LLM submits a structured Gate 1 object
13. `tool_request_submission_contract`: JSON-declared field order, labels, aliases, and `system_may_infer_missing_fields: false`
14. `diagnostic_markers.legacy_retry_templates`: JSON-declared ghost/fallback signatures for trace/eval diagnostics
15. `diagnostic_markers.deferred_tool_request_phrases`: JSON-declared phrases that indicate a draft is only asking for tools instead of submitting the declared gate/tool request
16. `diagnostic_markers.unresolved_tool_need_phrases`: JSON-declared phrases that indicate an unresolved tool need inside the private workflow gate
17. `diagnostic_markers.gate_choice_prefix_leakage_phrases`: JSON-declared visible-chat signatures for leaked private gate choices
18. `diagnostic_markers.prompt_analysis_leak_phrases`: JSON-declared visible-chat signatures for leaked prompt/workflow analysis
19. `diagnostic_markers.unsupported_tool_claim`: JSON-declared marker groups for detecting tool-result claims without current-turn tool receipts; required groups are `tool_surface_terms`, `execution_claim_phrases`, `empty_result_claim_phrases`, `result_context_terms`, `listing_claim_phrases`, and `hypothetical_phrases`
20. `diagnostic_markers.recorded_tool_result_answer`: JSON-declared marker groups for deciding whether an LLM answer acknowledges a recorded low-signal or failed tool result; required groups are `tool_result_terms` and `no_result_explanation_phrases`

Required gate semantics:

1. `gate_1_work_category_menu` asks exactly `What kind of work is this?`.
2. `Respond directly` and `Planning from current context` are private no-tool category tokens (`private_token: true`, `visible_chat: false`) and transition directly to `gate_6_llm_final_output`.
3. Tool-bearing categories are `Web research`, `Workspace/files`, `Code execution / terminal`, `Agent management`, `Memory/notes`, and `External apps/integrations`.
4. Gate 1 accepts only private submissions: a no-tool category token or `Category: <category>. Tool family: <family>. Tool: <tool>. Request payload: <JSON>.`.
5. Gate 1 forbids choice narration, recommendations without submission, and visible chat text.
6. Structured Gate 1 object submissions are valid only when their token field is named in `structured_token_fields`; Rust must not carry hidden built-in field names.
7. `gate_2_tool_family_menu` is multiple choice.
8. `gate_3_tool_menu` is multiple choice.
9. `gate_4_request_payload_input` is text input.
10. `gate_4b_tool_confirmation_menu` is multiple choice and contains `confirm` and `cancel`.
11. `cancel` is a formal terminal state transition to `cancelled`; it is not an emergent runtime convention.
12. `gate_5_post_tool_menu` is multiple choice and contains `finish` and `another_tool`.
13. `another_tool` must declare an explicit loopback to `gate_2_tool_family_menu`.
14. `gate_6_llm_final_output` is LLM-only final-authority text input.
15. `gate_6_llm_final_output.final_output_contract.visible_chat_source` is `llm_final_answer_only`.
16. `gate_6_llm_final_output.final_output_contract.internal_streams` includes `workflow_state`, `agent_internal_notes`, `tool_trace`, and `eval_trace`.
17. `gate_6_llm_final_output.final_output_contract.chat_excludes` includes every internal stream plus `prompt_analysis`.

Visibility rule:

1. Gate category tokens, `confirm`, and `cancel` are private workflow tokens by default.
2. Private workflow tokens may be stored in telemetry and diagnostics.
3. Private workflow tokens must not be rendered as assistant-visible chat.
4. The final chat box receives only the LLM-authored answer from `gate_6_llm_final_output`.

## Registration Rule (Required)

A valid JSON file is not loaded automatically just by existing in the folder.
Assistant response workflows used by the dashboard route must be wired in `046a-workflow-reader.rs`:

1. Add an `include_str!(...)` constant for the file.
2. Add `(source_path, constant)` entry to `WORKFLOW_SPEC_SOURCES`.
3. Ensure the workflow appears in the library catalog tests if applicable.

Orchestration Control Plane workflows must be wired in `orchestration/src/control_plane/workflow_contracts.rs` and `orchestration/src/control_plane/templates.rs`, then listed in `orchestration/src/control_plane/workflows/workflow_registry.json`.

## Authoring Checklist

Before opening a PR for a new workflow:

1. Start from `docs/workspace/templates/workflow/workflow_template.workflow.json`.
   For assistant toolbox workflows, start from `docs/workspace/templates/workflow/tool_menu_interface_v1.workflow.json`.
2. Keep `name` unique and versioned (example: `_v1`, `_v2`).
3. Set `workflow_type` to `control_plane_orchestration_workflow`.
4. Set `workflow_role` to either `assistant_response_workflow` or `assimilation_workflow_template`.
5. Ensure `stages` is non-empty and ordered by execution flow.
6. Decide whether it should be default (`default: true`) or non-default.
7. Register it in `046a-workflow-reader.rs` when it belongs to the assistant response reader.
8. Run workflow reader regression tests.

Suggested test commands:

1. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_loads_external_specs -- --nocapture`
2. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_enforces_single_default -- --nocapture`
3. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_sources_current_workflows_from_json_specs -- --nocapture`
4. `cargo test --manifest-path orchestration/Cargo.toml workflow_contract -- --nocapture`
5. `cargo run --quiet --manifest-path orchestration/Cargo.toml --bin workflow_contract_guard -- --strict=1`

## Policy Guardrail

Workflow definitions for assistant response flow must remain JSON specs.
Do not introduce new inline Rust-authored workflow definitions. Loader-error containment may emit diagnostics, but it must not define interaction gates, tool menus, transitions, prompt text, or final-answer wording.

## Capability vs Workflow Boundary (Required)

Use this split consistently:

1. Raw system capability/mechanics belong in Rust authority paths.
2. Workflow structure belongs in JSON workflow specs.
3. If a feature is executable authority or kernel/runtime truth, implement it in Rust and reference it from workflow JSON.
4. If a feature is sequencing/flow shape only, implement it as a workflow JSON update.
