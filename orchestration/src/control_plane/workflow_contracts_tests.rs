use super::lifecycle::select_workflow_template;
use super::workflow_contract_guard::workflow_web_provider_boundary_contract;
use super::workflow_contracts::{
    registered_workflow_graphs, registered_workflow_validations, tool_family_contracts,
    workflow_registry_contract_ok, REQUIRED_JSON_OWNS, REQUIRED_RUST_OWNS,
};
use super::workflow_lab_replay::{
    local_coding_program_builder_lab_execution_report,
    local_coding_program_builder_lab_replay_report,
};
use super::workflow_runtime::select_runtime_workflow;
use crate::contracts::{
    Mutability, OperationKind, PlanStatus, PolicyScope, RequestClass, RequestClassification,
    RequestKind, RequestSurface, ResourceKind, TypedOrchestrationRequest, WorkflowTemplate,
};
use serde_json::json;
use serde_json::Value;
use std::path::Path;

fn workflow_source_json(relative_path: &str) -> Value {
    let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let raw = std::fs::read_to_string(source_path).expect("workflow source");
    serde_json::from_str(&raw).expect("workflow json")
}

fn string_array_contains(value: &Value, pointer: &str, expected: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| items.iter().any(|item| item.as_str() == Some(expected)))
        .unwrap_or(false)
}

#[test]
fn workflow_specs_compile_to_no_injection_graphs() {
    let validations = registered_workflow_validations();
    assert!(validations.iter().all(|row| row.ok), "{validations:?}");
    assert!(validations.iter().all(|row| {
        row.graph
            .as_ref()
            .map(|graph| {
                graph.visible_chat_policy == "llm_final_only_no_system_injection"
                    && graph.interaction_source == "json_workflow_spec"
                    && graph.rust_reader_role == "validate_execute_trace_only"
                    && !graph.hardcoded_interaction_behavior_allowed
                    && !graph.final_response_policy.trim().is_empty()
                    && graph
                        .final_output_contract
                        .get("schema_version")
                        .and_then(|value| value.as_str())
                        .is_some()
                    && REQUIRED_JSON_OWNS
                        .iter()
                        .all(|item| graph.json_owns.iter().any(|row| row == item))
                    && REQUIRED_RUST_OWNS
                        .iter()
                        .all(|item| graph.rust_owns.iter().any(|row| row == item))
            })
            .unwrap_or(false)
    }));
}

#[test]
fn tool_family_contracts_are_receipt_bound_and_non_leaking() {
    let contracts = tool_family_contracts();
    assert_eq!(contracts.len(), 6);
    assert!(contracts.iter().all(|row| {
        row.receipt_binding_required
            && row.visible_chat_leakage_forbidden
            && !row.request_schema.is_empty()
            && !row.observation_schema.is_empty()
    }));
}

#[test]
fn workflow_json_calls_web_capabilities_not_vendor_provider_adapters() {
    let report = workflow_web_provider_boundary_contract(&registered_workflow_graphs());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report.get("violation_count").and_then(Value::as_u64),
        Some(0)
    );
}

#[test]
fn workflow_registry_separates_official_and_lab_profiles() {
    assert!(workflow_registry_contract_ok());
    let graphs = registered_workflow_graphs();
    assert!(graphs.iter().any(|graph| graph.workflow_tier == "official"));
    assert!(graphs.iter().any(|graph| graph.workflow_tier == "lab"));
    assert!(graphs.iter().all(|graph| {
        if graph.workflow_tier == "official" {
            graph.runtime_selectable
                && graph
                    .source_json_path
                    .starts_with("orchestration/src/control_plane/workflows/official/")
        } else {
            !graph.runtime_selectable
                && graph
                    .source_json_path
                    .starts_with("orchestration/src/control_plane/workflows/lab/")
        }
    }));
}

#[test]
fn lab_framework_workflows_are_not_runtime_selectable() {
    assert!(select_runtime_workflow("clarify_then_coordinate").is_some());
    assert!(select_runtime_workflow("coding_project_operator").is_some());
    assert!(select_runtime_workflow("openhands_control_plane_assimilation").is_none());
    assert!(select_runtime_workflow("codex_tooling_synthesis").is_none());
    assert!(select_runtime_workflow("local_coding_program_builder").is_none());
}

#[test]
fn coding_mutation_requests_route_to_coding_project_operator() {
    let request = TypedOrchestrationRequest {
        session_id: "routing-smoke".to_string(),
        surface: RequestSurface::Cli,
        legacy_intent: "Implement a Python CLI in this local repo and update tests.".to_string(),
        adapted: true,
        payload: json!({"goal": "local coding task"}),
        request_kind: RequestKind::Direct,
        operation_kind: OperationKind::Mutate,
        resource_kind: ResourceKind::Workspace,
        mutability: Mutability::Mutation,
        target_descriptors: Vec::new(),
        target_refs: vec!["src/main.py".to_string()],
        tool_hints: vec!["workspace".to_string(), "shell".to_string()],
        policy_scope: PolicyScope::WorkspaceOnly,
        user_constraints: Vec::new(),
        core_probe_envelope: None,
    };
    let classification = RequestClassification {
        request_class: RequestClass::Mutation,
        confidence: 0.95,
        reasons: vec!["local coding marker".to_string()],
        required_capabilities: Vec::new(),
        clarification_reasons: Vec::new(),
        needs_clarification: false,
        surface_adapter_used: false,
        surface_adapter_fallback: false,
    };
    assert_eq!(
        select_workflow_template(&request, &classification, PlanStatus::Ready, false, None),
        WorkflowTemplate::CodingProjectOperator
    );
}

#[test]
fn local_coding_program_builder_declares_master_coding_loop_contract() {
    let graphs = registered_workflow_graphs();
    let graph = graphs
        .iter()
        .find(|graph| graph.workflow_id == "local_coding_program_builder")
        .expect("local coding program builder graph");
    assert_eq!(graph.workflow_tier, "lab");
    assert_eq!(graph.promotion_status, "candidate");
    assert_eq!(graph.workflow_role, "assistant_response_workflow");
    assert!(!graph.runtime_selectable);
    assert_eq!(graph.primitive_level, 3);
    for child_id in [
        "research_synthesize_verify",
        "project_context_capture",
        "resume_context_retrieval",
        "memory_freshness_guard",
        "local_context_pack_builder",
        "local_coding_ingress_guard",
        "cli_intent_argument_ingress",
        "interactive_input_session_state",
        "command_prompt_generation",
        "user_prompt_context_assembly",
        "local_coding_session_bootstrap_guard",
        "agent_provider_model_binding",
        "conversation_session_bootstrap",
        "terminal_command_context_snapshot",
        "external_file_change_notice",
        "title_commit_helper_generation",
        "local_coding_remote_service_guard",
        "workspace_remote_auth_boundary",
        "workspace_sync_indexing",
        "codebase_semantic_search",
        "provider_transport_boundary",
        "local_coding_operator_integration_guard",
        "sandbox_worktree_isolation",
        "operator_command_surface_projection",
        "external_update_editor_auth_boundary",
        "schema_data_generation_pipeline",
        "local_coding_shell_terminal_guard",
        "zsh_terminal_context_capture",
        "zsh_command_dispatcher",
        "zsh_completion_buffer_projection",
        "zsh_environment_doctor",
        "local_policy_permission_guard",
        "forge_config_resolution",
        "operation_permission_gate",
        "local_context_loop_guard",
        "tool_access_resolver",
        "doom_loop_interrupt",
        "pending_todo_completion_gate",
        "context_compaction_summary",
        "tool_error_reflection",
        "agent_task_delegation",
        "local_tooling_surface_guard",
        "tool_schema_registry",
        "tool_call_normalization",
        "mcp_tool_bridge",
        "custom_command_skill_loader",
        "local_coding_prompt_template_guard",
        "embedded_template_registry",
        "system_skill_prompt_projection",
        "recovery_command_template_projection",
        "local_runtime_execution_loop",
        "agent_request_transform_pipeline",
        "turn_retry_stream_runner",
        "tool_call_execution_dispatch",
        "lifecycle_hook_dispatch",
        "conversation_state_persistence",
        "local_runtime_observability_guard",
        "chat_response_visibility_router",
        "streaming_markdown_projection",
        "tool_output_display_format",
        "trace_event_rate_limiter",
        "plan_execute_review",
        "plan_artifact_create",
        "local_code_edit_execution",
        "safe_file_read",
        "safe_file_write",
        "safe_file_patch",
        "validation_command_runner",
        "safe_file_undo",
        "followup_clarification_gate",
        "failure_diagnosis",
        "bounded_repair_loop",
        "checkpoint_handoff",
        "checkpoint_memory_write",
        "existing_project_evidence_contract",
    ] {
        assert!(graph
            .composed_of_workflow_ids
            .iter()
            .any(|row| row == child_id));
    }

    let source = workflow_source_json(
        "src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json",
    );
    let composition = source
        .get("workflow_composition_contract")
        .expect("composition contract");
    assert_eq!(
        composition.get("cd_kind").and_then(Value::as_str),
        Some("composite")
    );
    assert_eq!(
        composition
            .get("returns_exactly_one_terminal_artifact")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        composition
            .get("child_workflow_calls")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(21)
    );
    assert!(
        source
            .get("coding_memory_resume_contract")
            .and_then(|contract| contract.get("required_state_fields"))
            .and_then(Value::as_array)
            .map(|items| {
                [
                    "project_memory_refs",
                    "project_fingerprint",
                    "resume_context_summary",
                    "memory_freshness_status",
                    "checkpoint_memory_receipt",
                ]
                .iter()
                .all(|required| items.iter().any(|item| item.as_str() == Some(*required)))
            })
            .unwrap_or(false),
        "missing coding memory resume contract"
    );
    assert!(
        source
            .get("existing_project_evidence_contract")
            .and_then(|contract| contract.get("required_machine_fields"))
            .and_then(Value::as_array)
            .map(|items| {
                [
                    "slice_handoff",
                    "planning_confidence_score",
                    "tactical_readiness_score",
                    "changed_files",
                    "unrelated_file_preservation",
                    "repair_policy",
                    "checkpoint_handoff",
                ]
                .iter()
                .all(|required| items.iter().any(|item| item.as_str() == Some(*required)))
            })
            .unwrap_or(false),
        "missing canonical existing-project machine-readable evidence contract"
    );

    let contract = source
        .get("program_builder_contract")
        .expect("program builder contract");
    assert_eq!(
        contract.get("version").and_then(Value::as_str),
        Some("local_coding_program_builder_v1")
    );
    for required_section in [
        "checkpoint_policy",
        "memory_resume_policy",
        "project_initialization_policy",
        "architecture_contract_policy",
        "coding_ingress_guard_contract",
        "session_bootstrap_guard_contract",
        "remote_service_guard_contract",
        "operator_integration_guard_contract",
        "runtime_execution_loop_contract",
        "runtime_observability_guard_contract",
        "slice_policy",
        "loop_policy",
    ] {
        assert!(
            contract.get(required_section).is_some(),
            "missing {required_section}"
        );
    }
    assert!(contract
        .get("stop_conditions")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str().unwrap_or("").contains("validation fails")))
        .unwrap_or(false));
    assert!(contract
        .get("metrics")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str() == Some("architecture_drift_events")))
        .unwrap_or(false));

    let state_contract = source
        .get("state_tracking_contract")
        .expect("state tracking contract");
    assert!(state_contract
        .get("required_state_fields")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str() == Some("architecture_contract")))
        .unwrap_or(false));
    assert!(graph
        .final_output_contract
        .get("required_summary_fields")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str() == Some("recommended_next_checkpoint")))
        .unwrap_or(false));
}

#[test]
fn local_context_pack_builder_is_registered_as_level_zero_primitive() {
    let graphs = registered_workflow_graphs();
    let graph = graphs
        .iter()
        .find(|graph| graph.workflow_id == "local_context_pack_builder")
        .expect("local context pack builder graph");
    assert_eq!(graph.workflow_tier, "lab");
    assert_eq!(graph.promotion_status, "lab");
    assert_eq!(graph.primitive_level, 0);
    assert!(graph.composed_of_workflow_ids.is_empty());
    assert!(!graph.runtime_selectable);

    let source = workflow_source_json(
        "src/control_plane/workflows/lab/primitives/coding_context/local_context_pack_builder.workflow.json",
    );
    assert_eq!(
        source
            .pointer("/workflow_composition_contract/cd_kind")
            .and_then(Value::as_str),
        Some("primitive")
    );
    assert!(string_array_contains(
        &source,
        "/local_context_pack_contract/required_behaviors",
        "read_current_files_before_trusting_memory_hints"
    ));
    assert_eq!(
        source
            .pointer("/local_context_pack_contract/confidence_contract/execute_ready_min_score")
            .and_then(Value::as_f64),
        Some(0.8)
    );
}

#[test]
fn local_coding_program_builder_declares_strategic_planning_confidence_gate() {
    let source = workflow_source_json(
        "src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json",
    );

    assert_eq!(
        source
            .pointer("/strategic_planning_contract/schema_version")
            .and_then(Value::as_str),
        Some("local_coding_strategic_planning_v1")
    );
    for decision in [
        "product_goal_and_success_definition",
        "architecture_direction",
        "stack_and_runtime_choice",
        "checkpoint_or_slice_boundary",
        "quality_bar_and_validation_strategy",
        "user_owned_open_questions",
    ] {
        assert!(
            string_array_contains(
                &source,
                "/strategic_planning_contract/owns_decisions",
                decision
            ),
            "missing strategic decision owner {decision}"
        );
    }
    assert_eq!(
        source
            .pointer("/planning_confidence_gate/schema_version")
            .and_then(Value::as_str),
        Some("local_coding_planning_confidence_gate_v1")
    );
    assert_eq!(
        source
            .pointer("/planning_confidence_gate/thresholds/execute/min_score")
            .and_then(Value::as_f64),
        Some(0.8)
    );
    assert_eq!(
        source
            .pointer("/planning_confidence_gate/thresholds/self_repair_plan/min_score")
            .and_then(Value::as_f64),
        Some(0.6)
    );
    assert!(string_array_contains(
        &source,
        "/planning_confidence_gate/question_gate_policy/ask_user_only_if",
        "uncertainty_materially_changes_product_goal_architecture_stack_data_model_or_checkpoint"
    ));
    assert!(string_array_contains(
        &source,
        "/planning_confidence_gate/question_gate_policy/reject_questions_when",
        "the issue is a local setup_runtime_validation_or_evidence problem"
    ));
    assert_eq!(
        source
            .pointer("/slice_handoff_contract/schema_version")
            .and_then(Value::as_str),
        Some("local_coding_slice_handoff_v1")
    );
    for field in [
        "immutable_decisions",
        "flexible_decisions",
        "acceptance_criteria",
        "validation_contract",
        "planning_confidence_score",
    ] {
        assert!(
            string_array_contains(&source, "/slice_handoff_contract/required_fields", field),
            "missing slice handoff field {field}"
        );
    }
    assert!(string_array_contains(
        &source,
        "/slice_handoff_contract/immutable_decisions_default",
        "stack_choice"
    ));
    assert!(string_array_contains(
        &source,
        "/slice_handoff_contract/flexible_decisions_default",
        "file_layout_inside_slice"
    ));
    assert!(string_array_contains(
        &source,
        "/workflow_phase_contract/phase_order",
        "planning_confidence_gate"
    ));
    assert_eq!(
        source
            .pointer("/workflow_phase_contract/planning_collision_prevention/parent_owned_decisions_are_immutable_during_slice_execution")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn forgecode_plan_artifact_is_tactical_only_and_readiness_gated() {
    let source = workflow_source_json(
        "src/control_plane/workflows/lab/primitives/coding_loop/plan_artifact_create.workflow.json",
    );

    assert_eq!(
        source
            .pointer("/tactical_planning_contract/schema_version")
            .and_then(Value::as_str),
        Some("forgecode_tactical_slice_planning_v1")
    );
    for decision in [
        "files_to_read_or_edit_for_slice",
        "implementation_order",
        "local_test_or_validation_commands",
        "local_repair_strategy",
        "evidence_to_return",
    ] {
        assert!(
            string_array_contains(
                &source,
                "/tactical_planning_contract/owns_decisions",
                decision
            ),
            "missing tactical decision owner {decision}"
        );
    }
    for parent_owned in [
        "product_goal",
        "architecture_direction",
        "stack_choice",
        "checkpoint_scope",
        "acceptance_criteria",
    ] {
        assert!(
            string_array_contains(
                &source,
                "/tactical_planning_contract/does_not_own",
                parent_owned
            ),
            "missing tactical exclusion {parent_owned}"
        );
    }
    assert_eq!(
        source
            .pointer("/tactical_planning_contract/input_contract/requires_slice_handoff_contract")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        source
            .pointer("/slice_execution_readiness_gate/schema_version")
            .and_then(Value::as_str),
        Some("forgecode_slice_execution_readiness_gate_v1")
    );
    assert_eq!(
        source
            .pointer("/slice_execution_readiness_gate/execute_threshold")
            .and_then(Value::as_f64),
        Some(0.8)
    );
    assert!(string_array_contains(
        &source,
        "/slice_execution_readiness_gate/required_checks",
        "no_parent_owned_decision_needs_revision"
    ));
    assert!(string_array_contains(
        &source,
        "/slice_execution_readiness_gate/forbidden_when_not_ready",
        "change_stack_choice"
    ));
    assert_eq!(
        source
            .pointer("/slice_execution_readiness_gate/return_envelopes/parent_plan_gap")
            .and_then(Value::as_str),
        Some("needs_parent_planning")
    );
    assert!(string_array_contains(
        &source,
        "/quality_contracts",
        "parent_owned_decision_immutability"
    ));
}

#[test]
fn coding_workflow_routes_internal_breakpoints_to_repair_not_user_feedback() {
    let main = workflow_source_json(
        "src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json",
    );
    assert!(string_array_contains(
        &main,
        "/breakpoint_triage_contract/classification_axes/runtime_or_language_compatibility_recovery",
        "behavior_preserving_language_api_substitution_available"
    ));
    assert!(string_array_contains(
        &main,
        "/breakpoint_triage_contract/classification_axes/validation_and_evidence_completion",
        "validation_command_did_not_discover_tests"
    ));
    assert_eq!(
        main.pointer(
            "/typed_execution_contract/setup_preflight_policy/lock_current_working_directory_to_assigned_root_before_writes",
        )
        .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        main.pointer(
            "/breakpoint_triage_contract/mandatory_internal_repair_before_user_feedback/compatibility",
        )
        .and_then(Value::as_str),
        Some("apply_behavior_preserving_runtime_or_language_api_substitutions_without_asking_user")
    );

    let diagnosis = workflow_source_json(
        "src/control_plane/workflows/lab/primitives/coding_loop/failure_diagnosis.workflow.json",
    );
    assert_eq!(
        diagnosis
            .pointer(
                "/breakpoint_triage_contract/generalized_classification_policy/behavior_preserving_local_repairs_are_never_user_questions",
            )
            .and_then(Value::as_bool),
        Some(true)
    );

    let repair = workflow_source_json(
        "src/control_plane/workflows/lab/primitives/coding_loop/bounded_repair_loop.workflow.json",
    );
    assert_eq!(
        repair
            .pointer(
                "/breakpoint_repair_contract/mandatory_internal_repair_rules/behavior_preserving_compatibility_fix",
            )
            .and_then(Value::as_str),
        Some("must_repair_without_user_feedback")
    );
    assert_eq!(
        repair
            .pointer(
                "/breakpoint_repair_contract/mandatory_internal_repair_rules/zero_test_validation_fix",
            )
            .and_then(Value::as_str),
        Some("must_replace_with_explicit_test_discovery_before_counting_validation")
    );

    let followup = workflow_source_json(
        "src/control_plane/workflows/lab/primitives/coding_loop/followup_clarification_gate.workflow.json",
    );
    assert_eq!(
        followup
            .pointer(
                "/user_feedback_gate_contract/forbidden_user_question_policy/do_not_ask_for_behavior_preserving_local_repair",
            )
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(string_array_contains(
        &followup,
        "/user_feedback_gate_contract/reject_as_invalid_user_feedback_classes",
        "validation_command_discovery_recovery"
    ));

    let plan = workflow_source_json(
        "src/control_plane/workflows/lab/primitives/coding_loop/plan_artifact_create.workflow.json",
    );
    assert!(string_array_contains(
        &plan,
        "/slice_execution_readiness_gate/required_checks",
        "validation_commands_have_explicit_test_discovery_when_needed"
    ));
    assert!(string_array_contains(
        &plan,
        "/slice_execution_readiness_gate/allowed_when_not_ready",
        "replace_behavior_preserving_unsupported_runtime_api"
    ));
}

#[test]
fn local_coding_program_builder_candidate_lab_replay_proof_passes() {
    let report = local_coding_program_builder_lab_replay_report();
    assert!(report.ok, "{report:#?}");
    assert_eq!(report.workflow_id, "local_coding_program_builder");
    assert_eq!(report.promotion_status, "candidate");
    assert!(!report.runtime_selectable);
    assert_eq!(report.primitive_level, 3);
    assert_eq!(report.scenarios.len(), 3);
    for scenario_id in [
        "single_file_utility",
        "small_multi_file_app",
        "initialized_project_modification",
    ] {
        assert!(report
            .scenarios
            .iter()
            .any(|scenario| scenario.id == scenario_id && scenario.ok));
    }
}

#[test]
fn local_coding_program_builder_lab_execution_harness_emits_coding_task_plans() {
    let report = local_coding_program_builder_lab_execution_report();
    assert!(report.ok, "{report:#?}");
    assert_eq!(report.workflow_id, "local_coding_program_builder");
    assert_eq!(
        report.harness_kind,
        "local_coding_program_builder_lab_execution_dry_run_v1"
    );
    assert_eq!(report.task_executions.len(), 3);
    for execution in &report.task_executions {
        assert!(execution.ok, "{execution:#?}");
        assert!(!execution.checkpoint.acceptance_criteria.is_empty());
        assert!(!execution.architecture_contract.boundary_rules.is_empty());
        assert!(!execution.validation_plan.is_empty());
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_code_edit_execution"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_coding_ingress_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_coding_session_bootstrap_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_coding_remote_service_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_coding_operator_integration_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_policy_permission_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_context_loop_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_tooling_surface_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_runtime_execution_loop"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_runtime_observability_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "bounded_repair_loop"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "checkpoint_handoff"));
        assert!(execution
            .final_handoff_fields
            .contains(&"recommended_next_checkpoint"));
    }
    let multi_file = report
        .task_executions
        .iter()
        .find(|execution| execution.task_id == "small_multi_file_app")
        .expect("small multi-file app execution");
    assert!(multi_file
        .slice_invocations
        .iter()
        .any(|slice| slice.name == "domain_model_slice"));
    assert!(multi_file
        .slice_invocations
        .iter()
        .any(|slice| slice.name == "primary_flow_slice"));
}
