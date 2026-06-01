// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::capability_pack::CapabilityPackCatalog;
use crate::coding_execution_spine::CodingSpineAction;
use crate::coding_execution_spine_adapter::{
    coding_execution_spine_decision_from_native_receipts, coding_execution_spine_v1_enabled,
    coding_execution_spine_v1_routes_lane,
};
use crate::first_mutation_artifact_lane::{
    first_mutation_artifact_lane_v1_enabled, first_mutation_artifact_lane_v1_metadata,
    first_mutation_artifact_lane_v1_prompt, first_mutation_artifact_lane_v1_routes_lane,
    first_mutation_artifact_lane_v1_system,
    first_mutation_artifact_lane_v1_tools,
};
use crate::native_evidence::{
    native_tool_artifact_contract_enabled, native_tool_artifact_repair_reasons,
    native_tool_changed_paths, native_tool_changed_paths_include, native_tool_coding_task_lane,
    native_tool_context_only_turn, native_tool_failed_validation_command_refs,
    native_tool_failed_validation_receipt_details, native_tool_has_successful_memory_write_command,
    native_tool_has_successful_mutation, native_tool_has_successful_validation_command,
    native_tool_is_probable_micro_direct_write_task, native_tool_needs_artifact_finalization,
    native_tool_latest_failed_validation_receipt_details,
    native_tool_needs_public_report_finalization, native_tool_prompt_evidence_gaps,
    native_tool_prompt_expected_memory_row_id, native_tool_prompt_has_multiple_requirements,
    native_tool_prompt_memory_cli_pattern, native_tool_prompt_project_root,
    native_tool_prompt_required_changed_paths,
    native_tool_prompt_requires_memory_write, native_tool_prompt_requires_product_mutation,
    native_tool_prompt_requires_multi_file_product_slice, native_tool_prompt_requires_test_changes,
    native_tool_prompt_requires_validation_command, native_tool_product_slice_gaps,
    native_tool_should_synthesize_micro_final, native_tool_unique_code_path_mentions,
};
use crate::native_prompt_policy::{
    native_tool_completion_evidence_repair_prompt, native_tool_completion_repair_action_brief,
    native_tool_context_to_mutation_retry_prompt, native_tool_empty_retry_prompt,
    native_tool_failed_validation_repair_hint, native_tool_initial_prompt,
    native_tool_missing_test_change_repair_hint, native_tool_orchestration_prompt_text,
    native_tool_mutation_only_recovery_prompt, native_tool_compact_action_controller_prompt,
    native_tool_public_reasoning_finalization_prompt, native_tool_public_reasoning_metadata,
    native_tool_recovery_prompt,
};
use crate::native_synthetic_artifact::{
    native_tool_synthetic_completion_evidence_response, native_tool_synthetic_micro_final_response,
};
use crate::native_tools::{
    native_tool_observation_prompt, parse_native_tool_calls, NativeToolCall, NativeToolDispatcher,
    NativeToolReceipt,
};
use crate::native_workflow_artifact::native_tool_auto_workflow_artifact_receipts;
use crate::provider::{
    ProviderClientRegistry, ProviderError, ProviderErrorCode, ProviderRequest, ProviderResponse,
};
use crate::scheduler::SchedulePlan;
use crate::telemetry::{ReceiptEvent, ReceiptSpan};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentContract {
    pub name: String,
    pub preamble: String,
    pub initial_prompt: String,
    pub lifespan_seconds: u64,
    pub provider: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub capability_packs: Vec<String>,
    pub schedule: Option<SchedulePlan>,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub response: ProviderResponse,
    pub receipt: Value,
    pub trace: ReceiptSpan,
}

#[derive(Clone)]
pub struct AgentExecutionContext<'a> {
    pub provider_registry: &'a ProviderClientRegistry,
    pub capability_catalog: Option<&'a CapabilityPackCatalog>,
}

impl<'a> AgentExecutionContext<'a> {
    pub fn new(
        provider_registry: &'a ProviderClientRegistry,
        capability_catalog: Option<&'a CapabilityPackCatalog>,
    ) -> Self {
        Self {
            provider_registry,
            capability_catalog,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentBuildError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for AgentBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.code, self.message)
    }
}

impl std::error::Error for AgentBuildError {}

pub struct AgentBuilder {
    contract: AgentContract,
}

impl AgentBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            contract: AgentContract {
                name: sanitize_token(&name.into(), 80),
                preamble: String::new(),
                initial_prompt: String::new(),
                lifespan_seconds: 3600,
                provider: "local-echo".to_string(),
                model: None,
                tools: Vec::new(),
                capability_packs: Vec::new(),
                schedule: None,
                metadata: Value::Object(Map::new()),
            },
        }
    }

    pub fn preamble(mut self, preamble: impl Into<String>) -> Self {
        self.contract.preamble = sanitize_token(&preamble.into(), 2000);
        self
    }

    pub fn initial_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.contract.initial_prompt = sanitize_token(&prompt.into(), 64_000);
        self
    }

    pub fn lifespan_seconds(mut self, lifespan_seconds: u64) -> Self {
        self.contract.lifespan_seconds = lifespan_seconds;
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.contract.provider = sanitize_token(&provider.into(), 120);
        self
    }

    pub fn provider_from_env(
        mut self,
        env: &std::collections::HashMap<String, String>,
        key: &str,
    ) -> Self {
        if let Some(provider) = env
            .get(key)
            .or_else(|| env.get("INFRING_PROVIDER"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            self.contract.provider = sanitize_token(&provider, 120);
        }
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.contract.model = Some(sanitize_token(&model.into(), 120));
        self
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        let token = sanitize_token(&tool.into(), 120);
        if !token.is_empty() && !self.contract.tools.iter().any(|entry| entry == &token) {
            self.contract.tools.push(token);
        }
        self
    }

    pub fn capability_pack(mut self, pack: impl Into<String>) -> Self {
        let token = sanitize_token(&pack.into(), 120);
        if !token.is_empty()
            && !self
                .contract
                .capability_packs
                .iter()
                .any(|entry| entry == &token)
        {
            self.contract.capability_packs.push(token);
        }
        self
    }

    pub fn schedule(mut self, plan: SchedulePlan) -> Self {
        self.contract.schedule = Some(plan);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.contract.metadata = metadata;
        self
    }

    pub fn build(self) -> Result<AgentContract, AgentBuildError> {
        if self.contract.name.trim().is_empty() {
            return Err(AgentBuildError {
                code: "agent_name_required".to_string(),
                message: "Agent name is required.".to_string(),
            });
        }
        if self.contract.initial_prompt.trim().is_empty() {
            return Err(AgentBuildError {
                code: "agent_initial_prompt_required".to_string(),
                message: "Agent contract needs an initial prompt.".to_string(),
            });
        }
        if self.contract.lifespan_seconds == 0 || self.contract.lifespan_seconds > 31_536_000 {
            return Err(AgentBuildError {
                code: "agent_lifespan_invalid".to_string(),
                message: "Lifespan must be between 1 second and 1 year.".to_string(),
            });
        }
        Ok(self.contract)
    }
}

impl AgentContract {
    pub fn resolved_tools(&self, catalog: Option<&CapabilityPackCatalog>) -> Vec<String> {
        if let Some(catalog) = catalog {
            return catalog.expand_tools(&self.capability_packs, &self.tools);
        }
        self.tools.clone()
    }

    pub fn with_default_schedule_from_packs(mut self, catalog: &CapabilityPackCatalog) -> Self {
        if self.schedule.is_some() {
            return self;
        }
        let mut chosen = None;
        for pack_id in &self.capability_packs {
            if let Some(interval) = catalog.default_interval_for_pack(pack_id) {
                chosen = Some(
                    chosen
                        .map(|current: u64| current.min(interval))
                        .unwrap_or(interval),
                );
            }
        }
        if let Some(interval) = chosen {
            let max_runs = catalog.default_max_runs_for_packs(&self.capability_packs);
            self.schedule = Some(SchedulePlan {
                interval_seconds: interval,
                jitter_seconds: 15,
                max_runs,
            });
        }
        self
    }

    pub fn run_once(
        &self,
        context: &AgentExecutionContext<'_>,
    ) -> Result<AgentRunResult, ProviderError> {
        let run_started = Instant::now();
        let provider_resolve_started = Instant::now();
        let provider = context
            .provider_registry
            .from_provider_id(self.provider.as_str())?;
        let provider_resolve_ms = native_tool_bounded_patch_elapsed_ms(provider_resolve_started);
        let tool_resolve_started = Instant::now();
        let tools = self.resolved_tools(context.capability_catalog);
        let tool_resolve_ms = native_tool_bounded_patch_elapsed_ms(tool_resolve_started);
        let started_ms = Utc::now().timestamp_millis();
        let model_tool_loop_started = Instant::now();
        let (response, tool_receipts, provider_call_count, terminal_status) =
            self.run_with_optional_native_tools(provider, &tools)?;
        let model_tool_loop_ms = native_tool_bounded_patch_elapsed_ms(model_tool_loop_started);
        let finished_ms = Utc::now().timestamp_millis();
        let duration_ms = (finished_ms - started_ms).max(0) as u64;
        let mut events = vec![ReceiptEvent {
            event_id: "provider.complete".to_string(),
            status: terminal_status.clone(),
            duration_ms,
            error_code: None,
            timestamp_ms: finished_ms,
            attributes: BTreeMap::from([
                ("provider".to_string(), response.provider.clone()),
                ("model".to_string(), response.model.clone()),
                (
                    "provider_call_count".to_string(),
                    provider_call_count.to_string(),
                ),
            ]),
        }];
        for receipt in &tool_receipts {
            events.push(ReceiptEvent {
                event_id: format!("native_tool.{}", receipt.tool_name),
                status: receipt.status.clone(),
                duration_ms: receipt.duration_ms,
                error_code: receipt.error.clone(),
                timestamp_ms: finished_ms,
                attributes: BTreeMap::from([
                    ("tool_name".to_string(), receipt.tool_name.clone()),
                    ("call_id".to_string(), receipt.call_id.clone()),
                ]),
            });
        }
        let trace = ReceiptSpan {
            trace_id: format!("trace-{}-{}", self.name, finished_ms),
            agent_name: self.name.clone(),
            started_at_ms: started_ms,
            events,
            attributes: BTreeMap::from([("tools".to_string(), tools.join(","))]),
        };
        let receipt_serialization_started = Instant::now();
        let native_tool_call_count = tool_receipts.len();
        let native_tool_phase_latency_ms = native_tool_receipt_phase_latency(&tool_receipts);
        native_tool_persist_runtime_timeline_event(
            &self.metadata,
            &self.initial_prompt,
            "agent_receipt_serialization_start",
            native_tool_bounded_patch_elapsed_ms(run_started),
            json!({
                "native_tool_call_count": native_tool_call_count,
                "terminal_status": terminal_status,
            }),
        );
        let receipt = json!({
            "type": "agent_run_receipt",
            "agent": self.name,
            "provider": response.provider,
            "model": response.model,
            "status": terminal_status,
            "tool_count": tools.len(),
            "native_tool_call_count": native_tool_call_count,
            "lifespan_seconds": self.lifespan_seconds,
            "duration_ms": duration_ms,
            "agent_runtime_phase_latency_ms": {
                "provider_resolve_ms": provider_resolve_ms,
                "tool_resolve_ms": tool_resolve_ms,
                "model_tool_loop_ms": model_tool_loop_ms,
                "total_ms": native_tool_bounded_patch_elapsed_ms(run_started)
            },
            "native_tool_phase_latency_ms": native_tool_phase_latency_ms,
            "trace_id": trace.trace_id,
            "runtime_failure_analysis": native_tool_runtime_failure_analysis(
                &tool_receipts,
                &terminal_status,
            ),
            "workflow": self
                .metadata
                .get("workflow")
                .cloned()
                .unwrap_or(Value::Null),
            "native_tool_receipts": tool_receipts,
        });
        native_tool_persist_runtime_timeline_event(
            &self.metadata,
            &self.initial_prompt,
            "agent_receipt_serialization_end",
            native_tool_bounded_patch_elapsed_ms(run_started),
            json!({
                "duration_ms": native_tool_bounded_patch_elapsed_ms(receipt_serialization_started),
                "native_tool_call_count": native_tool_call_count,
            }),
        );
        native_tool_persist_runtime_timeline_event(
            &self.metadata,
            &self.initial_prompt,
            "agent_run_result_ready",
            native_tool_bounded_patch_elapsed_ms(run_started),
            json!({
                "duration_ms": duration_ms,
                "terminal_status": terminal_status,
            }),
        );
        Ok(AgentRunResult {
            response,
            receipt,
            trace,
        })
    }

    fn run_with_optional_native_tools(
        &self,
        provider: Arc<dyn crate::provider::ProviderClient>,
        tools: &[String],
    ) -> Result<(ProviderResponse, Vec<NativeToolReceipt>, u64, String), ProviderError> {
        let dispatcher = NativeToolDispatcher::with_metadata(tools, &self.metadata);
        if !dispatcher.has_native_tools() {
            let request = ProviderRequest {
                prompt: self.initial_prompt.clone(),
                system: Some(self.preamble.clone()),
                tools: tools.to_vec(),
                model: self.model.clone(),
                metadata: self.metadata.clone(),
            };
            return provider
                .complete(&request)
                .map(|response| (response, Vec::new(), 1, "ok".to_string()));
        }

        let native_timeline_started = Instant::now();
        native_tool_persist_runtime_timeline_event(
            &self.metadata,
            &self.initial_prompt,
            "runtime_lane_entry",
            0,
            json!({
                "tool_count": tools.len(),
                "has_successful_mutation_requirement": native_tool_requires_successful_mutation(&self.metadata),
            }),
        );
        let max_turns = native_tool_max_turns(&self.metadata);
        let mut prompt = native_tool_initial_prompt(&self.initial_prompt, &self.metadata);
        let system = if self.preamble.trim().is_empty() {
            dispatcher.tool_protocol_prompt()
        } else {
            format!("{}\n\n{}", self.preamble, dispatcher.tool_protocol_prompt())
        };
        let mut all_receipts = Vec::<NativeToolReceipt>::new();
        let mut last_response = None;
        let mut provider_call_count = 0u64;
        let empty_tool_retry_limit = native_tool_empty_retry_limit(&self.metadata);
        let mut empty_tool_retry_count = 0u64;
        let mut context_only_turn_count = 0u64;
        let loop_started = Instant::now();
        let wall_timeout = native_tool_wall_timeout(&self.metadata);
        let coding_task_lane = native_tool_coding_task_lane(&self.metadata, &self.initial_prompt);
        let micro_direct_write_task = coding_task_lane == "new_file_fast_path";
        let bounded_direct_edit_task = !micro_direct_write_task
            && native_tool_bounded_direct_edit_lane_active(&self.metadata, &self.initial_prompt);
        let bounded_fast_edit_preflight =
            bounded_direct_edit_task && native_tool_bounded_fast_edit_preflight_enabled(&self.metadata);
        let first_edit_batch_contract = bounded_fast_edit_preflight
            && native_tool_first_edit_batch_contract_enabled(&self.metadata);
        let multi_requirement_validation_task =
            native_tool_prompt_requires_pre_mutation_validation(&self.initial_prompt)
                && native_tool_prompt_has_multiple_requirements(&self.initial_prompt);
        let mut next_provider_timeout_seconds: Option<u64> = None;
        let mut mutation_only_recovery_pending = false;
        let mut mutation_only_recovery_reason: Option<String> = None;
        let mut validation_guided_compact_repair_pending = false;
        let mut validation_guided_compact_repair_reason: Option<String> = None;
        if bounded_direct_edit_task {
            prompt = native_tool_bounded_direct_edit_initial_prompt(
                &self.metadata,
                &self.initial_prompt,
            );
            let bootstrap_started = Instant::now();
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "bootstrap_context_start",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({ "lane": "bounded_direct_edit" }),
            );
            let bootstrap_receipts =
                native_tool_bootstrap_context_receipts(&dispatcher, &self.initial_prompt);
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "bootstrap_context_end",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "duration_ms": native_tool_bounded_patch_elapsed_ms(bootstrap_started),
                    "receipt_count": bootstrap_receipts.len(),
                }),
            );
            if !bootstrap_receipts.is_empty() {
                all_receipts.extend(bootstrap_receipts);
                let pre_validation_started = Instant::now();
                if let Some(validation_receipt) =
                    native_tool_pre_mutation_validation_bootstrap_receipt(
                        &dispatcher,
                        &self.initial_prompt,
                        &all_receipts,
                    )
                {
                    native_tool_persist_runtime_timeline_event(
                        &self.metadata,
                        &self.initial_prompt,
                        "bootstrap_validation_end",
                        native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                        json!({
                            "duration_ms": native_tool_bounded_patch_elapsed_ms(pre_validation_started),
                            "status": validation_receipt.status,
                            "success": validation_receipt.result.get("success").cloned().unwrap_or(Value::Null),
                        }),
                    );
                    all_receipts.push(validation_receipt);
                    let import_surface_validation_task = matches!(
                        native_tool_validation_failure_class(
                            &native_tool_latest_failed_validation_receipt_details(&all_receipts)
                        )
                        .as_deref(),
                        Some("import_surface_missing")
                    );
                    let mut import_seed_round_limit =
                        native_tool_python_import_surface_seed_round_limit(&self.metadata);
                    if import_seed_round_limit == 0
                        && native_tool_python_import_surface_seed_enabled(&self.metadata)
                    {
                        import_seed_round_limit = if import_surface_validation_task {
                            5
                        } else if multi_requirement_validation_task {
                            1
                        } else {
                            0
                        };
                    }
                    for import_seed_round in 0..import_seed_round_limit {
                        let import_seed_started = Instant::now();
                        let import_seed_receipts = native_tool_python_import_surface_seed_receipts(
                            &dispatcher,
                            &self.metadata,
                            &self.initial_prompt,
                            &all_receipts,
                        );
                        if import_seed_receipts.is_empty() {
                            break;
                        }
                        native_tool_persist_runtime_timeline_event(
                            &self.metadata,
                            &self.initial_prompt,
                            "bootstrap_python_import_surface_seed_end",
                            native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                            json!({
                                "duration_ms": native_tool_bounded_patch_elapsed_ms(import_seed_started),
                                "round": import_seed_round,
                                "receipt_count": import_seed_receipts.len(),
                            }),
                        );
                        all_receipts.extend(import_seed_receipts);
                        let import_seed_validation_started = Instant::now();
                        let Some(import_seed_validation_receipt) =
                            native_tool_pre_mutation_validation_receipt(
                                &dispatcher,
                                &self.initial_prompt,
                                &format!(
                                    "runtime_bootstrap_import_surface_seed_validation_{import_seed_round}"
                                ),
                            )
                        else {
                            break;
                        };
                        let import_seed_validation_success = import_seed_validation_receipt
                            .result
                            .get("success")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        native_tool_persist_runtime_timeline_event(
                            &self.metadata,
                            &self.initial_prompt,
                            "bootstrap_validation_after_import_surface_seed_end",
                            native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                            json!({
                                "duration_ms": native_tool_bounded_patch_elapsed_ms(import_seed_validation_started),
                                "round": import_seed_round,
                                "status": import_seed_validation_receipt.status,
                                "success": import_seed_validation_receipt.result.get("success").cloned().unwrap_or(Value::Null),
                            }),
                        );
                        all_receipts.push(import_seed_validation_receipt);
                        if import_seed_validation_success {
                            break;
                        }
                        let import_seed_failure_class = native_tool_validation_failure_class(
                            &native_tool_latest_failed_validation_receipt_details(&all_receipts),
                        );
                        if !matches!(
                            import_seed_failure_class.as_deref(),
                            Some("import_surface_missing")
                        ) {
                            break;
                        }
                        if import_seed_round + 1 >= import_seed_round_limit {
                            break;
                        }
                    }
                } else {
                    native_tool_persist_runtime_timeline_event(
                        &self.metadata,
                        &self.initial_prompt,
                        "bootstrap_validation_skipped",
                        native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                        json!({
                            "duration_ms": native_tool_bounded_patch_elapsed_ms(pre_validation_started),
                        }),
                    );
                }
                let observation =
                    native_tool_bootstrap_observation_prompt(&self.metadata, &all_receipts);
                let preflight_ready = bounded_fast_edit_preflight
                    && native_tool_has_successful_read_context_receipt(&all_receipts);
                let default_bootstrap_rule = if preflight_ready {
                    "Runtime bounded_fast_edit_preflight has already loaded bounded local context and any explicitly requested pre-mutation validation receipt. Do not call read/list/stat/resolve tools before the first mutation. Use the observed files and validation output to return one small JSON tool-call batch with file_patch/file_write edits first, followed by requested validation/probe command_run calls. Return a structured blocker only if the loaded context proves mutation is unsafe."
                } else if native_tool_prompt_requires_pre_mutation_validation(
                    &self.initial_prompt,
                ) {
                    "Runtime has already loaded the bounded local context for this direct edit. If a pre-mutation validation receipt is present, do not rerun the same failing command before repair; use its output to return the smallest source/test file_write or file_patch repair next, then rerun validation/probe commands."
                } else {
                    "Runtime has already loaded the bounded local context for this direct edit. Do not repeat file_list/file_read unless validation names a new missing file. Return only JSON tool calls for the smallest source/test file_write or file_patch mutation next, then run requested validation."
                };
                let bootstrap_rule = native_tool_orchestration_prompt_text(
                    &self.metadata,
                    "bounded_direct_edit_bootstrap_rule",
                    default_bootstrap_rule,
                );
                let edit_owner_hint = native_tool_edit_owner_hint(&all_receipts);
                prompt = format!(
                    "{prompt}\n\n{bootstrap_rule}{edit_owner_hint}\n\nNative tool observations:\n{observation}"
                );
            }
        }
        let import_surface_validation_task = matches!(
            native_tool_validation_failure_class(
                &native_tool_latest_failed_validation_receipt_details(&all_receipts)
            )
            .as_deref(),
            Some("import_surface_missing")
        );
        let direct_existing_project_mutation_entry_armed =
            native_tool_direct_existing_project_mutation_entry_enabled(&self.metadata)
            && first_mutation_artifact_lane_v1_routes_lane(coding_task_lane)
            && native_tool_has_successful_read_context_receipt(&all_receipts)
            && !native_tool_has_successful_mutation(&all_receipts)
            && native_tool_mutation_only_recovery_enabled(&self.metadata)
            && native_tool_compact_mutation_entry_packet_enabled(&self.metadata)
            && !multi_requirement_validation_task
            && !import_surface_validation_task;
        if direct_existing_project_mutation_entry_armed {
            let use_validation_guided_repair =
                native_tool_validation_guided_compact_repair_enabled(&self.metadata)
                    && native_tool_has_failed_validation_command(&all_receipts);
            if use_validation_guided_repair {
                validation_guided_compact_repair_pending = true;
                validation_guided_compact_repair_reason =
                    Some("direct_existing_project_mutation_entry_failed_validation".to_string());
            } else {
                mutation_only_recovery_pending = true;
                mutation_only_recovery_reason =
                    Some("direct_existing_project_mutation_entry".to_string());
            }
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "direct_existing_project_mutation_entry_armed",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "coding_task_lane": coding_task_lane,
                    "receipt_count": all_receipts.len(),
                    "recovery_type": if use_validation_guided_repair { "validation_guided_compact_repair" } else { "mutation_only_recovery" },
                    "skips_speculative_first_mutation_artifact_lane": true,
                }),
            );
        }
        if first_mutation_artifact_lane_v1_enabled(&self.metadata)
            && !direct_existing_project_mutation_entry_armed
            && first_mutation_artifact_lane_v1_routes_lane(coding_task_lane)
            && native_tool_has_successful_read_context_receipt(&all_receipts)
            && !native_tool_has_successful_mutation(&all_receipts)
        {
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "first_mutation_artifact_lane_v1_start",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "coding_task_lane": coding_task_lane,
                    "receipt_count": all_receipts.len(),
                }),
            );
            provider_call_count += 1;
            let lane_response = provider.complete(&ProviderRequest {
                prompt: first_mutation_artifact_lane_v1_prompt(&self.initial_prompt, &all_receipts),
                system: Some(first_mutation_artifact_lane_v1_system()),
                tools: first_mutation_artifact_lane_v1_tools(tools),
                model: self.model.clone(),
                metadata: first_mutation_artifact_lane_v1_metadata(&self.metadata),
            });
            let lane_response = match lane_response {
                Ok(response) => response,
                Err(error) if native_tool_provider_error_is_timeout(&error) => {
                    let response = native_tool_partial_progress_response(
                        provider.provider_id(),
                        self.model.as_deref(),
                        "first_mutation_artifact_lane_v1_timeout",
                        provider_call_count,
                        &all_receipts,
                    );
                    native_tool_persist_run_journal(
                        &self.metadata,
                        &self.initial_prompt,
                        "first_mutation_artifact_lane_v1_timeout",
                        provider_call_count,
                        &all_receipts,
                        None,
                        Some(error.message.as_str()),
                    );
                    native_tool_persist_runtime_timeline_event(
                        &self.metadata,
                        &self.initial_prompt,
                        "first_mutation_artifact_lane_v1_timeout_demoted",
                        native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                        json!({
                            "provider_call_count": provider_call_count,
                            "receipt_count": all_receipts.len(),
                            "demoted_to_parent_runtime_loop": true,
                        }),
                    );
                    prompt.push_str("\n\nFirst mutation fast lane evidence:\n- The bounded first-mutation lane timed out before emitting a successful mutation receipt.\n- Continue in the parent native tool loop using the already loaded context.\n- Do not repeat discovery before the next mutation unless a later validation receipt names a new missing file.\n- Return file_patch or file_write before final output.");
                    response
                }
                Err(error) => return Err(error),
            };
            let lane_calls = parse_native_tool_calls(&lane_response.output);
            let lane_receipts = native_tool_dispatch_first_mutation_artifact_lane_calls(
                &dispatcher,
                &self.initial_prompt,
                lane_calls,
                native_tool_max_calls_per_turn(&self.metadata),
            );
            all_receipts.extend(lane_receipts.clone());
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "first_mutation_artifact_lane_v1_end",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "provider_call_count": provider_call_count,
                    "receipt_count": lane_receipts.len(),
                    "successful_mutation_in_batch": native_tool_has_successful_mutation(&lane_receipts),
                }),
            );
            native_tool_persist_run_journal(
                &self.metadata,
                &self.initial_prompt,
                "first_mutation_artifact_lane_v1",
                provider_call_count,
                &all_receipts,
                Some(&lane_response.output),
                None,
            );
            if native_tool_has_successful_mutation(&lane_receipts) {
                if let Some(validation_receipt) =
                    native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
                {
                    all_receipts.push(validation_receipt);
                    native_tool_persist_run_journal(
                        &self.metadata,
                        &self.initial_prompt,
                        "first_mutation_artifact_lane_v1_auto_validation",
                        provider_call_count,
                        &all_receipts,
                        Some(&lane_response.output),
                        None,
                    );
                }
                let spine_decision = coding_execution_spine_decision_from_native_receipts(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                if matches!(spine_decision.action, CodingSpineAction::CloseSuccess) {
                    let mut response = native_tool_synthetic_completion_evidence_response(
                        &lane_response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        "first_mutation_artifact_lane_v1_spine_closed_success",
                    );
                    response.raw = json!({
                        "provider_raw": response.raw,
                        "native_tool_loop": {
                            "enabled": true,
                            "provider_call_count": provider_call_count,
                            "tool_call_count": all_receipts.len(),
                            "empty_tool_retry_count": empty_tool_retry_count,
                            "coding_task_lane": coding_task_lane,
                            "tool_receipts": all_receipts.clone(),
                            "terminal_status": "ok",
                            "first_mutation_artifact_lane_v1": true,
                            "coding_execution_spine_v1": spine_decision,
                        }
                    });
                    return Ok((
                        response,
                        all_receipts,
                        provider_call_count,
                        "ok".to_string(),
                    ));
                }
            }
            if !native_tool_has_successful_mutation(&all_receipts) {
                let can_enter_compact_recovery = native_tool_mutation_only_recovery_enabled(&self.metadata)
                    && native_tool_compact_mutation_entry_packet_enabled(&self.metadata)
                    && (native_tool_has_successful_read_context_receipt(&all_receipts)
                        || native_tool_has_successful_context_receipt(&all_receipts)
                        || native_tool_has_any_validation_command(&all_receipts));
                if can_enter_compact_recovery {
                    let use_validation_guided_repair =
                        native_tool_validation_guided_compact_repair_enabled(&self.metadata)
                            && native_tool_has_failed_validation_command(&all_receipts);
                    if use_validation_guided_repair {
                        validation_guided_compact_repair_pending = true;
                        validation_guided_compact_repair_reason = Some(
                            "first_mutation_artifact_lane_v1_no_successful_mutation_after_failed_validation"
                                .to_string(),
                        );
                    } else {
                        mutation_only_recovery_pending = true;
                        mutation_only_recovery_reason = Some(
                            "first_mutation_artifact_lane_v1_no_successful_mutation".to_string(),
                        );
                    }
                    native_tool_persist_runtime_timeline_event(
                        &self.metadata,
                        &self.initial_prompt,
                        "first_mutation_artifact_lane_v1_compact_recovery_armed",
                        native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                        json!({
                            "provider_call_count": provider_call_count,
                            "receipt_count": all_receipts.len(),
                            "reason": "first_mutation_artifact_lane_v1_no_successful_mutation",
                            "recovery_type": if use_validation_guided_repair { "validation_guided_compact_repair" } else { "mutation_only_recovery" },
                        }),
                    );
                } else {
                    native_tool_persist_runtime_timeline_event(
                        &self.metadata,
                        &self.initial_prompt,
                        "first_mutation_artifact_lane_v1_parent_loop_armed",
                        native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                        json!({
                            "provider_call_count": provider_call_count,
                            "receipt_count": all_receipts.len(),
                            "reason": "first_mutation_artifact_lane_v1_no_successful_mutation",
                            "recovery_type": "parent_native_tool_loop",
                        }),
                    );
                }
            }
        }
        if native_tool_checkpointed_project_operator_tool_loop_active(
            &self.metadata,
            &self.initial_prompt,
        ) && !native_tool_has_successful_memory_context_command(&all_receipts)
        {
            let memory_context_started = Instant::now();
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "checkpoint_memory_context_start",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({ "stage": "checkpoint_memory_context" }),
            );
            if let Some(memory_receipt) =
                native_tool_auto_memory_context_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
            {
                native_tool_persist_runtime_timeline_event(
                    &self.metadata,
                    &self.initial_prompt,
                    "checkpoint_memory_context_end",
                    native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                    json!({
                        "duration_ms": native_tool_bounded_patch_elapsed_ms(memory_context_started),
                        "status": memory_receipt.status,
                        "success": memory_receipt.result.get("success").cloned().unwrap_or(Value::Null),
                    }),
                );
                all_receipts.push(memory_receipt);
                native_tool_persist_run_journal(
                    &self.metadata,
                    &self.initial_prompt,
                    "checkpoint_memory_context",
                    provider_call_count,
                    &all_receipts,
                    None,
                    None,
                );
                if native_tool_compact_mutation_entry_packet_enabled(&self.metadata)
                    && !native_tool_has_successful_mutation(&all_receipts)
                {
                    let mutation_packet = native_tool_mutation_entry_packet(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    );
                    prompt = native_tool_context_to_mutation_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        "",
                        &mutation_packet,
                        0,
                    );
                }
            } else {
                native_tool_persist_runtime_timeline_event(
                    &self.metadata,
                    &self.initial_prompt,
                    "checkpoint_memory_context_skipped",
                    native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                    json!({
                        "duration_ms": native_tool_bounded_patch_elapsed_ms(memory_context_started),
                    }),
                );
            }
        }
        if !micro_direct_write_task && native_tool_requires_successful_mutation(&self.metadata) {
            if bounded_direct_edit_task {
                // The direct edit lane deliberately skips pre-mutation artifact synthesis.
                // It reuses the native tool loop below so small bounded edits can mutate
                // through receipt-backed file tools instead of timing out before mutation.
            } else if native_tool_bounded_patch_artifact_lane_enabled(&self.metadata) {
                let outcome = native_tool_bounded_patch_artifact_lane(
                    &provider,
                    &dispatcher,
                    tools,
                    self.model.clone(),
                    &self.metadata,
                    &self.initial_prompt,
                    &system,
                )?;
                if !outcome.observability_receipts.is_empty() {
                    all_receipts.extend(outcome.observability_receipts);
                    let observation = native_tool_observation_prompt(&all_receipts);
                    let bounded_patch_fallback_rule = native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "bounded_patch_artifact_fallback_continuation_rule",
                        "The bounded patch artifact lane attempted this task and did not reach terminal success. Continue from the receipt-backed observations below. If patches already mutated files but validation failed, repair from the failed validation output instead of restarting discovery. Return only JSON tool calls next.",
                    );
                    prompt = format!(
                        "{}\n\n{}\n\nNative tool observations:\n{}",
                        self.initial_prompt, bounded_patch_fallback_rule, observation
                    );
                }
                if let Some((response, terminal_receipts, provider_call_count, provider_id)) =
                    outcome.terminal
                {
                    if all_receipts.is_empty() {
                        return Ok((
                            response,
                            terminal_receipts,
                            provider_call_count,
                            provider_id,
                        ));
                    }
                    let mut merged_receipts = all_receipts.clone();
                    for receipt in terminal_receipts {
                        if !merged_receipts
                            .iter()
                            .any(|existing| existing.call_id == receipt.call_id)
                        {
                            merged_receipts.push(receipt);
                        }
                    }
                    return Ok((response, merged_receipts, provider_call_count, provider_id));
                }
            } else {
                all_receipts.push(native_tool_bounded_patch_artifact_marker_receipt(
                    "flag_not_loaded",
                    json!({
                        "enabled": false,
                        "reason": "bounded_patch_artifact_lane_flag_missing_or_false",
                    }),
                ));
            }
        }
        if !micro_direct_write_task
            && all_receipts.is_empty()
            && native_tool_bootstrap_context_before_first_provider(&self.metadata)
            && native_tool_requires_successful_mutation(&self.metadata)
            && native_tool_prompt_has_multiple_requirements(&self.initial_prompt)
        {
            let bootstrap_receipts =
                native_tool_bootstrap_context_receipts(&dispatcher, &self.initial_prompt);
            if !bootstrap_receipts.is_empty() {
                let observation = native_tool_observation_prompt(&bootstrap_receipts);
                all_receipts.extend(bootstrap_receipts);
                native_tool_persist_run_journal(
                    &self.metadata,
                    &self.initial_prompt,
                    "bootstrap_context",
                    provider_call_count,
                    &all_receipts,
                    None,
                    None,
                );
                let bootstrap_rule = if bounded_direct_edit_task {
                    native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "bounded_direct_edit_bootstrap_continuation_rule",
                        "The bounded direct edit lane has already read the relevant local files. Return only JSON tool calls next. Prefer the smallest file_patch/file_write mutations to source and test files, then run requested validation/probe commands. Do not ask for more context unless a receipt shows the task is blocked.",
                    )
                } else {
                    native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "bootstrap_context_continuation_rule",
                        "Runtime bootstrap context was collected before the first model call. Continue from this already-read context and return only JSON tool calls next. If the source/test files needed for this bounded edit are already present in the observations, do not repeat file_list/file_read; make the smallest file_write/file_patch mutation, then validate when requested.",
                    )
                };
                if native_tool_compact_mutation_entry_packet_enabled(&self.metadata) {
                    let mutation_packet = native_tool_mutation_entry_packet(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    );
                    prompt = native_tool_context_to_mutation_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        "",
                        &mutation_packet,
                        0,
                    );
                } else {
                    prompt = format!(
                        "{}\n\n{}\n\nNative tool observations:\n{}",
                        self.initial_prompt, bootstrap_rule, observation
                    );
                }
            }
        }
        if native_tool_checkpointed_project_operator_tool_loop_active(
            &self.metadata,
            &self.initial_prompt,
        ) && !native_tool_has_successful_mutation(&all_receipts)
        {
            if let Some(stage) = native_tool_current_live_stage(&self.initial_prompt, &all_receipts) {
                let observation = native_tool_observation_prompt(&all_receipts);
                prompt = native_tool_checkpointed_project_stage_prompt(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    stage,
                    &observation,
                    1,
                );
                native_tool_persist_runtime_timeline_event(
                    &self.metadata,
                    &self.initial_prompt,
                    "project_operator_context_packet_ready",
                    native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                    json!({
                        "stage": stage,
                        "receipt_count": all_receipts.len(),
                    }),
                );
                if native_tool_project_operator_first_mutation_lane_enabled(&self.metadata, stage) {
                    let mut lane_metadata = self.metadata.clone();
                    if let Some(object) = lane_metadata.as_object_mut() {
                        object.insert(
                            "provider_timeout_seconds".to_string(),
                            json!(native_tool_project_operator_first_mutation_provider_timeout_seconds(
                                &self.metadata,
                            )),
                        );
                        object.insert("provider_stream_until_tool_calls".to_string(), json!(true));
                        object.insert("omit_ollama_thinking_flags".to_string(), json!(true));
                    }
                    provider_call_count += 1;
                    let lane_request = ProviderRequest {
                        prompt: native_tool_project_operator_first_mutation_prompt(
                            &prompt,
                            stage,
                        ),
                        system: Some(native_tool_project_operator_first_mutation_system()),
                        tools: tools.to_vec(),
                        model: self.model.clone(),
                        metadata: lane_metadata,
                    };
                    match provider.stream_complete(&lane_request) {
                        Ok(stream) => {
                            let lane_response = stream.response;
                            let mut lane_receipts = Vec::new();
                            for call in parse_native_tool_calls(&lane_response.output)
                                .into_iter()
                                .take(native_tool_project_operator_first_mutation_max_calls(
                                    &self.metadata,
                                ))
                            {
                                let receipt = if let Some(blocked) =
                                    native_tool_live_stage_blocked_receipt(
                                        &self.metadata,
                                        &self.initial_prompt,
                                        &all_receipts,
                                        &call,
                                    ) {
                                    blocked
                                } else {
                                    dispatcher.dispatch(native_tool_call_with_prompt_defaults(
                                        call,
                                        &self.initial_prompt,
                                    ))
                                };
                                lane_receipts.push(receipt.clone());
                                all_receipts.push(receipt);
                            }
                            native_tool_persist_run_journal(
                                &self.metadata,
                                &self.initial_prompt,
                                "project_operator_first_mutation_artifact_lane",
                                provider_call_count,
                                &all_receipts,
                                Some(&lane_response.output),
                                None,
                            );
                            native_tool_persist_runtime_timeline_event(
                                &self.metadata,
                                &self.initial_prompt,
                                "project_operator_first_mutation_artifact_lane_end",
                                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                                json!({
                                    "stage": stage,
                                    "receipt_count": lane_receipts.len(),
                                    "successful_mutation": native_tool_has_successful_mutation(&all_receipts),
                                }),
                            );
                            if !lane_receipts.is_empty() {
                                let observation = native_tool_observation_prompt(&lane_receipts);
                                prompt = format!(
                                    "{}\n\nFirst mutation artifact lane observations:\n{}",
                                    native_tool_checkpointed_project_stage_prompt(
                                        &self.metadata,
                                        &self.initial_prompt,
                                        &all_receipts,
                                        native_tool_current_live_stage(
                                            &self.initial_prompt,
                                            &all_receipts,
                                        )
                                        .unwrap_or(stage),
                                        &native_tool_observation_prompt(&all_receipts),
                                        1,
                                    ),
                                    observation
                                );
                            }
                        }
                        Err(error) if native_tool_provider_error_is_timeout(&error) => {
                            native_tool_persist_runtime_timeline_event(
                                &self.metadata,
                                &self.initial_prompt,
                                "project_operator_first_mutation_artifact_lane_timeout",
                                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                                json!({
                                    "stage": stage,
                                    "error": error.message,
                                }),
                            );
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }

        for turn_idx in 0..max_turns {
            let active_wall_timeout =
                if validation_guided_compact_repair_pending
                    || mutation_only_recovery_pending
                    || (native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                        && native_tool_has_failed_validation_command(&all_receipts)
                        && !native_tool_has_successful_validation_after_latest_mutation(&all_receipts))
                {
                    native_tool_recovery_wall_timeout(&self.metadata).or(wall_timeout)
                } else {
                    wall_timeout
                };
            if let Some(timeout) = active_wall_timeout {
                if loop_started.elapsed() >= timeout {
                    if native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                        && native_tool_partial_progress_on_timeout(&self.metadata)
                    {
                        return native_tool_recovery_or_partial_progress(
                            &provider,
                            &dispatcher,
                            tools,
                            self.model.clone(),
                            &self.metadata,
                            &self.initial_prompt,
                            &system,
                            "native_tool_loop_wall_timeout",
                            provider_call_count,
                            all_receipts,
                        );
                    }
                    return Err(ProviderError::new(
                        ProviderErrorCode::Timeout,
                        format!(
                            "native_tool_loop_wall_timeout:timeout_seconds={}",
                            timeout.as_secs()
                        ),
                    ));
                }
            }
            provider_call_count += 1;
            let has_non_import_surface_mutation =
                native_tool_has_successful_non_import_surface_mutation(&all_receipts);
            let regular_staged_edit_turn = native_tool_staged_edit_controller_enabled(
                &self.metadata,
                bounded_direct_edit_task,
            ) && !native_tool_has_successful_mutation(&all_receipts);
            let seed_prepared_staged_edit_turn =
                native_tool_seed_prepared_staged_controller_enabled(&self.metadata)
                    && native_tool_python_import_surface_seed_source_receipt_count(&all_receipts)
                        >= native_tool_seed_prepared_staged_controller_min_source_seed_receipts(
                            &self.metadata,
                        )
                    && !has_non_import_surface_mutation;
            let staged_edit_turn = (regular_staged_edit_turn || seed_prepared_staged_edit_turn)
                && turn_idx == 0
                && native_tool_has_successful_read_context_receipt(&all_receipts);
            let first_edit_batch_turn = first_edit_batch_contract
                && native_tool_has_successful_read_context_receipt(&all_receipts)
                && !native_tool_has_successful_mutation(&all_receipts);
            let mutation_only_recovery_turn =
                mutation_only_recovery_pending
                    && native_tool_mutation_only_recovery_enabled(&self.metadata)
                    && !has_non_import_surface_mutation;
            let validation_guided_compact_repair_turn =
                validation_guided_compact_repair_pending
                    && native_tool_validation_guided_compact_repair_enabled(&self.metadata)
                    && !native_tool_has_successful_validation_after_latest_mutation(&all_receipts);
            let seeded_import_surface_repair_turn =
                native_tool_seeded_import_surface_compact_repair_enabled(&self.metadata)
                    && native_tool_has_python_import_surface_seed_receipt(&all_receipts)
                    && !mutation_only_recovery_pending
                    && !has_non_import_surface_mutation
                    && !native_tool_has_successful_validation_after_latest_mutation(&all_receipts)
                    && matches!(
                        native_tool_validation_failure_class(
                            &native_tool_latest_failed_validation_receipt_details(&all_receipts)
                        )
                        .as_deref(),
                        Some("import_surface_missing")
                    );
            let compact_bootstrap_mutation_turn =
                native_tool_compact_mutation_entry_packet_enabled(&self.metadata)
                    && native_tool_requires_successful_mutation(&self.metadata)
                    && !native_tool_prompt_requires_pre_mutation_validation(&self.initial_prompt)
                    && native_tool_has_successful_read_context_receipt(&all_receipts)
                    && !native_tool_has_successful_mutation(&all_receipts)
                    && !seeded_import_surface_repair_turn
                    && !validation_guided_compact_repair_turn
                    && !first_edit_batch_turn
                    && !staged_edit_turn
                    && !mutation_only_recovery_turn;
            let compact_action_controller_turn =
                native_tool_compact_action_controller_enabled(&self.metadata)
                    && native_tool_controlled_shell_edit_batch_enabled(&self.metadata)
                    && (mutation_only_recovery_turn || compact_bootstrap_mutation_turn);
            let validation_guided_compact_repair_reason_for_turn =
                if validation_guided_compact_repair_turn {
                    validation_guided_compact_repair_pending = false;
                    validation_guided_compact_repair_reason.take().unwrap_or_else(|| {
                        "failed_validation_before_required_mutation".to_string()
                    })
                } else {
                    String::new()
                };
            let mutation_only_recovery_reason_for_turn = if mutation_only_recovery_turn {
                mutation_only_recovery_pending = false;
                mutation_only_recovery_reason
                    .take()
                    .unwrap_or_else(|| "mutation_only_recovery_before_required_mutation".to_string())
            } else {
                String::new()
            };
            let mut request_metadata = self.metadata.clone();
            let mut provider_turn_timeout_seconds =
                native_tool_provider_turn_timeout_seconds(
                    &self.metadata,
                    &self.initial_prompt,
                    turn_idx,
                );
            if validation_guided_compact_repair_turn {
                provider_turn_timeout_seconds =
                    native_tool_validation_guided_compact_repair_provider_timeout_seconds(
                        &self.metadata,
                    );
            }
            if mutation_only_recovery_turn {
                provider_turn_timeout_seconds =
                    native_tool_mutation_only_recovery_provider_timeout_seconds(&self.metadata);
            }
            let seed_prepared_high_fanout_implementation_turn =
                native_tool_python_import_surface_seed_source_receipt_count(&all_receipts)
                    >= native_tool_seed_prepared_high_fanout_min_source_seed_receipts(
                        &self.metadata,
                    )
                    && !has_non_import_surface_mutation
                    && !staged_edit_turn
                    && !validation_guided_compact_repair_turn
                    && !mutation_only_recovery_turn
                    && !seeded_import_surface_repair_turn;
            if seed_prepared_high_fanout_implementation_turn {
                provider_turn_timeout_seconds =
                    native_tool_seed_prepared_high_fanout_provider_timeout_seconds(&self.metadata);
            }
            if native_tool_first_receipt_watchdog_enabled(&self.metadata, &self.initial_prompt)
                && provider_call_count <= 1
                && !native_tool_has_successful_mutation(&all_receipts)
            {
                provider_turn_timeout_seconds = provider_turn_timeout_seconds.min(
                    native_tool_first_receipt_deadline_seconds(&self.metadata, &self.initial_prompt),
                );
            }
            if let Value::Object(object) = &mut request_metadata {
                object
                    .entry("provider_timeout_seconds".to_string())
                    .or_insert_with(|| json!(provider_turn_timeout_seconds));
                if validation_guided_compact_repair_turn
                    || mutation_only_recovery_turn
                    || compact_action_controller_turn
                {
                    object.insert("omit_ollama_thinking_flags".to_string(), json!(true));
                }
            }
            if let Some(timeout_seconds) = next_provider_timeout_seconds.take() {
                if let Value::Object(object) = &mut request_metadata {
                    let timeout_seconds = if validation_guided_compact_repair_turn
                        || mutation_only_recovery_turn
                    {
                        timeout_seconds.max(provider_turn_timeout_seconds)
                    } else {
                        timeout_seconds
                    };
                    object.insert(
                        "provider_timeout_seconds".to_string(),
                        json!(timeout_seconds),
                    );
                }
            }
            let mut stream_until_tool_calls =
                native_tool_stream_until_tool_calls_enabled(&request_metadata, bounded_direct_edit_task);
            if compact_action_controller_turn {
                stream_until_tool_calls = false;
            }
            if stream_until_tool_calls {
                if let Value::Object(object) = &mut request_metadata {
                    object.insert("provider_stream_until_tool_calls".to_string(), json!(true));
                }
            }
            let request_prompt = if seeded_import_surface_repair_turn {
                    native_tool_seeded_import_surface_repair_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    )
                } else if validation_guided_compact_repair_turn {
                    native_tool_validation_guided_compact_repair_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &validation_guided_compact_repair_reason_for_turn,
                        &all_receipts,
                    )
                } else if compact_action_controller_turn {
                    let mutation_packet =
                        native_tool_mutation_entry_packet(&self.metadata, &self.initial_prompt, &all_receipts);
                    native_tool_compact_action_controller_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        if mutation_only_recovery_turn {
                            &mutation_only_recovery_reason_for_turn
                        } else {
                            "compact_bootstrap_mutation_before_required_mutation"
                        },
                        &mutation_packet,
                    )
                } else if mutation_only_recovery_turn {
                    let mutation_packet =
                        native_tool_mutation_entry_packet(&self.metadata, &self.initial_prompt, &all_receipts);
                    native_tool_mutation_only_recovery_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &mutation_only_recovery_reason_for_turn,
                        &mutation_packet,
                    )
                } else if compact_bootstrap_mutation_turn {
                    let mutation_packet = native_tool_mutation_entry_packet(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    );
                    native_tool_context_to_mutation_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        "",
                        &mutation_packet,
                        turn_idx as u64,
                    )
                } else if first_edit_batch_turn {
                    native_tool_first_edit_batch_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    )
                } else if staged_edit_turn {
                    native_tool_staged_edit_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    )
                } else {
                    prompt.clone()
                };
            let request_system = if seeded_import_surface_repair_turn {
                native_tool_seeded_import_surface_repair_system()
            } else if validation_guided_compact_repair_turn {
                native_tool_validation_guided_compact_repair_system()
            } else if compact_action_controller_turn {
                native_tool_compact_action_controller_system()
            } else if mutation_only_recovery_turn || compact_bootstrap_mutation_turn {
                native_tool_mutation_only_recovery_system()
            } else if first_edit_batch_turn {
                native_tool_first_edit_batch_system()
            } else if staged_edit_turn {
                native_tool_staged_edit_system()
            } else {
                system.clone()
            };
            let request_tools = if seeded_import_surface_repair_turn {
                native_tool_staged_edit_tools(tools)
            } else if validation_guided_compact_repair_turn {
                native_tool_mutation_recovery_tools(&self.metadata, tools)
            } else if compact_action_controller_turn {
                native_tool_compact_action_controller_tools(tools)
            } else if mutation_only_recovery_turn || compact_bootstrap_mutation_turn {
                native_tool_mutation_recovery_tools(&self.metadata, tools)
            } else if first_edit_batch_turn {
                native_tool_bounded_fast_edit_preflight_tools(tools)
            } else if staged_edit_turn {
                native_tool_staged_edit_tools(tools)
            } else {
                tools.to_vec()
            };
            let request_observation_chars =
                native_tool_observation_prompt(&all_receipts).chars().count();
            let request = ProviderRequest {
                prompt: request_prompt,
                system: Some(request_system),
                tools: request_tools.clone(),
                model: self.model.clone(),
                metadata: request_metadata,
            };
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "provider_request_start",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "provider_call_count": provider_call_count,
                    "turn_idx": turn_idx,
                    "prompt_chars": request.prompt.chars().count(),
                    "system_chars": request.system.as_ref().map(|value| value.chars().count()).unwrap_or(0),
                    "tool_count": request.tools.len(),
                    "stream_until_tool_calls": stream_until_tool_calls,
                    "staged_edit_turn": staged_edit_turn,
                    "seeded_import_surface_repair_turn": seeded_import_surface_repair_turn,
                    "validation_guided_compact_repair_turn": validation_guided_compact_repair_turn,
                    "mutation_only_recovery_turn": mutation_only_recovery_turn,
                    "compact_action_controller_turn": compact_action_controller_turn,
                }),
            );
            let provider_turn_started = Instant::now();
            let provider_result = if stream_until_tool_calls {
                provider.stream_complete(&request).map(|stream| {
                    let mut response = stream.response;
                    response.raw = json!({
                        "provider_raw": response.raw,
                        "provider_stream": {
                            "enabled": true,
                            "event_count": stream.events.len(),
                            "stopped_early": stream.stopped_early,
                            "stop_reason": stream.stop_reason,
                        }
                    });
                    response
                })
            } else {
                provider.complete(&request)
            };
            let provider_turn_latency_ms =
                native_tool_bounded_patch_elapsed_ms(provider_turn_started);
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "provider_request_end",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "provider_call_count": provider_call_count,
                    "turn_idx": turn_idx,
                    "duration_ms": provider_turn_latency_ms,
                    "status": if provider_result.is_ok() { "ok" } else { "error" },
                    "stream_until_tool_calls": stream_until_tool_calls,
                    "staged_edit_turn": staged_edit_turn,
                    "seeded_import_surface_repair_turn": seeded_import_surface_repair_turn,
                    "validation_guided_compact_repair_turn": validation_guided_compact_repair_turn,
                    "mutation_only_recovery_turn": mutation_only_recovery_turn,
                    "stream_diagnostics": provider_result
                        .as_ref()
                        .ok()
                        .and_then(|response| response.raw.pointer("/provider_raw/stream_diagnostics").cloned())
                        .unwrap_or(Value::Null),
                    "error_preview": provider_result
                        .as_ref()
                        .err()
                        .map(|error| error.message.chars().take(500).collect::<String>())
                        .unwrap_or_default(),
                }),
            );
            native_tool_persist_provider_turn_timing_probe(
                &self.metadata,
                &self.initial_prompt,
                provider_call_count,
                turn_idx,
                &request,
                request_observation_chars,
                provider_turn_latency_ms,
                if provider_result.is_ok() { "ok" } else { "error" },
                provider_result
                    .as_ref()
                    .err()
                    .map(|error| error.message.as_str()),
            );
            let response = match provider_result {
                Ok(response) => response,
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                        && native_tool_partial_progress_on_timeout(&self.metadata) =>
                {
                    return native_tool_recovery_or_partial_progress(
                        &provider,
                        &dispatcher,
                        tools,
                        self.model.clone(),
                        &self.metadata,
                        &self.initial_prompt,
                        &system,
                        error.message.as_str(),
                        provider_call_count,
                        all_receipts,
                    );
                }
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && !micro_direct_write_task
                        && !native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                        && provider_call_count <= 1
                        && native_tool_first_receipt_watchdog_enabled(
                            &self.metadata,
                            &self.initial_prompt,
                        ) =>
                {
                    if all_receipts.is_empty() {
                        let bootstrap_receipts = native_tool_bootstrap_context_receipts(
                            &dispatcher,
                            &self.initial_prompt,
                        );
                        if !bootstrap_receipts.is_empty() {
                            all_receipts.extend(bootstrap_receipts);
                            native_tool_persist_run_journal(
                                &self.metadata,
                                &self.initial_prompt,
                                "first_turn_timeout_bootstrap_context",
                                provider_call_count,
                                &all_receipts,
                                None,
                                None,
                            );
                        }
                    }
                    if native_tool_has_successful_read_context_receipt(&all_receipts)
                        || native_tool_has_successful_context_receipt(&all_receipts)
                    {
                        let timeout_reason = format!(
                            "first_turn_timeout_before_required_mutation:{}",
                            error.message.chars().take(180).collect::<String>()
                        );
                        if validation_guided_compact_repair_turn
                            || (native_tool_validation_guided_compact_repair_enabled(
                                &self.metadata,
                            ) && native_tool_has_failed_validation_command(&all_receipts))
                        {
                            validation_guided_compact_repair_pending = true;
                            validation_guided_compact_repair_reason =
                                Some(timeout_reason);
                        } else {
                            let observation = native_tool_observation_prompt(&all_receipts);
                            prompt = native_tool_first_turn_timeout_recovery_prompt(
                                &self.metadata,
                                &self.initial_prompt,
                                error.message.as_str(),
                                &observation,
                            );
                            mutation_only_recovery_pending = true;
                            mutation_only_recovery_reason = Some(timeout_reason);
                        }
                        if bounded_direct_edit_task
                            && native_tool_prompt_requires_pre_mutation_validation(
                                &self.initial_prompt,
                            )
                            && native_tool_has_any_validation_command(&all_receipts)
                        {
                            next_provider_timeout_seconds =
                                Some(native_tool_bounded_patch_artifact_provider_timeout_seconds(
                                    &self.metadata,
                                ));
                        }
                        native_tool_persist_run_journal(
                            &self.metadata,
                            &self.initial_prompt,
                            "first_turn_timeout_recovery",
                            provider_call_count,
                            &all_receipts,
                            None,
                            Some(error.message.as_str()),
                        );
                        continue;
                    }
                    if let Some(bootstrap_receipt) =
                        native_tool_bootstrap_discovery_receipt(&dispatcher, &self.initial_prompt)
                    {
                        let observation =
                            native_tool_observation_prompt(&[bootstrap_receipt.clone()]);
                        all_receipts.push(bootstrap_receipt);
                        native_tool_persist_run_journal(
                            &self.metadata,
                            &self.initial_prompt,
                            "first_turn_timeout_discovery",
                            provider_call_count,
                            &all_receipts,
                            None,
                            Some(error.message.as_str()),
                        );
                        let bootstrap_rule = native_tool_orchestration_prompt_text(
                            &self.metadata,
                            "first_turn_timeout_discovery_rule",
                            "The first model call timed out before mutation. Runtime discovery is now available. Return only JSON tool calls for the smallest safe source/test mutation next; do not summarize or ask for more input unless local completion is genuinely blocked.",
                        );
                        prompt = format!(
                            "{}\n\n{}\n\nNative tool observations:\n{}",
                            self.initial_prompt, bootstrap_rule, observation
                        );
                        continue;
                    }
                    return Err(error);
                }
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && !all_receipts.is_empty()
                        && native_tool_partial_progress_on_timeout(&self.metadata) =>
                {
                    return Ok((
                        native_tool_partial_progress_response(
                            provider.provider_id(),
                            self.model.as_deref(),
                            error.message.as_str(),
                            provider_call_count,
                            &all_receipts,
                        ),
                        all_receipts,
                        provider_call_count,
                        "partial_timeout".to_string(),
                    ));
                }
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && !micro_direct_write_task
                        && all_receipts.is_empty()
                        && native_tool_requires_successful_mutation(&self.metadata)
                        && native_tool_prompt_has_multiple_requirements(&self.initial_prompt) =>
                {
                    if let Some(bootstrap_receipt) =
                        native_tool_bootstrap_discovery_receipt(&dispatcher, &self.initial_prompt)
                    {
                        let observation =
                            native_tool_observation_prompt(&[bootstrap_receipt.clone()]);
                        all_receipts.push(bootstrap_receipt);
                        native_tool_persist_run_journal(
                            &self.metadata,
                            &self.initial_prompt,
                            "bootstrap_timeout_discovery",
                            provider_call_count,
                            &all_receipts,
                            None,
                            None,
                        );
                        let bootstrap_rule = native_tool_orchestration_prompt_text(
                            &self.metadata,
                            "bootstrap_timeout_discovery_rule",
                            "Runtime bootstrap discovery was performed after an initial timeout. Continue from these observations and return only JSON tool calls next.",
                        );
                        prompt = format!(
                            "{}\n\n{}\n\nNative tool observations:\n{}",
                            self.initial_prompt, bootstrap_rule, observation
                        );
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
            let mut calls = parse_native_tool_calls(&response.output);
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "provider_response_tool_parse",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "provider_call_count": provider_call_count,
                    "turn_idx": turn_idx,
                    "parsed_call_count": calls.len(),
                    "parsed_tool_names": calls
                        .iter()
                        .map(|call| call.name.as_str())
                        .take(12)
                        .collect::<Vec<_>>(),
                    "output_chars": response.output.chars().count(),
                    "output_preview": if calls.is_empty() {
                        response.output.chars().take(800).collect::<String>()
                    } else {
                        String::new()
                    },
                    "staged_edit_turn": staged_edit_turn,
                    "seeded_import_surface_repair_turn": seeded_import_surface_repair_turn,
                    "validation_guided_compact_repair_turn": validation_guided_compact_repair_turn,
                    "mutation_only_recovery_turn": mutation_only_recovery_turn,
                }),
            );
            if bounded_direct_edit_task
                && native_tool_prompt_requires_pre_mutation_validation(&self.initial_prompt)
                && !native_tool_has_any_validation_command(&all_receipts)
            {
                native_tool_prioritize_pre_mutation_validation_calls(&mut calls);
            }
            if calls.is_empty() {
                if staged_edit_turn
                    && !native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                    && empty_tool_retry_count
                        < native_tool_seed_prepared_staged_empty_retry_limit(&self.metadata)
                {
                    empty_tool_retry_count += 1;
                    let mutation_packet = native_tool_mutation_entry_packet(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    );
                    prompt = native_tool_mutation_only_recovery_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        "staged_edit_no_tool_calls_before_implementation_mutation",
                        &mutation_packet,
                    );
                    next_provider_timeout_seconds =
                        Some(native_tool_seed_prepared_staged_retry_provider_timeout_seconds(
                            &self.metadata,
                        ));
                    mutation_only_recovery_pending = true;
                    mutation_only_recovery_reason =
                        Some("staged_edit_no_tool_calls_before_implementation_mutation".to_string());
                    last_response = Some(response);
                    continue;
                }
                if native_tool_checkpointed_project_has_live_stage(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                ) && empty_tool_retry_count < empty_tool_retry_limit
                {
                    empty_tool_retry_count += 1;
                    if let Some(stage) =
                        native_tool_current_live_stage(&self.initial_prompt, &all_receipts)
                    {
                        let observation = native_tool_observation_prompt(&all_receipts);
                        prompt = native_tool_checkpointed_project_stage_prompt(
                            &self.metadata,
                            &self.initial_prompt,
                            &all_receipts,
                            stage,
                            &format!(
                                "Previous response did not emit native tool calls:\n{}\n\nCurrent receipts:\n{}",
                                response.output,
                                observation
                            ),
                            empty_tool_retry_count as usize,
                        );
                        last_response = Some(response);
                        continue;
                    }
                }
                if all_receipts.is_empty() && empty_tool_retry_count < empty_tool_retry_limit {
                    empty_tool_retry_count += 1;
                    prompt = native_tool_empty_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &response.output,
                        empty_tool_retry_count,
                    );
                    last_response = Some(response);
                    continue;
                }
                if all_receipts.is_empty()
                    && !micro_direct_write_task
                    && native_tool_requires_successful_mutation(&self.metadata)
                    && native_tool_prompt_has_multiple_requirements(&self.initial_prompt)
                {
                    if let Some(bootstrap_receipt) =
                        native_tool_bootstrap_discovery_receipt(&dispatcher, &self.initial_prompt)
                    {
                        let observation =
                            native_tool_observation_prompt(&[bootstrap_receipt.clone()]);
                        all_receipts.push(bootstrap_receipt);
                        native_tool_persist_run_journal(
                            &self.metadata,
                            &self.initial_prompt,
                            "bootstrap_no_tool_discovery",
                            provider_call_count,
                            &all_receipts,
                            None,
                            None,
                        );
                        let bootstrap_rule = native_tool_orchestration_prompt_text(
                            &self.metadata,
                            "bootstrap_no_tool_discovery_rule",
                            "Runtime bootstrap discovery was performed because previous responses did not call native tools. Continue from these observations and return only JSON tool calls next.",
                        );
                        prompt = format!(
                            "{}\n\n{}\n\nNative tool observations:\n{}",
                            self.initial_prompt, bootstrap_rule, observation
                        );
                        last_response = Some(response);
                        continue;
                    }
                }
                if native_tool_requires_successful_mutation(&self.metadata)
                    && !native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                    && !all_receipts.is_empty()
                    && empty_tool_retry_count < empty_tool_retry_limit
                {
                    empty_tool_retry_count += 1;
                    let observation = if native_tool_compact_mutation_entry_packet_enabled(
                        &self.metadata,
                    ) {
                        native_tool_mutation_entry_packet(
                            &self.metadata,
                            &self.initial_prompt,
                            &all_receipts,
                        )
                    } else {
                        native_tool_observation_prompt(&all_receipts)
                    };
                    prompt = native_tool_context_to_mutation_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &response.output,
                        &observation,
                        empty_tool_retry_count,
                    );
                    last_response = Some(response);
                    continue;
                }
                last_response = Some(response);
                break;
            }
            let tool_dispatch_started = Instant::now();
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "tool_dispatch_batch_start",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "provider_call_count": provider_call_count,
                    "turn_idx": turn_idx,
                    "call_count": calls.len().min(native_tool_max_calls_per_turn(&self.metadata)),
                }),
            );
            let mut turn_receipts = Vec::new();
            for call in calls
                .into_iter()
                .take(native_tool_max_calls_per_turn(&self.metadata))
            {
                let call = native_tool_call_with_prompt_defaults(call, &self.initial_prompt);
                let context_blocked = native_tool_requires_successful_mutation(&self.metadata)
                    && !has_non_import_surface_mutation
                    && context_only_turn_count
                        >= native_tool_max_context_only_turns(&self.metadata)
                    && native_tool_has_successful_context_receipt(&all_receipts)
                    && native_tool_call_is_context_only(&call);
                let preflight_context_blocked = bounded_fast_edit_preflight
                    && native_tool_has_successful_read_context_receipt(&all_receipts)
                    && !has_non_import_surface_mutation
                    && native_tool_call_is_context_only(&call);
                let compact_mutation_context_blocked = compact_bootstrap_mutation_turn
                    && !has_non_import_surface_mutation
                    && native_tool_call_is_context_only(&call);
                let first_edit_batch_command_blocked = first_edit_batch_turn
                    && !has_non_import_surface_mutation
                    && !native_tool_has_successful_non_import_surface_mutation(&turn_receipts)
                    && native_tool_call_is_command_run(&call);
                let seeded_import_surface_context_blocked =
                    seeded_import_surface_repair_turn && native_tool_call_is_context_only(&call);
                let seeded_import_surface_command_blocked =
                    seeded_import_surface_repair_turn && native_tool_call_is_command_run(&call);
                let staged_edit_context_blocked =
                    staged_edit_turn && native_tool_call_is_context_only(&call);
                let staged_edit_command_blocked =
                    staged_edit_turn && native_tool_call_is_command_run(&call);
                let edit_owner_blocked = bounded_direct_edit_task
                    && !has_non_import_surface_mutation
                    && native_tool_call_is_product_mutation(&call)
                    && !native_tool_prompt_explicit_new_file_allowed(&self.initial_prompt)
                    && !native_tool_call_targets_observed_product_path(&call, &all_receipts);
                let seeded_import_surface_seed_path_write =
                    seeded_import_surface_repair_turn
                        && native_tool_call_targets_python_import_surface_seed_path(
                            &call,
                            &all_receipts,
                        );
                let placeholder_path_blocked =
                    native_tool_placeholder_path_blocked_receipt(&call, &all_receipts);
                let placeholder_content_blocked =
                    native_tool_placeholder_content_blocked_receipt(&call, &all_receipts);
                let undeclared_tool_blocked =
                    native_tool_undeclared_tool_blocked_receipt(&call, &request_tools);
                let shell_edit_paths = native_tool_controlled_shell_edit_batch_paths(
                    &self.metadata,
                    &self.initial_prompt,
                    &call,
                );
                let shell_edit_receipt_paths = if shell_edit_paths.is_empty() {
                    native_tool_shell_edit_receipt_synthesis_paths(&self.initial_prompt, &call)
                } else {
                    shell_edit_paths.clone()
                };
                let shell_edit_command = !shell_edit_paths.is_empty()
                    && !has_non_import_surface_mutation;
                let shell_edit_receipt_command = !shell_edit_receipt_paths.is_empty()
                    && !has_non_import_surface_mutation;
                let mutation_only_non_edit_command_blocked = mutation_only_recovery_turn
                    && !has_non_import_surface_mutation
                    && native_tool_call_is_command_run(&call)
                    && !shell_edit_command
                    && !shell_edit_receipt_command;
                let validation_guided_non_edit_command_blocked =
                    validation_guided_compact_repair_turn
                        && !native_tool_has_successful_non_import_surface_mutation(&turn_receipts)
                        && native_tool_call_is_command_run(&call)
                        && !shell_edit_command
                        && !shell_edit_receipt_command;
                let unresolved_owner_source_blocked =
                    native_tool_unresolved_owner_source_first_blocked_receipt(
                        &all_receipts,
                        &call,
                        &shell_edit_receipt_paths,
                    );
                let pre_dispatch_receipts = if shell_edit_receipt_command {
                    Some(all_receipts.clone())
                } else {
                    None
                };
                let shell_edit_python_shape_snapshots = if shell_edit_receipt_command {
                    native_tool_shell_edit_python_shape_snapshots(&shell_edit_receipt_paths)
                } else {
                    std::collections::BTreeMap::new()
                };
                let receipt = if let Some(blocked) = placeholder_path_blocked {
                    blocked
                } else if let Some(blocked) = placeholder_content_blocked {
                    blocked
                } else if let Some(blocked) = unresolved_owner_source_blocked {
                    blocked
                } else if let Some(blocked) = undeclared_tool_blocked {
                    blocked
                } else if mutation_only_non_edit_command_blocked {
                    native_tool_mutation_only_command_blocked_receipt(call)
                } else if validation_guided_non_edit_command_blocked {
                    native_tool_validation_guided_command_blocked_receipt(call)
                } else if !shell_edit_command {
                    if let Some(blocked) = native_tool_live_stage_blocked_receipt(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    &call,
                    ) {
                        blocked
                    } else if let Some(blocked) = native_tool_guarded_blocked_receipt(
                        seeded_import_surface_seed_path_write,
                        native_tool_preserved_api_write_blocked_receipt(&self.initial_prompt, &call),
                    )
                    {
                        blocked
                    } else if let Some(blocked) = native_tool_guarded_blocked_receipt(
                        seeded_import_surface_seed_path_write,
                        native_tool_python_existing_shape_blocked_receipt(&self.initial_prompt, &call),
                    )
                    {
                        blocked
                    } else if let Some(blocked) =
                        native_tool_atomic_dependent_edit_blocked_receipt(&all_receipts, &call)
                    {
                        blocked
                    } else if let Some(blocked) = native_tool_python_src_prefix_import_blocked_receipt(
                        &self.initial_prompt,
                        &call,
                    ) {
                        blocked
                    } else if compact_mutation_context_blocked {
                        native_tool_compact_mutation_context_blocked_receipt(call)
                    } else if preflight_context_blocked {
                        native_tool_bounded_fast_edit_preflight_context_blocked_receipt(call)
                    } else if first_edit_batch_command_blocked {
                        native_tool_first_edit_batch_command_blocked_receipt(call)
                    } else if seeded_import_surface_context_blocked {
                        native_tool_staged_edit_context_blocked_receipt(call)
                    } else if seeded_import_surface_command_blocked {
                        native_tool_staged_edit_command_blocked_receipt(call)
                    } else if staged_edit_context_blocked {
                        native_tool_staged_edit_context_blocked_receipt(call)
                    } else if staged_edit_command_blocked {
                        native_tool_staged_edit_command_blocked_receipt(call)
                    } else if edit_owner_blocked {
                        native_tool_edit_owner_blocked_receipt(call, &all_receipts)
                    } else if context_blocked {
                        native_tool_product_repair_context_blocked_receipt(call, &[], &all_receipts)
                    } else {
                        dispatcher.dispatch(call)
                    }
                } else if let Some(blocked) =
                    native_tool_shell_edit_batch_blocked_receipt(&call, &shell_edit_paths)
                {
                    blocked
                } else {
                    dispatcher.dispatch(call)
                };
                turn_receipts.push(receipt.clone());
                all_receipts.push(receipt.clone());
                if shell_edit_receipt_command && native_tool_command_receipt_success(&receipt) {
                    if let Some(blocked) = native_tool_shell_edit_python_existing_shape_blocked_receipt(
                        &receipt.call_id,
                        &shell_edit_python_shape_snapshots,
                    ) {
                        turn_receipts.push(blocked.clone());
                        all_receipts.push(blocked);
                        continue;
                    }
                    if let Some(blocked) = native_tool_shell_edit_preserved_api_blocked_receipt(
                        &receipt.call_id,
                        &self.initial_prompt,
                        &shell_edit_python_shape_snapshots,
                    ) {
                        turn_receipts.push(blocked.clone());
                        all_receipts.push(blocked);
                        continue;
                    }
                    let synthetic_receipts = native_tool_synthesize_shell_edit_mutation_receipts(
                        &receipt.call_id,
                        &shell_edit_receipt_paths,
                        pre_dispatch_receipts.as_deref().unwrap_or(&[]),
                    );
                    for synthetic in synthetic_receipts {
                        turn_receipts.push(synthetic.clone());
                        all_receipts.push(synthetic);
                    }
                }
            }
            native_tool_persist_runtime_timeline_event(
                &self.metadata,
                &self.initial_prompt,
                "tool_dispatch_batch_end",
                native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                json!({
                    "provider_call_count": provider_call_count,
                    "turn_idx": turn_idx,
                    "duration_ms": native_tool_bounded_patch_elapsed_ms(tool_dispatch_started),
                    "receipt_count": turn_receipts.len(),
                    "successful_mutation_in_batch": native_tool_has_successful_mutation(&turn_receipts),
                    "successful_validation_after_batch": native_tool_has_successful_validation_command(&all_receipts),
                }),
            );
            native_tool_persist_run_journal(
                &self.metadata,
                &self.initial_prompt,
                "tool_turn",
                provider_call_count,
                &all_receipts,
                Some(&response.output),
                None,
            );
            if coding_execution_spine_v1_enabled(&self.metadata)
                && native_tool_has_successful_mutation(&all_receipts)
            {
                let spine_decision = coding_execution_spine_decision_from_native_receipts(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                if matches!(spine_decision.action, CodingSpineAction::CloseSuccess) {
                    let mut response = native_tool_synthetic_completion_evidence_response(
                        &response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        "tool_loop_spine_closed_success",
                    );
                    response.raw = json!({
                        "provider_raw": response.raw,
                        "native_tool_loop": {
                            "enabled": true,
                            "provider_call_count": provider_call_count,
                            "tool_call_count": all_receipts.len(),
                            "empty_tool_retry_count": empty_tool_retry_count,
                            "coding_task_lane": coding_task_lane,
                            "tool_receipts": all_receipts.clone(),
                            "terminal_status": "ok",
                            "coding_execution_spine_v1": spine_decision,
                        }
                    });
                    return Ok((
                        response,
                        all_receipts,
                        provider_call_count,
                        "ok".to_string(),
                    ));
                }
            }
            if native_tool_requires_successful_mutation(&self.metadata)
                && !native_tool_has_successful_non_import_surface_mutation(&all_receipts)
                && native_tool_mutation_only_recovery_enabled(&self.metadata)
            {
                if let Some(reason) =
                    native_tool_turn_required_mutation_blocker_reason(&turn_receipts)
                {
                    if native_tool_validation_guided_compact_repair_enabled(&self.metadata)
                        && native_tool_has_failed_validation_command(&all_receipts)
                        && !native_tool_reason_requires_context_to_mutation_recovery(&reason)
                    {
                        validation_guided_compact_repair_pending = true;
                        validation_guided_compact_repair_reason = Some(reason);
                    } else {
                        let mutation_packet = native_tool_mutation_entry_packet(
                            &self.metadata,
                            &self.initial_prompt,
                            &all_receipts,
                        );
                        prompt = native_tool_mutation_only_recovery_prompt(
                            &self.metadata,
                            &self.initial_prompt,
                            &reason,
                            &mutation_packet,
                        );
                        mutation_only_recovery_pending = true;
                        mutation_only_recovery_reason = Some(reason);
                    }
                    last_response = Some(response);
                    continue;
                }
            }
            if native_tool_has_successful_mutation(&turn_receipts) {
                let auto_validation_started = Instant::now();
                if !native_tool_should_defer_auto_validation_for_stage(
                    &self.initial_prompt,
                    &all_receipts,
                ) {
                    if let Some(validation_receipt) =
                        native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
                {
                    turn_receipts.push(validation_receipt.clone());
                    all_receipts.push(validation_receipt);
                    native_tool_persist_runtime_timeline_event(
                        &self.metadata,
                        &self.initial_prompt,
                        "auto_validation_after_mutation_end",
                        native_tool_bounded_patch_elapsed_ms(native_timeline_started),
                        json!({
                            "duration_ms": native_tool_bounded_patch_elapsed_ms(auto_validation_started),
                            "successful_validation_after_batch": native_tool_has_successful_validation_command(&all_receipts),
                        }),
                    );
                    native_tool_persist_run_journal(
                        &self.metadata,
                        &self.initial_prompt,
                        "auto_validation_after_mutation",
                        provider_call_count,
                        &all_receipts,
                        Some(&response.output),
                        None,
                    );
                }
                }
                if native_tool_requires_successful_mutation(&self.metadata)
                    && native_tool_validation_guided_compact_repair_enabled(&self.metadata)
                    && native_tool_has_failed_validation_command(&turn_receipts)
                    && !native_tool_has_successful_validation_after_latest_mutation(&all_receipts)
                {
                    validation_guided_compact_repair_pending = true;
                    validation_guided_compact_repair_reason =
                        Some("failed_validation_after_implementation_mutation".to_string());
                    last_response = Some(response);
                    continue;
                }
            }
            if coding_execution_spine_v1_enabled(&self.metadata)
                && coding_execution_spine_v1_routes_lane(coding_task_lane)
            {
                let mut spine_decision = coding_execution_spine_decision_from_native_receipts(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                if matches!(spine_decision.action, CodingSpineAction::Validate) {
                    if let Some(validation_receipt) =
                        native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
                    {
                        turn_receipts.push(validation_receipt.clone());
                        all_receipts.push(validation_receipt);
                        native_tool_persist_run_journal(
                            &self.metadata,
                            &self.initial_prompt,
                            "coding_execution_spine_auto_validation",
                            provider_call_count,
                            &all_receipts,
                            Some(&response.output),
                            None,
                        );
                        spine_decision = coding_execution_spine_decision_from_native_receipts(
                            &self.metadata,
                            &self.initial_prompt,
                            &all_receipts,
                        );
                    }
                }
                if matches!(spine_decision.action, CodingSpineAction::CloseSuccess) {
                    if bounded_direct_edit_task {
                        let direct_tool_call_count = all_receipts.len();
                        native_tool_push_bounded_direct_edit_marker_once(
                            &mut all_receipts,
                            "success",
                            json!({
                                "terminal_status": "ok",
                                "provider_call_count": provider_call_count,
                                "tool_call_count": direct_tool_call_count,
                                "reason": "coding_execution_spine_v1_closed_success",
                                "coding_execution_spine_v1": spine_decision.clone()
                            }),
                        );
                    }
                    let mut response = native_tool_synthetic_completion_evidence_response(
                        &response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        "coding_execution_spine_v1_closed_success",
                    );
                    response.raw = json!({
                        "provider_raw": response.raw,
                        "native_tool_loop": {
                            "enabled": true,
                            "provider_call_count": provider_call_count,
                            "tool_call_count": all_receipts.len(),
                            "empty_tool_retry_count": empty_tool_retry_count,
                            "coding_task_lane": coding_task_lane,
                            "tool_receipts": all_receipts.clone(),
                            "terminal_status": "ok",
                            "coding_execution_spine_v1": spine_decision,
                        }
                    });
                    return Ok((
                        response,
                        all_receipts,
                        provider_call_count,
                        "ok".to_string(),
                    ));
                }
            }
            if native_tool_should_synthesize_micro_final(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            ) {
                let mut response = native_tool_synthetic_micro_final_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                response.raw = json!({
                    "provider_raw": response.raw,
                    "native_tool_loop": {
                        "enabled": true,
                        "provider_call_count": provider_call_count,
                        "tool_call_count": all_receipts.len(),
                        "empty_tool_retry_count": empty_tool_retry_count,
                        "coding_task_lane": coding_task_lane,
                        "tool_receipts": all_receipts.clone(),
                        "terminal_status": "ok",
                        "synthetic_micro_final": true,
                    }
                });
                return Ok((
                    response,
                    all_receipts,
                    provider_call_count,
                    "ok".to_string(),
                ));
            }
            if native_tool_synthesize_final_after_successful_validation(&self.metadata)
                && native_tool_has_successful_mutation(&all_receipts)
                && native_tool_has_successful_validation_after_latest_mutation(&all_receipts)
                && native_tool_prompt_evidence_gaps(&self.initial_prompt, &all_receipts).is_empty()
                && !native_tool_checkpointed_project_has_live_stage(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                )
            {
                if bounded_direct_edit_task {
                    let direct_tool_call_count = all_receipts.len();
                    native_tool_push_bounded_direct_edit_marker_once(
                        &mut all_receipts,
                        "success",
                        json!({
                            "terminal_status": "ok",
                            "provider_call_count": provider_call_count,
                            "tool_call_count": direct_tool_call_count,
                            "reason": "successful_validation_receipt_runtime_early_success"
                        }),
                    );
                }
                let mut response = native_tool_synthetic_completion_evidence_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    "successful_validation_receipt_runtime_early_success",
                );
                response.raw = json!({
                    "provider_raw": response.raw,
                    "native_tool_loop": {
                        "enabled": true,
                        "provider_call_count": provider_call_count,
                        "tool_call_count": all_receipts.len(),
                        "empty_tool_retry_count": empty_tool_retry_count,
                        "coding_task_lane": coding_task_lane,
                        "tool_receipts": all_receipts.clone(),
                        "terminal_status": "ok",
                        "synthetic_completion_evidence_final": true,
                        "early_success_after_validation": true,
                    }
                });
                return Ok((
                    response,
                    all_receipts,
                    provider_call_count,
                    "ok".to_string(),
                ));
            }
            if native_tool_requires_successful_mutation(&self.metadata)
                && !native_tool_has_successful_mutation(&all_receipts)
                && native_tool_context_only_turn(&turn_receipts)
            {
                context_only_turn_count += 1;
            } else if native_tool_has_successful_mutation(&turn_receipts) {
                context_only_turn_count = 0;
            }
            let observation = native_tool_observation_prompt(&turn_receipts);
            if native_tool_requires_successful_mutation(&self.metadata)
                && !native_tool_has_successful_mutation(&all_receipts)
                && context_only_turn_count >= native_tool_max_context_only_turns(&self.metadata)
            {
                prompt = native_tool_context_to_mutation_retry_prompt(
                    &self.metadata,
                    &self.initial_prompt,
                    &response.output,
                    &observation,
                    context_only_turn_count,
                );
                last_response = Some(response);
                continue;
            }
            if native_tool_checkpointed_project_operator_tool_loop_active(
                &self.metadata,
                &self.initial_prompt,
            ) {
                if let Some(stage) = native_tool_current_live_stage(&self.initial_prompt, &all_receipts)
                {
                    prompt = native_tool_checkpointed_project_stage_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        stage,
                        &observation,
                        native_tool_turn_number_usize(turn_idx),
                    );
                    last_response = Some(response);
                    continue;
                }
            }
            prompt = format!(
                "{}\n\nAssistant tool request turn {}:\n{}\n\nNative tool observations:\n{}\n\n{}",
                self.initial_prompt,
                turn_idx + 1,
                response.output,
                observation,
                "Continue."
            );
            last_response = Some(response);
        }

        let mut response = last_response.ok_or_else(|| {
            ProviderError::new(
                crate::provider::ProviderErrorCode::Unavailable,
                "native_tool_loop_no_provider_response",
            )
        })?;
        let pending_terminal_calls = parse_native_tool_calls(&response.output);
        if !pending_terminal_calls.is_empty()
            && native_tool_requires_successful_mutation(&self.metadata)
            && !native_tool_has_successful_mutation(&all_receipts)
        {
            let existing_call_ids = all_receipts
                .iter()
                .map(|receipt| receipt.call_id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let mut terminal_receipts = Vec::new();
            for call in pending_terminal_calls
                .into_iter()
                .take(native_tool_max_calls_per_turn(&self.metadata))
            {
                if existing_call_ids.contains(&call.id) {
                    continue;
                }
                let receipt = dispatcher.dispatch(native_tool_call_with_prompt_defaults(
                    call,
                    &self.initial_prompt,
                ));
                terminal_receipts.push(receipt.clone());
                all_receipts.push(receipt);
            }
            native_tool_persist_run_journal(
                &self.metadata,
                &self.initial_prompt,
                "terminal_tool_calls",
                provider_call_count,
                &all_receipts,
                Some(&response.output),
                None,
            );
            if native_tool_should_synthesize_micro_final(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            ) {
                let mut response = native_tool_synthetic_micro_final_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                response.raw = json!({
                    "provider_raw": response.raw,
                    "native_tool_loop": {
                        "enabled": true,
                        "provider_call_count": provider_call_count,
                        "tool_call_count": all_receipts.len(),
                        "empty_tool_retry_count": empty_tool_retry_count,
                        "coding_task_lane": coding_task_lane,
                        "tool_receipts": all_receipts.clone(),
                        "terminal_status": "ok",
                        "synthetic_micro_final": true,
                        "executed_pending_terminal_tool_calls": terminal_receipts.len(),
                    }
                });
                return Ok((
                    response,
                    all_receipts,
                    provider_call_count,
                    "ok".to_string(),
                ));
            }
        }
        if !native_tool_should_defer_auto_validation_for_stage(&self.initial_prompt, &all_receipts)
        {
            if let Some(validation_receipt) =
                native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
            {
                all_receipts.push(validation_receipt);
                native_tool_persist_run_journal(
                    &self.metadata,
                    &self.initial_prompt,
                    "auto_validation",
                    provider_call_count,
                    &all_receipts,
                    Some(&response.output),
                    None,
                );
            }
        }
        let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
            &dispatcher,
            &self.metadata,
            &self.initial_prompt,
            &all_receipts,
        );
        if !auto_handoff_receipts.is_empty() {
            all_receipts.extend(auto_handoff_receipts);
            native_tool_persist_run_journal(
                &self.metadata,
                &self.initial_prompt,
                "auto_handoff",
                provider_call_count,
                &all_receipts,
                Some(&response.output),
                None,
            );
        }
        let initial_repair_reasons = native_tool_runtime_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if !initial_repair_reasons.is_empty()
            && native_tool_completion_evidence_repair_enabled(&self.metadata)
        {
            let repaired = native_tool_completion_evidence_repair_loop(
                &provider,
                &dispatcher,
                tools,
                self.model.clone(),
                &self.metadata,
                &self.initial_prompt,
                &system,
                response,
                all_receipts,
                provider_call_count,
                initial_repair_reasons,
            )?;
            response = repaired.0;
            all_receipts = repaired.1;
            provider_call_count = repaired.2;
        }
        let terminal_output_has_tool_calls = !parse_native_tool_calls(&response.output).is_empty();
        let completion_evidence_finalization = native_tool_needs_artifact_finalization(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if (terminal_output_has_tool_calls
            || native_tool_needs_public_report_finalization(&self.metadata, &response.output)
            || completion_evidence_finalization)
            && native_tool_synthesize_final_after_successful_validation(&self.metadata)
            && native_tool_has_successful_mutation(&all_receipts)
            && native_tool_has_successful_validation_after_latest_mutation(&all_receipts)
            && !native_tool_checkpointed_project_has_live_stage(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            )
        {
            response = native_tool_synthetic_completion_evidence_response(
                &response,
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
                "successful_validation_receipt_runtime_synthesized_final",
            );
        } else if terminal_output_has_tool_calls
            || native_tool_needs_public_report_finalization(&self.metadata, &response.output)
            || completion_evidence_finalization
        {
            provider_call_count += 1;
            let mut finalization_prompt = native_tool_public_reasoning_finalization_prompt(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
                &response.output,
            );
            if completion_evidence_finalization {
                finalization_prompt.push_str("\n\n");
                finalization_prompt.push_str(
                    &native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "coverage_finalization_guard_rule",
                        "Add workflow-required coverage status for the original task. Mark uncovered or blocked requirements accurately instead of reporting success without receipt-backed evidence.",
                    ),
                );
            }
            if terminal_output_has_tool_calls {
                finalization_prompt.push_str("\n\n");
                finalization_prompt.push_str(
                    &native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "terminal_tool_call_finalization_guard_rule",
                        "The previous assistant response still contained native tool calls, so tools are disabled for this pass. Return only a final receipt-backed user response and do not output tool calls.",
                    ),
                );
            }
            let request = ProviderRequest {
                prompt: finalization_prompt,
                system: Some(system.clone()),
                tools: Vec::new(),
                model: self.model.clone(),
                metadata: native_tool_public_reasoning_metadata(&self.metadata),
            };
            response = match provider.complete(&request) {
                Ok(response) => response,
                Err(error)
                    if error.code == ProviderErrorCode::Timeout
                        && native_tool_has_successful_mutation(&all_receipts)
                        && native_tool_completion_evidence_timeout_synthesis_enabled(
                            &self.metadata,
                        ) =>
                {
                    native_tool_synthetic_completion_evidence_response(
                        &response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        error.message.as_str(),
                    )
                }
                Err(error) => return Err(error),
            };
            if !parse_native_tool_calls(&response.output).is_empty() {
                return Err(ProviderError::new(
                    ProviderErrorCode::InvalidRequest,
                    "native_tool_terminal_tool_calls_after_finalization",
                ));
            }
            if native_tool_needs_artifact_finalization(
                &self.metadata,
                &self.initial_prompt,
                &response.output,
                &all_receipts,
            ) && native_tool_has_successful_mutation(&all_receipts)
            {
                response = native_tool_synthetic_completion_evidence_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    "missing_task_requirement_checklist_after_finalization",
                );
            }
        }
        let final_repair_reasons = native_tool_runtime_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if !final_repair_reasons.is_empty()
            && native_tool_completion_evidence_repair_enabled(&self.metadata)
        {
            let repaired = native_tool_completion_evidence_repair_loop(
                &provider,
                &dispatcher,
                tools,
                self.model.clone(),
                &self.metadata,
                &self.initial_prompt,
                &system,
                response,
                all_receipts,
                provider_call_count,
                final_repair_reasons,
            )?;
            response = repaired.0;
            all_receipts = repaired.1;
            provider_call_count = repaired.2;
        }
        if let Some(validation_receipt) =
            native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
        {
            all_receipts.push(validation_receipt);
        }
        let post_validation_repair_reasons = native_tool_runtime_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if native_tool_repair_reasons_include_failed_validation(&post_validation_repair_reasons)
            && native_tool_completion_evidence_repair_enabled(&self.metadata)
        {
            let repaired = native_tool_completion_evidence_repair_loop(
                &provider,
                &dispatcher,
                tools,
                self.model.clone(),
                &self.metadata,
                &self.initial_prompt,
                &system,
                response,
                all_receipts,
                provider_call_count,
                post_validation_repair_reasons,
            )?;
            response = repaired.0;
            all_receipts = repaired.1;
            provider_call_count = repaired.2;
            if let Some(validation_receipt) = native_tool_auto_validation_receipt(
                &dispatcher,
                &self.initial_prompt,
                &all_receipts,
            ) {
                all_receipts.push(validation_receipt);
            }
        }
        if !parse_native_tool_calls(&response.output).is_empty()
            && native_tool_has_successful_mutation(&all_receipts)
            && native_tool_completion_evidence_timeout_synthesis_enabled(&self.metadata)
        {
            response = native_tool_synthetic_completion_evidence_response(
                &response,
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
                "terminal_native_requests_after_evidence_repair",
            );
        }
        let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
            &dispatcher,
            &self.metadata,
            &self.initial_prompt,
            &all_receipts,
        );
        if !auto_handoff_receipts.is_empty() {
            all_receipts.extend(auto_handoff_receipts);
            if native_tool_prompt_evidence_gaps(&self.initial_prompt, &all_receipts).is_empty() {
                response = native_tool_synthetic_completion_evidence_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    "runtime_synthesized_handoff_artifacts",
                );
            }
        }
        if native_tool_synthesize_final_after_successful_validation(&self.metadata)
            && native_tool_has_successful_mutation(&all_receipts)
            && native_tool_has_successful_validation_after_latest_mutation(&all_receipts)
            && !native_tool_checkpointed_project_has_live_stage(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            )
        {
            response = native_tool_synthetic_completion_evidence_response(
                &response,
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
                "successful_validation_terminal_receipt_synthesis",
            );
        }
        let mut unresolved_final_reasons = native_tool_runtime_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if unresolved_final_reasons
            .iter()
            .any(|reason| reason == "missing_product_mutation_receipt")
            && native_tool_artifact_contract_enabled(&self.metadata)
            && native_tool_requires_successful_mutation(&self.metadata)
            && !native_tool_has_successful_mutation(&all_receipts)
            && empty_tool_retry_count < empty_tool_retry_limit
        {
            provider_call_count += 1;
            let observation = native_tool_observation_prompt(&all_receipts);
            let forced_mutation_prompt = native_tool_context_to_mutation_retry_prompt(
                &self.metadata,
                &self.initial_prompt,
                &response.output,
                &observation,
                empty_tool_retry_count + 1,
            );
            let request = ProviderRequest {
                prompt: forced_mutation_prompt,
                system: Some(system.clone()),
                tools: tools.to_vec(),
                model: self.model.clone(),
                metadata: native_tool_recovery_timeout_metadata(&self.metadata),
            };
            let forced_response = match provider.complete(&request) {
                Ok(response) => response,
                Err(error) if native_tool_provider_error_is_timeout(&error) => {
                    let response = native_tool_partial_progress_response(
                        provider.provider_id(),
                        self.model.as_deref(),
                        error.message.as_str(),
                        provider_call_count,
                        &all_receipts,
                    );
                    return Ok((
                        response,
                        all_receipts,
                        provider_call_count,
                        "partial_timeout".to_string(),
                    ));
                }
                Err(error) => return Err(error),
            };
            let forced_calls = parse_native_tool_calls(&forced_response.output);
            response = forced_response;
            if !forced_calls.is_empty() {
                let mut forced_receipts = Vec::new();
                for call in forced_calls
                    .into_iter()
                    .take(native_tool_max_calls_per_turn(&self.metadata))
                {
                    let call = native_tool_call_with_prompt_defaults(call, &self.initial_prompt);
                    let context_blocked = native_tool_has_successful_context_receipt(&all_receipts)
                        && !native_tool_has_successful_mutation(&all_receipts)
                        && native_tool_call_is_context_only(&call);
                    let receipt = if context_blocked {
                        native_tool_product_repair_context_blocked_receipt(call, &[], &all_receipts)
                    } else if let Some(blocked) =
                        native_tool_preserved_api_write_blocked_receipt(&self.initial_prompt, &call)
                    {
                        blocked
                    } else if let Some(blocked) =
                        native_tool_python_existing_shape_blocked_receipt(&self.initial_prompt, &call)
                    {
                        blocked
                    } else if let Some(blocked) =
                        native_tool_atomic_dependent_edit_blocked_receipt(&all_receipts, &call)
                    {
                        blocked
                    } else if let Some(blocked) =
                        native_tool_python_src_prefix_import_blocked_receipt(
                            &self.initial_prompt,
                            &call,
                        )
                    {
                        blocked
                    } else {
                        dispatcher.dispatch(call)
                    };
                    forced_receipts.push(receipt.clone());
                    all_receipts.push(receipt);
                }
                native_tool_persist_run_journal(
                    &self.metadata,
                    &self.initial_prompt,
                    "forced_mutation",
                    provider_call_count,
                    &all_receipts,
                    Some(&response.output),
                    None,
                );
                if let Some(validation_receipt) = native_tool_auto_validation_receipt(
                    &dispatcher,
                    &self.initial_prompt,
                    &all_receipts,
                ) {
                    all_receipts.push(validation_receipt);
                    native_tool_persist_run_journal(
                        &self.metadata,
                        &self.initial_prompt,
                        "forced_auto_validation",
                        provider_call_count,
                        &all_receipts,
                        Some(&response.output),
                        None,
                    );
                }
                let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
                    &dispatcher,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                if !auto_handoff_receipts.is_empty() {
                    all_receipts.extend(auto_handoff_receipts);
                    native_tool_persist_run_journal(
                        &self.metadata,
                        &self.initial_prompt,
                        "forced_auto_handoff",
                        provider_call_count,
                        &all_receipts,
                        Some(&response.output),
                        None,
                    );
                }
                unresolved_final_reasons = native_tool_runtime_repair_reasons(
                    &self.metadata,
                    &self.initial_prompt,
                    &response.output,
                    &all_receipts,
                );
                if !unresolved_final_reasons.is_empty()
                    && native_tool_completion_evidence_repair_enabled(&self.metadata)
                    && native_tool_has_successful_mutation(&all_receipts)
                {
                    let repaired = native_tool_completion_evidence_repair_loop(
                        &provider,
                        &dispatcher,
                        tools,
                        self.model.clone(),
                        &self.metadata,
                        &self.initial_prompt,
                        &system,
                        response,
                        all_receipts,
                        provider_call_count,
                        unresolved_final_reasons,
                    )?;
                    response = repaired.0;
                    all_receipts = repaired.1;
                    provider_call_count = repaired.2;
                    if let Some(validation_receipt) = native_tool_auto_validation_receipt(
                        &dispatcher,
                        &self.initial_prompt,
                        &all_receipts,
                    ) {
                        all_receipts.push(validation_receipt);
                    }
                    let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
                        &dispatcher,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    );
                    if !auto_handoff_receipts.is_empty() {
                        all_receipts.extend(auto_handoff_receipts);
                    }
                    unresolved_final_reasons = native_tool_runtime_repair_reasons(
                        &self.metadata,
                        &self.initial_prompt,
                        &response.output,
                        &all_receipts,
                    );
                }
                if unresolved_final_reasons.is_empty() {
                    response = native_tool_synthetic_completion_evidence_response(
                        &response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        "forced_product_mutation_retry_completed",
                    );
                }
            }
        }
        let bounded_direct_edit_hard_evidence_gaps = if bounded_direct_edit_task {
            native_tool_bounded_direct_edit_required_evidence_gaps(
                &self.initial_prompt,
                &all_receipts,
            )
        } else {
            Vec::new()
        };
        if !bounded_direct_edit_hard_evidence_gaps.is_empty() {
            let direct_tool_call_count = all_receipts.len();
            let bounded_direct_edit_hard_evidence_gap_summary =
                bounded_direct_edit_hard_evidence_gaps.join(",");
            native_tool_push_bounded_direct_edit_marker_once(
                &mut all_receipts,
                "partial_blocked",
                json!({
                    "terminal_status": "partial_blocked",
                    "provider_call_count": provider_call_count,
                    "tool_call_count": direct_tool_call_count,
                    "unresolved_completion_evidence": bounded_direct_edit_hard_evidence_gaps.clone(),
                }),
            );
            let reason = format!(
                "bounded_direct_edit_unresolved_required_evidence:{};receipt_summary={}",
                bounded_direct_edit_hard_evidence_gap_summary,
                native_tool_receipt_error_summary(&all_receipts)
            );
            response = native_tool_partial_progress_response(
                provider.provider_id(),
                self.model.as_deref(),
                &reason,
                provider_call_count,
                &all_receipts,
            );
            response.raw = json!({
                "provider_raw": response.raw,
                "native_tool_loop": {
                    "enabled": true,
                    "provider_call_count": provider_call_count,
                    "tool_call_count": all_receipts.len(),
                    "empty_tool_retry_count": empty_tool_retry_count,
                    "coding_task_lane": coding_task_lane,
                    "tool_receipts": all_receipts.clone(),
                    "terminal_status": "partial_blocked",
                    "unresolved_completion_evidence": bounded_direct_edit_hard_evidence_gaps,
                }
            });
            return Ok((
                response,
                all_receipts,
                provider_call_count,
                "partial_blocked".to_string(),
            ));
        }
        if !unresolved_final_reasons.is_empty()
            && native_tool_artifact_contract_enabled(&self.metadata)
        {
            if bounded_direct_edit_task {
                let direct_tool_call_count = all_receipts.len();
                native_tool_push_bounded_direct_edit_marker_once(
                    &mut all_receipts,
                    "partial_blocked",
                    json!({
                        "terminal_status": "partial_blocked",
                        "provider_call_count": provider_call_count,
                        "tool_call_count": direct_tool_call_count,
                        "unresolved_completion_evidence": unresolved_final_reasons.clone(),
                    }),
                );
            }
            let reason = format!(
                "native_tool_unresolved_completion_evidence:{};receipt_summary={}",
                unresolved_final_reasons.join(","),
                native_tool_receipt_error_summary(&all_receipts)
            );
            response = native_tool_partial_progress_response(
                provider.provider_id(),
                self.model.as_deref(),
                &reason,
                provider_call_count,
                &all_receipts,
            );
            response.raw = json!({
                "provider_raw": response.raw,
                "native_tool_loop": {
                    "enabled": true,
                    "provider_call_count": provider_call_count,
                    "tool_call_count": all_receipts.len(),
                    "empty_tool_retry_count": empty_tool_retry_count,
                    "coding_task_lane": coding_task_lane,
                    "tool_receipts": all_receipts.clone(),
                    "terminal_status": "partial_blocked",
                    "unresolved_completion_evidence": unresolved_final_reasons,
                }
            });
            return Ok((
                response,
                all_receipts,
                provider_call_count,
                "partial_blocked".to_string(),
            ));
        }
        if bounded_direct_edit_task {
            let direct_tool_call_count = all_receipts.len();
            native_tool_push_bounded_direct_edit_marker_once(
                &mut all_receipts,
                "success",
                json!({
                    "terminal_status": "ok",
                    "provider_call_count": provider_call_count,
                    "tool_call_count": direct_tool_call_count,
                    "reason": "terminal_receipt_synthesis"
                }),
            );
        }
        response.raw = json!({
            "provider_raw": response.raw,
            "native_tool_loop": {
                "enabled": true,
                "provider_call_count": provider_call_count,
                "tool_call_count": all_receipts.len(),
                "empty_tool_retry_count": empty_tool_retry_count,
                "coding_task_lane": coding_task_lane,
                "tool_receipts": all_receipts.clone(),
                "terminal_status": "ok",
            }
        });
        native_tool_persist_run_journal(
            &self.metadata,
            &self.initial_prompt,
            "terminal",
            provider_call_count,
            &all_receipts,
            Some(&response.output),
            Some("ok"),
        );
        Ok((
            response,
            all_receipts,
            provider_call_count,
            "ok".to_string(),
        ))
    }
}

fn native_tool_direct_existing_project_mutation_entry_enabled(metadata: &Value) -> bool {
    let key = "direct_existing_project_mutation_entry_enabled";
    metadata
        .get(key)
        .or_else(|| {
            metadata
                .get("native_success_criteria")
                .and_then(|criteria| criteria.get(key))
        })
        .or_else(|| metadata.pointer(&format!("/workflow/native_success_criteria/{key}")))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_persist_run_journal(
    metadata: &Value,
    original_prompt: &str,
    stage: &str,
    provider_call_count: u64,
    receipts: &[NativeToolReceipt],
    latest_output: Option<&str>,
    terminal_status: Option<&str>,
) {
    let Some(path) = native_tool_run_journal_path(metadata, original_prompt) else {
        return;
    };
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let changed_files = native_tool_changed_paths(receipts);
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
    let phase_latency_ms = native_tool_receipt_phase_latency(receipts);
    let payload = json!({
        "schema_version": "native_coding_run_journal_v1",
        "source": "infring_native_tool_runtime",
        "stage": stage,
        "updated_at_unix_ms": Utc::now().timestamp_millis(),
        "provider_call_count": provider_call_count,
        "terminal_status": terminal_status,
        "runtime_failure_analysis": native_tool_runtime_failure_analysis(
            receipts,
            terminal_status.unwrap_or("in_progress"),
        ),
        "native_tool_receipts": receipts,
        "changed_files": changed_files,
        "validation_receipts": validation_receipts,
        "native_tool_phase_latency_ms": phase_latency_ms,
        "latest_output_preview": latest_output.unwrap_or("").chars().take(2000).collect::<String>(),
        "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
    });
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(path, text);
    }
}

fn native_tool_receipt_phase_latency(receipts: &[NativeToolReceipt]) -> Value {
    let mut bootstrap_context_ms = 0u64;
    let mut bootstrap_validation_ms = 0u64;
    let mut mutation_ms = 0u64;
    let mut validation_ms = 0u64;
    let mut semantic_probe_ms = 0u64;
    let mut other_ms = 0u64;
    for receipt in receipts {
        if receipt.call_id.starts_with("runtime_bootstrap_file_")
            || receipt.call_id.starts_with("bounded_patch_artifact_file_read")
        {
            bootstrap_context_ms += receipt.duration_ms;
        } else if receipt.call_id == "runtime_bootstrap_pre_mutation_validation_command" {
            bootstrap_validation_ms += receipt.duration_ms;
        } else if matches!(receipt.tool_name.as_str(), "file_write" | "file_patch") {
            mutation_ms += receipt.duration_ms;
        } else if receipt.tool_name == "command_run"
            && native_tool_receipt_command_text(receipt).contains("semantic_probe.py")
        {
            semantic_probe_ms += receipt.duration_ms;
        } else if receipt.tool_name == "command_run" {
            validation_ms += receipt.duration_ms;
        } else {
            other_ms += receipt.duration_ms;
        }
    }
    json!({
        "bootstrap_context_ms": bootstrap_context_ms,
        "bootstrap_validation_ms": bootstrap_validation_ms,
        "mutation_ms": mutation_ms,
        "validation_ms": validation_ms,
        "semantic_probe_ms": semantic_probe_ms,
        "other_ms": other_ms,
        "receipt_total_ms": bootstrap_context_ms
            + bootstrap_validation_ms
            + mutation_ms
            + validation_ms
            + semantic_probe_ms
            + other_ms,
    })
}

fn native_tool_receipt_command_text(receipt: &NativeToolReceipt) -> String {
    receipt
        .result
        .get("cmd")
        .map(|cmd| match cmd {
            Value::Array(parts) => parts
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" "),
            Value::String(value) => value.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn native_tool_run_journal_path(metadata: &Value, original_prompt: &str) -> Option<PathBuf> {
    if let Some(path) = metadata
        .get("native_run_journal_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(PathBuf::from(path));
    }
    native_tool_prompt_project_root(original_prompt)
        .map(PathBuf::from)
        .map(|root| root.join(".infring").join("native_run_journal.json"))
}

fn native_tool_persist_runtime_timeline_event(
    metadata: &Value,
    original_prompt: &str,
    event: &str,
    elapsed_ms: u64,
    details: Value,
) {
    let Some(journal_path) = native_tool_run_journal_path(metadata, original_prompt) else {
        return;
    };
    let Some(parent) = journal_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let path = parent.join("native_runtime_timeline.jsonl");
    let payload = json!({
        "schema_version": "native_runtime_timeline_probe_v1",
        "source": "infring_native_tool_runtime",
        "updated_at_unix_ms": Utc::now().timestamp_millis(),
        "event": event,
        "elapsed_ms": elapsed_ms,
        "details": details,
    });
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        if let Ok(line) = serde_json::to_string(&payload) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn native_tool_persist_provider_turn_timing_probe(
    metadata: &Value,
    original_prompt: &str,
    provider_call_count: u64,
    turn_idx: u64,
    request: &ProviderRequest,
    observation_chars: usize,
    provider_latency_ms: u64,
    status: &str,
    error: Option<&str>,
) {
    let Some(journal_path) = native_tool_run_journal_path(metadata, original_prompt) else {
        return;
    };
    let Some(parent) = journal_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let path = parent.join("native_provider_turn_timing.jsonl");
    let payload = json!({
        "schema_version": "native_provider_turn_timing_probe_v1",
        "source": "infring_native_tool_runtime",
        "updated_at_unix_ms": Utc::now().timestamp_millis(),
        "provider_call_count": provider_call_count,
        "turn_idx": turn_idx,
        "status": status,
        "provider_latency_ms": provider_latency_ms,
        "prompt_chars": request.prompt.chars().count(),
        "system_chars": request.system.as_ref().map(|value| value.chars().count()).unwrap_or(0),
        "observation_chars": observation_chars,
        "tool_count": request.tools.len(),
        "error_preview": error.unwrap_or("").chars().take(500).collect::<String>(),
    });
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        if let Ok(line) = serde_json::to_string(&payload) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn native_tool_empty_retry_limit(metadata: &Value) -> u64 {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let default = criteria
        .and_then(|value| value.get("requires_native_tool_use"))
        .and_then(Value::as_bool)
        .unwrap_or(false) as u64;
    criteria
        .and_then(|value| value.get("empty_tool_retry_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(default)
        .clamp(0, 3)
}

fn native_tool_provider_error_is_timeout(error: &ProviderError) -> bool {
    error.code == ProviderErrorCode::Timeout || error.message.contains("ollama_run_timeout")
}

fn native_tool_runtime_failure_analysis(
    receipts: &[NativeToolReceipt],
    terminal_status: &str,
) -> Value {
    let validation_unresolved = !native_tool_has_successful_validation_after_latest_mutation(receipts);
    let validation_failure_class = if validation_unresolved {
        let validation_details = native_tool_latest_failed_validation_receipt_details(receipts);
        native_tool_validation_failure_class(&validation_details)
    } else {
        None
    };
    let seeded_python_import_surface = native_tool_has_python_import_surface_seed_receipt(receipts);
    let terminal_timeout = terminal_status.contains("timeout");
    let actionable_repair_class = native_tool_actionable_repair_class(
        seeded_python_import_surface,
        terminal_timeout,
        validation_failure_class.as_deref(),
    );
    json!({
        "schema_version": "native_runtime_failure_analysis_v1",
        "terminal_status": terminal_status,
        "seeded_python_import_surface": seeded_python_import_surface,
        "terminal_timeout": terminal_timeout,
        "seeded_repair_timeout": seeded_python_import_surface && terminal_timeout,
        "validation_failure_class": validation_failure_class.map(Value::String).unwrap_or(Value::Null),
        "actionable_repair_class": actionable_repair_class,
    })
}

fn native_tool_actionable_repair_class(
    seeded_python_import_surface: bool,
    terminal_timeout: bool,
    validation_failure_class: Option<&str>,
) -> Value {
    if seeded_python_import_surface && terminal_timeout {
        return Value::String("seeded_repair_timeout".to_string());
    }
    if let Some(class_name) = validation_failure_class {
        if seeded_python_import_surface {
            return Value::String(format!("seeded_repair_{class_name}"));
        }
        return Value::String(class_name.to_string());
    }
    Value::Null
}

fn native_tool_validation_failure_class(details: &str) -> Option<String> {
    if details == "<none>" || details.trim().is_empty() {
        return None;
    }
    let lower = details.to_ascii_lowercase();
    let class_name = if lower.contains("cannot import name") || lower.contains("modulenotfounderror")
    {
        "import_surface_missing"
    } else if lower.contains("attributeerror") || lower.contains("has no attribute") {
        "attribute_missing"
    } else if lower.contains("typeerror") {
        "type_error"
    } else if lower.contains("filenotfounderror") || lower.contains("no such file or directory") {
        "file_not_found"
    } else if lower.contains("assertionerror") || lower.contains("assert") {
        "assertion_mismatch"
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "command_timeout"
    } else if lower.contains("syntaxerror") || lower.contains("indentationerror") {
        "syntax_error"
    } else {
        "unknown_validation_failure"
    };
    Some(class_name.to_string())
}

fn native_tool_has_python_import_surface_seed_receipt(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok" && receipt.call_id.contains("runtime_python_import_surface_seed")
    })
}

fn native_tool_python_import_surface_seed_source_receipt_count(
    receipts: &[NativeToolReceipt],
) -> usize {
    receipts
        .iter()
        .filter(|receipt| {
            receipt.status == "ok"
                && receipt.call_id.contains("runtime_python_import_surface_seed_source")
        })
        .count()
}

fn native_tool_guarded_blocked_receipt(
    skip: bool,
    receipt: Option<NativeToolReceipt>,
) -> Option<NativeToolReceipt> {
    if skip {
        None
    } else {
        receipt
    }
}

fn native_tool_call_targets_python_import_surface_seed_path(
    call: &NativeToolCall,
    receipts: &[NativeToolReceipt],
) -> bool {
    if !native_tool_call_is_mutation(call) {
        return false;
    }
    let Some(target) = native_tool_call_path_arg(call) else {
        return false;
    };
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !receipt.call_id.contains("runtime_python_import_surface_seed")
            || !matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
        {
            return false;
        }
        receipt
            .result
            .get("path")
            .and_then(Value::as_str)
            .map(|path| native_tool_paths_same_or_suffix(path, target))
            .unwrap_or(false)
    })
}

fn native_tool_placeholder_path_blocked_receipt(
    call: &NativeToolCall,
    receipts: &[NativeToolReceipt],
) -> Option<NativeToolReceipt> {
    if !native_tool_call_is_mutation(call) {
        return None;
    }
    let path = native_tool_call_path_arg(call)?;
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let placeholder_path = lower.contains("exact ")
        || lower.contains("placeholder")
        || lower.contains("owner path")
        || lower.contains("observed path")
        || lower.contains("seeded path")
        || lower.contains(" from the list")
        || lower.contains("copy_one_owner_mutation_target")
        || lower.contains("<path>")
        || lower.contains("{path}");
    if !placeholder_path {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "placeholder_path_guard",
            "reason": "mutation_path_must_be_concrete_observed_path",
            "path": path,
            "observed_product_source_paths": native_tool_observed_product_source_paths(receipts),
            "seeded_import_surface_paths": native_tool_python_import_surface_seed_paths(receipts),
            "required_next_tool": "file_write_or_file_patch_with_concrete_observed_path"
        }),
        error: Some("mutation_path_must_be_concrete_observed_path".to_string()),
    })
}

fn native_tool_placeholder_content_blocked_receipt(
    call: &NativeToolCall,
    receipts: &[NativeToolReceipt],
) -> Option<NativeToolReceipt> {
    let name = call.name.trim().to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "file_write" | "write_file" | "workspace.write" | "workspace_write"
    ) {
        return None;
    }
    let content = call.args.get("content").and_then(Value::as_str)?;
    let lower = content.trim().to_ascii_lowercase();
    let placeholder_content = lower.is_empty()
        || lower.contains("complete replacement file content")
        || lower.contains("complete valid source code")
        || lower.contains("write_complete_python_source_here")
        || lower.contains("replacement file content")
        || lower.contains("preserving observed public api")
        || lower.contains("<complete")
        || lower.contains("{content}");
    if !placeholder_content {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: name,
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "placeholder_content_guard",
            "reason": "mutation_content_must_be_concrete_source",
            "path": native_tool_call_path_arg(call),
            "observed_product_source_paths": native_tool_observed_product_source_paths(receipts),
            "seeded_import_surface_paths": native_tool_python_import_surface_seed_paths(receipts),
            "required_next_tool": "file_write_with_complete_concrete_source_or_file_patch_with_exact_old_text",
            "next_action": "replace placeholder content with actual source code; do not copy prompt schema text into files",
        }),
        error: Some("mutation_content_must_be_concrete_source".to_string()),
    })
}

fn native_tool_python_import_surface_seed_paths(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = Vec::new();
    for receipt in receipts {
        if receipt.status != "ok"
            || !receipt.call_id.contains("runtime_python_import_surface_seed")
            || !matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
        {
            continue;
        }
        if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
            native_tool_push_unique_string(&mut paths, path.to_string());
        }
    }
    paths
}

fn native_tool_requires_successful_mutation(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("requires_successful_mutation_receipt"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_max_context_only_turns(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("max_context_only_turns"))
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 6)
}

fn native_tool_max_calls_per_turn(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("max_tool_calls_per_turn"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 16) as usize
}

fn native_tool_wall_timeout(metadata: &Value) -> Option<Duration> {
    let seconds = metadata
        .get("native_tool_wall_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/native_wall_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/native_wall_timeout_seconds")
                .and_then(Value::as_u64)
        })?;
    if seconds == 0 {
        None
    } else {
        Some(Duration::from_secs(seconds.clamp(1, 7200)))
    }
}

fn native_tool_recovery_wall_timeout(metadata: &Value) -> Option<Duration> {
    let seconds = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("recovery_wall_timeout_seconds"))
        .and_then(Value::as_u64)?;
    if seconds == 0 {
        None
    } else {
        Some(Duration::from_secs(seconds.clamp(1, 7200)))
    }
}

fn native_tool_provider_turn_timeout_seconds(
    metadata: &Value,
    original_prompt: &str,
    turn_idx: u64,
) -> u64 {
    if native_tool_checkpointed_project_operator_tool_loop_active(metadata, original_prompt) {
        let key = if turn_idx == 0 {
            "checkpointed_tool_loop_initial_provider_timeout_seconds"
        } else {
            "checkpointed_tool_loop_provider_timeout_seconds"
        };
        return native_tool_success_criteria_u64(metadata, key)
            .unwrap_or(if turn_idx == 0 { 180 } else { 120 })
            .clamp(30, 240);
    }
    native_tool_success_criteria_u64(metadata, "provider_timeout_seconds")
        .unwrap_or(120)
        .clamp(15, 600)
}

fn native_tool_success_criteria_u64(metadata: &Value, key: &str) -> Option<u64> {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
}

fn native_tool_checkpointed_project_operator_tool_loop_active(
    metadata: &Value,
    original_prompt: &str,
) -> bool {
    let enabled = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("checkpointed_project_operator_tool_loop_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !enabled {
        return false;
    }
    let lower = original_prompt.to_ascii_lowercase();
    native_tool_prompt_mentions_any(
        &lower,
        &["checkpoint", "handoff", "resume token", "memory row", "project context"],
    ) && native_tool_prompt_mentions_any(
        &lower,
        &["existing project", "project root", "local project", "validation command"],
    )
}

fn native_tool_auto_memory_context_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Option<NativeToolReceipt> {
    if native_tool_has_successful_memory_context_command(receipts) {
        return None;
    }
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let memory_cli_pattern = native_tool_prompt_memory_cli_pattern(original_prompt)?;
    let prior_memory_row_id = native_tool_prompt_prior_memory_row_id(original_prompt)
        .or_else(|| native_tool_prompt_expected_memory_row_id(original_prompt))?;
    let command = native_tool_memory_cli_command_from_pattern(
        &memory_cli_pattern,
        &format!(
            "checkpoint get --id={}",
            native_tool_shell_quote(&prior_memory_row_id)
        ),
    );
    Some(dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_checkpoint_memory_context_get".to_string(),
        name: "command_run".to_string(),
        args: json!({
            "cwd": project_root,
            "cmd": ["sh", "-c", command],
            "timeout_seconds": 90,
            "max_output_bytes": 12000
        }),
    }))
}

fn native_tool_prompt_prior_memory_row_id(original_prompt: &str) -> Option<String> {
    for line in original_prompt.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("prior memory row id") || lower.contains("previous memory row id") {
            return line
                .split_once(':')
                .map(|(_, value)| {
                    value
                        .trim()
                        .trim_matches(|ch: char| {
                            matches!(
                                ch,
                                '"' | '\'' | '`' | ',' | '.' | ';' | '(' | ')' | '[' | ']'
                            )
                        })
                        .to_string()
                })
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn native_tool_memory_cli_command_from_pattern(pattern: &str, command: &str) -> String {
    if pattern.contains("<command>") {
        pattern.replace("<command>", command)
    } else {
        format!("{} {}", pattern.trim(), command)
    }
}

fn native_tool_shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn native_tool_has_successful_memory_context_command(receipts: &[NativeToolReceipt]) -> bool {
    receipts
        .iter()
        .any(|receipt| native_tool_receipt_is_memory_context(receipt) && receipt.status == "ok")
}

fn native_tool_receipt_is_memory_context(receipt: &NativeToolReceipt) -> bool {
    receipt.call_id == "runtime_checkpoint_memory_context_get"
        || (receipt.tool_name == "command_run"
            && receipt
                .result
                .get("command")
                .and_then(Value::as_array)
                .map(|command| {
                    command
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .or_else(|| {
                    receipt
                        .result
                        .get("cmd")
                        .and_then(|cmd| {
                            cmd.as_str().map(str::to_string).or_else(|| {
                                cmd.as_array().map(|parts| {
                                    parts
                                        .iter()
                                        .filter_map(Value::as_str)
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                })
                            })
                        })
                })
                .map(|command| {
                    let lower = command.to_ascii_lowercase();
                    lower.contains("checkpoint get") && lower.contains("memory")
                })
                .unwrap_or(false))
}

fn native_tool_prompt_mentions_any(prompt_lower: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| prompt_lower.contains(needle))
}

fn native_tool_stream_until_tool_calls_enabled(metadata: &Value, bounded_direct_edit_task: bool) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("provider_stream_until_tool_calls"))
        .and_then(Value::as_bool)
        .unwrap_or(bounded_direct_edit_task)
}

fn native_tool_max_turns(metadata: &Value) -> u64 {
    metadata
        .get("native_tool_max_turns")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/max_provider_turns")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/max_provider_turns")
                .and_then(Value::as_u64)
        })
        .unwrap_or(6)
        .clamp(1, 12)
}

fn native_tool_partial_progress_on_timeout(metadata: &Value) -> bool {
    metadata
        .get("partial_progress_on_timeout")
        .and_then(Value::as_bool)
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/partial_progress_on_timeout")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/partial_progress_on_timeout")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn native_tool_first_turn_timeout_recovery_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("first_turn_timeout_recovery"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_first_receipt_watchdog_enabled(metadata: &Value, original_prompt: &str) -> bool {
    native_tool_first_turn_timeout_recovery_enabled(metadata)
        || native_tool_prompt_looks_checkpointed_project_operator(original_prompt)
        || native_tool_checkpointed_project_operator_tool_loop_active(metadata, original_prompt)
}

fn native_tool_first_receipt_deadline_seconds(metadata: &Value, original_prompt: &str) -> u64 {
    if let Ok(raw) = std::env::var("INFRING_FIRST_RECEIPT_DEADLINE_SECONDS") {
        if let Ok(parsed) = raw.parse::<u64>() {
            return parsed.clamp(10, 120);
        }
    }

    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| {
            criteria
                .get("first_receipt_deadline_seconds")
                .or_else(|| criteria.get("first_turn_timeout_seconds"))
        })
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            if native_tool_prompt_looks_checkpointed_project_operator(original_prompt)
                || native_tool_checkpointed_project_operator_tool_loop_active(
                    metadata,
                    original_prompt,
                )
            {
                45
            } else {
                60
            }
        })
        .clamp(10, 120)
}

fn native_tool_first_turn_timeout_recovery_prompt(
    metadata: &Value,
    original_prompt: &str,
    reason: &str,
    observation: &str,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "first_turn_timeout_recovery_rule",
        "The first model call timed out before mutation. Continue with a smaller patch-only native tool turn. Use the already gathered runtime observations; do not call file_list/file_read again unless the validation output names a new missing file. Return only JSON tool_calls. Make the smallest additive file_write/file_patch source/test mutation that satisfies the requested local code change, preserve observed public API/class constructor fields and baseline CLI behavior, add/keep baseline preservation tests when preservation is part of the task, then run requested validation. If the task is genuinely blocked, return a structured blocker instead of more exploration.",
    );
    format!(
        "{}\n\n{}\n\nTimeout reason:\n{}\n\nRuntime bootstrap observations:\n{}",
        original_prompt.chars().take(2400).collect::<String>(),
        rule,
        reason.chars().take(700).collect::<String>(),
        observation
    )
}

fn native_tool_bounded_patch_artifact_lane_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_lane"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_bounded_direct_edit_lane_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_direct_edit_lane"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_bounded_fast_edit_preflight_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_fast_edit_preflight"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_first_edit_batch_contract_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("first_edit_batch_contract"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_staged_edit_controller_enabled(metadata: &Value, _bounded_direct_edit_task: bool) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("staged_edit_controller"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_seed_prepared_staged_controller_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("seed_prepared_staged_controller_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_seed_prepared_staged_controller_min_source_seed_receipts(
    metadata: &Value,
) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| {
            value.get("seed_prepared_staged_controller_min_source_seed_receipts")
        })
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, 8) as usize
}

fn native_tool_seed_prepared_staged_empty_retry_limit(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("seed_prepared_staged_empty_retry_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(0, 2)
}

fn native_tool_seed_prepared_high_fanout_min_source_seed_receipts(
    metadata: &Value,
) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| {
            value
                .get("seed_prepared_high_fanout_min_source_seed_receipts")
                .or_else(|| value.get("seed_prepared_staged_controller_min_source_seed_receipts"))
        })
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(1, 8) as usize
}

fn native_tool_seed_prepared_staged_retry_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("seed_prepared_staged_retry_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(60)
        .clamp(8, 90)
}

fn native_tool_seed_prepared_high_fanout_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("seed_prepared_high_fanout_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(90)
        .clamp(8, 120)
}

fn native_tool_mutation_only_recovery_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| {
            value
                .get("mutation_only_recovery_gate_enabled")
                .or_else(|| value.get("premature_validation_recovery_gate_enabled"))
        })
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_compact_action_controller_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("compact_action_controller_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_controlled_shell_edit_batch_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("controlled_shell_edit_batch_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_compact_mutation_entry_packet_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("compact_mutation_entry_packet_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_public_contract_packet_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("public_contract_packet_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_python_import_surface_seed_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("python_import_surface_seed_mutation_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_python_import_surface_seed_round_limit(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("python_import_surface_seed_round_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .clamp(0, 5) as usize
}

fn native_tool_seeded_import_surface_compact_repair_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("seeded_import_surface_compact_repair_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_seeded_import_surface_contract_packet_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("seeded_import_surface_contract_packet_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_validation_guided_compact_repair_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("validation_guided_compact_repair_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_validation_guided_compact_repair_provider_timeout_seconds(
    metadata: &Value,
) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("validation_guided_compact_repair_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(8, 60)
}

fn native_tool_mutation_only_recovery_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("mutation_only_recovery_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(45)
        .clamp(8, 60)
}

fn native_tool_mutation_recovery_tools(metadata: &Value, tools: &[String]) -> Vec<String> {
    if !native_tool_controlled_shell_edit_batch_enabled(metadata) {
        return native_tool_staged_edit_tools(tools);
    }
    let filtered = tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "file_write"
                    | "write_file"
                    | "workspace.write"
                    | "workspace_write"
                    | "file_patch"
                    | "patch_file"
                    | "workspace.patch"
                    | "workspace_patch"
                    | "command_run"
                    | "run_command"
                    | "command.run"
                    | "shell.run"
                    | "shell_run"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        native_tool_staged_edit_tools(tools)
    } else {
        filtered
    }
}

fn native_tool_compact_action_controller_tools(tools: &[String]) -> Vec<String> {
    let filtered = tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        vec!["command_run".to_string()]
    } else {
        filtered
    }
}

fn native_tool_mutation_entry_packet(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    if !native_tool_compact_mutation_entry_packet_enabled(metadata) {
        return native_tool_observation_prompt(receipts);
    }

    let project_root = native_tool_prompt_project_root(original_prompt)
        .unwrap_or_else(|| "the local project root".to_string());
    let project_root_path = PathBuf::from(&project_root);
    let mut paths = Vec::new();
    let mut file_contents: Vec<(i32, String, String)> = Vec::new();
    let mut memory_context_sections = Vec::<String>::new();
    for receipt in receipts.iter().filter(|receipt| receipt.status == "ok") {
        if native_tool_receipt_is_memory_context(receipt) {
            if let Some(stdout) = receipt.result.get("stdout").and_then(Value::as_str) {
                let brief = native_tool_mutation_entry_memory_context_brief(stdout);
                if !brief.trim().is_empty() {
                    memory_context_sections.push(brief);
                }
            }
        }
        if let Some(entries) = receipt.result.get("entries").and_then(Value::as_array) {
            for entry in entries {
                if entry
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(|kind| kind == "file")
                    .unwrap_or(true)
                {
                    if let Some(path) = native_tool_packet_value_path(entry) {
                        native_tool_push_unique_string(
                            &mut paths,
                            native_tool_mutation_entry_display_path(&project_root_path, &path),
                        );
                    }
                }
            }
        }
        if let Some(files) = receipt.result.get("files").and_then(Value::as_array) {
            for file in files {
                if let Some(path) = native_tool_packet_value_path(file) {
                    let display_path =
                        native_tool_mutation_entry_display_path(&project_root_path, &path);
                    native_tool_push_unique_string(&mut paths, display_path.clone());
                    if let Some(content) = file.get("content").and_then(Value::as_str) {
                        file_contents.push((
                            native_tool_mutation_entry_path_score(&display_path),
                            display_path,
                            native_tool_mutation_entry_content_brief(content),
                        ));
                    }
                }
            }
        }
        if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
            let display_path = native_tool_mutation_entry_display_path(&project_root_path, path);
            native_tool_push_unique_string(&mut paths, display_path.clone());
            if let Some(content) = receipt.result.get("content").and_then(Value::as_str) {
                file_contents.push((
                    native_tool_mutation_entry_path_score(&display_path),
                    display_path,
                    native_tool_mutation_entry_content_brief(content),
                ));
            }
        }
    }

    paths.sort_by_key(|path| native_tool_mutation_entry_path_score(path));
    paths.dedup();
    file_contents.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let mut seen_content_paths = Vec::new();
    let content_sections = file_contents
        .into_iter()
        .filter_map(|(_, path, content)| {
            if seen_content_paths.iter().any(|existing| existing == &path) {
                return None;
            }
            native_tool_push_unique_string(&mut seen_content_paths, path.clone());
            Some(format!("--- {path}\n{content}"))
        })
        .take(6)
        .collect::<Vec<_>>();
    let candidate_paths = paths
        .into_iter()
        .filter(|path| !native_tool_mutation_entry_path_looks_generated(path))
        .take(32)
        .collect::<Vec<_>>();
    let required_batch_shape =
        native_tool_mutation_entry_required_batch_shape(
            original_prompt,
            &candidate_paths,
            native_tool_mutation_entry_controller_stage(
                metadata,
                original_prompt,
                receipts,
                &candidate_paths,
            ),
        );
    let controller_stage = native_tool_mutation_entry_controller_stage(
        metadata,
        original_prompt,
        receipts,
        &candidate_paths,
    );
    format!(
        "Compact mutation-entry packet:\n\
Project root: {project_root}\n\
Controller stage: {controller_stage}\n\
Allowed next tools: file_write, file_patch, or one controlled command_run shell edit batch.\n\
Forbidden next tools: validation/non-edit command_run, file_read, file_read_many, file_list, file_stat, final answer, prose.\n\
Goal: complete only the current controller stage before validation or closure.\n\n\
Required next batch shape:\n{required_batch_shape}\n\n\
Observed candidate paths:\n{}\n\n\
Checkpoint memory context:\n{}\n\n\
Observed file contents:\n{}",
        if candidate_paths.is_empty() {
            "- none".to_string()
        } else {
            candidate_paths
                .iter()
                .take(8)
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if memory_context_sections.is_empty() {
            "- none".to_string()
        } else {
            memory_context_sections
                .iter()
                .take(2)
                .map(|section| section.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        },
        if content_sections.is_empty() {
            "- no compact file contents available".to_string()
        } else {
            content_sections.join("\n\n")
        }
    )
}

fn native_tool_mutation_entry_display_path(project_root: &Path, path: &str) -> String {
    let path_buf = PathBuf::from(path);
    if let Ok(relative) = path_buf.strip_prefix(project_root) {
        return relative.display().to_string();
    }
    path.to_string()
}

fn native_tool_mutation_entry_content_brief(content: &str) -> String {
    let compact = content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("@dataclass")
                || trimmed.contains("add_parser")
            {
                Some(line)
            } else {
                None
            }
        })
        .take(20)
        .collect::<Vec<_>>()
        .join("\n");
    if compact.trim().is_empty() {
        content.chars().take(320).collect::<String>()
    } else {
        compact.chars().take(420).collect::<String>()
    }
}

fn native_tool_mutation_entry_extend_public_contract_lines(
    lines: &mut Vec<String>,
    path: &str,
    content: &str,
) {
    let path_lower = path.replace('\\', "/").to_ascii_lowercase();
    let contract_source = path_lower.contains("/tests/")
        || path_lower.starts_with("tests/")
        || path_lower.contains("test_")
        || path_lower.contains("_test.")
        || path_lower.contains(".test.")
        || path_lower.contains(".spec.")
        || path_lower.contains("semantic_probe")
        || path_lower.ends_with("__init__.py")
        || path_lower.ends_with("mod.rs")
        || path_lower.ends_with("lib.rs")
        || path_lower.ends_with("index.ts")
        || path_lower.ends_with("index.tsx")
        || path_lower.ends_with("index.js")
        || path_lower.ends_with("index.jsx");
    for line in content.lines() {
        let trimmed = line.trim();
        if !native_tool_line_looks_public_contract_line(trimmed, contract_source) {
            continue;
        }
        native_tool_push_unique_string(
            lines,
            format!("{path}: {}", trimmed.chars().take(220).collect::<String>()),
        );
        if lines.len() >= 64 {
            break;
        }
    }
}

fn native_tool_line_looks_public_contract_line(line: &str, contract_source: bool) -> bool {
    if line.is_empty() || line.len() > 260 || line.starts_with('#') || line.starts_with("//") {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    if native_tool_project_operator_looks_public_shape_line(line)
        || line.starts_with("import ")
        || line.starts_with("from ")
        || lower.contains("__all__")
        || line.starts_with("assert ")
        || lower.contains("self.assert")
        || lower.starts_with("expect(")
        || lower.contains(".to_equal(")
        || lower.contains(".toeq(")
        || lower.contains(".to_be(")
    {
        return true;
    }
    if !contract_source || !line.contains('(') || !line.contains(')') {
        return false;
    }
    if native_tool_line_starts_with_any(
        &lower,
        &[
            "if ",
            "elif ",
            "else",
            "for ",
            "while ",
            "with ",
            "return ",
            "raise ",
            "print(",
            "open(",
            "writer.",
            "reader.",
        ],
    ) {
        return false;
    }
    line.contains('.')
        || line.contains(" = ")
        || line.contains(" == ")
        || native_tool_line_has_uppercase_call(line)
}

fn native_tool_line_starts_with_any(line: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| line.starts_with(prefix))
}

fn native_tool_line_has_uppercase_call(line: &str) -> bool {
    line.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .any(|token| {
            token
                .chars()
                .next()
                .map(|ch| ch.is_ascii_uppercase())
                .unwrap_or(false)
                && line.contains(&format!("{token}("))
        })
}

fn native_tool_mutation_entry_memory_context_brief(stdout: &str) -> String {
    let compact = stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed.starts_with('{')
                || trimmed.starts_with('[')
                || trimmed.contains("checkpoint")
                || trimmed.contains("resume")
                || trimmed.contains("changed")
                || trimmed.contains("validation")
                || trimmed.contains("next")
                || trimmed.contains("path")
            {
                Some(trimmed)
            } else {
                None
            }
        })
        .take(18)
        .collect::<Vec<_>>()
        .join("\n");
    let source = if compact.trim().is_empty() {
        stdout.trim()
    } else {
        compact.trim()
    };
    source.chars().take(1400).collect::<String>()
}

fn native_tool_mutation_entry_controller_stage<'a>(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    candidate_paths: &'a [String],
) -> &'static str {
    if !native_tool_checkpointed_project_operator_tool_loop_active(metadata, original_prompt) {
        return "product_mutation";
    }
    if !native_tool_has_successful_mutation(receipts) {
        return "product_source_slice";
    }

    let prompt_lower = original_prompt.to_ascii_lowercase();
    let changed_paths = native_tool_changed_paths(receipts);
    let changed_test_paths = native_tool_mutation_entry_role_paths(&changed_paths, "test");
    let changed_operator_paths =
        native_tool_mutation_entry_role_paths(&changed_paths, "operator_surface");
    let changed_export_paths = native_tool_mutation_entry_role_paths(&changed_paths, "export_surface");

    let wants_operator_surface = native_tool_prompt_mentions_any(
        &prompt_lower,
        &[
            "cli",
            "operator",
            "report",
            "import",
            "export",
            "roundtrip",
            "round-trip",
            "public api",
            "surface",
        ],
    ) || !native_tool_merge_role_paths(
        &native_tool_mutation_entry_role_paths(candidate_paths, "operator_surface"),
        &native_tool_mutation_entry_role_paths(candidate_paths, "export_surface"),
    )
    .is_empty();
    if wants_operator_surface
        && changed_operator_paths.is_empty()
        && changed_export_paths.is_empty()
    {
        return "operator_surface_slice";
    }

    let wants_tests = native_tool_prompt_mentions_any(
        &prompt_lower,
        &["test", "validation", "verify", "regression", "baseline preservation"],
    );
    if wants_tests && changed_test_paths.is_empty() {
        return "test_slice";
    }

    "product_mutation"
}

fn native_tool_mutation_entry_required_batch_shape(
    original_prompt: &str,
    paths: &[String],
    controller_stage: &str,
) -> String {
    let product_sources = native_tool_mutation_entry_role_paths(paths, "product_source");
    let operator_surfaces = native_tool_mutation_entry_role_paths(paths, "operator_surface");
    let test_paths = native_tool_mutation_entry_role_paths(paths, "test");
    let export_surfaces = native_tool_mutation_entry_role_paths(paths, "export_surface");
    let prompt_lower = original_prompt.to_ascii_lowercase();
    let wants_tests = prompt_lower.contains("test")
        || prompt_lower.contains("validation")
        || prompt_lower.contains("verify");
    let wants_operator = prompt_lower.contains("cli")
        || prompt_lower.contains("operator")
        || prompt_lower.contains("report")
        || prompt_lower.contains("import")
        || prompt_lower.contains("export")
        || prompt_lower.contains("roundtrip")
        || prompt_lower.contains("round-trip");
    let wants_multi_file = native_tool_prompt_requires_multi_file_product_slice(&prompt_lower);
    let edit_mode_line = if wants_multi_file {
        match controller_stage {
            "product_source_slice" => "- For this product-source stage, prefer one command_run shell edit batch using cat > path <<'EOF' heredocs for only source/model/store/service files.",
            "operator_surface_slice" => "- For this operator-surface stage, prefer one command_run shell edit batch using cat > path <<'EOF' heredocs for only CLI/report/import/export files.",
            "test_slice" => "- For this test stage, prefer one command_run shell edit batch using cat > path <<'EOF' heredocs for only test files.",
            _ => "- For multi-file slices, prefer one command_run shell edit batch using cat > path <<'EOF' heredocs for all required source/operator/test files.",
        }
    } else {
        "- Prefer file_write/file_patch for small single-file edits."
    };
    if controller_stage == "product_source_slice" {
        let product_line = native_tool_mutation_entry_candidates_line(
            "product_source",
            &product_sources,
            "required now; implement the durable/domain/store/service behavior in source code",
        );
        return format!(
            "{edit_mode_line}\n\
- Stage: product_source_slice.\n\
- Mutate product/source files only. Do not write tests, checkpoint receipts, or memory rows in this stage.\n\
- If persistence, reporting, import/export, or checkpoint continuation is requested, add the underlying source model/store/service behavior here.\n\
{product_line}"
        );
    }
    if controller_stage == "operator_surface_slice" {
        let operator_line = native_tool_mutation_entry_candidates_line(
            "operator_or_export_surface",
            &native_tool_merge_role_paths(&operator_surfaces, &export_surfaces),
            "required now; expose the changed behavior through CLI/operator/report/import/export code",
        );
        return format!(
            "{edit_mode_line}\n\
- Stage: operator_surface_slice.\n\
- Mutate operator-facing source only. Do not write tests, checkpoint receipts, or memory rows in this stage.\n\
- Add the smallest public CLI/report/import/export surface needed to exercise the product behavior.\n\
{operator_line}"
        );
    }
    if controller_stage == "test_slice" {
        let test_line = native_tool_mutation_entry_candidates_line(
            "test",
            &test_paths,
            "required now; cover product behavior, operator surface, and baseline preservation",
        );
        return format!(
            "{edit_mode_line}\n\
- Stage: test_slice.\n\
- Mutate focused tests only. Do not write checkpoint receipts or memory rows in this stage.\n\
- Cover the newly added behavior plus any baseline behavior the task says must be preserved.\n\
{test_line}"
        );
    }
    let product_line = native_tool_mutation_entry_candidates_line(
        "product_source",
        &product_sources,
        "required first; implement the actual behavior in a source module",
    );
    let operator_line = native_tool_mutation_entry_candidates_line(
        "operator_or_export_surface",
        &native_tool_merge_role_paths(&operator_surfaces, &export_surfaces),
        if wants_operator {
            "required when the task asks for CLI/report/operator/import/export behavior"
        } else {
            "include only if needed to expose the changed behavior"
        },
    );
    let test_line = native_tool_mutation_entry_candidates_line(
        "focused_test",
        &test_paths,
        if wants_tests {
            "required when tests or validation evidence are requested"
        } else {
            "include when a focused regression test is the smallest proof"
        },
    );
    format!(
        "- Return one JSON object only: {{\"tool_calls\":[...]}}.\n\
- Mutate source/operator/test files before validation or final answer.\n\
- {edit_mode_line}\n\
- Never run validation before mutation.\n\
{}\n{}\n{}\n\
Example:\n\
{{\"tool_calls\":[{{\"id\":\"edit_batch\",\"name\":\"command_run\",\"arguments\":{{\"cmd\":\"cat > src/pkg/module.py <<'EOF'\\n<full file>\\nEOF\\ncat > tests/test_feature.py <<'EOF'\\n<full test>\\nEOF\",\"timeout_seconds\":20}}}}]}}",
        product_line, operator_line, test_line
    )
}

fn native_tool_mutation_entry_role_paths(paths: &[String], role: &str) -> Vec<String> {
    paths
        .iter()
        .filter(|path| match role {
            "product_source" => {
                native_tool_mutation_entry_path_is_code(path)
                    && !native_tool_mutation_entry_path_is_test(path)
                    && !native_tool_mutation_entry_path_is_export_surface(path)
                    && !native_tool_mutation_entry_path_is_operator_surface(path)
            }
            "operator_surface" => native_tool_mutation_entry_path_is_operator_surface(path),
            "test" => native_tool_mutation_entry_path_is_test(path),
            "export_surface" => native_tool_mutation_entry_path_is_export_surface(path),
            _ => false,
        })
        .take(6)
        .cloned()
        .collect()
}

fn native_tool_merge_role_paths(left: &[String], right: &[String]) -> Vec<String> {
    let mut merged = Vec::new();
    for path in left.iter().chain(right.iter()) {
        native_tool_push_unique_string(&mut merged, path.clone());
    }
    merged
}

fn native_tool_mutation_entry_candidates_line(
    role: &str,
    paths: &[String],
    instruction: &str,
) -> String {
    if paths.is_empty() {
        format!("- {role}: {instruction}; candidates: infer from observed source tree.")
    } else {
        format!(
            "- {role}: {instruction}; candidates: {}",
            paths
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn native_tool_mutation_entry_path_is_code(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".py")
        || lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".go")
        || lower.ends_with(".java")
        || lower.ends_with(".rb")
}

fn native_tool_mutation_entry_path_is_test(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.contains("_test.")
        || lower.contains("test_")
        || lower.contains(".test.")
        || lower.contains(".spec.")
}

fn native_tool_mutation_entry_path_is_operator_surface(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("cli")
        || lower.contains("cmd")
        || lower.contains("main.")
        || lower.contains("operator")
        || lower.contains("report")
        || lower.contains("app.")
        || lower.contains("bin/")
}

fn native_tool_mutation_entry_path_is_export_surface(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with("__init__.py")
        || lower.ends_with("mod.rs")
        || lower.ends_with("lib.rs")
        || lower.ends_with("index.ts")
        || lower.ends_with("index.tsx")
        || lower.ends_with("index.js")
        || lower.ends_with("index.jsx")
}

fn native_tool_packet_value_path(value: &Value) -> Option<String> {
    value
        .get("relative_path")
        .and_then(Value::as_str)
        .or_else(|| value.get("path").and_then(Value::as_str))
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
}

fn native_tool_mutation_entry_path_score(path: &str) -> i32 {
    let lower = path.to_ascii_lowercase();
    let mut score = 100;
    if lower.starts_with("src/") || lower.contains("/src/") {
        score -= 30;
    }
    if lower.starts_with("tests/") || lower.contains("/tests/") || lower.contains("test") {
        score -= 20;
    }
    if lower.ends_with(".py")
        || lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".go")
        || lower.ends_with(".java")
        || lower.ends_with(".rb")
    {
        score -= 10;
    }
    if lower.contains("__pycache__")
        || lower.ends_with(".pyc")
        || lower.contains("/target/")
        || lower.contains("/node_modules/")
    {
        score += 200;
    }
    score
}

fn native_tool_mutation_entry_path_looks_generated(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("__pycache__")
        || lower.ends_with(".pyc")
        || lower.contains("/target/")
        || lower.contains("/node_modules/")
}

fn native_tool_push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn native_tool_controlled_shell_edit_batch_paths(
    metadata: &Value,
    original_prompt: &str,
    call: &NativeToolCall,
) -> Vec<PathBuf> {
    if !native_tool_controlled_shell_edit_batch_enabled(metadata)
        || !native_tool_profile_allows(metadata, original_prompt, 4)
        || !native_tool_call_is_command_run(call)
    {
        return Vec::new();
    }
    native_tool_shell_edit_safe_write_paths(original_prompt, call)
}

fn native_tool_shell_edit_receipt_synthesis_paths(
    original_prompt: &str,
    call: &NativeToolCall,
) -> Vec<PathBuf> {
    if !native_tool_call_is_command_run(call) {
        return Vec::new();
    }
    native_tool_shell_edit_safe_write_paths(original_prompt, call)
}

fn native_tool_shell_edit_safe_write_paths(
    original_prompt: &str,
    call: &NativeToolCall,
) -> Vec<PathBuf> {
    let Some(command_text) = native_tool_command_call_text(&call.args) else {
        return Vec::new();
    };
    let command_control_text = native_tool_shell_command_control_text(&command_text);
    if native_tool_command_text_looks_like_validation(&command_control_text) {
        return Vec::new();
    }
    let project_root = native_tool_prompt_project_root(original_prompt)
        .map(PathBuf::from)
        .or_else(|| native_tool_command_call_cwd(&call.args));
    let Some(project_root) = project_root else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for raw_path in native_tool_shell_edit_write_path_candidates(&command_text) {
        let candidate = if Path::new(&raw_path).is_absolute() {
            PathBuf::from(raw_path)
        } else {
            project_root.join(raw_path)
        };
        if native_tool_path_is_within_root(&candidate, &project_root)
            && native_tool_shell_edit_path_is_safe_source_path(&candidate)
            && !paths.iter().any(|existing| existing == &candidate)
        {
            paths.push(candidate);
        }
    }
    paths
}

fn native_tool_selected_profile(metadata: &Value, original_prompt: &str) -> u8 {
    let lane = native_tool_coding_task_lane(metadata, original_prompt);
    match &*lane {
        "new_file_fast_path" | "micro_direct_mutation" => 0,
        "deterministic_local_action" => 1,
        "existing_project_patch" | "bounded_existing_project_edit" | "implementation_slice" => 2,
        "validation_repair" | "validated_repair_edit" => 3,
        "multi_file_slice" | "project_slice" => 4,
        "checkpointed_project_operator" | "long_run_project_operator" => 5,
        "long_horizon_project" => 6,
        _ => 2,
    }
}

fn native_tool_profile_allows(metadata: &Value, original_prompt: &str, min_profile: u8) -> bool {
    native_tool_selected_profile(metadata, original_prompt) >= min_profile
}

fn native_tool_shell_edit_batch_blocked_receipt(
    call: &NativeToolCall,
    shell_edit_paths: &[PathBuf],
) -> Option<NativeToolReceipt> {
    if !native_tool_call_is_command_run(call) || !shell_edit_paths.is_empty() {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "controlled_shell_edit_batch",
            "reason": "command_run_before_mutation_must_be_shell_edit_batch",
            "required_next_tool": "file_write_or_file_patch_or_shell_edit_batch"
        }),
        error: Some("command_run_before_mutation_must_be_shell_edit_batch".to_string()),
    })
}

fn native_tool_first_mutation_artifact_lane_blocked_receipt(
    call: &NativeToolCall,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "first_mutation_artifact_lane_v1",
            "reason": "first_mutation_artifact_lane_allows_only_file_write_or_file_patch",
            "required_next_tool": "file_write_or_file_patch"
        }),
        error: Some("first_mutation_artifact_lane_allows_only_file_write_or_file_patch".to_string()),
    }
}

fn native_tool_dispatch_first_mutation_artifact_lane_calls(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    calls: Vec<NativeToolCall>,
    max_calls: usize,
) -> Vec<NativeToolReceipt> {
    let mut lane_receipts = Vec::new();
    for call in calls.into_iter().take(max_calls) {
        let call = native_tool_call_with_prompt_defaults(call, original_prompt);
        let receipt = if native_tool_call_is_mutation(&call) {
            if let Some(blocked) = native_tool_preserved_api_write_blocked_receipt(original_prompt, &call)
            {
                blocked
            } else if let Some(blocked) =
                native_tool_python_existing_shape_blocked_receipt(original_prompt, &call)
            {
                blocked
            } else if let Some(blocked) =
                native_tool_python_src_prefix_import_blocked_receipt(original_prompt, &call)
            {
                blocked
            } else {
                dispatcher.dispatch(call)
            }
        } else {
            native_tool_first_mutation_artifact_lane_blocked_receipt(&call)
        };
        lane_receipts.push(receipt);
    }
    lane_receipts
}

fn native_tool_command_receipt_success(receipt: &NativeToolReceipt) -> bool {
    receipt.status == "ok"
        && receipt.tool_name == "command_run"
        && receipt
            .result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn native_tool_command_call_text(args: &Value) -> Option<String> {
    if let Some(raw) = args
        .get("cmd")
        .or_else(|| args.get("command"))
        .and_then(Value::as_str)
    {
        return Some(raw.to_string());
    }
    let values = args
        .get("cmd")
        .or_else(|| args.get("command"))
        .and_then(Value::as_array)?;
    let parts = values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if parts.len() >= 3 && parts[0].ends_with("sh") && parts[1] == "-lc" {
        return Some(parts[2].clone());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn native_tool_command_call_cwd(args: &Value) -> Option<PathBuf> {
    args.get("cwd")
        .or_else(|| args.get("path"))
        .or_else(|| args.get("working_directory"))
        .or_else(|| args.get("working_dir"))
        .or_else(|| args.get("workdir"))
        .or_else(|| args.get("directory"))
        .or_else(|| args.get("dir"))
        .or_else(|| args.get("project_root"))
        .or_else(|| args.get("root"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

fn native_tool_command_text_looks_like_validation(command_text: &str) -> bool {
    let lower = command_text.to_ascii_lowercase();
    lower.contains("unittest")
        || lower.contains("pytest")
        || lower.contains("cargo test")
        || lower.contains("npm test")
        || lower.contains("go test")
        || lower.contains("validation")
}

fn native_tool_shell_edit_write_path_candidates(command_text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in command_text.lines() {
        let trimmed = line.trim();
        for marker in ["cat >", "cat  >", "tee "] {
            if let Some((_, rest)) = trimmed.split_once(marker) {
                if let Some(path) = native_tool_shell_edit_first_path_token(rest) {
                    native_tool_push_unique_string(&mut paths, path);
                }
            }
        }
        if trimmed.starts_with("cat <<") || trimmed.starts_with("cat  <<") {
            if let Some((_, rest)) = trimmed.rsplit_once('>') {
                if let Some(path) = native_tool_shell_edit_first_path_token(rest) {
                    native_tool_push_unique_string(&mut paths, path);
                }
            }
        }
        for marker in ["Path(", "pathlib.Path("] {
            if let Some((_, rest)) = trimmed.split_once(marker) {
                if let Some(path) = native_tool_shell_edit_quoted_path_before(rest, ")") {
                    if trimmed.contains(".write_text")
                        || trimmed.contains(".write_bytes")
                        || trimmed.contains(".write(")
                    {
                        native_tool_push_unique_string(&mut paths, path);
                    }
                }
            }
        }
        if let Some((_, rest)) = trimmed.split_once("open(") {
            if trimmed.contains(", \"w")
                || trimmed.contains(", 'w")
                || trimmed.contains(",\"w")
                || trimmed.contains(",'w")
            {
                if let Some(path) = native_tool_shell_edit_quoted_path_before(rest, ",") {
                    native_tool_push_unique_string(&mut paths, path);
                }
            }
        }
    }
    paths
}

fn native_tool_shell_command_control_text(command_text: &str) -> String {
    let mut lines = Vec::new();
    let mut heredoc_end: Option<String> = None;
    for line in command_text.lines() {
        let trimmed = line.trim();
        if let Some(end) = heredoc_end.as_deref() {
            if trimmed == end {
                heredoc_end = None;
            }
            continue;
        }
        lines.push(trimmed.to_string());
        if let Some(marker) = native_tool_shell_heredoc_end_marker(trimmed) {
            heredoc_end = Some(marker);
        }
    }
    lines.join("\n")
}

fn native_tool_shell_heredoc_end_marker(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("<<")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('-').unwrap_or(rest).trim_start();
    let marker = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(';')
        .trim_matches('"')
        .trim_matches('\'')
        .trim();
    if marker.is_empty() {
        None
    } else {
        Some(marker.to_string())
    }
}

fn native_tool_shell_edit_quoted_path_before(rest: &str, delimiter: &str) -> Option<String> {
    let rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &rest[quote.len_utf8()..];
    let end = body.find(quote)?;
    let path = &body[..end];
    let after = body[end + quote.len_utf8()..].trim_start();
    if !after.starts_with(delimiter) {
        return None;
    }
    if path.trim().is_empty() {
        None
    } else {
        Some(path.trim().to_string())
    }
}

fn native_tool_shell_edit_first_path_token(rest: &str) -> Option<String> {
    let mut rest = rest.trim();
    if let Some(stripped) = rest.strip_prefix("-a ") {
        rest = stripped.trim();
    }
    let before_heredoc = rest.split("<<").next().unwrap_or(rest);
    let token = before_heredoc
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(';')
        .trim();
    if token.is_empty() || token == ">" || token.starts_with('&') || token.starts_with('|') {
        None
    } else {
        Some(token.to_string())
    }
}

fn native_tool_path_is_within_root(path: &Path, root: &Path) -> bool {
    let normalized_path = native_tool_normalize_path_without_fs(path);
    let normalized_root = native_tool_normalize_path_without_fs(root);
    normalized_path.starts_with(&normalized_root)
}

fn native_tool_normalize_path_without_fs(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn native_tool_shell_edit_path_is_safe_source_path(path: &Path) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    !native_tool_mutation_entry_path_looks_generated(path)
        && (native_tool_mutation_entry_path_is_code(path)
            || native_tool_mutation_entry_path_is_test(path)
            || path.ends_with(".md")
            || path.ends_with(".json")
            || path.ends_with(".txt"))
}

fn native_tool_synthesize_shell_edit_mutation_receipts(
    command_call_id: &str,
    paths: &[PathBuf],
    previous_receipts: &[NativeToolReceipt],
) -> Vec<NativeToolReceipt> {
    paths
        .iter()
        .enumerate()
        .filter_map(|(idx, path)| {
            let bytes = fs::read(path).ok()?;
            let new_hash = native_tool_sha256_hex(&bytes);
            let previous_hash = native_tool_previous_content_hash(previous_receipts, path);
            if previous_hash.as_deref() == Some(new_hash.as_str()) {
                return None;
            }
            Some(NativeToolReceipt {
                call_id: format!("{command_call_id}_shell_edit_mutation_{}", idx + 1),
                tool_name: "file_write".to_string(),
                status: "ok".to_string(),
                duration_ms: 0,
                result: json!({
                    "path": path.display().to_string(),
                    "created": previous_hash.is_none(),
                    "overwritten": previous_hash.is_some(),
                    "previous_content_hash": previous_hash,
                    "new_content_hash": new_hash,
                    "bytes_written": bytes.len(),
                    "synthesized_from_command_run": command_call_id,
                    "controlled_shell_edit_batch": true
                }),
                error: None,
            })
        })
        .collect()
}

fn native_tool_previous_content_hash(
    receipts: &[NativeToolReceipt],
    target_path: &Path,
) -> Option<String> {
    let target = target_path.to_string_lossy();
    for receipt in receipts.iter().rev() {
        if let Some(files) = receipt.result.get("files").and_then(Value::as_array) {
            for file in files {
                if file
                    .get("path")
                    .and_then(Value::as_str)
                    .map(|path| path == target)
                    .unwrap_or(false)
                {
                    return file
                        .get("content_hash")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
            }
        }
        if receipt
            .result
            .get("path")
            .and_then(Value::as_str)
            .map(|path| path == target)
            .unwrap_or(false)
        {
            if let Some(hash) = receipt.result.get("new_content_hash").and_then(Value::as_str) {
                return Some(hash.to_string());
            }
            if let Some(hash) = receipt.result.get("content_hash").and_then(Value::as_str) {
                return Some(hash.to_string());
            }
        }
    }
    None
}

fn native_tool_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn native_tool_mutation_only_recovery_system() -> String {
        "VISIBLE OUTPUT CONTRACT: the first visible byte must be `{`.\n\
Mutation-only recovery turn.\n\
Return exactly one JSON object. For a shell edit batch, prefer {\"actions\":[{\"command\":\"...\"}]}; otherwise use {\"tool_calls\":[...]}.\n\
Use file_patch/file_write, or one shell action/command_run only when it writes project files.\n\
No validation command/read/list/stat/resolve/prose/final answer.\n\
Runtime will run validation after a successful mutation."
        .to_string()
}

fn native_tool_compact_action_controller_system() -> String {
    "Compact local action controller.\n\
Return only JSON: {\"actions\":[{\"command\":\"shell command\"}]}.\n\
Use one shell edit batch that writes project source/test files with complete real content.\n\
No prose, reads, validation, planning, markdown, or final answer.\n\
Runtime validates after a successful mutation."
        .to_string()
}

fn native_tool_staged_edit_system() -> String {
    "Staged local edit turn.\n\
Return only JSON tool_calls.\n\
Use file_patch/file_write only.\n\
No command_run/read/list/stat/resolve/prose.\n\
Patch observed owner files; do not create new files unless explicitly requested."
        .to_string()
}

fn native_tool_validation_guided_compact_repair_system() -> String {
    "VISIBLE OUTPUT CONTRACT: the first visible byte must be `{`.\n\
Validation-guided compact repair turn.\n\
Return exactly one JSON object.\n\
Use file_patch/file_write, or one controlled command_run shell edit only when it writes project files.\n\
Patch observed product/source owner files to satisfy failed validation.\n\
No read/list/stat/resolve/non-edit command_run/prose/final answer."
        .to_string()
}

fn native_tool_validation_guided_compact_repair_prompt(
    metadata: &Value,
    original_prompt: &str,
    recovery_reason: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let packet = native_tool_validation_guided_compact_repair_packet(original_prompt, receipts);
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "validation_guided_compact_repair_rule",
        "Failed validation already identifies the requested local repair. Return one small file_patch/file_write batch against observed product/source owner files. Treat tests/probes as immutable contract evidence unless the user explicitly requested test edits. Do not call read/list/validation tools or finalize; runtime validates after mutation.",
    );
    format!(
        "User task:\n{}\n\n{}\n\nRecovery reason:\n{}\n\n{}",
        original_prompt.trim(),
        rule,
        recovery_reason,
        packet
    )
}

fn native_tool_validation_guided_compact_repair_packet(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let project_root = native_tool_prompt_project_root(original_prompt)
        .unwrap_or_else(|| "the local project root".to_string());
    let project_root_path = PathBuf::from(&project_root);
    let mut source_sections = Vec::<String>::new();
    let mut source_paths = Vec::<String>::new();
    let mut seeded_source_paths = Vec::<String>::new();
    let mut contract_lines = Vec::<String>::new();
    let mut validation_lines = Vec::<String>::new();
    let mut blocked_mutation_lines = Vec::<String>::new();

    for receipt in receipts {
        native_tool_seeded_import_surface_collect_validation_lines(receipt, &mut validation_lines);
        native_tool_validation_guided_collect_blocked_mutation_lines(
            receipt,
            &mut blocked_mutation_lines,
        );
        native_tool_validation_guided_collect_observed_files(
            receipt,
            &project_root_path,
            &mut source_sections,
            &mut source_paths,
            &mut contract_lines,
        );
        native_tool_validation_guided_collect_seeded_source_path(
            receipt,
            &project_root_path,
            &mut seeded_source_paths,
        );
    }

    source_sections.sort();
    source_sections.dedup();
    source_paths.sort_by_key(|path| native_tool_mutation_entry_path_score(path));
    source_paths.dedup();
    seeded_source_paths.sort_by_key(|path| native_tool_mutation_entry_path_score(path));
    seeded_source_paths.dedup();
    let suggested_repair_path =
        native_tool_validation_guided_suggested_repair_path(
            &validation_lines,
            &source_paths,
            &seeded_source_paths,
        )
            .unwrap_or_else(|| "exact observed owner path from the list below".to_string());

    format!(
        "Validation-guided repair packet:\n\
Project root: {project_root}\n\n\
Required response shape:\n\
{{\"tool_calls\":[{{\"id\":\"validation_repair_1\",\"name\":\"file_write\",\"args\":{{\"path\":\"{}\",\"content\":\"complete replacement file content preserving observed public API\",\"overwrite\":true}}}}]}}\n\
Path rule: the `path` value must exactly match an observed product/source owner file path listed below after `---`; do not invent an absolute path when the observed path is relative. Prefer seeded source owner files over package/export shims when validation reports incomplete behavior, missing attributes, or `NotImplementedError`. Prefer `file_write` with complete replacement content for small owner files or runtime seed stubs; use `file_patch` only when exact old text is present and unambiguous.\n\n\
Failed validation evidence:\n{}\n\n\
Previous blocked mutation evidence:\n{}\n\n\
Immutable contract evidence from tests/probes:\n{}\n\n\
Observed product/source owner files to patch first:\n{}",
        suggested_repair_path,
        if validation_lines.is_empty() {
            "- validation command failed; no compact stderr/stdout excerpt was available".to_string()
        } else {
            validation_lines
                .iter()
                .take(24)
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if blocked_mutation_lines.is_empty() {
            "- no previous blocked mutation receipt".to_string()
        } else {
            blocked_mutation_lines
                .iter()
                .take(8)
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if contract_lines.is_empty() {
            "- use the failed validation and user task as the public contract".to_string()
        } else {
            contract_lines
                .iter()
                .take(20)
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if source_sections.is_empty() {
            "- no compact source owner snippet available".to_string()
        } else {
            source_sections
                .iter()
                .take(4)
                .map(|section| section.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    )
}

fn native_tool_validation_guided_collect_blocked_mutation_lines(
    receipt: &NativeToolReceipt,
    blocked_mutation_lines: &mut Vec<String>,
) {
    if receipt.status != "error"
        || !matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
    {
        return;
    }
    let reason = receipt
        .result
        .get("reason")
        .and_then(Value::as_str)
        .or(receipt.error.as_deref())
        .unwrap_or("blocked_mutation");
    let attempted_path = receipt
        .result
        .get("attempted_path")
        .and_then(Value::as_str)
        .unwrap_or("<none>");
    let suggested_path = receipt
        .result
        .get("suggested_path")
        .and_then(Value::as_str)
        .unwrap_or("<none>");
    let owner_paths = receipt
        .result
        .get("observed_owner_paths")
        .and_then(Value::as_array)
        .map(|paths| {
            paths
                .iter()
                .filter_map(Value::as_str)
                .take(6)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let next_action = receipt
        .result
        .get("next_action")
        .and_then(Value::as_str)
        .unwrap_or("return a concrete mutation against an observed owner path");
    blocked_mutation_lines.push(format!(
        "{reason}; attempted_path={attempted_path}; suggested_path={suggested_path}; observed_owner_paths={owner_paths}; next_action={next_action}"
    ));
}

fn native_tool_validation_guided_collect_seeded_source_path(
    receipt: &NativeToolReceipt,
    project_root_path: &Path,
    seeded_source_paths: &mut Vec<String>,
) {
    if receipt.status != "ok"
        || !receipt
            .call_id
            .contains("runtime_python_import_surface_seed_source")
        || !matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
    {
        return;
    }
    if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
        native_tool_push_unique_string(
            seeded_source_paths,
            native_tool_mutation_entry_display_path(project_root_path, path),
        );
    }
}

fn native_tool_validation_guided_collect_observed_files(
    receipt: &NativeToolReceipt,
    project_root_path: &Path,
    source_sections: &mut Vec<String>,
    source_paths: &mut Vec<String>,
    contract_lines: &mut Vec<String>,
) {
    if receipt.tool_name != "file_read" && receipt.tool_name != "file_read_many" {
        return;
    }
    if let Some(files) = receipt.result.get("files").and_then(Value::as_array) {
        for file in files {
            if let Some(path) = native_tool_packet_value_path(file) {
                native_tool_validation_guided_collect_observed_file_content(
                    project_root_path,
                    &path,
                    file.get("content").and_then(Value::as_str),
                    source_sections,
                    source_paths,
                    contract_lines,
                );
            }
        }
    } else if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
        native_tool_validation_guided_collect_observed_file_content(
            project_root_path,
            path,
        receipt.result.get("content").and_then(Value::as_str),
        source_sections,
        source_paths,
        contract_lines,
    );
}
}

fn native_tool_validation_guided_collect_observed_file_content(
    project_root_path: &Path,
    path: &str,
    content: Option<&str>,
    source_sections: &mut Vec<String>,
    source_paths: &mut Vec<String>,
    contract_lines: &mut Vec<String>,
) {
    let Some(content) = content else {
        return;
    };
    let display_path = native_tool_mutation_entry_display_path(project_root_path, path);
    if native_tool_validation_guided_path_looks_test_or_probe(&display_path) {
        native_tool_mutation_entry_extend_public_contract_lines(
            contract_lines,
            &display_path,
            content,
        );
        native_tool_validation_guided_extend_semantic_contract_blocks(
            contract_lines,
            &display_path,
            content,
        );
        return;
    }
    native_tool_push_unique_string(source_paths, display_path.clone());
    source_sections.push(format!(
        "--- {display_path}\n{}",
        native_tool_validation_guided_source_brief(content)
    ));
}

fn native_tool_validation_guided_extend_semantic_contract_blocks(
    contract_lines: &mut Vec<String>,
    display_path: &str,
    content: &str,
) {
    let lines = content.lines().collect::<Vec<_>>();
    let mut emitted = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        if emitted >= 8 {
            break;
        }
        if !native_tool_validation_guided_line_starts_semantic_contract(line) {
            continue;
        }
        let start = idx.saturating_sub(1);
        let mut end = (idx + 8).min(lines.len());
        let mut balance = 0i32;
        for (offset, candidate) in lines[idx..end].iter().enumerate() {
            balance += native_tool_validation_guided_bracket_delta(candidate);
            let trimmed = candidate.trim_end();
            if offset > 0
                && balance <= 0
                && (trimmed.ends_with(')')
                    || trimmed.ends_with(");")
                    || trimmed.ends_with(']')
                    || trimmed.ends_with('}'))
            {
                end = idx + offset + 1;
                break;
            }
        }
        let block = lines[start..end]
            .iter()
            .map(|line| line.trim_end())
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" | ");
        if block.trim().is_empty() {
            continue;
        }
        let compact_block = block.chars().take(700).collect::<String>();
        native_tool_push_unique_string(
            contract_lines,
            format!(
                "{display_path}:{} semantic contract block: {compact_block}",
                start + 1
            ),
        );
        emitted += 1;
    }
}

fn native_tool_validation_guided_line_starts_semantic_contract(line: &str) -> bool {
    let lower = line.trim_start().to_ascii_lowercase();
    lower.contains("assert")
        || lower.contains("expect(")
        || lower.contains("expected")
        || lower.contains("should")
        || lower.contains("must ")
}

fn native_tool_validation_guided_bracket_delta(line: &str) -> i32 {
    let mut delta = 0i32;
    for ch in line.chars() {
        match ch {
            '(' | '[' | '{' => delta += 1,
            ')' | ']' | '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

fn native_tool_validation_guided_suggested_repair_path(
    validation_lines: &[String],
    source_paths: &[String],
    seeded_source_paths: &[String],
) -> Option<String> {
    if source_paths.is_empty() {
        return None;
    }
    let validation_text = validation_lines
        .join("\n")
        .replace('\\', "/")
        .to_ascii_lowercase();
    if native_tool_validation_guided_prefers_seeded_source_owner(&validation_text) {
        for path in seeded_source_paths {
            if source_paths
                .iter()
                .any(|source_path| native_tool_paths_same_or_suffix(source_path, path))
            {
                return Some(path.clone());
            }
        }
    }
    for path in source_paths {
        let normalized = path.replace('\\', "/").to_ascii_lowercase();
        let basename = normalized
            .rsplit('/')
            .next()
            .unwrap_or(normalized.as_str())
            .to_string();
        if validation_text.contains(&normalized) || validation_text.contains(&basename) {
            return Some(path.clone());
        }
    }
    source_paths.first().cloned()
}

fn native_tool_validation_guided_prefers_seeded_source_owner(validation_text: &str) -> bool {
    validation_text.contains("notimplementederror")
        || validation_text.contains("not implemented")
        || validation_text.contains("attributeerror")
        || validation_text.contains("has no attribute")
        || validation_text.contains("assertionerror")
        || validation_text.contains("assertion failed")
}

fn native_tool_validation_guided_path_looks_test_or_probe(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    lower.contains("/tests/")
        || lower.starts_with("tests/")
        || lower.contains("test_")
        || lower.contains("_test.")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("semantic_probe")
}

fn native_tool_validation_guided_source_brief(content: &str) -> String {
    let brief = content.lines().take(80).collect::<Vec<_>>().join("\n");
    brief.chars().take(1800).collect::<String>()
}

fn native_tool_staged_edit_prompt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let observation = native_tool_observation_prompt(receipts);
    let owner_hint = native_tool_edit_owner_hint(receipts);
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "staged_edit_controller_rule",
        "Use the receipt-backed context and failing validation evidence. Return only file_patch/file_write tool_calls for the smallest safe product/API edit. Runtime will run validation after mutation. Do not include command_run, reads, planning prose, or final answer.",
    );
    format!(
        "User task:\n{}\n\n{}\n{}\n\nNative tool observations:\n{}",
        original_prompt.trim(),
        rule,
        owner_hint.trim(),
        observation
    )
}

fn native_tool_staged_edit_tools(tools: &[String]) -> Vec<String> {
    let filtered = tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "file_write"
                    | "write_file"
                    | "workspace.write"
                    | "workspace_write"
                    | "file_patch"
                    | "patch_file"
                    | "workspace.patch"
                    | "workspace_patch"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        tools.to_vec()
    } else {
        filtered
    }
}

fn native_tool_seeded_import_surface_repair_system() -> String {
    "Seeded import-surface repair turn.\n\
Return only JSON tool_calls.\n\
Use file_write only with overwrite=true.\n\
No file_patch/read/list/stat/resolve/command_run/prose/final answer.\n\
Overwrite the seeded source/export files with complete real content; runtime will validate."
        .to_string()
}

fn native_tool_seeded_import_surface_repair_prompt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let contract_packet = if native_tool_seeded_import_surface_contract_packet_enabled(metadata) {
        native_tool_seeded_import_surface_contract_packet(original_prompt, receipts)
    } else {
        String::new()
    };
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "seeded_import_surface_repair_rule",
        "Runtime created a minimal Python source/export import surface from failed validation evidence. That seed is not completion. Treat tests/probes/import lines as immutable contract evidence. Return one small file_write-only batch overwriting the seeded source/export owner files with complete real source. Do not call read/list/validation tools or finalize; runtime validates after mutation.",
    );
    let shape = "Required response shape:\n\
{\"tool_calls\":[{\"id\":\"write_source\",\"name\":\"file_write\",\"args\":{\"path\":\"COPY_ONE_OWNER_MUTATION_TARGET_LISTED_BELOW\",\"content\":\"complete file content\",\"overwrite\":true}}]}\n\
The path value must be copied from Owner mutation targets below; do not copy the placeholder token.";
    let contract_section = if contract_packet.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n{contract_packet}")
    };
    format!(
        "Task brief:\nComplete the seeded public API implementation required by the immutable validation/probe contract below. Preserve existing behavior shown in source skeletons.\n\n{}\n\n{}{}",
        rule,
        shape,
        contract_section
    )
}

fn native_tool_seeded_import_surface_contract_packet(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let project_root = native_tool_prompt_project_root(original_prompt)
        .unwrap_or_else(|| "the local project root".to_string());
    let project_root_path = PathBuf::from(&project_root);
    let mut seeded_paths = Vec::<String>::new();
    let mut seed_source_sections = Vec::<String>::new();
    let mut public_contract_lines = Vec::<String>::new();
    let mut validation_lines = Vec::<String>::new();

    for receipt in receipts {
        native_tool_seeded_import_surface_collect_contract_file(
            receipt,
            &project_root_path,
            &mut seeded_paths,
            &mut seed_source_sections,
            &mut public_contract_lines,
        );
        native_tool_seeded_import_surface_collect_validation_lines(
            receipt,
            &mut validation_lines,
        );
    }

    seeded_paths.sort_by_key(|path| native_tool_mutation_entry_path_score(path));
    seeded_paths.dedup();
    seed_source_sections.sort();
    seed_source_sections.dedup();

    format!(
        "Seeded import-surface implementation contract:\n\
Owner mutation targets:\n{}\n\n\
Immutable public contract evidence from tests/probes/exports:\n{}\n\n\
Failed validation/import evidence:\n{}\n\n\
Current seeded source/export skeletons:\n{}",
        if seeded_paths.is_empty() {
            "- source/export owner paths inferred from latest seed receipts".to_string()
        } else {
            seeded_paths
                .iter()
                .take(8)
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if public_contract_lines.is_empty() {
            "- use the user task and import failure evidence as the contract".to_string()
        } else {
            public_contract_lines
                .iter()
                .take(24)
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if validation_lines.is_empty() {
            "- no compact validation excerpt available".to_string()
        } else {
            validation_lines
                .iter()
                .take(20)
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if seed_source_sections.is_empty() {
            "- no seeded source skeleton content available".to_string()
        } else {
            seed_source_sections
                .iter()
                .take(4)
                .map(|section| section.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    )
}

fn native_tool_seeded_import_surface_collect_contract_file(
    receipt: &NativeToolReceipt,
    project_root_path: &Path,
    seeded_paths: &mut Vec<String>,
    seed_source_sections: &mut Vec<String>,
    public_contract_lines: &mut Vec<String>,
) {
    if let Some(files) = receipt.result.get("files").and_then(Value::as_array) {
        for file in files {
            if let Some(path) = native_tool_packet_value_path(file) {
                native_tool_seeded_import_surface_collect_contract_content(
                    receipt,
                    project_root_path,
                    &path,
                    file.get("content").and_then(Value::as_str),
                    seeded_paths,
                    seed_source_sections,
                    public_contract_lines,
                );
            }
        }
    }
    if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
        native_tool_seeded_import_surface_collect_contract_content(
            receipt,
            project_root_path,
            path,
            receipt.result.get("content").and_then(Value::as_str),
            seeded_paths,
            seed_source_sections,
            public_contract_lines,
        );
    }
}

fn native_tool_seeded_import_surface_collect_contract_content(
    receipt: &NativeToolReceipt,
    project_root_path: &Path,
    path: &str,
    content: Option<&str>,
    seeded_paths: &mut Vec<String>,
    seed_source_sections: &mut Vec<String>,
    public_contract_lines: &mut Vec<String>,
) {
    let display_path = native_tool_mutation_entry_display_path(project_root_path, path);
    if let Some(content) = content {
        native_tool_mutation_entry_extend_public_contract_lines(
            public_contract_lines,
            &display_path,
            content,
        );
    }
    if receipt.call_id.contains("runtime_python_import_surface_seed") {
        native_tool_push_unique_string(seeded_paths, display_path.clone());
        if let Some(content) = content {
            seed_source_sections.push(format!(
                "--- {display_path}\n{}",
                native_tool_validation_guided_source_brief(content)
            ));
        }
    }
}

fn native_tool_seeded_import_surface_collect_validation_lines(
    receipt: &NativeToolReceipt,
    validation_lines: &mut Vec<String>,
) {
    let mut text = String::new();
    if let Some(error) = receipt.error.as_deref() {
        text.push_str(error);
        text.push('\n');
    }
    for key in ["stdout", "stderr", "message", "summary", "diagnostic"] {
        if let Some(value) = receipt.result.get(key).and_then(Value::as_str) {
            text.push_str(value);
            text.push('\n');
        }
    }
    if text.trim().is_empty() {
        return;
    }
    for line in text.lines() {
        let trimmed = line.trim();
        if !native_tool_seeded_import_surface_validation_line_is_useful(trimmed) {
            continue;
        }
        native_tool_push_unique_string(
            validation_lines,
            trimmed.chars().take(240).collect::<String>(),
        );
        if validation_lines.len() >= 48 {
            break;
        }
    }
}

fn native_tool_seeded_import_surface_validation_line_is_useful(line: &str) -> bool {
    if line.is_empty() || line.len() > 500 {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    lower.contains("importerror")
        || lower.contains("modulenotfounderror")
        || lower.contains("attributeerror")
        || lower.contains("nameerror")
        || lower.contains("notimplementederror")
        || lower.contains("assertionerror")
        || lower.contains("expected")
        || lower.contains("actual")
        || lower.contains("failed")
        || lower.contains("traceback")
        || lower.contains("cannot import")
        || lower.contains("no module named")
        || lower.contains("assert")
        || line.contains("from ")
        || line.contains("import ")
}

fn native_tool_first_edit_batch_system() -> String {
    "First edit batch contract.\n\
Return only JSON tool_calls.\n\
Use file_patch/file_write before command_run.\n\
No read/list/stat/resolve; blocker only if receipts prove unsafe."
        .to_string()
}

fn native_tool_first_edit_batch_prompt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> String {
    let observation = native_tool_observation_prompt(receipts);
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "first_edit_batch_contract_rule",
        "Use preflight observations. Return only JSON tool_calls. Mutate first with file_write/file_patch, then validate. No discovery/prose; blocker only if receipts prove unsafe.",
    );
    format!(
        "User task:\n{}\n\nReceipt-backed preflight observations:\n{}\n\n{}",
        original_prompt.trim(),
        observation,
        rule
    )
}

fn native_tool_bounded_fast_edit_preflight_tools(tools: &[String]) -> Vec<String> {
    let filtered = tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "file_write"
                    | "write_file"
                    | "workspace.write"
                    | "workspace_write"
                    | "file_patch"
                    | "patch_file"
                    | "workspace.patch"
                    | "workspace_patch"
                    | "command_run"
                    | "run_command"
                    | "command.run"
                    | "shell.run"
                    | "shell_run"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        tools.to_vec()
    } else {
        filtered
    }
}

fn native_tool_bounded_fast_edit_preflight_context_blocked_receipt(
    call: NativeToolCall,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "bounded_fast_edit_preflight",
            "reason": "preflight_context_already_loaded_first_mutation_required",
            "required_next_tool": "file_write_or_file_patch"
        }),
        error: Some("preflight_context_already_loaded_first_mutation_required".to_string()),
    }
}

fn native_tool_staged_edit_context_blocked_receipt(
    call: NativeToolCall,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "staged_edit_controller",
            "reason": "staged_edit_uses_preloaded_context",
            "required_next_tool": "file_write_or_file_patch"
        }),
        error: Some("staged_edit_uses_preloaded_context".to_string()),
    }
}

fn native_tool_staged_edit_command_blocked_receipt(
    call: NativeToolCall,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "staged_edit_controller",
            "reason": "runtime_runs_validation_after_mutation",
            "required_next_tool": "file_write_or_file_patch"
        }),
        error: Some("runtime_runs_validation_after_mutation".to_string()),
    }
}

fn native_tool_undeclared_tool_blocked_receipt(
    call: &NativeToolCall,
    request_tools: &[String],
) -> Option<NativeToolReceipt> {
    if request_tools.is_empty()
        || request_tools
            .iter()
            .any(|tool| native_tool_tool_family(tool) == native_tool_tool_family(&call.name))
    {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "declared_tool_surface",
            "reason": "tool_not_available_in_current_runtime_lane",
            "requested_tool": call.name.trim(),
            "available_tools": request_tools,
        }),
        error: Some("tool_not_available_in_current_runtime_lane".to_string()),
    })
}

fn native_tool_tool_family(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "file_list" | "list_files" | "workspace.list" | "workspace_list" => "file_list".to_string(),
        "file_stat" | "stat_file" | "file_exists" | "workspace.stat" | "workspace_stat" => {
            "file_stat".to_string()
        }
        "file_read" | "read_file" | "workspace.read" | "workspace_read" => "file_read".to_string(),
        "file_read_many" | "read_many_files" | "workspace.read_many" | "workspace_read_many" => {
            "file_read_many".to_string()
        }
        "file_write" | "write_file" | "workspace.write" | "workspace_write" => "file_write".to_string(),
        "file_patch" | "patch_file" | "workspace.patch" | "workspace_patch" => "file_patch".to_string(),
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run" => {
            "command_run".to_string()
        }
        "command_resolve" | "resolve_command" | "command.resolve" | "executable_resolve"
        | "executable.resolve" => "command_resolve".to_string(),
        _ => name.trim().to_ascii_lowercase(),
    }
}

fn native_tool_mutation_only_command_blocked_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "mutation_only_recovery",
            "reason": "mutation_only_recovery_requires_edit_before_non_edit_command",
            "required_next_tool": "file_write_or_file_patch_or_controlled_shell_edit"
        }),
        error: Some(
            "mutation_only_recovery_requires_edit_before_non_edit_command".to_string(),
        ),
    }
}

fn native_tool_validation_guided_command_blocked_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "validation_guided_compact_repair",
            "reason": "validation_guided_repair_requires_edit_before_non_edit_command",
            "required_next_tool": "file_write_or_file_patch_or_controlled_shell_edit"
        }),
        error: Some(
            "validation_guided_repair_requires_edit_before_non_edit_command".to_string(),
        ),
    }
}

fn native_tool_first_edit_batch_command_blocked_receipt(
    call: NativeToolCall,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "first_edit_batch_contract",
            "reason": "first_edit_batch_requires_mutation_before_command",
            "required_next_tool": "file_write_or_file_patch"
        }),
        error: Some("first_edit_batch_requires_mutation_before_command".to_string()),
    }
}

fn native_tool_atomic_dependent_edit_blocked_receipt(
    receipts: &[NativeToolReceipt],
    call: &NativeToolCall,
) -> Option<NativeToolReceipt> {
    let unresolved_paths = native_tool_unresolved_preserved_api_additive_patch_paths(receipts);
    if unresolved_paths.is_empty() || !native_tool_call_is_export_surface_mutation(call) {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "atomic_dependent_edit_guard",
            "reason": "dependent_export_requires_successful_product_source_mutation",
            "path": native_tool_call_path_arg(call),
            "unresolved_product_source_paths": unresolved_paths,
            "required_next_tool": "file_patch_or_file_write_on_blocked_product_source"
        }),
        error: Some("dependent_export_requires_successful_product_source_mutation".to_string()),
    })
}

fn native_tool_unresolved_owner_source_first_blocked_receipt(
    receipts: &[NativeToolReceipt],
    call: &NativeToolCall,
    shell_edit_paths: &[PathBuf],
) -> Option<NativeToolReceipt> {
    let unresolved_paths = native_tool_unresolved_preserved_api_additive_patch_paths(receipts);
    if unresolved_paths.is_empty()
        || native_tool_call_targets_unresolved_owner_source(call, shell_edit_paths, &unresolved_paths)
    {
        return None;
    }
    let blocks_file_mutation = native_tool_call_is_file_mutation(call);
    let blocks_shell_mutation = !shell_edit_paths.is_empty();
    let blocks_validation = native_tool_call_is_validation_or_probe_command(call);
    if !blocks_file_mutation && !blocks_shell_mutation && !blocks_validation {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "unresolved_owner_source_first_guard",
            "reason": "resolve_owner_source_before_dependent_edits_or_validation",
            "unresolved_product_source_paths": unresolved_paths,
            "attempted_path": native_tool_call_path_arg(call),
            "attempted_shell_edit_paths": shell_edit_paths,
            "required_next_tool": "file_patch_or_file_write_on_unresolved_product_source_owner"
        }),
        error: Some("resolve_owner_source_before_dependent_edits_or_validation".to_string()),
    })
}

fn native_tool_call_targets_unresolved_owner_source(
    call: &NativeToolCall,
    shell_edit_paths: &[PathBuf],
    unresolved_paths: &[String],
) -> bool {
    if let Some(path) = native_tool_call_path_arg(call) {
        if unresolved_paths
            .iter()
            .any(|unresolved| native_tool_paths_same_or_suffix(unresolved, &path))
        {
            return true;
        }
    }
    shell_edit_paths.iter().any(|path| {
        let path = path.display().to_string();
        unresolved_paths
            .iter()
            .any(|unresolved| native_tool_paths_same_or_suffix(unresolved, &path))
    })
}

fn native_tool_call_is_file_mutation(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    )
}

fn native_tool_call_is_validation_or_probe_command(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_command_run(call) {
        return false;
    }
    let Some(command_text) = native_tool_command_call_text(&call.args) else {
        return false;
    };
    let lower = command_text.to_ascii_lowercase();
    native_tool_command_text_looks_like_validation(&command_text)
        || lower.contains("semantic_probe")
        || lower.contains("pytest")
        || lower.contains("unittest")
}

fn native_tool_unresolved_preserved_api_additive_patch_paths(
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut unresolved: Vec<String> = Vec::new();
    for receipt in receipts {
        let path = receipt
            .result
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string);
        if receipt.error.as_deref() == Some("preserved_api_existing_source_requires_additive_patch")
        {
            if let Some(path) = path {
                if !unresolved
                    .iter()
                    .any(|existing| native_tool_paths_same_or_suffix(existing, &path))
                {
                    unresolved.push(path);
                }
            }
            continue;
        }
        if receipt.error.as_deref() == Some("shell_edit_existing_source_requires_additive_patch") {
            if let Some(changed_blocks) = receipt
                .result
                .get("changed_public_api_blocks")
                .and_then(Value::as_array)
            {
                for block in changed_blocks.iter().filter_map(Value::as_str) {
                    let Some((block_path, _name)) = block.rsplit_once(':') else {
                        continue;
                    };
                    if !unresolved
                        .iter()
                        .any(|existing| native_tool_paths_same_or_suffix(existing, block_path))
                    {
                        unresolved.push(block_path.to_string());
                    }
                }
            } else if let Some(path) = path {
                if !unresolved
                    .iter()
                    .any(|existing| native_tool_paths_same_or_suffix(existing, &path))
                {
                    unresolved.push(path);
                }
            }
            continue;
        }
        if receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
        {
            if let Some(path) = path {
                unresolved
                    .retain(|blocked| !native_tool_paths_same_or_suffix(blocked, &path));
            }
        }
    }
    unresolved
}

fn native_tool_call_is_export_surface_mutation(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_mutation(call) {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call) else {
        return false;
    };
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    lower.ends_with("/__init__.py")
        || lower.ends_with("/index.ts")
        || lower.ends_with("/index.tsx")
        || lower.ends_with("/index.js")
        || lower.ends_with("/index.jsx")
        || lower.ends_with("/mod.rs")
}

fn native_tool_bounded_direct_edit_max_files(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_direct_edit_max_files"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 16) as usize
}

fn native_tool_bounded_direct_edit_lane_active(metadata: &Value, original_prompt: &str) -> bool {
    if !native_tool_bounded_direct_edit_lane_enabled(metadata)
        || !native_tool_requires_successful_mutation(metadata)
    {
        return false;
    }
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if prompt_lower.contains("create a new project")
        || prompt_lower.contains("initialize a new project")
        || prompt_lower.contains("from scratch")
    {
        return false;
    }
    let Some(project_root) = native_tool_prompt_project_root(original_prompt) else {
        return false;
    };
    let root = PathBuf::from(project_root);
    if !root.is_dir() {
        return false;
    }
    let mut paths = native_tool_unique_code_path_mentions(original_prompt)
        .into_iter()
        .filter_map(|path| {
            let candidate = if path.starts_with('/') {
                PathBuf::from(path)
            } else {
                root.join(path.trim_start_matches("./"))
            };
            if candidate.is_file() {
                Some(candidate.display().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    paths.extend(native_tool_bootstrap_likely_context_paths(&root, &paths));
    paths.sort();
    paths.dedup();
    !paths.is_empty() && paths.len() <= native_tool_bounded_direct_edit_max_files(metadata)
}

fn native_tool_bounded_direct_edit_initial_prompt(
    metadata: &Value,
    original_prompt: &str,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "bounded_direct_edit_lane_rule",
        "Use the bounded direct edit lane for this local coding task. Keep the run concrete and receipt-backed: read bounded local context, make the smallest native file_patch/file_write mutations needed, run requested validation/probe commands, and stop with a receipt-backed final answer. Return JSON tool calls whenever tools are needed.",
    );
    format!("{}\n\n{}", original_prompt, rule)
}

fn native_tool_bounded_patch_artifact_max_files(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_max_files"))
        .and_then(Value::as_u64)
        .unwrap_or(6)
        .clamp(1, 12) as usize
}

fn native_tool_bounded_patch_artifact_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(60)
        .clamp(10, 180)
}

fn native_tool_bounded_patch_artifact_strict_retry_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_strict_retry"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_bounded_patch_artifact_timeout_strict_retry_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_timeout_strict_retry"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_bounded_patch_artifact_retry_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_retry_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(25)
        .clamp(10, 90)
}

fn native_tool_bounded_patch_open_loop_after_artifact_failure(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bounded_patch_artifact_open_loop_after_retry_failure"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_small_scoped_edit_artifact_lane_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("small_scoped_edit_artifact_lane"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_small_scoped_edit_max_files(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("small_scoped_edit_max_files"))
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, 8) as usize
}

fn native_tool_small_scoped_edit_max_context_bytes(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("small_scoped_edit_max_context_bytes"))
        .and_then(Value::as_u64)
        .unwrap_or(40_000)
        .clamp(4_000, 120_000)
}

fn native_tool_small_scoped_edit_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("small_scoped_edit_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(45)
        .clamp(10, 120)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeToolBoundedPatchArtifactProfile {
    GeneralPatchArtifact,
    SmallScopedEditArtifact,
}

impl NativeToolBoundedPatchArtifactProfile {
    fn as_str(self) -> &'static str {
        match self {
            NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact => "bounded_patch_artifact",
            NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact => {
                "small_scoped_edit_artifact"
            }
        }
    }
}

fn native_tool_bounded_patch_artifact_profile(
    metadata: &Value,
    context_paths: &[PathBuf],
) -> NativeToolBoundedPatchArtifactProfile {
    if !native_tool_small_scoped_edit_artifact_lane_enabled(metadata) {
        return NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact;
    }
    if context_paths.is_empty()
        || context_paths.len() > native_tool_small_scoped_edit_max_files(metadata)
    {
        return NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact;
    }
    let context_bytes = context_paths
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum::<u64>();
    if context_bytes > native_tool_small_scoped_edit_max_context_bytes(metadata) {
        return NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact;
    }
    NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact
}

fn native_tool_bounded_patch_elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn native_tool_bounded_patch_phase_latency_json(
    lane_started: Instant,
    context_read_ms: u64,
    file_context_ms: u64,
    model_call_ms: u64,
    patch_apply_ms: u64,
    validation_ms: u64,
) -> Value {
    json!({
        "context_read_ms": context_read_ms,
        "file_context_ms": file_context_ms,
        "model_call_ms": model_call_ms,
        "patch_apply_ms": patch_apply_ms,
        "validation_ms": validation_ms,
        "total_ms": native_tool_bounded_patch_elapsed_ms(lane_started),
    })
}

fn native_tool_bounded_patch_artifact_lane(
    provider: &Arc<dyn crate::provider::ProviderClient>,
    dispatcher: &NativeToolDispatcher,
    tools: &[String],
    model: Option<String>,
    metadata: &Value,
    original_prompt: &str,
    system: &str,
) -> Result<NativeToolBoundedPatchLaneOutcome, ProviderError> {
    let lane_started = Instant::now();
    let Some(project_root) = native_tool_prompt_project_root(original_prompt).map(PathBuf::from)
    else {
        return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
            "no_project_root",
            json!({ "reason": "prompt_did_not_contain_existing_local_project_root" }),
        ));
    };
    let context_paths =
        native_tool_bounded_patch_context_paths(metadata, original_prompt, &project_root);
    if context_paths.is_empty() {
        return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
            "no_context_paths",
            json!({
                "project_root": project_root.display().to_string(),
                "reason": "no_confident_bounded_patch_context_paths",
            }),
        ));
    }

    let mut receipts = Vec::<NativeToolReceipt>::new();
    let read_started = Instant::now();
    let read_receipt = dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "bounded_patch_artifact_file_read_many".to_string(),
        name: "file_read_many".to_string(),
        args: json!({
            "paths": context_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
        }),
    });
    let context_read_ms = native_tool_bounded_patch_elapsed_ms(read_started);
    if read_receipt.status != "ok" {
        return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
            "file_read_many_failed",
            json!({
                "context_paths": context_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>(),
                "read_error": read_receipt.error,
                "read_result": read_receipt.result,
                "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                    lane_started,
                    context_read_ms,
                    0,
                    0,
                    0,
                    0
                ),
            }),
        ));
    }
    receipts.push(read_receipt);

    let file_context_started = Instant::now();
    let Some(file_context) = native_tool_bounded_patch_file_context(&context_paths) else {
        return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
            "file_context_unavailable",
            json!({
                "context_paths": context_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>(),
                "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                    lane_started,
                    context_read_ms,
                    native_tool_bounded_patch_elapsed_ms(file_context_started),
                    0,
                    0,
                    0
                ),
            }),
        ));
    };
    let file_context_ms = native_tool_bounded_patch_elapsed_ms(file_context_started);
    let mut artifact_profile = native_tool_bounded_patch_artifact_profile(metadata, &context_paths);
    let mut attempted_artifact_profiles = vec![artifact_profile.as_str().to_string()];
    let mut patch_metadata = metadata.clone();
    if let Some(object) = patch_metadata.as_object_mut() {
        object.insert(
            "provider_timeout_seconds".to_string(),
            json!(match artifact_profile {
                NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact =>
                    native_tool_bounded_patch_artifact_provider_timeout_seconds(metadata),
                NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact =>
                    native_tool_small_scoped_edit_provider_timeout_seconds(metadata),
            }),
        );
        object.insert(
            "native_bounded_patch_artifact_lane".to_string(),
            json!(true),
        );
        object.insert(
            "native_patch_artifact_profile".to_string(),
            json!(artifact_profile.as_str()),
        );
    }
    let patch_prompt = native_tool_bounded_patch_artifact_prompt(
        metadata,
        original_prompt,
        &file_context,
        artifact_profile,
    );
    let request = ProviderRequest {
        prompt: patch_prompt,
        system: Some(native_tool_bounded_patch_artifact_system_prompt(
            artifact_profile,
        )),
        tools: Vec::new(),
        model: model.clone(),
        metadata: patch_metadata,
    };
    let mut provider_call_count = 1u64;
    let model_call_started = Instant::now();
    let mut response = match provider.complete(&request) {
        Ok(response) => response,
        Err(error)
            if native_tool_provider_error_is_timeout(&error)
                && artifact_profile
                    == NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact =>
        {
            artifact_profile = NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact;
            attempted_artifact_profiles.push(artifact_profile.as_str().to_string());
            provider_call_count += 1;
            let mut retry_metadata = metadata.clone();
            if let Some(object) = retry_metadata.as_object_mut() {
                object.insert(
                    "provider_timeout_seconds".to_string(),
                    json!(native_tool_bounded_patch_artifact_provider_timeout_seconds(
                        metadata
                    )),
                );
                object.insert(
                    "native_bounded_patch_artifact_lane".to_string(),
                    json!(true),
                );
                object.insert(
                    "native_patch_artifact_profile".to_string(),
                    json!(artifact_profile.as_str()),
                );
            }
            let retry_request = ProviderRequest {
                prompt: native_tool_bounded_patch_artifact_prompt(
                    metadata,
                    original_prompt,
                    &file_context,
                    artifact_profile,
                ),
                system: Some(native_tool_bounded_patch_artifact_system_prompt(
                    artifact_profile,
                )),
                tools: Vec::new(),
                model: model.clone(),
                metadata: retry_metadata,
            };
            match provider.complete(&retry_request) {
                Ok(response) => response,
                Err(retry_error) if native_tool_provider_error_is_timeout(&retry_error) => {
                    native_tool_persist_run_journal(
                        metadata,
                        original_prompt,
                        "bounded_patch_artifact_fallback",
                        provider_call_count,
                        &receipts,
                        None,
                        Some("provider_timeout_before_patch_artifact"),
                    );
                    return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
                        "provider_timeout_before_patch_artifact",
                        json!({
                            "provider_error": retry_error.message,
                            "first_profile_timeout": error.message,
                            "context_paths": context_paths
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect::<Vec<_>>(),
                            "artifact_profile": artifact_profile.as_str(),
                            "attempted_artifact_profiles": attempted_artifact_profiles,
                            "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                                lane_started,
                                context_read_ms,
                                file_context_ms,
                                native_tool_bounded_patch_elapsed_ms(model_call_started),
                                0,
                                0
                            ),
                        }),
                    ));
                }
                Err(retry_error) => return Err(retry_error),
            }
        }
        Err(error) if native_tool_provider_error_is_timeout(&error) => {
            let first_timeout = error.message.clone();
            if native_tool_bounded_patch_artifact_strict_retry_enabled(metadata)
                && native_tool_bounded_patch_artifact_timeout_strict_retry_enabled(metadata)
                && provider_call_count < 2
            {
                attempted_artifact_profiles.push(format!(
                    "{}_timeout_strict_retry",
                    artifact_profile.as_str()
                ));
                provider_call_count += 1;
                let mut retry_metadata = metadata.clone();
                if let Some(object) = retry_metadata.as_object_mut() {
                    object.insert(
                        "provider_timeout_seconds".to_string(),
                        json!(
                            native_tool_bounded_patch_artifact_retry_provider_timeout_seconds(
                                metadata
                            )
                        ),
                    );
                    object.insert(
                        "native_bounded_patch_artifact_lane".to_string(),
                        json!(true),
                    );
                    object.insert(
                        "native_patch_artifact_profile".to_string(),
                        json!(format!(
                            "{}_timeout_strict_retry",
                            artifact_profile.as_str()
                        )),
                    );
                }
                let retry_request = ProviderRequest {
                    prompt: native_tool_bounded_patch_artifact_retry_prompt(
                        metadata,
                        original_prompt,
                        &file_context,
                        "",
                        artifact_profile,
                    ),
                    system: Some(native_tool_bounded_patch_artifact_system_prompt(
                        NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact,
                    )),
                    tools: Vec::new(),
                    model: model.clone(),
                    metadata: retry_metadata,
                };
                match provider.complete(&retry_request) {
                    Ok(response) => response,
                    Err(retry_error) if native_tool_provider_error_is_timeout(&retry_error) => {
                        native_tool_persist_run_journal(
                            metadata,
                            original_prompt,
                            "bounded_patch_artifact_fallback",
                            provider_call_count,
                            &receipts,
                            None,
                            Some("artifact_timeout_strict_retry_timeout"),
                        );
                        let details = json!({
                            "provider_error": retry_error.message,
                            "first_timeout": first_timeout,
                            "context_paths": context_paths
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect::<Vec<_>>(),
                            "artifact_profile": artifact_profile.as_str(),
                            "attempted_artifact_profiles": attempted_artifact_profiles,
                            "open_loop_after_artifact_failure": native_tool_bounded_patch_open_loop_after_artifact_failure(metadata),
                            "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                                lane_started,
                                context_read_ms,
                                file_context_ms,
                                native_tool_bounded_patch_elapsed_ms(model_call_started),
                                0,
                                0
                            ),
                        });
                        if native_tool_bounded_patch_open_loop_after_artifact_failure(metadata) {
                            return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
                                "artifact_timeout_strict_retry_timeout",
                                details,
                            ));
                        }
                        return Ok(NativeToolBoundedPatchLaneOutcome::terminal_failure((
                            native_tool_bounded_patch_artifact_failure_response(
                                "runtime",
                                model.as_deref(),
                                "artifact_timeout_strict_retry_timeout",
                                provider_call_count,
                                &receipts,
                                details,
                            ),
                            receipts,
                            provider_call_count,
                            "artifact_failure".to_string(),
                        )));
                    }
                    Err(retry_error) => return Err(retry_error),
                }
            } else {
                native_tool_persist_run_journal(
                    metadata,
                    original_prompt,
                    "bounded_patch_artifact_fallback",
                    provider_call_count,
                    &receipts,
                    None,
                    Some("provider_timeout_before_patch_artifact"),
                );
                let details = json!({
                    "provider_error": first_timeout,
                    "context_paths": context_paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>(),
                    "artifact_profile": artifact_profile.as_str(),
                    "attempted_artifact_profiles": attempted_artifact_profiles,
                    "open_loop_after_artifact_failure": native_tool_bounded_patch_open_loop_after_artifact_failure(metadata),
                    "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                        lane_started,
                        context_read_ms,
                        file_context_ms,
                        native_tool_bounded_patch_elapsed_ms(model_call_started),
                        0,
                        0
                    ),
                });
                if native_tool_bounded_patch_open_loop_after_artifact_failure(metadata) {
                    return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
                        "provider_timeout_before_patch_artifact",
                        details,
                    ));
                }
                return Ok(NativeToolBoundedPatchLaneOutcome::terminal_failure((
                    native_tool_bounded_patch_artifact_failure_response(
                        "runtime",
                        model.as_deref(),
                        "provider_timeout_before_patch_artifact",
                        provider_call_count,
                        &receipts,
                        details,
                    ),
                    receipts,
                    provider_call_count,
                    "artifact_failure".to_string(),
                )));
            }
        }
        Err(error) => return Err(error),
    };

    let mut patches =
        native_tool_bounded_patch_artifact_patches(&response.output, &project_root, &context_paths);
    if patches.is_empty()
        && artifact_profile == NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact
    {
        artifact_profile = NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact;
        attempted_artifact_profiles.push(artifact_profile.as_str().to_string());
        provider_call_count += 1;
        let mut retry_metadata = metadata.clone();
        if let Some(object) = retry_metadata.as_object_mut() {
            object.insert(
                "provider_timeout_seconds".to_string(),
                json!(native_tool_bounded_patch_artifact_provider_timeout_seconds(
                    metadata
                )),
            );
            object.insert(
                "native_bounded_patch_artifact_lane".to_string(),
                json!(true),
            );
            object.insert(
                "native_patch_artifact_profile".to_string(),
                json!(artifact_profile.as_str()),
            );
        }
        let retry_request = ProviderRequest {
            prompt: native_tool_bounded_patch_artifact_prompt(
                metadata,
                original_prompt,
                &file_context,
                artifact_profile,
            ),
            system: Some(native_tool_bounded_patch_artifact_system_prompt(
                artifact_profile,
            )),
            tools: Vec::new(),
            model: model.clone(),
            metadata: retry_metadata,
        };
        response = match provider.complete(&retry_request) {
            Ok(response) => response,
            Err(error) if native_tool_provider_error_is_timeout(&error) => {
                native_tool_persist_run_journal(
                    metadata,
                    original_prompt,
                    "bounded_patch_artifact_fallback",
                    provider_call_count,
                    &receipts,
                    Some(&response.output),
                    Some("provider_timeout_before_patch_artifact"),
                );
                return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
                    "provider_timeout_before_patch_artifact",
                    json!({
                        "provider_error": error.message,
                        "first_profile_output_preview": response.output.chars().take(1600).collect::<String>(),
                        "context_paths": context_paths
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect::<Vec<_>>(),
                        "artifact_profile": artifact_profile.as_str(),
                        "attempted_artifact_profiles": attempted_artifact_profiles,
                        "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                            lane_started,
                            context_read_ms,
                            file_context_ms,
                            native_tool_bounded_patch_elapsed_ms(model_call_started),
                            0,
                            0
                        ),
                    }),
                ));
            }
            Err(error) => return Err(error),
        };
        patches = native_tool_bounded_patch_artifact_patches(
            &response.output,
            &project_root,
            &context_paths,
        );
    }
    if patches.is_empty()
        && native_tool_bounded_patch_artifact_strict_retry_enabled(metadata)
        && provider_call_count < 2
    {
        attempted_artifact_profiles.push(format!("{}_strict_retry", artifact_profile.as_str()));
        provider_call_count += 1;
        let mut retry_metadata = metadata.clone();
        if let Some(object) = retry_metadata.as_object_mut() {
            object.insert(
                "provider_timeout_seconds".to_string(),
                json!(native_tool_bounded_patch_artifact_retry_provider_timeout_seconds(metadata)),
            );
            object.insert(
                "native_bounded_patch_artifact_lane".to_string(),
                json!(true),
            );
            object.insert(
                "native_patch_artifact_profile".to_string(),
                json!(format!("{}_strict_retry", artifact_profile.as_str())),
            );
        }
        let retry_request = ProviderRequest {
            prompt: native_tool_bounded_patch_artifact_retry_prompt(
                metadata,
                original_prompt,
                &file_context,
                &response.output,
                artifact_profile,
            ),
            system: Some(native_tool_bounded_patch_artifact_system_prompt(
                NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact,
            )),
            tools: Vec::new(),
            model: model.clone(),
            metadata: retry_metadata,
        };
        response = match provider.complete(&retry_request) {
            Ok(response) => response,
            Err(error) if native_tool_provider_error_is_timeout(&error) => {
                native_tool_persist_run_journal(
                    metadata,
                    original_prompt,
                    "bounded_patch_artifact_fallback",
                    provider_call_count,
                    &receipts,
                    Some(&response.output),
                    Some("artifact_strict_retry_timeout"),
                );
                let details = json!({
                    "provider_error": error.message,
                    "first_profile_output_preview": response.output.chars().take(1600).collect::<String>(),
                    "context_paths": context_paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>(),
                    "artifact_profile": artifact_profile.as_str(),
                    "attempted_artifact_profiles": attempted_artifact_profiles,
                    "open_loop_after_artifact_failure": native_tool_bounded_patch_open_loop_after_artifact_failure(metadata),
                    "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                        lane_started,
                        context_read_ms,
                        file_context_ms,
                        native_tool_bounded_patch_elapsed_ms(model_call_started),
                        0,
                        0
                    ),
                });
                if native_tool_bounded_patch_open_loop_after_artifact_failure(metadata) {
                    return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
                        "artifact_strict_retry_timeout",
                        details,
                    ));
                }
                return Ok(NativeToolBoundedPatchLaneOutcome::terminal_failure((
                    native_tool_bounded_patch_artifact_failure_response(
                        "runtime",
                        model.as_deref(),
                        "artifact_strict_retry_timeout",
                        provider_call_count,
                        &receipts,
                        details,
                    ),
                    receipts,
                    provider_call_count,
                    "artifact_failure".to_string(),
                )));
            }
            Err(error) => return Err(error),
        };
        patches = native_tool_bounded_patch_artifact_patches(
            &response.output,
            &project_root,
            &context_paths,
        );
    }
    let model_call_ms = native_tool_bounded_patch_elapsed_ms(model_call_started);
    if patches.is_empty() {
        native_tool_persist_run_journal(
            metadata,
            original_prompt,
            "bounded_patch_artifact_fallback",
            provider_call_count,
            &receipts,
            Some(&response.output),
            Some("missing_or_invalid_patch_artifact"),
        );
        let details = json!({
            "output_preview": response.output.chars().take(1600).collect::<String>(),
            "context_paths": context_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
            "artifact_profile": artifact_profile.as_str(),
            "attempted_artifact_profiles": attempted_artifact_profiles,
            "open_loop_after_artifact_failure": native_tool_bounded_patch_open_loop_after_artifact_failure(metadata),
            "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                lane_started,
                context_read_ms,
                file_context_ms,
                model_call_ms,
                0,
                0
            ),
        });
        if native_tool_bounded_patch_open_loop_after_artifact_failure(metadata) {
            return Ok(NativeToolBoundedPatchLaneOutcome::fallback(
                "missing_or_invalid_patch_artifact",
                details,
            ));
        }
        return Ok(NativeToolBoundedPatchLaneOutcome::terminal_failure((
            native_tool_bounded_patch_artifact_failure_response(
                &response.provider,
                Some(&response.model),
                "missing_or_invalid_patch_artifact",
                provider_call_count,
                &receipts,
                details,
            ),
            receipts,
            provider_call_count,
            "artifact_failure".to_string(),
        )));
    }
    let patch_apply_started = Instant::now();
    for (idx, patch) in patches.into_iter().enumerate() {
        let receipt = dispatcher.dispatch(crate::native_tools::NativeToolCall {
            id: format!("bounded_patch_artifact_patch_{}", idx + 1),
            name: "file_patch".to_string(),
            args: json!({
                "path": patch.path,
                "old": patch.old,
                "new": patch.new,
                "allow_multiple": false
            }),
        });
        receipts.push(receipt);
    }
    let patch_apply_ms = native_tool_bounded_patch_elapsed_ms(patch_apply_started);
    if !native_tool_has_successful_mutation(&receipts) {
        native_tool_persist_run_journal(
            metadata,
            original_prompt,
            "bounded_patch_artifact_fallback",
            provider_call_count,
            &receipts,
            Some(&response.output),
            Some("patch_artifact_apply_failed_before_mutation"),
        );
        return Ok(NativeToolBoundedPatchLaneOutcome::fallback_with_receipts(
            "patch_artifact_apply_failed_before_mutation",
            json!({
                "patch_receipts": native_tool_bounded_patch_lane_receipt_summary(&receipts),
                "artifact_profile": artifact_profile.as_str(),
                "attempted_artifact_profiles": attempted_artifact_profiles,
                "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                    lane_started,
                    context_read_ms,
                    file_context_ms,
                    model_call_ms,
                    patch_apply_ms,
                    0
                ),
            }),
            receipts,
        ));
    }

    let validation_started = Instant::now();
    let mut validation_ms = 0u64;
    if let Some(validation_receipt) =
        native_tool_auto_validation_receipt(dispatcher, original_prompt, &receipts)
    {
        receipts.push(validation_receipt);
        validation_ms = native_tool_bounded_patch_elapsed_ms(validation_started);
    }

    let mut response = response;
    let completion_evidence_ready = native_tool_prompt_evidence_gaps(original_prompt, &receipts)
        .is_empty()
        && (!native_tool_prompt_requires_validation_command(&original_prompt.to_ascii_lowercase())
            || native_tool_has_successful_validation_after_latest_mutation(&receipts));
    let repair_reasons =
        native_tool_runtime_repair_reasons(metadata, original_prompt, &response.output, &receipts);
    if !completion_evidence_ready
        && !repair_reasons.is_empty()
        && native_tool_completion_evidence_repair_enabled(metadata)
    {
        let repaired = native_tool_completion_evidence_repair_loop(
            provider,
            dispatcher,
            tools,
            model,
            metadata,
            original_prompt,
            system,
            response,
            receipts,
            provider_call_count,
            repair_reasons,
        )?;
        response = repaired.0;
        receipts = repaired.1;
        provider_call_count = repaired.2;
    }

    if native_tool_prompt_evidence_gaps(original_prompt, &receipts).is_empty()
        && (!native_tool_prompt_requires_validation_command(&original_prompt.to_ascii_lowercase())
            || native_tool_has_successful_validation_after_latest_mutation(&receipts))
    {
        receipts.push(native_tool_bounded_patch_artifact_success_receipt(json!({
            "artifact_profile": artifact_profile.as_str(),
            "attempted_artifact_profiles": attempted_artifact_profiles,
            "context_paths": context_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
            "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                lane_started,
                context_read_ms,
                file_context_ms,
                model_call_ms,
                patch_apply_ms,
                validation_ms
            ),
        })));
        let mut final_response = native_tool_synthetic_completion_evidence_response(
            &response,
            metadata,
            original_prompt,
            &receipts,
            "bounded_patch_artifact_lane_success",
        );
        final_response.raw = json!({
            "provider_raw": final_response.raw,
            "native_bounded_patch_artifact_lane": {
                "enabled": true,
                "provider_call_count": provider_call_count,
                "tool_call_count": receipts.len(),
                "context_paths": context_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>(),
                "artifact_profile": artifact_profile.as_str(),
                "attempted_artifact_profiles": attempted_artifact_profiles,
                "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                    lane_started,
                    context_read_ms,
                    file_context_ms,
                    model_call_ms,
                    patch_apply_ms,
                    validation_ms
                ),
                "tool_receipts": receipts.clone(),
                "terminal_status": "ok"
            }
        });
        return Ok(NativeToolBoundedPatchLaneOutcome::terminal((
            final_response,
            receipts,
            provider_call_count,
            "ok".to_string(),
        )));
    }

    if !native_tool_bounded_patch_open_loop_after_artifact_failure(metadata) {
        let details = json!({
            "evidence_gaps": native_tool_prompt_evidence_gaps(original_prompt, &receipts),
            "has_successful_validation": native_tool_has_successful_validation_command(&receipts),
            "lane_receipts": native_tool_bounded_patch_lane_receipt_summary(&receipts),
            "artifact_profile": artifact_profile.as_str(),
            "attempted_artifact_profiles": attempted_artifact_profiles,
            "context_paths": context_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
            "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                lane_started,
                context_read_ms,
                file_context_ms,
                model_call_ms,
                patch_apply_ms,
                validation_ms
            ),
        });
        let mut terminal_receipts = receipts;
        terminal_receipts.push(native_tool_bounded_patch_artifact_marker_receipt(
            "unresolved_evidence_after_patch_lane",
            details.clone(),
        ));
        return Ok(NativeToolBoundedPatchLaneOutcome::terminal_failure((
            native_tool_bounded_patch_artifact_failure_response(
                &response.provider,
                Some(&response.model),
                "unresolved_evidence_after_patch_lane",
                provider_call_count,
                &terminal_receipts,
                details,
            ),
            terminal_receipts,
            provider_call_count,
            "runtime".to_string(),
        )));
    }

    Ok(NativeToolBoundedPatchLaneOutcome::fallback_with_receipts(
        "unresolved_evidence_after_patch_lane",
        json!({
            "evidence_gaps": native_tool_prompt_evidence_gaps(original_prompt, &receipts),
            "has_successful_validation": native_tool_has_successful_validation_command(&receipts),
            "lane_receipts": native_tool_bounded_patch_lane_receipt_summary(&receipts),
            "artifact_profile": artifact_profile.as_str(),
            "attempted_artifact_profiles": attempted_artifact_profiles,
            "phase_latency_ms": native_tool_bounded_patch_phase_latency_json(
                lane_started,
                context_read_ms,
                file_context_ms,
                model_call_ms,
                patch_apply_ms,
                validation_ms
            ),
        }),
        receipts,
    ))
}

#[derive(Clone, Debug)]
struct NativeToolBoundedPatchLaneOutcome {
    terminal: Option<(ProviderResponse, Vec<NativeToolReceipt>, u64, String)>,
    observability_receipts: Vec<NativeToolReceipt>,
}

impl NativeToolBoundedPatchLaneOutcome {
    fn terminal(terminal: (ProviderResponse, Vec<NativeToolReceipt>, u64, String)) -> Self {
        Self {
            terminal: Some(terminal),
            observability_receipts: Vec::new(),
        }
    }

    fn fallback(reason: &str, details: Value) -> Self {
        Self {
            terminal: None,
            observability_receipts: vec![native_tool_bounded_patch_artifact_marker_receipt(
                reason, details,
            )],
        }
    }

    fn fallback_with_receipts(
        reason: &str,
        details: Value,
        mut receipts: Vec<NativeToolReceipt>,
    ) -> Self {
        receipts.push(native_tool_bounded_patch_artifact_marker_receipt(
            reason, details,
        ));
        Self {
            terminal: None,
            observability_receipts: receipts,
        }
    }

    fn terminal_failure(terminal: (ProviderResponse, Vec<NativeToolReceipt>, u64, String)) -> Self {
        let mut observability_receipts = terminal.1.clone();
        if !observability_receipts
            .iter()
            .any(|receipt| receipt.tool_name == "bounded_patch_artifact_lane")
        {
            let details = serde_json::from_str::<Value>(&terminal.0.output)
                .unwrap_or_else(|_| json!({ "output_preview": terminal.0.output.chars().take(500).collect::<String>() }));
            let reason = details
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("terminal_failure")
                .to_string();
            observability_receipts.push(native_tool_bounded_patch_artifact_marker_receipt(
                &reason, details,
            ));
        }
        Self {
            terminal: Some(terminal),
            observability_receipts,
        }
    }
}

fn native_tool_bootstrap_observation_prompt(metadata: &Value, receipts: &[NativeToolReceipt]) -> String {
    let full = native_tool_observation_prompt(receipts);
    if full.chars().count() <= native_tool_bootstrap_observation_compact_threshold_chars(metadata) {
        return full;
    }
    native_tool_compact_observation_prompt(receipts)
}

fn native_tool_bootstrap_observation_compact_threshold_chars(metadata: &Value) -> usize {
    metadata
        .get("bootstrap_observation_compact_threshold_chars")
        .or_else(|| {
            metadata.pointer("/workflow/native_success_criteria/bootstrap_observation_compact_threshold_chars")
        })
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(12_000)
}

fn native_tool_compact_observation_prompt(receipts: &[NativeToolReceipt]) -> String {
    let summarized_receipts = receipts
        .iter()
        .rev()
        .take(14)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|receipt| {
            json!({
                "call_id": receipt.call_id,
                "tool_name": receipt.tool_name,
                "status": receipt.status,
                "error": receipt.error,
                "result": native_tool_compact_observation_result(&receipt.result),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "native_tool_observations": summarized_receipts,
        "instruction": "Use these compact receipts as authoritative. Preserve observed paths and public contract evidence. Continue with the smallest required mutation tool calls first."
    })
    .to_string()
}

fn native_tool_compact_observation_result(result: &Value) -> Value {
    let mut object = serde_json::Map::new();
    for key in [
        "path",
        "paths",
        "success",
        "exit_code",
        "cwd",
        "command",
        "reason",
        "attempted_path",
        "observed_owner_paths",
        "suggested_path",
        "suggested_next_tool",
        "next_action",
    ] {
        if let Some(value) = result.get(key) {
            object.insert(key.to_string(), value.clone());
        }
    }
    if let Some(files) = result.get("files").and_then(Value::as_array) {
        object.insert(
            "files".to_string(),
            Value::Array(
                files
                    .iter()
                    .take(4)
                    .map(native_tool_compact_observation_file)
                    .collect(),
            ),
        );
    }
    for (key, limit, tail) in [
        ("content", 1000usize, false),
        ("stdout", 900usize, true),
        ("stderr", 900usize, true),
        ("output", 900usize, true),
    ] {
        if let Some(text) = result.get(key).and_then(Value::as_str) {
            let compact = if tail {
                native_tool_compact_text_tail(text, limit)
            } else {
                native_tool_compact_text_head(text, limit)
            };
            object.insert(format!("{key}_preview"), json!(compact));
        }
    }
    Value::Object(object)
}

fn native_tool_compact_observation_file(file: &Value) -> Value {
    let mut object = serde_json::Map::new();
    for key in ["path", "status", "error"] {
        if let Some(value) = file.get(key) {
            object.insert(key.to_string(), value.clone());
        }
    }
    if let Some(content) = file.get("content").and_then(Value::as_str) {
        object.insert(
            "content_preview".to_string(),
            json!(native_tool_compact_text_head(content, 900)),
        );
    }
    Value::Object(object)
}

fn native_tool_compact_text_head(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

fn native_tool_compact_text_tail(text: &str, limit: usize) -> String {
    let mut chars = text.chars().rev().take(limit).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
}

fn native_tool_bounded_patch_lane_receipt_summary(receipts: &[NativeToolReceipt]) -> Vec<Value> {
    receipts
        .iter()
        .map(|receipt| {
            json!({
                "call_id": receipt.call_id.clone(),
                "tool_name": receipt.tool_name.clone(),
                "status": receipt.status.clone(),
                "error": receipt.error.clone(),
                "path": receipt.result.get("path").cloned().unwrap_or(Value::Null),
                "success": receipt.result.get("success").cloned().unwrap_or(Value::Null),
                "exit_code": receipt.result.get("exit_code").cloned().unwrap_or(Value::Null),
                "stdout_tail": receipt
                    .result
                    .get("stdout")
                    .and_then(Value::as_str)
                    .map(|text| text.chars().rev().take(1200).collect::<String>().chars().rev().collect::<String>())
                    .unwrap_or_default(),
                "stderr_tail": receipt
                    .result
                    .get("stderr")
                    .and_then(Value::as_str)
                    .map(|text| text.chars().rev().take(1200).collect::<String>().chars().rev().collect::<String>())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn native_tool_bounded_patch_artifact_marker_receipt(
    reason: &str,
    details: Value,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: format!(
            "bounded_patch_artifact_lane_{}",
            reason
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect::<String>()
        ),
        tool_name: "bounded_patch_artifact_lane".to_string(),
        status: "ok".to_string(),
        duration_ms: 0,
        result: json!({
            "terminal_status": "fallback",
            "reason": reason,
            "details": details,
        }),
        error: None,
    }
}

fn native_tool_bounded_patch_artifact_success_receipt(details: Value) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: "bounded_patch_artifact_lane_success".to_string(),
        tool_name: "bounded_patch_artifact_lane".to_string(),
        status: "ok".to_string(),
        duration_ms: 0,
        result: json!({
            "terminal_status": "ok",
            "reason": "success",
            "details": details,
        }),
        error: None,
    }
}

fn native_tool_bounded_direct_edit_marker_receipt(
    reason: &str,
    details: Value,
) -> NativeToolReceipt {
    let terminal_status = details
        .get("terminal_status")
        .cloned()
        .unwrap_or_else(|| json!("ok"));
    NativeToolReceipt {
        call_id: format!(
            "bounded_direct_edit_lane_{}",
            reason
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect::<String>()
        ),
        tool_name: "bounded_direct_edit_lane".to_string(),
        status: "ok".to_string(),
        duration_ms: 0,
        result: json!({
            "terminal_status": terminal_status,
            "reason": reason,
            "details": details,
        }),
        error: None,
    }
}

fn native_tool_push_bounded_direct_edit_marker_once(
    receipts: &mut Vec<NativeToolReceipt>,
    reason: &str,
    details: Value,
) {
    let call_id = format!(
        "bounded_direct_edit_lane_{}",
        reason
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>()
    );
    if receipts.iter().any(|receipt| receipt.call_id == call_id) {
        return;
    }
    receipts.push(native_tool_bounded_direct_edit_marker_receipt(
        reason, details,
    ));
}

fn native_tool_bounded_patch_artifact_failure_response(
    provider_id: &str,
    model: Option<&str>,
    reason: &str,
    provider_call_count: u64,
    receipts: &[NativeToolReceipt],
    details: Value,
) -> ProviderResponse {
    let output = format!(
        "Bounded patch artifact generation stopped before entering the open native tool loop: {reason}. Returning structured failure so quick-edit work fails fast instead of drifting into a long recovery loop."
    );
    ProviderResponse {
        provider: provider_id.to_string(),
        model: model.unwrap_or("unknown").to_string(),
        usage_tokens: output.split_whitespace().count() as u64,
        output,
        raw: json!({
            "ok": false,
            "terminal_status": "artifact_failure",
            "reason": reason,
            "provider_call_count": provider_call_count,
            "native_tool_call_count": receipts.len(),
            "details": details,
        }),
    }
}

#[derive(Clone, Debug)]
struct NativeToolBoundedPatch {
    path: String,
    old: String,
    new: String,
}

fn native_tool_bounded_patch_context_paths(
    metadata: &Value,
    original_prompt: &str,
    project_root: &Path,
) -> Vec<PathBuf> {
    let max_files = native_tool_bounded_patch_artifact_max_files(metadata);
    let mut paths = native_tool_prompt_required_changed_paths(original_prompt)
        .into_iter()
        .filter_map(|path| native_tool_resolve_bounded_patch_context_path(project_root, &path))
        .collect::<Vec<_>>();
    if paths.len() < 2 {
        for path in native_tool_bootstrap_likely_context_paths(project_root, &[]) {
            if let Some(path) = native_tool_resolve_bounded_patch_context_path(project_root, &path)
            {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() || paths.len() > max_files {
        return Vec::new();
    }
    paths
}

fn native_tool_resolve_bounded_patch_context_path(
    project_root: &Path,
    raw_path: &str,
) -> Option<PathBuf> {
    let candidate = if raw_path.starts_with('/') {
        PathBuf::from(raw_path)
    } else {
        project_root.join(raw_path.trim_start_matches("./"))
    };
    if !candidate.is_file() || !native_tool_bounded_patch_context_file_candidate(&candidate) {
        return None;
    }
    let normalized = candidate.display().to_string().replace('\\', "/");
    if normalized.contains("/.infring/")
        || normalized.contains("/.git/")
        || normalized.contains("/node_modules/")
        || normalized.contains("/target/")
    {
        return None;
    }
    Some(candidate)
}

fn native_tool_bounded_patch_context_file_candidate(path: &Path) -> bool {
    if !native_tool_bootstrap_file_candidate(path) {
        return false;
    }
    fs::metadata(path)
        .map(|metadata| metadata.len() <= 80_000)
        .unwrap_or(false)
}

fn native_tool_bounded_patch_file_context(paths: &[PathBuf]) -> Option<String> {
    let mut sections = Vec::<String>::new();
    for path in paths {
        let Ok(content) = fs::read_to_string(path) else {
            return None;
        };
        sections.push(format!(
            "FILE: {}\n```text\n{}\n```",
            path.display(),
            content
        ));
    }
    Some(sections.join("\n\n"))
}

fn native_tool_bounded_patch_artifact_system_prompt(
    artifact_profile: NativeToolBoundedPatchArtifactProfile,
) -> String {
    match artifact_profile {
        NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact => {
            "Emit only a machine-applicable patch artifact. Prefer SEARCH/REPLACE blocks with the absolute path immediately before each block. SEARCH must exactly match supplied context. No prose, no tool calls, no final answer.".to_string()
        }
        NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact => {
            "Return only SEARCH/REPLACE edit blocks. Put the absolute file path immediately before each <<<<<<< SEARCH. SEARCH must exactly match supplied file context. Use the fewest small blocks that complete the edit. No prose, no tool calls, no final answer.".to_string()
        }
    }
}

fn native_tool_bounded_patch_artifact_prompt(
    metadata: &Value,
    original_prompt: &str,
    file_context: &str,
    artifact_profile: NativeToolBoundedPatchArtifactProfile,
) -> String {
    match artifact_profile {
        NativeToolBoundedPatchArtifactProfile::GeneralPatchArtifact => {
            let rule = native_tool_orchestration_prompt_text(
                metadata,
                "bounded_patch_artifact_lane_rule",
                "Generate the smallest deterministic edit artifact for this bounded local code edit. Use only supplied paths and supplied file text. Do not ask for files.",
            );
            format!(
                "{rule}\n\nTASK:\n{}\n\nFILES:\n{}\n\nOUTPUT ONLY:\n/absolute/path\n<<<<<<< SEARCH\nexact old text\n=======\nnew text\n>>>>>>> REPLACE",
                original_prompt.chars().take(1200).collect::<String>(),
                file_context
            )
        }
        NativeToolBoundedPatchArtifactProfile::SmallScopedEditArtifact => {
            let rule = native_tool_orchestration_prompt_text(
                metadata,
                "small_scoped_edit_artifact_lane_rule",
                "Generate the smallest deterministic SEARCH/REPLACE edit artifact for this small scoped local code edit. The runtime has already supplied every file you may edit, will apply the blocks, then will run validation from receipts when requested. Do not ask for files.",
            );
            format!(
                "{rule}\n\nOriginal task:\n{}\n\nEditable file context:\n{}\n\nRequired output format for each edit:\n/absolute/path/to/file\n<<<<<<< SEARCH\nexact old text\n=======\nreplacement text\n>>>>>>> REPLACE\n\nReturn only SEARCH/REPLACE blocks now.",
                original_prompt.chars().take(1400).collect::<String>(),
                file_context
            )
        }
    }
}

fn native_tool_bounded_patch_artifact_retry_prompt(
    metadata: &Value,
    original_prompt: &str,
    file_context: &str,
    previous_output: &str,
    artifact_profile: NativeToolBoundedPatchArtifactProfile,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "bounded_patch_artifact_strict_retry_rule",
        "The previous patch artifact was empty, malformed, or not machine-applicable. Retry once with compact SEARCH/REPLACE edit blocks only. Do not ask for files, explain, or include tool calls.",
    );
    format!(
        "{rule}\n\nPROFILE: {}\nFAILURE: missing_or_invalid_patch_artifact\nBAD_OUTPUT_PREVIEW:\n{}\n\nTASK:\n{}\n\nFILES:\n{}\n\nOUTPUT ONLY:\n/absolute/path\n<<<<<<< SEARCH\nexact old text\n=======\nnew text\n>>>>>>> REPLACE",
        artifact_profile.as_str(),
        previous_output.chars().take(500).collect::<String>(),
        original_prompt.chars().take(900).collect::<String>(),
        file_context
    )
}

fn native_tool_bounded_patch_artifact_patches(
    output: &str,
    project_root: &Path,
    allowed_paths: &[PathBuf],
) -> Vec<NativeToolBoundedPatch> {
    let allowed = allowed_paths
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .collect::<Vec<_>>();
    let json_patches =
        native_tool_bounded_patch_artifact_json_patches(output, project_root, &allowed);
    if !json_patches.is_empty() {
        return json_patches;
    }
    native_tool_bounded_patch_artifact_search_replace_patches(output, project_root, &allowed)
}

fn native_tool_bounded_patch_artifact_json_patches(
    output: &str,
    project_root: &Path,
    allowed: &[PathBuf],
) -> Vec<NativeToolBoundedPatch> {
    let Some(value) = native_tool_parse_bounded_patch_artifact_json(output) else {
        return Vec::new();
    };
    let Some(patches) = value.get("patches").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::<NativeToolBoundedPatch>::new();
    for patch in patches {
        let Some(path) = patch.get("path").and_then(Value::as_str) else {
            continue;
        };
        let Some(old) = patch.get("old").and_then(Value::as_str) else {
            continue;
        };
        let Some(new) = patch.get("new").and_then(Value::as_str) else {
            continue;
        };
        if old.is_empty() || old == new {
            continue;
        }
        let resolved = if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            project_root.join(path.trim_start_matches("./"))
        };
        let Ok(canonical) = resolved.canonicalize() else {
            continue;
        };
        if !allowed.iter().any(|allowed| allowed == &canonical) {
            continue;
        }
        out.push(NativeToolBoundedPatch {
            path: resolved.display().to_string(),
            old: old.to_string(),
            new: new.to_string(),
        });
    }
    out
}

fn native_tool_bounded_patch_artifact_search_replace_patches(
    output: &str,
    project_root: &Path,
    allowed: &[PathBuf],
) -> Vec<NativeToolBoundedPatch> {
    let mut patches = Vec::<NativeToolBoundedPatch>::new();
    let mut cursor = 0usize;
    while let Some(search_rel) = output[cursor..].find("<<<<<<< SEARCH") {
        let search_start = cursor + search_rel;
        let before = &output[..search_start];
        let Some(path) = native_tool_search_replace_path_heading(before, project_root, allowed)
        else {
            cursor = search_start + "<<<<<<< SEARCH".len();
            continue;
        };
        let old_start = search_start + "<<<<<<< SEARCH".len();
        let Some(separator_rel) = output[old_start..].find("=======") else {
            break;
        };
        let separator_start = old_start + separator_rel;
        let new_start = separator_start + "=======".len();
        let Some(end_rel) = output[new_start..].find(">>>>>>> REPLACE") else {
            break;
        };
        let end_start = new_start + end_rel;
        let old = output[old_start..separator_start]
            .trim_matches('\n')
            .to_string();
        let new = output[new_start..end_start].trim_matches('\n').to_string();
        if !old.is_empty() && old != new {
            patches.push(NativeToolBoundedPatch { path, old, new });
        }
        cursor = end_start + ">>>>>>> REPLACE".len();
    }
    patches
}

fn native_tool_search_replace_path_heading(
    before: &str,
    project_root: &Path,
    allowed: &[PathBuf],
) -> Option<String> {
    for raw_line in before.lines().rev().take(8) {
        let cleaned = raw_line
            .trim()
            .trim_matches('`')
            .trim_matches('*')
            .trim()
            .trim_end_matches(':')
            .trim();
        if cleaned.is_empty()
            || cleaned.starts_with("```")
            || cleaned.starts_with('#')
            || cleaned.contains("SEARCH")
            || cleaned.contains("REPLACE")
        {
            continue;
        }
        let candidate = cleaned
            .split_whitespace()
            .last()
            .unwrap_or(cleaned)
            .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'));
        let resolved = if candidate.starts_with('/') {
            PathBuf::from(candidate)
        } else {
            project_root.join(candidate.trim_start_matches("./"))
        };
        let Ok(canonical) = resolved.canonicalize() else {
            continue;
        };
        if allowed.iter().any(|allowed| allowed == &canonical) {
            return Some(resolved.display().to_string());
        }
    }
    None
}

fn native_tool_parse_bounded_patch_artifact_json(output: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(output.trim()) {
        return Some(value);
    }
    let start = output.find('{')?;
    let end = output.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<Value>(&output[start..=end]).ok()
}

fn native_tool_partial_progress_response(
    provider_id: &str,
    model: Option<&str>,
    reason: &str,
    provider_call_count: u64,
    receipts: &[NativeToolReceipt],
) -> ProviderResponse {
    let successful_mutations = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .filter(|receipt| receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        .count();
    let output = format!(
        "Native coding run stopped with partial progress: {reason}. Successful native mutation receipts: {successful_mutations}. Returning a structured partial-progress terminal result so the parent workflow can report the timeout instead of hanging."
    );
    ProviderResponse {
        provider: provider_id.to_string(),
        model: model.unwrap_or("unknown").to_string(),
        usage_tokens: output.split_whitespace().count() as u64,
        output,
        raw: json!({
            "ok": false,
            "provider": provider_id,
            "terminal_status": "partial_timeout",
            "reason": reason,
            "provider_call_count": provider_call_count,
            "native_tool_call_count": receipts.len(),
            "successful_mutation_receipt_count": successful_mutations,
            "native_tool_receipt_summary": receipts.iter().map(|receipt| {
                json!({
                    "call_id": receipt.call_id.clone(),
                    "tool_name": receipt.tool_name.clone(),
                    "status": receipt.status.clone(),
                    "error": receipt.error.clone(),
                    "path": receipt.result.get("path").cloned().unwrap_or(Value::Null),
                })
            }).collect::<Vec<_>>(),
        }),
    }
}

fn native_tool_recovery_or_partial_progress(
    provider: &Arc<dyn crate::provider::ProviderClient>,
    dispatcher: &NativeToolDispatcher,
    tools: &[String],
    model: Option<String>,
    metadata: &Value,
    original_prompt: &str,
    system: &str,
    reason: &str,
    mut provider_call_count: u64,
    mut receipts: Vec<NativeToolReceipt>,
) -> Result<(ProviderResponse, Vec<NativeToolReceipt>, u64, String), ProviderError> {
    let changed_paths = native_tool_changed_paths(&receipts);
    if changed_paths.is_empty() {
        return Ok((
            native_tool_partial_progress_response(
                provider.provider_id(),
                model.as_deref(),
                reason,
                provider_call_count,
                &receipts,
            ),
            receipts,
            provider_call_count,
            "partial_timeout".to_string(),
        ));
    }

    let max_turns = native_tool_recovery_max_turns(metadata);
    let mut recovery_metadata = metadata.clone();
    if let Some(object) = recovery_metadata.as_object_mut() {
        object.insert(
            "provider_timeout_seconds".to_string(),
            json!(native_tool_recovery_provider_timeout_seconds(metadata)),
        );
        object.insert("native_recovery_pass".to_string(), json!(true));
    }
    let mut prompt =
        native_tool_recovery_prompt(metadata, original_prompt, reason, &changed_paths, &receipts);

    for turn_idx in 0..max_turns {
        provider_call_count += 1;
        let request = ProviderRequest {
            prompt: prompt.clone(),
            system: Some(system.to_string()),
            tools: tools.to_vec(),
            model: model.clone(),
            metadata: recovery_metadata.clone(),
        };
        let response = match provider.complete(&request) {
            Ok(response) => response,
            Err(error) if error.code == ProviderErrorCode::Timeout => {
                return Ok((
                    native_tool_partial_progress_response(
                        provider.provider_id(),
                        model.as_deref(),
                        error.message.as_str(),
                        provider_call_count,
                        &receipts,
                    ),
                    receipts,
                    provider_call_count,
                    "partial_timeout".to_string(),
                ));
            }
            Err(error) => return Err(error),
        };
        let calls = parse_native_tool_calls(&response.output);
        if calls.is_empty() {
            if native_tool_needs_public_report_finalization(metadata, &response.output) {
                provider_call_count += 1;
                let request = ProviderRequest {
                    prompt: native_tool_public_reasoning_finalization_prompt(
                        metadata,
                        original_prompt,
                        &receipts,
                        &response.output,
                    ),
                    system: Some(system.to_string()),
                    tools: Vec::new(),
                    model: model.clone(),
                    metadata: native_tool_public_reasoning_metadata(metadata),
                };
                let finalized = match provider.complete(&request) {
                    Ok(finalized) => finalized,
                    Err(error) if error.code == ProviderErrorCode::Timeout => {
                        return Ok((
                            native_tool_partial_progress_response(
                                provider.provider_id(),
                                model.as_deref(),
                                error.message.as_str(),
                                provider_call_count,
                                &receipts,
                            ),
                            receipts,
                            provider_call_count,
                            "partial_timeout".to_string(),
                        ));
                    }
                    Err(error) => return Err(error),
                };
                let mut finalized = finalized;
                finalized.raw = json!({
                    "provider_raw": finalized.raw,
                    "native_tool_recovery": {
                        "enabled": true,
                        "reason": reason,
                        "provider_call_count": provider_call_count,
                        "recovery_turns_used": turn_idx + 1,
                        "changed_paths": changed_paths,
                        "tool_call_count": receipts.len(),
                        "terminal_status": "ok",
                        "public_reasoning_finalization": true
                    }
                });
                return Ok((finalized, receipts, provider_call_count, "ok".to_string()));
            }
            let mut response = response;
            response.raw = json!({
                "provider_raw": response.raw,
                "native_tool_recovery": {
                    "enabled": true,
                    "reason": reason,
                    "provider_call_count": provider_call_count,
                    "recovery_turns_used": turn_idx + 1,
                    "changed_paths": changed_paths,
                    "tool_call_count": receipts.len(),
                    "terminal_status": "ok"
                }
            });
            return Ok((response, receipts, provider_call_count, "ok".to_string()));
        }
        let mut turn_receipts = Vec::new();
        for call in calls
            .into_iter()
            .take(native_tool_max_calls_per_turn(metadata))
        {
            let receipt =
                dispatcher.dispatch(native_tool_call_with_prompt_defaults(call, original_prompt));
            turn_receipts.push(receipt.clone());
            receipts.push(receipt);
        }
        let observation = native_tool_observation_prompt(&turn_receipts);
        let recovery_turn_rule = native_tool_orchestration_prompt_text(
            metadata,
            "partial_progress_recovery_turn_rule",
            "Continue the bounded recovery pass. If the changed files are repaired or no safe repair remains, provide the final user response.",
        );
        prompt = format!(
            "{}\n\nRecovery tool request turn {}:\n{}\n\nNative tool observations:\n{}\n\n{}",
            native_tool_recovery_prompt(
                metadata,
                original_prompt,
                reason,
                &changed_paths,
                &receipts
            ),
            turn_idx + 1,
            response.output,
            observation,
            recovery_turn_rule
        );
    }

    if let Some(validation_receipt) =
        native_tool_auto_validation_receipt(dispatcher, original_prompt, &receipts)
    {
        receipts.push(validation_receipt);
    }
    let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
        dispatcher,
        metadata,
        original_prompt,
        &receipts,
    );
    if !auto_handoff_receipts.is_empty() {
        receipts.extend(auto_handoff_receipts);
    }
    let mut response = native_tool_partial_progress_response(
        provider.provider_id(),
        model.as_deref(),
        "native_tool_recovery_pass_exhausted",
        provider_call_count,
        &receipts,
    );
    let completed_after_recovery = native_tool_has_successful_mutation(&receipts)
        && native_tool_has_successful_validation_after_latest_mutation(&receipts)
        && native_tool_prompt_evidence_gaps(original_prompt, &receipts).is_empty()
        && !native_tool_checkpointed_project_has_live_stage(metadata, original_prompt, &receipts);
    if completed_after_recovery {
        response = native_tool_synthetic_completion_evidence_response(
            &response,
            metadata,
            original_prompt,
            &receipts,
            "runtime_synthesized_timeout_recovery_closure",
        );
    }
    let terminal_status = if completed_after_recovery {
        "ok".to_string()
    } else {
        "partial_timeout".to_string()
    };
    Ok((response, receipts, provider_call_count, terminal_status))
}

fn native_tool_completion_evidence_repair_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("repair_uncovered_requirements_before_final"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_completion_evidence_repair_max_turns(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("completion_evidence_repair_max_turns"))
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 5)
}

fn native_tool_runtime_repair_reasons(
    metadata: &Value,
    original_prompt: &str,
    output: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut reasons =
        native_tool_artifact_repair_reasons(metadata, original_prompt, output, receipts);
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if native_tool_requires_successful_mutation(metadata)
        && native_tool_prompt_requires_product_mutation(&original_prompt.to_ascii_lowercase())
        && !native_tool_has_successful_mutation(receipts)
        && !reasons
            .iter()
            .any(|reason| reason == "missing_product_mutation_receipt")
    {
        reasons.push("missing_product_mutation_receipt".to_string());
    }
    for path in native_tool_prompt_required_handoff_artifact_paths(original_prompt) {
        if !native_tool_has_successful_mutation_for_suffix(receipts, &path)
            && !reasons
                .iter()
                .any(|reason| reason == &format!("missing_changed_path:{path}"))
        {
            reasons.push(format!("missing_changed_path:{path}"));
        }
    }
    for path in native_tool_unresolved_preserved_api_additive_patch_paths(receipts) {
        if !reasons
            .iter()
            .any(|reason| reason == &format!("unresolved_owner_source_path:{path}"))
        {
            reasons.push(format!("unresolved_owner_source_path:{path}"));
        }
    }
    if native_tool_prompt_requires_validation_command(&prompt_lower)
        && !native_tool_has_successful_validation_after_latest_mutation(receipts)
    {
        let failed_validation_refs = native_tool_failed_validation_command_refs(receipts);
        if failed_validation_refs.is_empty() {
            if !reasons
                .iter()
                .any(|reason| reason == "missing_validation_command_receipt")
            {
                reasons.push("missing_validation_command_receipt".to_string());
            }
        } else {
            reasons.extend(failed_validation_refs);
        }
        reasons.extend(native_tool_failed_validation_import_repair_reasons(
            original_prompt,
            receipts,
        ));
    }
    if native_tool_prompt_requires_memory_write(&prompt_lower)
        && !native_tool_has_successful_memory_write_command(
            receipts,
            native_tool_prompt_expected_memory_row_id(original_prompt).as_deref(),
            original_prompt,
        )
        && !reasons
            .iter()
            .any(|reason| reason.starts_with("missing_memory_write_receipt"))
    {
        reasons.push("missing_memory_write_receipt".to_string());
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn native_tool_failed_validation_import_repair_reasons(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let details = native_tool_failed_validation_receipt_details(receipts);
    if details == "<none>" {
        return Vec::new();
    }
    let mut reasons = Vec::new();
    for (symbol, module) in native_tool_python_cannot_import_name_errors(&details) {
        if let Some(path) = native_tool_prompt_python_module_path(original_prompt, &module) {
            reasons.push(format!("missing_imported_module:{module}:{path}"));
        }
        reasons.push(format!(
            "missing_public_interface_verification:{module}:{symbol}"
        ));
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn native_tool_python_cannot_import_name_errors(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("cannot import name '") {
        let after_start = &rest[start + "cannot import name '".len()..];
        let Some(symbol_end) = after_start.find('\'') else {
            break;
        };
        let symbol = after_start[..symbol_end].trim();
        let after_symbol = &after_start[symbol_end + 1..];
        let Some(from_start) = after_symbol.find(" from '") else {
            rest = after_symbol;
            continue;
        };
        let after_from = &after_symbol[from_start + " from '".len()..];
        let Some(module_end) = after_from.find('\'') else {
            break;
        };
        let module = after_from[..module_end].trim();
        if native_tool_python_identifier(symbol)
            && module
                .split('.')
                .all(|part| native_tool_python_identifier(part))
            && !pairs
                .iter()
                .any(|(existing_symbol, existing_module)| {
                    existing_symbol == symbol && existing_module == module
                })
        {
            pairs.push((symbol.to_string(), module.to_string()));
        }
        rest = &after_from[module_end + 1..];
    }
    pairs
}

fn native_tool_python_extend_seed_symbols_from_observed_imports(
    receipts: &[NativeToolReceipt],
    by_module: &mut BTreeMap<String, Vec<String>>,
) {
    if by_module.is_empty() {
        return;
    }
    let seeded_modules = by_module.keys().cloned().collect::<Vec<_>>();
    let mut contents = Vec::new();
    for receipt in receipts {
        native_tool_collect_text_contents(&receipt.result, &mut contents);
    }
    for content in contents {
        for line in content.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("from ") else {
                continue;
            };
            let Some((module, imports)) = rest.split_once(" import ") else {
                continue;
            };
            let module = module.trim();
            if !seeded_modules.iter().any(|candidate| candidate == module) {
                continue;
            }
            for symbol in native_tool_python_imported_symbols_from_clause(imports) {
                if native_tool_symbol_looks_private_or_generated(&symbol) {
                    continue;
                }
                let entry = by_module.entry(module.to_string()).or_default();
                if !entry.iter().any(|existing| existing == &symbol) {
                    entry.push(symbol);
                }
            }
        }
    }
}

fn native_tool_collect_text_contents(value: &Value, contents: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                native_tool_collect_text_contents(item, contents);
            }
        }
        Value::Object(object) => {
            if let Some(content) = object.get("content").and_then(Value::as_str) {
                contents.push(content.to_string());
            }
            if let Some(stdout) = object.get("stdout").and_then(Value::as_str) {
                contents.push(stdout.to_string());
            }
            if let Some(stderr) = object.get("stderr").and_then(Value::as_str) {
                contents.push(stderr.to_string());
            }
            for value in object.values() {
                native_tool_collect_text_contents(value, contents);
            }
        }
        Value::String(text) => contents.push(text.to_string()),
        _ => {}
    }
}

fn native_tool_python_imported_symbols_from_clause(imports: &str) -> Vec<String> {
    let imports = imports
        .split('#')
        .next()
        .unwrap_or(imports)
        .replace(['(', ')'], "");
    imports
        .split(',')
        .filter_map(|part| {
            let symbol = part
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim();
            if native_tool_python_identifier(symbol) {
                Some(symbol.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn native_tool_python_import_surface_seed_receipts(
    dispatcher: &NativeToolDispatcher,
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<NativeToolReceipt> {
    if !native_tool_python_import_surface_seed_enabled(metadata)
        || native_tool_has_successful_non_import_surface_mutation(receipts)
    {
        return Vec::new();
    }
    let details = native_tool_failed_validation_receipt_details(receipts);
    if details == "<none>" {
        return Vec::new();
    }
    native_tool_python_import_surface_seed_receipts_from_details(
        dispatcher,
        original_prompt,
        receipts,
        &details,
        "runtime_python_import_surface_seed",
    )
}

fn native_tool_has_successful_non_import_surface_mutation(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
            && !receipt.call_id.contains("runtime_python_import_surface_seed")
    })
}

fn native_tool_python_import_surface_seed_receipts_from_details(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    details: &str,
    call_id_prefix: &str,
) -> Vec<NativeToolReceipt> {
    let mut by_module = BTreeMap::<String, Vec<String>>::new();
    for (symbol, module) in native_tool_python_cannot_import_name_errors(&details) {
        if native_tool_symbol_looks_private_or_generated(&symbol) {
            continue;
        }
        let entry = by_module.entry(module).or_default();
        if !entry.iter().any(|existing| existing == &symbol) {
            entry.push(symbol);
        }
    }
    native_tool_python_extend_seed_symbols_from_observed_imports(receipts, &mut by_module);
    if by_module.is_empty() {
        return Vec::new();
    }
    let Some(project_root) = native_tool_prompt_project_root(original_prompt) else {
        return Vec::new();
    };
    let root = PathBuf::from(project_root);
    let mut seed_receipts = Vec::new();
    for (module, mut symbols) in by_module {
        symbols.sort();
        symbols.dedup();
        let Some(export_path_text) = native_tool_prompt_python_module_path(original_prompt, &module)
        else {
            continue;
        };
        let export_path = PathBuf::from(&export_path_text);
        let owner_path =
            native_tool_python_import_seed_owner_path(&root, &export_path, &module, receipts);
        let Ok(source_original) = fs::read_to_string(&owner_path) else {
            continue;
        };
        let source_next =
            native_tool_python_import_seed_source_content(&source_original, &symbols);
        if source_next != source_original {
            seed_receipts.push(dispatcher.dispatch(NativeToolCall {
                id: format!(
                    "{call_id_prefix}_source_{}",
                    native_tool_receipt_id_slug(&module)
                ),
                name: "file_write".to_string(),
                args: json!({
                    "path": owner_path.display().to_string(),
                    "content": source_next,
                    "overwrite": true,
                }),
            }));
        }
        if export_path != owner_path {
            if let Ok(export_original) = fs::read_to_string(&export_path) {
                let export_next = native_tool_python_import_seed_export_content(
                    &export_original,
                    &owner_path,
                    &export_path,
                    &symbols,
                );
                if export_next != export_original {
                    seed_receipts.push(dispatcher.dispatch(NativeToolCall {
                        id: format!(
                            "{call_id_prefix}_export_{}",
                            native_tool_receipt_id_slug(&module)
                        ),
                        name: "file_write".to_string(),
                        args: json!({
                            "path": export_path.display().to_string(),
                            "content": export_next,
                            "overwrite": true,
                        }),
                    }));
                }
            }
        }
    }
    seed_receipts
        .into_iter()
        .filter(|receipt| receipt.status == "ok")
        .collect()
}

fn native_tool_python_import_seed_owner_path(
    root: &Path,
    export_path: &Path,
    module: &str,
    receipts: &[NativeToolReceipt],
) -> PathBuf {
    if export_path
        .file_name()
        .and_then(|name| name.to_str())
        != Some("__init__.py")
    {
        return export_path.to_path_buf();
    }
    let package_dir = export_path.parent().unwrap_or(root);
    let observed = native_tool_python_observed_module_source_paths(package_dir, receipts);
    if let Some(path) = observed.into_iter().next() {
        return path;
    }
    let mut candidates = fs::read_dir(package_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("py")
                && path.file_name().and_then(|value| value.to_str()) != Some("__init__.py")
        })
        .collect::<Vec<_>>();
    candidates.sort();
    if let Some(path) = candidates.into_iter().next() {
        return path;
    }
    let fallback_name = module
        .split('.')
        .last()
        .filter(|part| native_tool_python_identifier(part))
        .unwrap_or("api");
    package_dir.join(format!("{fallback_name}.py"))
}

fn native_tool_python_observed_module_source_paths(
    package_dir: &Path,
    receipts: &[NativeToolReceipt],
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for receipt in receipts {
        native_tool_collect_observed_python_source_paths(package_dir, &receipt.result, &mut paths);
    }
    paths.sort();
    paths.dedup();
    paths
}

fn native_tool_collect_observed_python_source_paths(
    package_dir: &Path,
    value: &Value,
    paths: &mut Vec<PathBuf>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                native_tool_collect_observed_python_source_paths(package_dir, item, paths);
            }
        }
        Value::Object(object) => {
            if let Some(path) = object
                .get("path")
                .or_else(|| object.get("relative_path"))
                .and_then(Value::as_str)
            {
                let path = PathBuf::from(path);
                if path.is_file()
                    && path.parent() == Some(package_dir)
                    && path.extension().and_then(|value| value.to_str()) == Some("py")
                    && path.file_name().and_then(|value| value.to_str()) != Some("__init__.py")
                {
                    native_tool_push_unique_path(paths, path);
                }
            }
            for value in object.values() {
                native_tool_collect_observed_python_source_paths(package_dir, value, paths);
            }
        }
        _ => {}
    }
}

fn native_tool_python_import_seed_source_content(original: &str, symbols: &[String]) -> String {
    let mut additions = Vec::new();
    for symbol in symbols {
        if native_tool_python_content_defines_symbol(original, symbol) {
            continue;
        }
        if native_tool_python_symbol_looks_type(symbol) {
            additions.push(format!(
                "class {symbol}:\n    \"\"\"Runtime import-surface seed; behavior is repaired by the coding workflow.\"\"\"\n\n    def __init__(self, *args, **kwargs):\n        self.args = args\n        self.kwargs = kwargs\n"
            ));
        } else {
            additions.push(format!(
                "def {symbol}(*args, **kwargs):\n    \"\"\"Runtime import-surface seed; behavior is repaired by the coding workflow.\"\"\"\n    raise NotImplementedError(\"{symbol} behavior is not implemented yet\")\n"
            ));
        }
    }
    if additions.is_empty() {
        return original.to_string();
    }
    let mut next = original.trim_end().to_string();
    next.push_str("\n\n\n# Runtime import-surface seed: generic missing public API scaffold.\n");
    next.push_str(&additions.join("\n\n"));
    next.push('\n');
    next
}

fn native_tool_python_import_seed_export_content(
    original: &str,
    owner_path: &Path,
    export_path: &Path,
    symbols: &[String],
) -> String {
    let import_symbols = symbols
        .iter()
        .filter(|symbol| !native_tool_python_content_exports_symbol(original, symbol))
        .cloned()
        .collect::<Vec<_>>();
    if import_symbols.is_empty() {
        return original.to_string();
    }
    let owner_module = owner_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| native_tool_python_identifier(value))
        .unwrap_or("api");
    let relative_prefix = if owner_path.parent() == export_path.parent() {
        ".".to_string()
    } else {
        ".".to_string()
    };
    let mut next = original.trim_end().to_string();
    next.push_str("\n\n# Runtime import-surface seed: expose missing public API scaffold.\n");
    next.push_str(&format!(
        "from {relative_prefix}{owner_module} import {}\n",
        import_symbols.join(", ")
    ));
    next.push_str("try:\n    __all__\nexcept NameError:\n    __all__ = []\n");
    next.push_str(&format!(
        "__all__ = sorted(set(__all__) | {{{}}})\n",
        import_symbols
            .iter()
            .map(|symbol| format!("{symbol:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    next
}

fn native_tool_python_content_defines_symbol(content: &str, symbol: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(&format!("class {symbol}"))
            || trimmed.starts_with(&format!("def {symbol}"))
            || trimmed.starts_with(&format!("{symbol} ="))
    })
}

fn native_tool_python_content_exports_symbol(content: &str, symbol: &str) -> bool {
    content.contains(&format!("import {symbol}"))
        || content.contains(&format!("import {symbol},"))
        || content.contains(&format!(", {symbol}"))
        || content.contains(&format!("{symbol:?}"))
        || content.contains(&format!("'{symbol}'"))
}

fn native_tool_python_symbol_looks_type(symbol: &str) -> bool {
    symbol
        .chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn native_tool_symbol_looks_private_or_generated(symbol: &str) -> bool {
    symbol.starts_with('_') || symbol.contains("__")
}

fn native_tool_receipt_id_slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn native_tool_push_unique_path(values: &mut Vec<PathBuf>, value: PathBuf) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn native_tool_prompt_python_module_path(original_prompt: &str, module: &str) -> Option<String> {
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let root = PathBuf::from(project_root);
    let rel = module.replace('.', "/");
    let candidates = [
        root.join("src").join(format!("{rel}.py")),
        root.join(format!("{rel}.py")),
        root.join("src").join(&rel).join("__init__.py"),
        root.join(&rel).join("__init__.py"),
    ];
    candidates
        .iter()
        .find(|path| path.exists())
        .or_else(|| candidates.first())
        .map(|path| path.display().to_string())
}

fn native_tool_repair_reasons_include_product_mutation(repair_reasons: &[String]) -> bool {
    repair_reasons
        .iter()
        .any(|reason| reason == "missing_product_mutation_receipt")
}

fn native_tool_repair_reasons_include_product_slice(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason.starts_with("incomplete_product_slice")
            || reason.starts_with("missing_product_source_evidence:")
            || reason.starts_with("missing_public_interface_verification:")
            || reason.starts_with("missing_imported_module:")
            || reason.starts_with("unresolved_owner_source_path:")
    })
}

fn native_tool_repair_reasons_require_product_work(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason == "missing_product_mutation_receipt"
            || reason == "missing_test_change_receipt"
            || reason.starts_with("incomplete_product_slice")
            || reason.starts_with("missing_product_source_evidence:")
            || reason.starts_with("missing_public_interface_verification:")
            || reason.starts_with("missing_imported_module:")
            || reason.starts_with("missing_changed_path:")
            || reason.starts_with("unresolved_owner_source_path:")
    })
}

fn native_tool_has_successful_context_receipt(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && matches!(
                receipt.tool_name.as_str(),
                "file_read" | "file_read_many" | "file_list" | "file_stat"
            )
    })
}

fn native_tool_has_successful_read_context_receipt(receipts: &[NativeToolReceipt]) -> bool {
    native_tool_successful_read_context_count(receipts) > 0
}

fn native_tool_successful_read_context_count(receipts: &[NativeToolReceipt]) -> usize {
    receipts
        .iter()
        .filter(|receipt| {
            receipt.status == "ok"
                && matches!(receipt.tool_name.as_str(), "file_read" | "file_read_many")
        })
        .count()
}

fn native_tool_successful_read_context_count_after_first_mutation(
    receipts: &[NativeToolReceipt],
) -> usize {
    let mut saw_mutation = false;
    let mut count = 0;
    for receipt in receipts {
        if receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
        {
            saw_mutation = true;
            continue;
        }
        if saw_mutation
            && receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_read" | "file_read_many")
        {
            count += 1;
        }
    }
    count
}

fn native_tool_successful_read_context_count_after_first_failed_validation(
    receipts: &[NativeToolReceipt],
) -> usize {
    let mut saw_failed_validation = false;
    let mut count = 0;
    for receipt in receipts {
        if receipt.status == "ok"
            && receipt.tool_name == "command_run"
            && !receipt
                .result
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            saw_failed_validation = true;
            continue;
        }
        if saw_failed_validation
            && receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_read" | "file_read_many")
        {
            count += 1;
        }
    }
    count
}

fn native_tool_pre_mutation_read_budget(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("staged_pre_mutation_read_budget"))
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, 12) as usize
}

fn native_tool_has_successful_test_mutation(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !(receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        {
            return false;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            return false;
        };
        let lower = path.to_ascii_lowercase();
        lower.contains("/test")
            || lower.contains("\\test")
            || lower.contains("tests/")
            || lower.contains("test_")
    })
}

fn native_tool_has_successful_product_mutation(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !(receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        {
            return false;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            return false;
        };
        native_tool_path_is_product_mutation_path(path)
    })
}

fn native_tool_call_is_product_mutation(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_mutation(call) {
        return false;
    }
    native_tool_call_path_arg(call)
        .map(native_tool_path_is_product_mutation_path)
        .unwrap_or(false)
}

fn native_tool_edit_owner_hint(receipts: &[NativeToolReceipt]) -> String {
    let paths = native_tool_observed_product_source_paths(receipts);
    if paths.is_empty() {
        return String::new();
    }
    format!(
        "\n\nRuntime edit-owner hint: prefer mutating these observed product source files before creating new product modules: {}.",
        paths.join(", ")
    )
}

fn native_tool_edit_owner_blocked_receipt(
    call: NativeToolCall,
    receipts: &[NativeToolReceipt],
) -> NativeToolReceipt {
    let observed_owner_paths = native_tool_observed_product_source_paths(receipts);
    let attempted_path = native_tool_call_path_arg(&call)
        .map(str::to_string)
        .or_else(|| native_tool_unified_diff_target_path(
            call.args
                .get("patch")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ));
    let seed_paths = native_tool_python_import_surface_seed_paths(receipts);
    let suggested_owner_path = observed_owner_paths.first().cloned();
    NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.clone(),
        status: "error".to_string(),
        result: json!({
            "reason": "product_mutation_must_target_observed_owner_file",
            "attempted_path": attempted_path,
            "observed_owner_paths": observed_owner_paths,
            "seed_paths": seed_paths,
            "suggested_next_tool": "file_write_or_file_patch",
            "suggested_path": suggested_owner_path,
            "next_action": "mutate one of the observed_owner_paths exactly; do not patch seed/import-surface files or invent a new owner path",
        }),
        error: Some("product_mutation_must_target_observed_owner_file".to_string()),
        duration_ms: 0,
    }
}

fn native_tool_call_targets_observed_product_path(
    call: &NativeToolCall,
    receipts: &[NativeToolReceipt],
) -> bool {
    let Some(target) = native_tool_call_path_arg(call) else {
        return true;
    };
    let paths = native_tool_observed_product_source_paths(receipts);
    if paths.is_empty() {
        return true;
    }
    paths.iter()
        .any(|path| native_tool_paths_same_or_suffix(path, target))
}

fn native_tool_observed_product_source_paths(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = Vec::<String>::new();
    for receipt in receipts {
        if receipt.status != "ok" {
            continue;
        }
        match receipt.tool_name.as_str() {
            "file_read" => {
                if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
                    native_tool_push_observed_product_source_path(&mut paths, path);
                }
            }
            "file_read_many" => {
                if let Some(files) = receipt.result.get("files").and_then(Value::as_array) {
                    for file in files {
                        if let Some(path) = file.get("path").and_then(Value::as_str) {
                            native_tool_push_observed_product_source_path(&mut paths, path);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    paths
}

fn native_tool_push_observed_product_source_path(paths: &mut Vec<String>, path: &str) {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("/.infring/")
        || native_tool_bootstrap_path_looks_like_test(&lower)
        || !native_tool_path_is_product_mutation_path(&normalized)
    {
        return;
    }
    if !paths.iter().any(|existing| existing == &normalized) {
        paths.push(normalized);
    }
}

fn native_tool_paths_same_or_suffix(left: &str, right: &str) -> bool {
    let left = left.replace('\\', "/");
    let right = right.replace('\\', "/");
    left == right
        || left.ends_with(right.trim_start_matches("./"))
        || right.ends_with(left.trim_start_matches("./"))
}

fn native_tool_prompt_explicit_new_file_allowed(original_prompt: &str) -> bool {
    let prompt = original_prompt.to_ascii_lowercase();
    [
        "create a new file",
        "add a new file",
        "new source file",
        "new module",
        "new package",
        "new component",
        "from scratch",
        "initialize a new project",
        "create a new project",
    ]
    .iter()
    .any(|needle| prompt.contains(needle))
}

fn native_tool_path_is_product_mutation_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    !(lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("test_")
        || lower.ends_with("_test.py")
        || lower.ends_with(".test.js")
        || lower.ends_with(".spec.js")
        || native_tool_path_is_handoff_artifact(path))
}

fn native_tool_product_source_stage_satisfied(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    if !native_tool_has_successful_product_mutation(receipts) {
        return false;
    }
    if native_tool_prompt_requires_product_mutation(&original_prompt.to_ascii_lowercase())
        && !native_tool_has_successful_implementation_source_mutation(receipts)
    {
        return false;
    }
    if native_tool_preserved_api_source_paths_from_workspace(original_prompt)
        .into_iter()
        .any(|path| !native_tool_changed_paths_include(receipts, &path))
    {
        return false;
    }
    if !native_tool_product_slice_gaps(original_prompt, receipts).is_empty() {
        return false;
    }
    true
}

fn native_tool_has_successful_implementation_source_mutation(
    receipts: &[NativeToolReceipt],
) -> bool {
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !(receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        {
            return false;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            return false;
        };
        native_tool_path_is_implementation_source_path(path)
    })
}

fn native_tool_path_is_implementation_source_path(path: &str) -> bool {
    if !native_tool_path_is_product_mutation_path(path) {
        return false;
    }
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    if lower.ends_with("/__init__.py")
        || lower.ends_with("/index.ts")
        || lower.ends_with("/index.tsx")
        || lower.ends_with("/index.js")
        || lower.ends_with("/index.jsx")
        || lower.ends_with("/mod.rs")
    {
        return false;
    }
    lower.ends_with(".py")
        || lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
}

fn native_tool_live_stage_repair_reasons(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut reasons = native_tool_prompt_evidence_gaps(original_prompt, receipts)
        .into_iter()
        .filter(|reason| {
            reason.starts_with("missing_changed_path:")
                || reason.starts_with("missing_product_source_evidence:")
                || reason.starts_with("incomplete_product_slice")
                || reason.starts_with("missing_public_interface_verification:")
                || reason.starts_with("missing_validation_command_receipt")
                || reason.starts_with("failed_validation_command_receipt:")
                || reason.starts_with("missing_memory_write_receipt")
        })
        .collect::<Vec<_>>();
    reasons.extend(native_tool_semantic_closeout_gaps(original_prompt, receipts));
    for path in native_tool_prompt_required_handoff_artifact_paths(original_prompt) {
        if !native_tool_has_successful_mutation_for_suffix(receipts, &path) {
            reasons.push(format!("missing_changed_path:{path}"));
        }
    }
    if native_tool_prompt_requires_memory_write(&original_prompt.to_ascii_lowercase())
        && !native_tool_has_successful_memory_write_command(
            receipts,
            native_tool_prompt_expected_memory_row_id(original_prompt).as_deref(),
            original_prompt,
        )
    {
        reasons.push("missing_memory_write_receipt".to_string());
    }
    reasons.extend(native_tool_failed_validation_import_repair_reasons(
        original_prompt,
        receipts,
    ));
    for path in native_tool_preserved_api_source_paths_from_workspace(original_prompt) {
        if !native_tool_changed_paths_include(receipts, &path) {
            reasons.push(format!("missing_changed_path:{path}"));
        }
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn native_tool_bounded_direct_edit_required_evidence_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let prompt_lower = original_prompt.to_ascii_lowercase();
    let mut gaps = native_tool_prompt_evidence_gaps(original_prompt, receipts)
        .into_iter()
        .filter(|reason| {
            reason == "missing_test_change_receipt"
                || reason.starts_with("missing_changed_path:")
                || reason.starts_with("missing_product_source_evidence:")
                || reason.starts_with("incomplete_product_slice")
                || reason.starts_with("missing_public_interface_verification:")
        })
        .collect::<Vec<_>>();
    if native_tool_prompt_requires_test_changes(&prompt_lower)
        && !native_tool_has_successful_test_mutation(receipts)
        && !gaps
            .iter()
            .any(|reason| reason == "missing_test_change_receipt")
    {
        gaps.push("missing_test_change_receipt".to_string());
    }
    if native_tool_prompt_requires_validation_command(&prompt_lower)
        && !native_tool_has_successful_validation_after_latest_mutation(receipts)
    {
        let failed_validation_refs = native_tool_failed_validation_command_refs(receipts);
        if failed_validation_refs.is_empty() {
            gaps.push("missing_validation_command_receipt".to_string());
        } else {
            gaps.extend(failed_validation_refs);
        }
    }
    gaps.sort();
    gaps.dedup();
    gaps
}

fn native_tool_preserved_api_source_paths_from_workspace(original_prompt: &str) -> Vec<String> {
    let names = native_tool_prompt_preserved_api_names(original_prompt);
    let Some(project_root) = native_tool_prompt_project_root(original_prompt) else {
        return Vec::new();
    };
    if names.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    let root = std::path::PathBuf::from(project_root);
    for dir in [root.join("src"), root.clone()] {
        native_tool_collect_preserved_api_source_paths(&dir, &names, &mut paths, 0);
    }
    paths.sort();
    paths.dedup();
    paths
}

fn native_tool_collect_preserved_api_source_paths(
    dir: &std::path::Path,
    names: &[String],
    paths: &mut Vec<String>,
    depth: usize,
) {
    if depth > 5 || paths.len() >= 16 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let display = path.display().to_string();
        let lower = display.replace('\\', "/").to_ascii_lowercase();
        if lower.contains("/.git/")
            || lower.contains("/target/")
            || lower.contains("/node_modules/")
            || lower.contains("/tests/")
            || lower.contains("/test/")
            || lower.contains("__pycache__")
        {
            continue;
        }
        if path.is_dir() {
            native_tool_collect_preserved_api_source_paths(&path, names, paths, depth + 1);
            continue;
        }
        if !native_tool_path_is_product_mutation_path(&display) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if names
            .iter()
            .any(|name| native_tool_source_text_defines_api(&content, name))
            && !paths.iter().any(|existing| existing == &display)
        {
            paths.push(display);
        }
    }
}

fn native_tool_source_text_defines_api(content: &str, name: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    [
        format!("def {name}("),
        format!("async def {name}("),
        format!("class {name}("),
        format!("class {name}:"),
        format!("function {name}("),
        format!("export function {name}("),
        format!("pub fn {name}("),
        format!("fn {name}("),
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn native_tool_has_unmutated_required_repair_path(
    repair_reasons: &[String],
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_required_repair_paths(repair_reasons)
        .into_iter()
        .any(|path| !native_tool_has_successful_mutation_for_suffix(receipts, path))
}

fn native_tool_has_successful_mutation_for_suffix(
    receipts: &[NativeToolReceipt],
    expected: &str,
) -> bool {
    let expected = expected
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase();
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !(receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        {
            return false;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            return false;
        };
        let normalized = path.replace('\\', "/").to_ascii_lowercase();
        normalized.ends_with(&expected) || normalized.contains(&format!("/{expected}"))
    })
}

fn native_tool_call_is_context_only(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_list"
            | "list_files"
            | "workspace.list"
            | "workspace_list"
            | "file_stat"
            | "stat_file"
            | "file_exists"
            | "workspace.stat"
            | "workspace_stat"
            | "file_read"
            | "read_file"
            | "workspace.read"
            | "workspace_read"
            | "file_read_many"
            | "read_many_files"
            | "workspace.read_many"
            | "workspace_read_many"
            | "command_resolve"
            | "resolve_command"
            | "command.resolve"
            | "executable_resolve"
            | "executable.resolve"
    )
}

fn native_tool_prioritize_pre_mutation_validation_calls(calls: &mut Vec<NativeToolCall>) {
    if calls.len() < 2 || !calls.iter().any(native_tool_call_is_command_run) {
        return;
    }
    let mut validation_calls = Vec::new();
    let mut other_calls = Vec::new();
    for call in calls.drain(..) {
        if native_tool_call_is_command_run(&call) {
            validation_calls.push(call);
        } else {
            other_calls.push(call);
        }
    }
    validation_calls.extend(other_calls);
    *calls = validation_calls;
}

fn native_tool_product_repair_context_blocked_receipt(
    call: NativeToolCall,
    repair_reasons: &[String],
    receipts: &[NativeToolReceipt],
) -> NativeToolReceipt {
    let missing_changed_paths = native_tool_missing_changed_paths(repair_reasons);
    let observed_test_paths = native_tool_observed_test_paths(receipts);
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "completion_evidence_repair_controller",
            "reason": "product_repair_requires_file_write_or_file_patch_before_more_context_reads",
            "required_next_tool": "file_write_or_file_patch",
            "missing_changed_paths": missing_changed_paths,
            "observed_test_paths": observed_test_paths
        }),
        error: Some(
            "product_repair_requires_file_write_or_file_patch_before_more_context_reads"
                .to_string(),
        ),
    }
}

fn native_tool_compact_mutation_context_blocked_receipt(
    call: NativeToolCall,
) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "compact_mutation_entry_controller",
            "reason": "compact_mutation_entry_requires_mutation_or_memory_context_before_more_discovery",
            "required_next_tool": "file_write_or_file_patch_or_controlled_shell_edit_batch"
        }),
        error: Some(
            "compact_mutation_entry_requires_mutation_or_memory_context_before_more_discovery"
                .to_string(),
        ),
    }
}

fn native_tool_missing_changed_paths(repair_reasons: &[String]) -> Vec<String> {
    native_tool_required_repair_paths(repair_reasons)
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn native_tool_required_repair_paths(repair_reasons: &[String]) -> Vec<&str> {
    repair_reasons
        .iter()
        .filter_map(|reason| {
            reason
                .strip_prefix("missing_changed_path:")
                .or_else(|| reason.strip_prefix("unresolved_owner_source_path:"))
                .or_else(|| native_tool_missing_imported_module_repair_path(reason))
        })
        .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
        .collect()
}

fn native_tool_missing_imported_module_repair_path(reason: &str) -> Option<&str> {
    let rest = reason.strip_prefix("missing_imported_module:")?;
    rest.rsplit_once(':').map(|(_module, path)| path)
}

fn native_tool_observed_test_paths(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = Vec::new();
    for receipt in receipts {
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            continue;
        };
        let lower = path.replace('\\', "/").to_ascii_lowercase();
        if (lower.contains("/tests/")
            || lower.contains("/test/")
            || lower.contains("test_")
            || lower.ends_with(".test.js")
            || lower.ends_with(".spec.js")
            || lower.ends_with(".test.ts")
            || lower.ends_with(".spec.ts"))
            && !paths.iter().any(|existing| existing == path)
        {
            paths.push(path.to_string());
        }
    }
    paths
}

fn native_tool_call_with_prompt_defaults(
    mut call: NativeToolCall,
    original_prompt: &str,
) -> NativeToolCall {
    let project_root = native_tool_prompt_project_root(original_prompt);
    if let Some(project_root) = project_root.as_deref() {
        native_tool_apply_project_relative_file_paths(&mut call, project_root);
    }
    if native_tool_call_is_command_run(&call) && !native_tool_command_args_have_cwd(&call.args) {
        if let Some(project_root) = project_root {
            if !call.args.is_object() {
                call.args = json!({});
            }
            if let Some(args) = call.args.as_object_mut() {
                args.insert("cwd".to_string(), json!(project_root));
            }
        }
    }
    call
}

fn native_tool_apply_project_relative_file_paths(call: &mut NativeToolCall, project_root: &str) {
    if !native_tool_call_is_file_path_tool(call) || !call.args.is_object() {
        return;
    }
    let Some(args) = call.args.as_object_mut() else {
        return;
    };
    if !args.contains_key("path")
        && matches!(
            call.name.trim().to_ascii_lowercase().as_str(),
            "file_patch" | "patch_file" | "workspace.patch" | "workspace_patch"
        )
    {
        if let Some(diff_path) = args
            .get("patch")
            .and_then(Value::as_str)
            .and_then(native_tool_unified_diff_target_path)
        {
            args.insert(
                "path".to_string(),
                json!(native_tool_project_relative_path(&diff_path, project_root)),
            );
        }
    }
    let path_keys = [
        "path",
        "file_path",
        "filepath",
        "target_path",
        "target_file_path",
        "target",
        "file",
        "absolute_path",
        "full_path",
        "output_path",
        "destination",
        "dest",
        "filename",
    ];
    let mut normalized_path = None;
    for key in path_keys {
        let Some(value) = args.get(key).and_then(Value::as_str) else {
            continue;
        };
        let normalized = native_tool_project_relative_path(value, project_root);
        args.insert(key.to_string(), json!(normalized.clone()));
        normalized_path.get_or_insert(normalized);
    }
    if !args.contains_key("path") {
        if let Some(path) = normalized_path {
            args.insert("path".to_string(), json!(path));
        }
    }
    if let Some(paths) = args.get_mut("paths").and_then(Value::as_array_mut) {
        for path in paths {
            let Some(value) = path.as_str() else {
                continue;
            };
            *path = json!(native_tool_project_relative_path(value, project_root));
        }
    }
}

fn native_tool_project_relative_path(path: &str, project_root: &str) -> String {
    let path = path.trim();
    if path.is_empty() || std::path::Path::new(path).is_absolute() {
        return path.to_string();
    }
    let path = path.trim_start_matches("./");
    format!("{}/{}", project_root.trim_end_matches('/'), path)
}

fn native_tool_unified_diff_target_path(patch: &str) -> Option<String> {
    for line in patch.lines() {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("+++ b/") {
            return Some(path.trim().to_string());
        }
        if let Some(path) = trimmed.strip_prefix("+++ ") {
            let path = path.trim();
            if path != "/dev/null" {
                return Some(path.trim_start_matches("b/").to_string());
            }
        }
        if let Some(rest) = trimmed.strip_prefix("diff --git ") {
            let mut parts = rest.split_whitespace();
            let _old = parts.next();
            if let Some(new_path) = parts.next() {
                return Some(new_path.trim_start_matches("b/").to_string());
            }
        }
    }
    None
}

fn native_tool_call_is_file_path_tool(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_list"
            | "list_files"
            | "workspace.list"
            | "workspace_list"
            | "file_stat"
            | "stat_file"
            | "file_exists"
            | "workspace.stat"
            | "workspace_stat"
            | "file_read"
            | "read_file"
            | "workspace.read"
            | "workspace_read"
            | "file_read_many"
            | "read_many_files"
            | "workspace.read_many"
            | "workspace_read_many"
            | "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    )
}

fn native_tool_call_is_command_run(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run"
    )
}

fn native_tool_command_args_have_cwd(args: &Value) -> bool {
    [
        "cwd",
        "path",
        "working_directory",
        "working_dir",
        "workdir",
        "directory",
        "dir",
        "project_root",
        "root",
    ]
    .iter()
    .any(|key| args.get(*key).and_then(Value::as_str).is_some())
}

fn native_tool_call_targets_handoff_artifact(call: &NativeToolCall) -> bool {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    ) {
        return false;
    }
    let path = call
        .args
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    path.contains("/receipts/")
        || path.contains("\\receipts\\")
        || path.contains("checkpoint_")
        || path.contains("handoff")
}

fn native_tool_handoff_blocked_repair_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: Value::Null,
        error: Some(
            "checkpoint_or_handoff_write_blocked_until_product_source_and_test_evidence_is_complete"
                .to_string(),
        ),
    }
}

fn native_tool_live_stage_blocked_receipt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    call: &NativeToolCall,
) -> Option<NativeToolReceipt> {
    if native_tool_is_probable_micro_direct_write_task(metadata, original_prompt) {
        return None;
    }
    if !native_tool_profile_allows(metadata, original_prompt, 4) {
        return None;
    }
    if !native_tool_staged_execution_controller_enabled(metadata) {
        return None;
    }
    let stage = native_tool_current_live_stage(original_prompt, receipts)?;
    let live_repair_reasons = native_tool_live_stage_repair_reasons(original_prompt, receipts);
    native_tool_stage_block_reason(metadata, original_prompt, stage, receipts, call).map(|reason| {
        native_tool_stage_blocked_repair_receipt(
            call,
            stage,
            reason,
            &live_repair_reasons,
            receipts,
        )
    })
}

fn native_tool_current_live_stage(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Option<&'static str> {
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if native_tool_prompt_looks_checkpointed_project_operator(original_prompt) {
        if native_tool_prompt_requires_product_mutation(&prompt_lower)
            && !native_tool_checkpointed_product_slice_stage_satisfied(original_prompt, receipts)
        {
            return Some("product_mutation");
        }
        if native_tool_prompt_wants_operator_surface(original_prompt)
            && !native_tool_has_successful_operator_surface_mutation(receipts)
        {
            return Some("operator_surface_mutation");
        }
        if native_tool_prompt_requires_test_changes(&prompt_lower)
            && !native_tool_has_successful_test_mutation(receipts)
        {
            return Some("test_mutation");
        }
        if !native_tool_failed_validation_import_repair_reasons(original_prompt, receipts).is_empty()
        {
            return Some("validation_repair");
        }
    }
    if native_tool_has_successful_product_mutation(receipts)
        && native_tool_prompt_requires_test_changes(&prompt_lower)
        && !native_tool_has_successful_test_mutation(receipts)
    {
        return Some("test_mutation");
    }
    if native_tool_prompt_requires_product_mutation(&prompt_lower)
        && !native_tool_product_source_stage_satisfied(original_prompt, receipts)
    {
        return Some("product_mutation");
    }
    if native_tool_prompt_requires_test_changes(&prompt_lower)
        && !native_tool_has_successful_test_mutation(receipts)
    {
        return Some("test_mutation");
    }
    if !native_tool_failed_validation_import_repair_reasons(original_prompt, receipts).is_empty() {
        return Some("validation_repair");
    }
    if native_tool_prompt_requires_validation_command(&prompt_lower)
        && !native_tool_has_successful_validation_after_latest_mutation(receipts)
    {
        return Some("validation");
    }
    if !native_tool_semantic_closeout_gaps(original_prompt, receipts).is_empty() {
        return Some("semantic_closeout");
    }
    if native_tool_prompt_required_handoff_artifact_paths(original_prompt)
        .into_iter()
        .any(|path| !native_tool_has_successful_mutation_for_suffix(receipts, &path))
    {
        return Some("checkpoint_handoff");
    }
    if native_tool_prompt_requires_memory_write(&prompt_lower)
        && !native_tool_has_successful_memory_write_command(
            receipts,
            native_tool_prompt_expected_memory_row_id(original_prompt).as_deref(),
            original_prompt,
        )
    {
        return Some("memory_closure");
    }
    None
}

fn native_tool_prompt_looks_checkpointed_project_operator(original_prompt: &str) -> bool {
    let lower = original_prompt.to_ascii_lowercase();
    native_tool_prompt_mentions_any(
        &lower,
        &["checkpoint", "handoff", "resume token", "memory row", "project context"],
    ) && native_tool_prompt_mentions_any(
        &lower,
        &["existing project", "project root", "local project", "validation command"],
    )
}

fn native_tool_prompt_wants_operator_surface(original_prompt: &str) -> bool {
    let lower = original_prompt.to_ascii_lowercase();
    native_tool_prompt_mentions_any(
        &lower,
        &[
            "cli",
            "operator",
            "report",
            "import",
            "export",
            "roundtrip",
            "round-trip",
            "public api",
            "surface",
        ],
    )
}

fn native_tool_prompt_requires_semantic_closeout(original_prompt: &str) -> bool {
    native_tool_prompt_looks_checkpointed_project_operator(original_prompt)
        && native_tool_prompt_wants_operator_surface(original_prompt)
}

fn native_tool_semantic_closeout_gaps(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    if !native_tool_prompt_requires_semantic_closeout(original_prompt)
        || !native_tool_has_successful_validation_after_latest_mutation(receipts)
    {
        return Vec::new();
    }

    let prompt_lower = original_prompt.to_ascii_lowercase();
    let mut gaps = Vec::new();
    if !native_tool_has_successful_semantic_closeout_command_after_latest_validation(receipts) {
        gaps.push("missing_operator_semantic_probe_receipt".to_string());
    }
    if native_tool_prompt_mentions_any(&prompt_lower, &["report", "summary"])
        && !native_tool_has_successful_semantic_command_matching_after_latest_validation(
            receipts,
            &["report", "summary"],
        )
    {
        gaps.push("missing_report_semantic_probe_receipt".to_string());
    }
    if native_tool_prompt_mentions_any(
        &prompt_lower,
        &["import", "export", "roundtrip", "round-trip"],
    ) && !native_tool_has_successful_semantic_command_matching_after_latest_validation(
        receipts,
        &["import", "export", "roundtrip", "round-trip"],
    ) {
        gaps.push("missing_roundtrip_semantic_probe_receipt".to_string());
    }
    gaps.sort();
    gaps.dedup();
    gaps
}

fn native_tool_has_successful_product_source_foundation_mutation(
    receipts: &[NativeToolReceipt],
) -> bool {
    let changed = native_tool_changed_paths(receipts);
    !native_tool_mutation_entry_role_paths(&changed, "product_source").is_empty()
}

fn native_tool_checkpointed_product_slice_stage_satisfied(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_has_successful_product_source_foundation_mutation(receipts)
        && native_tool_product_slice_gaps(original_prompt, receipts).is_empty()
}

fn native_tool_has_successful_operator_surface_mutation(receipts: &[NativeToolReceipt]) -> bool {
    let changed = native_tool_changed_paths(receipts);
    !native_tool_merge_role_paths(
        &native_tool_mutation_entry_role_paths(&changed, "operator_surface"),
        &native_tool_mutation_entry_role_paths(&changed, "export_surface"),
    )
    .is_empty()
}

fn native_tool_should_defer_auto_validation_for_stage(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    matches!(
        native_tool_current_live_stage(original_prompt, receipts),
        Some("product_mutation" | "operator_surface_mutation" | "test_mutation")
    )
}

fn native_tool_checkpointed_project_has_live_stage(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    (native_tool_checkpointed_project_operator_tool_loop_active(metadata, original_prompt)
        || native_tool_prompt_looks_checkpointed_project_operator(original_prompt))
        && native_tool_current_live_stage(original_prompt, receipts).is_some()
}

fn native_tool_turn_number_usize(turn_idx: u64) -> usize {
    usize::try_from(turn_idx.saturating_add(1)).unwrap_or(usize::MAX)
}

fn native_tool_checkpointed_project_stage_prompt(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    stage: &str,
    observation: &str,
    turn_number: usize,
) -> String {
    let packet = native_tool_project_operator_context_packet(
        metadata,
        original_prompt,
        receipts,
        stage,
        observation,
        turn_number,
    );
    format!(
        "Use the compact project-operator context packet below. Emit native tool calls for this stage only.\n{}",
        serde_json::to_string_pretty(&packet).unwrap_or_else(|_| "{}".to_string())
    )
}

fn native_tool_project_operator_context_packet(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    stage: &str,
    observation: &str,
    turn_number: usize,
) -> Value {
    let project_root = native_tool_prompt_project_root(original_prompt)
        .unwrap_or_else(|| "<unknown_project_root>".to_string());
    let validation_command = native_tool_prompt_validation_shell_command(original_prompt)
        .unwrap_or_else(|| "<run the prompt-specified validation command>".to_string());
    let handoff_paths = native_tool_prompt_required_handoff_artifact_paths(original_prompt);
    let memory_row_id = native_tool_prompt_expected_memory_row_id(original_prompt)
        .unwrap_or_else(|| "<expected_memory_row_id>".to_string());
    let memory_cli = native_tool_prompt_memory_cli_pattern(original_prompt)
        .unwrap_or_else(|| "<prompt-specified memory CLI>".to_string());
    json!({
        "controller": "project_operator_context_packet_v1",
        "turn": turn_number,
        "active_stage": stage,
        "project_root": project_root,
        "task_excerpt": native_tool_checkpointed_project_task_excerpt(original_prompt),
        "allowed_next_tools": native_tool_checkpointed_project_stage_allowed_tools(stage),
        "stage_goal": native_tool_checkpointed_project_stage_goal(stage),
        "stage_stop_condition": native_tool_checkpointed_project_stage_stop_condition(stage),
        "changed_paths": native_tool_changed_paths(receipts),
        "repair_reasons": native_tool_live_stage_repair_reasons(original_prompt, receipts),
        "product_slice_gaps": native_tool_product_slice_gaps(original_prompt, receipts),
        "semantic_closeout_gaps": native_tool_semantic_closeout_gaps(original_prompt, receipts),
        "handoff_paths": handoff_paths,
        "validation_command": validation_command,
        "memory_row_id": memory_row_id,
        "memory_cli": memory_cli,
        "failed_validation": native_tool_failed_validation_receipt_details(receipts),
        "context_receipts": native_tool_project_operator_receipt_summary(receipts),
        "public_shape_lines_to_preserve": native_tool_project_operator_public_shape_lines(receipts),
        "public_contract_lines_to_preserve": if native_tool_public_contract_packet_enabled(metadata) {
            native_tool_project_operator_public_contract_lines(receipts)
        } else {
            Vec::<String>::new()
        },
        "compact_observation": native_tool_project_operator_compact_observation(
            metadata,
            observation,
        ),
        "rules": [
            "Complete only active_stage.",
            "Use the compact context packet as the source of truth for the next action.",
            "Do not restart planning.",
            "Preserve observed public API/class constructor fields and existing CLI behavior unless the task explicitly asks for a breaking change.",
            "Prefer additive product/source changes over rewrites.",
            "Do not run validation before product/operator/test stages are complete.",
            "Do not treat validation as terminal when operator/public semantic closeout, handoff, or memory closure is still required.",
            "Do not close memory or handoff before validation and required semantic closeout probes pass.",
            "If blocked, return the smallest receipt-backed blocker instead of broad exploration."
        ],
        "winning_runtime_pattern": [
            "read compact local context and checkpoint memory",
            "mutate product source additively",
            "mutate operator surface additively",
            "add tests that cover new behavior and baseline preservation",
            "validate",
            "run semantic operator probes",
            "write handoff and memory closure receipts"
        ],
        "workflow_profile": metadata
            .get("workflow")
            .and_then(|workflow| workflow.get("id"))
            .cloned()
            .unwrap_or(Value::Null)
    })
}

fn native_tool_project_operator_receipt_summary(
    receipts: &[NativeToolReceipt],
) -> Vec<Value> {
    receipts
        .iter()
        .rev()
        .take(16)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|receipt| {
            json!({
                "call_id": receipt.call_id.clone(),
                "tool": receipt.tool_name.clone(),
                "status": receipt.status.clone(),
                "path": native_tool_project_operator_receipt_path_hint(receipt),
                "command": native_tool_receipt_command_text(receipt),
                "error": receipt.error.clone(),
            })
        })
        .collect()
}

fn native_tool_project_operator_receipt_path_hint(
    receipt: &NativeToolReceipt,
) -> Option<String> {
    [
        "path",
        "file_path",
        "absolute_path",
        "relative_path",
        "target_path",
        "cwd",
    ]
    .into_iter()
    .find_map(|key| {
        receipt
            .result
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    })
    .or_else(|| {
        receipt
            .result
            .pointer("/args/path")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
    .or_else(|| {
        receipt
            .result
            .pointer("/arguments/path")
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn native_tool_project_operator_public_shape_lines(
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut lines = Vec::new();
    for receipt in receipts {
        if receipt.status != "ok" {
            continue;
        }
        let mut corpus = String::new();
        native_tool_project_operator_append_text_fragments(
            &receipt.result,
            &mut corpus,
            60_000,
        );
        for line in corpus.lines() {
            let trimmed = line.trim();
            if native_tool_project_operator_looks_public_shape_line(trimmed) {
                lines.push(trimmed.chars().take(220).collect::<String>());
            }
            if lines.len() >= 48 {
                break;
            }
        }
        if lines.len() >= 48 {
            break;
        }
    }
    lines.sort();
    lines.dedup();
    lines.truncate(32);
    lines
}

fn native_tool_project_operator_public_contract_lines(
    receipts: &[NativeToolReceipt],
) -> Vec<String> {
    let mut lines = Vec::new();
    for receipt in receipts {
        if let Some(files) = receipt.result.get("files").and_then(Value::as_array) {
            for file in files {
                let path = file
                    .get("relative_path")
                    .and_then(Value::as_str)
                    .or_else(|| file.get("path").and_then(Value::as_str))
                    .unwrap_or("<observed_file>");
                if let Some(content) = file.get("content").and_then(Value::as_str) {
                    native_tool_mutation_entry_extend_public_contract_lines(
                        &mut lines,
                        path,
                        content,
                    );
                }
            }
        }
        if let Some(path) = receipt.result.get("path").and_then(Value::as_str) {
            if let Some(content) = receipt.result.get("content").and_then(Value::as_str) {
                native_tool_mutation_entry_extend_public_contract_lines(&mut lines, path, content);
            }
        }
        if receipt.tool_name.eq_ignore_ascii_case("command_run") {
            for key in ["stdout", "stderr", "output"] {
                if let Some(text) = receipt.result.get(key).and_then(Value::as_str) {
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if native_tool_line_looks_public_contract_line(trimmed, true) {
                            native_tool_push_unique_string(
                                &mut lines,
                                format!(
                                    "validation_output: {}",
                                    trimmed.chars().take(220).collect::<String>()
                                ),
                            );
                        }
                    }
                }
            }
        }
        if lines.len() >= 64 {
            break;
        }
    }
    lines.sort();
    lines.dedup();
    lines.truncate(32);
    lines
}

fn native_tool_project_operator_append_text_fragments(
    value: &Value,
    out: &mut String,
    max_chars: usize,
) {
    if out.chars().count() >= max_chars {
        return;
    }
    match value {
        Value::String(text) => {
            if text.contains('\n')
                || native_tool_project_operator_looks_public_shape_line(text.trim())
            {
                let remaining = max_chars.saturating_sub(out.chars().count());
                if remaining > 0 {
                    out.push_str(&text.chars().take(remaining).collect::<String>());
                    out.push('\n');
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                native_tool_project_operator_append_text_fragments(item, out, max_chars);
                if out.chars().count() >= max_chars {
                    break;
                }
            }
        }
        Value::Object(object) => {
            for value in object.values() {
                native_tool_project_operator_append_text_fragments(value, out, max_chars);
                if out.chars().count() >= max_chars {
                    break;
                }
            }
        }
        _ => {}
    }
}

fn native_tool_project_operator_looks_public_shape_line(line: &str) -> bool {
    if line.is_empty() || line.len() > 260 {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    line.starts_with("class ")
        || line.starts_with("def ")
        || line.starts_with("fn ")
        || line.starts_with("pub fn ")
        || line.starts_with("struct ")
        || line.starts_with("pub struct ")
        || line.starts_with("enum ")
        || line.starts_with("pub enum ")
        || line.starts_with("interface ")
        || line.starts_with("type ")
        || line.starts_with("export ")
        || lower.contains("@dataclass")
        || lower.contains("argumentparser")
        || lower.contains("add_parser(")
        || lower.contains("subparsers")
}

fn native_tool_project_operator_compact_observation(
    metadata: &Value,
    observation: &str,
) -> String {
    if !native_tool_project_operator_context_packet_enabled(metadata) {
        return observation.to_string();
    }
    let max_chars = native_tool_success_criteria_u64(
        metadata,
        "project_operator_context_packet_max_observation_chars",
    )
    .unwrap_or(6000)
    .clamp(1000, 20_000) as usize;
    observation.chars().take(max_chars).collect()
}

fn native_tool_project_operator_context_packet_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| criteria.get("project_operator_context_packet_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_project_operator_first_mutation_lane_enabled(
    metadata: &Value,
    stage: &str,
) -> bool {
    matches!(
        stage,
        "product_mutation" | "operator_surface_mutation" | "test_mutation"
    ) && metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|criteria| criteria.get("project_operator_first_mutation_artifact_lane_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_project_operator_first_mutation_provider_timeout_seconds(
    metadata: &Value,
) -> u64 {
    native_tool_success_criteria_u64(
        metadata,
        "project_operator_first_mutation_provider_timeout_seconds",
    )
    .unwrap_or(75)
    .clamp(20, 180)
}

fn native_tool_project_operator_first_mutation_max_calls(metadata: &Value) -> usize {
    native_tool_success_criteria_u64(metadata, "project_operator_first_mutation_max_calls")
        .unwrap_or(4)
        .clamp(1, 8) as usize
}

fn native_tool_project_operator_first_mutation_system() -> String {
    "You are a constrained code edit engine. Return exactly one JSON object with a tool_calls array. Do not write prose, markdown, analysis, or a final answer. Prefer concrete file_write/file_patch edits or one controlled command_run shell edit batch. Preserve existing public API shape and public/test contract call sites unless the task explicitly asks for a breaking change."
        .to_string()
}

fn native_tool_project_operator_first_mutation_prompt(stage_prompt: &str, stage: &str) -> String {
    format!(
        "{stage_prompt}\n\nFIRST_MUTATION_ARTIFACT_LANE:\n\
- Active stage: {stage}\n\
- Return JSON only: {{\"tool_calls\":[...]}}.\n\
- The first tool call must mutate files for the active stage.\n\
- Allowed mutation tools: file_write, file_patch, or one controlled command_run heredoc edit batch.\n\
- Do not call read/list/stat tools in this lane; context is already provided.\n\
- Do not run validation, write handoff artifacts, or write memory rows in this lane.\n\
- Preserve public_shape_lines_to_preserve, public_contract_lines_to_preserve, and existing CLI behavior unless the task explicitly says otherwise.\n\
- If no safe mutation is possible from the packet, return {{\"tool_calls\":[]}}."
    )
}

fn native_tool_checkpointed_project_stage_allowed_tools(stage: &str) -> &'static str {
    match stage {
        "product_mutation" => {
            "file_write, file_patch, or one controlled shell edit batch against product source files"
        }
        "operator_surface_mutation" => {
            "file_write, file_patch, or one controlled shell edit batch against CLI/report/import/export surface files"
        }
        "test_mutation" => "file_write or file_patch against tests",
        "validation_repair" => {
            "file_write or file_patch against the module/file named by the failed validation import error"
        }
        "validation" => {
            "command_run for validation, or file_patch/file_write only to repair failed validation"
        }
        "semantic_closeout" => {
            "command_run for product/operator semantic probes, or file_patch/file_write only to repair a failed semantic probe"
        }
        "checkpoint_handoff" => {
            "file_write or file_patch for the required checkpoint/handoff artifact"
        }
        "memory_closure" => "command_run using the prompt-specified memory CLI",
        _ => "the smallest native tool call needed for the active stage",
    }
}

fn native_tool_checkpointed_project_stage_goal(stage: &str) -> &'static str {
    match stage {
        "product_mutation" => {
            "Implement the missing durable product slice in source files before validation. Preserve observed public API/class constructor fields and baseline behavior; prefer additive changes."
        }
        "operator_surface_mutation" => {
            "Expose the completed product slice through the requested operator-facing CLI/report/import/export surface without removing existing CLI commands or outputs."
        }
        "test_mutation" => {
            "Add regression tests for the completed product and operator surface plus baseline preservation of existing behavior."
        }
        "validation_repair" => {
            "Repair the smallest failed-validation edge, such as defining/exporting the missing imported symbol in the resolved module file."
        }
        "validation" => {
            "Run validation after the latest mutation, or make the smallest repair indicated by failed validation output."
        }
        "semantic_closeout" => {
            "Run the smallest product/operator probe commands that demonstrate the requested public or operator-facing behavior after validation."
        }
        "checkpoint_handoff" => {
            "Write the required checkpoint handoff artifact with changed files, validation result, risks, and next checkpoint."
        }
        "memory_closure" => {
            "Write the required checkpoint memory row with changed files, validation result, risks, and next checkpoint."
        }
        _ => "Complete the current stage with the smallest receipt-backed action.",
    }
}

fn native_tool_checkpointed_project_stage_stop_condition(stage: &str) -> &'static str {
    match stage {
        "product_mutation" => {
            "Stop after additive source receipts satisfy product slice evidence while preserving observed public API shape; do not validate yet."
        }
        "operator_surface_mutation" => {
            "Stop after operator-surface mutation receipts exist and existing operator paths remain represented; do not validate yet."
        }
        "test_mutation" => {
            "Stop after test mutation receipts exist for both new behavior and baseline preservation; do not validate yet."
        }
        "validation_repair" => {
            "Stop after the failed-validation target module has been patched; validation will rerun next."
        }
        "validation" => "Stop after validation passes after the latest mutation.",
        "semantic_closeout" => {
            "Stop after successful product/operator semantic probe receipts cover the requested surface."
        }
        "checkpoint_handoff" => "Stop after the checkpoint/handoff artifact write receipt exists.",
        "memory_closure" => "Stop after the expected memory row write command receipt exists.",
        _ => "Stop when the current stage receipt exists.",
    }
}

fn native_tool_checkpointed_project_task_excerpt(original_prompt: &str) -> String {
    let mut selected = Vec::new();
    for line in original_prompt.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let starts_numbered_requirement = trimmed
            .chars()
            .next()
            .map(|ch| ch.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.contains('.');
        if trimmed.starts_with("Goal:")
            || trimmed.starts_with("Workflow requirements:")
            || trimmed.starts_with("Completion guardrails:")
            || starts_numbered_requirement
        {
            selected.push(trimmed.to_string());
        }
        if selected.join("\n").len() > 1800 {
            break;
        }
    }
    if selected.is_empty() {
        native_tool_compact_text(original_prompt)
    } else {
        selected.join("\n")
    }
}

fn native_tool_staged_repair_blocked_receipt(
    metadata: &Value,
    original_prompt: &str,
    repair_reasons: &[String],
    receipts: &[NativeToolReceipt],
    call: &NativeToolCall,
) -> Option<NativeToolReceipt> {
    if native_tool_is_probable_micro_direct_write_task(metadata, original_prompt) {
        return None;
    }
    if !native_tool_profile_allows(metadata, original_prompt, 4) {
        return None;
    }
    if !native_tool_staged_execution_controller_enabled(metadata) {
        return None;
    }
    let stage = native_tool_current_repair_stage(repair_reasons, receipts)?;
    native_tool_stage_block_reason(metadata, original_prompt, stage, receipts, call).map(|reason| {
        native_tool_stage_blocked_repair_receipt(call, stage, reason, repair_reasons, receipts)
    })
}

fn native_tool_staged_execution_controller_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("staged_execution_controller_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_stage_block_reason(
    metadata: &Value,
    original_prompt: &str,
    stage: &str,
    receipts: &[NativeToolReceipt],
    call: &NativeToolCall,
) -> Option<&'static str> {
    let has_context = native_tool_has_successful_context_receipt(receipts);
    let read_context_count = native_tool_successful_read_context_count(receipts);
    let has_product_source = native_tool_product_source_stage_satisfied(original_prompt, receipts);
    let checkpointed_project_operator =
        native_tool_prompt_looks_checkpointed_project_operator(original_prompt);
    let needs_requested_pre_mutation_validation =
        native_tool_prompt_requires_pre_mutation_validation(original_prompt)
            && !native_tool_has_any_validation_command(receipts);
    match stage {
        "product_mutation" => {
            if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_product_source_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_context_retrieval(call) {
                None
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_product_source_before_memory_closure")
            } else if needs_requested_pre_mutation_validation && native_tool_call_is_mutation(call)
            {
                Some("staged_controller_requires_requested_validation_before_mutation")
            } else if checkpointed_project_operator && native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_product_source_mutation_before_more_context")
            } else if checkpointed_project_operator && native_tool_call_is_test_mutation(call) {
                Some("staged_controller_requires_product_source_mutation_before_test_mutation")
            } else if checkpointed_project_operator
                && native_tool_call_is_operator_surface_mutation(call)
            {
                Some("staged_controller_requires_product_source_mutation_before_operator_surface")
            } else if native_tool_call_is_test_mutation(call) {
                None
            } else if !has_product_source
                && native_tool_call_is_command_run(call)
                && !needs_requested_pre_mutation_validation
            {
                if native_tool_bounded_direct_edit_lane_active(metadata, original_prompt)
                    && native_tool_has_successful_mutation(receipts)
                {
                    None
                } else {
                    Some("staged_controller_requires_product_source_mutation_before_command")
                }
            } else if !has_product_source
                && read_context_count >= native_tool_pre_mutation_read_budget(metadata)
                && native_tool_call_is_context_only(call)
            {
                if native_tool_bounded_direct_edit_lane_active(metadata, original_prompt)
                    && native_tool_has_successful_product_mutation(receipts)
                {
                    None
                } else {
                    Some("staged_controller_requires_product_source_mutation_before_more_context")
                }
            } else {
                None
            }
        }
        "operator_surface_mutation" => {
            if native_tool_call_is_operator_surface_mutation(call) {
                None
            } else if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_operator_surface_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_operator_surface_before_memory_closure")
            } else if native_tool_call_is_command_run(call) {
                Some("staged_controller_requires_operator_surface_mutation_before_command")
            } else if native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_operator_surface_mutation_before_more_context")
            } else if native_tool_call_is_mutation(call) {
                Some("staged_controller_requires_operator_surface_mutation_now")
            } else {
                Some("staged_controller_requires_operator_surface_mutation_now")
            }
        }
        "test_mutation" => {
            if native_tool_call_is_test_mutation(call) {
                None
            } else if native_tool_call_is_command_run(call) {
                Some("staged_controller_requires_test_file_write_or_patch_before_validation")
            } else if native_tool_call_targets_handoff_artifact(call) {
                Some(
                    "staged_controller_requires_test_file_write_or_patch_before_checkpoint_handoff",
                )
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_test_file_write_or_patch_before_memory_closure")
            } else if native_tool_call_is_mutation(call) {
                Some("staged_controller_requires_test_file_write_or_patch_now_for_requested_tests")
            } else if checkpointed_project_operator && native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_test_file_write_or_patch_now_for_requested_tests")
            } else if native_tool_call_is_context_only(call)
                && native_tool_observed_test_paths(receipts).is_empty()
                && native_tool_successful_read_context_count_after_first_mutation(receipts) < 2
            {
                None
            } else {
                Some("staged_controller_requires_test_file_write_or_patch_now_for_requested_tests")
            }
        }
        "validation_repair" => {
            if native_tool_call_targets_failed_validation_import_repair_path(
                original_prompt,
                receipts,
                call,
            ) {
                None
            } else if native_tool_call_is_command_run(call) {
                Some("staged_controller_requires_failed_validation_import_patch_before_command")
            } else if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_failed_validation_import_patch_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_failed_validation_import_patch_before_memory_closure")
            } else if native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_failed_validation_import_patch_before_more_context")
            } else if native_tool_call_is_mutation(call) {
                Some("staged_controller_requires_failed_validation_import_patch_to_target_module")
            } else {
                Some("staged_controller_requires_failed_validation_import_patch_to_target_module")
            }
        }
        "validation" => {
            let has_failed_validation =
                !native_tool_failed_validation_command_refs(receipts).is_empty();
            let failed_validation_read_count =
                native_tool_successful_read_context_count_after_first_failed_validation(receipts);
            if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_validation_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_validation_before_memory_closure")
            } else if native_tool_call_is_command_run(call) {
                None
            } else if native_tool_call_is_mutation(call) {
                None
            } else if has_failed_validation
                && native_tool_call_is_context_only(call)
                && failed_validation_read_count < 2
            {
                None
            } else if has_failed_validation && native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_source_or_generated_test_patch_now_using_failed_validation_output")
            } else if has_context && native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_validation_or_validation_repair_before_more_context")
            } else {
                Some("staged_controller_requires_validation_command_or_validation_repair")
            }
        }
        "semantic_closeout" => {
            if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_semantic_closeout_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_semantic_closeout_before_memory_closure")
            } else if native_tool_call_is_semantic_closeout_command(call) {
                None
            } else if native_tool_call_is_mutation(call)
                && native_tool_has_failed_semantic_closeout_command_after_latest_validation(receipts)
            {
                None
            } else if native_tool_call_is_mutation(call) {
                Some("staged_controller_requires_semantic_probe_before_repair_mutation")
            } else if native_tool_call_is_context_only(call) {
                Some("staged_controller_requires_semantic_closeout_probe_before_more_context")
            } else {
                Some("staged_controller_requires_product_operator_semantic_probe")
            }
        }
        "checkpoint_handoff" => {
            if native_tool_call_targets_handoff_artifact(call) {
                None
            } else {
                Some("staged_controller_requires_checkpoint_handoff_before_memory_or_finalization")
            }
        }
        "memory_closure" => {
            if native_tool_call_is_memory_closure(call) {
                None
            } else {
                Some("staged_controller_requires_memory_closure_command_before_finalization")
            }
        }
        _ => None,
    }
}

fn native_tool_prompt_required_handoff_artifact_paths(original_prompt: &str) -> Vec<String> {
    let mut paths = native_tool_prompt_required_changed_paths(original_prompt)
        .into_iter()
        .filter(|path| native_tool_path_is_handoff_artifact(path))
        .collect::<Vec<_>>();
    for raw in original_prompt.split_whitespace() {
        let candidate = raw.trim_matches(|ch: char| {
            ch == '`'
                || ch == '"'
                || ch == '\''
                || ch == '.'
                || ch == ','
                || ch == ';'
                || ch == ':'
                || ch == ')'
                || ch == '('
                || ch == ']'
                || ch == '['
                || ch == '}'
                || ch == '{'
        });
        let lower = candidate.replace('\\', "/").to_ascii_lowercase();
        if candidate.contains('/')
            && lower.ends_with(".json")
            && native_tool_path_is_handoff_artifact(&lower)
            && !paths.iter().any(|path| path == candidate)
        {
            paths.push(candidate.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn native_tool_path_is_handoff_artifact(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/receipts/")
        || lower.contains("\\receipts\\")
        || lower.contains("checkpoint_")
        || lower.contains("handoff")
}

fn native_tool_current_repair_stage(
    repair_reasons: &[String],
    receipts: &[NativeToolReceipt],
) -> Option<&'static str> {
    if native_tool_has_successful_product_mutation(receipts)
        && native_tool_repair_reasons_include_test_change(repair_reasons)
        && !native_tool_has_successful_test_mutation(receipts)
    {
        return Some("test_mutation");
    }
    if native_tool_repair_reasons_include_product_mutation(repair_reasons)
        || native_tool_repair_reasons_include_product_slice(repair_reasons)
    {
        return Some("product_mutation");
    }
    if native_tool_has_unmutated_required_repair_path(repair_reasons, receipts) {
        return Some("product_mutation");
    }
    if repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
        && !native_tool_has_successful_test_mutation(receipts)
    {
        return Some("test_mutation");
    }
    if native_tool_repair_reasons_include_validation(repair_reasons) {
        return Some("validation");
    }
    if repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .any(|path| {
            let lower = path.to_ascii_lowercase();
            lower.contains("checkpoint_")
                || lower.contains("handoff")
                || lower.contains("/receipts/")
        })
    {
        return Some("checkpoint_handoff");
    }
    if repair_reasons
        .iter()
        .any(|reason| reason.starts_with("missing_memory_write_receipt"))
    {
        return Some("memory_closure");
    }
    None
}

fn native_tool_repair_reasons_include_test_change(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason == "missing_test_change_receipt"
            || reason
                .strip_prefix("missing_changed_path:")
                .map(native_tool_bootstrap_path_looks_like_test)
                .unwrap_or(false)
    })
}

fn native_tool_repair_reasons_include_validation(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason == "missing_validation_command_receipt"
            || reason.starts_with("failed_validation_command_receipt:")
    })
}

fn native_tool_repair_reasons_include_failed_validation(repair_reasons: &[String]) -> bool {
    repair_reasons
        .iter()
        .any(|reason| reason.starts_with("failed_validation_command_receipt:"))
}

fn native_tool_prompt_requires_pre_mutation_validation(original_prompt: &str) -> bool {
    let lower = original_prompt.to_ascii_lowercase();
    let validation_requested = lower.contains("validation")
        || lower.contains("run tests")
        || lower.contains("pytest")
        || lower.contains("unittest");
    validation_requested
        && (lower.contains("first run")
            || lower.contains("before editing")
            || lower.contains("before mutating")
            || lower.contains("before making changes")
            || lower.contains("observe the failure")
            || lower.contains("observe failing")
            || lower.contains("failing tests"))
}

fn native_tool_has_any_validation_command(receipts: &[NativeToolReceipt]) -> bool {
    receipts
        .iter()
        .any(|receipt| receipt.status == "ok" && receipt.tool_name == "command_run")
}

fn native_tool_has_failed_validation_command(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && native_tool_receipt_looks_validation_command(receipt)
            && !receipt
                .result
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
    })
}

fn native_tool_has_successful_semantic_closeout_command_after_latest_validation(
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_semantic_command_after_latest_validation(receipts, None, true)
}

fn native_tool_has_successful_semantic_command_matching_after_latest_validation(
    receipts: &[NativeToolReceipt],
    needles: &[&str],
) -> bool {
    native_tool_semantic_command_after_latest_validation(receipts, Some(needles), true)
}

fn native_tool_has_failed_semantic_closeout_command_after_latest_validation(
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_semantic_command_after_latest_validation(receipts, None, false)
}

fn native_tool_semantic_command_after_latest_validation(
    receipts: &[NativeToolReceipt],
    needles: Option<&[&str]>,
    require_success: bool,
) -> bool {
    let Some(validation_index) = native_tool_latest_successful_validation_index(receipts) else {
        return false;
    };
    let start = validation_index.saturating_add(1);
    receipts.iter().skip(start).any(|receipt| {
        if receipt.tool_name != "command_run" {
            return false;
        }
        if require_success && receipt.status != "ok" {
            return false;
        }
        if !require_success && receipt.status != "error" {
            return false;
        }
        if native_tool_receipt_looks_validation_command(receipt)
            || native_tool_receipt_looks_memory_command(receipt)
        {
            return false;
        }
        let command_text = native_tool_receipt_command_text(receipt);
        needles
            .map(|required| required.iter().any(|needle| command_text.contains(needle)))
            .unwrap_or(true)
    })
}

fn native_tool_latest_successful_validation_index(receipts: &[NativeToolReceipt]) -> Option<usize> {
    receipts.iter().rposition(|receipt| {
        receipt.status == "ok"
            && receipt.tool_name == "command_run"
            && native_tool_receipt_looks_validation_command(receipt)
    })
}

fn native_tool_receipt_looks_validation_command(receipt: &NativeToolReceipt) -> bool {
    let text = native_tool_receipt_command_text(receipt);
    if text.contains("test")
        && (text.contains("unittest")
            || text.contains("pytest")
            || text.contains("ran ")
            || text.contains("passed")
            || text.contains("failures")
            || text.contains("errors"))
    {
        return true;
    }
    native_tool_prompt_mentions_any(
        &text,
        &[
            "unittest",
            "pytest",
            "cargo test",
            "cargo nextest",
            "npm test",
            "pnpm test",
            "yarn test",
            "go test",
            "mvn test",
            "gradle test",
        ],
    )
}

fn native_tool_receipt_looks_memory_command(receipt: &NativeToolReceipt) -> bool {
    let text = native_tool_receipt_command_text(receipt);
    text.contains("memory-cli") || (text.contains("checkpoint") && text.contains("memory"))
}

fn native_tool_has_successful_validation_after_latest_mutation(
    receipts: &[NativeToolReceipt],
) -> bool {
    let Some(latest_mutation_index) = receipts.iter().rposition(|receipt| {
        receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
    }) else {
        return native_tool_has_successful_validation_command(receipts);
    };
    native_tool_has_successful_validation_command(&receipts[latest_mutation_index + 1..])
}

fn native_tool_call_is_mutation(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    )
}

fn native_tool_call_is_test_mutation(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_mutation(call) {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call) else {
        return false;
    };
    let lower = path.to_ascii_lowercase();
    lower.contains("/test")
        || lower.contains("\\test")
        || lower.contains("tests/")
        || lower.contains("test_")
}

fn native_tool_call_is_operator_surface_mutation(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_mutation(call) {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call) else {
        return false;
    };
    native_tool_mutation_entry_path_is_operator_surface(path)
        || native_tool_mutation_entry_path_is_export_surface(path)
}

fn native_tool_call_targets_failed_validation_import_repair_path(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    call: &NativeToolCall,
) -> bool {
    if !native_tool_call_is_mutation(call) {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call) else {
        return false;
    };
    let normalized_path = path.replace('\\', "/").to_ascii_lowercase();
    native_tool_failed_validation_import_repair_reasons(original_prompt, receipts)
        .into_iter()
        .filter_map(|reason| native_tool_missing_imported_module_repair_path(&reason).map(str::to_string))
        .any(|target| {
            let target = target.replace('\\', "/").to_ascii_lowercase();
            normalized_path.ends_with(&target) || target.ends_with(&normalized_path)
        })
}

fn native_tool_call_is_semantic_closeout_command(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_command_run(call) || native_tool_call_is_memory_closure(call) {
        return false;
    }
    let args = serde_json::to_string(&call.args)
        .unwrap_or_default()
        .to_ascii_lowercase();
    !native_tool_prompt_mentions_any(
        &args,
        &[
            "unittest",
            "pytest",
            "cargo test",
            "cargo nextest",
            "npm test",
            "pnpm test",
            "yarn test",
            "go test",
            "mvn test",
            "gradle test",
        ],
    )
}

fn native_tool_call_is_memory_closure(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_command_run(call) {
        return false;
    }
    let args = serde_json::to_string(&call.args)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !(args.contains("memory-cli") || (args.contains("checkpoint") && args.contains("memory"))) {
        return false;
    }
    native_tool_command_text_contains_any(
        &args,
        &[
            " ingest",
            " write",
            " set",
            " put",
            " update",
            " append",
            " checkpoint write",
            "--content",
            "--tags",
        ],
    )
}

fn native_tool_call_is_memory_context_retrieval(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_command_run(call) || native_tool_call_is_memory_closure(call) {
        return false;
    }
    let args = serde_json::to_string(&call.args)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !(args.contains("memory-cli") || (args.contains("checkpoint") && args.contains("memory"))) {
        return false;
    }
    native_tool_command_text_contains_any(
        &args,
        &[
            " get",
            " read",
            " recall",
            " search",
            " retrieve",
            " show",
            " list",
            " query",
        ],
    )
}

fn native_tool_command_text_contains_any(command_text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| command_text.contains(needle))
}

fn native_tool_stage_blocked_repair_receipt(
    call: &NativeToolCall,
    stage: &str,
    reason: &str,
    repair_reasons: &[String],
    receipts: &[NativeToolReceipt],
) -> NativeToolReceipt {
    let missing_changed_paths = native_tool_missing_changed_paths(repair_reasons);
    let observed_test_paths = native_tool_observed_test_paths(receipts);
    NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "staged_execution_controller",
            "required_stage": stage,
            "reason": reason,
            "required_next_tool": if reason.contains("test_file") {
                "file_write_or_file_patch_to_test_path"
            } else if !missing_changed_paths.is_empty() {
                "file_write_or_file_patch_to_missing_changed_path"
            } else if repair_reasons
                .iter()
                .any(|reason| {
                    reason.starts_with("missing_product_source_evidence:")
                        || reason.starts_with("incomplete_product_slice")
                })
            {
                "file_write_or_file_patch_for_missing_product_slice"
            } else {
                "file_write_or_file_patch_or_command_run_for_required_stage"
            },
            "missing_changed_paths": missing_changed_paths,
            "observed_test_paths": observed_test_paths
        }),
        error: Some(reason.to_string()),
    }
}

fn native_tool_turn_required_mutation_blocker_reason(
    receipts: &[NativeToolReceipt],
) -> Option<String> {
    receipts.iter().find_map(|receipt| {
        if receipt.status != "error" {
            return None;
        }
        let reason = receipt
            .error
            .as_deref()
            .or_else(|| receipt.result.get("reason").and_then(Value::as_str))
            .unwrap_or("");
        let required_next_tool = receipt
            .result
            .get("required_next_tool")
            .and_then(Value::as_str)
            .unwrap_or("");
        let direct_mutation_required =
            reason == "product_mutation_must_target_observed_owner_file"
                || reason == "preflight_context_already_loaded_first_mutation_required"
                || required_next_tool.starts_with("file_write_or_file_patch")
                || required_next_tool.starts_with("file_patch_or_file_write");
        if direct_mutation_required {
            return Some(if reason.is_empty() {
                "required_mutation_before_more_context".to_string()
            } else {
                reason.to_string()
            });
        }
        if receipt.tool_name != "command_run" {
            return None;
        }
        let validation_before_mutation =
            reason == "runtime_runs_validation_after_mutation"
                || reason == "first_edit_batch_requires_mutation_before_command"
                || reason == "staged_controller_requires_product_source_mutation_before_command"
                || reason == "staged_controller_requires_test_file_write_or_patch_before_validation"
                || (reason.contains("mutation") && reason.contains("before_command"))
                || reason.contains("before_validation")
                || reason == "shell_edit_existing_source_requires_additive_patch"
                || reason == "shell_edit_existing_python_class_shape_requires_additive_patch";
        if validation_before_mutation {
            Some(if reason.is_empty() {
                "premature_validation_before_mutation".to_string()
            } else {
                reason.to_string()
            })
        } else {
            None
        }
    })
}

fn native_tool_reason_requires_context_to_mutation_recovery(reason: &str) -> bool {
    matches!(
        reason,
        "preflight_context_already_loaded_first_mutation_required"
            | "product_repair_context_blocked_until_mutation"
            | "compact_mutation_context_blocked_until_mutation"
            | "staged_edit_context_blocked_until_mutation"
    )
}

fn native_tool_call_targets_unrelated_repair_path(
    call: &NativeToolCall,
    repair_reasons: &[String],
    original_prompt: &str,
) -> bool {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    ) || native_tool_call_targets_handoff_artifact(call)
    {
        return false;
    }
    let mut required = repair_reasons
        .iter()
        .filter_map(|reason| {
            reason
                .strip_prefix("missing_changed_path:")
                .or_else(|| native_tool_missing_imported_module_repair_path(reason))
        })
        .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
        .map(|path| path.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if required.is_empty()
        && repair_reasons
            .iter()
            .any(|reason| reason == "missing_product_mutation_receipt")
    {
        required = native_tool_prompt_required_changed_paths(original_prompt)
            .into_iter()
            .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
            .map(|path| path.to_ascii_lowercase())
            .collect::<Vec<_>>();
    }
    if required.is_empty() {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call).map(|path| path.to_ascii_lowercase()) else {
        return false;
    };
    let required_has_non_test_path = required.iter().any(|required_path| {
        !(required_path.contains("/test")
            || required_path.contains("\\test")
            || required_path.contains("tests/")
            || required_path.contains("test_"))
    });
    if repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
        && (path.contains("/test")
            || path.contains("\\test")
            || path.contains("tests/")
            || path.contains("test_"))
        && !required_has_non_test_path
    {
        return false;
    }
    if repair_reasons
        .iter()
        .any(|reason| reason.starts_with("missing_product_source_evidence:"))
        && (path.contains("/src/")
            || path.contains("\\src\\")
            || path.ends_with(".py")
            || path.ends_with(".rs")
            || path.ends_with(".ts")
            || path.ends_with(".js"))
    {
        return false;
    }
    !required.iter().any(|required| path.ends_with(required))
}

fn native_tool_call_path_arg(call: &NativeToolCall) -> Option<&str> {
    [
        "path",
        "file_path",
        "filepath",
        "target_path",
        "target",
        "file",
        "absolute_path",
        "full_path",
        "output_path",
        "destination",
        "dest",
        "filename",
    ]
    .iter()
    .find_map(|key| call.args.get(*key).and_then(Value::as_str))
}

fn native_tool_call_content_arg(call: &NativeToolCall) -> Option<&str> {
    ["content", "text", "body"]
        .iter()
        .find_map(|key| call.args.get(*key).and_then(Value::as_str))
}

fn native_tool_call_text_arg<'a>(call: &'a NativeToolCall, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| call.args.get(*key).and_then(Value::as_str))
}

fn native_tool_proposed_python_content_after_call(
    call: &NativeToolCall,
    existing: &str,
) -> Option<String> {
    match call.name.trim().to_ascii_lowercase().as_str() {
        "file_write" | "write_file" | "workspace.write" | "workspace_write" => {
            native_tool_call_content_arg(call).map(str::to_string)
        }
        "file_patch" | "patch_file" | "workspace.patch" | "workspace_patch" => {
            let old = native_tool_call_text_arg(
                call,
                &[
                    "old",
                    "old_text",
                    "search",
                    "search_text",
                    "before",
                    "target_text",
                ],
            )?;
            let new = native_tool_call_text_arg(
                call,
                &[
                    "new",
                    "new_text",
                    "replace",
                    "replace_text",
                    "after",
                    "replacement",
                ],
            )?;
            if old.is_empty() || existing.matches(old).count() != 1 {
                return None;
            }
            Some(existing.replacen(old, new, 1))
        }
        _ => None,
    }
}

fn native_tool_python_existing_shape_blocked_receipt(
    original_prompt: &str,
    call: &NativeToolCall,
) -> Option<NativeToolReceipt> {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    ) {
        return None;
    }
    let lower_prompt = original_prompt.to_ascii_lowercase();
    if native_tool_prompt_mentions_any(
        &lower_prompt,
        &[
            "remove field",
            "delete field",
            "drop field",
            "rename field",
            "remove class",
            "delete class",
            "drop class",
            "breaking change",
        ],
    ) {
        return None;
    }
    let path = native_tool_call_path_arg(call)?;
    if !path.to_ascii_lowercase().ends_with(".py") {
        return None;
    }
    let Ok(existing) = std::fs::read_to_string(path) else {
        return None;
    };
    let before = native_tool_python_annotated_class_fields(&existing);
    if before.is_empty() {
        return None;
    }
    let content = native_tool_proposed_python_content_after_call(call, &existing)?;
    let after = native_tool_python_annotated_class_fields(&content);
    let mut removed_classes = Vec::new();
    let mut removed_fields = Vec::new();
    for (class_name, fields) in before {
        let Some(after_fields) = after.get(&class_name) else {
            removed_classes.push(class_name);
            continue;
        };
        for field in fields {
            if !after_fields.iter().any(|candidate| candidate == &field) {
                removed_fields.push(format!("{}.{}", class_name, field));
            }
        }
    }
    if removed_classes.is_empty() && removed_fields.is_empty() {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "python_existing_shape_guardrail",
            "reason": "python_existing_class_shape_requires_additive_patch",
            "path": path,
            "removed_classes": removed_classes,
            "removed_fields": removed_fields,
            "required_next_tool": "file_patch_or_file_write_that_preserves_existing_python_class_fields"
        }),
        error: Some("python_existing_class_shape_requires_additive_patch".to_string()),
    })
}

fn native_tool_shell_edit_python_shape_snapshots(
    paths: &[PathBuf],
) -> std::collections::BTreeMap<
    String,
    (
        Option<Vec<u8>>,
        std::collections::BTreeMap<String, Vec<String>>,
    ),
> {
    let mut snapshots = std::collections::BTreeMap::new();
    for path in paths {
        let display = path.display().to_string();
        let content = std::fs::read(path).ok();
        let shape = if display.to_ascii_lowercase().ends_with(".py") {
            content
                .as_ref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(native_tool_python_annotated_class_fields)
                .unwrap_or_default()
        } else {
            std::collections::BTreeMap::new()
        };
        snapshots.insert(display, (content, shape));
    }
    snapshots
}

fn native_tool_shell_edit_python_existing_shape_blocked_receipt(
    command_call_id: &str,
    snapshots: &std::collections::BTreeMap<
        String,
        (
            Option<Vec<u8>>,
            std::collections::BTreeMap<String, Vec<String>>,
        ),
    >,
) -> Option<NativeToolReceipt> {
    if snapshots.is_empty() {
        return None;
    }
    let mut removed_classes = Vec::new();
    let mut removed_fields = Vec::new();
    let mut restored_paths = Vec::new();
    let mut deleted_new_paths = Vec::new();
    for (path, (_before_content, before_shape)) in snapshots {
        if before_shape.is_empty() {
            continue;
        }
        let after_content = std::fs::read_to_string(path).unwrap_or_default();
        let after_shape = native_tool_python_annotated_class_fields(&after_content);
        for (class_name, fields) in before_shape {
            let Some(after_fields) = after_shape.get(class_name) else {
                removed_classes.push(format!("{path}:{class_name}"));
                continue;
            };
            for field in fields {
                if !after_fields.iter().any(|candidate| candidate == field) {
                    removed_fields.push(format!("{path}:{class_name}.{field}"));
                }
            }
        }
    }
    if removed_classes.is_empty() && removed_fields.is_empty() {
        return None;
    }
    for (path, (before_content, _before_shape)) in snapshots {
        match before_content {
            Some(bytes) => {
                let _ = std::fs::write(path, bytes);
                restored_paths.push(path.clone());
            }
            None => {
                if Path::new(path).exists() {
                    let _ = std::fs::remove_file(path);
                    deleted_new_paths.push(path.clone());
                }
            }
        }
    }
    Some(NativeToolReceipt {
        call_id: format!("{command_call_id}_python_shape_guard"),
        tool_name: "command_run".to_string(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "shell_edit_python_existing_shape_guardrail",
            "reason": "shell_edit_existing_python_class_shape_requires_additive_patch",
            "removed_classes": removed_classes,
            "removed_fields": removed_fields,
            "restored_paths": restored_paths,
            "deleted_new_paths": deleted_new_paths,
            "required_next_tool": "file_patch_or_controlled_shell_edit_batch_that_preserves_existing_python_class_fields"
        }),
        error: Some("shell_edit_existing_python_class_shape_requires_additive_patch".to_string()),
    })
}

fn native_tool_shell_edit_preserved_api_blocked_receipt(
    command_call_id: &str,
    original_prompt: &str,
    snapshots: &std::collections::BTreeMap<
        String,
        (
            Option<Vec<u8>>,
            std::collections::BTreeMap<String, Vec<String>>,
        ),
    >,
) -> Option<NativeToolReceipt> {
    let names = native_tool_prompt_preserved_api_names(original_prompt);
    if names.is_empty() || snapshots.is_empty() {
        return None;
    }
    let mut changed = Vec::new();
    let mut restored_paths = Vec::new();
    for (path, (before_content, _before_shape)) in snapshots {
        let Some(before_bytes) = before_content else {
            continue;
        };
        let Ok(before) = std::str::from_utf8(before_bytes) else {
            continue;
        };
        let Ok(after) = std::fs::read_to_string(path) else {
            continue;
        };
        for name in &names {
            let Some(before_block) = native_tool_python_api_block(before, name) else {
                continue;
            };
            let Some(after_block) = native_tool_python_api_block(&after, name) else {
                changed.push(format!("{path}:{name}"));
                continue;
            };
            if native_tool_preserved_api_behavior_signature(&before_block)
                != native_tool_preserved_api_behavior_signature(&after_block)
            {
                changed.push(format!("{path}:{name}"));
            }
        }
    }
    if changed.is_empty() {
        return None;
    }
    let changed_owner_path = changed
        .iter()
        .find_map(|entry| entry.rsplit_once(':').map(|(path, _name)| path.to_string()));
    for (path, (before_content, _before_shape)) in snapshots {
        if let Some(bytes) = before_content {
            let _ = std::fs::write(path, bytes);
            restored_paths.push(path.clone());
        }
    }
    Some(NativeToolReceipt {
        call_id: format!("{command_call_id}_preserved_api_guard"),
        tool_name: "command_run".to_string(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "shell_edit_preserved_api_guardrail",
            "reason": "shell_edit_existing_source_requires_additive_patch",
            "changed_public_api_blocks": changed,
            "path": changed_owner_path,
            "restored_paths": restored_paths,
            "required_next_tool": "file_patch_or_file_write_that_preserves_existing_api_blocks_and_adds_missing_imported_symbols"
        }),
        error: Some("shell_edit_existing_source_requires_additive_patch".to_string()),
    })
}

fn native_tool_python_annotated_class_fields(
    text: &str,
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut result = std::collections::BTreeMap::new();
    let mut current_name: Option<String> = None;
    let mut current_indent = 0usize;
    let mut current_fields: Vec<String> = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());

        if let Some(class_name) = native_tool_python_class_name(trimmed) {
            if let Some(previous_name) = current_name.take() {
                if !current_fields.is_empty() {
                    result.insert(previous_name, std::mem::take(&mut current_fields));
                } else {
                    current_fields.clear();
                }
            }
            current_name = Some(class_name);
            current_indent = indent;
            continue;
        }

        if current_name.is_none() {
            continue;
        }
        if !trimmed.is_empty() && indent <= current_indent {
            if let Some(previous_name) = current_name.take() {
                if !current_fields.is_empty() {
                    result.insert(previous_name, std::mem::take(&mut current_fields));
                } else {
                    current_fields.clear();
                }
            }
            continue;
        }
        if let Some(field_name) = native_tool_python_annotated_field_name(trimmed) {
            if !current_fields.iter().any(|field| field == &field_name) {
                current_fields.push(field_name);
            }
        }
    }

    if let Some(previous_name) = current_name {
        if !current_fields.is_empty() {
            result.insert(previous_name, current_fields);
        }
    }
    result
}

fn native_tool_python_class_name(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("class ")?;
    let name = rest
        .split(|ch: char| ch == '(' || ch == ':' || ch.is_whitespace())
        .next()
        .unwrap_or("")
        .trim();
    if native_tool_python_identifier(name) {
        Some(name.to_string())
    } else {
        None
    }
}

fn native_tool_python_annotated_field_name(trimmed: &str) -> Option<String> {
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with('@')
        || trimmed.starts_with("def ")
        || trimmed.starts_with("async def ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("with ")
    {
        return None;
    }
    let colon = trimmed.find(':')?;
    let equals = trimmed.find('=').unwrap_or(usize::MAX);
    let open_paren = trimmed.find('(').unwrap_or(usize::MAX);
    if colon > equals || colon > open_paren {
        return None;
    }
    let name = trimmed[..colon].trim();
    if name.contains('.') || name.contains(' ') || name.contains('\t') {
        return None;
    }
    if native_tool_python_identifier(name) {
        Some(name.to_string())
    } else {
        None
    }
}

fn native_tool_python_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn native_tool_preserved_api_write_blocked_receipt(
    original_prompt: &str,
    call: &NativeToolCall,
) -> Option<NativeToolReceipt> {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    ) {
        return None;
    }
    let path = native_tool_call_path_arg(call)?;
    if !path.to_ascii_lowercase().ends_with(".py") {
        return None;
    }
    let names = native_tool_prompt_preserved_api_names(original_prompt);
    if names.is_empty() {
        return None;
    }
    let Ok(existing) = std::fs::read_to_string(path) else {
        return None;
    };
    let content = native_tool_proposed_python_content_after_call(call, &existing)?;
    let mut changed = Vec::new();
    for name in names {
        let Some(before) = native_tool_python_api_block(&existing, &name) else {
            continue;
        };
        let Some(after) = native_tool_python_api_block(&content, &name) else {
            changed.push(name);
            continue;
        };
        if native_tool_preserved_api_behavior_signature(&before)
            != native_tool_preserved_api_behavior_signature(&after)
        {
            changed.push(name);
        }
    }
    if changed.is_empty() {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "preserved_api_guardrail",
            "reason": "preserved_api_existing_source_requires_additive_patch",
            "path": path,
            "preserved_api_names": changed,
            "required_next_tool": "file_patch_or_file_write_that_preserves_existing_api_blocks_and_adds_missing_imported_symbols"
        }),
        error: Some("preserved_api_existing_source_requires_additive_patch".to_string()),
    })
}

fn native_tool_preserved_api_behavior_signature(block: &str) -> String {
    let signature = block
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("return ")
                || *line == "return"
                || line.starts_with("raise ")
                || *line == "raise"
                || line.starts_with("yield ")
                || *line == "yield"
        })
        .collect::<Vec<_>>();
    if signature.is_empty() {
        block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        signature.join("\n")
    }
}

fn native_tool_python_src_prefix_import_blocked_receipt(
    original_prompt: &str,
    call: &NativeToolCall,
) -> Option<NativeToolReceipt> {
    if !original_prompt.contains("PYTHONPATH=src") {
        return None;
    }
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write" | "file_patch"
    ) {
        return None;
    }
    let path = native_tool_call_path_arg(call)?;
    let lower_path = path.replace('\\', "/").to_ascii_lowercase();
    if !lower_path.contains("/tests/") || !lower_path.ends_with(".py") {
        return None;
    }
    let content = native_tool_call_content_arg(call)
        .or_else(|| call.args.get("new").and_then(Value::as_str))
        .unwrap_or_default();
    if !(content.contains("from src.") || content.contains("import src.")) {
        return None;
    }
    Some(NativeToolReceipt {
        call_id: call.id.clone(),
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: json!({
            "blocked_by": "python_import_guardrail",
            "reason": "pythonpath_src_tests_must_not_import_src_prefix",
            "path": path,
            "required_next_tool": "file_write_or_file_patch_with_package_import_without_src_prefix"
        }),
        error: Some("pythonpath_src_tests_must_not_import_src_prefix".to_string()),
    })
}

fn native_tool_prompt_preserved_api_names(original_prompt: &str) -> Vec<String> {
    let mut names = Vec::<String>::new();
    for segment in original_prompt.split(['.', ';', '\n']) {
        let lower = segment.to_ascii_lowercase();
        let Some(start) = lower.find("preserve ") else {
            continue;
        };
        let preserved = &segment[start + "preserve ".len()..];
        for token in preserved.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let lower_token = token.to_ascii_lowercase();
            if matches!(
                lower_token.as_str(),
                "and"
                    | "api"
                    | "behavior"
                    | "behaviour"
                    | "current"
                    | "existing"
                    | "public"
                    | "the"
            ) {
                continue;
            }
            if !names.iter().any(|existing| existing == token) {
                names.push(token.to_string());
            }
            break;
        }
    }
    names
}

fn native_tool_python_api_block(text: &str, name: &str) -> Option<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let lower_name = name.to_ascii_lowercase();
    let mut start = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start().to_ascii_lowercase();
        if trimmed.starts_with(&format!("def {lower_name}("))
            || trimmed.starts_with(&format!("async def {lower_name}("))
            || trimmed.starts_with(&format!("class {lower_name}("))
            || trimmed == format!("class {lower_name}:")
            || trimmed.starts_with(&format!("class {lower_name}:"))
        {
            start = Some(idx);
            break;
        }
    }
    let start = start?;
    let base_indent = lines[start].len() - lines[start].trim_start().len();
    let mut end = lines.len();
    for (idx, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start().to_ascii_lowercase();
        if indent <= base_indent
            && (trimmed.starts_with("def ")
                || trimmed.starts_with("async def ")
                || trimmed.starts_with("class "))
        {
            end = idx;
            break;
        }
    }
    Some(lines[start..end].join("\n"))
}

fn native_tool_unrelated_repair_path_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: Value::Null,
        error: Some(
            "repair_write_blocked_until_missing_prompt_derived_paths_are_mutated".to_string(),
        ),
    }
}

fn native_tool_receipt_error_summary(receipts: &[NativeToolReceipt]) -> String {
    let summary = receipts
        .iter()
        .map(|receipt| {
            let result = &receipt.result;
            json!({
                "id": receipt.call_id,
                "tool": receipt.tool_name,
                "status": receipt.status,
                "path": result.get("path").or_else(|| result.get("cwd")).or_else(|| result.get("paths")).cloned().unwrap_or(Value::Null),
                "args_keys": result.get("args_keys").cloned().unwrap_or(Value::Null),
                "command": result.get("command").or_else(|| result.get("cmd")).cloned().unwrap_or(Value::Null),
                "error": receipt.error.clone().map(Value::String).unwrap_or(Value::Null),
                "success": result.get("success").cloned().unwrap_or(Value::Null),
                "exit_code": result.get("exit_code").cloned().unwrap_or(Value::Null),
                "stdout": result.get("stdout").and_then(Value::as_str).map(native_tool_compact_text).map(Value::String).unwrap_or(Value::Null),
                "stderr": result.get("stderr").and_then(Value::as_str).map(native_tool_compact_text).map(Value::String).unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&summary).unwrap_or_else(|_| "[]".to_string())
}

fn native_tool_compact_text(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= 240 {
        compact
    } else {
        format!("{}...", compact.chars().take(240).collect::<String>())
    }
}

fn native_tool_prioritize_repair_calls(calls: &mut [NativeToolCall], repair_reasons: &[String]) {
    let owner_source_paths = native_tool_unresolved_owner_source_paths(repair_reasons);
    let needs_test_change = repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt");
    let needs_missing_path = repair_reasons
        .iter()
        .any(|reason| {
            reason.starts_with("missing_changed_path:")
                || reason.starts_with("missing_imported_module:")
                || reason.starts_with("unresolved_owner_source_path:")
        });
    let needs_product_work = native_tool_repair_reasons_require_product_work(repair_reasons);
    let needs_validation_repair = native_tool_repair_reasons_include_validation(repair_reasons);
    if !(needs_test_change
        || needs_missing_path
        || needs_product_work
        || needs_validation_repair
        || !owner_source_paths.is_empty())
    {
        return;
    }
    calls.sort_by_key(|call| {
        if native_tool_call_targets_required_owner_source_path(call, &owner_source_paths) {
            0
        } else if !owner_source_paths.is_empty() && native_tool_call_is_command_run(call) {
            4
        } else if !owner_source_paths.is_empty() && native_tool_call_is_mutation(call) {
            1
        } else if needs_test_change && !needs_missing_path && native_tool_call_is_test_mutation(call) {
            0
        } else if native_tool_call_is_mutation(call) {
            1
        } else if native_tool_call_is_command_run(call) {
            2
        } else {
            3
        }
    });
}

fn native_tool_completion_repair_tools(tools: &[String], repair_reasons: &[String]) -> Vec<String> {
    if repair_reasons
        .iter()
        .any(|reason| reason.starts_with("unresolved_owner_source_path:"))
    {
        let filtered = tools
            .iter()
            .filter(|tool| {
                matches!(
                    tool.trim().to_ascii_lowercase().as_str(),
                    "file_write"
                        | "write_file"
                        | "workspace.write"
                        | "workspace_write"
                        | "file_patch"
                        | "patch_file"
                        | "workspace.patch"
                        | "workspace_patch"
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        return if filtered.is_empty() {
            tools.to_vec()
        } else {
            filtered
        };
    }
    let missing_test_only = repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
        && !repair_reasons
            .iter()
            .any(|reason| {
                reason.starts_with("missing_changed_path:")
                    || reason.starts_with("missing_imported_module:")
            })
        && !native_tool_repair_reasons_include_validation(repair_reasons)
        && !native_tool_repair_reasons_include_product_mutation(repair_reasons)
        && !native_tool_repair_reasons_include_product_slice(repair_reasons);
    if !missing_test_only {
        return tools.to_vec();
    }
    let filtered = tools
        .iter()
        .filter(|tool| {
            let lower = tool.to_ascii_lowercase();
            lower.contains("file_write") || lower.contains("file_patch")
        })
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        tools.to_vec()
    } else {
        filtered
    }
}

fn native_tool_unresolved_owner_source_paths(repair_reasons: &[String]) -> Vec<String> {
    repair_reasons
        .iter()
        .filter_map(|reason| {
            reason
                .strip_prefix("unresolved_owner_source_path:")
                .map(str::to_string)
        })
        .collect()
}

fn native_tool_call_targets_required_owner_source_path(
    call: &NativeToolCall,
    owner_source_paths: &[String],
) -> bool {
    if owner_source_paths.is_empty() {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call) else {
        return false;
    };
    owner_source_paths
        .iter()
        .any(|owner_path| native_tool_paths_same_or_suffix(owner_path, &path))
}

fn native_tool_completion_evidence_repair_loop(
    provider: &Arc<dyn crate::provider::ProviderClient>,
    dispatcher: &NativeToolDispatcher,
    tools: &[String],
    model: Option<String>,
    metadata: &Value,
    original_prompt: &str,
    system: &str,
    mut response: ProviderResponse,
    mut receipts: Vec<NativeToolReceipt>,
    mut provider_call_count: u64,
    mut repair_reasons: Vec<String>,
) -> Result<(ProviderResponse, Vec<NativeToolReceipt>, u64), ProviderError> {
    let max_turns = native_tool_completion_evidence_repair_max_turns(metadata);
    let mut prompt = if native_tool_checkpointed_project_operator_tool_loop_active(
        metadata,
        original_prompt,
    ) {
        if let Some(stage) = native_tool_current_live_stage(original_prompt, &receipts) {
            native_tool_checkpointed_project_stage_prompt(
                metadata,
                original_prompt,
                &receipts,
                stage,
                &format!(
                    "Initial repair reasons:\n{}\n\nLatest assistant output:\n{}",
                    repair_reasons.join("\n"),
                    response.output
                ),
                0,
            )
        } else {
            native_tool_completion_evidence_repair_prompt(
                metadata,
                original_prompt,
                &response.output,
                &receipts,
                &repair_reasons,
            )
        }
    } else {
        native_tool_completion_evidence_repair_prompt(
            metadata,
            original_prompt,
            &response.output,
            &receipts,
            &repair_reasons,
        )
    };
    for turn_idx in 0..max_turns {
        provider_call_count += 1;
        let product_mutation_missing =
            native_tool_repair_reasons_include_product_mutation(&repair_reasons);
        let product_slice_incomplete =
            native_tool_repair_reasons_include_product_slice(&repair_reasons);
        let test_change_missing = repair_reasons
            .iter()
            .any(|reason| reason == "missing_test_change_receipt");
        let repair_tools = native_tool_completion_repair_tools(tools, &repair_reasons);
        let request = ProviderRequest {
            prompt: prompt.clone(),
            system: Some(system.to_string()),
            tools: repair_tools,
            model: model.clone(),
            metadata: if product_mutation_missing || product_slice_incomplete {
                native_tool_recovery_timeout_metadata(metadata)
            } else {
                metadata.clone()
            },
        };
        let next_response = match provider.complete(&request) {
            Ok(response) => response,
            Err(error) if native_tool_provider_error_is_timeout(&error) => {
                let completed_with_receipts = native_tool_has_successful_mutation(&receipts)
                    && native_tool_has_successful_validation_after_latest_mutation(&receipts)
                    && !native_tool_checkpointed_project_has_live_stage(
                        metadata,
                        original_prompt,
                        &receipts,
                    )
                    && native_tool_runtime_repair_reasons(
                        metadata,
                        original_prompt,
                        &response.output,
                        &receipts,
                    )
                    .is_empty();
                let response = if completed_with_receipts {
                    native_tool_synthetic_completion_evidence_response(
                        &response,
                        metadata,
                        original_prompt,
                        &receipts,
                        error.message.as_str(),
                    )
                } else {
                    native_tool_partial_progress_response(
                        provider.provider_id(),
                        model.as_deref(),
                        error.message.as_str(),
                        provider_call_count,
                        &receipts,
                    )
                };
                return Ok((response, receipts, provider_call_count));
            }
            Err(error) => return Err(error),
        };
        let mut calls = parse_native_tool_calls(&next_response.output);
        if calls.is_empty() {
            response = next_response;
            repair_reasons = native_tool_runtime_repair_reasons(
                metadata,
                original_prompt,
                &response.output,
                &receipts,
            );
            if repair_reasons.is_empty() {
                break;
            }
            if product_mutation_missing || product_slice_incomplete {
                break;
            }
            prompt = if native_tool_checkpointed_project_operator_tool_loop_active(
                metadata,
                original_prompt,
            ) {
                if let Some(stage) = native_tool_current_live_stage(original_prompt, &receipts) {
                    native_tool_checkpointed_project_stage_prompt(
                        metadata,
                        original_prompt,
                        &receipts,
                        stage,
                        &format!(
                            "No tool calls were emitted. Remaining repair reasons:\n{}",
                            repair_reasons.join("\n")
                        ),
                        native_tool_turn_number_usize(turn_idx),
                    )
                } else {
                    native_tool_completion_evidence_repair_prompt(
                        metadata,
                        original_prompt,
                        &response.output,
                        &receipts,
                        &repair_reasons,
                    )
                }
            } else {
                native_tool_completion_evidence_repair_prompt(
                    metadata,
                    original_prompt,
                    &response.output,
                    &receipts,
                    &repair_reasons,
                )
            };
            continue;
        }
        native_tool_prioritize_repair_calls(&mut calls, &repair_reasons);
        let mut turn_receipts = Vec::new();
        let product_work_required =
            native_tool_repair_reasons_require_product_work(&repair_reasons);
        let has_successful_mutation = native_tool_has_successful_mutation(&receipts);
        let has_successful_test_mutation = native_tool_has_successful_test_mutation(&receipts);
        let has_exhausted_repair_read_budget = native_tool_successful_read_context_count(&receipts)
            >= native_tool_pre_mutation_read_budget(metadata);
        let has_exhausted_post_mutation_test_read_budget =
            native_tool_successful_read_context_count_after_first_mutation(&receipts) >= 4;
        let has_unmutated_required_repair_path =
            native_tool_has_unmutated_required_repair_path(&repair_reasons, &receipts);
        for call in calls
            .into_iter()
            .take(native_tool_max_calls_per_turn(metadata))
        {
            let call = native_tool_call_with_prompt_defaults(call, original_prompt);
            let shell_edit_paths =
                native_tool_controlled_shell_edit_batch_paths(metadata, original_prompt, &call);
            let shell_edit_command = product_work_required && !shell_edit_paths.is_empty();
            let shell_edit_receipt_paths = if shell_edit_paths.is_empty() {
                native_tool_shell_edit_receipt_synthesis_paths(original_prompt, &call)
            } else {
                shell_edit_paths.clone()
            };
            let shell_edit_receipt_command = product_work_required
                && !shell_edit_receipt_paths.is_empty();
            let unresolved_owner_source_blocked =
                native_tool_unresolved_owner_source_first_blocked_receipt(
                    &receipts,
                    &call,
                    &shell_edit_receipt_paths,
                );
            let pre_dispatch_receipts = if shell_edit_receipt_command {
                Some(receipts.clone())
            } else {
                None
            };
            let shell_edit_python_shape_snapshots = if shell_edit_receipt_command {
                native_tool_shell_edit_python_shape_snapshots(&shell_edit_receipt_paths)
            } else {
                std::collections::BTreeMap::new()
            };
            let receipt = if let Some(blocked) = unresolved_owner_source_blocked {
                blocked
            } else if shell_edit_command {
                if let Some(blocked) =
                    native_tool_shell_edit_batch_blocked_receipt(&call, &shell_edit_paths)
                {
                    blocked
                } else {
                    dispatcher.dispatch(call)
                }
            } else if let Some(blocked) = native_tool_staged_repair_blocked_receipt(
                metadata,
                original_prompt,
                &repair_reasons,
                &receipts,
                &call,
            ) {
                blocked
            } else if product_work_required
                && has_exhausted_repair_read_budget
                && ((!has_successful_mutation)
                    || (has_successful_mutation
                        && (((test_change_missing && !has_successful_test_mutation)
                            && has_exhausted_post_mutation_test_read_budget)
                            || has_unmutated_required_repair_path)))
                && native_tool_call_is_context_only(&call)
            {
                native_tool_product_repair_context_blocked_receipt(call, &repair_reasons, &receipts)
            } else if (product_mutation_missing || product_slice_incomplete || test_change_missing)
                && native_tool_call_targets_handoff_artifact(&call)
            {
                native_tool_handoff_blocked_repair_receipt(call)
            } else if product_work_required
                && native_tool_call_targets_unrelated_repair_path(
                    &call,
                    &repair_reasons,
                    original_prompt,
                )
            {
                native_tool_unrelated_repair_path_receipt(call)
            } else if let Some(blocked) =
                native_tool_preserved_api_write_blocked_receipt(original_prompt, &call)
            {
                blocked
            } else if let Some(blocked) =
                native_tool_python_existing_shape_blocked_receipt(original_prompt, &call)
            {
                blocked
            } else if let Some(blocked) =
                native_tool_atomic_dependent_edit_blocked_receipt(&receipts, &call)
            {
                blocked
            } else if let Some(blocked) =
                native_tool_python_src_prefix_import_blocked_receipt(original_prompt, &call)
            {
                blocked
            } else {
                dispatcher.dispatch(call)
            };
            turn_receipts.push(receipt.clone());
            receipts.push(receipt.clone());
            if shell_edit_receipt_command && native_tool_command_receipt_success(&receipt) {
                if let Some(blocked) = native_tool_shell_edit_python_existing_shape_blocked_receipt(
                    &receipt.call_id,
                    &shell_edit_python_shape_snapshots,
                ) {
                    turn_receipts.push(blocked.clone());
                    receipts.push(blocked);
                    continue;
                }
                if let Some(blocked) = native_tool_shell_edit_preserved_api_blocked_receipt(
                    &receipt.call_id,
                    original_prompt,
                    &shell_edit_python_shape_snapshots,
                ) {
                    turn_receipts.push(blocked.clone());
                    receipts.push(blocked);
                    continue;
                }
                let synthetic_receipts = native_tool_synthesize_shell_edit_mutation_receipts(
                    &receipt.call_id,
                    &shell_edit_receipt_paths,
                    pre_dispatch_receipts.as_deref().unwrap_or(&[]),
                );
                for synthetic in synthetic_receipts {
                    turn_receipts.push(synthetic.clone());
                    receipts.push(synthetic);
                }
            }
        }
        response = next_response;
        repair_reasons = native_tool_runtime_repair_reasons(
            metadata,
            original_prompt,
            &response.output,
            &receipts,
        );
        if product_mutation_missing && !native_tool_has_successful_mutation(&receipts) {
            break;
        }
        if repair_reasons.is_empty() {
            break;
        }
        let observation = native_tool_observation_prompt(&turn_receipts);
        let failed_validation_details = native_tool_failed_validation_receipt_details(&receipts);
        let repair_actions =
            native_tool_completion_repair_action_brief(metadata, original_prompt, &repair_reasons);
        let test_change_repair_hint =
            native_tool_missing_test_change_repair_hint(&receipts, &repair_reasons);
        let failed_validation_repair_hint = native_tool_failed_validation_repair_hint(&receipts);
        let repair_turn_rule = native_tool_orchestration_prompt_text(
            metadata,
            "completion_evidence_repair_turn_rule",
            "Continue repairing only the remaining uncovered requirements from this native tool task. Return JSON tool calls, or return a structured blocker only when local completion is genuinely blocked.",
        );
        prompt = if native_tool_checkpointed_project_operator_tool_loop_active(
            metadata,
            original_prompt,
        ) {
            if let Some(stage) = native_tool_current_live_stage(original_prompt, &receipts) {
                native_tool_checkpointed_project_stage_prompt(
                    metadata,
                    original_prompt,
                    &receipts,
                    stage,
                    &format!(
                        "Repair turn {} observations:\n{}\n\nRemaining uncovered requirements:\n{}\n\nRequired repair actions:\n{}\n\nTest mutation repair hint:\n{}\n\nFailed validation repair hint:\n{}",
                        turn_idx + 1,
                        observation,
                        repair_reasons.join("\n"),
                        repair_actions,
                        test_change_repair_hint,
                        failed_validation_repair_hint
                    ),
                    native_tool_turn_number_usize(turn_idx),
                )
            } else {
                format!(
                    "{}\n\nRepair turn {} produced observations:\n{}\n\nFailed validation receipt details:\n{}\n\nRemaining uncovered requirements:\n{}\n\nRequired repair actions:\n{}\n\nTest mutation repair hint:\n{}\n\nFailed validation repair hint:\n{}\n\n{}",
                    original_prompt,
                    turn_idx + 1,
                    observation,
                    failed_validation_details,
                    repair_reasons.join("\n"),
                    repair_actions,
                    test_change_repair_hint,
                    failed_validation_repair_hint,
                    repair_turn_rule
                )
            }
        } else {
            format!(
                "{}\n\nRepair turn {} produced observations:\n{}\n\nFailed validation receipt details:\n{}\n\nRemaining uncovered requirements:\n{}\n\nRequired repair actions:\n{}\n\nTest mutation repair hint:\n{}\n\nFailed validation repair hint:\n{}\n\n{}",
                original_prompt,
                turn_idx + 1,
                observation,
                failed_validation_details,
                repair_reasons.join("\n"),
                repair_actions,
                test_change_repair_hint,
                failed_validation_repair_hint,
                repair_turn_rule
            )
        };
    }
    Ok((response, receipts, provider_call_count))
}

fn native_tool_completion_evidence_timeout_synthesis_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("synthesize_completion_evidence_on_finalization_timeout"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_synthesize_final_after_successful_validation(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("synthesize_final_after_successful_validation"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_recovery_timeout_metadata(metadata: &Value) -> Value {
    let mut out = metadata.clone();
    let timeout_seconds = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("recovery_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(120);
    if let Some(object) = out.as_object_mut() {
        object.insert(
            "provider_timeout_seconds".to_string(),
            json!(timeout_seconds),
        );
    }
    out
}

fn native_tool_bootstrap_context_before_first_provider(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bootstrap_context_before_first_provider"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_bootstrap_discovery_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
) -> Option<NativeToolReceipt> {
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let receipt = dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_bootstrap_file_list".to_string(),
        name: "file_list".to_string(),
        args: json!({
            "path": project_root,
            "recursive": false,
            "max_entries": 200
        }),
    });
    if receipt.status == "ok" {
        Some(receipt)
    } else {
        None
    }
}

fn native_tool_bootstrap_context_receipts(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
) -> Vec<NativeToolReceipt> {
    let Some(project_root) = native_tool_prompt_project_root(original_prompt) else {
        return Vec::new();
    };
    let mut receipts = Vec::new();
    let list_receipt = dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_bootstrap_file_list".to_string(),
        name: "file_list".to_string(),
        args: json!({
            "path": project_root,
            "recursive": true,
            "max_depth": 3,
            "max_entries": 200
        }),
    });
    receipts.push(list_receipt);
    let root = std::path::PathBuf::from(&project_root);
    let mut paths = native_tool_unique_code_path_mentions(original_prompt)
        .into_iter()
        .filter_map(|path| {
            let candidate = if path.starts_with('/') {
                std::path::PathBuf::from(path)
            } else {
                root.join(path.trim_start_matches("./"))
            };
            if candidate.is_file() {
                Some(candidate.display().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    paths.extend(native_tool_bootstrap_likely_context_paths(&root, &paths));
    paths.sort();
    paths.dedup();
    if !paths.is_empty() {
        receipts.push(dispatcher.dispatch(crate::native_tools::NativeToolCall {
            id: "runtime_bootstrap_file_read_many".to_string(),
            name: "file_read_many".to_string(),
            args: json!({ "paths": paths }),
        }));
    }
    receipts
        .into_iter()
        .filter(|receipt| receipt.status == "ok")
        .collect()
}

fn native_tool_bootstrap_likely_context_paths(
    root: &Path,
    existing_paths: &[String],
) -> Vec<String> {
    let mut candidates = Vec::<PathBuf>::new();
    native_tool_collect_bootstrap_context_candidates(root, 0, &mut candidates);
    let max_project_candidates = 24usize;
    if candidates.is_empty() || candidates.len() > max_project_candidates {
        return Vec::new();
    }

    candidates.sort_by_key(|path| {
        let normalized = path
            .display()
            .to_string()
            .replace('\\', "/")
            .to_ascii_lowercase();
        let is_test = native_tool_bootstrap_path_looks_like_test(&normalized);
        let depth = path
            .strip_prefix(root)
            .ok()
            .map(|relative| relative.components().count())
            .unwrap_or(usize::MAX);
        (is_test, depth, normalized)
    });

    let mut selected = Vec::<String>::new();
    for path in candidates {
        let path = path.display().to_string();
        if existing_paths.iter().any(|existing| existing == &path)
            || selected.iter().any(|existing| existing == &path)
        {
            continue;
        }
        selected.push(path);
        if selected.len() >= 8 {
            break;
        }
    }
    selected
}

fn native_tool_collect_bootstrap_context_candidates(
    dir: &Path,
    depth: usize,
    candidates: &mut Vec<PathBuf>,
) {
    if depth > 3 || candidates.len() > 24 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if native_tool_bootstrap_path_ignored(&name) {
            continue;
        }
        if path.is_dir() {
            native_tool_collect_bootstrap_context_candidates(&path, depth + 1, candidates);
            continue;
        }
        if native_tool_bootstrap_file_candidate(&path) {
            candidates.push(path);
        }
        if candidates.len() > 24 {
            return;
        }
    }
}

fn native_tool_bootstrap_file_candidate(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "py" | "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "rb" | "php" | "swift" | "kt"
    )
}

fn native_tool_bootstrap_path_ignored(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".venv"
            | "venv"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | "__pycache__"
            | ".pytest_cache"
    )
}

fn native_tool_bootstrap_path_looks_like_test(path: &str) -> bool {
    path.contains("/tests/")
        || path.contains("/test/")
        || path.contains("test_")
        || path.ends_with("_test.py")
        || path.ends_with(".test.js")
        || path.ends_with(".spec.js")
        || path.ends_with(".test.ts")
        || path.ends_with(".spec.ts")
}

fn native_tool_auto_validation_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Option<NativeToolReceipt> {
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if !native_tool_prompt_requires_validation_command(&prompt_lower)
        || native_tool_has_successful_validation_after_latest_mutation(receipts)
    {
        return None;
    }
    if native_tool_prompt_requires_product_mutation(&prompt_lower)
        && !native_tool_has_successful_mutation(receipts)
    {
        return None;
    }
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let project_root_path = std::path::PathBuf::from(&project_root);
    let semantic_probe_path = project_root_path.join(".infring").join("semantic_probe.py");
    let semantic_probe_suffix = if semantic_probe_path.is_file() {
        Some("PYTHONPATH=src:. python3 .infring/semantic_probe.py")
    } else {
        None
    };
    let cmd = if let Some(command) = native_tool_prompt_validation_shell_command(original_prompt) {
        let command = if let Some(probe) = semantic_probe_suffix {
            format!("({command}) && {probe}")
        } else {
            command
        };
        vec!["sh".to_string(), "-c".to_string(), command]
    } else if prompt_lower.contains("pytest")
        || (project_root_path.join("pyproject.toml").exists()
            && project_root_path.join("tests").is_dir())
    {
        if let Some(probe) = semantic_probe_suffix {
            vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("PYTHONPATH=src:. python3 -m pytest -q && {probe}"),
            ]
        } else {
            vec![
                "python3".to_string(),
                "-m".to_string(),
                "pytest".to_string(),
                "-q".to_string(),
            ]
        }
    } else if prompt_lower.contains("unittest")
        || (project_root_path.join("src").is_dir() && project_root_path.join("tests").is_dir())
    {
        let command = if let Some(probe) = semantic_probe_suffix {
            format!("PYTHONPATH=src:. python3 -m unittest discover -s tests && {probe}")
        } else {
            "PYTHONPATH=src python3 -m unittest discover -s tests".to_string()
        };
        vec!["sh".to_string(), "-c".to_string(), command]
    } else {
        return None;
    };
    Some(dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_auto_validation_command".to_string(),
        name: "command_run".to_string(),
        args: json!({
            "cwd": project_root,
            "cmd": cmd,
            "timeout_seconds": 120,
            "max_output_bytes": 12000
        }),
    }))
}

fn native_tool_pre_mutation_validation_bootstrap_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Option<NativeToolReceipt> {
    if !native_tool_prompt_requires_pre_mutation_validation(original_prompt)
        || native_tool_has_any_validation_command(receipts)
    {
        return None;
    }
    native_tool_pre_mutation_validation_receipt(
        dispatcher,
        original_prompt,
        "runtime_bootstrap_pre_mutation_validation_command",
    )
}

fn native_tool_pre_mutation_validation_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    call_id: &str,
) -> Option<NativeToolReceipt> {
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let command = native_tool_prompt_validation_shell_command(original_prompt)?;
    Some(dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: call_id.to_string(),
        name: "command_run".to_string(),
        args: json!({
            "cwd": project_root,
            "cmd": ["sh", "-c", command],
            "timeout_seconds": 120,
            "max_output_bytes": 12000
        }),
    }))
}

fn native_tool_prompt_validation_shell_command(original_prompt: &str) -> Option<String> {
    let lower = original_prompt.to_ascii_lowercase();
    let markers = [
        "first run this validation command from project root to observe the failure:",
        "first run this validation command from project root:",
        "run this validation command from project root to observe the failure:",
        "validation command from project root:",
        "validation command before final response:",
        "run this validation command before final response:",
        "run this validation command from project root:",
    ];
    for marker in markers {
        let Some(start) = lower.find(marker) else {
            continue;
        };
        let raw = &original_prompt[start + marker.len()..];
        let mut command = raw
            .split('\n')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('`')
            .trim()
            .to_string();
        for delimiter in [
            ". Run this semantic",
            ". Run the semantic",
            ". Then inspect",
            ". Then run",
            ". Do not",
        ] {
            if let Some((head, _tail)) = command.split_once(delimiter) {
                command = head.trim().to_string();
            }
        }
        if command.ends_with('.') {
            command.pop();
        }
        let command_lower = command.to_ascii_lowercase();
        if command.is_empty()
            || !(command_lower.contains("python")
                || command_lower.contains("pytest")
                || command_lower.contains("cargo test")
                || command_lower.contains("npm test")
                || command_lower.contains("pnpm test")
                || command_lower.contains("yarn test"))
        {
            continue;
        }
        return Some(command);
    }
    None
}

fn native_tool_recovery_max_turns(metadata: &Value) -> u64 {
    metadata
        .pointer("/native_success_criteria/partial_recovery_max_turns")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/partial_recovery_max_turns")
                .and_then(Value::as_u64)
        })
        .unwrap_or(3)
        .clamp(1, 6)
}

fn native_tool_recovery_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .pointer("/native_success_criteria/recovery_provider_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/recovery_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .unwrap_or(120)
        .clamp(1, 600)
}

fn sanitize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.chars().take(max_len) {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            continue;
        }
        out.push(ch);
    }
    out.trim().to_string()
}
