// Layer ownership: orchestration (non-canonical lab workflow replay proofs only).
use super::workflow_contracts::registered_workflow_graphs;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

const BUILDER_WORKFLOW_ID: &str = "local_coding_program_builder";

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabScenarioReport {
    pub id: &'static str,
    pub user_goal: &'static str,
    pub project_state: &'static str,
    pub expected_checkpoint: &'static str,
    pub planned_slices: Vec<&'static str>,
    pub required_child_capabilities: Vec<&'static str>,
    pub stop_conditions_exercised: Vec<&'static str>,
    pub ok: bool,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabReplayReport {
    pub workflow_id: &'static str,
    pub promotion_status: String,
    pub runtime_selectable: bool,
    pub primitive_level: u64,
    pub ok: bool,
    pub scenarios: Vec<LocalCodingProgramBuilderLabScenarioReport>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabExecutionReport {
    pub workflow_id: &'static str,
    pub harness_kind: &'static str,
    pub ok: bool,
    pub task_executions: Vec<LocalCodingProgramBuilderLabTaskExecution>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderLabTaskExecution {
    pub task_id: &'static str,
    pub user_goal: &'static str,
    pub project_state: &'static str,
    pub checkpoint: LocalCodingProgramBuilderCheckpoint,
    pub architecture_contract: LocalCodingProgramBuilderArchitectureContract,
    pub slice_invocations: Vec<LocalCodingProgramBuilderSliceInvocation>,
    pub validation_plan: Vec<&'static str>,
    pub final_handoff_fields: Vec<&'static str>,
    pub ok: bool,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderCheckpoint {
    pub name: &'static str,
    pub goal: &'static str,
    pub included_scope: Vec<&'static str>,
    pub excluded_scope: Vec<&'static str>,
    pub acceptance_criteria: Vec<&'static str>,
    pub stop_conditions: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderArchitectureContract {
    pub project_type: &'static str,
    pub stack: &'static str,
    pub runtime: &'static str,
    pub package_manager: &'static str,
    pub source_dirs: Vec<&'static str>,
    pub test_runner: &'static str,
    pub formatter: &'static str,
    pub linter: &'static str,
    pub architecture_pattern: &'static str,
    pub boundary_rules: Vec<&'static str>,
    pub validation_commands: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalCodingProgramBuilderSliceInvocation {
    pub name: &'static str,
    pub goal: &'static str,
    pub child_workflow_capability: &'static str,
    pub child_workflow_id: &'static str,
    pub input_contract: &'static str,
    pub output_artifact: &'static str,
    pub allowed_files_or_areas: Vec<&'static str>,
    pub acceptance_criteria: Vec<&'static str>,
    pub validation_expectation: &'static str,
    pub code_edit_payload_keys: Vec<&'static str>,
}

#[derive(Debug, Clone)]
struct LocalCodingProgramBuilderLabScenario {
    id: &'static str,
    user_goal: &'static str,
    project_state: &'static str,
    expected_checkpoint: &'static str,
    planned_slices: &'static [&'static str],
    required_child_capabilities: &'static [&'static str],
    stop_conditions_exercised: &'static [&'static str],
}

const LAB_SCENARIOS: &[LocalCodingProgramBuilderLabScenario] = &[
    LocalCodingProgramBuilderLabScenario {
        id: "single_file_utility",
        user_goal: "Create a single-file local checksum utility with tests.",
        project_state: "uninitialized",
        expected_checkpoint: "single_file_utility_mvp",
        planned_slices: &[
            "coding_ingress_guard_initialization",
            "session_bootstrap_guard_initialization",
            "remote_service_guard_initialization",
            "operator_integration_guard_initialization",
            "project_initialization_assessment",
            "architecture_contract_definition",
            "single_file_local_code_execution",
            "syntax_and_behavior_validation",
        ],
        required_child_capabilities: &[
            "coding_ingress_guard",
            "session_bootstrap_guard",
            "remote_service_guard",
            "operator_integration_guard",
            "policy_permission_guard",
            "context_loop_guard",
            "tooling_surface_guard",
            "runtime_execution_loop",
            "runtime_observability_guard",
            "implementation_planning",
            "plan_artifact",
            "local_code_execution",
            "focused_repair",
            "checkpoint_handoff",
        ],
        stop_conditions_exercised: &["validation fails after repair budget is exhausted"],
    },
    LocalCodingProgramBuilderLabScenario {
        id: "small_multi_file_app",
        user_goal: "Build a small multi-file task tracker with domain logic, UI/API boundary, and validation.",
        project_state: "uninitialized",
        expected_checkpoint: "bounded_multi_file_app_mvp",
        planned_slices: &[
            "coding_ingress_guard_initialization",
            "session_bootstrap_guard_initialization",
            "remote_service_guard_initialization",
            "operator_integration_guard_initialization",
            "project_initialization_assessment",
            "architecture_contract_definition",
            "domain_model_slice",
            "primary_flow_slice",
            "integration_review",
            "validation_routing",
        ],
        required_child_capabilities: &[
            "research_context",
            "coding_ingress_guard",
            "session_bootstrap_guard",
            "remote_service_guard",
            "operator_integration_guard",
            "policy_permission_guard",
            "context_loop_guard",
            "tooling_surface_guard",
            "runtime_execution_loop",
            "runtime_observability_guard",
            "implementation_planning",
            "plan_artifact",
            "local_code_execution",
            "focused_repair",
            "checkpoint_handoff",
        ],
        stop_conditions_exercised: &["request expands beyond the current checkpoint"],
    },
    LocalCodingProgramBuilderLabScenario {
        id: "initialized_project_modification",
        user_goal: "Add a feature to an existing project without changing its stack or architecture.",
        project_state: "initialized",
        expected_checkpoint: "existing_project_increment",
        planned_slices: &[
            "coding_ingress_guard_initialization",
            "session_bootstrap_guard_initialization",
            "remote_service_guard_initialization",
            "operator_integration_guard_initialization",
            "project_initialization_assessment",
            "architecture_contract_definition",
            "targeted_feature_slice",
            "integration_review",
            "architecture_drift_validation",
        ],
        required_child_capabilities: &[
            "coding_ingress_guard",
            "session_bootstrap_guard",
            "remote_service_guard",
            "operator_integration_guard",
            "policy_permission_guard",
            "context_loop_guard",
            "tooling_surface_guard",
            "runtime_execution_loop",
            "runtime_observability_guard",
            "implementation_planning",
            "plan_artifact",
            "local_code_execution",
            "focused_repair",
            "checkpoint_handoff",
        ],
        stop_conditions_exercised: &["existing project architecture conflicts with requested implementation"],
    },
];

pub fn local_coding_program_builder_lab_replay_report() -> LocalCodingProgramBuilderLabReplayReport
{
    let mut failures = Vec::new();
    let graphs = registered_workflow_graphs();
    let graph = graphs
        .iter()
        .find(|graph| graph.workflow_id == BUILDER_WORKFLOW_ID);
    let Some(graph) = graph else {
        return LocalCodingProgramBuilderLabReplayReport {
            workflow_id: BUILDER_WORKFLOW_ID,
            promotion_status: String::new(),
            runtime_selectable: false,
            primitive_level: 0,
            ok: false,
            scenarios: Vec::new(),
            failures: vec!["missing_local_coding_program_builder_graph".to_string()],
        };
    };

    if graph.workflow_tier != "lab" {
        failures.push("local_coding_program_builder_not_lab_tier".to_string());
    }
    if graph.promotion_status != "candidate" {
        failures.push("local_coding_program_builder_not_candidate".to_string());
    }
    if graph.runtime_selectable {
        failures.push("candidate_lab_workflow_must_not_be_runtime_selectable".to_string());
    }
    if graph.primitive_level != 3 {
        failures.push("local_coding_program_builder_wrong_primitive_level".to_string());
    }

    let source = read_builder_source();
    let child_capabilities = child_capabilities(&source);
    let max_slices = source
        .pointer("/program_builder_contract/slice_policy/default_max_slices")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stop_conditions = string_array_at(&source, "/program_builder_contract/stop_conditions");
    let required_state_fields =
        string_array_at(&source, "/state_tracking_contract/required_state_fields");
    let summary_fields = graph
        .final_output_contract
        .get("required_summary_fields")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    for field in [
        "checkpoint",
        "architecture_contract",
        "completed_slices",
        "validation_results",
        "files_changed",
        "next_actions",
    ] {
        if !required_state_fields.contains(field) {
            failures.push(format!("missing_state_field:{field}"));
        }
    }
    for field in [
        "completed_checkpoint",
        "changed_files",
        "validation_results",
        "known_risks",
        "intentionally_excluded_scope",
        "recommended_next_checkpoint",
    ] {
        if !summary_fields.contains(field) {
            failures.push(format!("missing_final_summary_field:{field}"));
        }
    }

    let scenarios = LAB_SCENARIOS
        .iter()
        .map(|scenario| {
            scenario_report(scenario, &child_capabilities, &stop_conditions, max_slices)
        })
        .collect::<Vec<_>>();
    let ok = failures.is_empty() && scenarios.iter().all(|scenario| scenario.ok);

    LocalCodingProgramBuilderLabReplayReport {
        workflow_id: BUILDER_WORKFLOW_ID,
        promotion_status: graph.promotion_status.clone(),
        runtime_selectable: graph.runtime_selectable,
        primitive_level: graph.primitive_level,
        ok,
        scenarios,
        failures,
    }
}

pub fn local_coding_program_builder_lab_execution_report(
) -> LocalCodingProgramBuilderLabExecutionReport {
    let replay = local_coding_program_builder_lab_replay_report();
    let source = read_builder_source();
    let child_calls = child_workflow_calls(&source);
    let final_handoff_fields =
        string_array_at_vec(&source, "/final_output_contract/required_summary_fields");

    let task_executions = LAB_SCENARIOS
        .iter()
        .map(|scenario| {
            task_execution(
                scenario,
                &child_calls,
                final_handoff_fields
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let mut failures = replay.failures;
    if !replay.ok {
        failures.push("candidate_replay_proof_failed".to_string());
    }
    let ok = failures.is_empty() && task_executions.iter().all(|execution| execution.ok);

    LocalCodingProgramBuilderLabExecutionReport {
        workflow_id: BUILDER_WORKFLOW_ID,
        harness_kind: "local_coding_program_builder_lab_execution_dry_run_v1",
        ok,
        task_executions,
        failures,
    }
}

fn scenario_report(
    scenario: &LocalCodingProgramBuilderLabScenario,
    child_capabilities: &HashSet<String>,
    stop_conditions: &HashSet<String>,
    max_slices: u64,
) -> LocalCodingProgramBuilderLabScenarioReport {
    let mut failures = Vec::new();
    if max_slices == 0 || scenario.planned_slices.len() as u64 > max_slices {
        failures.push(format!("scenario_exceeds_slice_budget:{}", scenario.id));
    }
    for capability in scenario.required_child_capabilities {
        if !child_capabilities.contains(*capability) {
            failures.push(format!(
                "scenario_missing_child_capability:{}:{capability}",
                scenario.id
            ));
        }
    }
    for stop_condition in scenario.stop_conditions_exercised {
        if !stop_conditions
            .iter()
            .any(|value| value.contains(stop_condition))
        {
            failures.push(format!(
                "scenario_missing_stop_condition:{}:{stop_condition}",
                scenario.id
            ));
        }
    }

    LocalCodingProgramBuilderLabScenarioReport {
        id: scenario.id,
        user_goal: scenario.user_goal,
        project_state: scenario.project_state,
        expected_checkpoint: scenario.expected_checkpoint,
        planned_slices: scenario.planned_slices.to_vec(),
        required_child_capabilities: scenario.required_child_capabilities.to_vec(),
        stop_conditions_exercised: scenario.stop_conditions_exercised.to_vec(),
        ok: failures.is_empty(),
        failures,
    }
}

fn task_execution(
    scenario: &LocalCodingProgramBuilderLabScenario,
    child_calls: &[ChildWorkflowCall],
    final_handoff_fields: Vec<&str>,
) -> LocalCodingProgramBuilderLabTaskExecution {
    let checkpoint = checkpoint_for_scenario(scenario);
    let architecture_contract = architecture_for_scenario(scenario);
    let slice_invocations = slices_for_scenario(scenario, child_calls);
    let validation_plan = validation_plan_for_scenario(scenario);
    let mut failures = Vec::new();

    if slice_invocations.is_empty() {
        failures.push(format!("no_slice_invocations:{}", scenario.id));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_code_edit_execution")
    {
        failures.push(format!(
            "missing_local_code_edit_execution_slice:{}",
            scenario.id
        ));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_coding_ingress_guard")
    {
        failures.push(format!(
            "missing_local_coding_ingress_guard_slice:{}",
            scenario.id
        ));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_coding_session_bootstrap_guard")
    {
        failures.push(format!(
            "missing_local_coding_session_bootstrap_guard_slice:{}",
            scenario.id
        ));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_coding_remote_service_guard")
    {
        failures.push(format!(
            "missing_local_coding_remote_service_guard_slice:{}",
            scenario.id
        ));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_coding_operator_integration_guard")
    {
        failures.push(format!(
            "missing_local_coding_operator_integration_guard_slice:{}",
            scenario.id
        ));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_runtime_execution_loop")
    {
        failures.push(format!(
            "missing_local_runtime_execution_loop_slice:{}",
            scenario.id
        ));
    }
    if !slice_invocations
        .iter()
        .any(|slice| slice.child_workflow_id == "local_runtime_observability_guard")
    {
        failures.push(format!(
            "missing_local_runtime_observability_guard_slice:{}",
            scenario.id
        ));
    }
    for key in [
        "task",
        "architecture_contract",
        "slice_acceptance_criteria",
        "allowed_files_or_areas",
        "expected_outputs",
        "validation_expectation",
        "stop_conditions",
    ] {
        if !slice_invocations.iter().all(|slice| {
            slice
                .code_edit_payload_keys
                .iter()
                .any(|value| value == &key)
        }) {
            failures.push(format!("slice_missing_payload_key:{}:{key}", scenario.id));
        }
    }
    for field in [
        "completed_checkpoint",
        "changed_files",
        "validation_results",
        "known_risks",
        "intentionally_excluded_scope",
        "recommended_next_checkpoint",
    ] {
        if !final_handoff_fields.iter().any(|value| value == &field) {
            failures.push(format!(
                "missing_final_handoff_field:{}:{field}",
                scenario.id
            ));
        }
    }

    LocalCodingProgramBuilderLabTaskExecution {
        task_id: scenario.id,
        user_goal: scenario.user_goal,
        project_state: scenario.project_state,
        checkpoint,
        architecture_contract,
        slice_invocations,
        validation_plan,
        final_handoff_fields: final_handoff_fields
            .into_iter()
            .map(|field| match field {
                "completed_checkpoint" => "completed_checkpoint",
                "changed_files" => "changed_files",
                "validation_results" => "validation_results",
                "known_risks" => "known_risks",
                "intentionally_excluded_scope" => "intentionally_excluded_scope",
                "recommended_next_checkpoint" => "recommended_next_checkpoint",
                _ => "unknown_final_handoff_field",
            })
            .collect(),
        ok: failures.is_empty(),
        failures,
    }
}

fn checkpoint_for_scenario(
    scenario: &LocalCodingProgramBuilderLabScenario,
) -> LocalCodingProgramBuilderCheckpoint {
    match scenario.id {
        "single_file_utility" => LocalCodingProgramBuilderCheckpoint {
            name: scenario.expected_checkpoint,
            goal: "Build the smallest complete utility with tests and no framework commitment.",
            included_scope: vec![
                "one executable/source file",
                "unit tests for checksum behavior",
                "local command invocation",
            ],
            excluded_scope: vec!["network IO", "package publishing", "daemon mode"],
            acceptance_criteria: vec![
                "utility computes deterministic checksums",
                "invalid input returns a structured error",
                "tests cover happy path and bad input",
            ],
            stop_conditions: vec![
                "validation fails after repair budget is exhausted",
                "dependency choice would effectively choose the product architecture",
            ],
        },
        "small_multi_file_app" => LocalCodingProgramBuilderCheckpoint {
            name: scenario.expected_checkpoint,
            goal: "Build a bounded multi-file app checkpoint with separated domain and interface layers.",
            included_scope: vec![
                "project foundation",
                "domain model",
                "primary user flow",
                "syntax and behavioral validation",
            ],
            excluded_scope: vec![
                "authentication",
                "cloud sync",
                "billing",
                "multi-user collaboration",
            ],
            acceptance_criteria: vec![
                "domain logic is isolated from interface glue",
                "primary flow works end to end",
                "validation proves the checkpoint without expanding scope",
            ],
            stop_conditions: vec![
                "request expands beyond the current checkpoint",
                "security, privacy, licensing, or data-loss risk is discovered",
            ],
        },
        _ => LocalCodingProgramBuilderCheckpoint {
            name: scenario.expected_checkpoint,
            goal: "Modify an existing project while preserving its architecture contract.",
            included_scope: vec![
                "existing stack detection",
                "targeted feature slice",
                "integration review",
                "architecture drift validation",
            ],
            excluded_scope: vec![
                "stack replacement",
                "architecture rewrite",
                "unrequested migrations",
            ],
            acceptance_criteria: vec![
                "feature follows existing project conventions",
                "no new parallel state model is introduced",
                "validation covers the changed behavior",
            ],
            stop_conditions: vec![
                "existing project architecture conflicts with requested implementation",
                "user decision required for stack, architecture, auth, persistence, payment, deployment, external API, or destructive migration",
            ],
        },
    }
}

fn architecture_for_scenario(
    scenario: &LocalCodingProgramBuilderLabScenario,
) -> LocalCodingProgramBuilderArchitectureContract {
    match scenario.id {
        "single_file_utility" => LocalCodingProgramBuilderArchitectureContract {
            project_type: "cli",
            stack: "rust-or-existing-local-default",
            runtime: "local",
            package_manager: "repo_native",
            source_dirs: vec!["src", "tests"],
            test_runner: "repo_native_test_runner",
            formatter: "repo_native_formatter",
            linter: "repo_native_linter",
            architecture_pattern: "single_entry_with_testable_core_function",
            boundary_rules: vec![
                "keep parsing separate from checksum calculation",
                "avoid global mutable state",
                "do not add external dependencies for checksum primitives",
            ],
            validation_commands: vec!["format", "unit_tests"],
        },
        "small_multi_file_app" => LocalCodingProgramBuilderArchitectureContract {
            project_type: "small_app",
            stack: "repo_native_or_minimal_stable_default",
            runtime: "local",
            package_manager: "repo_native",
            source_dirs: vec!["src/domain", "src/app", "tests"],
            test_runner: "repo_native_test_runner",
            formatter: "repo_native_formatter",
            linter: "repo_native_linter",
            architecture_pattern: "domain_first_with_interface_adapter",
            boundary_rules: vec![
                "domain logic must not depend on interface glue",
                "interface layer calls domain services through explicit functions",
                "persistence remains local and swappable for this checkpoint",
            ],
            validation_commands: vec!["format", "unit_tests", "smoke_test_primary_flow"],
        },
        _ => LocalCodingProgramBuilderArchitectureContract {
            project_type: "existing_project_increment",
            stack: "detected_existing_stack",
            runtime: "detected_existing_runtime",
            package_manager: "detected_existing_package_manager",
            source_dirs: vec!["detected_existing_source_dirs"],
            test_runner: "detected_existing_test_runner",
            formatter: "detected_existing_formatter",
            linter: "detected_existing_linter",
            architecture_pattern: "preserve_existing_architecture",
            boundary_rules: vec![
                "prefer existing naming and module boundaries",
                "do not introduce a second service layer",
                "update architecture contract before material boundary changes",
            ],
            validation_commands: vec!["targeted_existing_tests", "architecture_drift_check"],
        },
    }
}

fn slices_for_scenario(
    scenario: &LocalCodingProgramBuilderLabScenario,
    child_calls: &[ChildWorkflowCall],
) -> Vec<LocalCodingProgramBuilderSliceInvocation> {
    let plan = child_call(
        child_calls,
        "implementation_planning",
        "plan_execute_review",
    );
    let context_guard = child_call(
        child_calls,
        "context_loop_guard",
        "local_context_loop_guard",
    );
    let policy_guard = child_call(
        child_calls,
        "policy_permission_guard",
        "local_policy_permission_guard",
    );
    let tooling_guard = child_call(
        child_calls,
        "tooling_surface_guard",
        "local_tooling_surface_guard",
    );
    let runtime_loop = child_call(
        child_calls,
        "runtime_execution_loop",
        "local_runtime_execution_loop",
    );
    let observability_guard = child_call(
        child_calls,
        "runtime_observability_guard",
        "local_runtime_observability_guard",
    );
    let plan_artifact = child_call(child_calls, "plan_artifact", "plan_artifact_create");
    let code = child_call(
        child_calls,
        "local_code_execution",
        "local_code_edit_execution",
    );
    let repair = child_call(child_calls, "focused_repair", "bounded_repair_loop");
    let handoff = child_call(child_calls, "checkpoint_handoff", "checkpoint_handoff");
    let research = child_call(
        child_calls,
        "research_context",
        "research_synthesize_verify",
    );
    let ingress_guard = child_call(
        child_calls,
        "coding_ingress_guard",
        "local_coding_ingress_guard",
    );
    let session_bootstrap_guard = child_call(
        child_calls,
        "session_bootstrap_guard",
        "local_coding_session_bootstrap_guard",
    );
    let remote_service_guard = child_call(
        child_calls,
        "remote_service_guard",
        "local_coding_remote_service_guard",
    );
    let operator_integration_guard = child_call(
        child_calls,
        "operator_integration_guard",
        "local_coding_operator_integration_guard",
    );

    match scenario.id {
        "single_file_utility" => vec![
            slice(
                "coding_ingress_guard_initialization",
                "Normalize ForgeCode-style CLI/session input, command routes, and user prompt context before policy or planning.",
                ingress_guard,
                vec!["prompt source", "conversation state", "command route", "attachments"],
                vec!["ingress route is known", "prompt context receipts are available"],
                "ingress guard artifact is available before policy guard",
            ),
            slice(
                "session_bootstrap_guard_initialization",
                "Bind agent/provider/model, initialize conversation runtime state, snapshot terminal context, and detect externally changed files before policy or planning.",
                session_bootstrap_guard,
                vec!["conversation state", "agent runtime", "terminal context", "tracked file metrics"],
                vec!["session runtime state is initialized", "external file changes are receipt-backed"],
                "session bootstrap artifact is available before policy guard",
            ),
            slice(
                "remote_service_guard_initialization",
                "Route optional remote workspace auth, sync, semantic search, and provider transport through explicit boundaries before local planning.",
                remote_service_guard,
                vec!["remote workspace auth", "semantic index", "provider transport"],
                vec!["remote service use is explicit", "local file receipts remain authoritative"],
                "remote service guard artifact is available before policy guard",
            ),
            slice(
                "operator_integration_guard_initialization",
                "Route sandbox, command-surface, editor/update/OAuth, and schema-data-generation integrations through explicit operator boundaries before local planning.",
                operator_integration_guard,
                vec!["sandbox worktree", "operator commands", "editor/update/auth callbacks", "data generation"],
                vec!["operator integrations are explicit", "external-visible actions stay approval-gated"],
                "operator integration artifact is available before policy guard",
            ),
            slice(
                "policy_permission_guard_initialization",
                "Resolve ForgeCode-style config and operation permission guardrails before exposing local coding tools.",
                policy_guard,
                vec!["config layers", "permissions policy"],
                vec!["tool support is known", "restricted mode and operation policy are receipt-backed"],
                "policy guard artifact is available before context guard",
            ),
            slice(
                "context_loop_guard_initialization",
                "Resolve local coding tools and initialize context-loop guards before planning.",
                context_guard,
                vec!["agent tool catalog", "conversation context"],
                vec!["local file tools are available", "loop guards are receipt-backed"],
                "context guard artifact is available before planning",
            ),
            slice(
                "tooling_surface_guard_initialization",
                "Initialize ForgeCode-style tool schema, call normalization, MCP bridge, and command/skill inventory.",
                tooling_guard,
                vec!["tool registry", "mcp config", "commands", "skills"],
                vec!["tool schemas are explicit", "tool-call normalization is receipt-backed"],
                "tooling surface artifact is available before implementation planning",
            ),
            slice(
                "runtime_execution_loop_initialization",
                "Initialize ForgeCode-style request transform, retry streaming, tool dispatch, lifecycle hook, and conversation persistence loop.",
                runtime_loop,
                vec!["conversation context", "agent runtime", "tool results", "lifecycle hooks"],
                vec!["runtime loop receipts are explicit", "yield and interrupt conditions are bounded"],
                "runtime loop artifact is available before implementation planning",
            ),
            slice(
                "runtime_observability_guard_initialization",
                "Initialize ForgeCode-style visibility routing, streaming markdown, tool display formatting, and trace rate limiting.",
                observability_guard,
                vec!["chat responses", "markdown stream", "tool display output", "trace events"],
                vec!["visible output is separated from telemetry", "trace volume is bounded"],
                "observability guard artifact is available before implementation planning",
            ),
            slice(
                "checkpoint_and_architecture_plan",
                "Define the single-file utility checkpoint and architecture contract.",
                plan,
                vec!["project root", "src", "tests"],
                vec!["checkpoint is bounded", "validation commands are explicit"],
                "plan artifact is complete before implementation",
            ),
            slice(
                "plan_artifact_checkpoint",
                "Persist the plan as a non-overwriting checkpoint artifact before writing code.",
                plan_artifact,
                vec!["plans"],
                vec!["plan artifact is dated", "existing plans are not overwritten"],
                "plan artifact receipt is captured",
            ),
            slice(
                "single_file_utility_implementation",
                "Implement utility code and focused tests through safe local code-edit execution.",
                code,
                vec!["src/checksum.*", "tests/checksum_*"],
                vec![
                    "utility behavior matches acceptance criteria",
                    "tests cover deterministic and invalid input paths",
                ],
                "unit tests and syntax checks are expected",
            ),
            slice(
                "focused_repair_if_validation_fails",
                "Repair only the failing checksum slice if validation fails.",
                repair,
                vec!["src/checksum.*", "tests/checksum_*"],
                vec!["repair does not change checkpoint scope"],
                "repair is bounded by loop policy",
            ),
            slice(
                "checkpoint_handoff",
                "Package completed checkpoint, changed files, validation receipts, risks, and next checkpoint.",
                handoff,
                vec!["final response artifact"],
                vec!["handoff distinguishes completed scope from excluded scope"],
                "handoff receipt is complete",
            ),
        ],
        "small_multi_file_app" => vec![
            slice(
                "coding_ingress_guard_initialization",
                "Normalize ForgeCode-style CLI/session input, command routes, and user prompt context before policy or planning.",
                ingress_guard,
                vec!["prompt source", "conversation state", "command route", "attachments"],
                vec!["ingress route is known", "prompt context receipts are available"],
                "ingress guard artifact is available before policy guard",
            ),
            slice(
                "session_bootstrap_guard_initialization",
                "Bind agent/provider/model, initialize conversation runtime state, snapshot terminal context, and detect externally changed files before policy or planning.",
                session_bootstrap_guard,
                vec!["conversation state", "agent runtime", "terminal context", "tracked file metrics"],
                vec!["session runtime state is initialized", "external file changes are receipt-backed"],
                "session bootstrap artifact is available before policy guard",
            ),
            slice(
                "remote_service_guard_initialization",
                "Route optional remote workspace auth, sync, semantic search, and provider transport through explicit boundaries before local planning.",
                remote_service_guard,
                vec!["remote workspace auth", "semantic index", "provider transport"],
                vec!["remote service use is explicit", "local file receipts remain authoritative"],
                "remote service guard artifact is available before policy guard",
            ),
            slice(
                "operator_integration_guard_initialization",
                "Route sandbox, command-surface, editor/update/OAuth, and schema-data-generation integrations through explicit operator boundaries before local planning.",
                operator_integration_guard,
                vec!["sandbox worktree", "operator commands", "editor/update/auth callbacks", "data generation"],
                vec!["operator integrations are explicit", "external-visible actions stay approval-gated"],
                "operator integration artifact is available before policy guard",
            ),
            slice(
                "policy_permission_guard_initialization",
                "Resolve ForgeCode-style config and operation permission guardrails before exposing local coding tools.",
                policy_guard,
                vec!["config layers", "permissions policy"],
                vec!["tool support is known", "restricted mode and operation policy are receipt-backed"],
                "policy guard artifact is available before context guard",
            ),
            slice(
                "context_loop_guard_initialization",
                "Resolve local coding tools and initialize context-loop guards before planning.",
                context_guard,
                vec!["agent tool catalog", "conversation context"],
                vec!["local file tools are available", "loop guards are receipt-backed"],
                "context guard artifact is available before planning",
            ),
            slice(
                "tooling_surface_guard_initialization",
                "Initialize ForgeCode-style tool schema, call normalization, MCP bridge, and command/skill inventory.",
                tooling_guard,
                vec!["tool registry", "mcp config", "commands", "skills"],
                vec!["tool schemas are explicit", "tool-call normalization is receipt-backed"],
                "tooling surface artifact is available before implementation planning",
            ),
            slice(
                "runtime_execution_loop_initialization",
                "Initialize ForgeCode-style request transform, retry streaming, tool dispatch, lifecycle hook, and conversation persistence loop.",
                runtime_loop,
                vec!["conversation context", "agent runtime", "tool results", "lifecycle hooks"],
                vec!["runtime loop receipts are explicit", "yield and interrupt conditions are bounded"],
                "runtime loop artifact is available before implementation planning",
            ),
            slice(
                "runtime_observability_guard_initialization",
                "Initialize ForgeCode-style visibility routing, streaming markdown, tool display formatting, and trace rate limiting.",
                observability_guard,
                vec!["chat responses", "markdown stream", "tool display output", "trace events"],
                vec!["visible output is separated from telemetry", "trace volume is bounded"],
                "observability guard artifact is available before implementation planning",
            ),
            slice(
                "context_research",
                "Inspect project context or defaults before choosing the app foundation.",
                research,
                vec!["project root", "package files", "existing docs"],
                vec!["stack choice is justified or safely inferred"],
                "context artifact is available before planning",
            ),
            slice(
                "architecture_and_slice_plan",
                "Define bounded MVP checkpoint, architecture contract, and implementation slices.",
                plan,
                vec!["project root", "src", "tests"],
                vec!["domain and interface boundaries are explicit"],
                "plan artifact contains slice acceptance criteria",
            ),
            slice(
                "plan_artifact_checkpoint",
                "Persist the bounded MVP plan as a non-overwriting checkpoint artifact.",
                plan_artifact,
                vec!["plans"],
                vec!["domain/interface slice plan", "validation plan"],
                "plan artifact receipt is captured",
            ),
            slice(
                "domain_model_slice",
                "Implement domain model and tests through safe local code-edit execution.",
                code,
                vec!["src/domain", "tests/domain"],
                vec!["domain logic is isolated", "domain tests pass"],
                "unit tests expected",
            ),
            slice(
                "primary_flow_slice",
                "Implement the primary app flow through safe local code-edit execution without expanding scope.",
                code,
                vec!["src/app", "src/interface", "tests/app"],
                vec!["primary flow works end to end"],
                "integration or smoke test expected",
            ),
            slice(
                "integration_repair_if_needed",
                "Repair integration failures without rewriting prior slices.",
                repair,
                vec!["src/domain", "src/app", "tests"],
                vec!["interfaces compose correctly"],
                "repair remains inside current checkpoint",
            ),
            slice(
                "checkpoint_handoff",
                "Package completed checkpoint, changed files, validation receipts, risks, and next checkpoint.",
                handoff,
                vec!["final response artifact"],
                vec!["handoff distinguishes completed scope from excluded scope"],
                "handoff receipt is complete",
            ),
        ],
        _ => vec![
            slice(
                "coding_ingress_guard_initialization",
                "Normalize ForgeCode-style CLI/session input, command routes, and user prompt context before policy or planning.",
                ingress_guard,
                vec!["prompt source", "conversation state", "command route", "attachments"],
                vec!["ingress route is known", "prompt context receipts are available"],
                "ingress guard artifact is available before policy guard",
            ),
            slice(
                "session_bootstrap_guard_initialization",
                "Bind agent/provider/model, initialize conversation runtime state, snapshot terminal context, and detect externally changed files before policy or planning.",
                session_bootstrap_guard,
                vec!["conversation state", "agent runtime", "terminal context", "tracked file metrics"],
                vec!["session runtime state is initialized", "external file changes are receipt-backed"],
                "session bootstrap artifact is available before policy guard",
            ),
            slice(
                "remote_service_guard_initialization",
                "Route optional remote workspace auth, sync, semantic search, and provider transport through explicit boundaries before local planning.",
                remote_service_guard,
                vec!["remote workspace auth", "semantic index", "provider transport"],
                vec!["remote service use is explicit", "local file receipts remain authoritative"],
                "remote service guard artifact is available before policy guard",
            ),
            slice(
                "operator_integration_guard_initialization",
                "Route sandbox, command-surface, editor/update/OAuth, and schema-data-generation integrations through explicit operator boundaries before local planning.",
                operator_integration_guard,
                vec!["sandbox worktree", "operator commands", "editor/update/auth callbacks", "data generation"],
                vec!["operator integrations are explicit", "external-visible actions stay approval-gated"],
                "operator integration artifact is available before policy guard",
            ),
            slice(
                "policy_permission_guard_initialization",
                "Resolve ForgeCode-style config and operation permission guardrails before exposing local coding tools.",
                policy_guard,
                vec!["config layers", "permissions policy"],
                vec!["tool support is known", "restricted mode and operation policy are receipt-backed"],
                "policy guard artifact is available before context guard",
            ),
            slice(
                "context_loop_guard_initialization",
                "Resolve local coding tools and initialize context-loop guards before planning.",
                context_guard,
                vec!["agent tool catalog", "conversation context"],
                vec!["local file tools are available", "loop guards are receipt-backed"],
                "context guard artifact is available before planning",
            ),
            slice(
                "tooling_surface_guard_initialization",
                "Initialize ForgeCode-style tool schema, call normalization, MCP bridge, and command/skill inventory.",
                tooling_guard,
                vec!["tool registry", "mcp config", "commands", "skills"],
                vec!["tool schemas are explicit", "tool-call normalization is receipt-backed"],
                "tooling surface artifact is available before implementation planning",
            ),
            slice(
                "runtime_execution_loop_initialization",
                "Initialize ForgeCode-style request transform, retry streaming, tool dispatch, lifecycle hook, and conversation persistence loop.",
                runtime_loop,
                vec!["conversation context", "agent runtime", "tool results", "lifecycle hooks"],
                vec!["runtime loop receipts are explicit", "yield and interrupt conditions are bounded"],
                "runtime loop artifact is available before implementation planning",
            ),
            slice(
                "runtime_observability_guard_initialization",
                "Initialize ForgeCode-style visibility routing, streaming markdown, tool display formatting, and trace rate limiting.",
                observability_guard,
                vec!["chat responses", "markdown stream", "tool display output", "trace events"],
                vec!["visible output is separated from telemetry", "trace volume is bounded"],
                "observability guard artifact is available before implementation planning",
            ),
            slice(
                "existing_project_assessment",
                "Detect existing stack, boundaries, and validation commands.",
                plan,
                vec![
                    "project root",
                    "existing source dirs",
                    "package/config files",
                ],
                vec!["existing architecture contract is captured"],
                "assessment artifact is complete before implementation",
            ),
            slice(
                "plan_artifact_checkpoint",
                "Persist the existing-project increment plan as a non-overwriting checkpoint artifact.",
                plan_artifact,
                vec!["plans"],
                vec!["existing stack assumptions are captured"],
                "plan artifact receipt is captured",
            ),
            slice(
                "targeted_feature_implementation",
                "Implement only the requested feature through safe local code-edit execution.",
                code,
                vec!["detected feature module", "associated tests"],
                vec![
                    "feature follows existing conventions",
                    "no unrequested architecture change",
                ],
                "targeted tests expected",
            ),
            slice(
                "architecture_drift_repair",
                "Repair drift or validation failure without broad refactor.",
                repair,
                vec!["changed feature module", "associated tests"],
                vec!["project remains at least as coherent as before"],
                "architecture drift check expected",
            ),
            slice(
                "checkpoint_handoff",
                "Package completed checkpoint, changed files, validation receipts, risks, and next checkpoint.",
                handoff,
                vec!["final response artifact"],
                vec!["handoff distinguishes completed scope from excluded scope"],
                "handoff receipt is complete",
            ),
        ],
    }
}

fn slice(
    name: &'static str,
    goal: &'static str,
    child_call: ChildWorkflowCall,
    allowed_files_or_areas: Vec<&'static str>,
    acceptance_criteria: Vec<&'static str>,
    validation_expectation: &'static str,
) -> LocalCodingProgramBuilderSliceInvocation {
    LocalCodingProgramBuilderSliceInvocation {
        name,
        goal,
        child_workflow_capability: child_call.capability,
        child_workflow_id: child_call.workflow_id,
        input_contract: child_call.input_contract,
        output_artifact: child_call.output_artifact,
        allowed_files_or_areas,
        acceptance_criteria,
        validation_expectation,
        code_edit_payload_keys: vec![
            "task",
            "architecture_contract",
            "slice_acceptance_criteria",
            "allowed_files_or_areas",
            "expected_outputs",
            "validation_expectation",
            "stop_conditions",
        ],
    }
}

fn validation_plan_for_scenario(
    scenario: &LocalCodingProgramBuilderLabScenario,
) -> Vec<&'static str> {
    match scenario.id {
        "single_file_utility" => vec!["syntax", "behavioral"],
        "small_multi_file_app" => vec![
            "syntax",
            "behavioral",
            "product_smoke",
            "architecture_drift",
        ],
        _ => vec!["syntax", "behavioral", "architecture_drift"],
    }
}

fn read_builder_source() -> Value {
    let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json",
    );
    let raw = std::fs::read_to_string(source_path).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&raw).unwrap_or(Value::Null)
}

fn child_capabilities(source: &Value) -> HashSet<String> {
    source
        .pointer("/workflow_composition_contract/child_workflow_calls")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("capability").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy)]
struct ChildWorkflowCall {
    capability: &'static str,
    workflow_id: &'static str,
    input_contract: &'static str,
    output_artifact: &'static str,
}

fn child_workflow_calls(source: &Value) -> Vec<ChildWorkflowCall> {
    source
        .pointer("/workflow_composition_contract/child_workflow_calls")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(ChildWorkflowCall {
                        capability: static_child_value(
                            item.get("capability").and_then(Value::as_str)?,
                        ),
                        workflow_id: static_child_value(
                            item.get("workflow_id").and_then(Value::as_str)?,
                        ),
                        input_contract: static_child_value(
                            item.get("input_contract").and_then(Value::as_str)?,
                        ),
                        output_artifact: static_child_value(
                            item.get("output_artifact").and_then(Value::as_str)?,
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn child_call(
    child_calls: &[ChildWorkflowCall],
    capability: &'static str,
    fallback_workflow_id: &'static str,
) -> ChildWorkflowCall {
    child_calls
        .iter()
        .find(|call| call.capability == capability)
        .copied()
        .unwrap_or(ChildWorkflowCall {
            capability,
            workflow_id: fallback_workflow_id,
            input_contract: "workflow_input_envelope_v1",
            output_artifact: "missing_child_call_artifact",
        })
}

fn static_child_value(value: &str) -> &'static str {
    match value {
        "research_context" => "research_context",
        "coding_ingress_guard" => "coding_ingress_guard",
        "session_bootstrap_guard" => "session_bootstrap_guard",
        "remote_service_guard" => "remote_service_guard",
        "operator_integration_guard" => "operator_integration_guard",
        "policy_permission_guard" => "policy_permission_guard",
        "context_loop_guard" => "context_loop_guard",
        "tooling_surface_guard" => "tooling_surface_guard",
        "runtime_execution_loop" => "runtime_execution_loop",
        "runtime_observability_guard" => "runtime_observability_guard",
        "implementation_planning" => "implementation_planning",
        "plan_artifact" => "plan_artifact",
        "local_code_execution" => "local_code_execution",
        "focused_repair" => "focused_repair",
        "checkpoint_handoff" => "checkpoint_handoff",
        "research_synthesize_verify" => "research_synthesize_verify",
        "local_coding_ingress_guard" => "local_coding_ingress_guard",
        "local_coding_ingress_guard_input_envelope_v1" => {
            "local_coding_ingress_guard_input_envelope_v1"
        }
        "local_coding_ingress_guard_result_artifact_v1" => {
            "local_coding_ingress_guard_result_artifact_v1"
        }
        "local_coding_session_bootstrap_guard" => "local_coding_session_bootstrap_guard",
        "local_coding_session_bootstrap_guard_input_envelope_v1" => {
            "local_coding_session_bootstrap_guard_input_envelope_v1"
        }
        "local_coding_session_bootstrap_guard_result_artifact_v1" => {
            "local_coding_session_bootstrap_guard_result_artifact_v1"
        }
        "local_coding_remote_service_guard" => "local_coding_remote_service_guard",
        "local_coding_remote_service_guard_input_envelope_v1" => {
            "local_coding_remote_service_guard_input_envelope_v1"
        }
        "local_coding_remote_service_guard_result_artifact_v1" => {
            "local_coding_remote_service_guard_result_artifact_v1"
        }
        "local_coding_operator_integration_guard" => "local_coding_operator_integration_guard",
        "local_coding_operator_integration_guard_input_envelope_v1" => {
            "local_coding_operator_integration_guard_input_envelope_v1"
        }
        "local_coding_operator_integration_guard_result_artifact_v1" => {
            "local_coding_operator_integration_guard_result_artifact_v1"
        }
        "local_policy_permission_guard" => "local_policy_permission_guard",
        "local_policy_permission_guard_input_envelope_v1" => {
            "local_policy_permission_guard_input_envelope_v1"
        }
        "local_policy_permission_guard_result_artifact_v1" => {
            "local_policy_permission_guard_result_artifact_v1"
        }
        "local_context_loop_guard" => "local_context_loop_guard",
        "local_context_loop_guard_input_envelope_v1" => {
            "local_context_loop_guard_input_envelope_v1"
        }
        "local_context_loop_guard_result_artifact_v1" => {
            "local_context_loop_guard_result_artifact_v1"
        }
        "local_tooling_surface_guard" => "local_tooling_surface_guard",
        "local_tooling_surface_guard_input_envelope_v1" => {
            "local_tooling_surface_guard_input_envelope_v1"
        }
        "local_tooling_surface_guard_result_artifact_v1" => {
            "local_tooling_surface_guard_result_artifact_v1"
        }
        "local_runtime_execution_loop" => "local_runtime_execution_loop",
        "local_runtime_execution_loop_input_envelope_v1" => {
            "local_runtime_execution_loop_input_envelope_v1"
        }
        "local_runtime_execution_loop_result_artifact_v1" => {
            "local_runtime_execution_loop_result_artifact_v1"
        }
        "local_runtime_observability_guard" => "local_runtime_observability_guard",
        "local_runtime_observability_guard_input_envelope_v1" => {
            "local_runtime_observability_guard_input_envelope_v1"
        }
        "local_runtime_observability_guard_result_artifact_v1" => {
            "local_runtime_observability_guard_result_artifact_v1"
        }
        "plan_execute_review" => "plan_execute_review",
        "plan_artifact_create" => "plan_artifact_create",
        "plan_artifact_create_input_envelope_v1" => "plan_artifact_create_input_envelope_v1",
        "plan_artifact_create_result_artifact_v1" => "plan_artifact_create_result_artifact_v1",
        "local_code_edit_execution" => "local_code_edit_execution",
        "local_code_edit_slice_input_envelope_v1" => "local_code_edit_slice_input_envelope_v1",
        "local_code_edit_execution_result_artifact_v1" => {
            "local_code_edit_execution_result_artifact_v1"
        }
        "bounded_repair_loop" => "bounded_repair_loop",
        "bounded_repair_loop_input_envelope_v1" => "bounded_repair_loop_input_envelope_v1",
        "bounded_repair_loop_result_artifact_v1" => "bounded_repair_loop_result_artifact_v1",
        "checkpoint_handoff_input_envelope_v1" => "checkpoint_handoff_input_envelope_v1",
        "checkpoint_handoff_result_artifact_v1" => "checkpoint_handoff_result_artifact_v1",
        "workflow_input_envelope_v1" => "workflow_input_envelope_v1",
        "research_result_artifact_v1" => "research_result_artifact_v1",
        "plan_result_artifact_v1" => "plan_result_artifact_v1",
        _ => "unknown_child_workflow_value",
    }
}

fn string_array_at(source: &Value, pointer: &str) -> HashSet<String> {
    source
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn string_array_at_vec(source: &Value, pointer: &str) -> Vec<String> {
    source
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
