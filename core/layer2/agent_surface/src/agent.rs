// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::capability_pack::CapabilityPackCatalog;
use crate::native_evidence::{
    native_tool_artifact_contract_enabled, native_tool_artifact_repair_reasons,
    native_tool_changed_paths, native_tool_changed_paths_include, native_tool_coding_task_lane,
    native_tool_context_only_turn, native_tool_failed_validation_command_refs,
    native_tool_failed_validation_receipt_details, native_tool_has_successful_memory_write_command,
    native_tool_has_successful_mutation, native_tool_has_successful_validation_command,
    native_tool_is_probable_micro_direct_write_task, native_tool_needs_artifact_finalization,
    native_tool_needs_public_report_finalization, native_tool_prompt_evidence_gaps,
    native_tool_prompt_expected_memory_row_id, native_tool_prompt_has_multiple_requirements,
    native_tool_prompt_project_root, native_tool_prompt_required_changed_paths,
    native_tool_prompt_requires_memory_write, native_tool_prompt_requires_product_mutation,
    native_tool_prompt_requires_test_changes, native_tool_prompt_requires_validation_command,
    native_tool_should_synthesize_micro_final,
    native_tool_unique_code_path_mentions,
};
use crate::native_synthetic_artifact::{
    native_tool_synthetic_completion_evidence_response,
    native_tool_synthetic_micro_final_response,
};
use crate::native_workflow_artifact::{
    native_tool_auto_workflow_artifact_receipts,
};
use crate::native_prompt_policy::{
    native_tool_completion_repair_action_brief,
    native_tool_completion_evidence_repair_prompt, native_tool_context_to_mutation_retry_prompt,
    native_tool_empty_retry_prompt, native_tool_initial_prompt,
    native_tool_failed_validation_repair_hint,
    native_tool_missing_test_change_repair_hint,
    native_tool_orchestration_prompt_text, native_tool_public_reasoning_finalization_prompt,
    native_tool_public_reasoning_metadata, native_tool_recovery_prompt,
};
use crate::native_tools::{
    native_tool_observation_prompt, parse_native_tool_calls, NativeToolCall, NativeToolDispatcher,
    NativeToolReceipt,
};
use crate::provider::{
    ProviderClientRegistry, ProviderError, ProviderErrorCode, ProviderRequest, ProviderResponse,
};
use crate::scheduler::SchedulePlan;
use crate::telemetry::{ReceiptEvent, ReceiptSpan};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
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
        let provider = context
            .provider_registry
            .from_provider_id(self.provider.as_str())?;
        let tools = self.resolved_tools(context.capability_catalog);
        let started_ms = Utc::now().timestamp_millis();
        let (response, tool_receipts, provider_call_count, terminal_status) =
            self.run_with_optional_native_tools(provider, &tools)?;
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
        let receipt = json!({
            "type": "agent_run_receipt",
            "agent": self.name,
            "provider": response.provider,
            "model": response.model,
            "status": terminal_status,
            "tool_count": tools.len(),
            "native_tool_call_count": tool_receipts.len(),
            "lifespan_seconds": self.lifespan_seconds,
            "duration_ms": duration_ms,
            "trace_id": trace.trace_id,
            "workflow": self
                .metadata
                .get("workflow")
                .cloned()
                .unwrap_or(Value::Null),
            "native_tool_receipts": tool_receipts,
        });
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
        let dispatcher = NativeToolDispatcher::new(tools);
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
        if !micro_direct_write_task
            && native_tool_bounded_patch_artifact_lane_enabled(&self.metadata)
            && native_tool_requires_successful_mutation(&self.metadata)
        {
            if let Some(result) = native_tool_bounded_patch_artifact_lane(
                &provider,
                &dispatcher,
                tools,
                self.model.clone(),
                &self.metadata,
                &self.initial_prompt,
                &system,
            )? {
                return Ok(result);
            }
        }
        if !micro_direct_write_task
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
                let bootstrap_rule = native_tool_orchestration_prompt_text(
                    &self.metadata,
                    "bootstrap_context_continuation_rule",
                    "Runtime bootstrap context was collected before the first model call. Continue from this already-read context and return only JSON tool calls next. If the source/test files needed for this bounded edit are already present in the observations, do not repeat file_list/file_read; make the smallest file_write/file_patch mutation, then validate when requested.",
                );
                prompt = format!(
                    "{}\n\n{}\n\nNative tool observations:\n{}",
                    self.initial_prompt,
                    bootstrap_rule,
                    observation
                );
            }
        }

        for turn_idx in 0..max_turns {
            if let Some(timeout) = wall_timeout {
                if loop_started.elapsed() >= timeout {
                    if native_tool_has_successful_mutation(&all_receipts)
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
            let request = ProviderRequest {
                prompt: prompt.clone(),
                system: Some(system.clone()),
                tools: tools.to_vec(),
                model: self.model.clone(),
                metadata: self.metadata.clone(),
            };
            let response = match provider.complete(&request) {
                Ok(response) => response,
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && native_tool_has_successful_mutation(&all_receipts)
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
                        && !native_tool_has_successful_mutation(&all_receipts)
                        && provider_call_count <= 1
                        && native_tool_first_turn_timeout_recovery_enabled(&self.metadata) =>
                {
                    if all_receipts.is_empty() {
                        let bootstrap_receipts =
                            native_tool_bootstrap_context_receipts(&dispatcher, &self.initial_prompt);
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
                        let observation = native_tool_observation_prompt(&all_receipts);
                        prompt = native_tool_first_turn_timeout_recovery_prompt(
                            &self.metadata,
                            &self.initial_prompt,
                            error.message.as_str(),
                            &observation,
                        );
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
                        let observation = native_tool_observation_prompt(&[bootstrap_receipt.clone()]);
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
                            self.initial_prompt,
                            bootstrap_rule,
                            observation
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
                            self.initial_prompt,
                            bootstrap_rule,
                            observation
                        );
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
            let calls = parse_native_tool_calls(&response.output);
            if calls.is_empty() {
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
                            self.initial_prompt,
                            bootstrap_rule,
                            observation
                        );
                        last_response = Some(response);
                        continue;
                    }
                }
                if native_tool_requires_successful_mutation(&self.metadata)
                    && !native_tool_has_successful_mutation(&all_receipts)
                    && !all_receipts.is_empty()
                    && empty_tool_retry_count < empty_tool_retry_limit
                {
                    empty_tool_retry_count += 1;
                    let observation = native_tool_observation_prompt(&all_receipts);
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
            let mut turn_receipts = Vec::new();
            for call in calls
                .into_iter()
                .take(native_tool_max_calls_per_turn(&self.metadata))
            {
                let call = native_tool_call_with_prompt_defaults(call, &self.initial_prompt);
                let context_blocked = native_tool_requires_successful_mutation(&self.metadata)
                    && !native_tool_has_successful_mutation(&all_receipts)
                    && context_only_turn_count
                        >= native_tool_max_context_only_turns(&self.metadata)
                    && native_tool_has_successful_context_receipt(&all_receipts)
                    && native_tool_call_is_context_only(&call);
                let receipt = if let Some(blocked) = native_tool_live_stage_blocked_receipt(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    &call,
                ) {
                    blocked
                } else if let Some(blocked) =
                    native_tool_preserved_api_write_blocked_receipt(&self.initial_prompt, &call)
                {
                    blocked
                } else if let Some(blocked) =
                    native_tool_python_src_prefix_import_blocked_receipt(&self.initial_prompt, &call)
                {
                    blocked
                } else if context_blocked {
                    native_tool_product_repair_context_blocked_receipt(call, &[], &all_receipts)
                } else {
                    dispatcher.dispatch(call)
                };
                turn_receipts.push(receipt.clone());
                all_receipts.push(receipt);
            }
            native_tool_persist_run_journal(
                &self.metadata,
                &self.initial_prompt,
                "tool_turn",
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
                let mut response =
                    native_tool_synthetic_micro_final_response(
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
                return Ok((response, all_receipts, provider_call_count, "ok".to_string()));
            }
            if native_tool_synthesize_final_after_successful_validation(&self.metadata)
                && native_tool_has_successful_mutation(&all_receipts)
                && native_tool_has_successful_validation_command(&all_receipts)
                && native_tool_prompt_evidence_gaps(&self.initial_prompt, &all_receipts).is_empty()
            {
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
                return Ok((response, all_receipts, provider_call_count, "ok".to_string()));
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
                return Ok((response, all_receipts, provider_call_count, "ok".to_string()));
            }
        }
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
            && native_tool_has_successful_validation_command(&all_receipts)
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
        let auto_handoff_receipts =
            native_tool_auto_workflow_artifact_receipts(&dispatcher, &self.metadata, &self.initial_prompt, &all_receipts);
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
            && native_tool_has_successful_validation_command(&all_receipts)
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
                    let context_blocked =
                        native_tool_has_successful_context_receipt(&all_receipts)
                            && !native_tool_has_successful_mutation(&all_receipts)
                            && native_tool_call_is_context_only(&call);
                    let receipt = if context_blocked {
                        native_tool_product_repair_context_blocked_receipt(call, &[], &all_receipts)
                    } else if let Some(blocked) =
                        native_tool_preserved_api_write_blocked_receipt(&self.initial_prompt, &call)
                    {
                        blocked
                    } else if let Some(blocked) =
                        native_tool_python_src_prefix_import_blocked_receipt(&self.initial_prompt, &call)
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
        if !unresolved_final_reasons.is_empty()
            && native_tool_artifact_contract_enabled(&self.metadata)
        {
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
            return Ok((response, all_receipts, provider_call_count, "partial_blocked".to_string()));
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
        Ok((response, all_receipts, provider_call_count, "ok".to_string()))
    }
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
    let payload = json!({
        "schema_version": "native_coding_run_journal_v1",
        "source": "infring_native_tool_runtime",
        "stage": stage,
        "updated_at_unix_ms": Utc::now().timestamp_millis(),
        "provider_call_count": provider_call_count,
        "terminal_status": terminal_status,
        "native_tool_receipts": receipts,
        "changed_files": changed_files,
        "validation_receipts": validation_receipts,
        "latest_output_preview": latest_output.unwrap_or("").chars().take(2000).collect::<String>(),
        "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
    });
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(path, text);
    }
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

fn native_tool_first_turn_timeout_recovery_prompt(
    metadata: &Value,
    original_prompt: &str,
    reason: &str,
    observation: &str,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "first_turn_timeout_recovery_rule",
        "The first model call timed out before mutation. Continue with a smaller patch-only native tool turn. Use the already gathered runtime observations; do not call file_list/file_read again unless the validation output names a new missing file. Return only JSON tool_calls. Make the smallest file_write/file_patch source/test mutation that satisfies the requested local code change, then run requested validation. If the task is genuinely blocked, return a structured blocker instead of more exploration.",
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

fn native_tool_bounded_patch_artifact_lane(
    provider: &Arc<dyn crate::provider::ProviderClient>,
    dispatcher: &NativeToolDispatcher,
    tools: &[String],
    model: Option<String>,
    metadata: &Value,
    original_prompt: &str,
    system: &str,
) -> Result<Option<(ProviderResponse, Vec<NativeToolReceipt>, u64, String)>, ProviderError> {
    let Some(project_root) = native_tool_prompt_project_root(original_prompt).map(PathBuf::from)
    else {
        return Ok(None);
    };
    let context_paths =
        native_tool_bounded_patch_context_paths(metadata, original_prompt, &project_root);
    if context_paths.is_empty() {
        return Ok(None);
    }

    let mut receipts = Vec::<NativeToolReceipt>::new();
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
    if read_receipt.status != "ok" {
        return Ok(None);
    }
    receipts.push(read_receipt);

    let Some(file_context) = native_tool_bounded_patch_file_context(&context_paths) else {
        return Ok(None);
    };
    let mut patch_metadata = metadata.clone();
    if let Some(object) = patch_metadata.as_object_mut() {
        object.insert(
            "provider_timeout_seconds".to_string(),
            json!(native_tool_bounded_patch_artifact_provider_timeout_seconds(metadata)),
        );
        object.insert("native_bounded_patch_artifact_lane".to_string(), json!(true));
    }
    let patch_prompt =
        native_tool_bounded_patch_artifact_prompt(metadata, original_prompt, &file_context);
    let request = ProviderRequest {
        prompt: patch_prompt,
        system: Some(native_tool_bounded_patch_artifact_system_prompt()),
        tools: Vec::new(),
        model: model.clone(),
        metadata: patch_metadata,
    };
    let mut provider_call_count = 1u64;
    let response = match provider.complete(&request) {
        Ok(response) => response,
        Err(error) if native_tool_provider_error_is_timeout(&error) => return Ok(None),
        Err(error) => return Err(error),
    };

    let patches =
        native_tool_bounded_patch_artifact_patches(&response.output, &project_root, &context_paths);
    if patches.is_empty() {
        return Ok(None);
    }
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
    if !native_tool_has_successful_mutation(&receipts) {
        return Ok(None);
    }

    if let Some(validation_receipt) =
        native_tool_auto_validation_receipt(dispatcher, original_prompt, &receipts)
    {
        receipts.push(validation_receipt);
    }

    let mut response = response;
    let repair_reasons =
        native_tool_runtime_repair_reasons(metadata, original_prompt, &response.output, &receipts);
    if !repair_reasons.is_empty() && native_tool_completion_evidence_repair_enabled(metadata) {
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
            || native_tool_has_successful_validation_command(&receipts))
    {
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
                "tool_receipts": receipts.clone(),
                "terminal_status": "ok"
            }
        });
        return Ok(Some((final_response, receipts, provider_call_count, "ok".to_string())));
    }

    Ok(None)
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
            if let Some(path) =
                native_tool_resolve_bounded_patch_context_path(project_root, &path)
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

fn native_tool_bounded_patch_artifact_system_prompt() -> String {
    "You are Infring's bounded patch artifact generator. Return only valid JSON. Do not use markdown, prose, or tool calls. Your JSON must have this shape: {\"patches\":[{\"path\":\"/absolute/path\",\"old\":\"exact existing text\",\"new\":\"replacement text\"}]}. Each old field must be an exact contiguous substring from the supplied file context. Prefer the smallest complete source/test patch set. Do not include validation output, explanations, or final answers.".to_string()
}

fn native_tool_bounded_patch_artifact_prompt(
    metadata: &Value,
    original_prompt: &str,
    file_context: &str,
) -> String {
    let rule = native_tool_orchestration_prompt_text(
        metadata,
        "bounded_patch_artifact_lane_rule",
        "Generate the smallest deterministic patch artifact for this bounded local code edit. The runtime will apply the patch and run validation from receipts. Do not ask for files; the relevant file context is already supplied.",
    );
    format!(
        "{}\n\nOriginal task:\n{}\n\nSelected local file context:\n{}\n\nReturn only the JSON patch artifact now.",
        rule,
        original_prompt.chars().take(2600).collect::<String>(),
        file_context
    )
}

fn native_tool_bounded_patch_artifact_patches(
    output: &str,
    project_root: &Path,
    allowed_paths: &[PathBuf],
) -> Vec<NativeToolBoundedPatch> {
    let Some(value) = native_tool_parse_bounded_patch_artifact_json(output) else {
        return Vec::new();
    };
    let Some(patches) = value.get("patches").and_then(Value::as_array) else {
        return Vec::new();
    };
    let allowed = allowed_paths
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .collect::<Vec<_>>();
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
            let receipt = dispatcher.dispatch(native_tool_call_with_prompt_defaults(
                call,
                original_prompt,
            ));
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
            native_tool_recovery_prompt(metadata, original_prompt, reason, &changed_paths, &receipts),
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
    let auto_handoff_receipts =
        native_tool_auto_workflow_artifact_receipts(dispatcher, metadata, original_prompt, &receipts);
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
        && native_tool_has_successful_validation_command(&receipts)
        && native_tool_prompt_evidence_gaps(original_prompt, &receipts).is_empty();
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
    let mut reasons = native_tool_artifact_repair_reasons(metadata, original_prompt, output, receipts);
    if native_tool_requires_successful_mutation(metadata)
        && native_tool_prompt_requires_product_mutation(&original_prompt.to_ascii_lowercase())
        && !native_tool_has_successful_mutation(receipts)
        && !reasons
            .iter()
            .any(|reason| reason == "missing_product_mutation_receipt")
    {
        reasons.push("missing_product_mutation_receipt".to_string());
    }
    reasons.sort();
    reasons.dedup();
    reasons
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
    })
}

fn native_tool_repair_reasons_require_product_work(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason == "missing_product_mutation_receipt"
            || reason == "missing_test_change_receipt"
            || reason.starts_with("incomplete_product_slice")
            || reason.starts_with("missing_product_source_evidence:")
            || reason.starts_with("missing_public_interface_verification:")
            || reason.starts_with("missing_changed_path:")
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
    receipts.iter().filter(|receipt| {
        receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_read" | "file_read_many")
    }).count()
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
    !native_tool_prompt_evidence_gaps(original_prompt, receipts)
        .iter()
        .any(|reason| {
            reason
                .strip_prefix("missing_changed_path:")
                .map(native_tool_path_is_product_mutation_path)
                .unwrap_or(false)
        })
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
        .filter(|reason| reason.starts_with("missing_changed_path:"))
        .collect::<Vec<_>>();
    for path in native_tool_preserved_api_source_paths_from_workspace(original_prompt) {
        if !native_tool_changed_paths_include(receipts, &path) {
            reasons.push(format!("missing_changed_path:{path}"));
        }
    }
    reasons.sort();
    reasons.dedup();
    reasons
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
    repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
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
    )
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

fn native_tool_missing_changed_paths(repair_reasons: &[String]) -> Vec<String> {
    repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
        .map(str::to_string)
        .collect()
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
    let path_keys = [
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
        "file_write" | "write_file" | "workspace.write" | "workspace_write" | "file_patch"
            | "patch_file" | "workspace.patch" | "workspace_patch"
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
    if !native_tool_staged_execution_controller_enabled(metadata) {
        return None;
    }
    let stage = native_tool_current_live_stage(original_prompt, receipts)?;
    let live_repair_reasons = native_tool_live_stage_repair_reasons(original_prompt, receipts);
    native_tool_stage_block_reason(metadata, original_prompt, stage, receipts, call)
        .map(|reason| {
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
    if native_tool_prompt_requires_validation_command(&prompt_lower)
        && !native_tool_has_successful_validation_command(receipts)
    {
        return Some("validation");
    }
    if native_tool_prompt_required_changed_paths(original_prompt)
        .into_iter()
        .filter(|path| native_tool_path_is_handoff_artifact(path))
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
    if !native_tool_staged_execution_controller_enabled(metadata) {
        return None;
    }
    let stage = native_tool_current_repair_stage(repair_reasons, receipts)?;
    native_tool_stage_block_reason(metadata, original_prompt, stage, receipts, call)
        .map(|reason| {
            native_tool_stage_blocked_repair_receipt(
                call,
                stage,
                reason,
                repair_reasons,
                receipts,
            )
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
    let needs_requested_pre_mutation_validation =
        native_tool_prompt_requires_pre_mutation_validation(original_prompt)
            && !native_tool_has_any_validation_command(receipts);
    match stage {
        "product_mutation" => {
            if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_product_source_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_product_source_before_memory_closure")
            } else if needs_requested_pre_mutation_validation && native_tool_call_is_mutation(call)
            {
                Some("staged_controller_requires_requested_validation_before_mutation")
            } else if native_tool_call_is_test_mutation(call) {
                None
            } else if !has_product_source
                && native_tool_call_is_command_run(call)
                && !needs_requested_pre_mutation_validation
            {
                Some("staged_controller_requires_product_source_mutation_before_command")
            } else if !has_product_source
                && read_context_count >= native_tool_pre_mutation_read_budget(metadata)
                && native_tool_call_is_context_only(call)
            {
                Some("staged_controller_requires_product_source_mutation_before_more_context")
            } else {
                None
            }
        }
        "test_mutation" => {
            if native_tool_call_is_test_mutation(call) {
                None
            } else if native_tool_call_is_command_run(call) {
                Some("staged_controller_requires_test_file_write_or_patch_before_validation")
            } else if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_test_file_write_or_patch_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_test_file_write_or_patch_before_memory_closure")
            } else if native_tool_call_is_mutation(call) {
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
        "validation" => {
            let has_failed_validation = !native_tool_failed_validation_command_refs(receipts).is_empty();
            let failed_validation_read_count =
                native_tool_successful_read_context_count_after_first_failed_validation(receipts);
            if native_tool_call_targets_handoff_artifact(call) {
                Some("staged_controller_requires_validation_before_checkpoint_handoff")
            } else if native_tool_call_is_memory_closure(call) {
                Some("staged_controller_requires_validation_before_memory_closure")
            } else if native_tool_call_is_command_run(call) {
                None
            } else if has_failed_validation && native_tool_call_is_mutation(call) {
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
            lower.contains("checkpoint_") || lower.contains("handoff") || lower.contains("/receipts/")
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

fn native_tool_call_is_memory_closure(call: &NativeToolCall) -> bool {
    if !native_tool_call_is_command_run(call) {
        return false;
    }
    let args = serde_json::to_string(&call.args)
        .unwrap_or_default()
        .to_ascii_lowercase();
    args.contains("memory-cli") || (args.contains("checkpoint") && args.contains("memory"))
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
            } else {
                "file_write_or_file_patch_or_command_run_for_required_stage"
            },
            "missing_changed_paths": missing_changed_paths,
            "observed_test_paths": observed_test_paths
        }),
        error: Some(reason.to_string()),
    }
}

fn native_tool_call_targets_unrelated_repair_path(
    call: &NativeToolCall,
    repair_reasons: &[String],
    original_prompt: &str,
) -> bool {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write" | "write_file" | "workspace.write" | "workspace_write" | "file_patch"
            | "patch_file" | "workspace.patch" | "workspace_patch"
    ) || native_tool_call_targets_handoff_artifact(call)
    {
        return false;
    }
    let mut required = repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
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
                &["old", "old_text", "search", "search_text", "before", "target_text"],
            )?;
            let new = native_tool_call_text_arg(
                call,
                &["new", "new_text", "replace", "replace_text", "after", "replacement"],
            )?;
            if old.is_empty() || existing.matches(old).count() != 1 {
                return None;
            }
            Some(existing.replacen(old, new, 1))
        }
        _ => None,
    }
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
            "required_next_tool": "file_patch_or_file_write_that_keeps_preserved_api_block_identical"
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
                "and" | "api" | "behavior" | "behaviour" | "current" | "existing" | "public" | "the"
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

fn native_tool_numeric_tokens(block: &str) -> Vec<String> {
    block
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .filter(|token| token.chars().any(|ch| ch.is_ascii_digit()))
        .map(str::to_string)
        .collect()
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

fn native_tool_prioritize_repair_calls(
    calls: &mut [NativeToolCall],
    repair_reasons: &[String],
) {
    let needs_test_change = repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt");
    let needs_missing_path = repair_reasons
        .iter()
        .any(|reason| reason.starts_with("missing_changed_path:"));
    let needs_product_work = native_tool_repair_reasons_require_product_work(repair_reasons);
    let needs_validation_repair = native_tool_repair_reasons_include_validation(repair_reasons);
    if !(needs_test_change || needs_missing_path || needs_product_work || needs_validation_repair) {
        return;
    }
    calls.sort_by_key(|call| {
        if needs_test_change && !needs_missing_path && native_tool_call_is_test_mutation(call) {
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

fn native_tool_completion_repair_tools(
    tools: &[String],
    repair_reasons: &[String],
) -> Vec<String> {
    let missing_test_only = repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
        && !repair_reasons
            .iter()
            .any(|reason| reason.starts_with("missing_changed_path:"))
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
    let mut prompt =
        native_tool_completion_evidence_repair_prompt(
            metadata,
            original_prompt,
            &response.output,
            &receipts,
            &repair_reasons,
    );
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
                    && native_tool_has_successful_validation_command(&receipts)
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
            prompt = native_tool_completion_evidence_repair_prompt(
                metadata,
                original_prompt,
                &response.output,
                &receipts,
                &repair_reasons,
            );
            continue;
        }
        native_tool_prioritize_repair_calls(&mut calls, &repair_reasons);
        let mut turn_receipts = Vec::new();
        let product_work_required =
            native_tool_repair_reasons_require_product_work(&repair_reasons);
        let has_successful_mutation = native_tool_has_successful_mutation(&receipts);
        let has_successful_test_mutation = native_tool_has_successful_test_mutation(&receipts);
        let has_exhausted_repair_read_budget =
            native_tool_successful_read_context_count(&receipts)
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
            let receipt =
                if let Some(blocked) = native_tool_staged_repair_blocked_receipt(
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
                    native_tool_product_repair_context_blocked_receipt(
                        call,
                        &repair_reasons,
                        &receipts,
                    )
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
                    native_tool_python_src_prefix_import_blocked_receipt(original_prompt, &call)
                {
                    blocked
                } else {
                    dispatcher.dispatch(call)
                };
            turn_receipts.push(receipt.clone());
            receipts.push(receipt);
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
        prompt = format!(
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
        );
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
        object.insert("provider_timeout_seconds".to_string(), json!(timeout_seconds));
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

fn native_tool_bootstrap_likely_context_paths(root: &Path, existing_paths: &[String]) -> Vec<String> {
    let mut candidates = Vec::<PathBuf>::new();
    native_tool_collect_bootstrap_context_candidates(root, 0, &mut candidates);
    let max_project_candidates = 24usize;
    if candidates.is_empty() || candidates.len() > max_project_candidates {
        return Vec::new();
    }

    candidates.sort_by_key(|path| {
        let normalized = path.display().to_string().replace('\\', "/").to_ascii_lowercase();
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
        || native_tool_has_successful_validation_command(receipts)
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
    let cmd = if prompt_lower.contains("pytest")
        || (project_root_path.join("pyproject.toml").exists()
            && project_root_path.join("tests").is_dir())
    {
        vec!["python3", "-m", "pytest", "-q"]
    } else if prompt_lower.contains("unittest")
        || (project_root_path.join("src").is_dir() && project_root_path.join("tests").is_dir())
    {
        vec![
            "sh",
            "-c",
            "PYTHONPATH=src python3 -m unittest discover -s tests",
        ]
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
