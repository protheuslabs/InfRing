// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::agent::{AgentBuildError, AgentBuilder, AgentExecutionContext, AgentRunResult};
use crate::capability_pack::CapabilityPackCatalog;
use crate::merkle_receipt::{merkle_receipt_options_from_value, merkle_receipt_payload};
use crate::native_tools::{
    parse_native_tool_calls, NativeToolCall, NativeToolDispatcher, NativeToolReceipt,
};
use crate::native_workflow_artifact::native_tool_auto_workflow_artifact_receipts;
use crate::provider::{ProviderClientRegistry, ProviderError, ProviderRequest, ProviderResponse};
use crate::rbac_memory::{
    memory_read_allowed, memory_write_allowed, permission_for, permission_manifest_from_value,
    permission_manifest_from_value_with_inheritance, permission_manifest_snapshot, PermissionTrit,
};
use crate::realtime_voice::{normalize_voice_session_request, voice_session_contract};
use crate::runtime_state::{
    runtime_lane_state_load, runtime_lane_state_mark_schedule_failure,
    runtime_lane_state_mark_schedule_success, runtime_lane_state_path,
    runtime_lane_state_record_denied_action, runtime_lane_state_record_merkle_continuity_failure,
    runtime_lane_state_release_gate_counters, runtime_lane_state_save, RuntimeLaneDurableState,
};
use crate::scheduler::SchedulePlan;
use crate::telemetry::ReceiptSpan;
use crate::wasm_sandbox::{
    evaluate_wasm_execution_boundary, evaluate_wasm_policy, wasm_policy_from_value,
    wasm_policy_snapshot, WasmPolicyDecision,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeLaneRequest {
    pub name: String,
    pub preamble: Option<String>,
    pub initial_prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub capability_packs: Vec<String>,
    pub lifespan_seconds: Option<u64>,
    pub metadata: Value,
    #[serde(default)]
    pub permissions_manifest: Option<Value>,
    #[serde(default)]
    pub wasm_sandbox: Option<Value>,
    #[serde(default)]
    pub voice_session: Option<Value>,
    #[serde(default)]
    pub receipt_merkle: Option<Value>,
    #[serde(default)]
    pub previous_receipt_root: Option<String>,
    #[serde(default)]
    pub schedule_interval_seconds: Option<u64>,
    #[serde(default)]
    pub schedule_max_runs: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeLaneResponse {
    pub ok: bool,
    pub contract: Value,
    pub receipt: Value,
    pub trace_summary: Value,
    pub output: String,
    pub error: Option<String>,
}

fn runtime_lane_response_allows_bounded_existing_project_fallback(
    response: &RuntimeLaneResponse,
) -> bool {
    matches!(
        response.error.as_deref(),
        Some("runtime_lane_bounded_existing_project_edit_loop_manifest_failed")
            | Some("runtime_lane_bounded_existing_project_edit_loop_manifest_repair_failed")
    )
}

fn runtime_lane_metadata_string_list(metadata: &Value, pointers: &[&str]) -> Option<Vec<String>> {
    for pointer in pointers {
        let Some(values) = metadata.pointer(pointer).and_then(Value::as_array) else {
            continue;
        };
        return Some(
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        );
    }
    None
}

fn runtime_lane_metadata_string_list_allows(
    metadata: &Value,
    pointers: &[&str],
    value: &str,
) -> Option<bool> {
    runtime_lane_metadata_string_list(metadata, pointers)
        .map(|values| values.iter().any(|entry| entry == "*" || entry == value))
}

#[derive(Debug)]
pub enum RuntimeLaneError {
    Build(AgentBuildError),
    Provider(ProviderError),
}

impl std::fmt::Display for RuntimeLaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build(error) => write!(f, "build:{}", error),
            Self::Provider(error) => write!(f, "provider:{}", error.message),
        }
    }
}

impl std::error::Error for RuntimeLaneError {}

pub fn run_runtime_lane(
    request: RuntimeLaneRequest,
) -> Result<RuntimeLaneResponse, RuntimeLaneError> {
    let providers = ProviderClientRegistry::with_builtin();
    run_runtime_lane_with_registry(request, &providers)
}

pub fn run_runtime_lane_with_registry(
    request: RuntimeLaneRequest,
    providers: &ProviderClientRegistry,
) -> Result<RuntimeLaneResponse, RuntimeLaneError> {
    let RuntimeLaneRequest {
        name,
        preamble,
        initial_prompt,
        provider,
        model,
        tools,
        capability_packs,
        lifespan_seconds,
        metadata,
        permissions_manifest,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
        schedule_interval_seconds,
        schedule_max_runs,
    } = request;

    let pre_agent_setup_started = Instant::now();
    let state_load_started = Instant::now();
    let state_path = runtime_lane_state_path(&metadata);
    let mut durable_state = runtime_lane_state_load(&state_path);
    let state_load_ms = state_load_started.elapsed().as_millis() as u64;
    let permission_setup_started = Instant::now();
    let parent_permissions_manifest =
        permission_manifest_from_value(metadata.get("parent_permissions_manifest"));
    let permissions_template = metadata
        .get("permissions_template")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let permissions = permission_manifest_from_value_with_inheritance(
        permissions_manifest.as_ref(),
        permissions_template,
        Some(&parent_permissions_manifest),
    );
    let parent_permissions_snapshot = permission_manifest_snapshot(&parent_permissions_manifest);
    let parent_permissions_manifest_present = parent_permissions_snapshot
        .get("grants")
        .and_then(Value::as_object)
        .map(|grants| !grants.is_empty())
        .unwrap_or(false);
    let effective_permissions_snapshot = permission_manifest_snapshot(&permissions);
    let parent_permissions_patch_clamped = metadata
        .get("parent_permissions_patch_clamped")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let permission_setup_ms = permission_setup_started.elapsed().as_millis() as u64;
    let permission_gate_started = Instant::now();
    let catalog = CapabilityPackCatalog::new();
    let required_pack_permissions = catalog.required_permissions_for_packs(&capability_packs);
    for permission in &required_pack_permissions {
        let state = permission_for(&permissions, permission);
        if state != PermissionTrit::Allow {
            let effective_state = permission_trit_code(state);
            let parent_state =
                permission_trit_code(permission_for(&parent_permissions_manifest, permission));
            return Ok(runtime_lane_fail_closed_with_state(
                "runtime_lane_pack_permission_denied",
                json!({
                    "permission": permission,
                    "permission_state": effective_state,
                    "enforcement_mode": "strict_fail_closed",
                    "blocked_permission_key_lineage": {
                        "permission": permission,
                        "effective_state": effective_state,
                        "parent_state": parent_state,
                        "lineage_chain": [
                            {"source": "effective_manifest", "state": effective_state},
                            {"source": "parent_manifest", "state": parent_state}
                        ]
                    },
                    "parent_permissions_manifest_present": parent_permissions_manifest_present,
                    "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                    "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                    "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
                }),
                &permissions,
                wasm_sandbox.as_ref(),
                voice_session.as_ref(),
                &state_path,
                &mut durable_state,
            ));
        }
    }
    if tools.iter().any(|tool| tool == "memory.read") && !memory_read_allowed(&permissions) {
        let effective_state = permission_trit_code(permission_for(&permissions, "memory.read"));
        let parent_state =
            permission_trit_code(permission_for(&parent_permissions_manifest, "memory.read"));
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_memory_read_denied",
            json!({
                "permission": "memory.read",
                "permission_state": effective_state,
                "enforcement_mode": "strict_fail_closed",
                "blocked_permission_key_lineage": {
                    "permission": "memory.read",
                    "effective_state": effective_state,
                    "parent_state": parent_state,
                    "lineage_chain": [
                        {"source": "effective_manifest", "state": effective_state},
                        {"source": "parent_manifest", "state": parent_state}
                    ]
                },
                "parent_permissions_manifest_present": parent_permissions_manifest_present,
                "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if tools.iter().any(|tool| tool == "memory.write") && !memory_write_allowed(&permissions) {
        let effective_state = permission_trit_code(permission_for(&permissions, "memory.write"));
        let parent_state =
            permission_trit_code(permission_for(&parent_permissions_manifest, "memory.write"));
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_memory_write_denied",
            json!({
                "permission": "memory.write",
                "permission_state": effective_state,
                "enforcement_mode": "strict_fail_closed",
                "blocked_permission_key_lineage": {
                    "permission": "memory.write",
                    "effective_state": effective_state,
                    "parent_state": parent_state,
                    "lineage_chain": [
                        {"source": "effective_manifest", "state": effective_state},
                        {"source": "parent_manifest", "state": parent_state}
                    ]
                },
                "parent_permissions_manifest_present": parent_permissions_manifest_present,
                "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if let Some((tool, permission)) = tools
        .iter()
        .filter_map(|tool| file_tool_permission(tool).map(|permission| (tool, permission)))
        .find(|(_, permission)| permission_for(&permissions, permission) != PermissionTrit::Allow)
    {
        let effective_state = permission_trit_code(permission_for(&permissions, permission));
        let parent_state =
            permission_trit_code(permission_for(&parent_permissions_manifest, permission));
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_file_tool_permission_denied",
            json!({
                "tool": tool,
                "permission": permission,
                "permission_state": effective_state,
                "enforcement_mode": "strict_fail_closed",
                "blocked_permission_key_lineage": {
                    "permission": permission,
                    "effective_state": effective_state,
                    "parent_state": parent_state,
                    "lineage_chain": [
                        {"source": "effective_manifest", "state": effective_state},
                        {"source": "parent_manifest", "state": parent_state}
                    ]
                },
                "parent_permissions_manifest_present": parent_permissions_manifest_present,
                "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    let permission_gate_ms = permission_gate_started.elapsed().as_millis() as u64;

    let wasm_policy_started = Instant::now();
    let wasm_policy = wasm_policy_from_value(wasm_sandbox.as_ref());
    let requested_modules = runtime_requested_wasm_modules(&tools, &metadata);
    let requests_network = runtime_requests_network(&tools, &metadata);
    match evaluate_wasm_policy(&wasm_policy, &requested_modules, requests_network) {
        WasmPolicyDecision::Allowed => {}
        WasmPolicyDecision::Blocked(error_code) => {
            return Ok(runtime_lane_fail_closed_with_state(
                &error_code,
                json!({
                    "requested_modules": requested_modules,
                    "requests_network": requests_network
                }),
                &permissions,
                wasm_sandbox.as_ref(),
                voice_session.as_ref(),
                &state_path,
                &mut durable_state,
            ));
        }
    }

    let voice_request = normalize_voice_session_request(voice_session.as_ref());
    if voice_session.is_some() && voice_request.is_none() {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_voice_contract_invalid",
            json!({"voice_session": voice_session}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    let wasm_policy_ms = wasm_policy_started.elapsed().as_millis() as u64;

    let direct_mutation_probe_started = Instant::now();
    if let Some(response) = runtime_lane_try_direct_mutation(
        &name,
        &initial_prompt,
        &metadata,
        &tools,
        &capability_packs,
        &required_pack_permissions,
        &permissions,
        wasm_sandbox.as_ref(),
        voice_session.as_ref(),
        receipt_merkle.as_ref(),
        previous_receipt_root.as_ref(),
        &state_path,
        &mut durable_state,
    ) {
        return Ok(response);
    }
    let direct_mutation_probe_ms = direct_mutation_probe_started.elapsed().as_millis() as u64;

    let deterministic_local_loop_probe_started = Instant::now();
    if let Some(response) = runtime_lane_try_deterministic_local_loop(
        &name,
        &initial_prompt,
        &metadata,
        &tools,
        &capability_packs,
        &required_pack_permissions,
        &permissions,
        wasm_sandbox.as_ref(),
        voice_session.as_ref(),
        receipt_merkle.as_ref(),
        previous_receipt_root.as_ref(),
        &state_path,
        &mut durable_state,
    ) {
        return Ok(response);
    }
    let deterministic_local_loop_probe_ms =
        deterministic_local_loop_probe_started.elapsed().as_millis() as u64;

    let first_edit_tool_calls_probe_started = Instant::now();
    if let Some(response) = runtime_lane_try_first_edit_tool_calls_lane(
        &name,
        &initial_prompt,
        provider.as_deref(),
        model.as_ref(),
        &metadata,
        &tools,
        &capability_packs,
        &required_pack_permissions,
        &permissions,
        wasm_sandbox.as_ref(),
        voice_session.as_ref(),
        receipt_merkle.as_ref(),
        previous_receipt_root.as_ref(),
        &state_path,
        &mut durable_state,
        providers,
    ) {
        return Ok(response);
    }
    let first_edit_tool_calls_probe_ms =
        first_edit_tool_calls_probe_started.elapsed().as_millis() as u64;
    let mut agent_metadata = metadata.clone();
    let first_edit_tool_calls_demotion_marker_ms = runtime_lane_metadata_u64_value(
        &metadata,
        &[
            "/native_success_criteria/first_edit_tool_calls_demotion_marker_ms",
            "/workflow/native_success_criteria/first_edit_tool_calls_demotion_marker_ms",
        ],
    )
    .unwrap_or(10_000)
    .clamp(1_000, 60_000);
    if first_edit_tool_calls_probe_ms >= first_edit_tool_calls_demotion_marker_ms {
        runtime_lane_set_native_success_criteria_value(
            &mut agent_metadata,
            "first_edit_tool_calls_demoted_after_provider_budget",
            json!(true),
        );
        runtime_lane_set_native_success_criteria_value(
            &mut agent_metadata,
            "first_edit_tool_calls_demoted_probe_ms",
            json!(first_edit_tool_calls_probe_ms),
        );
    }

    let public_api_extension_probe_started = Instant::now();
    if let Some(response) = runtime_lane_try_public_api_extension_lane(
        &name,
        &initial_prompt,
        provider.as_deref(),
        model.as_ref(),
        &metadata,
        &tools,
        &capability_packs,
        &required_pack_permissions,
        &permissions,
        wasm_sandbox.as_ref(),
        voice_session.as_ref(),
        receipt_merkle.as_ref(),
        previous_receipt_root.as_ref(),
        &state_path,
        &mut durable_state,
        providers,
    ) {
        return Ok(response);
    }
    let public_api_extension_probe_ms = public_api_extension_probe_started.elapsed().as_millis() as u64;

    let checkpointed_tool_loop_selected =
        runtime_lane_should_skip_model_manifest_for_checkpointed_tool_loop(
            &metadata,
            &initial_prompt,
        );

    let bounded_existing_project_probe_started = Instant::now();
    let mut bounded_existing_project_fallback_probe = None;
    if !checkpointed_tool_loop_selected {
        if let Some(response) = runtime_lane_try_bounded_existing_project_edit_loop(
            &name,
            &initial_prompt,
            provider.as_deref(),
            model.as_ref(),
            &metadata,
            &tools,
            &capability_packs,
            &required_pack_permissions,
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            receipt_merkle.as_ref(),
            previous_receipt_root.as_ref(),
            &state_path,
            &mut durable_state,
            providers,
        ) {
            if !runtime_lane_response_allows_bounded_existing_project_fallback(&response) {
                return Ok(response);
            }
            bounded_existing_project_fallback_probe = Some(json!({
                "fallback_source": "bounded_existing_project_edit_loop",
                "discarded_error": response.error,
                "discarded_receipt_status": response.receipt.get("status").cloned().unwrap_or(Value::Null),
                "discarded_receipt_details": response.receipt.get("details").cloned().unwrap_or(Value::Null),
                "discarded_coding_runtime_probe": response.receipt.get("coding_runtime_probe").cloned().unwrap_or(Value::Null),
            }));
        }
    }
    let bounded_existing_project_probe_ms =
        bounded_existing_project_probe_started.elapsed().as_millis() as u64;

    let model_manifest_probe_started = Instant::now();
    let model_manifest_probe_skipped = checkpointed_tool_loop_selected;
    if !model_manifest_probe_skipped {
        if let Some(mut response) = runtime_lane_try_model_manifest_planner(
            &name,
            &initial_prompt,
            preamble.as_deref(),
            provider.as_deref(),
            model.as_ref(),
            &metadata,
            &tools,
            &capability_packs,
            &required_pack_permissions,
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            receipt_merkle.as_ref(),
            previous_receipt_root.as_ref(),
            &state_path,
            &mut durable_state,
            providers,
        ) {
            if let Some(probe) = bounded_existing_project_fallback_probe {
                runtime_lane_attach_coding_runtime_probe(&mut response, probe);
            }
            return Ok(response);
        }
    }
    let model_manifest_probe_ms = model_manifest_probe_started.elapsed().as_millis() as u64;

    let agent_builder_started = Instant::now();
    let merkle_options = merkle_receipt_options_from_value(receipt_merkle.as_ref());
    let bootstrap_receipts = if model_manifest_probe_skipped {
        runtime_lane_checkpointed_bootstrap_receipts(&initial_prompt)
    } else {
        Vec::new()
    };
    if model_manifest_probe_skipped {
        return Ok(runtime_lane_try_checkpointed_pre_agent_entry(
            &name,
            &initial_prompt,
        provider.as_deref(),
        model.as_ref(),
        &agent_metadata,
        &tools,
        &bootstrap_receipts,
        providers,
    ));
    }
    let agent_initial_prompt = if model_manifest_probe_skipped {
        runtime_lane_checkpointed_tool_loop_prompt(&initial_prompt, &bootstrap_receipts)
    } else {
        initial_prompt.clone()
    };
    let agent_initial_prompt =
        runtime_lane_apply_agent_loop_public_api_contract_prompt(&agent_metadata, agent_initial_prompt);
    let checkpointed_recovery_provider = provider.clone();
    let checkpointed_recovery_model = model.clone();
    let mut builder = AgentBuilder::new(name)
        .initial_prompt(agent_initial_prompt)
        .metadata(agent_metadata.clone());
    if let Some(value) = preamble {
        builder = builder.preamble(value);
    }
    if let Some(value) = provider {
        builder = builder.provider(value);
    }
    if let Some(value) = model {
        builder = builder.model(value);
    }
    if let Some(value) = lifespan_seconds {
        builder = builder.lifespan_seconds(value);
    }
    if schedule_interval_seconds == Some(0) {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_schedule_interval_invalid",
            json!({"schedule_interval_seconds": schedule_interval_seconds}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if schedule_max_runs == Some(0) {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_schedule_max_runs_invalid",
            json!({"schedule_max_runs": schedule_max_runs}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if schedule_interval_seconds.is_some() || schedule_max_runs.is_some() {
        builder = builder.schedule(SchedulePlan {
            interval_seconds: schedule_interval_seconds.unwrap_or(300),
            jitter_seconds: 15,
            max_runs: schedule_max_runs,
        });
    }
    for tool in tools.clone() {
        builder = builder.tool(tool);
    }
    for pack in capability_packs.clone() {
        builder = builder.capability_pack(pack);
    }
    let contract = builder.build().map_err(RuntimeLaneError::Build)?;
    let contract = contract.with_default_schedule_from_packs(&catalog);
    let resolved_tools = contract.resolved_tools(Some(&catalog));
    let agent_builder_ms = agent_builder_started.elapsed().as_millis() as u64;
    let wasm_execution_boundary_started = Instant::now();
    let wasm_execution_fuel_used = metadata
        .get("wasm_execution")
        .and_then(|value| value.get("fuel_used"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let wasm_execution_elapsed_ms = metadata
        .get("wasm_execution")
        .and_then(|value| value.get("elapsed_ms"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    for tool in &resolved_tools {
        if let Some(module_id) = tool.strip_prefix("wasm.") {
            match evaluate_wasm_execution_boundary(
                &wasm_policy,
                module_id,
                wasm_execution_fuel_used,
                wasm_execution_elapsed_ms,
                requests_network,
            ) {
                WasmPolicyDecision::Allowed => {}
                WasmPolicyDecision::Blocked(error_code) => {
                    if let Some(plan) = &contract.schedule {
                        let pack_id = contract
                            .capability_packs
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "runtime".to_string());
                        runtime_lane_state_mark_schedule_failure(
                            &mut durable_state,
                            contract.name.as_str(),
                            pack_id.as_str(),
                            plan,
                            error_code.as_str(),
                        );
                    }
                    return Ok(runtime_lane_fail_closed_with_state(
                        error_code.as_str(),
                        json!({
                            "boundary": "wasm_execution",
                            "module_id": module_id,
                            "fuel_used": wasm_execution_fuel_used,
                            "elapsed_ms": wasm_execution_elapsed_ms,
                            "requests_network": requests_network,
                        }),
                        &permissions,
                        wasm_sandbox.as_ref(),
                        voice_session.as_ref(),
                        &state_path,
                        &mut durable_state,
                    ));
                }
            }
        }
    }
    let wasm_execution_boundary_ms = wasm_execution_boundary_started.elapsed().as_millis() as u64;
    let pre_agent_setup_ms = pre_agent_setup_started.elapsed().as_millis() as u64;
    let context = AgentExecutionContext::new(providers, Some(&catalog));
    let agent_run_started = std::time::Instant::now();
    let mut run: AgentRunResult = match contract.run_once(&context) {
        Ok(result) => result,
        Err(error) => {
            if let Some(plan) = &contract.schedule {
                let pack_id = contract
                    .capability_packs
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "runtime".to_string());
                runtime_lane_state_mark_schedule_failure(
                    &mut durable_state,
                    contract.name.as_str(),
                    pack_id.as_str(),
                    plan,
                    error.code.as_str(),
                );
            }
            let _ = runtime_lane_state_save(&state_path, &durable_state);
            return Err(RuntimeLaneError::Provider(error));
        }
    };
    if !bootstrap_receipts.is_empty() {
        runtime_lane_merge_bootstrap_receipts_into_agent_run(&mut run, &bootstrap_receipts);
    }
    if model_manifest_probe_skipped && runtime_lane_agent_run_has_no_successful_mutation(&run) {
        runtime_lane_try_checkpointed_mutation_entry_recovery(
            &mut run,
            providers,
            checkpointed_recovery_provider.as_deref(),
            checkpointed_recovery_model.as_ref(),
            &metadata,
            &initial_prompt,
            &tools,
        );
    }
    let agent_run_ms = agent_run_started.elapsed().as_millis() as u64;
    let contract_checks_started = std::time::Instant::now();
    if let Some((error_code, details)) =
        native_success_contract_violation(&metadata, &run.receipt, &run.response.output)
    {
        if let Some(plan) = &contract.schedule {
            let pack_id = contract
                .capability_packs
                .first()
                .cloned()
                .unwrap_or_else(|| "runtime".to_string());
            runtime_lane_state_mark_schedule_failure(
                &mut durable_state,
                contract.name.as_str(),
                pack_id.as_str(),
                plan,
                error_code.as_str(),
            );
        }
        let mut response = runtime_lane_fail_closed_with_state(
            error_code.as_str(),
            details,
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        );
        runtime_lane_attach_agent_run_journal(&mut response, &run);
        return Ok(response);
    }
    if let Some((error_code, details)) =
        public_reasoning_contract_violation(&metadata, &run.receipt, &run.response.output)
    {
        if let Some(plan) = &contract.schedule {
            let pack_id = contract
                .capability_packs
                .first()
                .cloned()
                .unwrap_or_else(|| "runtime".to_string());
            runtime_lane_state_mark_schedule_failure(
                &mut durable_state,
                contract.name.as_str(),
                pack_id.as_str(),
                plan,
                error_code.as_str(),
            );
        }
        let mut response = runtime_lane_fail_closed_with_state(
            error_code.as_str(),
            details,
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        );
        runtime_lane_attach_agent_run_journal(&mut response, &run);
        return Ok(response);
    }
    let contract_checks_ms = contract_checks_started.elapsed().as_millis() as u64;
    let merkle_started = std::time::Instant::now();
    let persisted_previous_root = durable_state
        .merkle_roots
        .get(contract.name.as_str())
        .cloned();
    if let (Some(requested), Some(persisted)) = (
        previous_receipt_root.as_deref(),
        persisted_previous_root.as_deref(),
    ) {
        if requested != persisted {
            runtime_lane_state_record_merkle_continuity_failure(&mut durable_state);
        }
    }
    let effective_previous_root = previous_receipt_root
        .as_deref()
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&run.receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(contract.name.clone(), root.to_string());
    }
    let merkle_ms = merkle_started.elapsed().as_millis() as u64;
    let mut schedule_mark_ms = 0u64;
    if let Some(plan) = &contract.schedule {
        let schedule_mark_started = std::time::Instant::now();
        let pack_id = contract
            .capability_packs
            .first()
            .cloned()
            .unwrap_or_else(|| "runtime".to_string());
        runtime_lane_state_mark_schedule_success(
            &mut durable_state,
            contract.name.as_str(),
            pack_id.as_str(),
            plan,
        );
        schedule_mark_ms = schedule_mark_started.elapsed().as_millis() as u64;
    }
    let state_persist_started = std::time::Instant::now();
    let state_persist_error = runtime_lane_state_save(&state_path, &durable_state);
    let state_persist_ms = state_persist_started.elapsed().as_millis() as u64;
    let response_build_started = std::time::Instant::now();
    let voice = voice_request
        .as_ref()
        .map(|request| {
            voice_session_contract(
                request,
                permission_for(&permissions, "voice.realtime") == PermissionTrit::Allow,
            )
        })
        .unwrap_or(Value::Null);
    let agent_status = run
        .receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("ok")
        .to_string();
    let response_ok = agent_status == "ok";
    let response_error = if response_ok {
        None
    } else {
        Some(format!("runtime_lane_agent_status:{agent_status}"))
    };
    Ok(RuntimeLaneResponse {
        ok: response_ok,
        contract: json!({
            "name": contract.name,
            "provider": contract.provider,
            "agent_status": agent_status.clone(),
            "tool_count": contract.resolved_tools(Some(&catalog)).len(),
            "native_tool_call_count": run
                .receipt
                .get("native_tool_call_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            "tools": tools,
            "capability_packs": capability_packs,
            "capability_profiles": catalog.autonomy_profiles_for_packs(&contract.capability_packs),
            "required_permissions": required_pack_permissions,
            "schedule": contract.schedule,
            "lifespan_seconds": contract.lifespan_seconds,
            "permissions_manifest": permission_manifest_snapshot(&permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy),
            "voice_session": voice,
            "receipt_merkle": merkle,
            "workflow": metadata
                .get("workflow")
                .cloned()
                .unwrap_or(Value::Null),
        }),
        receipt: run.receipt,
        trace_summary: json!({
            "trace_id": run.trace.trace_id,
            "event_count": run.trace.events.len(),
            "agent_name": run.trace.agent_name,
            "wasm_modules": requested_modules,
            "requests_network": requests_network,
            "release_gate_counters": runtime_lane_state_release_gate_counters(&durable_state),
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
            "runtime_lane_phase_latency_ms": {
                "pre_agent_setup_ms": pre_agent_setup_ms,
                "state_load_ms": state_load_ms,
                "permission_setup_ms": permission_setup_ms,
                "permission_gate_ms": permission_gate_ms,
                "wasm_policy_ms": wasm_policy_ms,
                "direct_mutation_probe_ms": direct_mutation_probe_ms,
                "deterministic_local_loop_probe_ms": deterministic_local_loop_probe_ms,
                "first_edit_tool_calls_probe_ms": first_edit_tool_calls_probe_ms,
                "public_api_extension_probe_ms": public_api_extension_probe_ms,
                "bounded_existing_project_probe_ms": bounded_existing_project_probe_ms,
                "model_manifest_probe_skipped": model_manifest_probe_skipped,
                "model_manifest_probe_ms": model_manifest_probe_ms,
                "agent_builder_ms": agent_builder_ms,
                "wasm_execution_boundary_ms": wasm_execution_boundary_ms,
                "agent_run_ms": agent_run_ms,
                "contract_checks_ms": contract_checks_ms,
                "merkle_ms": merkle_ms,
                "schedule_mark_ms": schedule_mark_ms,
                "state_persist_ms": state_persist_ms,
                "response_build_ms": response_build_started.elapsed().as_millis() as u64,
            },
        }),
        output: run.response.output,
        error: response_error,
    })
}

fn runtime_lane_fail_closed_with_state(
    error_code: &str,
    details: Value,
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
) -> RuntimeLaneResponse {
    runtime_lane_state_record_denied_action(durable_state, error_code);
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    RuntimeLaneResponse {
        ok: false,
        contract: json!({
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "state_path": state_path.display().to_string(),
            "failure_details": details.clone(),
        }),
        receipt: json!({
            "type": "runtime_lane_receipt",
            "status": "fail_closed",
            "error_code": error_code,
            "details": details,
        }),
        trace_summary: json!({
            "status": "fail_closed",
            "error_code": error_code,
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
            "state_persist_error": state_persist_error,
        }),
        output: String::new(),
        error: Some(error_code.to_string()),
    }
}

#[derive(Clone, Debug)]
struct DirectMutationCandidate {
    workspace_root: Option<PathBuf>,
    target_path: PathBuf,
    content: String,
    content_source: &'static str,
    overwrite: bool,
}

#[derive(Clone, Debug)]
enum DirectMutationGate {
    NotCandidate,
    Blocked {
        failure_code: &'static str,
        failure_message: String,
        needed_input: Option<String>,
        target_path: Option<String>,
    },
    Candidate(DirectMutationCandidate),
}

#[derive(Clone, Debug)]
struct DeterministicLocalLoopCandidate {
    workspace_root: PathBuf,
    actions: Vec<DeterministicLocalAction>,
    requires_validation: bool,
}

#[derive(Clone, Debug)]
enum DeterministicLocalAction {
    WriteFile {
        target_path: PathBuf,
        content: String,
        overwrite: bool,
    },
    PatchFile {
        target_path: PathBuf,
        old: String,
        new: String,
        allow_multiple: bool,
    },
    CommandRun {
        cwd: PathBuf,
        cmd: Vec<String>,
        timeout_seconds: u64,
        max_output_bytes: u64,
    },
}

#[derive(Clone, Debug)]
enum DeterministicLocalLoopGate {
    NotCandidate,
    Blocked {
        failure_code: &'static str,
        failure_message: String,
        needed_input: Option<String>,
    },
    Candidate(DeterministicLocalLoopCandidate),
}

fn runtime_lane_try_direct_mutation(
    name: &str,
    prompt: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
) -> Option<RuntimeLaneResponse> {
    let total_started = Instant::now();
    let gate_started = Instant::now();
    let gate = runtime_lane_direct_mutation_candidate(prompt, tools, capability_packs, permissions);
    let execution_shape_gate_ms =
        gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    match gate {
        DirectMutationGate::NotCandidate => None,
        DirectMutationGate::Blocked {
            failure_code,
            failure_message,
            needed_input,
            target_path,
        } => {
            let response = runtime_lane_fail_closed_with_state(
                "runtime_lane_direct_mutation_blocked",
                json!({
                    "lane": "structured_blocker",
                    "lane_reason": "direct_mutation_precondition_failed",
                    "failure_code": failure_code,
                    "failure_message": failure_message,
                    "needed_input": needed_input,
                    "target_path": target_path,
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": 0,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            );
            Some(response)
        }
        DirectMutationGate::Candidate(candidate) => {
            let dispatch_started = Instant::now();
            let dispatcher = NativeToolDispatcher::new(&["file_write".to_string()]);
            let receipt = dispatcher.dispatch(NativeToolCall {
                id: "single_mutation_execution_1".to_string(),
                name: "file_write".to_string(),
                args: json!({
                    "path": candidate.target_path.display().to_string(),
                    "content": candidate.content.clone(),
                    "overwrite": candidate.overwrite,
                    "direct_mutation_lane": true,
                    "content_source": candidate.content_source,
                }),
            });
            let tool_dispatch_ms = dispatch_started
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            Some(runtime_lane_direct_mutation_response(
                name,
                metadata,
                tools,
                capability_packs,
                required_pack_permissions,
                permissions,
                wasm_sandbox,
                voice_session,
                receipt_merkle,
                previous_receipt_root,
                state_path,
                durable_state,
                candidate,
                receipt,
                execution_shape_gate_ms,
                tool_dispatch_ms,
                total_started,
            ))
        }
    }
}

fn runtime_lane_try_deterministic_local_loop(
    name: &str,
    prompt: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
) -> Option<RuntimeLaneResponse> {
    let total_started = Instant::now();
    let gate_started = Instant::now();
    let gate = runtime_lane_deterministic_local_loop_candidate(
        prompt,
        tools,
        capability_packs,
        permissions,
    );
    let execution_shape_gate_ms =
        gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    match gate {
        DeterministicLocalLoopGate::NotCandidate => None,
        DeterministicLocalLoopGate::Blocked {
            failure_code,
            failure_message,
            needed_input,
        } => {
            let response = runtime_lane_fail_closed_with_state(
                "runtime_lane_deterministic_local_loop_blocked",
                json!({
                    "lane": "structured_blocker",
                    "lane_reason": "deterministic_local_loop_precondition_failed",
                    "failure_code": failure_code,
                    "failure_message": failure_message,
                    "needed_input": needed_input,
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": 0,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            );
            Some(response)
        }
        DeterministicLocalLoopGate::Candidate(candidate) => {
            let dispatch_started = Instant::now();
            let dispatcher = NativeToolDispatcher::new(&[
                "file_write".to_string(),
                "file_patch".to_string(),
                "command_run".to_string(),
            ]);
            let mut receipts = Vec::<NativeToolReceipt>::new();
            for (index, action) in candidate.actions.iter().enumerate() {
                let call = match action {
                    DeterministicLocalAction::WriteFile {
                        target_path,
                        content,
                        overwrite,
                    } => NativeToolCall {
                        id: format!("deterministic_local_loop_{}", index + 1),
                        name: "file_write".to_string(),
                        args: json!({
                            "path": target_path.display().to_string(),
                            "content": content,
                            "overwrite": overwrite,
                            "deterministic_local_loop": true,
                        }),
                    },
                    DeterministicLocalAction::PatchFile {
                        target_path,
                        old,
                        new,
                        allow_multiple,
                    } => NativeToolCall {
                        id: format!("deterministic_local_loop_{}", index + 1),
                        name: "file_patch".to_string(),
                        args: json!({
                            "path": target_path.display().to_string(),
                            "old": old,
                            "new": new,
                            "allow_multiple": allow_multiple,
                            "deterministic_local_loop": true,
                        }),
                    },
                    DeterministicLocalAction::CommandRun {
                        cwd,
                        cmd,
                        timeout_seconds,
                        max_output_bytes,
                    } => NativeToolCall {
                        id: format!("deterministic_local_loop_{}", index + 1),
                        name: "command_run".to_string(),
                        args: json!({
                            "cwd": cwd.display().to_string(),
                            "cmd": cmd,
                            "timeout_seconds": timeout_seconds,
                            "max_output_bytes": max_output_bytes,
                            "deterministic_local_loop": true,
                        }),
                    },
                };
                let receipt = dispatcher.dispatch(call);
                let should_stop = receipt.status != "ok"
                    || (receipt.tool_name == "command_run"
                        && !receipt
                            .result
                            .get("success")
                            .and_then(Value::as_bool)
                            .unwrap_or(true));
                receipts.push(receipt);
                if should_stop {
                    break;
                }
            }
            let tool_dispatch_ms = dispatch_started
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            Some(runtime_lane_deterministic_local_loop_response(
                name,
                metadata,
                tools,
                capability_packs,
                required_pack_permissions,
                permissions,
                wasm_sandbox,
                voice_session,
                receipt_merkle,
                previous_receipt_root,
                state_path,
                durable_state,
                candidate,
                receipts,
                execution_shape_gate_ms,
                tool_dispatch_ms,
                total_started,
            ))
        }
    }
}

fn runtime_lane_try_public_api_extension_lane(
    name: &str,
    prompt: &str,
    provider: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    providers: &ProviderClientRegistry,
) -> Option<RuntimeLaneResponse> {
    let enabled = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/public_api_extension_lane_enabled",
            "/workflow/native_success_criteria/public_api_extension_lane_enabled",
        ],
    )
    .unwrap_or(false);
    if !enabled {
        return None;
    }
    let workspace_root = runtime_lane_extract_workspace_root(prompt)?;
    let context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
    if context_pack.trim().is_empty() {
        return None;
    }
    let public_api_bindings =
        runtime_lane_public_api_bindings(prompt, &context_pack, &workspace_root);
    if public_api_bindings.is_empty() {
        return None;
    }
    if !runtime_lane_public_api_bindings_have_missing_owner_symbols(&public_api_bindings) {
        return None;
    }
    if !runtime_lane_bounded_existing_project_edit_loop_eligible(
        prompt,
        &workspace_root,
        tools,
        capability_packs,
        permissions,
    ) {
        return None;
    }

    let public_api_contract =
        runtime_lane_public_api_bindings_prompt_section(&public_api_bindings, &workspace_root);
    let narrowed_prompt = format!(
        "{}\n\nPublic API extension lane selected.\nThis task has an explicit local import contract, so first satisfy the imported symbols in their owner module.\nDo not create a sibling source module as the primary implementation for these imported symbols.\nDo not rely on package-level __init__ re-exports for these imported symbols.\nAdd focused regression tests only when requested by the task.\nExisting behavior must remain compatible unless the task explicitly asks to change it.\n\n{}",
        prompt.trim(),
        public_api_contract
    );

    let mut response = runtime_lane_try_bounded_existing_project_edit_loop(
        name,
        &narrowed_prompt,
        provider,
        model,
        metadata,
        tools,
        capability_packs,
        required_pack_permissions,
        permissions,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
        state_path,
        durable_state,
        providers,
    )?;
    if runtime_lane_response_allows_bounded_existing_project_fallback(&response)
        || !runtime_lane_response_has_successful_mutation(&response)
    {
        return None;
    }
    runtime_lane_relabel_generated_manifest_response(
        &mut response,
        "public_api_extension_lane",
        "runtime_lane_public_api_extension_lane_failed",
        "public_api_owner_binding_first_loop",
    );
    Some(response)
}

fn runtime_lane_response_has_successful_mutation(response: &RuntimeLaneResponse) -> bool {
    response
        .receipt
        .get("native_tool_receipts")
        .and_then(Value::as_array)
        .map(|receipts| {
            receipts.iter().any(|receipt| {
                receipt.get("status").and_then(Value::as_str) == Some("ok")
                    && matches!(
                        receipt.get("tool_name").and_then(Value::as_str),
                        Some("file_write" | "file_patch")
                    )
            })
        })
        .unwrap_or(false)
}

fn runtime_lane_try_first_edit_tool_calls_lane(
    name: &str,
    prompt: &str,
    provider: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    providers: &ProviderClientRegistry,
) -> Option<RuntimeLaneResponse> {
    let enabled = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/first_edit_tool_calls_lane_enabled",
            "/workflow/native_success_criteria/first_edit_tool_calls_lane_enabled",
        ],
    )
    .unwrap_or(false);
    if !enabled {
        return None;
    }
    let strict_diagnostics = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/first_edit_tool_calls_strict_diagnostics",
            "/workflow/native_success_criteria/first_edit_tool_calls_strict_diagnostics",
        ],
    )
    .unwrap_or(true);
    let timeout_demotes_to_parent = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/first_edit_tool_calls_timeout_demotes_to_parent",
            "/workflow/native_success_criteria/first_edit_tool_calls_timeout_demotes_to_parent",
        ],
    )
    .unwrap_or(true);
    let empty_response_demotes_to_parent = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/first_edit_tool_calls_empty_demotes_to_parent",
            "/workflow/native_success_criteria/first_edit_tool_calls_empty_demotes_to_parent",
        ],
    )
    .unwrap_or(true);
    let workspace_root = runtime_lane_extract_workspace_root(prompt)?;
    let existing_project_first_edit_eligible = runtime_lane_bounded_existing_project_edit_loop_eligible(
        prompt,
        &workspace_root,
        tools,
        capability_packs,
        permissions,
    );
    let bounded_new_file_first_edit_eligible = runtime_lane_bounded_new_file_first_edit_eligible(
        prompt,
        &workspace_root,
        tools,
        capability_packs,
        permissions,
    );
    if !existing_project_first_edit_eligible && !bounded_new_file_first_edit_eligible {
        runtime_lane_record_first_edit_fallback(
            &workspace_root,
            "not_eligible",
            json!({
                "existing_project_first_edit_eligible": existing_project_first_edit_eligible,
                "bounded_new_file_first_edit_eligible": bounded_new_file_first_edit_eligible,
            }),
        );
        return None;
    }
    let context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
    let provider_id = provider.unwrap_or_else(|| providers.default_provider_id());
    let provider_client = match providers.from_provider_id(provider_id) {
        Ok(provider) => provider,
        Err(error) => {
            if !strict_diagnostics {
                return None;
            }
            return Some(runtime_lane_fail_closed_with_state(
                "runtime_lane_first_edit_tool_calls_provider_unavailable",
                json!({
                    "lane": "first_edit_tool_calls",
                    "failure_code": error.code.as_str(),
                    "failure_message": error.message,
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            ));
        }
    };
    let model_lock = std::env::var("INFRING_RUNTIME_LANE_MODEL_LOCK")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let lane_model = model_lock.or_else(|| model.cloned());
    let mut timeout_seconds = runtime_lane_metadata_u64_value(
        metadata,
        &[
            "/native_success_criteria/first_edit_tool_calls_provider_timeout_seconds",
            "/workflow/native_success_criteria/first_edit_tool_calls_provider_timeout_seconds",
            "/native_success_criteria/first_mutation_artifact_lane_v1_provider_timeout_seconds",
            "/workflow/native_success_criteria/first_mutation_artifact_lane_v1_provider_timeout_seconds",
        ],
    )
    .unwrap_or(15)
    .clamp(5, 60);
    if bounded_new_file_first_edit_eligible {
        let new_file_timeout_seconds = runtime_lane_metadata_u64_value(
            metadata,
            &[
                "/native_success_criteria/first_edit_tool_calls_new_file_provider_timeout_seconds",
                "/workflow/native_success_criteria/first_edit_tool_calls_new_file_provider_timeout_seconds",
                "/native_success_criteria/fast_lane_provider_timeout_seconds",
                "/workflow/native_success_criteria/fast_lane_provider_timeout_seconds",
            ],
        )
        .unwrap_or(15)
        .clamp(5, 30);
        timeout_seconds = timeout_seconds.min(new_file_timeout_seconds);
    } else if existing_project_first_edit_eligible {
        timeout_seconds = runtime_lane_metadata_u64_value(
            metadata,
            &[
                "/native_success_criteria/first_edit_tool_calls_existing_project_provider_timeout_seconds",
                "/workflow/native_success_criteria/first_edit_tool_calls_existing_project_provider_timeout_seconds",
            ],
        )
        .unwrap_or(60)
        .clamp(15, 75);
    }
    let total_started = Instant::now();
    let gate_started = Instant::now();
    let compact_context = if context_pack.trim().is_empty() && bounded_new_file_first_edit_eligible {
        "Local context pack: empty or new workspace. Create only the bounded target file(s) implied by the task; do not scaffold a project.".to_string()
    } else {
        runtime_lane_compact_context_pack(&context_pack)
    };
    let prompt_text = runtime_lane_first_edit_tool_calls_prompt(prompt, &workspace_root, &compact_context);
    let first_edit_fallback_target_path =
        runtime_lane_first_edit_fallback_target_path(prompt, &workspace_root);
    let execution_shape_gate_ms =
        gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let model_started = Instant::now();
    let mut provider_response = match provider_client.stream_complete(&ProviderRequest {
            prompt: prompt_text,
            system: Some(runtime_lane_first_edit_tool_calls_system()),
            tools: Vec::new(),
            model: lane_model.clone(),
            metadata: json!({
                "provider_timeout_seconds": timeout_seconds,
                "provider_stream_until_tool_calls": true,
                "omit_ollama_thinking_flags": true,
                "lane": "first_edit_tool_calls",
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
            }),
        }) {
        Ok(stream) => stream.response,
        Err(error) => {
            let model_call_ms = model_started
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            if timeout_demotes_to_parent
                && runtime_lane_provider_error_is_timeout(error.code.as_str(), &error.message)
            {
                runtime_lane_record_first_edit_fallback(
                    &workspace_root,
                    "provider_timeout_demoted_to_parent",
                    json!({
                        "provider_latency_ms": model_call_ms,
                        "provider_timeout_seconds": timeout_seconds,
                        "bounded_new_file_first_edit_eligible": bounded_new_file_first_edit_eligible,
                        "failure_code": error.code.as_str(),
                        "failure_message": error.message,
                    }),
                );
                return None;
            }
            if !strict_diagnostics {
                runtime_lane_record_first_edit_fallback(
                    &workspace_root,
                    "provider_failed_demoted_to_parent",
                    json!({
                        "provider_latency_ms": model_call_ms,
                        "provider_timeout_seconds": timeout_seconds,
                        "bounded_new_file_first_edit_eligible": bounded_new_file_first_edit_eligible,
                        "failure_code": error.code.as_str(),
                        "failure_message": error.message,
                    }),
                );
                return None;
            }
            return Some(runtime_lane_fail_closed_with_state(
                "runtime_lane_first_edit_tool_calls_provider_failed",
                json!({
                    "lane": "first_edit_tool_calls",
                    "failure_code": error.code.as_str(),
                    "failure_message": error.message,
                    "provider_latency_ms": model_call_ms,
                    "provider_timeout_seconds": timeout_seconds,
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            ));
        }
    };
    let mut model_call_ms = model_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let mut parsed_tool_calls = parse_native_tool_calls(&provider_response.output);
    if runtime_lane_first_edit_tool_calls_placeholder_only(&parsed_tool_calls) {
        let mut placeholder_target_paths = runtime_lane_first_edit_tool_call_target_paths(
            &parsed_tool_calls,
            &workspace_root,
            first_edit_fallback_target_path.as_deref(),
        );
        if existing_project_first_edit_eligible {
            placeholder_target_paths = runtime_lane_first_edit_contextual_target_paths(
                &placeholder_target_paths,
                &workspace_root,
                &context_pack,
            );
        }
        let retry_timeout_seconds = if placeholder_target_paths.is_empty() {
            timeout_seconds.min(10)
        } else {
            timeout_seconds.min(30).max(15)
        };
        let retry_started = Instant::now();
        match provider_client.stream_complete(&ProviderRequest {
            prompt: runtime_lane_first_edit_tool_calls_placeholder_retry_prompt(
                prompt,
                &workspace_root,
                first_edit_fallback_target_path.as_deref(),
                &placeholder_target_paths,
                &compact_context,
            ),
            system: Some(runtime_lane_first_edit_tool_calls_system()),
            tools: Vec::new(),
            model: lane_model.clone(),
            metadata: json!({
                "provider_timeout_seconds": retry_timeout_seconds,
                "provider_stream_until_tool_calls": true,
                "omit_ollama_thinking_flags": true,
                "lane": "first_edit_tool_calls",
                "attempt": "placeholder_retry",
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
            }),
        }) {
            Ok(stream) => {
                model_call_ms += retry_started
                    .elapsed()
                    .as_millis()
                    .min(u128::from(u64::MAX)) as u64;
                provider_response = stream.response;
                parsed_tool_calls = parse_native_tool_calls(&provider_response.output);
            }
            Err(error) => {
                model_call_ms += retry_started
                    .elapsed()
                    .as_millis()
                    .min(u128::from(u64::MAX)) as u64;
                runtime_lane_record_first_edit_fallback(
                    &workspace_root,
                    "placeholder_retry_provider_failed",
                    json!({
                        "provider_latency_ms": model_call_ms,
                        "provider_timeout_seconds": retry_timeout_seconds,
                        "bounded_new_file_first_edit_eligible": bounded_new_file_first_edit_eligible,
                        "target_constrained_retry": !placeholder_target_paths.is_empty(),
                        "target_paths": placeholder_target_paths
                            .iter()
                            .map(|path| runtime_lane_path_relative_display(&workspace_root, path))
                            .collect::<Vec<_>>(),
                        "failure_code": error.code.as_str(),
                        "failure_message": error.message,
                    }),
                );
            }
        }
    }
    let actions = parsed_tool_calls
        .into_iter()
        .take(6)
        .filter_map(|call| {
            runtime_lane_first_edit_tool_call_action(
                &call,
                &workspace_root,
                first_edit_fallback_target_path.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    if existing_project_first_edit_eligible
        && !runtime_lane_first_edit_actions_within_context(
            &actions,
            &workspace_root,
            &context_pack,
        )
    {
        runtime_lane_record_first_edit_fallback(
            &workspace_root,
            "contextual_action_path_not_in_context",
            json!({
                "provider_latency_ms": model_call_ms,
                "provider_timeout_seconds": timeout_seconds,
                "provider_output_preview": provider_response.output.chars().take(800).collect::<String>(),
            }),
        );
        return None;
    }
    if actions.is_empty() {
        if empty_response_demotes_to_parent {
            runtime_lane_record_first_edit_fallback(
                &workspace_root,
                "empty_tool_calls_demoted_to_parent",
                json!({
                    "provider_latency_ms": model_call_ms,
                    "provider_timeout_seconds": timeout_seconds,
                    "bounded_new_file_first_edit_eligible": bounded_new_file_first_edit_eligible,
                    "provider_output_preview": provider_response.output.chars().take(800).collect::<String>(),
                }),
            );
            return None;
        }
        if !strict_diagnostics {
            runtime_lane_record_first_edit_fallback(
                &workspace_root,
                "empty_tool_calls_demoted_to_parent",
                json!({
                    "provider_latency_ms": model_call_ms,
                    "provider_timeout_seconds": timeout_seconds,
                    "bounded_new_file_first_edit_eligible": bounded_new_file_first_edit_eligible,
                    "provider_output_preview": provider_response.output.chars().take(800).collect::<String>(),
                }),
            );
            return None;
        }
        return Some(runtime_lane_fail_closed_with_state(
            "runtime_lane_first_edit_tool_calls_no_mutation_tool_calls",
            json!({
                "lane": "first_edit_tool_calls",
                "provider_latency_ms": model_call_ms,
                "provider_timeout_seconds": timeout_seconds,
                "provider_output_preview": provider_response.output.chars().take(1600).collect::<String>(),
                "needed_input": "Return native tool_calls with file_write or file_patch only.",
            }),
            permissions,
            wasm_sandbox,
            voice_session,
            state_path,
            durable_state,
        ));
    }
    let mut candidate = DeterministicLocalLoopCandidate {
        workspace_root,
        actions,
        requires_validation: false,
    };
    let dispatch_started = Instant::now();
    let dispatcher = NativeToolDispatcher::new(&["file_write".to_string(), "file_patch".to_string()]);
    let mut receipts = Vec::<NativeToolReceipt>::new();
    for (index, action) in candidate.actions.iter().enumerate() {
        let call = match action {
            DeterministicLocalAction::WriteFile {
                target_path,
                content,
                overwrite,
            } => NativeToolCall {
                id: format!("first_edit_tool_calls_{}", index + 1),
                name: "file_write".to_string(),
                args: json!({
                    "path": target_path.display().to_string(),
                    "content": content,
                    "overwrite": overwrite,
                    "first_edit_tool_calls": true,
                }),
            },
            DeterministicLocalAction::PatchFile {
                target_path,
                old,
                new,
                allow_multiple,
            } => NativeToolCall {
                id: format!("first_edit_tool_calls_{}", index + 1),
                name: "file_patch".to_string(),
                args: json!({
                    "path": target_path.display().to_string(),
                    "old": old,
                    "new": new,
                    "allow_multiple": allow_multiple,
                    "first_edit_tool_calls": true,
                }),
            },
            DeterministicLocalAction::CommandRun { .. } => continue,
        };
        let receipt = dispatcher.dispatch(call);
        let should_stop = receipt.status != "ok";
        receipts.push(receipt);
        if should_stop {
            break;
        }
    }
    if !receipts.iter().any(runtime_lane_receipt_is_successful_mutation) {
        if !strict_diagnostics {
            return None;
        }
        return Some(runtime_lane_fail_closed_with_state(
            "runtime_lane_first_edit_tool_calls_no_successful_mutation",
            json!({
                "lane": "first_edit_tool_calls",
                "provider_latency_ms": model_call_ms,
                "provider_timeout_seconds": timeout_seconds,
                "native_tool_receipts": receipts,
            }),
            permissions,
            wasm_sandbox,
            voice_session,
            state_path,
            durable_state,
        ));
    }
    let validation_command =
        runtime_lane_extract_validation_command(prompt, &candidate.workspace_root);
    let semantic_probe =
        runtime_lane_extract_or_default_semantic_probe_command(prompt, &candidate.workspace_root);
    if validation_command.is_some() || semantic_probe.is_some() {
        candidate.requires_validation = true;
        runtime_lane_append_explicit_validation_recheck(
            &mut receipts,
            &candidate.workspace_root,
            validation_command.as_ref(),
            semantic_probe.as_ref(),
            "first_edit_tool_calls_recheck",
        );
        if let Some(failure) = runtime_lane_semantic_probe_failure(&receipts, None) {
            let repair_context_pack =
                runtime_lane_model_manifest_context_pack(prompt, &candidate.workspace_root);
            let repair_timeout_seconds = runtime_lane_metadata_u64_value(
                metadata,
                &[
                    "/native_success_criteria/first_edit_tool_calls_repair_provider_timeout_seconds",
                    "/workflow/native_success_criteria/first_edit_tool_calls_repair_provider_timeout_seconds",
                    "/native_success_criteria/fast_lane_repair_provider_timeout_seconds",
                    "/workflow/native_success_criteria/fast_lane_repair_provider_timeout_seconds",
                ],
            )
            .unwrap_or(45)
            .clamp(5, 60);
            match provider_client.complete(&ProviderRequest {
                prompt: runtime_lane_model_manifest_semantic_repair_prompt(
                    prompt,
                    &candidate.workspace_root,
                    &repair_context_pack,
                    &failure,
                ),
                system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
                tools: Vec::new(),
                model: lane_model,
                metadata: json!({
                    "provider_timeout_seconds": repair_timeout_seconds,
                    "omit_ollama_thinking_flags": false,
                    "lane": "first_edit_tool_calls",
                    "attempt": "post_mutation_semantic_repair",
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                Ok(repair_response) => {
                    match runtime_lane_model_manifest_candidate_from_output(
                        &repair_response.output,
                        tools,
                        capability_packs,
                        permissions,
                    ) {
                        Ok(mut repair_candidate) => {
                            repair_candidate.requires_validation = true;
                            let mut repair_receipts = runtime_lane_dispatch_model_manifest_actions(
                                &repair_candidate,
                                "first_edit_tool_calls_semantic_repair",
                            );
                            if repair_receipts
                                .iter()
                                .any(runtime_lane_receipt_is_successful_mutation)
                            {
                                runtime_lane_append_explicit_validation_recheck(
                                    &mut repair_receipts,
                                    &candidate.workspace_root,
                                    validation_command.as_ref(),
                                    semantic_probe.as_ref(),
                                    "first_edit_tool_calls_semantic_repair_recheck",
                                );
                            }
                            receipts.append(&mut repair_receipts);
                        }
                        Err(error) => receipts.push(runtime_lane_semantic_repair_failure_receipt(
                            "first_edit_tool_calls_semantic_repair_parse_failed",
                            "semantic_repair_parse_failed",
                            "Semantic repair provider response did not produce a valid deterministic local loop.",
                            json!({
                                "lane": "first_edit_tool_calls",
                                "provider_timeout_seconds": repair_timeout_seconds,
                                "parse_error": format!("{error:?}"),
                            }),
                        )),
                    }
                }
                Err(error) => receipts.push(runtime_lane_semantic_repair_failure_receipt(
                    "first_edit_tool_calls_semantic_repair_provider_failed",
                    error.code.as_str(),
                    "Semantic repair provider call failed.",
                    json!({
                        "lane": "first_edit_tool_calls",
                        "provider_timeout_seconds": repair_timeout_seconds,
                        "failure_message": error.message,
                    }),
                )),
            }
        }
    }
    let tool_dispatch_ms = dispatch_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let mut response = runtime_lane_deterministic_local_loop_response(
        name,
        metadata,
        tools,
        capability_packs,
        required_pack_permissions,
        permissions,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
        state_path,
        durable_state,
        candidate,
        receipts,
        execution_shape_gate_ms,
        tool_dispatch_ms,
        total_started,
    );
    runtime_lane_attach_coding_runtime_probe(
        &mut response,
        json!({
            "lane": "first_edit_tool_calls",
            "provider_latency_ms": model_call_ms,
            "provider_timeout_seconds": timeout_seconds,
            "prompt_chars": provider_response.output.chars().count(),
            "time_to_first_tool_ms": provider_response.raw
                .get("ollama_timing")
                .and_then(|timing| timing.get("balanced_tool_calls_json_ms"))
                .cloned()
                .unwrap_or(Value::Null),
        }),
    );
    Some(response)
}

fn runtime_lane_metadata_u64_value(metadata: &Value, pointers: &[&str]) -> Option<u64> {
    for pointer in pointers {
        if let Some(value) = metadata.pointer(pointer).and_then(Value::as_u64) {
            return Some(value);
        }
    }
    None
}

fn runtime_lane_set_native_success_criteria_value(
    metadata: &mut Value,
    key: &str,
    value: Value,
) {
    if !metadata.is_object() {
        *metadata = json!({});
    }
    let Some(object) = metadata.as_object_mut() else {
        return;
    };
    let criteria = object
        .entry("native_success_criteria".to_string())
        .or_insert_with(|| json!({}));
    if !criteria.is_object() {
        *criteria = json!({});
    }
    if let Some(criteria_object) = criteria.as_object_mut() {
        criteria_object.insert(key.to_string(), value);
    }
}

fn runtime_lane_first_edit_tool_calls_system() -> String {
    "Return only one JSON object with key tool_calls. Allowed tool names are exactly file_write and file_patch. Forbidden: bash, shell, cat, file_read, command_run, markdown, and prose. Runtime validates later."
        .to_string()
}

fn runtime_lane_first_edit_tool_calls_prompt(
    prompt: &str,
    workspace_root: &Path,
    compact_context: &str,
) -> String {
    let allowed_paths = runtime_lane_first_edit_allowed_mutation_paths(compact_context);
    let allowed_section = runtime_lane_first_edit_allowed_paths_section(&allowed_paths);
    format!(
        "Root: {}\nTask: {}\n{}\nContext:\n{}\nReturn JSON only with tool_calls. file_write args: path, content, overwrite. file_patch args: path, old, new, allow_multiple. Use actual file text only; never use placeholder text. Use an actual path under Root, never /absolute/path or .... For existing-project edits, use exactly one of the allowed mutation paths. Never edit .infring, semantic probe, or validation harness files. Use only file_write/file_patch under Root. Mutate source before exports/tests. Include tests if requested. No validation/prose.",
        workspace_root.display(),
        runtime_lane_first_edit_task_brief(prompt),
        allowed_section,
        compact_context
    )
}

fn runtime_lane_first_edit_tool_calls_placeholder_retry_prompt(
    prompt: &str,
    workspace_root: &Path,
    fallback_target_path: Option<&Path>,
    constrained_target_paths: &[PathBuf],
    compact_context: &str,
) -> String {
    let allowed_paths = runtime_lane_first_edit_allowed_mutation_paths(compact_context);
    let allowed_section = runtime_lane_first_edit_allowed_paths_section(&allowed_paths);
    let target_line = if constrained_target_paths.is_empty() {
        fallback_target_path
            .filter(|path| path.starts_with(workspace_root))
            .map(|path| {
                format!(
                    "Use this target path if it satisfies the task: {}",
                    path.display()
                )
            })
            .unwrap_or_else(|| "Use an actual safe file path under Root.".to_string())
    } else {
        format!(
            "Target-constrained retry. Edit only these path(s):\n{}",
            constrained_target_paths
                .iter()
                .map(|path| format!("- {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    format!(
        "Root: {}\nTask: {}\n{}\nContext:\n{}\nPrevious response used placeholders or disallowed paths.\nRetry once. {}\nReturn JSON only with tool_calls. Allowed tool names: file_write or file_patch. Every path must be one of the target/allowed paths and under Root. Every content/old/new value must be actual complete text, never placeholders. Mutate source before exports/tests. No validation/prose.",
        workspace_root.display(),
        runtime_lane_first_edit_task_brief(prompt),
        allowed_section,
        compact_context,
        target_line
    )
}

fn runtime_lane_first_edit_allowed_mutation_paths(context_pack: &str) -> Vec<String> {
    runtime_lane_context_pack_relative_paths(context_pack)
        .into_iter()
        .filter(|path| {
            let lower = path.to_ascii_lowercase();
            !lower.starts_with(".infring/")
                && !lower.contains("semantic_probe")
                && !lower.contains("validation")
        })
        .take(8)
        .collect()
}

fn runtime_lane_first_edit_allowed_paths_section(paths: &[String]) -> String {
    if paths.is_empty() {
        return "Allowed mutation paths: none supplied; create only the bounded target file implied by the task.".to_string();
    }
    format!(
        "Allowed mutation paths:\n{}",
        paths
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn runtime_lane_first_edit_task_brief(prompt: &str) -> String {
    prompt
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("Rules:")
                && !trimmed.starts_with("Native")
                && !trimmed.starts_with("Use this")
        })
        .take(18)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(1800)
        .collect()
}

fn runtime_lane_first_edit_fallback_target_path(prompt: &str, workspace_root: &Path) -> Option<PathBuf> {
    let raw_target = runtime_lane_extract_target_file_path(prompt)?;
    runtime_lane_resolve_target_path(&raw_target, Some(&workspace_root.to_path_buf())).ok()
}

fn runtime_lane_record_first_edit_fallback(workspace_root: &Path, reason: &str, details: Value) {
    let trace_dir = workspace_root.join(".infring");
    let _ = std::fs::create_dir_all(&trace_dir);
    let updated_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let record = json!({
        "schema_version": "first_edit_fallback_probe_v1",
        "reason": reason,
        "details": details,
        "updated_at_unix_ms": updated_at_unix_ms,
    });
    let mut line = record.to_string();
    line.push('\n');
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(trace_dir.join("first_edit_fallbacks.jsonl"))
    {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}

fn runtime_lane_first_edit_tool_call_action(
    call: &NativeToolCall,
    workspace_root: &Path,
    fallback_target_path: Option<&Path>,
) -> Option<DeterministicLocalAction> {
    match call.name.trim().to_ascii_lowercase().as_str() {
        "file_write" | "write_file" | "workspace.write" | "workspace_write" => {
            let path = runtime_lane_first_edit_tool_call_path(call, workspace_root, fallback_target_path)?;
            let content = runtime_lane_first_edit_tool_call_text(
                call,
                &["content", "contents", "text", "body"],
            )?
            .to_string();
            Some(DeterministicLocalAction::WriteFile {
                target_path: path,
                content,
                overwrite: call
                    .args
                    .get("overwrite")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            })
        }
        "file_patch" | "patch_file" | "workspace.patch" | "workspace_patch" => {
            let path = runtime_lane_first_edit_tool_call_path(call, workspace_root, fallback_target_path)?;
            let old = runtime_lane_first_edit_tool_call_text(
                call,
                &["old", "find", "search", "before", "original"],
            )?
            .to_string();
            let new = runtime_lane_first_edit_tool_call_text(
                call,
                &["new", "replace", "replacement", "after", "updated"],
            )?
            .to_string();
            Some(DeterministicLocalAction::PatchFile {
                target_path: path,
                old,
                new,
                allow_multiple: call
                    .args
                    .get("allow_multiple")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        }
        _ => None,
    }
}

fn runtime_lane_first_edit_tool_call_path(
    call: &NativeToolCall,
    workspace_root: &Path,
    fallback_target_path: Option<&Path>,
) -> Option<PathBuf> {
    let raw = runtime_lane_first_edit_tool_call_text(
        call,
        &[
            "path",
            "file_path",
            "filepath",
            "target_path",
            "target",
            "file",
            "absolute_path",
            "full_path",
        ],
    )?;
    if runtime_lane_first_edit_placeholder_path(raw) {
        if let Some(path) = fallback_target_path.filter(|path| path.starts_with(workspace_root)) {
            return Some(path.to_path_buf());
        }
    }
    let candidate = if Path::new(raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        workspace_root.join(raw)
    };
    if candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    if candidate.starts_with(workspace_root) {
        Some(candidate)
    } else {
        None
    }
}

fn runtime_lane_first_edit_tool_call_target_paths(
    calls: &[NativeToolCall],
    workspace_root: &Path,
    fallback_target_path: Option<&Path>,
) -> Vec<PathBuf> {
    let mut target_paths = Vec::new();
    for call in calls {
        match call.name.trim().to_ascii_lowercase().as_str() {
            "file_write" | "write_file" | "workspace.write" | "workspace_write" | "file_patch"
            | "patch_file" | "workspace.patch" | "workspace_patch" => {}
            _ => continue,
        }
        let Some(path) =
            runtime_lane_first_edit_tool_call_path(call, workspace_root, fallback_target_path)
        else {
            continue;
        };
        let relative_path = runtime_lane_path_relative_display(workspace_root, &path);
        if !runtime_lane_first_edit_relative_path_is_safe_mutation_target(&relative_path) {
            continue;
        }
        if !target_paths.iter().any(|existing| existing == &path) {
            target_paths.push(path);
        }
    }
    target_paths.truncate(6);
    target_paths
}

fn runtime_lane_first_edit_placeholder_path(value: &str) -> bool {
    matches!(
        value.trim(),
        "/absolute/path" | "absolute/path" | "/path/to/file" | "path/to/file" | "target_file.py"
    )
}

fn runtime_lane_first_edit_placeholder_text(value: &str) -> bool {
    matches!(
        value.trim(),
        "..." | "…" | "complete file content" | "replacement text" | "exact observed text"
    )
}

fn runtime_lane_first_edit_raw_tool_arg<'a>(
    call: &'a NativeToolCall,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| call.args.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn runtime_lane_first_edit_tool_calls_placeholder_only(calls: &[NativeToolCall]) -> bool {
    let mut saw_relevant_call = false;
    for call in calls {
        match call.name.trim().to_ascii_lowercase().as_str() {
            "file_write" | "write_file" | "workspace.write" | "workspace_write" => {
                saw_relevant_call = true;
                let path_is_placeholder = runtime_lane_first_edit_raw_tool_arg(
                    call,
                    &[
                        "path",
                        "file_path",
                        "filepath",
                        "target_path",
                        "target",
                        "file",
                        "absolute_path",
                        "full_path",
                    ],
                )
                .map(runtime_lane_first_edit_placeholder_path)
                .unwrap_or(false);
                let content_is_placeholder = runtime_lane_first_edit_raw_tool_arg(
                    call,
                    &["content", "contents", "text", "body"],
                )
                .map(runtime_lane_first_edit_placeholder_text)
                .unwrap_or(false);
                if !path_is_placeholder && !content_is_placeholder {
                    return false;
                }
            }
            "file_patch" | "patch_file" | "workspace.patch" | "workspace_patch" => {
                saw_relevant_call = true;
                let path_is_placeholder = runtime_lane_first_edit_raw_tool_arg(
                    call,
                    &[
                        "path",
                        "file_path",
                        "filepath",
                        "target_path",
                        "target",
                        "file",
                        "absolute_path",
                        "full_path",
                    ],
                )
                .map(runtime_lane_first_edit_placeholder_path)
                .unwrap_or(false);
                let old_is_placeholder = runtime_lane_first_edit_raw_tool_arg(
                    call,
                    &["old", "find", "search", "before", "original"],
                )
                .map(runtime_lane_first_edit_placeholder_text)
                .unwrap_or(false);
                let new_is_placeholder = runtime_lane_first_edit_raw_tool_arg(
                    call,
                    &["new", "replace", "replacement", "after", "updated"],
                )
                .map(runtime_lane_first_edit_placeholder_text)
                .unwrap_or(false);
                if !path_is_placeholder && !old_is_placeholder && !new_is_placeholder {
                    return false;
                }
            }
            _ => {}
        }
    }
    saw_relevant_call
}

fn runtime_lane_first_edit_tool_call_text<'a>(
    call: &'a NativeToolCall,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| call.args.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "..." && *value != "…")
}

fn runtime_lane_try_bounded_existing_project_edit_loop(
    name: &str,
    prompt: &str,
    provider: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    providers: &ProviderClientRegistry,
) -> Option<RuntimeLaneResponse> {
    let enabled = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/bounded_existing_project_edit_loop_enabled",
            "/workflow/native_success_criteria/bounded_existing_project_edit_loop_enabled",
        ],
    )
    .unwrap_or(true);
    if !enabled {
        return None;
    }
    let workspace_root = runtime_lane_extract_workspace_root(prompt)?;
    if !runtime_lane_bounded_existing_project_edit_loop_eligible(
        prompt,
        &workspace_root,
        tools,
        capability_packs,
        permissions,
    ) {
        return None;
    }

    let total_started = Instant::now();
    let gate_started = Instant::now();
    let context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
    if context_pack.trim().is_empty() {
        return None;
    }
    let public_api_bindings =
        runtime_lane_public_api_bindings(prompt, &context_pack, &workspace_root);
    let execution_shape_gate_ms =
        gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let provider_id = provider.unwrap_or_else(|| providers.default_provider_id());
    let provider_client = match providers.from_provider_id(provider_id) {
        Ok(provider) => provider,
        Err(error) => {
            return Some(runtime_lane_fail_closed_with_state(
                "runtime_lane_bounded_existing_project_edit_loop_provider_unavailable",
                json!({
                    "lane": "bounded_existing_project_edit_loop",
                    "failure_code": error.code.as_str(),
                    "failure_message": error.message,
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": 0,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            ));
        }
    };
    let model_lock = std::env::var("INFRING_RUNTIME_LANE_MODEL_LOCK")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let fast_lane_model = model_lock
        .clone()
        .or_else(|| {
            runtime_lane_metadata_string(
                metadata,
                &[
                    "/native_success_criteria/fast_lane_model",
                    "/workflow/native_success_criteria/fast_lane_model",
                ],
            )
        })
        .or_else(|| model.cloned());
    let fast_lane_repair_model = model_lock
        .clone()
        .or_else(|| {
            runtime_lane_metadata_string(
                metadata,
                &[
                    "/native_success_criteria/fast_lane_repair_model",
                    "/workflow/native_success_criteria/fast_lane_repair_model",
                ],
            )
        })
        .or_else(|| model.cloned());
    let fast_lane_omit_ollama_thinking_flags = runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
            "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
        ],
    )
    .unwrap_or(true);
    let planning_depth = runtime_lane_planning_depth_selector(prompt, metadata);
    let manifest_prompt = runtime_lane_apply_planning_depth_prompt(
        runtime_lane_bounded_existing_project_edit_loop_prompt(
            prompt,
            &workspace_root,
            &context_pack,
            &public_api_bindings,
        ),
        &planning_depth,
    );
    let manifest_system = runtime_lane_bounded_existing_project_edit_loop_system();
    let manifest_provider_timeout_seconds = metadata
        .pointer("/native_success_criteria/bounded_existing_project_provider_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/bounded_existing_project_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/fast_lane_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/fast_lane_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .unwrap_or(60)
        .max(60);
    let manifest_repair_provider_timeout_seconds = metadata
        .pointer("/native_success_criteria/bounded_existing_project_repair_provider_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/bounded_existing_project_repair_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/fast_lane_repair_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/fast_lane_repair_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .unwrap_or(45)
        .max(45);
    let model_started = Instant::now();
    let mut provider_response = match provider_client.complete(&ProviderRequest {
            prompt: manifest_prompt.clone(),
            system: Some(manifest_system.clone()),
            tools: Vec::new(),
            model: fast_lane_model.clone(),
            metadata: json!({
                "provider_timeout_seconds": manifest_provider_timeout_seconds,
                "omit_ollama_thinking_flags": fast_lane_omit_ollama_thinking_flags,
                "lane": "bounded_existing_project_edit_loop",
                "planning_depth_selector": planning_depth.to_json(),
                "attempt": "fast_manifest_first_pass",
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
            }),
        }) {
        Ok(response) => response,
        Err(error) => {
            let model_call_ms = model_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            return Some(runtime_lane_fail_closed_with_state(
                "runtime_lane_bounded_existing_project_edit_loop_provider_failed",
                json!({
                    "lane": "bounded_existing_project_edit_loop",
                    "failure_code": error.code.as_str(),
                    "failure_message": error.message,
                    "planner_model": fast_lane_model.clone(),
                    "coding_runtime_probe": {
                        "selected_planning_depth": planning_depth.to_json(),
                        "manifest_provider_timeout_seconds": manifest_provider_timeout_seconds,
                        "repair_provider_timeout_seconds": manifest_repair_provider_timeout_seconds,
                        "prompt_chars": manifest_prompt.chars().count(),
                        "system_chars": manifest_system.chars().count(),
                        "context_pack_file_count": runtime_lane_context_pack_file_count(&context_pack),
                        "context_pack_chars": context_pack.chars().count(),
                        "provider_latency_ms": model_call_ms,
                        "time_to_first_tool_ms": Value::Null,
                    },
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": model_call_ms,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            ));
        }
    };
    let mut candidate = match runtime_lane_model_manifest_candidate_from_output(
        &provider_response.output,
        tools,
        capability_packs,
        permissions,
    ) {
        Ok(candidate) => candidate,
        Err(failure) => {
            let repair_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                prompt,
                &workspace_root,
                &context_pack,
                "bounded_existing_project_manifest_repair",
                &failure.failure_code,
                &failure.failure_message,
                failure.needed_input.as_deref(),
            );
            match provider_client.complete(&ProviderRequest {
                prompt: repair_prompt,
                system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
                tools: Vec::new(),
                model: fast_lane_repair_model.clone(),
                metadata: json!({
                    "provider_timeout_seconds": manifest_repair_provider_timeout_seconds,
                    "omit_ollama_thinking_flags": runtime_lane_metadata_bool(
                        metadata,
                        &[
                            "/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                            "/workflow/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                            "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                            "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                        ],
                    )
                    .unwrap_or(true),
                    "lane": "bounded_existing_project_edit_loop",
                    "attempt": "manifest_repair",
                    "previous_failure_code": failure.failure_code,
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                Ok(repair_response) => {
                    match runtime_lane_model_manifest_candidate_from_output(
                        &repair_response.output,
                        tools,
                        capability_packs,
                        permissions,
                    ) {
                        Ok(candidate) => {
                            provider_response = repair_response;
                            candidate
                        }
                        Err(repair_failure) => {
                            let model_call_ms = model_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                            return Some(runtime_lane_fail_closed_with_state(
                                "runtime_lane_bounded_existing_project_edit_loop_manifest_failed",
                                json!({
                                    "lane": "bounded_existing_project_edit_loop",
                                    "failure_code": repair_failure.failure_code,
                                    "failure_message": repair_failure.failure_message,
                                    "needed_input": repair_failure.needed_input,
                                    "provider_output_preview": repair_failure.provider_output_preview,
                                    "previous_failure_code": failure.failure_code,
                                    "planner_model": fast_lane_repair_model.clone(),
                                    "coding_runtime_probe": {
                                        "selected_planning_depth": planning_depth.to_json(),
                                        "manifest_provider_timeout_seconds": manifest_provider_timeout_seconds,
                                        "repair_provider_timeout_seconds": manifest_repair_provider_timeout_seconds,
                                        "prompt_chars": manifest_prompt.chars().count(),
                                        "system_chars": manifest_system.chars().count(),
                                        "context_pack_file_count": runtime_lane_context_pack_file_count(&context_pack),
                                        "context_pack_chars": context_pack.chars().count(),
                                        "provider_latency_ms": model_call_ms,
                                        "time_to_first_tool_ms": Value::Null,
                                    },
                                    "phase_latency_ms": {
                                        "workflow_load": 0,
                                        "execution_shape_gate": execution_shape_gate_ms,
                                        "provider_start": 0,
                                        "model_call": model_call_ms,
                                        "tool_dispatch": 0,
                                        "mutation": 0,
                                        "validation": 0,
                                        "repair": 0,
                                        "final_synthesis": 0,
                                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                                    }
                                }),
                                permissions,
                                wasm_sandbox,
                                voice_session,
                                state_path,
                                durable_state,
                            ));
                        }
                    }
                }
                Err(error) => {
                    let model_call_ms = model_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_bounded_existing_project_edit_loop_manifest_repair_failed",
                        json!({
                            "lane": "bounded_existing_project_edit_loop",
                            "failure_code": error.code.as_str(),
                            "failure_message": error.message,
                            "previous_failure_code": failure.failure_code,
                            "planner_model": fast_lane_repair_model.clone(),
                            "coding_runtime_probe": {
                                "selected_planning_depth": planning_depth.to_json(),
                                "manifest_provider_timeout_seconds": manifest_provider_timeout_seconds,
                                "repair_provider_timeout_seconds": manifest_repair_provider_timeout_seconds,
                                "prompt_chars": manifest_prompt.chars().count(),
                                "system_chars": manifest_system.chars().count(),
                                "context_pack_file_count": runtime_lane_context_pack_file_count(&context_pack),
                                "context_pack_chars": context_pack.chars().count(),
                                "provider_latency_ms": model_call_ms,
                                "time_to_first_tool_ms": Value::Null,
                            },
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            }
        }
    };
    let mut public_api_violations = runtime_lane_public_api_manifest_violations(
        &candidate,
        &public_api_bindings,
        &workspace_root,
    );
    if !public_api_violations.is_empty() {
        if let Some(retargeted_candidate) = runtime_lane_public_api_owner_retarget_candidate(
            &candidate,
            &public_api_bindings,
            &workspace_root,
        ) {
            let retargeted_violations = runtime_lane_public_api_manifest_violations(
                &retargeted_candidate,
                &public_api_bindings,
                &workspace_root,
            );
            if retargeted_violations.is_empty() {
                candidate = retargeted_candidate;
                public_api_violations = Vec::new();
            } else {
                public_api_violations = retargeted_violations;
            }
        }
    }
    if !public_api_violations.is_empty() {
        let repair_prompt = runtime_lane_model_manifest_public_api_repair_prompt(
            prompt,
            &workspace_root,
            &context_pack,
            &public_api_bindings,
            &public_api_violations,
        );
        match provider_client.complete(&ProviderRequest {
            prompt: repair_prompt,
            system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
            tools: Vec::new(),
            model: fast_lane_repair_model.clone(),
            metadata: json!({
                "provider_timeout_seconds": metadata
                    .pointer("/native_success_criteria/fast_lane_repair_provider_timeout_seconds")
                    .and_then(Value::as_u64)
                    .or_else(|| metadata.pointer("/workflow/native_success_criteria/fast_lane_repair_provider_timeout_seconds").and_then(Value::as_u64))
                    .unwrap_or(45),
                "omit_ollama_thinking_flags": runtime_lane_metadata_bool(
                    metadata,
                    &[
                        "/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                        "/workflow/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                        "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                        "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                    ],
                )
                .unwrap_or(true),
                "lane": "bounded_existing_project_edit_loop",
                "attempt": "public_api_owner_binding_repair",
                "public_api_bindings": runtime_lane_public_api_bindings_json(&public_api_bindings, &workspace_root),
                "previous_public_api_violations": public_api_violations.clone(),
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
            }),
        }) {
            Ok(repair_response) => {
                let mut repair_candidate = match runtime_lane_model_manifest_candidate_from_output(
                    &repair_response.output,
                    tools,
                    capability_packs,
                    permissions,
                ) {
                    Ok(candidate) => candidate,
                    Err(repair_failure) => {
                        let model_call_ms =
                            model_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                        return Some(runtime_lane_fail_closed_with_state(
                            "runtime_lane_bounded_existing_project_public_api_gate_failed",
                            json!({
                                "lane": "bounded_existing_project_edit_loop",
                                "failure_code": repair_failure.failure_code,
                                "failure_message": repair_failure.failure_message,
                                "needed_input": repair_failure.needed_input,
                                "provider_output_preview": repair_failure.provider_output_preview,
                                "public_api_bindings": runtime_lane_public_api_bindings_json(&public_api_bindings, &workspace_root),
                                "previous_public_api_violations": public_api_violations,
                                "planner_model": fast_lane_repair_model.clone(),
                                "phase_latency_ms": {
                                    "workflow_load": 0,
                                    "execution_shape_gate": execution_shape_gate_ms,
                                    "provider_start": 0,
                                    "model_call": model_call_ms,
                                    "tool_dispatch": 0,
                                    "mutation": 0,
                                    "validation": 0,
                                    "repair": 0,
                                    "final_synthesis": 0,
                                    "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                                }
                            }),
                            permissions,
                            wasm_sandbox,
                            voice_session,
                            state_path,
                            durable_state,
                        ));
                    }
                };
                let mut repair_violations = runtime_lane_public_api_manifest_violations(
                    &repair_candidate,
                    &public_api_bindings,
                    &workspace_root,
                );
                if !repair_violations.is_empty() {
                    if let Some(retargeted_candidate) = runtime_lane_public_api_owner_retarget_candidate(
                        &repair_candidate,
                        &public_api_bindings,
                        &workspace_root,
                    ) {
                        let retargeted_violations = runtime_lane_public_api_manifest_violations(
                            &retargeted_candidate,
                            &public_api_bindings,
                            &workspace_root,
                        );
                        if retargeted_violations.is_empty() {
                            repair_candidate = retargeted_candidate;
                            repair_violations = Vec::new();
                        } else {
                            repair_violations = retargeted_violations;
                        }
                    }
                }
                if !repair_violations.is_empty() {
                    let model_call_ms =
                        model_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_bounded_existing_project_public_api_gate_failed",
                        json!({
                            "lane": "bounded_existing_project_edit_loop",
                            "failure_code": "public_api_owner_binding_violation",
                            "failure_message": "The planner attempted to satisfy imported public symbols outside their owning module after repair.",
                            "needed_input": "Return a manifest that edits the owner module named by the import surface.",
                            "public_api_bindings": runtime_lane_public_api_bindings_json(&public_api_bindings, &workspace_root),
                            "public_api_violations": repair_violations,
                            "planner_model": fast_lane_repair_model.clone(),
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
                candidate = repair_candidate;
                provider_response = repair_response;
            }
            Err(error) => {
                let model_call_ms =
                    model_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                return Some(runtime_lane_fail_closed_with_state(
                    "runtime_lane_bounded_existing_project_public_api_gate_failed",
                    json!({
                        "lane": "bounded_existing_project_edit_loop",
                        "failure_code": error.code.as_str(),
                        "failure_message": error.message,
                        "public_api_bindings": runtime_lane_public_api_bindings_json(&public_api_bindings, &workspace_root),
                        "public_api_violations": public_api_violations,
                        "planner_model": fast_lane_repair_model.clone(),
                        "phase_latency_ms": {
                            "workflow_load": 0,
                            "execution_shape_gate": execution_shape_gate_ms,
                            "provider_start": 0,
                            "model_call": model_call_ms,
                            "tool_dispatch": 0,
                            "mutation": 0,
                            "validation": 0,
                            "repair": 0,
                            "final_synthesis": 0,
                            "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                        }
                    }),
                    permissions,
                    wasm_sandbox,
                    voice_session,
                    state_path,
                    durable_state,
                ));
            }
        }
    }
    let validation_command = runtime_lane_extract_validation_command(prompt, &workspace_root);
    let mut validation_call_id = None;
    runtime_lane_attach_semantic_probe_action(
        &mut candidate,
        validation_command.as_ref(),
        "bounded_existing_project_edit_loop_validation",
        &mut validation_call_id,
    );
    if validation_command.is_some() {
        candidate.requires_validation = true;
    }
    let semantic_probe = runtime_lane_extract_or_default_semantic_probe_command(prompt, &workspace_root);
    let mut semantic_probe_call_id = None;
    runtime_lane_attach_semantic_probe_action(
        &mut candidate,
        semantic_probe.as_ref(),
        "bounded_existing_project_edit_loop",
        &mut semantic_probe_call_id,
    );
    if let Some(gate_report) =
        runtime_lane_capability_evidence_gate_report(metadata, prompt, &candidate)
    {
        if !gate_report.missing.is_empty() {
            let gate_hint = runtime_lane_capability_evidence_gate_retry_hint(&gate_report);
            let repair_context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
            let retry_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                prompt,
                &workspace_root,
                &repair_context_pack,
                "bounded_existing_project_capability_evidence_gate_repair",
                "capability_evidence_gate_failed",
                &runtime_lane_capability_evidence_gate_message(&gate_report),
                Some(&gate_hint),
            );
            let retry_response = match provider_client.complete(&ProviderRequest {
                prompt: retry_prompt,
                system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
                tools: Vec::new(),
                model: fast_lane_repair_model.clone(),
                metadata: json!({
                    "provider_timeout_seconds": manifest_repair_provider_timeout_seconds,
                    "omit_ollama_thinking_flags": runtime_lane_metadata_bool(
                        metadata,
                        &[
                            "/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                            "/workflow/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                            "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                            "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                        ],
                    )
                    .unwrap_or(true),
                    "lane": "bounded_existing_project_edit_loop",
                    "attempt": "capability_evidence_gate_repair",
                    "previous_failure_code": "capability_evidence_gate_failed",
                    "missing_capability_evidence": gate_report.missing.clone(),
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                Ok(response) => response,
                Err(retry_error) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_bounded_existing_project_capability_evidence_gate_provider_failed",
                        json!({
                            "lane": "bounded_existing_project_edit_loop",
                            "failure_code": retry_error.code.as_str(),
                            "failure_message": retry_error.message,
                            "missing_capability_evidence": gate_report.missing,
                            "present_capability_evidence": gate_report.present,
                            "required_capability_evidence": gate_report.required,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            };
            provider_response = retry_response;
            candidate = match runtime_lane_model_manifest_candidate_from_output(
                &provider_response.output,
                tools,
                capability_packs,
                permissions,
            ) {
                Ok(mut retry_candidate) => {
                    validation_call_id = None;
                    runtime_lane_attach_semantic_probe_action(
                        &mut retry_candidate,
                        validation_command.as_ref(),
                        "bounded_existing_project_capability_repair_validation",
                        &mut validation_call_id,
                    );
                    if validation_command.is_some() {
                        retry_candidate.requires_validation = true;
                    }
                    semantic_probe_call_id = None;
                    runtime_lane_attach_semantic_probe_action(
                        &mut retry_candidate,
                        semantic_probe.as_ref(),
                        "bounded_existing_project_capability_repair",
                        &mut semantic_probe_call_id,
                    );
                    if let Some(retry_gate_report) =
                        runtime_lane_capability_evidence_gate_report(metadata, prompt, &retry_candidate)
                    {
                        if !retry_gate_report.missing.is_empty() {
                            let model_call_ms = model_started
                                .elapsed()
                                .as_millis()
                                .min(u128::from(u64::MAX)) as u64;
                            return Some(runtime_lane_fail_closed_with_state(
                                "runtime_lane_bounded_existing_project_capability_evidence_gate_failed",
                                json!({
                                    "lane": "bounded_existing_project_edit_loop",
                                    "failure_code": "capability_evidence_gate_failed",
                                    "failure_message": runtime_lane_capability_evidence_gate_message(&retry_gate_report),
                                    "missing_capability_evidence": retry_gate_report.missing,
                                    "present_capability_evidence": retry_gate_report.present,
                                    "required_capability_evidence": retry_gate_report.required,
                                    "repair_attempted": true,
                                    "phase_latency_ms": {
                                        "workflow_load": 0,
                                        "execution_shape_gate": execution_shape_gate_ms,
                                        "provider_start": 0,
                                        "model_call": model_call_ms,
                                        "tool_dispatch": 0,
                                        "mutation": 0,
                                        "validation": 0,
                                        "repair": 0,
                                        "final_synthesis": 0,
                                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                                    }
                                }),
                                permissions,
                                wasm_sandbox,
                                voice_session,
                                state_path,
                                durable_state,
                            ));
                        }
                    }
                    retry_candidate
                }
                Err(retry_failure) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_bounded_existing_project_capability_evidence_manifest_failed",
                        json!({
                            "lane": "bounded_existing_project_edit_loop",
                            "failure_code": retry_failure.failure_code,
                            "failure_message": retry_failure.failure_message,
                            "needed_input": retry_failure.needed_input,
                            "provider_output_preview": retry_failure.provider_output_preview,
                            "missing_capability_evidence": gate_report.missing,
                            "present_capability_evidence": gate_report.present,
                            "required_capability_evidence": gate_report.required,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            };
        }
    }
    let model_call_ms = model_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let dispatch_started = Instant::now();
    let mut receipts = runtime_lane_dispatch_model_manifest_actions(
        &candidate,
        "bounded_existing_project_edit_loop",
    );
    if !receipts
        .iter()
        .any(runtime_lane_receipt_is_successful_mutation)
    {
        if let Some(failure_summary) = runtime_lane_first_receipt_failure_summary(&receipts) {
            let repair_context_pack =
                runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
            let repair_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                prompt,
                &workspace_root,
                &repair_context_pack,
                "bounded_existing_project_tool_dispatch_repair",
                "tool_dispatch_failed",
                &failure_summary,
                Some("Return a corrected manifest that avoids the rejected action, preserves existing public exports, mutates source/tests, and runs validation plus the semantic probe when supplied."),
            );
            if let Ok(repair_response) = provider_client.complete(&ProviderRequest {
                prompt: repair_prompt,
                system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
                tools: Vec::new(),
                model: fast_lane_repair_model.clone(),
                metadata: json!({
                    "provider_timeout_seconds": metadata
                        .pointer("/native_success_criteria/fast_lane_repair_provider_timeout_seconds")
                        .and_then(Value::as_u64)
                        .or_else(|| metadata.pointer("/workflow/native_success_criteria/fast_lane_repair_provider_timeout_seconds").and_then(Value::as_u64))
                        .unwrap_or(45),
                    "omit_ollama_thinking_flags": runtime_lane_metadata_bool(
                        metadata,
                        &[
                            "/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                            "/workflow/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                            "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                            "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                        ],
                    )
                    .unwrap_or(true),
                    "lane": "bounded_existing_project_edit_loop",
                    "attempt": "tool_dispatch_repair",
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                if let Ok(mut repair_candidate) = runtime_lane_model_manifest_candidate_from_output(
                    &repair_response.output,
                    tools,
                    capability_packs,
                    permissions,
                ) {
                    let mut repair_public_api_violations = runtime_lane_public_api_manifest_violations(
                        &repair_candidate,
                        &public_api_bindings,
                        &workspace_root,
                    );
                    if !repair_public_api_violations.is_empty() {
                        if let Some(retargeted_candidate) = runtime_lane_public_api_owner_retarget_candidate(
                            &repair_candidate,
                            &public_api_bindings,
                            &workspace_root,
                        ) {
                            let retargeted_violations = runtime_lane_public_api_manifest_violations(
                                &retargeted_candidate,
                                &public_api_bindings,
                                &workspace_root,
                            );
                            if retargeted_violations.is_empty() {
                                repair_candidate = retargeted_candidate;
                                repair_public_api_violations = Vec::new();
                            } else {
                                repair_public_api_violations = retargeted_violations;
                            }
                        }
                    }
                    if repair_public_api_violations.is_empty() {
                        let mut repair_probe_call_id = None;
                        let mut repair_validation_call_id = None;
                        runtime_lane_attach_semantic_probe_action(
                            &mut repair_candidate,
                            validation_command.as_ref(),
                            "bounded_existing_project_edit_loop_tool_repair_validation",
                            &mut repair_validation_call_id,
                        );
                        runtime_lane_attach_semantic_probe_action(
                            &mut repair_candidate,
                            semantic_probe.as_ref(),
                            "bounded_existing_project_edit_loop_tool_repair",
                            &mut repair_probe_call_id,
                        );
                        let repair_receipts = runtime_lane_dispatch_model_manifest_actions(
                            &repair_candidate,
                            "bounded_existing_project_edit_loop_tool_repair",
                        );
                        if repair_receipts
                            .iter()
                            .any(runtime_lane_receipt_is_successful_mutation)
                        {
                            candidate = repair_candidate;
                            receipts = repair_receipts;
                            provider_response = repair_response;
                            if repair_probe_call_id.is_some() {
                                semantic_probe_call_id = repair_probe_call_id;
                            }
                        }
                    }
                }
            }
        }
    }
    if runtime_lane_receipts_need_repair(&receipts, candidate.requires_validation)
        && receipts
            .iter()
            .any(runtime_lane_receipt_is_successful_mutation)
    {
        runtime_lane_append_explicit_validation_recheck(
            &mut receipts,
            &workspace_root,
            validation_command.as_ref(),
            semantic_probe.as_ref(),
            "bounded_existing_project_edit_loop_validation_recheck",
        );
    }
    let validation_repair_has_mutation = receipts
        .iter()
        .any(runtime_lane_receipt_is_successful_mutation);
    if runtime_lane_receipts_need_repair(&receipts, candidate.requires_validation)
        && validation_repair_has_mutation
    {
        if let Some(initial_failure_summary) = runtime_lane_first_receipt_failure_summary(&receipts)
        {
            let mut repair_failure_summary = initial_failure_summary;
            let max_validation_repair_attempts = 2;
            for repair_attempt_index in 0..max_validation_repair_attempts {
                let repair_context_pack = runtime_lane_model_manifest_context_pack_with_failure(
                    prompt,
                    &workspace_root,
                    &repair_failure_summary,
                );
                let repair_prompt = runtime_lane_model_manifest_validation_repair_prompt(
                    prompt,
                    &workspace_root,
                    &repair_context_pack,
                    &repair_failure_summary,
                );
                let repair_response = match provider_client.complete(&ProviderRequest {
                    prompt: repair_prompt,
                    system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
                    tools: Vec::new(),
                    model: fast_lane_repair_model.clone(),
                    metadata: json!({
                        "provider_timeout_seconds": metadata
                            .pointer("/native_success_criteria/fast_lane_repair_provider_timeout_seconds")
                            .and_then(Value::as_u64)
                            .or_else(|| metadata.pointer("/workflow/native_success_criteria/fast_lane_repair_provider_timeout_seconds").and_then(Value::as_u64))
                            .unwrap_or(45),
                        "omit_ollama_thinking_flags": runtime_lane_metadata_bool(
                            metadata,
                            &[
                                "/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                                "/workflow/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                                "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                                "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                            ],
                        )
                        .unwrap_or(true),
                        "lane": "bounded_existing_project_edit_loop",
                        "attempt": "validation_repair",
                        "repair_attempt_index": repair_attempt_index,
                        "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                    }),
                }) {
                    Ok(response) => response,
                    Err(error) => {
                        let failure_message = format!("{:?}", error);
                        receipts.push(runtime_lane_validation_repair_failure_receipt(
                            &format!(
                                "bounded_existing_project_edit_loop_validation_repair_provider_{}",
                                repair_attempt_index + 1
                            ),
                            "provider_call_failed",
                            &failure_message,
                            json!({"repair_attempt_index": repair_attempt_index}),
                        ));
                        repair_failure_summary = format!(
                            "Validation repair provider call failed on attempt {}.\nerror: {:?}",
                            repair_attempt_index + 1,
                            error
                        );
                        continue;
                    }
                };
                let mut repair_candidate = match runtime_lane_model_manifest_candidate_from_output(
                    &repair_response.output,
                    tools,
                    capability_packs,
                    permissions,
                ) {
                    Ok(candidate) => candidate,
                    Err(repair_failure) => {
                        let failure_message = format!(
                            "{}: {}",
                            repair_failure.failure_code, repair_failure.failure_message
                        );
                        receipts.push(runtime_lane_validation_repair_failure_receipt(
                            &format!(
                                "bounded_existing_project_edit_loop_validation_repair_manifest_{}",
                                repair_attempt_index + 1
                            ),
                            "manifest_not_executable",
                            &failure_message,
                            json!({
                                "repair_attempt_index": repair_attempt_index,
                                "needed_input": repair_failure.needed_input,
                                "provider_output_preview": repair_failure.provider_output_preview,
                            }),
                        ));
                        repair_failure_summary = format!(
                            "Validation repair manifest was not executable on attempt {}.\nfailure_code: {}\nfailure_message: {}\nneeded_input: {:?}\nprovider_output_preview: {}",
                            repair_attempt_index + 1,
                            repair_failure.failure_code,
                            repair_failure.failure_message,
                            repair_failure.needed_input,
                            repair_failure.provider_output_preview
                        );
                        continue;
                    }
                };
                let mut repair_public_api_violations = runtime_lane_public_api_manifest_violations(
                    &repair_candidate,
                    &public_api_bindings,
                    &workspace_root,
                );
                if !repair_public_api_violations.is_empty() {
                    if let Some(retargeted_candidate) =
                        runtime_lane_public_api_owner_retarget_candidate(
                            &repair_candidate,
                            &public_api_bindings,
                            &workspace_root,
                        )
                    {
                        let retargeted_violations = runtime_lane_public_api_manifest_violations(
                            &retargeted_candidate,
                            &public_api_bindings,
                            &workspace_root,
                        );
                        if retargeted_violations.is_empty() {
                            repair_candidate = retargeted_candidate;
                            repair_public_api_violations = Vec::new();
                        } else {
                            repair_public_api_violations = retargeted_violations;
                        }
                    }
                }
                if !repair_public_api_violations.is_empty() {
                    let failure_message = repair_public_api_violations.join("\n");
                    receipts.push(runtime_lane_validation_repair_failure_receipt(
                        &format!(
                            "bounded_existing_project_edit_loop_validation_repair_public_api_{}",
                            repair_attempt_index + 1
                        ),
                        "public_api_owner_binding_violation",
                        &failure_message,
                        json!({
                            "repair_attempt_index": repair_attempt_index,
                            "public_api_bindings": runtime_lane_public_api_bindings_json(&public_api_bindings, &workspace_root),
                        }),
                    ));
                    repair_failure_summary = format!(
                        "Validation repair manifest violated public API owner bindings on attempt {}.\n{}",
                        repair_attempt_index + 1,
                        failure_message
                    );
                    continue;
                }
                let mut repair_probe_call_id = None;
                let mut repair_validation_call_id = None;
                runtime_lane_attach_semantic_probe_action(
                    &mut repair_candidate,
                    validation_command.as_ref(),
                    "bounded_existing_project_edit_loop_validation_repair_validation",
                    &mut repair_validation_call_id,
                );
                if validation_command.is_some() {
                    repair_candidate.requires_validation = true;
                }
                runtime_lane_attach_semantic_probe_action(
                    &mut repair_candidate,
                    semantic_probe.as_ref(),
                    "bounded_existing_project_edit_loop_validation_repair",
                    &mut repair_probe_call_id,
                );
                let mut repair_receipts = runtime_lane_dispatch_model_manifest_actions(
                    &repair_candidate,
                    "bounded_existing_project_edit_loop_validation_repair",
                );
                if runtime_lane_receipts_need_repair(
                    &repair_receipts,
                    repair_candidate.requires_validation,
                ) && repair_receipts
                    .iter()
                    .any(runtime_lane_receipt_is_successful_mutation)
                {
                    runtime_lane_append_explicit_validation_recheck(
                        &mut repair_receipts,
                        &workspace_root,
                        validation_command.as_ref(),
                        semantic_probe.as_ref(),
                        "bounded_existing_project_edit_loop_validation_repair_recheck",
                    );
                }
                let repair_success = repair_receipts
                    .iter()
                    .any(runtime_lane_receipt_is_successful_mutation)
                    && runtime_lane_receipts_validation_ok(
                        &repair_receipts,
                        repair_candidate.requires_validation,
                    );
                if repair_success {
                    candidate = repair_candidate;
                    receipts = repair_receipts;
                    provider_response = repair_response;
                    if repair_probe_call_id.is_some() {
                        semantic_probe_call_id = repair_probe_call_id;
                    }
                    break;
                }
                if repair_probe_call_id.is_some() {
                    semantic_probe_call_id = repair_probe_call_id;
                }
                if let Some(next_failure_summary) =
                    runtime_lane_first_receipt_failure_summary(&repair_receipts)
                {
                    repair_failure_summary = next_failure_summary;
                } else {
                    repair_failure_summary = format!(
                        "Validation repair attempt {} did not produce passing validation receipts.",
                        repair_attempt_index + 1
                    );
                }
                candidate = repair_candidate;
                receipts = repair_receipts;
            }
        }
    } else if runtime_lane_receipts_need_repair(&receipts, candidate.requires_validation)
        && !validation_repair_has_mutation
    {
        receipts.push(runtime_lane_validation_repair_failure_receipt(
            "bounded_existing_project_edit_loop_pre_mutation_validation_repair_blocked",
            "pre_mutation_validation_repair_blocked",
            "Validation repair is blocked until at least one source or test mutation receipt exists. Baseline validation failure is context for the first edit, not a repair stage.",
            json!({
                "required_prior_receipt": "file_write_or_file_patch",
                "blocked_stage": "validation_repair",
                "reason": "baseline_validation_before_mutation"
            }),
        ));
    }
    if let Some(failure) =
        runtime_lane_semantic_probe_failure(&receipts, semantic_probe_call_id.as_deref())
    {
        let repair_context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
        if let Ok(repair_response) = provider_client.complete(&ProviderRequest {
            prompt: runtime_lane_model_manifest_semantic_repair_prompt(
                prompt,
                &workspace_root,
                &repair_context_pack,
                &failure,
            ),
            system: Some(runtime_lane_bounded_existing_project_edit_loop_system()),
            tools: Vec::new(),
                model: fast_lane_repair_model.clone(),
                metadata: json!({
                "provider_timeout_seconds": metadata
                    .pointer("/native_success_criteria/fast_lane_repair_provider_timeout_seconds")
                    .and_then(Value::as_u64)
                    .or_else(|| metadata.pointer("/workflow/native_success_criteria/fast_lane_repair_provider_timeout_seconds").and_then(Value::as_u64))
                    .unwrap_or(45),
                "omit_ollama_thinking_flags": runtime_lane_metadata_bool(
                    metadata,
                    &[
                        "/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                        "/workflow/native_success_criteria/fast_lane_repair_omit_ollama_thinking_flags",
                        "/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                        "/workflow/native_success_criteria/fast_lane_omit_ollama_thinking_flags",
                    ],
                )
                .unwrap_or(true),
                "lane": "bounded_existing_project_edit_loop",
                "attempt": "post_validation_semantic_completion_repair",
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
            }),
        }) {
            if let Ok(mut repair_candidate) = runtime_lane_model_manifest_candidate_from_output(
                &repair_response.output,
                tools,
                capability_packs,
                permissions,
            ) {
                let mut repair_public_api_violations = runtime_lane_public_api_manifest_violations(
                    &repair_candidate,
                    &public_api_bindings,
                    &workspace_root,
                );
                if !repair_public_api_violations.is_empty() {
                    if let Some(retargeted_candidate) = runtime_lane_public_api_owner_retarget_candidate(
                        &repair_candidate,
                        &public_api_bindings,
                        &workspace_root,
                    ) {
                        let retargeted_violations = runtime_lane_public_api_manifest_violations(
                            &retargeted_candidate,
                            &public_api_bindings,
                            &workspace_root,
                        );
                        if retargeted_violations.is_empty() {
                            repair_candidate = retargeted_candidate;
                            repair_public_api_violations = Vec::new();
                        } else {
                            repair_public_api_violations = retargeted_violations;
                        }
                    }
                }
                if repair_public_api_violations.is_empty() {
                    let mut repair_probe_call_id = None;
                    let mut repair_validation_call_id = None;
                    runtime_lane_attach_semantic_probe_action(
                        &mut repair_candidate,
                        validation_command.as_ref(),
                        "bounded_existing_project_edit_loop_semantic_repair_validation",
                        &mut repair_validation_call_id,
                    );
                    runtime_lane_attach_semantic_probe_action(
                        &mut repair_candidate,
                        semantic_probe.as_ref(),
                        "bounded_existing_project_edit_loop_semantic_repair",
                        &mut repair_probe_call_id,
                    );
                    let mut repair_receipts = runtime_lane_dispatch_model_manifest_actions(
                        &repair_candidate,
                        "bounded_existing_project_edit_loop_semantic_repair",
                    );
                    receipts.append(&mut repair_receipts);
                } else {
                    let failure_message = repair_public_api_violations.join("\n");
                    receipts.push(runtime_lane_validation_repair_failure_receipt(
                        "bounded_existing_project_edit_loop_semantic_repair_public_api",
                        "public_api_owner_binding_violation",
                        &failure_message,
                        json!({
                            "public_api_bindings": runtime_lane_public_api_bindings_json(&public_api_bindings, &workspace_root),
                        }),
                    ));
                }
            }
        }
    }
    let tool_dispatch_ms = dispatch_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let mut response = runtime_lane_model_manifest_planner_response(
        name,
        metadata,
        prompt,
        tools,
        capability_packs,
        required_pack_permissions,
        permissions,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
        state_path,
        durable_state,
        candidate,
        receipts,
        provider_response,
        execution_shape_gate_ms,
        model_call_ms,
        tool_dispatch_ms,
        total_started,
    );
    runtime_lane_attach_coding_runtime_probe(
        &mut response,
        json!({
            "selected_planning_depth": planning_depth.to_json(),
            "manifest_provider_timeout_seconds": manifest_provider_timeout_seconds,
            "repair_provider_timeout_seconds": manifest_repair_provider_timeout_seconds,
            "prompt_chars": manifest_prompt.chars().count(),
            "system_chars": manifest_system.chars().count(),
            "context_pack_file_count": runtime_lane_context_pack_file_count(&context_pack),
            "context_pack_chars": context_pack.chars().count(),
            "provider_latency_ms": model_call_ms,
            "time_to_first_tool_ms": execution_shape_gate_ms.saturating_add(model_call_ms),
        }),
    );
    runtime_lane_relabel_generated_manifest_response(
        &mut response,
        "bounded_existing_project_edit_loop",
        "runtime_lane_bounded_existing_project_edit_loop_failed",
        "compact_existing_project_patch_first_loop",
    );
    Some(response)
}

fn runtime_lane_try_model_manifest_planner(
    name: &str,
    prompt: &str,
    preamble: Option<&str>,
    provider: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    providers: &ProviderClientRegistry,
) -> Option<RuntimeLaneResponse> {
    if !runtime_lane_model_manifest_planner_eligible(
        metadata,
        prompt,
        tools,
        capability_packs,
        permissions,
    ) {
        return None;
    }
    let total_started = Instant::now();
    let gate_started = Instant::now();
    let workspace_root = runtime_lane_extract_workspace_root(prompt)?;
    let execution_shape_gate_ms =
        gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
    let provider_id = provider.unwrap_or_else(|| providers.default_provider_id());
    let provider_client = match providers.from_provider_id(provider_id) {
        Ok(provider) => provider,
        Err(error) => {
            return Some(runtime_lane_fail_closed_with_state(
                "runtime_lane_model_manifest_provider_unavailable",
                json!({
                    "lane": "model_manifest_planner",
                    "failure_code": error.code.as_str(),
                    "failure_message": error.message,
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": 0,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            ));
        }
    };
    let model_started = Instant::now();
    let planner_system = if runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/model_manifest_compact_system",
            "/workflow/native_success_criteria/model_manifest_compact_system",
        ],
    )
    .unwrap_or(false)
    {
        runtime_lane_model_manifest_planner_system_compact(preamble)
    } else {
        runtime_lane_model_manifest_planner_system(preamble)
    };
    let mut provider_response = match provider_client.complete(&ProviderRequest {
        prompt: runtime_lane_model_manifest_planner_prompt(prompt, &workspace_root, &context_pack),
        system: Some(planner_system.clone()),
        tools: Vec::new(),
        model: model.cloned(),
        metadata: json!({
            "provider_timeout_seconds": metadata
                .pointer("/native_success_criteria/provider_timeout_seconds")
                .and_then(Value::as_u64)
                .or_else(|| metadata.pointer("/workflow/native_success_criteria/provider_timeout_seconds").and_then(Value::as_u64))
                .unwrap_or(90),
            "lane": "model_manifest_planner",
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
        }),
    }) {
        Ok(response) => response,
        Err(error) => {
            if runtime_lane_provider_error_is_timeout(error.code.as_str(), &error.message) {
                let retry_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                    prompt,
                    &workspace_root,
                    &context_pack,
                    "provider_timeout_recovery",
                    error.code.as_str(),
                    &error.message,
                    Some("Return the smallest deterministic manifest that mutates source/tests and runs validation plus any semantic probe command from the task."),
                );
                match provider_client.complete(&ProviderRequest {
                    prompt: retry_prompt,
                    system: Some(planner_system.clone()),
                    tools: Vec::new(),
                    model: model.cloned(),
                    metadata: json!({
                        "provider_timeout_seconds": metadata
                            .pointer("/native_success_criteria/recovery_provider_timeout_seconds")
                            .and_then(Value::as_u64)
                            .or_else(|| metadata.pointer("/workflow/native_success_criteria/recovery_provider_timeout_seconds").and_then(Value::as_u64))
                            .unwrap_or(90)
                            .max(90),
                        "lane": "model_manifest_planner",
                        "attempt": "provider_timeout_recovery",
                        "previous_failure_code": error.code.as_str(),
                        "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                    }),
                }) {
                    Ok(response) => response,
                    Err(recovery_error) => {
                        let model_call_ms = model_started
                            .elapsed()
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64;
                        return Some(runtime_lane_fail_closed_with_state(
                            "runtime_lane_model_manifest_provider_failed",
                            json!({
                                "lane": "model_manifest_planner",
                                "failure_code": recovery_error.code.as_str(),
                                "failure_message": recovery_error.message,
                                "first_failure_code": error.code.as_str(),
                                "first_failure_message": error.message,
                                "repair_attempted": true,
                                "recovery_artifact": {
                                    "kind": "provider_timeout_recovery_artifact_v1",
                                    "status": "provider_timeout_before_mutation",
                                    "next_action": "retry_with_compact_context_or_fallback_lane",
                                    "mutation_receipts_present": false,
                                    "success_claim_allowed": false
                                },
                                "phase_latency_ms": {
                                    "workflow_load": 0,
                                    "execution_shape_gate": execution_shape_gate_ms,
                                    "provider_start": 0,
                                    "model_call": model_call_ms,
                                    "tool_dispatch": 0,
                                    "mutation": 0,
                                    "validation": 0,
                                    "repair": 0,
                                    "final_synthesis": 0,
                                    "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                                }
                            }),
                            permissions,
                            wasm_sandbox,
                            voice_session,
                            state_path,
                            durable_state,
                        ));
                    }
                }
            } else {
            let retry_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                prompt,
                &workspace_root,
                &context_pack,
                "provider_failed",
                error.code.as_str(),
                &error.message,
                None,
            );
            match provider_client.complete(&ProviderRequest {
                prompt: retry_prompt,
                system: Some(planner_system.clone()),
                tools: Vec::new(),
                model: model.cloned(),
                metadata: json!({
                    "provider_timeout_seconds": metadata
                        .pointer("/native_success_criteria/provider_timeout_seconds")
                        .and_then(Value::as_u64)
                        .or_else(|| metadata.pointer("/workflow/native_success_criteria/provider_timeout_seconds").and_then(Value::as_u64))
                        .unwrap_or(90),
                    "lane": "model_manifest_planner",
                    "attempt": "provider_retry",
                    "previous_failure_code": error.code.as_str(),
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                Ok(response) => response,
                Err(retry_error) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_model_manifest_provider_failed",
                        json!({
                            "lane": "model_manifest_planner",
                            "failure_code": retry_error.code.as_str(),
                            "failure_message": retry_error.message,
                            "first_failure_code": error.code.as_str(),
                            "first_failure_message": error.message,
                            "repair_attempted": true,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            }
            }
        }
    };
    let mut candidate = match runtime_lane_model_manifest_candidate_from_output(
        &provider_response.output,
        tools,
        capability_packs,
        permissions,
    ) {
        Ok(candidate) => candidate,
        Err(first_failure) => {
            let retry_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                prompt,
                &workspace_root,
                &context_pack,
                "manifest_repair",
                &first_failure.failure_code,
                &first_failure.failure_message,
                first_failure.needed_input.as_deref(),
            );
            let retry_response = match provider_client.complete(&ProviderRequest {
                prompt: retry_prompt,
                system: Some(planner_system.clone()),
                tools: Vec::new(),
                model: model.cloned(),
                metadata: json!({
                    "provider_timeout_seconds": metadata
                        .pointer("/native_success_criteria/provider_timeout_seconds")
                        .and_then(Value::as_u64)
                        .or_else(|| metadata.pointer("/workflow/native_success_criteria/provider_timeout_seconds").and_then(Value::as_u64))
                        .unwrap_or(90),
                    "lane": "model_manifest_planner",
                    "attempt": "manifest_repair",
                    "previous_failure_code": first_failure.failure_code.as_str(),
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                Ok(response) => response,
                Err(retry_error) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_model_manifest_repair_provider_failed",
                        json!({
                            "lane": "model_manifest_planner",
                            "failure_code": retry_error.code.as_str(),
                            "failure_message": retry_error.message,
                            "first_failure_code": first_failure.failure_code,
                            "first_failure_message": first_failure.failure_message,
                            "first_needed_input": first_failure.needed_input,
                            "first_provider_output_preview": first_failure.provider_output_preview,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            };
            provider_response = retry_response;
            match runtime_lane_model_manifest_candidate_from_output(
                &provider_response.output,
                tools,
                capability_packs,
                permissions,
            ) {
                Ok(candidate) => candidate,
                Err(retry_failure) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_model_manifest_repair_failed",
                        json!({
                            "lane": "model_manifest_planner",
                            "failure_code": retry_failure.failure_code,
                            "failure_message": retry_failure.failure_message,
                            "needed_input": retry_failure.needed_input,
                            "provider": provider_response.provider,
                            "model": provider_response.model,
                            "provider_output_preview": retry_failure.provider_output_preview,
                            "first_failure_code": first_failure.failure_code,
                            "first_failure_message": first_failure.failure_message,
                            "repair_attempted": true,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            }
        }
    };
    let validation_command = runtime_lane_extract_validation_command(prompt, &workspace_root);
    let mut validation_call_id = None;
    runtime_lane_attach_semantic_probe_action(
        &mut candidate,
        validation_command.as_ref(),
        "model_manifest_planner_validation",
        &mut validation_call_id,
    );
    let semantic_probe = runtime_lane_extract_semantic_probe_command(prompt, &workspace_root);
    let mut semantic_probe_call_id = None;
    runtime_lane_attach_semantic_probe_action(
        &mut candidate,
        semantic_probe.as_ref(),
        "model_manifest_planner",
        &mut semantic_probe_call_id,
    );
    if let Some(gate_report) =
        runtime_lane_capability_evidence_gate_report(metadata, prompt, &candidate)
    {
        if !gate_report.missing.is_empty() {
            let gate_hint = runtime_lane_capability_evidence_gate_retry_hint(&gate_report);
            let retry_prompt = runtime_lane_model_manifest_planner_retry_prompt(
                prompt,
                &workspace_root,
                &context_pack,
                "capability_evidence_gate_repair",
                "capability_evidence_gate_failed",
                &runtime_lane_capability_evidence_gate_message(&gate_report),
                Some(&gate_hint),
            );
            let retry_response = match provider_client.complete(&ProviderRequest {
                prompt: retry_prompt,
                system: Some(planner_system.clone()),
                tools: Vec::new(),
                model: model.cloned(),
                metadata: json!({
                    "provider_timeout_seconds": metadata
                        .pointer("/native_success_criteria/provider_timeout_seconds")
                        .and_then(Value::as_u64)
                        .or_else(|| metadata.pointer("/workflow/native_success_criteria/provider_timeout_seconds").and_then(Value::as_u64))
                        .unwrap_or(90),
                    "lane": "model_manifest_planner",
                    "attempt": "capability_evidence_gate_repair",
                    "previous_failure_code": "capability_evidence_gate_failed",
                    "missing_capability_evidence": gate_report.missing.clone(),
                    "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
                }),
            }) {
                Ok(response) => response,
                Err(retry_error) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_model_manifest_capability_evidence_gate_provider_failed",
                        json!({
                            "lane": "model_manifest_planner",
                            "failure_code": retry_error.code.as_str(),
                            "failure_message": retry_error.message,
                            "missing_capability_evidence": gate_report.missing,
                            "present_capability_evidence": gate_report.present,
                            "required_capability_evidence": gate_report.required,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            };
            provider_response = retry_response;
            candidate = match runtime_lane_model_manifest_candidate_from_output(
                &provider_response.output,
                tools,
                capability_packs,
                permissions,
            ) {
                Ok(mut retry_candidate) => {
                    validation_call_id = None;
                    runtime_lane_attach_semantic_probe_action(
                        &mut retry_candidate,
                        validation_command.as_ref(),
                        "model_manifest_planner_capability_repair_validation",
                        &mut validation_call_id,
                    );
                    semantic_probe_call_id = None;
                    runtime_lane_attach_semantic_probe_action(
                        &mut retry_candidate,
                        semantic_probe.as_ref(),
                        "model_manifest_planner_capability_repair",
                        &mut semantic_probe_call_id,
                    );
                    if let Some(retry_gate_report) =
                        runtime_lane_capability_evidence_gate_report(metadata, prompt, &retry_candidate)
                    {
                        if !retry_gate_report.missing.is_empty() {
                            let model_call_ms = model_started
                                .elapsed()
                                .as_millis()
                                .min(u128::from(u64::MAX)) as u64;
                            return Some(runtime_lane_fail_closed_with_state(
                                "runtime_lane_model_manifest_capability_evidence_gate_failed",
                                json!({
                                    "lane": "model_manifest_planner",
                                    "failure_code": "capability_evidence_gate_failed",
                                    "failure_message": runtime_lane_capability_evidence_gate_message(&retry_gate_report),
                                    "missing_capability_evidence": retry_gate_report.missing,
                                    "present_capability_evidence": retry_gate_report.present,
                                    "required_capability_evidence": retry_gate_report.required,
                                    "repair_attempted": true,
                                    "phase_latency_ms": {
                                        "workflow_load": 0,
                                        "execution_shape_gate": execution_shape_gate_ms,
                                        "provider_start": 0,
                                        "model_call": model_call_ms,
                                        "tool_dispatch": 0,
                                        "mutation": 0,
                                        "validation": 0,
                                        "repair": 0,
                                        "final_synthesis": 0,
                                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                                    }
                                }),
                                permissions,
                                wasm_sandbox,
                                voice_session,
                                state_path,
                                durable_state,
                            ));
                        }
                    }
                    retry_candidate
                }
                Err(retry_failure) => {
                    let model_call_ms = model_started
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64;
                    return Some(runtime_lane_fail_closed_with_state(
                        "runtime_lane_model_manifest_capability_evidence_gate_repair_failed",
                        json!({
                            "lane": "model_manifest_planner",
                            "failure_code": retry_failure.failure_code,
                            "failure_message": retry_failure.failure_message,
                            "needed_input": retry_failure.needed_input,
                            "provider_output_preview": retry_failure.provider_output_preview,
                            "missing_capability_evidence": gate_report.missing,
                            "present_capability_evidence": gate_report.present,
                            "required_capability_evidence": gate_report.required,
                            "repair_attempted": true,
                            "phase_latency_ms": {
                                "workflow_load": 0,
                                "execution_shape_gate": execution_shape_gate_ms,
                                "provider_start": 0,
                                "model_call": model_call_ms,
                                "tool_dispatch": 0,
                                "mutation": 0,
                                "validation": 0,
                                "repair": 0,
                                "final_synthesis": 0,
                                "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                            }
                        }),
                        permissions,
                        wasm_sandbox,
                        voice_session,
                        state_path,
                        durable_state,
                    ));
                }
            };
        }
    }
    let model_call_ms = model_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let dispatch_started = Instant::now();
    let mut receipts =
        runtime_lane_dispatch_model_manifest_actions(&candidate, "model_manifest_planner");
    if let Some(failure) =
        runtime_lane_semantic_probe_failure(&receipts, semantic_probe_call_id.as_deref())
    {
        let repair_context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
        let semantic_repair_prompt = runtime_lane_model_manifest_semantic_repair_prompt(
            prompt,
            &workspace_root,
            &repair_context_pack,
            &failure,
        );
        match provider_client.complete(&ProviderRequest {
            prompt: semantic_repair_prompt,
            system: Some(planner_system.clone()),
            tools: Vec::new(),
            model: model.cloned(),
            metadata: json!({
                "provider_timeout_seconds": metadata
                    .pointer("/native_success_criteria/provider_timeout_seconds")
                    .and_then(Value::as_u64)
                    .or_else(|| metadata.pointer("/workflow/native_success_criteria/provider_timeout_seconds").and_then(Value::as_u64))
                    .unwrap_or(90),
                "lane": "model_manifest_planner",
                "attempt": "post_validation_semantic_completion_repair",
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null)
            }),
        }) {
            Ok(repair_response) => match runtime_lane_model_manifest_candidate_from_output(
                &repair_response.output,
                tools,
                capability_packs,
                permissions,
            ) {
                Ok(mut repair_candidate) => {
                    let mut repair_probe_call_id = None;
                    let mut repair_validation_call_id = None;
                    runtime_lane_attach_semantic_probe_action(
                        &mut repair_candidate,
                        validation_command.as_ref(),
                        "model_manifest_planner_semantic_repair_validation",
                        &mut repair_validation_call_id,
                    );
                    if validation_command.is_some() {
                        repair_candidate.requires_validation = true;
                    }
                    runtime_lane_attach_semantic_probe_action(
                        &mut repair_candidate,
                        semantic_probe.as_ref(),
                        "model_manifest_planner_semantic_repair",
                        &mut repair_probe_call_id,
                    );
                    let mut repair_receipts = runtime_lane_dispatch_model_manifest_actions(
                        &repair_candidate,
                        "model_manifest_planner_semantic_repair",
                    );
                    let repair_success = repair_receipts
                        .iter()
                        .all(|receipt| receipt.status == "ok")
                        && repair_receipts
                            .iter()
                            .any(runtime_lane_receipt_is_successful_mutation)
                        && runtime_lane_receipts_validation_ok(
                            &repair_receipts,
                            repair_candidate.requires_validation,
                        )
                        && runtime_lane_semantic_probe_failure(
                            &repair_receipts,
                            repair_probe_call_id.as_deref(),
                        )
                        .is_none();
                    if repair_success {
                        candidate = repair_candidate;
                        receipts = repair_receipts;
                        provider_response = repair_response;
                    } else {
                        receipts.append(&mut repair_receipts);
                    }
                }
                Err(repair_failure) => receipts.push(runtime_lane_semantic_repair_failure_receipt(
                    "model_manifest_planner_semantic_repair_manifest_failed",
                    "manifest_parse_failed",
                    &repair_failure.failure_message,
                    json!({
                        "needed_input": repair_failure.needed_input,
                        "provider_output_preview": repair_failure.provider_output_preview,
                        "failed_probe": {
                            "command": failure.command,
                            "exit_code": failure.exit_code,
                            "stdout_tail": failure.stdout_tail,
                            "stderr_tail": failure.stderr_tail,
                        }
                    }),
                )),
            },
            Err(repair_error) => receipts.push(runtime_lane_semantic_repair_failure_receipt(
                "model_manifest_planner_semantic_repair_provider_failed",
                repair_error.code.as_str(),
                &repair_error.message,
                json!({
                    "failed_probe": {
                        "command": failure.command,
                        "exit_code": failure.exit_code,
                        "stdout_tail": failure.stdout_tail,
                        "stderr_tail": failure.stderr_tail,
                    }
                }),
            )),
        }
    }
    let tool_dispatch_ms = dispatch_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    Some(runtime_lane_model_manifest_planner_response(
        name,
        metadata,
        prompt,
        tools,
        capability_packs,
        required_pack_permissions,
        permissions,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
        state_path,
        durable_state,
        candidate,
        receipts,
        provider_response,
        execution_shape_gate_ms,
        model_call_ms,
        tool_dispatch_ms,
        total_started,
    ))
}

fn runtime_lane_deterministic_local_loop_response(
    name: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    candidate: DeterministicLocalLoopCandidate,
    receipts: Vec<NativeToolReceipt>,
    execution_shape_gate_ms: u64,
    tool_dispatch_ms: u64,
    total_started: Instant,
) -> RuntimeLaneResponse {
    let mutation_count = receipts
        .iter()
        .filter(|receipt| runtime_lane_receipt_is_successful_mutation(receipt))
        .count();
    let validation_ok =
        runtime_lane_receipts_validation_ok(&receipts, candidate.requires_validation)
            || (candidate.requires_validation
                && runtime_lane_post_mutation_command_receipts_all_successful(&receipts));
    let ok = !receipts.is_empty()
        && receipts.iter().all(|receipt| receipt.status == "ok")
        && mutation_count > 0
        && validation_ok;
    let changed_files = receipts
        .iter()
        .filter_map(runtime_lane_changed_file_summary_item)
        .collect::<Vec<_>>();
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| receipt.call_id.clone())
        .collect::<Vec<_>>();
    let mutation_ms = receipts
        .iter()
        .filter(|receipt| runtime_lane_receipt_is_mutation_tool(receipt))
        .map(|receipt| receipt.duration_ms)
        .sum::<u64>();
    let validation_ms = receipts
        .iter()
        .filter(|receipt| receipt.tool_name == "command_run")
        .map(|receipt| receipt.duration_ms)
        .sum::<u64>();
    let final_synthesis_started = Instant::now();
    let validation_status = if candidate.requires_validation {
        if validation_ok {
            "passed"
        } else {
            "failed"
        }
    } else {
        "not_run"
    };
    let output = format!(
        "{} via deterministic_local_loop.\n\nChanged files:\n{}\n\nValidation: {validation_status}.\nReceipts: {}",
        if ok { "Completed" } else { "Stopped" },
        changed_files
            .iter()
            .filter_map(|item| item.get("path").and_then(Value::as_str))
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
        receipt_refs.join(", ")
    );
    runtime_lane_persist_native_run_journal(
        metadata,
        &candidate.workspace_root,
        "deterministic_local_loop_completed",
        &receipts,
        &output,
        if ok {
            "ok"
        } else {
            "deterministic_local_loop_failed"
        },
    );
    let final_synthesis_ms = final_synthesis_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let phase_latency_ms = json!({
        "workflow_load": 0,
        "execution_shape_gate": execution_shape_gate_ms,
        "provider_start": 0,
        "model_call": 0,
        "tool_dispatch": tool_dispatch_ms,
        "mutation": mutation_ms,
        "validation": validation_ms,
        "repair": 0,
        "final_synthesis": final_synthesis_ms,
        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    });
    let mut runtime_receipt = json!({
        "type": "runtime_lane_receipt",
        "status": if ok { "ok" } else { "deterministic_local_loop_failed" },
        "lane": "deterministic_local_loop",
        "lane_reason": "declared_local_action_manifest",
        "requires_model": false,
        "requires_discovery": false,
        "requires_validation": candidate.requires_validation,
        "target_scope": "declared_manifest",
        "mutation_safety": "safe_manifest_actions",
        "workspace_root": candidate.workspace_root.display().to_string(),
        "changed_file_summary": changed_files,
        "native_tool_call_count": receipts.len(),
        "native_tool_receipts": receipts,
        "receipt_refs": receipt_refs,
        "validation_status": validation_status,
        "phase_latency_ms": phase_latency_ms,
    });
    let merkle_options = merkle_receipt_options_from_value(receipt_merkle);
    let persisted_previous_root = durable_state.merkle_roots.get(name).cloned();
    let effective_previous_root = previous_receipt_root
        .map(String::as_str)
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&runtime_receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(name.to_string(), root.to_string());
    }
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    if let Some(object) = runtime_receipt.as_object_mut() {
        object.insert("receipt_merkle".to_string(), merkle.clone());
    }
    RuntimeLaneResponse {
        ok,
        contract: json!({
            "name": name,
            "provider": Value::Null,
            "agent_status": if ok { "ok" } else { "deterministic_local_loop_failed" },
            "tool_count": tools.len(),
            "native_tool_call_count": runtime_receipt
                .get("native_tool_call_count")
                .cloned()
                .unwrap_or(Value::Null),
            "tools": tools,
            "capability_packs": capability_packs,
            "required_permissions": required_pack_permissions,
            "schedule": Value::Null,
            "lifespan_seconds": Value::Null,
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "receipt_merkle": merkle,
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "execution_shape": {
                "lane": "deterministic_local_loop",
                "confidence": 1.0,
                "requires_model": false,
                "requires_discovery": false,
                "requires_validation": candidate.requires_validation,
                "target_scope": "declared_manifest",
                "mutation_safety": "safe_manifest_actions",
                "escalation_reason": Value::Null
            }
        }),
        receipt: runtime_receipt,
        trace_summary: json!({
            "status": if ok { "ok" } else { "deterministic_local_loop_failed" },
            "lane": "deterministic_local_loop",
            "events": [
                "coding.task_contract.created",
                "coding.execution_shape.selected",
                "coding.local_action_loop.started",
                "coding.mutation.requested",
                if mutation_count > 0 { "coding.mutation.applied" } else { "coding.mutation.failed" },
                if candidate.requires_validation { "coding.validation.completed" } else { "coding.validation.skipped" },
                "coding.final_synthesis.completed"
            ],
            "phase_latency_ms": phase_latency_ms,
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
        }),
        output,
        error: if ok {
            None
        } else {
            Some("runtime_lane_deterministic_local_loop_failed".to_string())
        },
    }
}

fn runtime_lane_model_manifest_planner_response(
    name: &str,
    metadata: &Value,
    original_prompt: &str,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    candidate: DeterministicLocalLoopCandidate,
    mut receipts: Vec<NativeToolReceipt>,
    provider_response: ProviderResponse,
    execution_shape_gate_ms: u64,
    model_call_ms: u64,
    tool_dispatch_ms: u64,
    total_started: Instant,
) -> RuntimeLaneResponse {
    runtime_lane_apply_post_validation_closure_controller(metadata, original_prompt, &mut receipts);
    let mutation_count = receipts
        .iter()
        .filter(|receipt| runtime_lane_receipt_is_successful_mutation(receipt))
        .count();
    let validation_ok =
        runtime_lane_receipts_validation_ok(&receipts, candidate.requires_validation)
            || (candidate.requires_validation
                && runtime_lane_post_mutation_command_receipts_all_successful(&receipts));
    let post_dispatch_capability_evidence =
        runtime_lane_post_dispatch_capability_evidence_gate_report(
            metadata,
            original_prompt,
            &candidate.workspace_root,
            &receipts,
        );
    let post_dispatch_capability_evidence_ok = post_dispatch_capability_evidence
        .as_ref()
        .map(|report| report.missing.is_empty())
        .unwrap_or(true);
    let failure_status = if post_dispatch_capability_evidence_ok {
        "model_manifest_planner_failed"
    } else {
        "post_dispatch_capability_evidence_gate_failed"
    };
    let ok = !receipts.is_empty()
        && receipts.iter().all(|receipt| receipt.status == "ok")
        && mutation_count > 0
        && validation_ok
        && post_dispatch_capability_evidence_ok;
    let changed_files = receipts
        .iter()
        .filter_map(runtime_lane_changed_file_summary_item)
        .collect::<Vec<_>>();
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| receipt.call_id.clone())
        .collect::<Vec<_>>();
    let mutation_ms = receipts
        .iter()
        .filter(|receipt| runtime_lane_receipt_is_mutation_tool(receipt))
        .map(|receipt| receipt.duration_ms)
        .sum::<u64>();
    let validation_ms = receipts
        .iter()
        .filter(|receipt| receipt.tool_name == "command_run")
        .map(|receipt| receipt.duration_ms)
        .sum::<u64>();
    let final_synthesis_started = Instant::now();
    let validation_status = if candidate.requires_validation {
        if validation_ok {
            "passed"
        } else {
            "failed"
        }
    } else {
        "not_run"
    };
    let output = format!(
        "{} via model_manifest_planner.\n\nChanged files:\n{}\n\nValidation: {validation_status}.\nCapability coverage: {}.\nReceipts: {}",
        if ok { "Completed" } else { "Stopped" },
        changed_files
            .iter()
            .filter_map(|item| item.get("path").and_then(Value::as_str))
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
        post_dispatch_capability_evidence
            .as_ref()
            .map(|report| {
                if report.missing.is_empty() {
                    "passed".to_string()
                } else {
                    format!("missing {}", runtime_lane_join_or_none(&report.missing))
                }
            })
            .unwrap_or_else(|| "not_required".to_string()),
        receipt_refs.join(", ")
    );
    runtime_lane_persist_native_run_journal(
        metadata,
        &candidate.workspace_root,
        "model_manifest_planner_completed",
        &receipts,
        &output,
        if ok {
            "ok"
        } else {
            failure_status
        },
    );
    let final_synthesis_ms = final_synthesis_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let phase_latency_ms = json!({
        "workflow_load": 0,
        "execution_shape_gate": execution_shape_gate_ms,
        "provider_start": 0,
        "model_call": model_call_ms,
        "tool_dispatch": tool_dispatch_ms,
        "mutation": mutation_ms,
        "validation": validation_ms,
        "repair": 0,
        "final_synthesis": final_synthesis_ms,
        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    });
    let mut runtime_receipt = json!({
        "type": "runtime_lane_receipt",
        "status": if ok { "ok" } else { "model_manifest_planner_failed" },
        "lane": "model_manifest_planner",
        "execution_lane": "deterministic_local_loop",
        "lane_reason": "bounded_local_task_manifest_generated_by_model",
        "requires_model": true,
        "requires_discovery": false,
        "requires_validation": candidate.requires_validation,
        "target_scope": "model_generated_manifest",
        "mutation_safety": "safe_manifest_actions",
        "workspace_root": candidate.workspace_root.display().to_string(),
        "planner_provider": provider_response.provider,
        "planner_model": provider_response.model,
        "planner_usage_tokens": provider_response.usage_tokens,
        "changed_file_summary": changed_files,
        "post_dispatch_capability_evidence": post_dispatch_capability_evidence
            .as_ref()
            .map(|report| json!({
                "required": report.required.clone(),
                "present": report.present.clone(),
                "missing": report.missing.clone(),
            }))
            .unwrap_or(Value::Null),
        "native_tool_call_count": receipts.len(),
        "native_tool_receipts": receipts,
        "receipt_refs": receipt_refs,
        "validation_status": validation_status,
        "phase_latency_ms": phase_latency_ms,
    });
    let merkle_options = merkle_receipt_options_from_value(receipt_merkle);
    let persisted_previous_root = durable_state.merkle_roots.get(name).cloned();
    let effective_previous_root = previous_receipt_root
        .map(String::as_str)
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&runtime_receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(name.to_string(), root.to_string());
    }
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    if let Some(object) = runtime_receipt.as_object_mut() {
        object.insert("receipt_merkle".to_string(), merkle.clone());
    }
    RuntimeLaneResponse {
        ok,
        contract: json!({
            "name": name,
            "provider": Value::Null,
            "planner_provider": runtime_receipt.get("planner_provider").cloned().unwrap_or(Value::Null),
            "planner_model": runtime_receipt.get("planner_model").cloned().unwrap_or(Value::Null),
            "agent_status": if ok { "ok" } else { failure_status },
            "tool_count": tools.len(),
            "native_tool_call_count": runtime_receipt
                .get("native_tool_call_count")
                .cloned()
                .unwrap_or(Value::Null),
            "tools": tools,
            "capability_packs": capability_packs,
            "required_permissions": required_pack_permissions,
            "schedule": Value::Null,
            "lifespan_seconds": Value::Null,
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "receipt_merkle": merkle,
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "execution_shape": {
                "lane": "model_manifest_planner",
                "execution_lane": "deterministic_local_loop",
                "confidence": 0.8,
                "requires_model": true,
                "requires_discovery": false,
                "requires_validation": candidate.requires_validation,
                "target_scope": "model_generated_manifest",
                "mutation_safety": "safe_manifest_actions",
                "escalation_reason": "natural_language_task_required_model_to_generate_deterministic_manifest"
            }
        }),
        receipt: runtime_receipt,
        trace_summary: json!({
            "status": if ok { "ok" } else { failure_status },
            "lane": "model_manifest_planner",
            "execution_lane": "deterministic_local_loop",
            "events": [
                "coding.task_contract.created",
                "coding.execution_shape.selected",
                "coding.model_manifest_planner.started",
                "coding.model_manifest_planner.completed",
                "coding.local_action_loop.started",
                "coding.mutation.requested",
                if mutation_count > 0 { "coding.mutation.applied" } else { "coding.mutation.failed" },
                if candidate.requires_validation { "coding.validation.completed" } else { "coding.validation.skipped" },
                "coding.final_synthesis.completed"
            ],
            "phase_latency_ms": phase_latency_ms,
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
        }),
        output,
        error: if ok {
            None
        } else {
            Some(format!("runtime_lane_{failure_status}"))
        },
    }
}

#[derive(Clone, Debug)]
struct RuntimeLaneCapabilityEvidenceGateReport {
    required: Vec<String>,
    present: Vec<String>,
    missing: Vec<String>,
}

fn runtime_lane_capability_evidence_gate_report(
    metadata: &Value,
    prompt: &str,
    candidate: &DeterministicLocalLoopCandidate,
) -> Option<RuntimeLaneCapabilityEvidenceGateReport> {
    if !runtime_lane_capability_evidence_gate_enabled_for_task_lane(metadata, prompt) {
        return None;
    }
    let required = runtime_lane_required_capability_evidence_categories(prompt);
    if required.is_empty() {
        return None;
    }
    let present = runtime_lane_present_capability_evidence_categories(candidate);
    let missing = required
        .iter()
        .filter(|category| !runtime_lane_string_vec_contains(&present, category))
        .cloned()
        .collect::<Vec<_>>();
    Some(RuntimeLaneCapabilityEvidenceGateReport {
        required,
        present,
        missing,
    })
}

fn runtime_lane_capability_evidence_gate_enabled(metadata: &Value) -> bool {
    runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/capability_evidence_gate_enabled",
            "/workflow/native_success_criteria/capability_evidence_gate_enabled",
        ],
    )
    .unwrap_or(false)
}

fn runtime_lane_capability_evidence_gate_enabled_for_task_lane(
    metadata: &Value,
    prompt: &str,
) -> bool {
    if !runtime_lane_capability_evidence_gate_enabled(metadata) {
        return false;
    }
    let task_lane = crate::native_evidence::native_tool_coding_task_lane(metadata, prompt);
    runtime_lane_metadata_string_list_allows(
        metadata,
        &[
            "/native_success_criteria/capability_evidence_gate_enabled_lanes",
            "/workflow/native_success_criteria/capability_evidence_gate_enabled_lanes",
        ],
        task_lane,
    )
    .unwrap_or(true)
}

fn runtime_lane_required_capability_evidence_categories(prompt: &str) -> Vec<String> {
    let lower = prompt.to_ascii_lowercase();
    let durable_task = runtime_lane_text_contains_any(
        &lower,
        &[
            "persist",
            "persistent",
            "store",
            "repository",
            "checkpoint",
            "receipt",
            "memory row",
            "memory entry",
            "resume context",
            "handoff",
            "cli",
            "command line",
            "report",
            "roundtrip",
            "round-trip",
            "import/export",
            "import and export",
            "export and import",
        ],
    );
    if !durable_task {
        return Vec::new();
    }

    let mut required = Vec::<String>::new();
    runtime_lane_push_unique_string(&mut required, "domain_model");
    runtime_lane_push_unique_string(&mut required, "persistence_store");
    runtime_lane_push_unique_string(&mut required, "regression_tests");
    runtime_lane_push_unique_string(&mut required, "validation");

    if runtime_lane_text_contains_any(
        &lower,
        &[
            "service",
            "subsystem",
            "workflow",
            "pipeline",
            "manager",
            "integration",
            "operator",
        ],
    ) {
        runtime_lane_push_unique_string(&mut required, "service_integration");
    }
    if runtime_lane_text_contains_any(
        &lower,
        &["cli", "command line", "operator", "report", "summary surface"],
    ) {
        runtime_lane_push_unique_string(&mut required, "operator_surface");
    }
    if runtime_lane_text_contains_any(
        &lower,
        &[
            "import/export",
            "import and export",
            "export and import",
            "roundtrip",
            "round-trip",
            "serialize",
            "deserialize",
        ],
    ) {
        runtime_lane_push_unique_string(&mut required, "import_export_or_roundtrip");
    }
    if runtime_lane_text_contains_any(&lower, &["checkpoint", "receipt", "handoff"]) {
        runtime_lane_push_unique_string(&mut required, "checkpoint_receipt");
    }
    if runtime_lane_text_contains_any(
        &lower,
        &["memory row", "memory entry", "resume context", "project context"],
    ) {
        runtime_lane_push_unique_string(&mut required, "memory_row");
    }
    required
}

fn runtime_lane_present_capability_evidence_categories(
    candidate: &DeterministicLocalLoopCandidate,
) -> Vec<String> {
    let mut present = Vec::<String>::new();
    let mut mutation_text = String::new();
    let mut has_test_mutation = false;
    let mut has_command_run = candidate.requires_validation;

    for action in &candidate.actions {
        match action {
            DeterministicLocalAction::WriteFile {
                target_path,
                content,
                ..
            } => {
                let path = runtime_lane_path_relative_display(&candidate.workspace_root, target_path)
                    .to_ascii_lowercase();
                has_test_mutation |= runtime_lane_path_looks_like_test(&path)
                    || runtime_lane_text_looks_like_test(content);
                mutation_text.push('\n');
                mutation_text.push_str(&path);
                mutation_text.push('\n');
                mutation_text.push_str(&content.to_ascii_lowercase());
            }
            DeterministicLocalAction::PatchFile {
                target_path,
                old,
                new,
                ..
            } => {
                let path = runtime_lane_path_relative_display(&candidate.workspace_root, target_path)
                    .to_ascii_lowercase();
                has_test_mutation |= runtime_lane_path_looks_like_test(&path)
                    || runtime_lane_text_looks_like_test(new);
                mutation_text.push('\n');
                mutation_text.push_str(&path);
                mutation_text.push('\n');
                mutation_text.push_str(&old.to_ascii_lowercase());
                mutation_text.push('\n');
                mutation_text.push_str(&new.to_ascii_lowercase());
            }
            DeterministicLocalAction::CommandRun { .. } => {
                has_command_run = true;
            }
        }
    }

    if runtime_lane_text_contains_any(
        &mutation_text,
        &[
            "class ",
            "struct ",
            "dataclass",
            "record",
            "model",
            "typedict",
            "interface ",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "domain_model");
    }
    if runtime_lane_text_contains_any(
        &mutation_text,
        &[
            "persist",
            "repository",
            "store",
            "storage",
            "jsonl",
            "json",
            "sqlite",
            "read_text",
            "write_text",
            "open(",
            "fs::",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "persistence_store");
    }
    if runtime_lane_text_contains_any(
        &mutation_text,
        &[
            "service",
            "manager",
            "controller",
            "orchestr",
            "repository",
            "store",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "service_integration");
    }
    if runtime_lane_text_contains_any(
        &mutation_text,
        &[
            "cli",
            "argparse",
            "click",
            "typer",
            "command line",
            "report",
            "println!",
            "console",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "operator_surface");
    }
    if runtime_lane_text_contains_any(
        &mutation_text,
        &[
            "export",
            "import_",
            "import ",
            "roundtrip",
            "round_trip",
            "serialize",
            "deserialize",
            "to_json",
            "from_json",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "import_export_or_roundtrip");
    }
    if has_test_mutation {
        runtime_lane_push_unique_string(&mut present, "regression_tests");
    }
    if has_command_run {
        runtime_lane_push_unique_string(&mut present, "validation");
    }
    if runtime_lane_text_contains_any(&mutation_text, &["checkpoint", "receipt", "handoff"]) {
        runtime_lane_push_unique_string(&mut present, "checkpoint_receipt");
    }
    if runtime_lane_text_contains_any(
        &mutation_text,
        &["memory row", "memory entry", "resume context", "project context"],
    ) {
        runtime_lane_push_unique_string(&mut present, "memory_row");
    }
    present
}

fn runtime_lane_post_dispatch_capability_evidence_gate_report(
    metadata: &Value,
    prompt: &str,
    workspace_root: &Path,
    receipts: &[NativeToolReceipt],
) -> Option<RuntimeLaneCapabilityEvidenceGateReport> {
    if !runtime_lane_capability_evidence_gate_enabled_for_task_lane(metadata, prompt) {
        return None;
    }
    let required = runtime_lane_required_capability_evidence_categories(prompt);
    if required.is_empty() {
        return None;
    }
    let present =
        runtime_lane_present_post_dispatch_capability_evidence_categories(workspace_root, receipts);
    let missing = required
        .iter()
        .filter(|category| !runtime_lane_string_vec_contains(&present, category))
        .cloned()
        .collect::<Vec<_>>();
    Some(RuntimeLaneCapabilityEvidenceGateReport {
        required,
        present,
        missing,
    })
}

fn runtime_lane_present_post_dispatch_capability_evidence_categories(
    workspace_root: &Path,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut present = Vec::<String>::new();
    let mut source_corpus = String::new();
    let mut test_corpus = String::new();

    for receipt in receipts {
        if receipt.status != "ok" {
            continue;
        }

        if receipt.tool_name == "command_run" && runtime_lane_command_receipt_success(receipt) {
            let command_corpus = format!(
                "{}\n{}",
                runtime_lane_receipt_command_text(receipt),
                runtime_lane_value_string_corpus(&receipt.result, 8_000)
            )
            .to_ascii_lowercase();
            if runtime_lane_text_contains_any(
                &command_corpus,
                &[
                    "cargo test",
                    "npm test",
                    "pnpm test",
                    "yarn test",
                    "pytest",
                    "unittest",
                    "go test",
                    "swift test",
                    "validation",
                    "passed",
                ],
            ) {
                runtime_lane_push_unique_string(&mut present, "validation");
            }
            if runtime_lane_text_contains_any(
                &command_corpus,
                &[
                    "checkpoint write",
                    "memory write",
                    "memory-cli",
                    "resume context",
                    "project context",
                    "persist context",
                ],
            ) {
                runtime_lane_push_unique_string(&mut present, "memory_row");
            }
        }

        let Some(receipt_path) = runtime_lane_receipt_path_string(receipt) else {
            continue;
        };
        let lower_path = receipt_path.to_ascii_lowercase();
        if runtime_lane_text_contains_any(&lower_path, &["checkpoint", "handoff", "receipt"]) {
            runtime_lane_push_unique_string(&mut present, "checkpoint_receipt");
        }
        if runtime_lane_text_contains_any(
            &lower_path,
            &["memory", "resume_context", "project_context"],
        ) {
            runtime_lane_push_unique_string(&mut present, "memory_row");
        }

        if !runtime_lane_receipt_is_successful_mutation(receipt) {
            continue;
        }
        let Some(content) =
            runtime_lane_read_receipt_file_content(workspace_root, &receipt_path, 24_000)
        else {
            continue;
        };
        if runtime_lane_path_looks_like_test(&lower_path)
            || runtime_lane_text_looks_like_test(&content)
        {
            runtime_lane_append_bounded_corpus(&mut test_corpus, &content, 48_000);
        } else {
            runtime_lane_append_bounded_corpus(&mut source_corpus, &content, 64_000);
        }
    }

    let source_lower = source_corpus.to_ascii_lowercase();
    let test_lower = test_corpus.to_ascii_lowercase();

    if runtime_lane_text_contains_any(
        &source_lower,
        &[
            "dataclass",
            "class ",
            "struct ",
            "interface ",
            "type ",
            "model",
            "record",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "domain_model");
    }
    if runtime_lane_text_contains_any(
        &source_lower,
        &[
            "jsonl",
            "sqlite",
            "repository",
            "store",
            "storage",
            "persist",
            "read_to_string",
            "write_text",
            "open(",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "persistence_store");
    }
    if runtime_lane_text_contains_any(
        &source_lower,
        &[
            "service",
            "manager",
            "controller",
            "orchestr",
            "repository",
            "route",
            "handler",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "service_integration");
    }
    if runtime_lane_text_contains_any(
        &source_lower,
        &[
            "argparse",
            "click",
            "typer",
            "subparser",
            "cli",
            "command",
            "report",
            "summary",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "operator_surface");
    }
    if runtime_lane_text_contains_any(
        &source_lower,
        &[
            "export",
            "import_",
            "roundtrip",
            "round_trip",
            "serialize",
            "deserialize",
            "to_json",
            "from_json",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "import_export_or_roundtrip");
    }
    if runtime_lane_text_contains_any(
        &test_lower,
        &[
            "def test_",
            "#[test]",
            "it(",
            "describe(",
            "assert",
            "expect(",
            "unittest",
            "pytest",
        ],
    ) {
        runtime_lane_push_unique_string(&mut present, "regression_tests");
    }

    present
}

fn runtime_lane_receipt_path_string(receipt: &NativeToolReceipt) -> Option<String> {
    for key in ["path", "file_path", "target_path", "target"] {
        if let Some(path) = receipt.result.get(key).and_then(Value::as_str) {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    receipt
        .result
        .get("args")
        .and_then(Value::as_object)
        .and_then(|args| {
            ["path", "file_path", "target_path", "target"]
                .iter()
                .find_map(|key| args.get(*key).and_then(Value::as_str))
        })
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
}

fn runtime_lane_receipt_command_text(receipt: &NativeToolReceipt) -> String {
    for key in ["cmd", "command", "argv"] {
        if let Some(value) = receipt.result.get(key) {
            let text = runtime_lane_value_string_corpus(value, 4_000);
            if !text.trim().is_empty() {
                return text;
            }
        }
    }
    receipt
        .result
        .get("args")
        .map(|value| runtime_lane_value_string_corpus(value, 4_000))
        .unwrap_or_default()
}

fn runtime_lane_value_string_corpus(value: &Value, max_chars: usize) -> String {
    fn visit(value: &Value, output: &mut String, max_chars: usize) {
        if output.chars().count() >= max_chars {
            return;
        }
        match value {
            Value::String(text) => {
                runtime_lane_append_bounded_corpus(output, text, max_chars);
                runtime_lane_append_bounded_corpus(output, "\n", max_chars);
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, output, max_chars);
                    if output.chars().count() >= max_chars {
                        break;
                    }
                }
            }
            Value::Object(map) => {
                for item in map.values() {
                    visit(item, output, max_chars);
                    if output.chars().count() >= max_chars {
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    let mut output = String::new();
    visit(value, &mut output, max_chars);
    output
}

fn runtime_lane_read_receipt_file_content(
    workspace_root: &Path,
    receipt_path: &str,
    max_chars: usize,
) -> Option<String> {
    let raw_path = PathBuf::from(receipt_path);
    let full_path = if raw_path.is_absolute() {
        raw_path
    } else {
        workspace_root.join(raw_path)
    };
    if workspace_root.is_absolute()
        && full_path.is_absolute()
        && !full_path.starts_with(workspace_root)
    {
        return None;
    }
    std::fs::read_to_string(full_path)
        .ok()
        .map(|content| content.chars().take(max_chars).collect())
}

fn runtime_lane_append_bounded_corpus(output: &mut String, text: &str, max_chars: usize) {
    let remaining = max_chars.saturating_sub(output.chars().count());
    if remaining == 0 {
        return;
    }
    output.extend(text.chars().take(remaining));
}

fn runtime_lane_capability_evidence_gate_message(
    report: &RuntimeLaneCapabilityEvidenceGateReport,
) -> String {
    format!(
        "The manifest lacks source-level evidence for required capability categories. Missing: {}. Present: {}. Required: {}.",
        runtime_lane_join_or_none(&report.missing),
        runtime_lane_join_or_none(&report.present),
        runtime_lane_join_or_none(&report.required)
    )
}

fn runtime_lane_capability_evidence_gate_retry_hint(
    report: &RuntimeLaneCapabilityEvidenceGateReport,
) -> String {
    format!(
        "Return a corrected deterministic_local_loop manifest. Add real source mutations, not comments, that cover these generic capability categories: {}. Include faithful regression tests and a validation command when validation is required.",
        runtime_lane_join_or_none(&report.missing)
    )
}

fn runtime_lane_text_contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn runtime_lane_path_looks_like_test(path: &str) -> bool {
    path.contains("test") || path.contains("spec")
}

fn runtime_lane_text_looks_like_test(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    runtime_lane_text_contains_any(
        &lower,
        &["unittest", "pytest", "#[test]", "describe(", "it(", "assert "],
    )
}

fn runtime_lane_push_unique_string(values: &mut Vec<String>, value: &str) {
    if !runtime_lane_string_vec_contains(values, value) {
        values.push(value.to_string());
    }
}

fn runtime_lane_string_vec_contains(values: &[String], value: &str) -> bool {
    values.iter().any(|entry| entry == value)
}

fn runtime_lane_join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn runtime_lane_apply_post_validation_closure_controller(
    metadata: &Value,
    original_prompt: &str,
    receipts: &mut Vec<NativeToolReceipt>,
) {
    if !runtime_lane_post_validation_closure_controller_enabled(metadata) {
        return;
    }
    let dispatcher = NativeToolDispatcher::new(&[
        "file_write".to_string(),
        "command_run".to_string(),
    ]);
    let mut closure_receipts =
        native_tool_auto_workflow_artifact_receipts(&dispatcher, metadata, original_prompt, receipts);
    if closure_receipts.is_empty() {
        return;
    }
    receipts.append(&mut closure_receipts);
}

fn runtime_lane_post_validation_closure_controller_enabled(metadata: &Value) -> bool {
    runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/post_validation_closure_controller_enabled",
            "/workflow/native_success_criteria/post_validation_closure_controller_enabled",
        ],
    )
    .unwrap_or(false)
}

fn runtime_lane_direct_mutation_response(
    name: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    candidate: DirectMutationCandidate,
    receipt: NativeToolReceipt,
    execution_shape_gate_ms: u64,
    tool_dispatch_ms: u64,
    total_started: Instant,
) -> RuntimeLaneResponse {
    let ok = receipt.status == "ok";
    let changed_file = candidate.target_path.display().to_string();
    let mutation_ms = receipt.duration_ms;
    let final_synthesis_started = Instant::now();
    let output = if ok {
        format!(
            "Completed via direct_mutation.\n\nChanged files:\n- {changed_file}\n\nValidation: not run.\nReceipt: {}",
            receipt.call_id
        )
    } else {
        format!(
            "Direct mutation was blocked before provider startup.\n\nTarget: {changed_file}\nError: {}",
            receipt.error.clone().unwrap_or_else(|| "unknown_error".to_string())
        )
    };
    let final_synthesis_ms = final_synthesis_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let phase_latency_ms = json!({
        "workflow_load": 0,
        "execution_shape_gate": execution_shape_gate_ms,
        "provider_start": 0,
        "model_call": 0,
        "tool_dispatch": tool_dispatch_ms,
        "mutation": mutation_ms,
        "validation": 0,
        "repair": 0,
        "final_synthesis": final_synthesis_ms,
        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    });
    let receipt_refs = if ok {
        vec![receipt.call_id.clone()]
    } else {
        Vec::new()
    };
    let mut runtime_receipt = json!({
        "type": "runtime_lane_receipt",
        "status": if ok { "ok" } else { "direct_mutation_failed" },
        "lane": "direct_mutation",
        "lane_reason": "explicit_content_and_safe_target_path",
        "requires_model": false,
        "requires_discovery": false,
        "requires_validation": false,
        "target_scope": if candidate.overwrite { "known_file" } else { "new_file" },
        "mutation_safety": if candidate.overwrite { "safe_overwrite_requested" } else { "safe_new" },
        "workspace_root": candidate
            .workspace_root
            .as_ref()
            .map(|path| path.display().to_string()),
        "changed_file_summary": if ok {
            json!([{
                "path": changed_file,
                "operation": if candidate.overwrite { "overwritten" } else { "created_or_written" },
                "receipt_ref": receipt.call_id
            }])
        } else {
            json!([])
        },
        "native_tool_call_count": 1,
        "native_tool_receipts": [receipt.clone()],
        "receipt_refs": receipt_refs,
        "phase_latency_ms": phase_latency_ms,
    });
    let merkle_options = merkle_receipt_options_from_value(receipt_merkle);
    let persisted_previous_root = durable_state.merkle_roots.get(name).cloned();
    let effective_previous_root = previous_receipt_root
        .map(String::as_str)
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&runtime_receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(name.to_string(), root.to_string());
    }
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    if let Some(object) = runtime_receipt.as_object_mut() {
        object.insert("receipt_merkle".to_string(), merkle.clone());
    }
    RuntimeLaneResponse {
        ok,
        contract: json!({
            "name": name,
            "provider": Value::Null,
            "agent_status": if ok { "ok" } else { "direct_mutation_failed" },
            "tool_count": tools.len(),
            "native_tool_call_count": 1,
            "tools": tools,
            "capability_packs": capability_packs,
            "required_permissions": required_pack_permissions,
            "schedule": Value::Null,
            "lifespan_seconds": Value::Null,
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "receipt_merkle": merkle,
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "execution_shape": {
                "lane": "direct_mutation",
                "confidence": 1.0,
                "requires_model": false,
                "requires_discovery": false,
                "requires_validation": false,
                "target_scope": if candidate.overwrite { "known_file" } else { "new_file" },
                "mutation_safety": if candidate.overwrite { "safe_overwrite_requested" } else { "safe_new" },
                "escalation_reason": Value::Null
            }
        }),
        receipt: runtime_receipt,
        trace_summary: json!({
            "status": if ok { "ok" } else { "direct_mutation_failed" },
            "lane": "direct_mutation",
            "events": [
                "coding.task_contract.created",
                "coding.execution_shape.selected",
                "coding.mutation.requested",
                if ok { "coding.mutation.applied" } else { "coding.mutation.failed" },
                "coding.final_synthesis.completed"
            ],
            "phase_latency_ms": phase_latency_ms,
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
        }),
        output,
        error: if ok {
            None
        } else {
            Some("runtime_lane_direct_mutation_failed".to_string())
        },
    }
}

fn runtime_lane_direct_mutation_candidate(
    prompt: &str,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> DirectMutationGate {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return DirectMutationGate::NotCandidate;
    }
    if permission_for(permissions, "file.write") != PermissionTrit::Allow {
        return DirectMutationGate::NotCandidate;
    }
    let Some(content) = runtime_lane_extract_explicit_file_content(prompt) else {
        return DirectMutationGate::NotCandidate;
    };
    let workspace_root = runtime_lane_extract_workspace_root(prompt);
    let Some(raw_target) = runtime_lane_extract_target_file_path(prompt) else {
        return DirectMutationGate::NotCandidate;
    };
    let target_path = match runtime_lane_resolve_target_path(&raw_target, workspace_root.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            return DirectMutationGate::Blocked {
                failure_code: "unsafe_or_unresolved_target_path",
                failure_message: error,
                needed_input: Some(
                    "Provide an explicit safe target path inside the workspace.".to_string(),
                ),
                target_path: Some(raw_target),
            };
        }
    };
    let overwrite = runtime_lane_prompt_requests_overwrite(prompt);
    if target_path.exists() && !overwrite {
        return DirectMutationGate::Blocked {
            failure_code: "unsafe_overwrite",
            failure_message: "Target already exists and overwrite was not explicitly requested."
                .to_string(),
            needed_input: Some("Confirm overwrite or choose a new target path.".to_string()),
            target_path: Some(target_path.display().to_string()),
        };
    }
    DirectMutationGate::Candidate(DirectMutationCandidate {
        workspace_root,
        target_path,
        content,
        content_source: "explicit_prompt_content",
        overwrite,
    })
}

fn runtime_lane_deterministic_local_loop_candidate(
    prompt: &str,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> DeterministicLocalLoopGate {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return DeterministicLocalLoopGate::NotCandidate;
    }
    let Some(manifest) = runtime_lane_extract_deterministic_manifest(prompt) else {
        return DeterministicLocalLoopGate::NotCandidate;
    };
    if permission_for(permissions, "file.write") != PermissionTrit::Allow {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "permission_denied",
            failure_message:
                "file.write permission is required for deterministic local loop mutations."
                    .to_string(),
            needed_input: Some("Grant file.write or use a non-mutating workflow.".to_string()),
        };
    }
    let root_value = manifest
        .get("workspace_root")
        .or_else(|| manifest.get("project_root"))
        .or_else(|| manifest.get("repo_root"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(root_value) = root_value else {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "workspace_root_required",
            failure_message: "Deterministic local loop manifests require workspace_root."
                .to_string(),
            needed_input: Some(
                "Add workspace_root to the deterministic local action manifest.".to_string(),
            ),
        };
    };
    let workspace_root = PathBuf::from(root_value);
    if !workspace_root.is_absolute() {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "absolute_workspace_root_required",
            failure_message: "workspace_root must be absolute.".to_string(),
            needed_input: Some("Use an absolute workspace_root path.".to_string()),
        };
    }
    let Some(actions_value) = manifest
        .get("actions")
        .or_else(|| manifest.get("files"))
        .and_then(Value::as_array)
    else {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "actions_required",
            failure_message: "Deterministic local loop manifests require actions or files."
                .to_string(),
            needed_input: Some(
                "Add at least one write_file action with path and content.".to_string(),
            ),
        };
    };
    let mut actions = Vec::<DeterministicLocalAction>::new();
    for action in actions_value {
        let (kind_hint, action) = runtime_lane_manifest_action_payload(action);
        let kind = kind_hint
            .or_else(|| {
                action
                    .get("type")
                    .or_else(|| action.get("kind"))
                    .or_else(|| action.get("action"))
                    .and_then(Value::as_str)
            })
            .unwrap_or_else(|| {
                if action.get("cmd").is_some() || action.get("command").is_some() {
                    "command_run"
                } else {
                    "write_file"
                }
            })
            .trim()
            .to_ascii_lowercase();
        match kind.as_str() {
            "write_file" | "create_file" | "file_write" | "write" => {
                let Some(path) = action
                    .get("path")
                    .or_else(|| action.get("target_path"))
                    .or_else(|| action.get("target_file"))
                    .or_else(|| action.get("target"))
                    .or_else(|| action.get("file"))
                    .or_else(|| action.get("file_path"))
                    .or_else(|| action.get("filepath"))
                    .or_else(|| action.get("filename"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "path_required",
                        failure_message: "write_file actions require path.".to_string(),
                        needed_input: Some("Add a path to each write_file action.".to_string()),
                    };
                };
                let Some(content) = action
                    .get("content")
                    .or_else(|| action.get("text"))
                    .or_else(|| action.get("body"))
                    .and_then(Value::as_str)
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "content_required",
                        failure_message: "write_file actions require content.".to_string(),
                        needed_input: Some("Add content to each write_file action.".to_string()),
                    };
                };
                let target_path =
                    match runtime_lane_resolve_target_path(path, Some(&workspace_root)) {
                        Ok(path) => path,
                        Err(error) => {
                            return DeterministicLocalLoopGate::Blocked {
                                failure_code: "unsafe_or_unresolved_target_path",
                                failure_message: error,
                                needed_input: Some(
                                    "Use a safe path inside workspace_root.".to_string(),
                                ),
                            };
                        }
                    };
                let overwrite = action
                    .get("overwrite")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if target_path.exists() && !overwrite {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "unsafe_overwrite",
                        failure_message: format!(
                            "Target already exists and overwrite was not explicitly requested: {}",
                            target_path.display()
                        ),
                        needed_input: Some(
                            "Set overwrite=true or choose a new target path.".to_string(),
                        ),
                    };
                }
                actions.push(DeterministicLocalAction::WriteFile {
                    target_path,
                    content: ensure_trailing_newline(content.to_string()),
                    overwrite,
                });
            }
            "file_patch" | "patch_file" | "patch" => {
                if permission_for(permissions, "file.patch") != PermissionTrit::Allow {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "permission_denied",
                        failure_message: "file.patch permission is required for patch actions."
                            .to_string(),
                        needed_input: Some(
                            "Grant file.patch or use write_file with overwrite=true.".to_string(),
                        ),
                    };
                }
                let Some(path) = action
                    .get("path")
                    .or_else(|| action.get("target_path"))
                    .or_else(|| action.get("target_file"))
                    .or_else(|| action.get("target"))
                    .or_else(|| action.get("file"))
                    .or_else(|| action.get("file_path"))
                    .or_else(|| action.get("filepath"))
                    .or_else(|| action.get("filename"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "path_required",
                        failure_message: "file_patch actions require path.".to_string(),
                        needed_input: Some("Add a path to each file_patch action.".to_string()),
                    };
                };
                let Some(old) = action
                    .get("old")
                    .or_else(|| action.get("find"))
                    .or_else(|| action.get("search"))
                    .or_else(|| action.get("old_text"))
                    .or_else(|| action.get("search_text"))
                    .or_else(|| action.get("find_text"))
                    .or_else(|| action.get("before_text"))
                    .or_else(|| action.get("before"))
                    .or_else(|| action.get("original"))
                    .or_else(|| action.get("original_text"))
                    .and_then(Value::as_str)
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "old_required",
                        failure_message: "file_patch actions require old/find/search text."
                            .to_string(),
                        needed_input: Some(
                            "Add exact old text from the local context pack.".to_string(),
                        ),
                    };
                };
                let Some(new) = action
                    .get("new")
                    .or_else(|| action.get("replace"))
                    .or_else(|| action.get("replacement"))
                    .or_else(|| action.get("new_text"))
                    .or_else(|| action.get("replace_text"))
                    .or_else(|| action.get("replacement_text"))
                    .or_else(|| action.get("after_text"))
                    .or_else(|| action.get("after"))
                    .or_else(|| action.get("updated"))
                    .or_else(|| action.get("updated_text"))
                    .and_then(Value::as_str)
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "new_required",
                        failure_message: "file_patch actions require new/replacement text."
                            .to_string(),
                        needed_input: Some("Add replacement text.".to_string()),
                    };
                };
                let target_path =
                    match runtime_lane_resolve_target_path(path, Some(&workspace_root)) {
                        Ok(path) => path,
                        Err(error) => {
                            return DeterministicLocalLoopGate::Blocked {
                                failure_code: "unsafe_or_unresolved_target_path",
                                failure_message: error,
                                needed_input: Some(
                                    "Use a safe path inside workspace_root.".to_string(),
                                ),
                            };
                        }
                    };
                actions.push(DeterministicLocalAction::PatchFile {
                    target_path,
                    old: old.to_string(),
                    new: new.to_string(),
                    allow_multiple: action
                        .get("allow_multiple")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                });
            }
            "command_run" | "run_command" | "validate" | "validation" => {
                if permission_for(permissions, "command.run") != PermissionTrit::Allow {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "permission_denied",
                        failure_message:
                            "command.run permission is required for validation actions.".to_string(),
                        needed_input: Some(
                            "Grant command.run or remove validation actions.".to_string(),
                        ),
                    };
                }
                let cwd = action
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| runtime_lane_resolve_target_path(value, Some(&workspace_root)))
                    .transpose();
                let cwd = match cwd {
                    Ok(Some(path)) => path,
                    Ok(None) => workspace_root.clone(),
                    Err(error) => {
                        return DeterministicLocalLoopGate::Blocked {
                            failure_code: "unsafe_or_unresolved_cwd",
                            failure_message: error,
                            needed_input: Some("Use a safe cwd inside workspace_root.".to_string()),
                        };
                    }
                };
                let Some(cmd) = runtime_lane_manifest_command(action) else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "command_required",
                        failure_message: "command_run actions require cmd or command.".to_string(),
                        needed_input: Some(
                            "Add cmd as a string array or shell command string.".to_string(),
                        ),
                    };
                };
                actions.push(DeterministicLocalAction::CommandRun {
                    cwd,
                    cmd,
                    timeout_seconds: action
                        .get("timeout_seconds")
                        .and_then(Value::as_u64)
                        .unwrap_or(30),
                    max_output_bytes: action
                        .get("max_output_bytes")
                        .and_then(Value::as_u64)
                        .unwrap_or(12000),
                });
            }
            other => {
                return DeterministicLocalLoopGate::Blocked {
                    failure_code: "unsupported_action_type",
                    failure_message: format!(
                        "Unsupported deterministic local loop action: {other}"
                    ),
                    needed_input: Some(
                        "Use write_file/create_file or command_run/validation actions.".to_string(),
                    ),
                };
            }
        }
    }
    if let Some(validation) = manifest.get("validation").filter(|value| value.is_object()) {
        let (_, validation) = runtime_lane_manifest_action_payload(validation);
        if permission_for(permissions, "command.run") != PermissionTrit::Allow {
            return DeterministicLocalLoopGate::Blocked {
                failure_code: "permission_denied",
                failure_message: "command.run permission is required for validation.".to_string(),
                needed_input: Some("Grant command.run or remove validation.".to_string()),
            };
        }
        let Some(cmd) = runtime_lane_manifest_command(validation) else {
            return DeterministicLocalLoopGate::Blocked {
                failure_code: "command_required",
                failure_message: "validation requires cmd or command.".to_string(),
                needed_input: Some(
                    "Add validation.cmd as a string array or shell command string.".to_string(),
                ),
            };
        };
        actions.push(DeterministicLocalAction::CommandRun {
            cwd: workspace_root.clone(),
            cmd,
            timeout_seconds: validation
                .get("timeout_seconds")
                .and_then(Value::as_u64)
                .unwrap_or(30),
            max_output_bytes: validation
                .get("max_output_bytes")
                .and_then(Value::as_u64)
                .unwrap_or(12000),
        });
    }
    let mutation_count = actions
        .iter()
        .filter(|action| {
            matches!(
                action,
                DeterministicLocalAction::WriteFile { .. }
                    | DeterministicLocalAction::PatchFile { .. }
            )
        })
        .count();
    if mutation_count == 0 {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "mutation_action_required",
            failure_message: "Deterministic local loops require at least one mutation action."
                .to_string(),
            needed_input: Some("Add a write_file or file_patch action.".to_string()),
        };
    }
    DeterministicLocalLoopGate::Candidate(DeterministicLocalLoopCandidate {
        workspace_root,
        requires_validation: actions
            .iter()
            .any(|action| matches!(action, DeterministicLocalAction::CommandRun { .. })),
        actions,
    })
}

fn runtime_lane_manifest_action_payload(action: &Value) -> (Option<&str>, &Value) {
    for key in [
        "write_file",
        "create_file",
        "file_write",
        "write",
        "file_patch",
        "patch_file",
        "patch",
        "command_run",
        "run_command",
        "validation",
        "validate",
    ] {
        if let Some(inner) = action.get(key).filter(|value| value.is_object()) {
            return (Some(key), inner);
        }
    }
    (None, action)
}

fn runtime_lane_bounded_existing_project_edit_loop_eligible(
    prompt: &str,
    workspace_root: &Path,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> bool {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return false;
    }
    if permission_for(permissions, "file.write") != PermissionTrit::Allow
        || permission_for(permissions, "file.patch") != PermissionTrit::Allow
        || permission_for(permissions, "command.run") != PermissionTrit::Allow
    {
        return false;
    }
    if runtime_lane_extract_explicit_file_content(prompt).is_some()
        || runtime_lane_extract_deterministic_manifest(prompt).is_some()
    {
        return false;
    }
    if !workspace_root.join("src").exists() && !workspace_root.join("tests").exists() {
        return false;
    }
    let lower = prompt.to_ascii_lowercase();
    let has_mutation_intent = [
        "create",
        "write",
        "build",
        "implement",
        "add",
        "generate",
        "make",
        "extend",
    ]
    .iter()
    .any(|token| lower.contains(token));
    let existing_project_signal = [
        "existing",
        "local files",
        "local package",
        "read relevant",
        "preserve",
        "regression tests",
        "validation command",
    ]
    .iter()
    .any(|token| lower.contains(token));
    let broad_architecture_signal = [
        "from scratch",
        "new project",
        "architecture",
        "stack",
        "database",
        "multi-service",
        "deploy",
        "docker",
    ]
    .iter()
    .any(|token| lower.contains(token));
    has_mutation_intent && existing_project_signal && !broad_architecture_signal
}

fn runtime_lane_bounded_new_file_first_edit_eligible(
    prompt: &str,
    workspace_root: &Path,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> bool {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return false;
    }
    if permission_for(permissions, "file.write") != PermissionTrit::Allow
        || permission_for(permissions, "file.patch") != PermissionTrit::Allow
        || permission_for(permissions, "command.run") != PermissionTrit::Allow
    {
        return false;
    }
    if runtime_lane_extract_explicit_file_content(prompt).is_some()
        || runtime_lane_extract_deterministic_manifest(prompt).is_some()
    {
        return false;
    }
    if workspace_root.join("src").exists() || workspace_root.join("tests").exists() {
        return false;
    }
    let lower = prompt.to_ascii_lowercase();
    let has_mutation_intent = [
        "create",
        "write",
        "build",
        "implement",
        "add",
        "generate",
        "make",
    ]
    .iter()
    .any(|token| lower.contains(token));
    if !has_mutation_intent {
        return false;
    }
    let broad_architecture_signal = [
        "from scratch",
        "new project",
        "architecture",
        "stack",
        "database",
        "multi-service",
        "deploy",
        "docker",
    ]
    .iter()
    .any(|token| lower.contains(token));
    if broad_architecture_signal {
        return false;
    }
    let Some(raw_target) = runtime_lane_extract_target_file_path(prompt) else {
        return false;
    };
    let Ok(target_path) =
        runtime_lane_resolve_target_path(&raw_target, Some(&workspace_root.to_path_buf()))
    else {
        return false;
    };
    !target_path.exists()
}

#[derive(Debug, Clone, Copy)]
struct RuntimeLanePlanningDepthProfile {
    depth: u8,
    name: &'static str,
    setup_scope: &'static str,
    planning_budget_ms: u64,
    first_mutation_target_ms: u64,
    pre_mutation_read_budget: u64,
    escalation_policy: &'static str,
}

impl RuntimeLanePlanningDepthProfile {
    fn to_json(self) -> Value {
        json!({
            "depth": self.depth,
            "name": self.name,
            "setup_scope": self.setup_scope,
            "planning_budget_ms": self.planning_budget_ms,
            "first_mutation_target_ms": self.first_mutation_target_ms,
            "pre_mutation_read_budget": self.pre_mutation_read_budget,
            "escalation_policy": self.escalation_policy,
        })
    }
}

fn runtime_lane_planning_depth_selector(
    prompt: &str,
    metadata: &Value,
) -> RuntimeLanePlanningDepthProfile {
    let requested_depth = runtime_lane_planning_depth_requested_name(metadata)
        .unwrap_or_else(|| runtime_lane_infer_planning_depth_name(prompt));
    let mut profile = runtime_lane_planning_depth_profile(requested_depth)
        .unwrap_or_else(|| runtime_lane_planning_depth_profile("local_slice").unwrap());
    profile.planning_budget_ms = runtime_lane_planning_depth_u64(
        metadata,
        "planning_budget_ms",
        profile.name,
        profile.planning_budget_ms,
    );
    profile.first_mutation_target_ms = runtime_lane_planning_depth_u64(
        metadata,
        "first_mutation_target_ms",
        profile.name,
        profile.first_mutation_target_ms,
    );
    profile.pre_mutation_read_budget = runtime_lane_planning_depth_u64(
        metadata,
        "pre_mutation_read_budget",
        profile.name,
        profile.pre_mutation_read_budget,
    );
    profile
}

fn runtime_lane_planning_depth_requested_name(metadata: &Value) -> Option<&str> {
    for path in [
        "/native_success_criteria/planning_depth",
        "/workflow/native_success_criteria/planning_depth",
        "/native_success_criteria/planning_depth_selector/default_depth",
        "/workflow/native_success_criteria/planning_depth_selector/default_depth",
    ] {
        if let Some(value) = metadata.pointer(path).and_then(Value::as_str) {
            return Some(value);
        }
    }
    None
}

fn runtime_lane_infer_planning_depth_name(prompt: &str) -> &'static str {
    let lower = prompt.to_lowercase();
    let direct_edit_signal = [
        "hello world",
        "single file",
        "one file",
        "small edit",
        "simple file",
        "create a file",
    ]
    .iter()
    .any(|token| lower.contains(token));
    if direct_edit_signal {
        return "direct_bounded_edit";
    }
    let architecture_signal = [
        "architecture",
        "stack",
        "scaffold",
        "new project",
        "from scratch",
        "bootstrap",
        "system design",
    ]
    .iter()
    .any(|token| lower.contains(token));
    if architecture_signal {
        return "architecture_bootstrap";
    }
    let context_pack_signal = [
        "multiple files",
        "multi-file",
        "integration",
        "database",
        "persistence",
        "public api",
        "existing project",
    ]
    .iter()
    .any(|token| lower.contains(token));
    if context_pack_signal {
        return "context_pack_implementation_plan";
    }
    "local_slice"
}

fn runtime_lane_planning_depth_profile(name: &str) -> Option<RuntimeLanePlanningDepthProfile> {
    match name {
        "0" | "depth0" | "direct_bounded_edit" => Some(RuntimeLanePlanningDepthProfile {
            depth: 0,
            name: "direct_bounded_edit",
            setup_scope: "use supplied prompt and obvious local target only",
            planning_budget_ms: 2_000,
            first_mutation_target_ms: 5_000,
            pre_mutation_read_budget: 0,
            escalation_policy:
                "escalate only when no safe target path or required content can be inferred",
        }),
        "1" | "depth1" | "local_slice" => Some(RuntimeLanePlanningDepthProfile {
            depth: 1,
            name: "local_slice",
            setup_scope: "read bounded relevant files, make the smallest safe slice plan, then mutate",
            planning_budget_ms: 8_000,
            first_mutation_target_ms: 15_000,
            pre_mutation_read_budget: 4,
            escalation_policy: "escalate only for missing relevant files, unsafe ambiguity, or cross-module dependency evidence",
        }),
        "2" | "depth2" | "context_pack_implementation_plan" => Some(RuntimeLanePlanningDepthProfile {
            depth: 2,
            name: "context_pack_implementation_plan",
            setup_scope: "build a local context pack and implementation plan for multi-file existing-project work",
            planning_budget_ms: 20_000,
            first_mutation_target_ms: 30_000,
            pre_mutation_read_budget: 12,
            escalation_policy: "escalate only when architecture, stack, or checkpoint scope must be decided before editing",
        }),
        "3" | "depth3" | "architecture_bootstrap" => Some(RuntimeLanePlanningDepthProfile {
            depth: 3,
            name: "architecture_bootstrap",
            setup_scope: "define architecture, stack, interfaces, and project layout before implementation",
            planning_budget_ms: 45_000,
            first_mutation_target_ms: 60_000,
            pre_mutation_read_budget: 24,
            escalation_policy:
                "escalate to checkpointed operation when the task cannot be completed in one coherent slice",
        }),
        "4" | "depth4" | "checkpointed_project_operator" => Some(RuntimeLanePlanningDepthProfile {
            depth: 4,
            name: "checkpointed_project_operator",
            setup_scope: "run long-horizon checkpoint planning, memory handoff, validation loops, and stop conditions",
            planning_budget_ms: 90_000,
            first_mutation_target_ms: 120_000,
            pre_mutation_read_budget: 40,
            escalation_policy: "stop or ask for user input when the checkpoint boundary, requirements, or blocker state is unclear",
        }),
        _ => None,
    }
}

fn runtime_lane_planning_depth_u64(
    metadata: &Value,
    key: &str,
    depth_name: &str,
    fallback: u64,
) -> u64 {
    for path in [
        format!("/native_success_criteria/planning_depth_selector/{key}/{depth_name}"),
        format!("/workflow/native_success_criteria/planning_depth_selector/{key}/{depth_name}"),
        format!("/native_success_criteria/{key}"),
        format!("/workflow/native_success_criteria/{key}"),
    ] {
        if let Some(value) = metadata.pointer(&path).and_then(Value::as_u64) {
            return value;
        }
    }
    fallback
}

fn runtime_lane_context_pack_file_count(context_pack: &str) -> usize {
    context_pack
        .lines()
        .find_map(|line| {
            line.strip_prefix("selected_files:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0)
}

fn runtime_lane_attach_coding_runtime_probe(response: &mut RuntimeLaneResponse, probe: Value) {
    if let Some(object) = response.receipt.as_object_mut() {
        object.insert("coding_runtime_probe".to_string(), probe.clone());
    }
    if let Some(object) = response.trace_summary.as_object_mut() {
        object.insert("coding_runtime_probe".to_string(), probe);
    }
}

fn runtime_lane_apply_planning_depth_prompt(
    mut prompt: String,
    profile: &RuntimeLanePlanningDepthProfile,
) -> String {
    prompt.push_str(&format!(
        "\nPlanning: {}. Mutate first; escalate only if unsafe.\n",
        profile.name
    ));
    prompt
}

fn runtime_lane_bounded_existing_project_edit_loop_system() -> String {
    "You are the bounded_existing_project_edit_loop fast lane for a primitive-first local coding runtime.\n\
Return only valid JSON. No markdown. No prose.\n\
Use this exact shape: {\"deterministic_local_loop\":{\"workspace_root\":\"/absolute/path\",\"actions\":[...]}}.\n\
Allowed actions: file_patch with path, old, new, allow_multiple; write_file with path, content, overwrite; command_run with cmd.\n\
Order source/test file_patch or write_file actions before command_run actions. Baseline validation output is setup context for the first edit; validation repair is only legal after a mutation receipt exists.\n\
Respect the planning depth profile supplied in the prompt as a ceiling. Depth 0/1 must not perform architecture or stack planning; mutate from supplied local context or return a structured blocker.\n\
Prefer file_patch for existing-file localized edits. Use write_file only for new files or broad replacements.\n\
Keep the action list small. Use exact old text from the context pack for patches. Preserve existing behavior.\n\
Preserve existing public import paths and owner modules. If tests, probes, or callers import a symbol from a module, make that module provide the symbol directly; a package __init__ re-export alone is not enough.\n\
Include focused regression tests when requested. Include validation and semantic probe commands when present.\n\
Only return {\"structured_blocker\":{\"reason\":\"insufficient_context\"}} when the context pack lacks the source/test files needed to make a safe edit. If relevant source and tests are present, produce the smallest safe manifest."
        .to_string()
}

fn runtime_lane_bounded_existing_project_edit_loop_prompt(
    prompt: &str,
    workspace_root: &Path,
    context_pack: &str,
    public_api_bindings: &[RuntimeLanePublicApiBinding],
) -> String {
    let compact_context = runtime_lane_compact_context_pack(context_pack);
    let public_api_section =
        runtime_lane_public_api_bindings_prompt_section(public_api_bindings, workspace_root);
    let public_api_write_strategy = if public_api_bindings.is_empty() {
        "No public API owner binding strategy is active.".to_string()
    } else {
        "Public API extension write strategy: for owner modules and focused regression tests, prefer file_patch for additive changes. If using write_file with overwrite=true on an existing Python source file, the full content must preserve every existing class, public import path, and annotated field from the context pack.".to_string()
    };
    format!(
        "Workspace root: {}\n\nUser task:\n{}\n\n{}\n{}\n\nCompact authoritative local context:\n{}\n\nReturn the smallest safe deterministic_local_loop manifest. Prefer patch actions for existing files. Preserve existing public import paths, owner modules, Python classes, and annotated fields. If a validation test or semantic probe imports a symbol from a module, make that module provide the symbol directly. Run validation and the semantic probe command if supplied by the task.",
        workspace_root.display(),
        prompt.trim(),
        public_api_section,
        public_api_write_strategy,
        compact_context
    )
}

fn runtime_lane_model_manifest_planner_eligible(
    metadata: &Value,
    prompt: &str,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> bool {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return false;
    }
    if permission_for(permissions, "file.write") != PermissionTrit::Allow {
        return false;
    }
    if runtime_lane_extract_workspace_root(prompt).is_none() {
        return false;
    }
    if runtime_lane_extract_explicit_file_content(prompt).is_some()
        || runtime_lane_extract_deterministic_manifest(prompt).is_some()
    {
        return false;
    }
    if !runtime_lane_model_manifest_planner_enabled_for_task_lane(metadata, prompt) {
        return false;
    }
    let lower = prompt.to_ascii_lowercase();
    let has_mutation_intent = [
        "create",
        "write",
        "build",
        "implement",
        "add",
        "generate",
        "make",
    ]
    .iter()
    .any(|token| lower.contains(token));
    let requires_existing_discovery = [
        "existing project",
        "modify existing",
        "refactor",
        "debug",
        "fix bug",
        "inspect",
        "read the project",
        "look through",
    ]
    .iter()
    .any(|token| lower.contains(token));
    has_mutation_intent && !requires_existing_discovery
}

fn runtime_lane_model_manifest_planner_enabled_for_task_lane(metadata: &Value, prompt: &str) -> bool {
    let task_lane = crate::native_evidence::native_tool_coding_task_lane(metadata, prompt);
    runtime_lane_metadata_string_list_allows(
        metadata,
        &[
            "/native_success_criteria/model_manifest_planner_enabled_lanes",
            "/workflow/native_success_criteria/model_manifest_planner_enabled_lanes",
        ],
        task_lane,
    )
    .unwrap_or_else(|| {
        runtime_lane_metadata_bool(
            metadata,
            &[
                "/native_success_criteria/model_manifest_planner_enabled",
                "/workflow/native_success_criteria/model_manifest_planner_enabled",
            ],
        )
        .unwrap_or(true)
    })
}

fn runtime_lane_should_skip_model_manifest_for_checkpointed_tool_loop(
    metadata: &Value,
    prompt: &str,
) -> bool {
    if !runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/checkpointed_project_operator_tool_loop_enabled",
            "/workflow/native_success_criteria/checkpointed_project_operator_tool_loop_enabled",
        ],
    )
    .unwrap_or(false)
    {
        return false;
    }
    let lower = prompt.to_ascii_lowercase();
    let checkpointed_operator_task = runtime_lane_text_contains_any(
        &lower,
        &[
            "checkpoint",
            "handoff",
            "resume token",
            "memory row",
            "memory db",
            "project context",
        ],
    );
    let project_tool_loop_task = runtime_lane_text_contains_any(
        &lower,
        &[
            "existing project",
            "project root",
            "local project",
            "read the local project",
            "validation command",
        ],
    );
    let durable_vertical_slice = runtime_lane_text_contains_any(
        &lower,
        &[
            "persistent",
            "persistence",
            "storage",
            "service integration",
            "operator-facing",
            "cli",
            "report",
            "import/export",
            "round trip",
            "roundtrip",
        ],
    );
    checkpointed_operator_task && project_tool_loop_task && durable_vertical_slice
}

fn runtime_lane_checkpointed_tool_loop_prompt(
    prompt: &str,
    bootstrap_receipts: &[NativeToolReceipt],
) -> String {
    let bootstrap_context = runtime_lane_checkpointed_bootstrap_context_packet(prompt);
    let first_tool_instruction = runtime_lane_checkpointed_first_tool_instruction(prompt);
    let bootstrap_receipt_packet = if bootstrap_receipts.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(bootstrap_receipts).unwrap_or_else(|_| "[]".to_string())
    };
    format!(
        "{}\n\nRuntime lane guidance:\n\
Checkpointed existing-project bootstrap is already complete. Current local files are authoritative.\n\
\n\
Bootstrap context packet:\n{}\n\
\n\
Runtime-dispatched bootstrap receipts:\n{}\n\
\n\
First native tool contract:\n\
- Your first assistant action must be a native tool call, not a prose plan.\n\
- Bootstrap receipts above already count as discovery; do not repeat file_list/file_read for files included there.\n\
- {}.\n\
- Do not start with validation or checkpoint closure.\n\
- After any checkpoint-memory read, immediately mutate the product slice with file_write/file_patch.\n\
\n\
Execution contract:\n\
- Use the native tool loop incrementally: use the bootstrapped context, retrieve required checkpoint memory, edit the vertical slice, validate, then close receipt and memory artifacts.\n\
- Use native file_write/file_patch for source, test, receipt, and local artifact mutations so mutation receipts are explicit.\n\
- Use command_run for validation and required project/memory CLI commands, not as the primary way to rewrite source files.\n\
- Do not stop after a model/store-only slice; complete the requested model, persistence, service, operator/report, tests, validation, receipt, and memory closure when those capabilities are requested.",
        prompt.trim(),
        bootstrap_context,
        bootstrap_receipt_packet,
        first_tool_instruction
    )
}

fn runtime_lane_checkpointed_bootstrap_receipts(prompt: &str) -> Vec<NativeToolReceipt> {
    let Some(workspace_root) = runtime_lane_extract_workspace_root(prompt) else {
        return Vec::new();
    };
    let dispatcher = NativeToolDispatcher::new(&[
        "file_list".to_string(),
        "file_read_many".to_string(),
        "command_run".to_string(),
    ]);
    let mut receipts = Vec::<NativeToolReceipt>::new();
    let file_list_receipt = dispatcher.dispatch(NativeToolCall {
        id: "checkpointed_project_bootstrap_file_list".to_string(),
        name: "file_list".to_string(),
        args: json!({
            "path": workspace_root.display().to_string(),
            "recursive": true,
            "max_depth": 3,
            "max_entries": 200
        }),
    });
    receipts.push(file_list_receipt);

    let context_paths = runtime_lane_checkpointed_bootstrap_context_paths(prompt, &workspace_root);
    if !context_paths.is_empty() {
        receipts.push(dispatcher.dispatch(NativeToolCall {
            id: "checkpointed_project_bootstrap_file_read_many".to_string(),
            name: "file_read_many".to_string(),
            args: json!({
                "paths": context_paths,
            }),
        }));
    }

    if let Some(command) = runtime_lane_checkpointed_memory_get_command(prompt) {
        receipts.push(dispatcher.dispatch(NativeToolCall {
            id: "checkpointed_project_bootstrap_memory_read".to_string(),
            name: "command_run".to_string(),
            args: json!({
                "cwd": workspace_root.display().to_string(),
                "cmd": command,
                "timeout_seconds": 45,
                "max_output_bytes": 8_000
            }),
        }));
    }

    receipts
}

fn runtime_lane_checkpointed_bootstrap_context_paths(
    prompt: &str,
    workspace_root: &Path,
) -> Vec<String> {
    let context_pack = runtime_lane_model_manifest_context_pack(prompt, workspace_root);
    let mut paths = Vec::<String>::new();
    for line in context_pack.lines() {
        let Some(rest) = line.strip_prefix("--- file: ") else {
            continue;
        };
        let relative = rest
            .split_once(" role:")
            .map(|(path, _)| path)
            .unwrap_or(rest)
            .trim();
        if relative.is_empty() || relative.contains("..") || relative.starts_with('/') {
            continue;
        }
        let full_path = workspace_root.join(relative);
        if full_path.is_file() {
            let display = full_path.display().to_string();
            if !paths.iter().any(|path| path == &display) {
                paths.push(display);
            }
        }
        if paths.len() >= 8 {
            break;
        }
    }
    paths
}

fn runtime_lane_merge_bootstrap_receipts_into_agent_run(
    run: &mut AgentRunResult,
    bootstrap_receipts: &[NativeToolReceipt],
) {
    let Some(receipt_object) = run.receipt.as_object_mut() else {
        return;
    };
    let mut merged_receipts = bootstrap_receipts
        .iter()
        .cloned()
        .map(|receipt| json!(receipt))
        .collect::<Vec<_>>();
    if let Some(existing) = receipt_object
        .get("native_tool_receipts")
        .and_then(Value::as_array)
    {
        merged_receipts.extend(existing.iter().cloned());
    }
    receipt_object.insert(
        "runtime_bootstrap_native_tool_receipts".to_string(),
        json!(bootstrap_receipts),
    );
    receipt_object.insert(
        "native_tool_call_count".to_string(),
        json!(merged_receipts.len()),
    );
    receipt_object.insert("native_tool_receipts".to_string(), json!(merged_receipts));
}

fn runtime_lane_try_checkpointed_mutation_entry_recovery(
    run: &mut AgentRunResult,
    providers: &ProviderClientRegistry,
    provider_id: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    prompt: &str,
    _tools: &[String],
) -> bool {
    if !runtime_lane_agent_run_looks_like_provider_timeout(run)
        && runtime_lane_agent_run_has_successful_tool_call(run)
    {
        return false;
    }
    let Some(workspace_root) = runtime_lane_extract_workspace_root(prompt) else {
        return false;
    };
    let provider_id = provider_id
        .map(ToOwned::to_owned)
        .or_else(|| {
            let provider = run.response.provider.trim();
            if provider.is_empty() {
                None
            } else {
                Some(provider.to_string())
            }
        })
        .unwrap_or_else(|| "ollama".to_string());
    let provider = match providers.from_provider_id(&provider_id) {
        Ok(provider) => provider,
        Err(_) => return false,
    };
    let receipts = runtime_lane_agent_run_native_tool_receipts(run);
    let recovery_prompt =
        runtime_lane_checkpointed_mutation_entry_recovery_prompt(prompt, &workspace_root, &receipts);
    let mut recovery_metadata = metadata.clone();
    if let Some(object) = recovery_metadata.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(120));
        object.insert("provider_stream_until_tool_calls".to_string(), json!(true));
        object.insert("omit_ollama_thinking_flags".to_string(), json!(false));
        object.insert(
            "runtime_recovery_lane".to_string(),
            json!("checkpointed_mutation_entry"),
        );
    }
    let request = ProviderRequest {
        prompt: recovery_prompt,
        system: Some(runtime_lane_checkpointed_mutation_entry_recovery_system()),
        tools: vec!["file_write".to_string(), "file_patch".to_string()],
        model: model.cloned(),
        metadata: recovery_metadata,
    };
    let response = match provider.stream_complete(&request) {
        Ok(stream) => stream.response,
        Err(error) => {
            runtime_lane_append_synthetic_agent_receipt(
                run,
                "checkpointed_mutation_entry_provider_failure",
                "fail",
                json!({
                    "error_code": error.code.as_str(),
                    "error_message": error.message,
                }),
            );
            return false;
        }
    };
    let mut calls = parse_native_tool_calls(&response.output)
        .into_iter()
        .filter_map(|call| runtime_lane_checkpointed_mutation_entry_call(call, &workspace_root))
        .take(6)
        .collect::<Vec<_>>();
    if calls.is_empty() {
        runtime_lane_append_synthetic_agent_receipt(
            run,
            "checkpointed_mutation_entry_no_tool_calls",
            "fail",
            json!({
                "provider": response.provider,
                "model": response.model,
                "output_preview": response.output.chars().take(1200).collect::<String>(),
            }),
        );
        return false;
    }
    let dispatcher = NativeToolDispatcher::with_metadata(
        &["file_write".to_string(), "file_patch".to_string()],
        metadata,
    );
    let mut mutation_receipts = Vec::<NativeToolReceipt>::new();
    for (index, call) in calls.drain(..).enumerate() {
        let mut call = call;
        call.id = format!("checkpointed_mutation_entry_{}", index + 1);
        mutation_receipts.push(dispatcher.dispatch(call));
    }
    runtime_lane_append_agent_run_receipts(run, &mutation_receipts);
    if runtime_lane_agent_run_has_no_successful_mutation(run) {
        runtime_lane_append_synthetic_agent_receipt(
            run,
            "checkpointed_mutation_entry_no_successful_mutation",
            "fail",
            json!({
                "provider": response.provider,
                "model": response.model,
                "output_preview": response.output.chars().take(1200).collect::<String>(),
            }),
        );
        return false;
    }
    let integration_receipts = if runtime_lane_checkpointed_changed_files(&mutation_receipts).len() < 3 {
        runtime_lane_checkpointed_integration_stage(
            providers,
            Some(provider_id.as_str()),
            model,
            metadata,
            prompt,
            &workspace_root,
            &mutation_receipts,
        )
    } else {
        Vec::new()
    };
    if !integration_receipts.is_empty() {
        runtime_lane_append_agent_run_receipts(run, &integration_receipts);
    }
    let mut product_receipts = mutation_receipts.clone();
    product_receipts.extend(
        integration_receipts
            .iter()
            .filter(|receipt| runtime_lane_receipt_is_successful_mutation(receipt))
            .cloned(),
    );
    let closeout_receipts = runtime_lane_checkpointed_mutation_closeout(
        prompt,
        &workspace_root,
        metadata,
        &product_receipts,
    );
    if !closeout_receipts.is_empty() {
        runtime_lane_append_agent_run_receipts(run, &closeout_receipts);
    }
    run.response.output = format!(
        "{}\n\nCheckpointed staged recovery produced {} entry mutation receipt(s), {} integration receipt(s), and {} closeout receipt(s).",
        response.output,
        mutation_receipts.len(),
        integration_receipts.len(),
        closeout_receipts.len()
    );
    if let Some(object) = run.receipt.as_object_mut() {
        object.insert("status".to_string(), json!("ok"));
        object.insert(
            "checkpointed_mutation_entry_recovery".to_string(),
            json!({
                "provider": response.provider,
                "model": response.model,
                "usage_tokens": response.usage_tokens,
                "mutation_receipt_count": mutation_receipts.len(),
                "integration_receipt_count": integration_receipts.len(),
                "closeout_receipt_count": closeout_receipts.len(),
            }),
        );
    }
    true
}

fn runtime_lane_try_checkpointed_pre_agent_entry(
    name: &str,
    prompt: &str,
    provider_id: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    tools: &[String],
    bootstrap_receipts: &[NativeToolReceipt],
    providers: &ProviderClientRegistry,
) -> RuntimeLaneResponse {
    let selected_provider = provider_id.unwrap_or("ollama");
    let started_at_ms = Utc::now().timestamp_millis();
    let mut run = AgentRunResult {
        response: ProviderResponse {
            provider: selected_provider.to_string(),
            model: model.cloned().unwrap_or_default(),
            output: "checkpointed_pre_agent_entry: provider_timeout bypass; starting staged mutation entry before full agent turn".to_string(),
            usage_tokens: 0,
            raw: json!({
                "runtime_stage": "checkpointed_pre_agent_entry",
            }),
        },
        receipt: json!({
            "status": "partial_timeout",
            "runtime_stage": "checkpointed_pre_agent_entry",
            "native_tool_call_count": bootstrap_receipts.len(),
            "native_tool_receipts": bootstrap_receipts,
        }),
        trace: ReceiptSpan {
            trace_id: format!("checkpointed_pre_agent_entry_{started_at_ms}"),
            agent_name: name.to_string(),
            started_at_ms,
            events: Vec::new(),
            attributes: BTreeMap::new(),
        },
    };
    runtime_lane_try_checkpointed_mutation_entry_recovery(
        &mut run,
        providers,
        provider_id,
        model,
        metadata,
        prompt,
        tools,
    );
    let status = run
        .receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail_closed")
        .to_string();
    let has_mutation = !runtime_lane_agent_run_has_no_successful_mutation(&run);
    let ok = status == "ok" && has_mutation;
    let error = if ok {
        None
    } else {
        Some("runtime_lane_checkpointed_pre_agent_mutation_entry_failed".to_string())
    };
    RuntimeLaneResponse {
        ok,
        contract: json!({
            "name": name,
            "provider": selected_provider,
            "agent_status": status,
            "native_tool_call_count": run
                .receipt
                .get("native_tool_call_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            "tools": tools,
            "checkpointed_pre_agent_entry": true,
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
        }),
        receipt: run.receipt,
        trace_summary: json!({
            "trace_id": run.trace.trace_id,
            "event_count": run.trace.events.len(),
            "agent_name": run.trace.agent_name,
            "runtime_stage": "checkpointed_pre_agent_entry",
            "status": if ok { "ok" } else { "fail_closed" },
        }),
        output: run.response.output,
        error,
    }
}

fn runtime_lane_checkpointed_mutation_entry_recovery_system() -> String {
    "You are the checkpointed_mutation_entry runtime lane. Return only JSON tool calls. Allowed tools: file_write and file_patch. Forbidden tools: file_list, file_stat, file_read, file_read_many, command_run, command_resolve. Do not write prose, markdown, validation commands, checkpoint receipts, or memory rows. Never return placeholders such as \"...\", \"/absolute/path\", \"exact observed text\", or \"replacement text\". Existing Python classes and annotated fields are preserved API: never replace a class with a partial class body and never remove observed fields. Return at most four tool calls. Your job is the smallest coherent additive checkpoint slice that can pass validation.".to_string()
}

fn runtime_lane_checkpointed_mutation_entry_recovery_prompt(
    prompt: &str,
    workspace_root: &Path,
    receipts: &[NativeToolReceipt],
) -> String {
    let receipt_packet = runtime_lane_checkpointed_mutation_entry_receipt_packet(receipts);
    let shape_guard_feedback = runtime_lane_checkpointed_shape_guard_feedback(receipts);
    let task_brief = runtime_lane_checkpointed_task_brief(prompt, workspace_root);
    format!(
        "Workspace root: {}\n\nTask brief:\n{}\n\nShape guard feedback:\n{}\n\nSmall source/memory packet:\n{}\n\nReturn JSON only, using one of these concrete tool forms:\n{{\"tool_calls\":[{{\"name\":\"file_patch\",\"args\":{{\"path\":\"ACTUAL_ABSOLUTE_PATH_FROM_SOURCE_PACKET\",\"old\":\"ACTUAL_TEXT_COPIED_FROM_SOURCE_PACKET\",\"new\":\"FULL_REPLACEMENT_TEXT\",\"allow_multiple\":false}}}}]}}\n{{\"tool_calls\":[{{\"name\":\"file_write\",\"args\":{{\"path\":\"ACTUAL_ABSOLUTE_PATH_UNDER_WORKSPACE\",\"content\":\"FULL_FILE_CONTENT\",\"overwrite\":true}}}}]}}\n\nStage-one mutation rules:\n- Placeholder values are invalid. Do not output \"...\", \"ACTUAL_*\", \"/absolute/path\", or prose.\n- Use absolute paths under the workspace root.\n- Make the smallest coherent product slice now, up to four file_write/file_patch calls.\n- Preserve named user-requested surfaces literally: if the task asks for report, destination summary, retryable, CLI, import/export, or round-trip behavior, include those public terms in source/CLI/tests now.\n- If the task asks for durable delivery/report/CLI behavior, include a source module, CLI or service integration, and focused tests when possible.\n- Prefer file_write for new focused source/test files and small full-file integrations. Use file_patch only with exact old text from the packet.\n- Preserve every observed class and annotated field in existing Python source.\n- Do not validate, write checkpoint receipts, or write memory rows in this lane; the runtime controller performs closeout after successful mutation.",
        workspace_root.display(),
        task_brief,
        shape_guard_feedback,
        receipt_packet
    )
}

fn runtime_lane_checkpointed_mutation_entry_receipt_packet(
    receipts: &[NativeToolReceipt],
) -> String {
    let mut packet = String::new();
    if let Some(memory_hint) = runtime_lane_checkpointed_memory_hint(receipts) {
        packet.push_str("memory_hint:\n");
        runtime_lane_append_bounded_corpus(&mut packet, &memory_hint, 1_400);
        packet.push_str("\n\n");
    }
    let mut snippets = Vec::<Value>::new();
    let mut seen_paths = Vec::<String>::new();
    for receipt in receipts {
        if matches!(receipt.tool_name.as_str(), "file_read_many" | "file_read") {
            runtime_lane_checkpointed_collect_file_snippets(
                &receipt.result,
                &mut snippets,
                &mut seen_paths,
            );
        }
    }
    if !snippets.is_empty() {
        packet.push_str("source_snippets:\n");
        for snippet in snippets.into_iter().take(6) {
            packet.push_str(&snippet.to_string());
            packet.push('\n');
            if packet.len() >= 6_000 {
                break;
            }
        }
    }
    if packet.trim().is_empty() {
        return "No compact source packet available; use only supplied task brief and shape guard feedback."
            .to_string();
    }
    runtime_lane_compact_context_pack(&packet)
}

fn runtime_lane_checkpointed_task_brief(prompt: &str, workspace_root: &Path) -> String {
    let mut lines = Vec::<String>::new();
    lines.push(format!("workspace_root={}", workspace_root.display()));
    for marker in [
        "Goal:",
        "Python package:",
        "Expected new memory row id:",
        "Validation command from project root:",
    ] {
        if let Some(value) = runtime_lane_prompt_line_value(prompt, marker) {
            lines.push(format!(
                "{}={}",
                marker.trim_end_matches(':').to_ascii_lowercase().replace(' ', "_"),
                value
            ));
        }
    }
    for line in prompt.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if trimmed.is_empty() {
            continue;
        }
        if lower.contains("implement checkpoint_")
            || lower.contains("persistent")
            || lower.contains("delivery-attempt")
            || lower.contains("report")
            || lower.contains("retryable")
            || lower.contains("import/export")
            || lower.contains("round-trip")
            || lower.contains("cli")
            || lower.contains("preserving baseline")
        {
            lines.push(trimmed.to_string());
        }
        if lines.len() >= 10 {
            break;
        }
    }
    lines.push("stage=mutation_entry_only; create one small source/test foothold; runtime validates and writes closeout artifacts after successful mutation".to_string());
    lines.join("\n")
}

fn runtime_lane_checkpointed_memory_hint(receipts: &[NativeToolReceipt]) -> Option<String> {
    for receipt in receipts {
        if receipt.tool_name != "command_run" || receipt.call_id != "runtime_checkpoint_memory_context_get" {
            continue;
        }
        let stdout = receipt.result.get("stdout").and_then(Value::as_str)?;
        let mut hints = Vec::<String>::new();
        for needle in [
            "next_slice_goal",
            "recommended_next_checkpoint",
            "constraints",
            "completed_checkpoint",
            "changed_files",
        ] {
            if let Some(index) = stdout.find(needle) {
                let start = index.saturating_sub(120);
                let end = std::cmp::min(stdout.len(), index + 700);
                hints.push(stdout[start..end].to_string());
            }
        }
        if !hints.is_empty() {
            return Some(hints.join("\n"));
        }
    }
    None
}

fn runtime_lane_checkpointed_collect_file_snippets(
    value: &Value,
    out: &mut Vec<Value>,
    seen_paths: &mut Vec<String>,
) {
    if out.len() >= 16 {
        return;
    }
    if let Some(object) = value.as_object() {
        if let (Some(path), Some(content)) = (
            object.get("path").and_then(Value::as_str),
            object.get("content").and_then(Value::as_str),
        ) {
            if !seen_paths.iter().any(|seen| seen == path) {
                seen_paths.push(path.to_string());
                let snippet = content.chars().take(900).collect::<String>();
                out.push(json!({
                    "path": path,
                    "content": snippet,
                    "truncated": content.chars().count() > 900,
                }));
            }
        }
        for child in object.values() {
            runtime_lane_checkpointed_collect_file_snippets(child, out, seen_paths);
            if out.len() >= 16 {
                break;
            }
        }
    } else if let Some(items) = value.as_array() {
        for item in items {
            runtime_lane_checkpointed_collect_file_snippets(item, out, seen_paths);
            if out.len() >= 16 {
                break;
            }
        }
    }
}

fn runtime_lane_checkpointed_integration_stage(
    providers: &ProviderClientRegistry,
    provider_id: Option<&str>,
    model: Option<&String>,
    metadata: &Value,
    prompt: &str,
    workspace_root: &Path,
    entry_mutation_receipts: &[NativeToolReceipt],
) -> Vec<NativeToolReceipt> {
    let provider_id = provider_id.unwrap_or("ollama");
    let provider = match providers.from_provider_id(provider_id) {
        Ok(provider) => provider,
        Err(_) => {
            return vec![runtime_lane_checkpointed_synthetic_receipt(
                "checkpointed_integration_provider_unavailable",
                "fail",
                json!({"provider": provider_id}),
            )]
        }
    };
    let dispatcher = NativeToolDispatcher::with_metadata(
        &[
            "file_read_many".to_string(),
            "file_write".to_string(),
            "file_patch".to_string(),
        ],
        metadata,
    );
    let context_paths = runtime_lane_checkpointed_integration_context_paths(
        prompt,
        workspace_root,
        entry_mutation_receipts,
    );
    let mut receipts = Vec::<NativeToolReceipt>::new();
    let context_receipt = dispatcher.dispatch(NativeToolCall {
        id: "checkpointed_integration_context_read".to_string(),
        name: "file_read_many".to_string(),
        args: json!({
            "paths": context_paths,
        }),
    });
    let context_packet =
        runtime_lane_checkpointed_mutation_entry_receipt_packet(&[context_receipt.clone()]);
    receipts.push(context_receipt);
    let request = ProviderRequest {
        prompt: runtime_lane_checkpointed_integration_prompt(
            prompt,
            workspace_root,
            entry_mutation_receipts,
            &context_packet,
        ),
        system: Some(runtime_lane_checkpointed_integration_system()),
        tools: vec!["file_write".to_string(), "file_patch".to_string()],
        model: model.cloned(),
        metadata: runtime_lane_checkpointed_stage_metadata(metadata, 90, "checkpointed_integration"),
    };
    let response = match provider.stream_complete(&request) {
        Ok(stream) => stream.response,
        Err(error) => {
            receipts.push(runtime_lane_checkpointed_synthetic_receipt(
                "checkpointed_integration_skipped",
                "ok",
                json!({
                    "skipped": true,
                    "reason": "optional_integration_stage_unavailable",
                    "provider_error_code": error.code.as_str(),
                }),
            ));
            return receipts;
        }
    };
    let calls = parse_native_tool_calls(&response.output)
        .into_iter()
        .filter_map(|call| runtime_lane_checkpointed_mutation_entry_call(call, workspace_root))
        .take(5)
        .collect::<Vec<_>>();
    if calls.is_empty() {
        receipts.push(runtime_lane_checkpointed_synthetic_receipt(
            "checkpointed_integration_no_tool_calls",
            "fail",
            json!({
                "provider": response.provider,
                "model": response.model,
                "output_preview": response.output.chars().take(1200).collect::<String>(),
            }),
        ));
        return receipts;
    }
    for (index, mut call) in calls.into_iter().enumerate() {
        call.id = format!("checkpointed_integration_{}", index + 1);
        receipts.push(dispatcher.dispatch(call));
    }
    receipts
}

fn runtime_lane_checkpointed_integration_system() -> String {
    "You are the checkpointed_integration runtime lane. Return only JSON tool calls. Allowed tools: file_write and file_patch. Forbidden tools: reads, command_run, validation, receipt writes, and memory writes. Never return placeholders. Make a small integration slice that connects the stage-one foothold to existing service, CLI, or tests while preserving existing behavior.".to_string()
}

fn runtime_lane_checkpointed_integration_prompt(
    prompt: &str,
    workspace_root: &Path,
    entry_mutation_receipts: &[NativeToolReceipt],
    context_packet: &str,
) -> String {
    format!(
        "Workspace root: {}\n\nTask brief:\n{}\n\nEntry mutation receipts:\n{}\n\nCurrent integration context:\n{}\n\nReturn JSON only with at most five file_write/file_patch calls.\nIntegration rules:\n- Do not use placeholders or prose.\n- Preserve existing route/service/CLI behavior.\n- Connect the new foothold to source, service, CLI, and/or regression tests as requested by the task.\n- If the task asks for reporting, retryable status, import/export, or durable round trip, expose a small source/CLI/test surface for those terms now.\n- Prefer additive patches or full small-file writes that preserve existing public classes and fields.\n- Do not validate, write checkpoint receipts, or write memory rows.",
        workspace_root.display(),
        runtime_lane_checkpointed_task_brief(prompt, workspace_root),
        serde_json::to_string(entry_mutation_receipts).unwrap_or_else(|_| "[]".to_string()),
        context_packet
    )
}

fn runtime_lane_checkpointed_stage_metadata(
    metadata: &Value,
    timeout_seconds: u64,
    stage: &str,
) -> Value {
    let mut stage_metadata = metadata.clone();
    if let Some(object) = stage_metadata.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(timeout_seconds));
        object.insert("provider_stream_until_tool_calls".to_string(), json!(true));
        object.insert("omit_ollama_thinking_flags".to_string(), json!(false));
        object.insert("runtime_recovery_lane".to_string(), json!(stage));
    }
    stage_metadata
}

fn runtime_lane_checkpointed_integration_context_paths(
    prompt: &str,
    workspace_root: &Path,
    entry_mutation_receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut paths = runtime_lane_checkpointed_changed_files(entry_mutation_receipts);
    if let Some(package) = runtime_lane_prompt_line_value(prompt, "Python package:") {
        let package_path = package.replace('.', "/");
        for relative in [
            "models.py",
            "repository.py",
            "service.py",
            "cli.py",
            "__init__.py",
        ] {
            let path = workspace_root
                .join("src")
                .join(&package_path)
                .join(relative);
            if path.is_file() {
                let path = path.display().to_string();
                if !paths.iter().any(|existing| existing == &path) {
                    paths.push(path);
                }
            }
        }
    }
    let tests_path = workspace_root.join("tests").join("test_baseline.py");
    if tests_path.is_file() {
        let path = tests_path.display().to_string();
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }
    paths
}

fn runtime_lane_checkpointed_synthetic_receipt(
    call_id: &str,
    status: &str,
    result: Value,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call_id.to_string(),
        tool_name: "runtime_checkpointed_integration_stage".to_string(),
        status: status.to_string(),
        duration_ms: 0,
        result,
        error: if status == "ok" {
            None
        } else {
            Some(status.to_string())
        },
    }
}

fn runtime_lane_checkpointed_mutation_closeout(
    prompt: &str,
    workspace_root: &Path,
    metadata: &Value,
    mutation_receipts: &[NativeToolReceipt],
) -> Vec<NativeToolReceipt> {
    let dispatcher = NativeToolDispatcher::with_metadata(
        &["command_run".to_string(), "file_write".to_string()],
        metadata,
    );
    let mut receipts = Vec::<NativeToolReceipt>::new();
    let mut validation_summary = json!({
        "status": "not_requested",
    });
    let mut validation_passed = runtime_lane_extract_validation_command(prompt, workspace_root)
        .is_none();
    if let Some(command) = runtime_lane_extract_validation_command(prompt, workspace_root) {
        let validation_receipt = dispatcher.dispatch(NativeToolCall {
            id: "checkpointed_stage_validation".to_string(),
            name: "command_run".to_string(),
            args: json!({
                "cwd": command.cwd.display().to_string(),
                "cmd": command.cmd,
                "timeout_seconds": 90,
                "max_output_bytes": 12_000,
                "runtime_stage": "checkpointed_validation",
            }),
        });
        validation_passed = runtime_lane_checkpointed_command_succeeded(&validation_receipt);
        validation_summary = runtime_lane_checkpointed_validation_summary(&validation_receipt);
        receipts.push(validation_receipt);
    }
    if !validation_passed {
        return receipts;
    }
    if let Some(receipt_call) = runtime_lane_checkpointed_receipt_file_call(
        prompt,
        mutation_receipts,
        &validation_summary,
    ) {
        receipts.push(dispatcher.dispatch(receipt_call));
    }
    if let Some(memory_call) = runtime_lane_checkpointed_memory_write_call(
        prompt,
        workspace_root,
        mutation_receipts,
        &validation_summary,
    ) {
        receipts.push(dispatcher.dispatch(memory_call));
    }
    receipts
}

fn runtime_lane_checkpointed_command_succeeded(receipt: &NativeToolReceipt) -> bool {
    if receipt.status != "ok" || receipt.tool_name != "command_run" {
        return false;
    }
    receipt
        .result
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            receipt
                .result
                .get("exit_code")
                .and_then(Value::as_i64)
                == Some(0)
        })
}

fn runtime_lane_checkpointed_validation_summary(receipt: &NativeToolReceipt) -> Value {
    json!({
        "command": receipt.result.get("cmd").cloned().unwrap_or(Value::Null),
        "status": if runtime_lane_checkpointed_command_succeeded(receipt) { "pass" } else { "fail" },
        "exit_code": receipt.result.get("exit_code").cloned().unwrap_or(Value::Null),
        "stdout_tail": receipt.result.get("stdout").and_then(Value::as_str).map(|value| value.chars().rev().take(1200).collect::<String>().chars().rev().collect::<String>()).unwrap_or_default(),
        "stderr_tail": receipt.result.get("stderr").and_then(Value::as_str).map(|value| value.chars().rev().take(1200).collect::<String>().chars().rev().collect::<String>()).unwrap_or_default(),
    })
}

fn runtime_lane_checkpointed_receipt_file_call(
    prompt: &str,
    mutation_receipts: &[NativeToolReceipt],
    validation_summary: &Value,
) -> Option<NativeToolCall> {
    let path = runtime_lane_checkpointed_receipt_path(prompt)?;
    let completed_checkpoint = runtime_lane_checkpointed_completed_checkpoint(prompt);
    let changed_files = runtime_lane_checkpointed_changed_files(mutation_receipts);
    let content = json!({
        "schema_version": "checkpoint_handoff_v1",
        "completed_checkpoint": completed_checkpoint,
        "changed_files": changed_files,
        "validation_results": validation_summary,
        "known_risks": ["staged runtime closeout is based on validation receipts and mutation receipts"],
        "recommended_next_checkpoint": runtime_lane_checkpointed_recommended_next_checkpoint(&completed_checkpoint),
    })
    .to_string();
    Some(NativeToolCall {
        id: "checkpointed_stage_receipt_write".to_string(),
        name: "file_write".to_string(),
        args: json!({
            "path": path.display().to_string(),
            "content": content,
            "overwrite": true,
            "runtime_stage": "checkpointed_receipt_closeout",
        }),
    })
}

fn runtime_lane_checkpointed_memory_write_call(
    prompt: &str,
    workspace_root: &Path,
    mutation_receipts: &[NativeToolReceipt],
    validation_summary: &Value,
) -> Option<NativeToolCall> {
    let command_pattern = runtime_lane_prompt_line_value(prompt, "Memory CLI command pattern:")?;
    if !command_pattern.contains("<command>") {
        return None;
    }
    let row_id = runtime_lane_prompt_line_value(prompt, "Expected new memory row id:")?;
    let completed_checkpoint = runtime_lane_checkpointed_completed_checkpoint(prompt);
    let content = json!({
        "schema_version": "checkpoint_memory_write_v1",
        "completed_checkpoint": completed_checkpoint,
        "changed_files": runtime_lane_checkpointed_changed_files(mutation_receipts),
        "validation_results": validation_summary,
        "known_risks": ["staged runtime closeout should be reviewed against product-level acceptance criteria"],
        "recommended_next_checkpoint": runtime_lane_checkpointed_recommended_next_checkpoint(&completed_checkpoint),
    })
    .to_string();
    let write_subcommand = format!(
        "checkpoint write --id={} --content={} --tags=coding,checkpoint,resume,project_context",
        runtime_lane_shell_single_quote(&row_id),
        runtime_lane_shell_single_quote(&content)
    );
    let command = command_pattern.replace("<command>", &write_subcommand);
    Some(NativeToolCall {
        id: "checkpointed_stage_memory_write".to_string(),
        name: "command_run".to_string(),
        args: json!({
            "cwd": workspace_root.display().to_string(),
            "cmd": ["sh", "-lc", command],
            "timeout_seconds": 90,
            "max_output_bytes": 12_000,
            "runtime_stage": "checkpointed_memory_closeout",
        }),
    })
}

fn runtime_lane_checkpointed_receipt_path(prompt: &str) -> Option<PathBuf> {
    for line in prompt.lines() {
        if !line.to_ascii_lowercase().contains("checkpoint receipt under") {
            continue;
        }
        if let Some(path) = runtime_lane_first_absolute_path_in_text(line) {
            return Some(PathBuf::from(path));
        }
    }
    None
}

fn runtime_lane_first_absolute_path_in_text(text: &str) -> Option<String> {
    let start = text.find('/')?;
    let tail = &text[start..];
    let end = tail
        .find(|character: char| character.is_whitespace())
        .unwrap_or(tail.len());
    let path = tail[..end]
        .trim_matches(|character| matches!(character, '`' | '"' | '\'' | ',' | ';' | '.'))
        .to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn runtime_lane_checkpointed_completed_checkpoint(prompt: &str) -> String {
    for line in prompt.lines() {
        let lower = line.to_ascii_lowercase();
        let Some(index) = lower.find("implement checkpoint_") else {
            continue;
        };
        let token = line[index + "implement ".len()..]
            .split_whitespace()
            .next()
            .unwrap_or("checkpoint_unknown")
            .trim_matches(|character| matches!(character, '`' | '"' | '\'' | ',' | ';' | '.'));
        if !token.is_empty() {
            return token.to_string();
        }
    }
    "checkpoint_unknown".to_string()
}

fn runtime_lane_checkpointed_changed_files(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = Vec::<String>::new();
    for receipt in receipts {
        if receipt.status != "ok" || !matches!(receipt.tool_name.as_str(), "file_write" | "file_patch") {
            continue;
        }
        if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
            if !paths.iter().any(|existing| existing == path) {
                paths.push(path.to_string());
            }
        }
    }
    paths
}

fn runtime_lane_checkpointed_recommended_next_checkpoint(completed_checkpoint: &str) -> String {
    let Some(index) = completed_checkpoint.find("checkpoint_") else {
        return "next_checkpoint".to_string();
    };
    let digits_start = index + "checkpoint_".len();
    let digits = completed_checkpoint[digits_start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if let Ok(number) = digits.parse::<u32>() {
        return format!("checkpoint_{:03}_next_slice", number + 1);
    }
    "next_checkpoint".to_string()
}

fn runtime_lane_shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn runtime_lane_checkpointed_shape_guard_feedback(receipts: &[NativeToolReceipt]) -> String {
    let mut removed_classes = Vec::<String>::new();
    let mut removed_fields = Vec::<String>::new();
    let mut restored_paths = Vec::<String>::new();
    for receipt in receipts {
        if receipt.error.as_deref()
            != Some("shell_edit_existing_python_class_shape_requires_additive_patch")
        {
            continue;
        }
        runtime_lane_extend_json_string_array(
            &mut removed_classes,
            receipt.result.get("removed_classes"),
        );
        runtime_lane_extend_json_string_array(
            &mut removed_fields,
            receipt.result.get("removed_fields"),
        );
        runtime_lane_extend_json_string_array(
            &mut restored_paths,
            receipt.result.get("restored_paths"),
        );
    }
    if removed_classes.is_empty() && removed_fields.is_empty() && restored_paths.is_empty() {
        return "No shape guard blocker is active. Still preserve existing Python classes and annotated fields."
            .to_string();
    }
    format!(
        "The previous mutation was restored because it removed Python API shape. Preserve these classes: [{}]. Preserve these fields: [{}]. Restored paths that need additive patching: [{}].",
        removed_classes.join(", "),
        removed_fields.join(", "),
        restored_paths.join(", ")
    )
}

fn runtime_lane_extend_json_string_array(out: &mut Vec<String>, value: Option<&Value>) {
    let Some(items) = value.and_then(Value::as_array) else {
        return;
    };
    for item in items.iter().filter_map(Value::as_str) {
        let item = item.trim();
        if !item.is_empty() && !out.iter().any(|existing| existing == item) {
            out.push(item.to_string());
        }
    }
}

fn runtime_lane_checkpointed_mutation_entry_call(
    mut call: NativeToolCall,
    workspace_root: &Path,
) -> Option<NativeToolCall> {
    let normalized_name = call.name.trim().to_ascii_lowercase();
    if !matches!(
        normalized_name.as_str(),
        "file_write" | "write_file" | "workspace.write" | "workspace_write" | "file_patch"
            | "patch_file" | "workspace.patch" | "workspace_patch"
    ) {
        return None;
    }
    call.name = if normalized_name.contains("patch") {
        "file_patch".to_string()
    } else {
        "file_write".to_string()
    };
    let Some(path) = runtime_lane_native_tool_call_path(&call) else {
        return None;
    };
    let raw_path = PathBuf::from(path);
    let full_path = if raw_path.is_absolute() {
        raw_path
    } else {
        workspace_root.join(raw_path)
    };
    if full_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
        || !full_path.starts_with(workspace_root)
    {
        return None;
    }
    if let Some(object) = call.args.as_object_mut() {
        object.insert("path".to_string(), json!(full_path.display().to_string()));
    }
    Some(call)
}

fn runtime_lane_native_tool_call_path(call: &NativeToolCall) -> Option<String> {
    ["path", "file_path", "target_path", "target"]
        .iter()
        .find_map(|key| call.args.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
}

fn runtime_lane_agent_run_native_tool_receipts(run: &AgentRunResult) -> Vec<NativeToolReceipt> {
    run.receipt
        .get("native_tool_receipts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value::<NativeToolReceipt>(item.clone()).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn runtime_lane_agent_run_has_no_successful_mutation(run: &AgentRunResult) -> bool {
    !runtime_lane_agent_run_native_tool_receipts(run)
        .iter()
        .any(runtime_lane_receipt_is_successful_mutation)
}

fn runtime_lane_agent_run_has_successful_tool_call(run: &AgentRunResult) -> bool {
    runtime_lane_agent_run_native_tool_receipts(run)
        .iter()
        .any(|receipt| receipt.status == "ok")
}

fn runtime_lane_agent_run_looks_like_provider_timeout(run: &AgentRunResult) -> bool {
    let text = format!(
        "{}\n{}",
        run.response.output,
        run.receipt
            .get("details")
            .map(Value::to_string)
            .unwrap_or_default()
    )
    .to_ascii_lowercase();
    runtime_lane_text_contains_any(
        &text,
        &[
            "provider_timeout",
            "ollama_run_timeout",
            "timeout_seconds",
            "partial progress",
        ],
    )
}

fn runtime_lane_append_agent_run_receipts(
    run: &mut AgentRunResult,
    receipts: &[NativeToolReceipt],
) {
    let Some(object) = run.receipt.as_object_mut() else {
        return;
    };
    let mut merged = object
        .get("native_tool_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    merged.extend(receipts.iter().cloned().map(|receipt| json!(receipt)));
    object.insert("native_tool_receipts".to_string(), json!(merged.clone()));
    object.insert("native_tool_call_count".to_string(), json!(merged.len()));
}

fn runtime_lane_append_synthetic_agent_receipt(
    run: &mut AgentRunResult,
    call_id: &str,
    status: &str,
    result: Value,
) {
    let receipt = NativeToolReceipt {
        call_id: call_id.to_string(),
        tool_name: "runtime_checkpointed_mutation_entry".to_string(),
        status: status.to_string(),
        duration_ms: 0,
        result,
        error: if status == "ok" {
            None
        } else {
            Some(status.to_string())
        },
    };
    runtime_lane_append_agent_run_receipts(run, &[receipt]);
}

fn runtime_lane_checkpointed_bootstrap_context_packet(prompt: &str) -> String {
    let Some(workspace_root) = runtime_lane_extract_workspace_root(prompt) else {
        return "workspace_root=unavailable; use file_read first to discover the local project."
            .to_string();
    };
    let context_pack = runtime_lane_model_manifest_context_pack(prompt, &workspace_root);
    let mut packet = String::new();
    packet.push_str(&format!(
        "workspace_root={}\n",
        workspace_root.display()
    ));
    if let Some(package) = runtime_lane_prompt_line_value(prompt, "Python package:") {
        packet.push_str(&format!("package={package}\n"));
    }
    if let Some(validation) = runtime_lane_prompt_line_value(prompt, "Validation command from project root:") {
        packet.push_str(&format!("validation_command={validation}\n"));
    }
    if let Some(prior_memory_row_id) = runtime_lane_prompt_line_value(prompt, "Prior memory row id:") {
        packet.push_str(&format!("prior_memory_row_id={prior_memory_row_id}\n"));
    }
    if let Some(expected_memory_row_id) =
        runtime_lane_prompt_line_value(prompt, "Expected new memory row id:")
    {
        packet.push_str(&format!("expected_new_memory_row_id={expected_memory_row_id}\n"));
    }
    packet.push_str("\nlocal_file_context:\n");
    runtime_lane_append_bounded_corpus(&mut packet, &context_pack, 12_000);
    packet
}

fn runtime_lane_checkpointed_first_tool_instruction(prompt: &str) -> String {
    if let Some(command) = runtime_lane_checkpointed_memory_get_command(prompt) {
        return format!(
            "call command_run from the project root to retrieve checkpoint memory: {command}"
        );
    }
    "call file_write or file_patch to begin the requested source/test/operator slice from the bootstrap context".to_string()
}

fn runtime_lane_checkpointed_memory_get_command(prompt: &str) -> Option<String> {
    let pattern = runtime_lane_prompt_line_value(prompt, "Memory CLI command pattern:")?;
    let prior_memory_row_id = runtime_lane_prompt_line_value(prompt, "Prior memory row id:")?;
    let command = format!(
        "checkpoint get --id={}",
        runtime_lane_shell_single_quote(&prior_memory_row_id)
    );
    if pattern.contains("<command>") {
        return Some(pattern.replace("<command>", &command));
    }
    Some(format!("{pattern} {command}"))
}

fn runtime_lane_prompt_line_value(prompt: &str, label: &str) -> Option<String> {
    prompt.lines().find_map(|line| {
        let trimmed = line.trim().trim_start_matches("- ").trim();
        trimmed
            .strip_prefix(label)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn runtime_lane_model_manifest_planner_system(preamble: Option<&str>) -> String {
    let prior = preamble
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n\nWorkflow context:\n{value}"))
        .unwrap_or_default();
    format!(
        "You are the Tier 2 model_manifest_planner for a primitive-first local coding runtime.\n\
Return only valid JSON. Do not use markdown. Do not include prose.\n\
Your JSON must be either {{\"deterministic_local_loop\":{{...}}}} or the inner manifest object.\n\
The manifest must include workspace_root and actions.\n\
Allowed actions are file_patch with path, old, new, and optional allow_multiple; write_file/create_file with path and content; plus optional validation with cmd.\n\
Use relative paths inside workspace_root. Do not use parent directory segments. Do not overwrite unrelated existing files. For existing files, prefer file_patch with exact old text from the local context pack when the edit is small and localized. Use write_file with overwrite=true only when creating a new file or replacing most of an existing file while preserving unrelated behavior.\n\
Keep this lane for bounded local coding tasks. If a local context pack is provided, treat it as the completed discovery step. Return structured_blocker only when the provided context is insufficient, the task needs architecture decisions, external packages, secrets, or user input.\n\
If the task provides a semantic probe or public API probe command, include it as a command_run action after validation. Validation passing without the provided semantic probe is not completion evidence.\n\
When the task names a public class/object plus companion behavior over that object's state, expose the behavior as an instance method on the class; a top-level wrapper may delegate to that method when useful.\n\
When the task names a result/decision object, prefer a small dataclass-like value object with attributes over an enum unless the task explicitly asks for enum constants. For override or resolution helpers, include fields for the primary decision value and provenance/source when the task names those concepts.\n\
When the task asks for a multi-file slice or names a public capability noun, create or update a semantically named module for that capability when the package layout allows it, and keep imports from that module working.\n\
When extending an existing package, preserve adjacent existing import surfaces: if a public helper belongs near an existing owner module, keep it importable from that owner module even when you also create a new semantic module.\n\
If a new public helper composes, wraps, or records calls to an existing public helper, define or re-export the new helper from the existing helper's owner module. A package __init__ re-export alone is not enough when the owner module is the natural public API surface.\n\
For parse/load/import helpers, honor the input shape described by the task. If raw CSV/header text is implied, parse headers and return plain records keyed by those headers unless the task explicitly asks for richer domain objects.\n\
For resolution/override helpers, higher-priority maps named by the task should take precedence, and provenance/source fields should preserve the most specific user-named source label when safe instead of generic labels such as override.\n\
Prefer the smallest complete manifest that satisfies the task; do not invent broad schemas, extra concepts, or extra validation beyond the local command and faithful regression tests.\n\
For durable/checkpoint/report tasks, the manifest must mutate the actual capability slice: model, store, service, operator surface when requested, tests, validation, and requested receipt or memory artifacts.\n\
Prefer small source plus faithful tests when tests are requested. Include validation only when a standard local command is obvious.\n\
For Python validation, use python3 commands such as python3 -m unittest rather than python.{prior}"
    )
}

fn runtime_lane_model_manifest_planner_system_compact(preamble: Option<&str>) -> String {
    let prior = preamble
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n\nWorkflow context:\n{value}"))
        .unwrap_or_default();
    format!(
        "You are a compact local coding manifest planner.\n\
Return only valid JSON. No markdown, no prose.\n\
Use this shape: {{\"deterministic_local_loop\":{{\"workspace_root\":\"/absolute/path\",\"actions\":[...]}}}}.\n\
Allowed actions: write_file with path/content/overwrite, file_patch with path/old/new/allow_multiple, command_run with cmd.\n\
Use the provided local context as authoritative. Preserve existing public imports and behavior.\n\
For existing files, prefer full corrected write_file when the file is small; otherwise use exact file_patch.\n\
When tests or regression evidence are requested, update or add focused tests.\n\
Always include the supplied validation command and semantic probe command after edits.\n\
For durable/checkpoint/report tasks, include real source/test mutations for the requested capability slice, not just manifests or docs.\n\
Return structured_blocker only for missing local context, secrets, external services, or user decisions.{prior}"
    )
}

fn runtime_lane_model_manifest_planner_prompt(
    prompt: &str,
    workspace_root: &Path,
    context_pack: &str,
) -> String {
    let context_section = if context_pack.trim().is_empty() {
        "Local context pack: none available. If existing project context is required, return structured_blocker with the missing context.".to_string()
    } else {
        format!(
            "Local context pack. Current files are authoritative. Use this instead of asking for discovery:\n{}",
            context_pack.trim()
        )
    };
    format!(
        "Workspace root: {}\n\nUser task:\n{}\n\n{}\n\nProduce the deterministic_local_loop JSON manifest now. For edits to existing files shown above, emit write_file actions with overwrite=true and full file content. Include focused regression tests when requested. Include a validation action when the command is clear from the task or context.",
        workspace_root.display(),
        prompt.trim(),
        context_section
    )
}

fn runtime_lane_model_manifest_planner_retry_prompt(
    prompt: &str,
    workspace_root: &Path,
    context_pack: &str,
    retry_kind: &str,
    failure_code: &str,
    failure_message: &str,
    needed_input: Option<&str>,
) -> String {
    let compact_context = runtime_lane_compact_context_pack(context_pack);
    format!(
        "Workspace root: {}\n\nUser task:\n{}\n\nPrevious planner attempt failed before completion.\nRetry kind: {}\nFailure code: {}\nFailure message: {}\nNeeded input: {}\n\nCompact local context pack:\n{}\n\nReturn only corrected deterministic_local_loop JSON. Use this exact outer shape: {{\"deterministic_local_loop\":{{\"workspace_root\":\"{}\",\"actions\":[...]}}}}. Prefer type=\"file_patch\" with path, old, and new for localized edits. Use type=\"write_file\" with path, content, and overwrite only for new files or broad replacements. Every validation action must include type=\"command_run\" and cmd. Keep the manifest minimal and complete.",
        workspace_root.display(),
        prompt.trim(),
        retry_kind,
        failure_code,
        failure_message,
        needed_input.unwrap_or("none"),
        compact_context,
        workspace_root.display()
    )
}

fn runtime_lane_model_manifest_semantic_repair_prompt(
    prompt: &str,
    workspace_root: &Path,
    context_pack: &str,
    failure: &RuntimeLaneSemanticProbeFailure,
) -> String {
    let compact_context = runtime_lane_compact_context_pack(context_pack);
    format!(
        "Workspace root: {}\n\nUser task:\n{}\n\nPost-validation semantic completion probe failed after mutation and validation.\nThis is a bounded repair, not a restart. Patch only the public API shape, placement, return type, or behavior needed by the probe while preserving existing tests and behavior.\n\nFailed command: {}\nExit code: {}\nStdout tail:\n{}\nStderr tail:\n{}\n\nCompact local context pack:\n{}\n\nReturn only corrected deterministic_local_loop JSON. Use this exact outer shape: {{\"deterministic_local_loop\":{{\"workspace_root\":\"{}\",\"actions\":[...]}}}}. Prefer the smallest file_patch source/test actions, rerun validation, and rerun the semantic probe command.",
        workspace_root.display(),
        prompt.trim(),
        failure.command,
        failure.exit_code
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        failure.stdout_tail,
        failure.stderr_tail,
        compact_context,
        workspace_root.display()
    )
}

fn runtime_lane_model_manifest_validation_repair_prompt(
    prompt: &str,
    workspace_root: &Path,
    context_pack: &str,
    failure_summary: &str,
) -> String {
    let compact_context = runtime_lane_compact_context_pack(context_pack);
    format!(
        "Workspace root: {}\n\nUser task:\n{}\n\nPost-mutation validation failed. This is a bounded repair of the current project state, not a restart.\nUse the validation failure as evidence. Patch the smallest source/test surface that caused the failure, preserve existing public behavior, and rerun validation.\nIf the traceback, tests, or semantic probe import a symbol from a module, make that module provide the symbol directly; do not rely only on package-level re-exports or unrelated sibling modules.\n\nValidation failure evidence:\n{}\n\nCurrent local context, including files mentioned by the failure when available:\n{}\n\nReturn only corrected deterministic_local_loop JSON. Use this exact outer shape: {{\"deterministic_local_loop\":{{\"workspace_root\":\"{}\",\"actions\":[...]}}}}. Prefer type=\"file_patch\" with path, old, and new for localized edits. If the failed file is small or patch context is uncertain, use type=\"write_file\" with full current corrected content and overwrite=true. Include a command_run action for the validation command. Do not edit validation harness files or semantic probe files unless the user explicitly asked to change them.",
        workspace_root.display(),
        prompt.trim(),
        failure_summary,
        compact_context,
        workspace_root.display()
    )
}

#[derive(Debug, Clone)]
struct RuntimeLanePublicApiBinding {
    module: String,
    symbols: Vec<String>,
    owner_path: PathBuf,
    source: &'static str,
}

fn runtime_lane_public_api_bindings(
    prompt: &str,
    context_pack: &str,
    workspace_root: &Path,
) -> Vec<RuntimeLanePublicApiBinding> {
    let mut bindings = Vec::<RuntimeLanePublicApiBinding>::new();
    runtime_lane_collect_python_public_api_bindings(
        prompt,
        workspace_root,
        "user_task",
        &mut bindings,
    );
    runtime_lane_collect_python_public_api_bindings(
        context_pack,
        workspace_root,
        "context_pack",
        &mut bindings,
    );
    for probe_text in runtime_lane_public_api_probe_texts(prompt, workspace_root) {
        runtime_lane_collect_python_public_api_bindings(
            &probe_text,
            workspace_root,
            "local_probe",
            &mut bindings,
        );
    }
    bindings
}

fn runtime_lane_apply_agent_loop_public_api_contract_prompt(
    metadata: &Value,
    prompt: String,
) -> String {
    if !runtime_lane_metadata_bool(
        metadata,
        &[
            "/native_success_criteria/public_api_extension_lane_enabled",
            "/workflow/native_success_criteria/public_api_extension_lane_enabled",
        ],
    )
    .unwrap_or(false)
    {
        return prompt;
    }
    let Some(workspace_root) = runtime_lane_extract_workspace_root(&prompt) else {
        return prompt;
    };
    let context_pack = runtime_lane_model_manifest_context_pack(&prompt, &workspace_root);
    let public_api_bindings =
        runtime_lane_public_api_bindings(&prompt, &context_pack, &workspace_root);
    if public_api_bindings.is_empty() {
        return prompt;
    }
    let public_api_section =
        runtime_lane_public_api_bindings_prompt_section(&public_api_bindings, &workspace_root);
    format!(
        "{}\n\nPublic API contract from local imports/probes:\n{}\nWhen a local validation or semantic probe imports a symbol that is missing from its owner module, satisfy that import in the owner module. Prefer additive edits that preserve existing public functions unless the task explicitly asks to change them.",
        prompt.trim(),
        public_api_section
    )
}

fn runtime_lane_public_api_probe_texts(prompt: &str, workspace_root: &Path) -> Vec<String> {
    let mut texts = Vec::<String>::new();
    for line in prompt.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("probe command") && !lower.contains("validation command") {
            continue;
        }
        for token in line.split_whitespace() {
            let token = token.trim_matches(|ch: char| {
                ch == '\''
                    || ch == '"'
                    || ch == '`'
                    || ch == ','
                    || ch == ';'
                    || ch == ':'
                    || ch == '('
                    || ch == ')'
            });
            if !token.ends_with(".py") {
                continue;
            }
            let candidate = if Path::new(token).is_absolute() {
                PathBuf::from(token)
            } else {
                workspace_root.join(token)
            };
            if !runtime_lane_path_is_under(workspace_root, &candidate) || !candidate.is_file() {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(candidate) {
                texts.push(text);
            }
        }
    }
    texts
}

fn runtime_lane_collect_python_public_api_bindings(
    text: &str,
    workspace_root: &Path,
    source: &'static str,
    bindings: &mut Vec<RuntimeLanePublicApiBinding>,
) {
    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        let import_start = line
            .strip_prefix("from ")
            .map(|_| 0)
            .or_else(|| line.find("from "));
        let Some(import_start) = import_start else {
            continue;
        };
        let import_clause = &line[import_start..];
        let Some(after_from) = import_clause.strip_prefix("from ") else {
            continue;
        };
        let Some((module, imports)) = after_from.split_once(" import ") else {
            continue;
        };
        let module = module.trim();
        if !runtime_lane_python_module_name_is_import_surface(module) {
            continue;
        }
        let Some(owner_path) = runtime_lane_python_module_owner_path(workspace_root, module) else {
            continue;
        };
        let symbols = runtime_lane_python_import_symbols(imports);
        if symbols.is_empty() {
            continue;
        }
        runtime_lane_add_public_api_binding(
            bindings,
            module.to_string(),
            symbols,
            owner_path,
            source,
        );
    }
}

fn runtime_lane_add_public_api_binding(
    bindings: &mut Vec<RuntimeLanePublicApiBinding>,
    module: String,
    symbols: Vec<String>,
    owner_path: PathBuf,
    source: &'static str,
) {
    if let Some(binding) = bindings.iter_mut().find(|binding| {
        binding.module == module && runtime_lane_paths_equal(&binding.owner_path, &owner_path)
    }) {
        for symbol in symbols {
            if !binding.symbols.iter().any(|existing| existing == &symbol) {
                binding.symbols.push(symbol);
            }
        }
        return;
    }
    bindings.push(RuntimeLanePublicApiBinding {
        module,
        symbols,
        owner_path,
        source,
    });
}

fn runtime_lane_python_module_name_is_import_surface(module: &str) -> bool {
    if module.is_empty() || module.starts_with('.') {
        return false;
    }
    module.split('.').all(runtime_lane_python_identifier_like)
}

fn runtime_lane_python_identifier_like(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn runtime_lane_python_import_symbols(imports: &str) -> Vec<String> {
    let normalized = imports
        .replace('(', " ")
        .replace(')', " ")
        .replace('\\', " ");
    let mut symbols = Vec::<String>::new();
    for item in normalized.split(',') {
        let symbol = item
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()));
        if symbol == "*" || !runtime_lane_python_identifier_like(symbol) {
            continue;
        }
        if !symbols.iter().any(|existing| existing == symbol) {
            symbols.push(symbol.to_string());
        }
    }
    symbols
}

fn runtime_lane_python_module_owner_path(workspace_root: &Path, module: &str) -> Option<PathBuf> {
    let relative_module = module.replace('.', "/");
    let file_candidate = format!("{relative_module}.py");
    let candidates = [
        workspace_root.join("src").join(&file_candidate),
        workspace_root.join(&file_candidate),
        workspace_root
            .join("src")
            .join(&relative_module)
            .join("__init__.py"),
        workspace_root.join(&relative_module).join("__init__.py"),
    ];
    candidates.iter().find(|path| path.is_file()).cloned()
}

fn runtime_lane_public_api_bindings_prompt_section(
    bindings: &[RuntimeLanePublicApiBinding],
    workspace_root: &Path,
) -> String {
    if bindings.is_empty() {
        return "Public API owner bindings: none detected from local import surfaces.".to_string();
    }
    let mut out = String::from(
        "Public API owner bindings detected from local import surfaces. Treat these as hard placement constraints:\n",
    );
    for binding in bindings {
        out.push_str(&format!(
            "- module {} owns symbol(s) {} in owner file {}. If you add/change these symbols, mutate this owner file directly; do not satisfy them only through a package __init__ or sibling module.\n",
            binding.module,
            binding.symbols.join(", "),
            runtime_lane_path_relative_display(workspace_root, &binding.owner_path)
        ));
    }
    out
}

fn runtime_lane_public_api_bindings_json(
    bindings: &[RuntimeLanePublicApiBinding],
    workspace_root: &Path,
) -> Value {
    Value::Array(
        bindings
            .iter()
            .map(|binding| {
                json!({
                    "module": binding.module.clone(),
                    "symbols": binding.symbols.clone(),
                    "owner_path": runtime_lane_path_relative_display(workspace_root, &binding.owner_path),
                    "source": binding.source,
                })
            })
            .collect(),
    )
}

fn runtime_lane_public_api_bindings_have_missing_owner_symbols(
    bindings: &[RuntimeLanePublicApiBinding],
) -> bool {
    bindings.iter().any(|binding| {
        let owner_content = std::fs::read_to_string(&binding.owner_path).unwrap_or_default();
        binding
            .symbols
            .iter()
            .any(|symbol| !runtime_lane_text_mentions_identifier(&owner_content, symbol))
    })
}

fn runtime_lane_public_api_manifest_violations(
    candidate: &DeterministicLocalLoopCandidate,
    bindings: &[RuntimeLanePublicApiBinding],
    workspace_root: &Path,
) -> Vec<String> {
    if bindings.is_empty() {
        return Vec::new();
    }
    let mut violations = Vec::<String>::new();
    for binding in bindings {
        let mut owner_mutated = false;
        let existing_owner_content =
            std::fs::read_to_string(&binding.owner_path).unwrap_or_default();
        let mut owner_candidate_content = String::new();
        let symbols_missing_from_existing_owner = binding
            .symbols
            .iter()
            .filter(|symbol| {
                !runtime_lane_text_mentions_identifier(&existing_owner_content, symbol)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut non_owner_symbol_targets = Vec::<String>::new();
        for action in &candidate.actions {
            let Some((target_path, content)) =
                runtime_lane_mutation_action_target_and_content(action)
            else {
                continue;
            };
            if !runtime_lane_python_source_path(target_path) {
                continue;
            }
            if runtime_lane_paths_equal(target_path, &binding.owner_path) {
                owner_mutated = true;
                owner_candidate_content.push('\n');
                owner_candidate_content.push_str(content);
                continue;
            }
            if symbols_missing_from_existing_owner
                .iter()
                .any(|symbol| runtime_lane_text_mentions_identifier(content, symbol))
            {
                non_owner_symbol_targets.push(runtime_lane_path_relative_display(
                    workspace_root,
                    target_path,
                ));
            }
        }
        if !owner_mutated && !non_owner_symbol_targets.is_empty() {
            violations.push(format!(
                "Symbols imported from {} ({}) were planned in non-owner file(s): {}. Owner file must be mutated: {}",
                binding.module,
                binding.symbols.join(", "),
                non_owner_symbol_targets.join(", "),
                runtime_lane_path_relative_display(workspace_root, &binding.owner_path)
            ));
        }
        let missing_owner_symbols = binding
            .symbols
            .iter()
            .filter(|symbol| {
                !runtime_lane_text_mentions_identifier(&existing_owner_content, symbol)
                    && !runtime_lane_text_mentions_identifier(&owner_candidate_content, symbol)
            })
            .cloned()
            .collect::<Vec<_>>();
        if owner_mutated && !missing_owner_symbols.is_empty() {
            violations.push(format!(
                "Owner file {} was mutated but did not provide imported symbol(s): {}",
                runtime_lane_path_relative_display(workspace_root, &binding.owner_path),
                missing_owner_symbols.join(", ")
            ));
        } else if !owner_mutated && !missing_owner_symbols.is_empty() {
            violations.push(format!(
                "Imported symbol(s) from {} are missing in owner file {} and the owner file was not mutated: {}",
                binding.module,
                runtime_lane_path_relative_display(workspace_root, &binding.owner_path),
                missing_owner_symbols.join(", ")
            ));
        }
    }
    violations
}

fn runtime_lane_public_api_owner_retarget_candidate(
    candidate: &DeterministicLocalLoopCandidate,
    bindings: &[RuntimeLanePublicApiBinding],
    workspace_root: &Path,
) -> Option<DeterministicLocalLoopCandidate> {
    if bindings.is_empty() {
        return None;
    }
    let mut retargeted = candidate.clone();
    let mut owner_actions = Vec::<DeterministicLocalAction>::new();
    for binding in bindings {
        if retargeted.actions.iter().any(|action| {
            runtime_lane_mutation_action_target_and_content(action)
                .map(|(target_path, _)| runtime_lane_paths_equal(target_path, &binding.owner_path))
                .unwrap_or(false)
        }) {
            continue;
        }
        let mut extracted_blocks = Vec::<String>::new();
        for action in &retargeted.actions {
            let Some((target_path, content)) =
                runtime_lane_mutation_action_target_and_content(action)
            else {
                continue;
            };
            if runtime_lane_paths_equal(target_path, &binding.owner_path)
                || !runtime_lane_python_source_path(target_path)
                || !runtime_lane_path_is_under(workspace_root, target_path)
            {
                continue;
            }
            if !binding
                .symbols
                .iter()
                .any(|symbol| runtime_lane_text_mentions_identifier(content, symbol))
            {
                continue;
            }
            extracted_blocks.extend(runtime_lane_extract_python_public_symbol_blocks(
                content,
                &binding.symbols,
            ));
        }
        if extracted_blocks.is_empty() {
            continue;
        }
        let Ok(owner_content) = std::fs::read_to_string(&binding.owner_path) else {
            continue;
        };
        let missing_blocks = extracted_blocks
            .into_iter()
            .filter(|block| {
                runtime_lane_python_block_declared_symbol(block)
                    .map(|symbol| !runtime_lane_text_mentions_identifier(&owner_content, symbol))
                    .unwrap_or(false)
            })
            .fold(Vec::<String>::new(), |mut blocks, block| {
                if !blocks.iter().any(|existing| existing == &block) {
                    blocks.push(block);
                }
                blocks
            });
        if missing_blocks.is_empty() {
            continue;
        }
        let mut new_owner_content = ensure_trailing_newline(owner_content);
        if !new_owner_content.ends_with("\n\n") {
            new_owner_content.push('\n');
        }
        new_owner_content.push_str(&missing_blocks.join("\n\n"));
        if !new_owner_content.ends_with('\n') {
            new_owner_content.push('\n');
        }
        owner_actions.push(DeterministicLocalAction::WriteFile {
            target_path: binding.owner_path.clone(),
            content: new_owner_content,
            overwrite: true,
        });
    }
    if owner_actions.is_empty() {
        return None;
    }
    let insert_index = retargeted
        .actions
        .iter()
        .position(|action| matches!(action, DeterministicLocalAction::CommandRun { .. }))
        .unwrap_or(retargeted.actions.len());
    for (offset, action) in owner_actions.into_iter().enumerate() {
        retargeted.actions.insert(insert_index + offset, action);
    }
    Some(retargeted)
}

fn runtime_lane_extract_python_public_symbol_blocks(
    content: &str,
    symbols: &[String],
) -> Vec<String> {
    let lines = content.lines().collect::<Vec<_>>();
    let import_preamble = runtime_lane_python_public_block_import_preamble(&lines);
    let mut blocks = Vec::<String>::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(symbol) = runtime_lane_python_top_level_declared_symbol(line) else {
            continue;
        };
        if !symbols.iter().any(|expected| expected == symbol) {
            continue;
        }
        let mut start = index;
        while start > 0 && lines[start - 1].trim_start().starts_with('@') {
            start -= 1;
        }
        let mut end = lines.len();
        for (next_index, next_line) in lines.iter().enumerate().skip(index + 1) {
            if runtime_lane_python_top_level_declared_symbol(next_line).is_some() {
                end = next_index;
                break;
            }
        }
        let body = lines[start..end].join("\n").trim().to_string();
        if body.is_empty() {
            continue;
        }
        let block = if import_preamble.is_empty() {
            body
        } else {
            format!("{import_preamble}\n\n{body}")
        };
        if !blocks.iter().any(|existing| existing == &block) {
            blocks.push(block);
        }
    }
    blocks
}

fn runtime_lane_python_public_block_import_preamble(lines: &[&str]) -> String {
    let mut imports = Vec::<String>::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("from __future__ ") {
            continue;
        }
        if (trimmed.starts_with("from ") || trimmed.starts_with("import "))
            && line
                .chars()
                .next()
                .map(|ch| !ch.is_whitespace())
                .unwrap_or(false)
            && !imports.iter().any(|existing| existing == trimmed)
        {
            imports.push(trimmed.to_string());
        }
    }
    imports.join("\n")
}

fn runtime_lane_python_top_level_declared_symbol(line: &str) -> Option<&str> {
    if line
        .chars()
        .next()
        .map(|ch| ch.is_whitespace())
        .unwrap_or(false)
    {
        return None;
    }
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("class ")
        .or_else(|| trimmed.strip_prefix("def "))?;
    let end = rest
        .find(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .unwrap_or(rest.len());
    let symbol = &rest[..end];
    if runtime_lane_python_identifier_like(symbol) {
        Some(symbol)
    } else {
        None
    }
}

fn runtime_lane_python_block_declared_symbol(block: &str) -> Option<&str> {
    block
        .lines()
        .find_map(runtime_lane_python_top_level_declared_symbol)
}

fn runtime_lane_path_is_under(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok()
}

fn runtime_lane_mutation_action_target_and_content(
    action: &DeterministicLocalAction,
) -> Option<(&Path, &str)> {
    match action {
        DeterministicLocalAction::WriteFile {
            target_path,
            content,
            ..
        } => Some((target_path.as_path(), content.as_str())),
        DeterministicLocalAction::PatchFile {
            target_path, new, ..
        } => Some((target_path.as_path(), new.as_str())),
        DeterministicLocalAction::CommandRun { .. } => None,
    }
}

fn runtime_lane_python_source_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("py")
}

fn runtime_lane_text_mentions_identifier(text: &str, identifier: &str) -> bool {
    if identifier.is_empty() {
        return false;
    }
    let mut search_from = 0usize;
    while let Some(offset) = text[search_from..].find(identifier) {
        let start = search_from + offset;
        let end = start + identifier.len();
        let before_ok = text[..start]
            .chars()
            .next_back()
            .map(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()))
            .unwrap_or(true);
        let after_ok = text[end..]
            .chars()
            .next()
            .map(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()))
            .unwrap_or(true);
        if before_ok && after_ok {
            return true;
        }
        search_from = end;
    }
    false
}

fn runtime_lane_paths_equal(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn runtime_lane_path_relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn runtime_lane_model_manifest_public_api_repair_prompt(
    prompt: &str,
    workspace_root: &Path,
    context_pack: &str,
    public_api_bindings: &[RuntimeLanePublicApiBinding],
    public_api_violations: &[String],
) -> String {
    let compact_context = runtime_lane_compact_context_pack(context_pack);
    let public_api_section =
        runtime_lane_public_api_bindings_prompt_section(public_api_bindings, workspace_root);
    format!(
        "Workspace root: {}\n\nUser task:\n{}\n\nThe previous manifest violated public API owner bindings before any mutation was allowed.\nViolations:\n{}\n\n{}\n\nCompact authoritative local context:\n{}\n\nReturn only corrected deterministic_local_loop JSON. Use this exact outer shape: {{\"deterministic_local_loop\":{{\"workspace_root\":\"{}\",\"actions\":[...]}}}}. Patch the owner file for imported public symbols. Keep the manifest minimal. Include validation and semantic probe commands when supplied by the task.",
        workspace_root.display(),
        prompt.trim(),
        public_api_violations.join("\n"),
        public_api_section,
        compact_context,
        workspace_root.display()
    )
}

fn runtime_lane_model_manifest_context_pack(prompt: &str, workspace_root: &Path) -> String {
    if !workspace_root.is_dir() {
        return String::new();
    }
    let lower_prompt = prompt.to_ascii_lowercase();
    let mut candidates = Vec::<RuntimeLaneContextCandidate>::new();
    runtime_lane_collect_context_candidates(
        workspace_root,
        workspace_root,
        0,
        &lower_prompt,
        &mut candidates,
    );
    candidates.sort_by(|left, right| {
        left.score
            .cmp(&right.score)
            .reverse()
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    let mut selected = Vec::<RuntimeLaneContextCandidate>::new();
    let mut total_bytes = 0usize;
    let max_files = 8usize;
    let max_total_bytes = 36_000usize;
    for candidate in candidates {
        if selected.len() >= max_files {
            break;
        }
        let Ok(content) = std::fs::read_to_string(&candidate.path) else {
            continue;
        };
        let content_len = content.len();
        if content_len > 12_000 || total_bytes.saturating_add(content_len) > max_total_bytes {
            continue;
        }
        total_bytes = total_bytes.saturating_add(content_len);
        selected.push(RuntimeLaneContextCandidate {
            content,
            ..candidate
        });
    }
    if selected.is_empty() {
        return String::new();
    }

    let validation_hint = runtime_lane_context_validation_hint(&selected);
    let mut out = String::new();
    out.push_str(&format!(
        "schema_version: context_pack_receipt_v1\nstatus: selected\nselected_files: {}\n",
        selected.len()
    ));
    if let Some(command) = validation_hint {
        out.push_str(&format!("likely_validation_command: {}\n", command));
    }
    for file in selected {
        out.push_str(&format!(
            "\n--- file: {} role: {} reason: {} ---\n",
            file.relative_path, file.role, file.reason
        ));
        out.push_str(&file.content);
        if !file.content.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn runtime_lane_model_manifest_context_pack_with_failure(
    prompt: &str,
    workspace_root: &Path,
    failure_summary: &str,
) -> String {
    let mut out = runtime_lane_model_manifest_context_pack(prompt, workspace_root);
    let mut appended = 0usize;
    for relative_path in runtime_lane_paths_mentioned_by_failure(workspace_root, failure_summary) {
        if appended >= 6 {
            break;
        }
        let marker = format!("--- file: {}", relative_path);
        let failed_marker = format!("--- failed file: {}", relative_path);
        if out.contains(&marker) || out.contains(&failed_marker) {
            continue;
        }
        let path = workspace_root.join(&relative_path);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if content.len() > 12_000 {
            continue;
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!(
            "\n--- failed file: {} reason: validation_failure_traceback ---\n",
            relative_path
        ));
        out.push_str(&content);
        if !content.ends_with('\n') {
            out.push('\n');
        }
        appended += 1;
    }
    out
}

fn runtime_lane_paths_mentioned_by_failure(
    workspace_root: &Path,
    failure_summary: &str,
) -> Vec<String> {
    let root = workspace_root.display().to_string();
    let mut paths = Vec::<String>::new();
    for line in failure_summary.lines() {
        let mut search_from = 0usize;
        while let Some(index) = line[search_from..].find(&root) {
            let absolute_start = search_from + index;
            let tail = &line[absolute_start + root.len()..];
            if let Some(relative) = runtime_lane_failure_tail_to_relative_path(tail) {
                if !paths.iter().any(|path| path == &relative) {
                    paths.push(relative);
                }
            }
            search_from = absolute_start + root.len();
        }
    }
    paths
}

fn runtime_lane_failure_tail_to_relative_path(tail: &str) -> Option<String> {
    let tail = tail.strip_prefix('/')?;
    let extensions = [
        ".py", ".rs", ".ts", ".tsx", ".js", ".jsx", ".json", ".toml", ".yaml", ".yml", ".md",
    ];
    let end = extensions
        .iter()
        .filter_map(|extension| tail.find(extension).map(|index| index + extension.len()))
        .min()?;
    let candidate = &tail[..end];
    if candidate.contains("..") || candidate.starts_with('/') || candidate.trim().is_empty() {
        return None;
    }
    Some(candidate.to_string())
}

fn runtime_lane_compact_context_pack(context_pack: &str) -> String {
    let trimmed = context_pack.trim();
    if trimmed.len() <= 18_000 {
        return trimmed.to_string();
    }
    let mut out = String::new();
    for line in trimmed.lines() {
        if out.len() >= 18_000 {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn runtime_lane_context_pack_relative_paths(context_pack: &str) -> Vec<String> {
    context_pack
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("--- file: ")?;
            let path = rest.split(" role: ").next()?.trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect()
}

fn runtime_lane_first_edit_actions_within_context(
    actions: &[DeterministicLocalAction],
    workspace_root: &Path,
    context_pack: &str,
) -> bool {
    let allowed_paths = runtime_lane_context_pack_relative_paths(context_pack);
    if allowed_paths.is_empty() {
        return false;
    }
    let allowed_dirs = allowed_paths
        .iter()
        .filter_map(|path| Path::new(path).parent())
        .filter_map(Path::to_str)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    actions.iter().all(|action| {
        let target_path = match action {
            DeterministicLocalAction::WriteFile { target_path, .. }
            | DeterministicLocalAction::PatchFile { target_path, .. } => target_path,
            DeterministicLocalAction::CommandRun { .. } => return true,
        };
        if !runtime_lane_path_is_under(workspace_root, target_path) {
            return false;
        }
        let relative_path = runtime_lane_path_relative_display(workspace_root, target_path);
        if allowed_paths.iter().any(|path| path == &relative_path) {
            return true;
        }
        !target_path.exists()
            && runtime_lane_first_edit_relative_path_is_safe_mutation_target(&relative_path)
            && allowed_dirs
                .iter()
                .any(|dir| relative_path.strip_prefix(dir).is_some_and(|rest| rest.starts_with('/')))
    })
}

fn runtime_lane_first_edit_contextual_target_paths(
    target_paths: &[PathBuf],
    workspace_root: &Path,
    context_pack: &str,
) -> Vec<PathBuf> {
    let allowed_paths = runtime_lane_context_pack_relative_paths(context_pack);
    if allowed_paths.is_empty() {
        return Vec::new();
    }
    let allowed_dirs = allowed_paths
        .iter()
        .filter_map(|path| Path::new(path).parent())
        .filter_map(Path::to_str)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    target_paths
        .iter()
        .filter(|target_path| {
            if !runtime_lane_path_is_under(workspace_root, target_path) {
                return false;
            }
            let relative_path = runtime_lane_path_relative_display(workspace_root, target_path);
            if allowed_paths.iter().any(|path| path == &relative_path) {
                return true;
            }
            !target_path.exists()
                && runtime_lane_first_edit_relative_path_is_safe_mutation_target(&relative_path)
                && allowed_dirs.iter().any(|dir| {
                    relative_path
                        .strip_prefix(dir)
                        .is_some_and(|rest| rest.starts_with('/'))
                })
        })
        .cloned()
        .collect()
}

fn runtime_lane_first_edit_relative_path_is_safe_mutation_target(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    !path.contains("..")
        && !lower.starts_with(".infring/")
        && !lower.contains("semantic_probe")
        && !lower.contains("validation")
}

#[derive(Debug, Clone)]
struct RuntimeLaneSemanticProbeCommand {
    cwd: PathBuf,
    cmd: Vec<String>,
}

#[derive(Debug, Clone)]
struct RuntimeLaneSemanticProbeFailure {
    command: String,
    exit_code: Option<i64>,
    stdout_tail: String,
    stderr_tail: String,
}

fn runtime_lane_extract_semantic_probe_command(
    prompt: &str,
    workspace_root: &Path,
) -> Option<RuntimeLaneSemanticProbeCommand> {
    let marker = "semantic probe command from project root";
    let line = prompt
        .lines()
        .find(|line| line.to_ascii_lowercase().contains(marker))?;
    let command = runtime_lane_clean_inline_shell_command(
        runtime_lane_inline_shell_command_after_marker(line, marker)?,
    );
    if command.is_empty() {
        return None;
    }
    Some(RuntimeLaneSemanticProbeCommand {
        cwd: workspace_root.to_path_buf(),
        cmd: vec![
            "sh".to_string(),
            "-c".to_string(),
            runtime_lane_normalize_shell_command(command),
        ],
    })
}

fn runtime_lane_extract_or_default_semantic_probe_command(
    prompt: &str,
    workspace_root: &Path,
) -> Option<RuntimeLaneSemanticProbeCommand> {
    runtime_lane_extract_semantic_probe_command(prompt, workspace_root).or_else(|| {
        let probe_path = workspace_root.join(".infring").join("semantic_probe.py");
        if !probe_path.is_file() {
            return None;
        }
        Some(RuntimeLaneSemanticProbeCommand {
            cwd: workspace_root.to_path_buf(),
            cmd: vec![
                "sh".to_string(),
                "-lc".to_string(),
                "PYTHONPATH=. python3 .infring/semantic_probe.py".to_string(),
            ],
        })
    })
}

fn runtime_lane_extract_validation_command(
    prompt: &str,
    workspace_root: &Path,
) -> Option<RuntimeLaneSemanticProbeCommand> {
    let marker = "validation command from project root";
    let line = prompt
        .lines()
        .find(|line| line.to_ascii_lowercase().contains(marker))?;
    let command = runtime_lane_clean_inline_shell_command(
        runtime_lane_inline_shell_command_after_marker(line, marker)?,
    );
    if command.is_empty() {
        return None;
    }
    Some(RuntimeLaneSemanticProbeCommand {
        cwd: workspace_root.to_path_buf(),
        cmd: vec![
            "sh".to_string(),
            "-lc".to_string(),
            runtime_lane_normalize_shell_command(command),
        ],
    })
}

fn runtime_lane_inline_shell_command_after_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    let marker_index = lower.find(marker)?;
    let after_marker = &line[marker_index + marker.len()..];
    let command = after_marker
        .trim_start()
        .trim_start_matches(|ch| matches!(ch, ':' | '-' | '>' | ' ' | '\t'));
    if command.trim().is_empty() {
        None
    } else {
        Some(command)
    }
}

fn runtime_lane_clean_inline_shell_command(value: &str) -> &str {
    let trimmed = value.trim();
    let mut end = trimmed.len();
    for marker in [
        ". Then ",
        " Then ",
        ". Do not ",
        " Do not ",
        ". Please ",
        " Please ",
        ", then ",
        ", and then ",
        " and then ",
    ] {
        if let Some(index) = trimmed.find(marker) {
            end = end.min(index);
        }
    }
    trimmed[..end].trim_end_matches('.').trim()
}

fn runtime_lane_attach_semantic_probe_action(
    candidate: &mut DeterministicLocalLoopCandidate,
    semantic_probe: Option<&RuntimeLaneSemanticProbeCommand>,
    call_prefix: &str,
    semantic_probe_call_id: &mut Option<String>,
) {
    let Some(semantic_probe) = semantic_probe else {
        return;
    };
    let existing_index = candidate.actions.iter().position(|action| {
        matches!(
            action,
            DeterministicLocalAction::CommandRun { cwd, cmd, .. }
                if cwd == &semantic_probe.cwd
                    && runtime_lane_commands_equivalent(cmd, &semantic_probe.cmd)
        )
    });
    let index = match existing_index {
        Some(index) => index,
        None => {
            candidate
                .actions
                .push(DeterministicLocalAction::CommandRun {
                    cwd: semantic_probe.cwd.clone(),
                    cmd: semantic_probe.cmd.clone(),
                    timeout_seconds: 30,
                    max_output_bytes: 12000,
                });
            candidate.requires_validation = true;
            candidate.actions.len() - 1
        }
    };
    *semantic_probe_call_id = Some(format!("{call_prefix}_{}", index + 1));
}

fn runtime_lane_append_explicit_validation_recheck(
    receipts: &mut Vec<NativeToolReceipt>,
    workspace_root: &Path,
    validation_command: Option<&RuntimeLaneSemanticProbeCommand>,
    semantic_probe: Option<&RuntimeLaneSemanticProbeCommand>,
    call_prefix: &str,
) {
    if validation_command.is_none() && semantic_probe.is_none() {
        return;
    }
    let mut recheck_candidate = DeterministicLocalLoopCandidate {
        workspace_root: workspace_root.to_path_buf(),
        actions: Vec::new(),
        requires_validation: true,
    };
    let mut validation_call_id = None;
    runtime_lane_attach_semantic_probe_action(
        &mut recheck_candidate,
        validation_command,
        call_prefix,
        &mut validation_call_id,
    );
    let mut semantic_probe_call_id = None;
    runtime_lane_attach_semantic_probe_action(
        &mut recheck_candidate,
        semantic_probe,
        call_prefix,
        &mut semantic_probe_call_id,
    );
    if recheck_candidate.actions.is_empty() {
        return;
    }
    let mut recheck_receipts =
        runtime_lane_dispatch_model_manifest_actions(&recheck_candidate, call_prefix);
    receipts.append(&mut recheck_receipts);
}

fn runtime_lane_commands_equivalent(left: &[String], right: &[String]) -> bool {
    if left == right {
        return true;
    }
    match (
        runtime_lane_shell_command_payload(left),
        runtime_lane_shell_command_payload(right),
    ) {
        (Some(left), Some(right)) => left.trim() == right.trim(),
        _ => false,
    }
}

fn runtime_lane_shell_command_payload(command: &[String]) -> Option<&str> {
    if command.len() < 3 {
        return None;
    }
    let shell = command.first()?.as_str();
    let flag = command.get(1)?.as_str();
    if !matches!(shell, "sh" | "bash") || !matches!(flag, "-c" | "-lc") {
        return None;
    }
    command.get(2).map(String::as_str)
}

fn runtime_lane_normalize_shell_command_vector(mut cmd: Vec<String>) -> Vec<String> {
    if cmd.len() >= 3
        && matches!(cmd[0].trim(), "sh" | "bash")
        && matches!(cmd[1].trim(), "-c" | "-lc")
    {
        cmd[2] = runtime_lane_normalize_shell_command(&cmd[2]);
    }
    cmd
}

fn runtime_lane_dispatch_model_manifest_actions(
    candidate: &DeterministicLocalLoopCandidate,
    call_prefix: &str,
) -> Vec<NativeToolReceipt> {
    let dispatcher = NativeToolDispatcher::new(&[
        "file_write".to_string(),
        "file_patch".to_string(),
        "command_run".to_string(),
    ]);
    let mut receipts = Vec::<NativeToolReceipt>::new();
    let ordered_actions = runtime_lane_manifest_actions_mutation_before_command(&candidate.actions);
    for (index, action) in ordered_actions.into_iter().enumerate() {
        let call = match action {
            DeterministicLocalAction::WriteFile {
                target_path,
                content,
                overwrite,
            } => NativeToolCall {
                id: format!("{call_prefix}_{}", index + 1),
                name: "file_write".to_string(),
                args: json!({
                    "path": target_path.display().to_string(),
                    "content": content,
                    "overwrite": overwrite,
                    "model_manifest_planner": true,
                }),
            },
            DeterministicLocalAction::PatchFile {
                target_path,
                old,
                new,
                allow_multiple,
            } => NativeToolCall {
                id: format!("{call_prefix}_{}", index + 1),
                name: "file_patch".to_string(),
                args: json!({
                    "path": target_path.display().to_string(),
                    "old": old,
                    "new": new,
                    "allow_multiple": allow_multiple,
                    "model_manifest_planner": true,
                }),
            },
            DeterministicLocalAction::CommandRun {
                cwd,
                cmd,
                timeout_seconds,
                max_output_bytes,
            } => {
                let cmd = runtime_lane_normalize_shell_command_vector(cmd.clone());
                NativeToolCall {
                    id: format!("{call_prefix}_{}", index + 1),
                    name: "command_run".to_string(),
                    args: json!({
                        "cwd": cwd.display().to_string(),
                        "cmd": cmd,
                        "timeout_seconds": timeout_seconds,
                        "max_output_bytes": max_output_bytes,
                        "model_manifest_planner": true,
                    }),
                }
            }
        };
        let receipt = dispatcher.dispatch(call);
        let should_stop = receipt.status != "ok"
            || (receipt.tool_name == "command_run"
                && !receipt
                    .result
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(true));
        receipts.push(receipt);
        if should_stop {
            break;
        }
    }
    receipts
}

fn runtime_lane_manifest_actions_mutation_before_command(
    actions: &[DeterministicLocalAction],
) -> Vec<&DeterministicLocalAction> {
    let mut ordered = actions
        .iter()
        .filter(|action| runtime_lane_manifest_action_is_mutation(action))
        .collect::<Vec<_>>();
    ordered.extend(
        actions
            .iter()
            .filter(|action| !runtime_lane_manifest_action_is_mutation(action)),
    );
    ordered
}

fn runtime_lane_manifest_action_is_mutation(action: &DeterministicLocalAction) -> bool {
    matches!(
        action,
        DeterministicLocalAction::WriteFile { .. } | DeterministicLocalAction::PatchFile { .. }
    )
}

fn runtime_lane_semantic_probe_failure(
    receipts: &[NativeToolReceipt],
    semantic_probe_call_id: Option<&str>,
) -> Option<RuntimeLaneSemanticProbeFailure> {
    let receipt = semantic_probe_call_id
        .and_then(|call_id| receipts.iter().find(|receipt| receipt.call_id == call_id))
        .or_else(|| {
            receipts.iter().rev().find(|receipt| {
                receipt.tool_name == "command_run"
                    && (receipt.status != "ok"
                        || !receipt
                            .result
                            .get("success")
                            .and_then(Value::as_bool)
                            .unwrap_or(true))
            })
        })?;
    let success = receipt.status == "ok"
        && receipt
            .result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(true);
    if success {
        return None;
    }
    let command = receipt
        .result
        .get("cmd")
        .map(Value::to_string)
        .unwrap_or_else(|| receipt.call_id.clone());
    Some(RuntimeLaneSemanticProbeFailure {
        command,
        exit_code: receipt.result.get("exit_code").and_then(Value::as_i64),
        stdout_tail: receipt
            .result
            .get("stdout")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .rev()
            .take(2000)
            .collect::<String>()
            .chars()
            .rev()
            .collect(),
        stderr_tail: receipt
            .result
            .get("stderr")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .rev()
            .take(2000)
            .collect::<String>()
            .chars()
            .rev()
            .collect(),
    })
}

fn runtime_lane_semantic_repair_failure_receipt(
    call_id: &str,
    failure_code: &str,
    failure_message: &str,
    details: Value,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call_id.to_string(),
        tool_name: "semantic_completion_repair".to_string(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "kind": "post_validation_semantic_completion_repair_artifact_v1",
            "failure_code": failure_code,
            "failure_message": failure_message,
            "success_claim_allowed": false,
            "details": details,
        }),
        error: Some(failure_message.to_string()),
    }
}

fn runtime_lane_validation_repair_failure_receipt(
    call_id: &str,
    failure_code: &str,
    failure_message: &str,
    details: Value,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call_id.to_string(),
        tool_name: "validation_repair".to_string(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "kind": "post_validation_repair_artifact_v1",
            "failure_code": failure_code,
            "failure_message": failure_message,
            "success_claim_allowed": false,
            "details": details,
        }),
        error: Some(failure_message.to_string()),
    }
}

fn runtime_lane_receipt_is_mutation_tool(receipt: &NativeToolReceipt) -> bool {
    matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
}

fn runtime_lane_receipt_is_successful_mutation(receipt: &NativeToolReceipt) -> bool {
    receipt.status == "ok" && runtime_lane_receipt_is_mutation_tool(receipt)
}

fn runtime_lane_changed_file_summary_item(receipt: &NativeToolReceipt) -> Option<Value> {
    if !runtime_lane_receipt_is_successful_mutation(receipt) {
        return None;
    }
    let operation = match receipt.tool_name.as_str() {
        "file_patch" => "patched",
        "file_write" => {
            if receipt
                .result
                .get("created")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "created"
            } else {
                "written"
            }
        }
        _ => return None,
    };
    Some(json!({
        "path": receipt.result.get("path")?.as_str()?,
        "operation": operation,
        "receipt_ref": receipt.call_id,
    }))
}

fn runtime_lane_first_receipt_failure_summary(receipts: &[NativeToolReceipt]) -> Option<String> {
    receipts.iter().find_map(|receipt| {
        if receipt.status != "ok" {
            return Some(format!(
                "tool={} call_id={} status={} error={}",
                receipt.tool_name,
                receipt.call_id,
                receipt.status,
                receipt
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            ));
        }
        if receipt.tool_name == "command_run"
            && !receipt
                .result
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        {
            return Some(format!(
                "tool=command_run call_id={} exit_code={} stderr_tail={}",
                receipt.call_id,
                receipt
                    .result
                    .get("exit_code")
                    .and_then(Value::as_i64)
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                runtime_lane_tail(
                    receipt
                        .result
                        .get("stderr")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    800,
                )
            ));
        }
        None
    })
}

fn runtime_lane_receipts_need_repair(
    receipts: &[NativeToolReceipt],
    requires_validation: bool,
) -> bool {
    if receipts.iter().any(|receipt| receipt.status != "ok") {
        return true;
    }
    if requires_validation {
        return !runtime_lane_receipts_validation_ok(receipts, requires_validation);
    }
    receipts.iter().any(|receipt| {
        receipt.tool_name == "command_run" && !runtime_lane_command_receipt_success(receipt)
    })
}

fn runtime_lane_receipts_validation_ok(
    receipts: &[NativeToolReceipt],
    requires_validation: bool,
) -> bool {
    if !requires_validation {
        return true;
    }
    let start_index = receipts
        .iter()
        .rposition(runtime_lane_receipt_is_successful_mutation)
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut saw_command = false;
    let mut latest_by_command = BTreeMap::<String, bool>::new();
    let mut latest_validation_command = None;
    let mut latest_semantic_probe_command = None;
    for receipt in receipts
        .iter()
        .skip(start_index)
        .filter(|receipt| receipt.tool_name == "command_run")
    {
        saw_command = true;
        let success = runtime_lane_command_receipt_success(receipt);
        match runtime_lane_command_receipt_validation_category(receipt) {
            Some("semantic_probe") => latest_semantic_probe_command = Some(success),
            Some("validation") => latest_validation_command = Some(success),
            _ => {}
        }
        latest_by_command.insert(
            runtime_lane_command_receipt_key(receipt),
            success,
        );
    }
    if latest_validation_command.is_some() || latest_semantic_probe_command.is_some() {
        return latest_validation_command.unwrap_or(true)
            && latest_semantic_probe_command.unwrap_or(true);
    }
    saw_command && latest_by_command.values().all(|success| *success)
}

fn runtime_lane_command_receipt_success(receipt: &NativeToolReceipt) -> bool {
    receipt.status == "ok"
        && receipt
            .result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(true)
}

fn runtime_lane_command_receipt_key(receipt: &NativeToolReceipt) -> String {
    receipt
        .result
        .get("cmd")
        .map(Value::to_string)
        .unwrap_or_else(|| receipt.call_id.clone())
}

fn runtime_lane_post_mutation_command_receipts_all_successful(
    receipts: &[NativeToolReceipt],
) -> bool {
    let start_index = receipts
        .iter()
        .rposition(runtime_lane_receipt_is_successful_mutation)
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut saw_command = false;
    for receipt in receipts
        .iter()
        .skip(start_index)
        .filter(|receipt| receipt.tool_name == "command_run")
    {
        saw_command = true;
        if !runtime_lane_command_receipt_success(receipt) {
            return false;
        }
    }
    saw_command
}

fn runtime_lane_command_receipt_validation_category(
    receipt: &NativeToolReceipt,
) -> Option<&'static str> {
    let command_text = receipt
        .result
        .get("cmd")
        .map(Value::to_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if command_text.contains("semantic_probe") {
        return Some("semantic_probe");
    }
    if command_text.contains("pytest")
        || command_text.contains("unittest")
        || command_text.contains("cargo test")
        || command_text.contains("npm test")
        || command_text.contains("pnpm test")
        || command_text.contains("yarn test")
        || command_text.contains("go test")
        || command_text.contains("swift test")
        || command_text.contains("mix test")
        || command_text.contains("dotnet test")
    {
        return Some("validation");
    }
    None
}

fn runtime_lane_tail(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}

fn runtime_lane_relabel_generated_manifest_response(
    response: &mut RuntimeLaneResponse,
    lane: &str,
    failure_status: &str,
    lane_reason: &str,
) {
    response.output = response.output.replace("model_manifest_planner", lane);
    if !response.ok {
        response.error = Some(failure_status.to_string());
    }
    if let Some(receipt) = response.receipt.as_object_mut() {
        receipt.insert("lane".to_string(), json!(lane));
        receipt.insert("lane_reason".to_string(), json!(lane_reason));
        if !response.ok {
            receipt.insert("status".to_string(), json!(failure_status));
        }
    }
    if let Some(contract) = response.contract.as_object_mut() {
        contract.insert(
            "agent_status".to_string(),
            json!(if response.ok { "ok" } else { failure_status }),
        );
        if let Some(execution_shape) = contract
            .get_mut("execution_shape")
            .and_then(Value::as_object_mut)
        {
            execution_shape.insert("lane".to_string(), json!(lane));
            execution_shape.insert(
                "escalation_reason".to_string(),
                json!("bounded_existing_project_fast_lane_selected"),
            );
        }
    }
    if let Some(trace) = response.trace_summary.as_object_mut() {
        trace.insert(
            "status".to_string(),
            json!(if response.ok { "ok" } else { failure_status }),
        );
        trace.insert("lane".to_string(), json!(lane));
    }
}

fn runtime_lane_metadata_string(metadata: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .filter_map(|pointer| metadata.pointer(pointer))
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .next()
}

fn runtime_lane_metadata_bool(metadata: &Value, pointers: &[&str]) -> Option<bool> {
    pointers
        .iter()
        .filter_map(|pointer| metadata.pointer(pointer))
        .filter_map(Value::as_bool)
        .next()
}

fn runtime_lane_provider_error_is_timeout(code: &str, message: &str) -> bool {
    let code = code.to_ascii_lowercase();
    let message = message.to_ascii_lowercase();
    code.contains("timeout") || message.contains("timeout")
}

#[derive(Debug, Clone)]
struct RuntimeLaneModelManifestFailure {
    failure_code: String,
    failure_message: String,
    needed_input: Option<String>,
    provider_output_preview: String,
}

fn runtime_lane_model_manifest_candidate_from_output(
    output: &str,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> Result<DeterministicLocalLoopCandidate, RuntimeLaneModelManifestFailure> {
    let Some(manifest) = runtime_lane_parse_deterministic_manifest_from_text(output) else {
        return Err(RuntimeLaneModelManifestFailure {
            failure_code: "manifest_parse_failed".to_string(),
            failure_message: "The model must return only JSON with deterministic_local_loop.workspace_root and actions.".to_string(),
            needed_input: Some("Return deterministic_local_loop JSON only.".to_string()),
            provider_output_preview: output.chars().take(1600).collect::<String>(),
        });
    };
    let mut manifest = manifest;
    runtime_lane_normalize_model_manifest_for_execution(&mut manifest);
    let manifest_prompt = format!(
        "```json\n{}\n```",
        serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".to_string())
    );
    match runtime_lane_deterministic_local_loop_candidate(
        &manifest_prompt,
        tools,
        capability_packs,
        permissions,
    ) {
        DeterministicLocalLoopGate::Candidate(candidate) => Ok(candidate),
        DeterministicLocalLoopGate::Blocked {
            failure_code,
            failure_message,
            needed_input,
        } => Err(RuntimeLaneModelManifestFailure {
            failure_code: failure_code.to_string(),
            failure_message,
            needed_input,
            provider_output_preview: output.chars().take(1600).collect::<String>(),
        }),
        DeterministicLocalLoopGate::NotCandidate => Err(RuntimeLaneModelManifestFailure {
            failure_code: "manifest_not_candidate".to_string(),
            failure_message:
                "The returned JSON was not accepted as a deterministic local action manifest."
                    .to_string(),
            needed_input: Some(
                "Return deterministic_local_loop JSON with workspace_root and write actions."
                    .to_string(),
            ),
            provider_output_preview: output.chars().take(1600).collect::<String>(),
        }),
    }
}

fn runtime_lane_normalize_model_manifest_for_execution(manifest: &mut Value) {
    let manifest_root = if manifest.get("deterministic_local_loop").is_some() {
        manifest
            .get_mut("deterministic_local_loop")
            .expect("deterministic_local_loop checked")
    } else {
        manifest
    };
    let actions = if manifest_root.get("actions").is_some() {
        manifest_root.get_mut("actions")
    } else {
        manifest_root.get_mut("files")
    }
    .and_then(Value::as_array_mut);
    let Some(actions) = actions else {
        return;
    };
    for action in actions {
        runtime_lane_normalize_model_manifest_action(action);
    }
}

fn runtime_lane_normalize_model_manifest_action(action: &mut Value) {
    let Some(object) = action.as_object_mut() else {
        return;
    };
    for key in ["write_file", "file_write", "write"] {
        if let Some(payload) = object.get_mut(key).and_then(Value::as_object_mut) {
            payload
                .entry("overwrite".to_string())
                .or_insert(json!(true));
            return;
        }
    }
    let kind = object
        .get("type")
        .or_else(|| object.get("kind"))
        .or_else(|| object.get("action"))
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    let is_write = match kind.as_deref() {
        Some("write_file" | "file_write" | "write") => true,
        Some("create_file") => false,
        Some(_) => false,
        None => {
            object.contains_key("content")
                && !object.contains_key("cmd")
                && !object.contains_key("command")
        }
    };
    if is_write {
        object.entry("overwrite".to_string()).or_insert(json!(true));
    }
}

#[derive(Debug, Clone)]
struct RuntimeLaneContextCandidate {
    path: PathBuf,
    relative_path: String,
    role: &'static str,
    reason: &'static str,
    score: i32,
    content: String,
}

fn runtime_lane_collect_context_candidates(
    root: &Path,
    dir: &Path,
    depth: usize,
    lower_prompt: &str,
    candidates: &mut Vec<RuntimeLaneContextCandidate>,
) {
    if depth > 4 || candidates.len() > 64 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if runtime_lane_context_path_is_ignored(&name) {
            continue;
        }
        if path.is_dir() {
            runtime_lane_collect_context_candidates(
                root,
                &path,
                depth + 1,
                lower_prompt,
                candidates,
            );
            continue;
        }
        if !runtime_lane_context_file_is_supported(&path) {
            continue;
        }
        let relative_path = path
            .strip_prefix(root)
            .ok()
            .and_then(|path| path.to_str())
            .unwrap_or_default()
            .replace('\\', "/");
        if relative_path.is_empty() {
            continue;
        }
        let (role, reason, mut score) = runtime_lane_context_file_role(&relative_path);
        score += runtime_lane_context_prompt_score(&relative_path, lower_prompt);
        candidates.push(RuntimeLaneContextCandidate {
            path,
            relative_path,
            role,
            reason,
            score,
            content: String::new(),
        });
    }
}

fn runtime_lane_context_path_is_ignored(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".infring"
            | "__pycache__"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | ".venv"
            | "venv"
    )
}

fn runtime_lane_context_file_is_supported(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if matches!(
        name,
        "Cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "setup.py"
            | "go.mod"
            | "requirements.txt"
            | "README.md"
            | "README"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some(
            "py" | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "rs"
                | "go"
                | "java"
                | "rb"
                | "php"
                | "cs"
                | "swift"
                | "kt"
                | "toml"
                | "json"
                | "yaml"
                | "yml"
                | "md"
        )
    )
}

fn runtime_lane_context_file_role(relative_path: &str) -> (&'static str, &'static str, i32) {
    let lower = relative_path.to_ascii_lowercase();
    if lower.contains("/test")
        || lower.starts_with("test")
        || lower.contains("_test.")
        || lower.contains(".test.")
        || lower.contains(".spec.")
    {
        return (
            "test",
            "existing tests define expected behavior and style",
            90,
        );
    }
    if lower.starts_with("src/")
        || lower.contains("/src/")
        || matches!(
            Path::new(relative_path)
                .extension()
                .and_then(|value| value.to_str()),
            Some("py" | "js" | "ts" | "tsx" | "jsx" | "rs" | "go" | "java" | "rb" | "php" | "cs")
        )
    {
        return ("source", "existing source owns adjacent behavior", 80);
    }
    if matches!(
        Path::new(relative_path)
            .file_name()
            .and_then(|value| value.to_str()),
        Some(
            "Cargo.toml"
                | "package.json"
                | "pyproject.toml"
                | "setup.py"
                | "go.mod"
                | "requirements.txt"
        )
    ) {
        return (
            "config",
            "project manifest suggests validation and package shape",
            70,
        );
    }
    if lower.ends_with(".md") || lower.starts_with("readme") {
        return ("docs", "documentation may describe project conventions", 30);
    }
    ("unknown", "supported local context file", 10)
}

fn runtime_lane_context_prompt_score(relative_path: &str, lower_prompt: &str) -> i32 {
    let lower_path = relative_path.to_ascii_lowercase();
    let mut score = 0;
    for token in lower_path
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
    {
        if lower_prompt.contains(token) {
            score += 12;
        }
    }
    score
}

fn runtime_lane_context_validation_hint(
    selected: &[RuntimeLaneContextCandidate],
) -> Option<&'static str> {
    let paths = selected
        .iter()
        .map(|item| item.relative_path.as_str())
        .collect::<Vec<_>>();
    if paths.iter().any(|path| path.ends_with("Cargo.toml")) {
        return Some("cargo test");
    }
    if paths.iter().any(|path| path.ends_with("package.json")) {
        return Some("npm test");
    }
    if paths.iter().any(|path| path.ends_with("go.mod")) {
        return Some("go test ./...");
    }
    if paths.iter().any(|path| path.ends_with(".py"))
        && paths
            .iter()
            .any(|path| path.starts_with("tests/") || path.contains("/tests/"))
    {
        return Some("PYTHONPATH=src python3 -m unittest discover -s tests");
    }
    None
}

fn runtime_lane_persist_native_run_journal(
    metadata: &Value,
    workspace_root: &Path,
    stage: &str,
    receipts: &[NativeToolReceipt],
    latest_output: &str,
    terminal_status: &str,
) {
    let path = metadata
        .get("native_run_journal_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace_root
                .join(".infring")
                .join("native_run_journal.json")
        });
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let changed_files = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .filter(|receipt| matches!(receipt.tool_name.as_str(), "file_write" | "file_patch"))
        .filter_map(|receipt| {
            Some(json!({
                "path": receipt.result.get("path")?.as_str()?,
                "operation": if receipt
                    .result
                    .get("created")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "created"
                } else {
                    "written"
                },
                "receipt_ref": receipt.call_id,
            }))
        })
        .collect::<Vec<_>>();
    let validation_receipts = receipts
        .iter()
        .filter(|receipt| receipt.tool_name == "command_run")
        .map(|receipt| {
            json!({
                "call_id": receipt.call_id,
                "status": receipt.status,
                "success": receipt.result.get("success").cloned().unwrap_or(Value::Null),
                "cmd": receipt.result.get("cmd").cloned().unwrap_or(Value::Null),
                "exit_code": receipt.result.get("exit_code").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let updated_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let payload = json!({
        "schema_version": "native_coding_run_journal_v1",
        "source": "infring_runtime_lane",
        "stage": stage,
        "updated_at_unix_ms": updated_at_unix_ms,
        "terminal_status": terminal_status,
        "native_tool_receipts": receipts,
        "changed_files": changed_files,
        "validation_receipts": validation_receipts,
        "latest_output_preview": latest_output.chars().take(2000).collect::<String>(),
        "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
    });
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = std::fs::write(path, text);
    }
}

fn runtime_lane_parse_deterministic_manifest_from_text(text: &str) -> Option<Value> {
    for block in runtime_lane_fenced_blocks(text) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&block) {
            if let Some(manifest) = runtime_lane_manifest_from_json_value(&parsed) {
                return Some(manifest);
            }
        }
        if let Some(normalized) = runtime_lane_jsonish_triple_quote_to_json(&block) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&normalized) {
                if let Some(manifest) = runtime_lane_manifest_from_json_value(&parsed) {
                    return Some(manifest);
                }
            }
        }
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(text.trim()) {
        if let Some(manifest) = runtime_lane_manifest_from_json_value(&parsed) {
            return Some(manifest);
        }
    }
    if let Some(normalized) = runtime_lane_jsonish_triple_quote_to_json(text.trim()) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&normalized) {
            if let Some(manifest) = runtime_lane_manifest_from_json_value(&parsed) {
                return Some(manifest);
            }
        }
    }
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    let slice = &text[start..=end];
    if let Ok(parsed) = serde_json::from_str::<Value>(slice) {
        return runtime_lane_manifest_from_json_value(&parsed);
    }
    let normalized = runtime_lane_jsonish_triple_quote_to_json(slice)?;
    let parsed = serde_json::from_str::<Value>(&normalized).ok()?;
    runtime_lane_manifest_from_json_value(&parsed)
}

fn runtime_lane_jsonish_triple_quote_to_json(text: &str) -> Option<String> {
    if !text.contains("\"\"\"") {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut changed = false;
    while let Some(start) = rest.find("\"\"\"") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + 3..];
        let Some(end) = after_start.find("\"\"\"") else {
            return None;
        };
        let content = &after_start[..end];
        out.push('"');
        for ch in content.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                _ => out.push(ch),
            }
        }
        out.push('"');
        rest = &after_start[end + 3..];
        changed = true;
    }
    out.push_str(rest);
    changed.then_some(out)
}

fn runtime_lane_manifest_from_json_value(value: &Value) -> Option<Value> {
    if value.get("structured_blocker").is_some() {
        return None;
    }
    if value.get("deterministic_local_loop").is_some()
        || value.get("actions").is_some()
        || value.get("files").is_some()
        || value.get("edits").is_some()
        || value.get("operations").is_some()
    {
        let mut manifest = value.clone();
        runtime_lane_normalize_manifest_action_container(&mut manifest);
        return Some(manifest);
    }
    for key in [
        "manifest",
        "deterministic_manifest",
        "deterministic_local_loop_manifest",
        "local_loop",
        "result",
        "output",
        "payload",
        "plan",
        "final",
        "answer",
        "json",
    ] {
        if let Some(inner) = value.get(key) {
            if let Some(manifest) = runtime_lane_manifest_from_json_value(inner) {
                return Some(manifest);
            }
        }
    }
    if let Some(object) = value.as_object() {
        for inner in object.values() {
            if let Some(manifest) = runtime_lane_manifest_from_json_value(inner) {
                return Some(manifest);
            }
        }
    }
    if let Some(array) = value.as_array() {
        for inner in array {
            if let Some(manifest) = runtime_lane_manifest_from_json_value(inner) {
                return Some(manifest);
            }
        }
    }
    None
}

fn runtime_lane_normalize_manifest_action_container(manifest: &mut Value) {
    let manifest_root = if manifest.get("deterministic_local_loop").is_some() {
        manifest
            .get_mut("deterministic_local_loop")
            .expect("deterministic_local_loop checked")
    } else {
        manifest
    };
    if manifest_root.get("actions").is_none() {
        if let Some(edits) = manifest_root.get("edits").cloned() {
            if let Some(object) = manifest_root.as_object_mut() {
                object.insert("actions".to_string(), edits);
            }
        } else if let Some(operations) = manifest_root.get("operations").cloned() {
            if let Some(object) = manifest_root.as_object_mut() {
                object.insert("actions".to_string(), operations);
            }
        }
    }
}

fn runtime_lane_extract_deterministic_manifest(prompt: &str) -> Option<Value> {
    for block in runtime_lane_fenced_blocks(prompt) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&block) {
            if let Some(manifest) = parsed.get("deterministic_local_loop") {
                return Some(manifest.clone());
            }
            if parsed.get("actions").is_some() || parsed.get("files").is_some() {
                return Some(parsed);
            }
        }
    }
    None
}

fn runtime_lane_manifest_command(value: &Value) -> Option<Vec<String>> {
    let command = value.get("cmd").or_else(|| value.get("command"))?;
    if let Some(items) = command.as_array() {
        let mut out = items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if out.is_empty() {
            None
        } else {
            if out.len() >= 3
                && out[0].trim() == "sh"
                && matches!(out[1].trim(), "-c" | "-lc")
            {
                out[2] = runtime_lane_normalize_shell_command(&out[2]);
            }
            Some(out)
        }
    } else {
        let command = command.as_str()?.trim();
        if command.is_empty() {
            None
        } else {
            Some(vec![
                "sh".to_string(),
                "-lc".to_string(),
                runtime_lane_normalize_shell_command(command),
            ])
        }
    }
}

fn runtime_lane_normalize_shell_command(command: &str) -> String {
    let command = runtime_lane_strip_shell_command_prose_prefix(command);
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return command.to_string();
    }
    let script_index = tokens.iter().position(|token| {
        !token.contains('=') || token.starts_with("./") || token.starts_with('/')
    });
    let Some(script_index) = script_index else {
        return command.to_string();
    };
    let script = tokens[script_index];
    if !script.ends_with(".py") || script.ends_with("python.py") {
        return command.to_string();
    }
    if script_index > 0 && matches!(tokens.get(script_index - 1), Some(&"python" | &"python3")) {
        return command.to_string();
    }
    let mut normalized = Vec::with_capacity(tokens.len() + 1);
    normalized.extend(tokens[..script_index].iter().copied());
    normalized.push("python3");
    normalized.extend(tokens[script_index..].iter().copied());
    normalized.join(" ")
}

fn runtime_lane_strip_shell_command_prose_prefix(command: &str) -> &str {
    let trimmed = command.trim();
    let lower = trimmed.to_ascii_lowercase();
    for prefix in [
        "after validation:",
        "after tests:",
        "semantic probe:",
        "semantic probe command:",
        "validation:",
        "validation command:",
        "run:",
        "command:",
    ] {
        if lower.starts_with(prefix) {
            return trimmed[prefix.len()..].trim_start();
        }
    }
    trimmed
}

fn runtime_lane_direct_mutation_surface_enabled(
    tools: &[String],
    capability_packs: &[String],
) -> bool {
    capability_packs
        .iter()
        .any(|pack| pack.trim().eq_ignore_ascii_case("local-coding-files"))
        || tools.iter().any(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "file_write" | "write_file" | "workspace.write" | "workspace_write"
            )
        })
}

fn runtime_lane_extract_explicit_file_content(prompt: &str) -> Option<String> {
    if let Some(content) = runtime_lane_first_fenced_block(prompt) {
        return Some(content);
    }
    let lower = prompt.to_ascii_lowercase();
    let marker_index = lower
        .find("content:")
        .or_else(|| lower.find("contents:"))
        .or_else(|| lower.find("file text:"))?;
    let marker_end = prompt[marker_index..].find(':')? + marker_index + 1;
    let content =
        prompt[marker_end..].trim_start_matches(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'));
    if content.trim().is_empty() {
        None
    } else {
        Some(ensure_trailing_newline(content.to_string()))
    }
}

fn runtime_lane_first_fenced_block(prompt: &str) -> Option<String> {
    let (_, after_open) = prompt.split_once("```")?;
    let (block, _) = after_open.split_once("```")?;
    let block = block.trim_start_matches('\n');
    let mut lines = block.lines();
    let first = lines.next().unwrap_or("");
    let content = if runtime_lane_looks_like_fence_language(first) {
        lines.collect::<Vec<_>>().join("\n")
    } else {
        block.to_string()
    };
    if content.trim().is_empty() {
        None
    } else {
        Some(ensure_trailing_newline(content))
    }
}

fn runtime_lane_fenced_blocks(prompt: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut rest = prompt;
    while let Some(open) = rest.find("```") {
        let after_open = &rest[open + 3..];
        let Some(close) = after_open.find("```") else {
            break;
        };
        let block = after_open[..close].trim_start_matches('\n');
        let mut lines = block.lines();
        let first = lines.next().unwrap_or("");
        let content = if runtime_lane_looks_like_fence_language(first) {
            lines.collect::<Vec<_>>().join("\n")
        } else {
            block.to_string()
        };
        if !content.trim().is_empty() {
            blocks.push(content);
        }
        rest = &after_open[close + 3..];
    }
    blocks
}

fn runtime_lane_looks_like_fence_language(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 24
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '#'))
}

fn ensure_trailing_newline(mut content: String) -> String {
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

fn runtime_lane_extract_workspace_root(prompt: &str) -> Option<PathBuf> {
    for line in prompt.lines() {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("project root")
            || lower.contains("workspace root")
            || lower.contains("repo root"))
        {
            continue;
        }
        if let Some((_, value)) = line.split_once(':') {
            if let Some(path) = runtime_lane_first_absolute_path_token(value) {
                return Some(path);
            }
        }
        if let Some(path) = runtime_lane_first_absolute_path_token(line) {
            return Some(path);
        }
    }
    None
}

fn runtime_lane_first_absolute_path_token(text: &str) -> Option<PathBuf> {
    text.split_whitespace()
        .map(runtime_lane_clean_path_token)
        .filter(|token| token.starts_with('/'))
        .map(PathBuf::from)
        .next()
}

fn runtime_lane_extract_target_file_path(prompt: &str) -> Option<String> {
    let without_fences = runtime_lane_strip_fenced_blocks(prompt);
    for span in runtime_lane_inline_code_spans(&without_fences) {
        if runtime_lane_is_file_like_path(&span) {
            return Some(span);
        }
    }
    for line in without_fences.lines() {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("target")
            || lower.contains("file")
            || lower.contains("path")
            || lower.contains("create")
            || lower.contains("write"))
        {
            continue;
        }
        for token in line.split_whitespace().map(runtime_lane_clean_path_token) {
            if runtime_lane_is_file_like_path(&token) {
                return Some(token);
            }
        }
    }
    None
}

fn runtime_lane_strip_fenced_blocks(prompt: &str) -> String {
    let mut out = String::new();
    let mut rest = prompt;
    loop {
        let Some(open) = rest.find("```") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 3..];
        let Some(close) = after_open.find("```") else {
            break;
        };
        rest = &after_open[close + 3..];
    }
    out
}

fn runtime_lane_inline_code_spans(text: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let span = runtime_lane_clean_path_token(&after_start[..end]);
        if !span.is_empty() {
            spans.push(span);
        }
        rest = &after_start[end + 1..];
    }
    spans
}

fn runtime_lane_clean_path_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches(',')
        .trim_matches(';')
        .trim_matches(':')
        .trim_end_matches('.')
        .trim_matches(')')
        .trim_matches('(')
        .trim_matches(']')
        .trim_matches('[')
        .to_string()
}

fn runtime_lane_is_file_like_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\n') || value.ends_with('/') {
        return false;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return false;
    }
    let path = Path::new(value);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return true;
    }
    if path.extension().is_some() {
        return true;
    }
    value.starts_with("./")
        || value.starts_with('/') && value.rsplit('/').next().unwrap_or("").contains('.')
}

fn runtime_lane_resolve_target_path(
    raw_target: &str,
    workspace_root: Option<&PathBuf>,
) -> Result<PathBuf, String> {
    let raw_path = PathBuf::from(raw_target);
    if raw_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("target_path_must_not_contain_parent_segments".to_string());
    }
    let target = if raw_path.is_absolute() {
        raw_path
    } else {
        let Some(root) = workspace_root else {
            return Err("relative_target_requires_workspace_root".to_string());
        };
        root.join(raw_path)
    };
    if let Some(root) = workspace_root {
        if !target.starts_with(root) {
            return Err("target_path_outside_workspace_root".to_string());
        }
    }
    Ok(target)
}

fn runtime_lane_prompt_requests_overwrite(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    lower.contains("overwrite")
        || lower.contains("replace the file")
        || lower.contains("replace file")
        || lower.contains("update the file")
}

fn runtime_lane_attach_agent_run_journal(response: &mut RuntimeLaneResponse, run: &AgentRunResult) {
    let native_tool_receipts = run
        .receipt
        .get("native_tool_receipts")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(object) = response.receipt.as_object_mut() {
        object.insert("agent_run_receipt".to_string(), run.receipt.clone());
        object.insert("native_tool_receipts".to_string(), native_tool_receipts);
        object.insert(
            "provider_output".to_string(),
            Value::String(run.response.output.clone()),
        );
        object.insert("provider_raw".to_string(), run.response.raw.clone());
    }
    if response.output.is_empty() {
        response.output = run.response.output.clone();
    }
}

fn permission_trit_code(value: PermissionTrit) -> i8 {
    match value {
        PermissionTrit::Deny => -1,
        PermissionTrit::Ask => 0,
        PermissionTrit::Allow => 1,
    }
}

fn file_tool_permission(tool: &str) -> Option<&'static str> {
    match tool.trim().to_ascii_lowercase().as_str() {
        "file_list" | "list_files" | "workspace.list" | "workspace_list" | "file_stat"
        | "stat_file" | "file_exists" | "workspace.stat" | "workspace_stat" => Some("file.read"),
        "file_read"
        | "file_read_many"
        | "read_file"
        | "read_many_files"
        | "workspace.read"
        | "workspace.read_many"
        | "workspace_read"
        | "workspace_read_many" => Some("file.read"),
        "file_write" | "write_file" | "workspace.write" | "workspace_write" => Some("file.write"),
        "file_patch" | "patch_file" | "apply_patch" | "workspace.patch" | "workspace_patch" => {
            Some("file.patch")
        }
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run" => {
            Some("command.run")
        }
        "command_resolve" | "resolve_command" | "command.resolve" | "executable_resolve"
        | "executable.resolve" => Some("command.resolve"),
        _ => None,
    }
}

fn native_success_contract_violation(
    metadata: &Value,
    run_receipt: &Value,
    output: &str,
) -> Option<(String, Value)> {
    let criteria = metadata.get("native_success_criteria")?;
    if !criteria.is_object() {
        return None;
    }
    let requires_native_tool_use = criteria
        .get("requires_native_tool_use")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_successful_mutation_receipt = criteria
        .get("requires_successful_mutation_receipt")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_successful_discovery_receipt = criteria
        .get("requires_successful_discovery_receipt")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let min_successful_tool_receipts = criteria
        .get("min_successful_tool_receipts")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let min_successful_discovery_receipts = criteria
        .get("min_successful_discovery_receipts")
        .and_then(Value::as_u64)
        .unwrap_or(if requires_successful_discovery_receipt {
            1
        } else {
            0
        });
    let successful_discovery_tools = criteria
        .get("successful_discovery_tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_native_tool_name)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec!["file_list".to_string(), "file_stat".to_string()]);
    let successful_mutation_tools = criteria
        .get("successful_mutation_tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_native_tool_name)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec!["file_write".to_string(), "file_patch".to_string()]);

    let receipts = run_receipt
        .get("native_tool_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let receipt_count = run_receipt
        .get("native_tool_call_count")
        .and_then(Value::as_u64)
        .unwrap_or(receipts.len() as u64);
    let successful_tool_receipt_count = receipts
        .iter()
        .filter(|receipt| receipt.get("status").and_then(Value::as_str) == Some("ok"))
        .count() as u64;
    let successful_discovery_receipt_count = receipts
        .iter()
        .filter(|receipt| receipt.get("status").and_then(Value::as_str) == Some("ok"))
        .filter(|receipt| {
            receipt
                .get("tool_name")
                .and_then(Value::as_str)
                .map(normalize_native_tool_name)
                .map(|tool| {
                    successful_discovery_tools
                        .iter()
                        .any(|allowed| allowed == &tool)
                })
                .unwrap_or(false)
        })
        .count() as u64;
    let successful_mutation_receipt_count = receipts
        .iter()
        .filter(|receipt| receipt.get("status").and_then(Value::as_str) == Some("ok"))
        .filter(|receipt| {
            receipt
                .get("tool_name")
                .and_then(Value::as_str)
                .map(normalize_native_tool_name)
                .map(|tool| {
                    successful_mutation_tools
                        .iter()
                        .any(|allowed| allowed == &tool)
                })
                .unwrap_or(false)
        })
        .count() as u64;

    let details = || {
        json!({
            "criteria": criteria,
            "native_tool_call_count": receipt_count,
            "successful_tool_receipt_count": successful_tool_receipt_count,
            "successful_discovery_receipt_count": successful_discovery_receipt_count,
            "successful_mutation_receipt_count": successful_mutation_receipt_count,
            "native_tool_receipt_summary": native_tool_receipt_summary(&receipts),
            "agent_output_preview": output.chars().take(1200).collect::<String>(),
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "enforcement_mode": "strict_fail_closed",
        })
    };

    if requires_native_tool_use && receipt_count == 0 {
        return Some((
            "runtime_lane_required_native_tool_use_missing".to_string(),
            details(),
        ));
    }
    if min_successful_discovery_receipts > 0
        && successful_discovery_receipt_count < min_successful_discovery_receipts
    {
        return Some((
            "runtime_lane_required_native_discovery_receipt_missing".to_string(),
            details(),
        ));
    }
    if min_successful_tool_receipts > 0
        && successful_tool_receipt_count < min_successful_tool_receipts
    {
        return Some((
            "runtime_lane_required_native_tool_receipt_missing".to_string(),
            details(),
        ));
    }
    if requires_successful_mutation_receipt && successful_mutation_receipt_count == 0 {
        return Some((
            "runtime_lane_required_native_mutation_receipt_missing".to_string(),
            details(),
        ));
    }
    None
}

fn native_tool_receipt_summary(receipts: &[Value]) -> Vec<Value> {
    receipts
        .iter()
        .map(|receipt| {
            json!({
                "call_id": receipt.get("call_id").and_then(Value::as_str).unwrap_or(""),
                "tool_name": receipt.get("tool_name").and_then(Value::as_str).unwrap_or(""),
                "status": receipt.get("status").and_then(Value::as_str).unwrap_or(""),
                "error": receipt.get("error").cloned().unwrap_or(Value::Null),
                "path": receipt
                    .get("result")
                    .and_then(|result| result.get("path"))
                    .cloned()
                    .unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn normalize_native_tool_name(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "list_files" | "workspace.list" | "workspace_list" => "file_list".to_string(),
        "stat_file" | "file_exists" | "workspace.stat" | "workspace_stat" => {
            "file_stat".to_string()
        }
        "write_file" | "workspace.write" | "workspace_write" => "file_write".to_string(),
        "patch_file" | "apply_patch" | "workspace.patch" | "workspace_patch" => {
            "file_patch".to_string()
        }
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run" => {
            "command_run".to_string()
        }
        "command_resolve" | "resolve_command" | "command.resolve" | "executable_resolve"
        | "executable.resolve" => "command_resolve".to_string(),
        "read_file" | "workspace.read" | "workspace_read" => "file_read".to_string(),
        "read_many_files" | "workspace.read_many" | "workspace_read_many" => {
            "file_read_many".to_string()
        }
        other => other.to_string(),
    }
}

fn public_reasoning_contract_violation(
    metadata: &Value,
    run_receipt: &Value,
    output: &str,
) -> Option<(String, Value)> {
    let contract = metadata.get("public_reasoning_trace_contract")?;
    if !contract.is_object() {
        return None;
    }
    let agent_status = run_receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("ok");
    if agent_status != "ok" {
        return None;
    }

    let emitted = contract
        .get("emits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let requires_trace = emitted
        .iter()
        .any(|item| item == "public_reasoning_trace_v1")
        || contract
            .get("local_trace_required_fields")
            .and_then(Value::as_array)
            .is_some();
    let requires_rollup = emitted
        .iter()
        .any(|item| item == "public_reasoning_rollup_v1");
    let has_trace =
        output.contains("public_reasoning_trace") && output.contains("public_reasoning_trace_v1");
    let has_rollup =
        output.contains("reasoning_rollup") && output.contains("public_reasoning_rollup_v1");
    let still_requests_tools =
        output.contains("\"tool_calls\"") || output.contains("{\"tool_calls\"");
    let redaction_policy = contract
        .get("redaction_policy")
        .and_then(Value::as_str)
        .unwrap_or("no_hidden_chain_of_thought");
    let mentions_redaction = output.contains(redaction_policy)
        || output.contains("hidden chain-of-thought")
        || output.contains("hidden chain of thought")
        || output.contains("redaction");

    if still_requests_tools
        || (requires_trace && !has_trace)
        || (requires_rollup && !has_rollup)
        || !mentions_redaction
    {
        return Some((
            "runtime_lane_public_reasoning_trace_missing".to_string(),
            json!({
                "criteria": {
                    "requires_public_reasoning_trace": requires_trace,
                    "requires_reasoning_rollup": requires_rollup,
                    "requires_redaction_policy_ack": true,
                    "redaction_policy": redaction_policy,
                },
                "observed": {
                    "has_public_reasoning_trace": has_trace,
                    "has_reasoning_rollup": has_rollup,
                    "mentions_redaction_policy": mentions_redaction,
                    "still_requests_tools": still_requests_tools,
                },
                "agent_status": agent_status,
                "agent_output_preview": output.chars().take(1200).collect::<String>(),
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
                "enforcement_mode": "strict_fail_closed",
            }),
        ));
    }
    None
}

fn runtime_requested_wasm_modules(tools: &[String], metadata: &Value) -> Vec<String> {
    let mut modules = Vec::<String>::new();
    for tool in tools {
        if let Some(module) = tool.strip_prefix("wasm.") {
            let normalized = module.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                modules.push(normalized);
            }
        }
    }
    if let Some(items) = metadata.get("wasm_modules").and_then(Value::as_array) {
        for item in items {
            let Some(text) = item.as_str() else {
                continue;
            };
            let normalized = text.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                modules.push(normalized);
            }
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

fn runtime_requests_network(tools: &[String], metadata: &Value) -> bool {
    if metadata
        .get("wasm_requests_network")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    tools.iter().any(|tool| {
        matches!(
            tool.as_str(),
            "web.search" | "web.fetch" | "network.request"
        )
    })
}
