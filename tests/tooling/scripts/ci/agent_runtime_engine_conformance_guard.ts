#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');
const ROOT = process.cwd();
const registryPath = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const socketPath = 'validation/conformance/contracts/agent_runtime_socket_contract.json';
const adapterContractsPath = 'validation/conformance/contracts/agent_runtime_adapter_contracts.json';
const structuredTransportContractPath = 'validation/conformance/contracts/agent_runtime_structured_transport_contract.json';
const turnOutcomeContractPath = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const engineScorecardContractPath = 'validation/conformance/contracts/agent_runtime_engine_scorecard_contract.json';
const routerScopeContractPath = 'validation/conformance/contracts/agent_runtime_router_scope_contract.json';
const contextPackContractPath = 'validation/conformance/contracts/agent_runtime_context_pack_contract.json';
const contextAuthorityBoundaryContractPath = 'validation/conformance/contracts/context_authority_boundary_contract.json';
const universalToolsContractPath = 'validation/conformance/contracts/agent_runtime_universal_tools_contract.json';
const outPath = 'core/local/artifacts/agent_runtime_engine_conformance_guard_current.json';
const violations: any[] = [];

function readJson(rel: string): any {
  return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
}

function exists(rel: string): boolean {
  return fs.existsSync(path.join(ROOT, rel));
}

const registry = readJson(registryPath);
const socket = readJson(socketPath);
const adapterContracts = readJson(adapterContractsPath);
const structuredTransportContract = readJson(structuredTransportContractPath);
const turnOutcomeContract = readJson(turnOutcomeContractPath);
const engineScorecardContract = readJson(engineScorecardContractPath);
const routerScopeContract = readJson(routerScopeContractPath);
const contextPackContract = readJson(contextPackContractPath);
const contextAuthorityBoundaryContract = readJson(contextAuthorityBoundaryContractPath);
const universalToolsContract = readJson(universalToolsContractPath);

if (registry.socket_contract !== socketPath) violations.push({ kind: 'registry_socket_contract_mismatch', path: registryPath });
if (registry.turn_outcome_contract !== turnOutcomeContractPath) violations.push({ kind: 'registry_turn_outcome_contract_mismatch', path: registryPath });
if (registry.router_scope_contract !== routerScopeContractPath) violations.push({ kind: 'registry_router_scope_contract_mismatch', path: registryPath });
if (registry.universal_tools_contract !== universalToolsContractPath) violations.push({ kind: 'registry_universal_tools_contract_mismatch', path: registryPath });
if (socket.canonical_endpoint?.canonical_route_pattern !== '/ws/agent-runtime') violations.push({ kind: 'socket_route_not_canonical', path: socketPath });
if (!socket.trace_identity_rule?.trace_id_required_on_every_message) violations.push({ kind: 'trace_id_not_required', path: socketPath });
if (!socket.trace_identity_rule?.gateway_router_rejects_adapter_trace_id_replacement) violations.push({ kind: 'trace_replacement_rejection_not_required', path: socketPath });
if (!socket.kernel_authority_invariant?.durable_effects_require_kernel_or_gateway_policy) violations.push({ kind: 'kernel_authority_not_invariant', path: socketPath });
if (registry.private_adapter_contracts !== adapterContractsPath) violations.push({ kind: 'registry_private_adapter_contracts_mismatch', path: registryPath });
if (registry.structured_transport_contract !== structuredTransportContractPath) violations.push({ kind: 'registry_structured_transport_contract_mismatch', path: registryPath });
if (socket.private_adapter_contracts !== adapterContractsPath) violations.push({ kind: 'socket_private_adapter_contracts_mismatch', path: socketPath });
if (socket.turn_outcome_contract !== turnOutcomeContractPath) violations.push({ kind: 'socket_turn_outcome_contract_mismatch', path: socketPath });
if (socket.router_scope_contract !== routerScopeContractPath) violations.push({ kind: 'socket_router_scope_contract_mismatch', path: socketPath });
if (socket.context_pack_contract !== contextPackContractPath) violations.push({ kind: 'socket_context_pack_contract_mismatch', path: socketPath });
if (socket.universal_tools_contract !== universalToolsContractPath) violations.push({ kind: 'socket_universal_tools_contract_mismatch', path: socketPath });
if (contextPackContract.context_authority_boundary_contract !== contextAuthorityBoundaryContractPath) violations.push({ kind: 'context_pack_boundary_contract_mismatch', path: contextPackContractPath });
if (contextPackContract.universal_tools_contract !== universalToolsContractPath) violations.push({ kind: 'context_pack_universal_tools_contract_mismatch', path: contextPackContractPath });
if (adapterContracts.universal_tools_contract !== universalToolsContractPath) violations.push({ kind: 'adapter_universal_tools_contract_mismatch', path: adapterContractsPath });
if (adapterContracts.structured_transport_contract !== structuredTransportContractPath) violations.push({ kind: 'adapter_structured_transport_contract_mismatch', path: adapterContractsPath });
if (structuredTransportContract.type !== 'agent_runtime_structured_transport_contract') violations.push({ kind: 'structured_transport_contract_type_wrong', path: structuredTransportContractPath });
if (structuredTransportContract.public_socket_contract !== socketPath) violations.push({ kind: 'structured_transport_public_socket_contract_mismatch', path: structuredTransportContractPath });
if (structuredTransportContract.context_pack_contract !== contextPackContractPath) violations.push({ kind: 'structured_transport_context_pack_contract_mismatch', path: structuredTransportContractPath });
if (structuredTransportContract.universal_tools_contract !== universalToolsContractPath) violations.push({ kind: 'structured_transport_universal_tools_contract_mismatch', path: structuredTransportContractPath });
if (!structuredTransportContract.canonical_turn_schema?.builder_path || !exists(structuredTransportContract.canonical_turn_schema.builder_path)) {
  violations.push({ kind: 'structured_transport_builder_missing', path: structuredTransportContract.canonical_turn_schema?.builder_path || null });
}
if (structuredTransportContract.canonical_turn_schema?.builder_export !== 'buildAgentRuntimeStructuredTurn') {
  violations.push({ kind: 'structured_transport_builder_export_wrong', actual: structuredTransportContract.canonical_turn_schema?.builder_export, path: structuredTransportContractPath });
}
const structuredTransportEvalRequirements = structuredTransportContract.structured_transport_eval_requirements || {};
if (!structuredTransportEvalRequirements.script || !exists(structuredTransportEvalRequirements.script)) {
  violations.push({ kind: 'structured_transport_eval_script_missing', path: structuredTransportEvalRequirements.script || null });
}
if (structuredTransportEvalRequirements.must_write_artifact !== 'core/local/artifacts/agent_runtime_structured_transport_eval_current.json') {
  violations.push({ kind: 'structured_transport_eval_artifact_path_wrong', actual: structuredTransportEvalRequirements.must_write_artifact, path: structuredTransportContractPath });
}
for (const [field, expected] of Object.entries({
  gateway_must_attach_structured_turn_before_adapter_dispatch: true,
  adapters_may_render_prompt_text_from_structured_turn: true,
  adapters_may_not_invent_context_sections_outside_structured_turn: true,
  shell_may_not_construct_structured_turn: true,
  kernel_may_record_refs_but_not_depend_on_engine_schema: true,
})) {
  if (structuredTransportContract.gateway_materialization_policy?.[field] !== expected) {
    violations.push({ kind: 'structured_transport_materialization_policy_missing', field, expected, actual: structuredTransportContract.gateway_materialization_policy?.[field], path: structuredTransportContractPath });
  }
}
for (const [field, expected] of Object.entries({
  must_verify_gateway_payload: true,
  must_verify_required_sections: true,
  must_verify_prompt_compat_derives_from_structured_turn: true,
  must_verify_no_secret_material: true,
})) {
  if (structuredTransportEvalRequirements[field] !== expected) {
    violations.push({ kind: 'structured_transport_eval_requirement_missing', field, expected, actual: structuredTransportEvalRequirements[field], path: structuredTransportContractPath });
  }
}
if (turnOutcomeContract.type !== 'agent_runtime_turn_outcome_contract') violations.push({ kind: 'turn_outcome_contract_type_wrong', path: turnOutcomeContractPath });
if (turnOutcomeContract.public_socket_contract !== socketPath) violations.push({ kind: 'turn_outcome_public_socket_contract_mismatch', path: turnOutcomeContractPath });
if (turnOutcomeContract.engine_registry !== registryPath) violations.push({ kind: 'turn_outcome_engine_registry_mismatch', path: turnOutcomeContractPath });
if (engineScorecardContract.type !== 'agent_runtime_engine_scorecard_contract') violations.push({ kind: 'engine_scorecard_contract_type_wrong', path: engineScorecardContractPath });
if (engineScorecardContract.engine_registry !== registryPath) violations.push({ kind: 'engine_scorecard_registry_mismatch', path: engineScorecardContractPath });
if (engineScorecardContract.turn_outcome_contract !== turnOutcomeContractPath) violations.push({ kind: 'engine_scorecard_turn_outcome_contract_mismatch', path: engineScorecardContractPath });
if (!engineScorecardContract.script || !exists(engineScorecardContract.script)) violations.push({ kind: 'engine_scorecard_script_missing', path: engineScorecardContract.script || null });
if (engineScorecardContract.artifact !== 'core/local/artifacts/agent_runtime_engine_scorecard_current.json') violations.push({ kind: 'engine_scorecard_artifact_path_wrong', actual: engineScorecardContract.artifact, path: engineScorecardContractPath });
if (engineScorecardContract.shell_boundary_rule?.shell_may_not_score_or_interpret_engines !== true) violations.push({ kind: 'engine_scorecard_shell_boundary_missing', path: engineScorecardContractPath });
for (const capability of ['registered', 'adapter_contract', 'discovery_metadata', 'model_catalog_metadata', 'context_continuity', 'live_work_completion', 'approval_pause', 'durable_receipts', 'activity_trace', 'practical_usability_loop', 'structured_transport', 'transport_migration', 'error_projection', 'real_work_replay']) {
  if (!Array.isArray(engineScorecardContract.scored_capabilities) || !engineScorecardContract.scored_capabilities.includes(capability)) {
    violations.push({ kind: 'engine_scorecard_capability_missing', capability, path: engineScorecardContractPath });
  }
}
if (routerScopeContract.type !== 'agent_runtime_router_scope_contract') violations.push({ kind: 'router_scope_contract_type_wrong', path: routerScopeContractPath });
if (routerScopeContract.router_path !== 'gateway/runtime/agent_runtime/agent_runtime_router.ts') violations.push({ kind: 'router_scope_router_path_wrong', path: routerScopeContractPath });
if (routerScopeContract.universal_tools_path !== 'gateway/runtime/agent_runtime/universal_core_tools.ts') violations.push({ kind: 'router_scope_universal_tools_path_wrong', path: routerScopeContractPath });
if (routerScopeContract.public_socket_contract !== socketPath) violations.push({ kind: 'router_scope_socket_contract_mismatch', path: routerScopeContractPath });
for (const forbidden of ['plan_workflows', 'execute_workflows', 'own_kernel_authority', 'own_shell_state', 'implement_provider_business_logic', 'execute_terminal_commands', 'perform_direct_file_mutations', 'make_policy_approval_decisions']) {
  if (!Array.isArray(routerScopeContract.forbidden_router_responsibilities) || !routerScopeContract.forbidden_router_responsibilities.includes(forbidden)) {
    violations.push({ kind: 'router_scope_forbidden_responsibility_missing', forbidden, path: routerScopeContractPath });
  }
}
if (routerScopeContract.universal_core_tool_scope?.max_tool_count !== 6) violations.push({ kind: 'router_scope_universal_tool_count_wrong', path: routerScopeContractPath });
for (const toolId of ['conversation.read', 'memory.read', 'memory.write_propose', 'artifact.read', 'artifact.create_propose', 'permission.request']) {
  if (!Array.isArray(routerScopeContract.universal_core_tool_scope?.allowed_tool_ids) || !routerScopeContract.universal_core_tool_scope.allowed_tool_ids.includes(toolId)) {
    violations.push({ kind: 'router_scope_universal_tool_missing', tool_id: toolId, path: routerScopeContractPath });
  }
}
if (routerScopeContract.universal_core_tool_scope?.native_workflow_tools_exposed !== false) violations.push({ kind: 'router_scope_native_workflow_tools_not_forbidden', path: routerScopeContractPath });
if (routerScopeContract.growth_guard?.provider_specific_logic_belongs_in_adapter !== true) violations.push({ kind: 'router_scope_provider_logic_guard_missing', path: routerScopeContractPath });
if (universalToolsContract.router_scope_contract !== routerScopeContractPath) violations.push({ kind: 'universal_tools_router_scope_contract_mismatch', path: universalToolsContractPath });
if (universalToolsContract.tool_surface_scope?.max_tool_count !== 6) violations.push({ kind: 'universal_tools_scope_tool_count_wrong', path: universalToolsContractPath });
if (universalToolsContract.tool_surface_scope?.workflow_tools_allowed !== false || universalToolsContract.tool_surface_scope?.provider_specific_tools_allowed !== false) {
  violations.push({ kind: 'universal_tools_scope_allows_non_core_tools', path: universalToolsContractPath });
}
for (const status of ['completed', 'permission_required', 'failed_with_reason', 'timed_out_with_reason']) {
  if (!Array.isArray(turnOutcomeContract.required_terminal_outcomes) || !turnOutcomeContract.required_terminal_outcomes.some((row: any) => row && row.status === status)) {
    violations.push({ kind: 'turn_outcome_status_missing', status, path: turnOutcomeContractPath });
  }
}
for (const forbidden of ['silent_failure', 'worked_timer_without_final_status', 'missing_error_code_on_failure']) {
  if (!Array.isArray(turnOutcomeContract.forbidden_turn_outcomes) || !turnOutcomeContract.forbidden_turn_outcomes.includes(forbidden)) {
    violations.push({ kind: 'turn_outcome_forbidden_case_missing', forbidden, path: turnOutcomeContractPath });
  }
}
if (turnOutcomeContract.pre_turn_failure_rule?.engine_unavailable_must_project_chat_visible_failure !== true) {
  violations.push({ kind: 'turn_outcome_pre_turn_failure_projection_missing', path: turnOutcomeContractPath });
}
const liveWorkEvalRequirements = turnOutcomeContract.live_work_eval_requirements || {};
if (!liveWorkEvalRequirements.script || !exists(liveWorkEvalRequirements.script)) {
  violations.push({ kind: 'agent_runtime_live_work_eval_script_missing', path: liveWorkEvalRequirements.script || null });
}
for (const [field, expected] of Object.entries({
  must_submit_real_turn: true,
  must_verify_completed_turn: true,
  must_verify_activity_trace: true,
  must_verify_receipt_refs: true,
  must_verify_approval_pause: true,
  must_verify_decision_resolves_stored_request: true,
  must_remove_probe_artifact: true,
})) {
  if (liveWorkEvalRequirements[field] !== expected) {
    violations.push({ kind: 'agent_runtime_live_work_eval_requirement_missing', field, expected, actual: liveWorkEvalRequirements[field], path: turnOutcomeContractPath });
  }
}
if (liveWorkEvalRequirements.must_write_artifact !== 'core/local/artifacts/agent_runtime_live_work_eval_current.json') {
  violations.push({ kind: 'agent_runtime_live_work_eval_artifact_path_wrong', actual: liveWorkEvalRequirements.must_write_artifact, path: turnOutcomeContractPath });
}
if (liveWorkEvalRequirements.supports_engine_matrix !== true) {
  violations.push({ kind: 'agent_runtime_live_work_eval_matrix_not_supported', path: turnOutcomeContractPath });
}
if (liveWorkEvalRequirements.engine_matrix_field !== 'engine_results') {
  violations.push({ kind: 'agent_runtime_live_work_eval_matrix_field_wrong', actual: liveWorkEvalRequirements.engine_matrix_field, path: turnOutcomeContractPath });
}
if (!Array.isArray(liveWorkEvalRequirements.default_engine_matrix) || !liveWorkEvalRequirements.default_engine_matrix.includes('codex_cli') || !liveWorkEvalRequirements.default_engine_matrix.includes('claude_code')) {
  violations.push({ kind: 'agent_runtime_live_work_eval_default_matrix_missing_required_engine', path: turnOutcomeContractPath, matrix: liveWorkEvalRequirements.default_engine_matrix });
}
const approvalPauseResumeRule = turnOutcomeContract.approval_pause_resume_rule || {};
for (const [field, expected] of Object.entries({
  gateway_must_record_pending_request: true,
  shell_may_only_render_request_and_submit_decision: true,
  shell_may_not_reconstruct_proposal_arguments: true,
  decision_route_must_resolve_from_gateway_pending_request: true,
  decision_must_emit_ack_receipt: true,
})) {
  if (approvalPauseResumeRule[field] !== expected) {
    violations.push({ kind: 'approval_pause_resume_rule_missing', field, expected, actual: approvalPauseResumeRule[field], path: turnOutcomeContractPath });
  }
}
for (const decision of ['allow_once', 'deny', 'always_allow_tool_call']) {
  if (!Array.isArray(approvalPauseResumeRule.allowed_decisions) || !approvalPauseResumeRule.allowed_decisions.includes(decision)) {
    violations.push({ kind: 'approval_pause_resume_decision_missing', decision, path: turnOutcomeContractPath });
  }
}
for (const strategy of ['gateway_apply_approved_effect', 'grant_then_retry_next_turn']) {
  if (!Array.isArray(approvalPauseResumeRule.resume_strategies) || !approvalPauseResumeRule.resume_strategies.includes(strategy)) {
    violations.push({ kind: 'approval_pause_resume_strategy_missing', strategy, path: turnOutcomeContractPath });
  }
}
const receiptFirstRule = turnOutcomeContract.receipt_first_rule || {};
for (const [field, expected] of Object.entries({
  gateway_must_emit_turn_receipts: true,
  receipt_projection_must_be_bounded: true,
  receipt_refs_required_on_terminal_turn_projection: true,
})) {
  if (receiptFirstRule[field] !== expected) {
    violations.push({ kind: 'agent_runtime_receipt_first_rule_missing', field, expected, actual: receiptFirstRule[field], path: turnOutcomeContractPath });
  }
}
if (receiptFirstRule.receipt_store_owner !== 'gateway.runtime.agent_runtime_receipts') {
  violations.push({ kind: 'agent_runtime_receipt_store_owner_wrong', actual: receiptFirstRule.receipt_store_owner, path: turnOutcomeContractPath });
}
for (const kind of ['engine_selection', 'context_pack']) {
  if (!Array.isArray(receiptFirstRule.required_receipt_kinds) || !receiptFirstRule.required_receipt_kinds.includes(kind)) {
    violations.push({ kind: 'agent_runtime_required_receipt_kind_missing', receipt_kind: kind, path: turnOutcomeContractPath });
  }
}
for (const kind of ['final_response', 'approval_pause', 'failure_classification']) {
  if (!Array.isArray(receiptFirstRule.terminal_receipt_kinds) || !receiptFirstRule.terminal_receipt_kinds.includes(kind)) {
    violations.push({ kind: 'agent_runtime_terminal_receipt_kind_missing', receipt_kind: kind, path: turnOutcomeContractPath });
  }
}
const activityTraceParityRule = turnOutcomeContract.activity_trace_parity_rule || {};
for (const [field, expected] of Object.entries({
  gateway_must_emit_activity_trace_projection: true,
  shell_may_render_trace_but_not_interpret_provider_payloads: true,
  activity_trace_must_be_bounded: true,
  activity_trace_collapsed_by_default: true,
  activity_trace_requires_worked_label: true,
  activity_rows_must_have_user_facing_title: true,
  raw_provider_events_must_remain_behind_refs: true,
  status_suffix_sandwich_forbidden: true,
})) {
  if (activityTraceParityRule[field] !== expected) {
    violations.push({ kind: 'agent_runtime_activity_trace_parity_rule_missing', field, expected, actual: activityTraceParityRule[field], path: turnOutcomeContractPath });
  }
}
for (const field of ['type', 'trace_id', 'engine_id', 'session_id', 'turn_id', 'collapsed_by_default', 'collapse_label', 'worked_ms', 'row_count', 'rows', 'summary_text']) {
  if (!Array.isArray(activityTraceParityRule.required_activity_trace_fields) || !activityTraceParityRule.required_activity_trace_fields.includes(field)) {
    violations.push({ kind: 'agent_runtime_activity_trace_required_field_missing', field, path: turnOutcomeContractPath });
  }
}
for (const failureClass of ['provider_quota_or_subscription_unavailable', 'provider_auth_required', 'provider_rate_limited', 'runtime_not_available']) {
  if (!Array.isArray(turnOutcomeContract.provider_failure_reason_classes) || !turnOutcomeContract.provider_failure_reason_classes.includes(failureClass)) {
    violations.push({ kind: 'turn_outcome_provider_failure_class_missing', failure_class: failureClass, path: turnOutcomeContractPath });
  }
}
const hardFailureInjectionRequirements = turnOutcomeContract.hard_failure_injection_requirements || {};
if (!hardFailureInjectionRequirements.script || !exists(hardFailureInjectionRequirements.script)) {
  violations.push({ kind: 'agent_runtime_hard_failure_injection_script_missing', path: hardFailureInjectionRequirements.script || null });
}
if (hardFailureInjectionRequirements.must_write_artifact !== 'core/local/artifacts/agent_runtime_hard_failure_injection_eval_current.json') {
  violations.push({ kind: 'agent_runtime_hard_failure_injection_artifact_path_wrong', actual: hardFailureInjectionRequirements.must_write_artifact, path: turnOutcomeContractPath });
}
for (const [field, expected] of Object.entries({
  must_project_http_200: true,
  must_include_display_text: true,
  must_include_next_actions: true,
  must_include_receipt_refs: true,
  must_include_activity_trace: true,
})) {
  if (hardFailureInjectionRequirements[field] !== expected) {
    violations.push({ kind: 'agent_runtime_hard_failure_injection_requirement_missing', field, expected, actual: hardFailureInjectionRequirements[field], path: turnOutcomeContractPath });
  }
}
for (const failureClass of ['provider_auth_required', 'provider_quota_or_subscription_unavailable', 'provider_rate_limited', 'provider_network_unavailable', 'runtime_not_available', 'turn_timeout', 'agent_runtime_payload_budget_exceeded', 'agent_runtime_transport_failure']) {
  if (!Array.isArray(hardFailureInjectionRequirements.must_cover_failure_classes) || !hardFailureInjectionRequirements.must_cover_failure_classes.includes(failureClass)) {
    violations.push({ kind: 'agent_runtime_hard_failure_injection_class_missing', failure_class: failureClass, path: turnOutcomeContractPath });
  }
}
if (!engineScorecardContract.evidence_inputs || engineScorecardContract.evidence_inputs.hard_failure_injection !== 'core/local/artifacts/agent_runtime_hard_failure_injection_eval_current.json') {
  violations.push({ kind: 'engine_scorecard_hard_failure_evidence_missing', path: engineScorecardContractPath });
}
if (!engineScorecardContract.evidence_inputs || engineScorecardContract.evidence_inputs.native_transport_probe !== 'core/local/artifacts/agent_runtime_native_transport_probe_current.json') {
  violations.push({ kind: 'engine_scorecard_native_transport_probe_evidence_missing', path: engineScorecardContractPath });
}
for (const mode of ['prompt_text_compat', 'structured_json', 'native_session_bridge']) {
  if (!Array.isArray(structuredTransportContract.allowed_transport_modes) || !structuredTransportContract.allowed_transport_modes.includes(mode)) {
    violations.push({ kind: 'structured_transport_mode_missing', mode, path: structuredTransportContractPath });
  }
  if (!structuredTransportContract.mode_definitions || !structuredTransportContract.mode_definitions[mode]) {
    violations.push({ kind: 'structured_transport_mode_definition_missing', mode, path: structuredTransportContractPath });
  }
}
for (const section of ['context_pack', 'user_message', 'tool_grants', 'approval_requests', 'activity_events', 'assistant_output', 'receipts']) {
  if (!Array.isArray(structuredTransportContract.canonical_turn_schema?.required_fields) || !structuredTransportContract.canonical_turn_schema.required_fields.includes(section)) {
    violations.push({ kind: 'structured_transport_required_section_missing', section, path: structuredTransportContractPath });
  }
}
if (contextAuthorityBoundaryContract.canonical_authority?.owner !== 'kernel.layer2.memory') violations.push({ kind: 'context_authority_owner_not_kernel', path: contextAuthorityBoundaryContractPath });
if (contextAuthorityBoundaryContract.allowed_non_authority_roles?.transitional_gateway_cache?.canonical_authority !== false) violations.push({ kind: 'context_gateway_cache_not_non_authority', path: contextAuthorityBoundaryContractPath });
if (contextAuthorityBoundaryContract.legacy_context_systems?.['core/layer0/ops/src/memory/stacks']?.runtime_engine_context_authority_allowed !== false) violations.push({ kind: 'legacy_context_stacks_not_denied_for_runtime_engine_context', path: contextAuthorityBoundaryContractPath });
if (adapterContracts.engine_registry !== registryPath) violations.push({ kind: 'adapter_contract_registry_mismatch', path: adapterContractsPath });
if (adapterContracts.public_socket_contract !== socketPath) violations.push({ kind: 'adapter_contract_public_socket_mismatch', path: adapterContractsPath });
if (contextPackContract.hierarchy?.fanout_target !== 7) violations.push({ kind: 'context_pack_fanout_target_wrong', path: contextPackContractPath });
if (contextPackContract.hierarchy?.hot_tail_count !== 4) violations.push({ kind: 'context_pack_hot_tail_count_wrong', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_must_build_or_normalize_pack) violations.push({ kind: 'context_pack_gateway_builder_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_context_store_required_until_kernel_materializer_route_live) violations.push({ kind: 'context_pack_gateway_context_store_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_must_append_user_turn_atoms) violations.push({ kind: 'context_pack_user_turn_atom_append_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_must_append_assistant_turn_atoms) violations.push({ kind: 'context_pack_assistant_turn_atom_append_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_must_materialize_pack_from_stored_atoms_only_when_kernel_unavailable) violations.push({ kind: 'context_pack_store_materialization_fallback_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.kernel_materializer_cli_required) violations.push({ kind: 'context_pack_kernel_materializer_cli_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.production_requires_kernel_materializer) violations.push({ kind: 'context_pack_production_kernel_materializer_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.workspace_dev_may_use_kernel_materializer_cargo_auto) violations.push({ kind: 'context_pack_workspace_cargo_auto_not_declared', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_must_prefer_kernel_materializer_when_available) violations.push({ kind: 'context_pack_kernel_materializer_preference_not_required', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_store_fallback_allowed_when_kernel_materializer_unavailable) violations.push({ kind: 'context_pack_gateway_store_fallback_not_declared', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.gateway_context_store_is_transitional) violations.push({ kind: 'context_pack_gateway_store_not_transitional', path: contextPackContractPath });
if (!contextPackContract.live_slice_policy?.adapter_must_inject_pack_before_current_turn) violations.push({ kind: 'context_pack_adapter_injection_not_required', path: contextPackContractPath });
if (!contextPackContract.engine_switch_continuity_rule?.same_session_same_pack_shape_for_all_engines) violations.push({ kind: 'context_pack_engine_switch_continuity_missing', path: contextPackContractPath });
if (!contextPackContract.engine_switch_continuity_rule?.context_continuity_eval_required) violations.push({ kind: 'context_pack_continuity_eval_not_required', path: contextPackContractPath });
if (!contextPackContract.universal_tool_policy?.attached_to_context_pack || !contextPackContract.universal_tool_policy?.gateway_validation_required) violations.push({ kind: 'context_pack_universal_tool_policy_missing', path: contextPackContractPath });
if (!Array.isArray(contextPackContract.required_gateway_enriched_pack_fields) || !contextPackContract.required_gateway_enriched_pack_fields.includes('universal_tool_grants')) violations.push({ kind: 'context_pack_gateway_enriched_universal_tool_grants_missing', path: contextPackContractPath });
if (universalToolsContract.type !== 'agent_runtime_universal_tools_contract') violations.push({ kind: 'universal_tools_contract_type_wrong', path: universalToolsContractPath });
if (!universalToolsContract.authority_model?.engines_may_propose_tool_calls || universalToolsContract.authority_model?.engines_may_not_execute_universal_tools_directly !== true) violations.push({ kind: 'universal_tools_authority_model_wrong', path: universalToolsContractPath });
if (!universalToolsContract.authority_model?.permission_gatekeeper_is_runtime_neutral || !universalToolsContract.permission_policy?.framework_independent) violations.push({ kind: 'universal_tool_permission_policy_missing', path: universalToolsContractPath });
for (const decision of ['allow_once', 'deny', 'always_allow_tool_call']) {
  if (!Array.isArray(universalToolsContract.permission_policy?.decisions) || !universalToolsContract.permission_policy.decisions.includes(decision)) violations.push({ kind: 'universal_tool_permission_decision_missing', decision, path: universalToolsContractPath });
}
for (const toolId of ['conversation.read', 'memory.read', 'memory.write_propose', 'artifact.read', 'artifact.create_propose', 'permission.request']) {
  if (!Array.isArray(universalToolsContract.tools) || !universalToolsContract.tools.some((row) => row.tool_id === toolId)) violations.push({ kind: 'universal_tool_missing', tool_id: toolId, path: universalToolsContractPath });
}
const publicSocketInvariant = adapterContracts.canonical_public_socket_invariant || {};
for (const [field, expected] of Object.entries({
  single_gateway_socket_schema: true,
  runtime_specific_public_schemas_allowed: false,
  public_events_are_runtime_neutral: true,
  engine_specific_payloads_private_to_adapter: true,
  adapter_must_normalize_to_public_events: true,
  shell_sees_engine_projection_not_private_schema: true,
})) {
  if (publicSocketInvariant[field] !== expected) violations.push({ kind: 'adapter_public_socket_invariant_wrong', field, expected, actual: publicSocketInvariant[field] });
}
const discoveryPolicy = adapterContracts.discovery_policy || {};
for (const [field, expected] of Object.entries({
  defaults_are_probes_not_truth: true,
  custom_location_allowed_for_external_engines: true,
  user_override_wins: true,
  config_value_wins_over_environment: true,
  environment_wins_over_path_and_defaults: true,
  path_and_default_locations_are_lower_authority_probes: true,
  socket_urls_must_be_health_checked_before_available: true,
  missing_engines_project_download_or_config_action: true,
})) {
  if (discoveryPolicy[field] !== expected) violations.push({ kind: 'adapter_discovery_policy_missing', field, expected, actual: discoveryPolicy[field] });
}
for (const source of ['user_override', 'config_value', 'environment_variable', 'path_discovery', 'default_location_probe', 'default_url_probe', 'missing_installable']) {
  if (!Array.isArray(discoveryPolicy.authority_order) || !discoveryPolicy.authority_order.includes(source)) {
    violations.push({ kind: 'adapter_discovery_authority_source_missing', source });
  }
}
const kernelDecouplingInvariant = adapterContracts.kernel_decoupling_invariant || {};
for (const field of [
  'kernel_core_never_imports_engine_adapters',
  'kernel_core_never_selects_engine_id',
  'kernel_receipts_may_record_engine_id',
  'kernel_receipts_may_not_embed_private_adapter_payloads',
  'orchestration_is_native_engine_not_kernel_dependency',
  'engine_outputs_are_intents_effects_or_receipt_refs',
  'durable_effects_remain_kernel_or_gateway_policy_authorized',
]) {
  if (kernelDecouplingInvariant[field] !== true) violations.push({ kind: 'kernel_decoupling_invariant_missing', field });
}

const requiredMethods = Array.isArray(registry.required_engine_interface) ? registry.required_engine_interface : [];
for (const method of ['health_check', 'start_session', 'submit_turn', 'stream_events', 'cancel_turn', 'collect_artifacts', 'emit_receipts']) {
  if (!requiredMethods.includes(method)) violations.push({ kind: 'required_method_missing', method, path: registryPath });
}

const engines = Array.isArray(registry.engines) ? registry.engines : [];
const ids = new Set<string>();
const allowedTransportModes = new Set<string>(
  Array.isArray(structuredTransportContract.allowed_transport_modes)
    ? structuredTransportContract.allowed_transport_modes.map((mode) => String(mode))
    : [],
);
const allowedTransportTargets = new Set<string>(
  Array.isArray(structuredTransportContract.allowed_transport_targets)
    ? structuredTransportContract.allowed_transport_targets.map((target) => String(target))
    : Array.from(allowedTransportModes),
);
for (const engine of engines) {
  const id = String(engine.engine_id || '');
  if (!id) violations.push({ kind: 'engine_id_missing', engine });
  if (ids.has(id)) violations.push({ kind: 'engine_id_duplicate', engine_id: id });
  ids.add(id);
  for (const field of ['display_name', 'engine_kind', 'implementation_path', 'transport_kind', 'status', 'health_contract', 'adapter_boundary']) {
    if (!engine[field]) violations.push({ kind: 'engine_field_missing', engine_id: id, field });
  }
  if (!Array.isArray(engine.capabilities) || engine.capabilities.length === 0) violations.push({ kind: 'engine_capabilities_missing', engine_id: id });
  const engineMode = String(engine.context_transport_mode || '');
  const engineTarget = String(engine.structured_transport_target || '');
  if (!allowedTransportModes.has(engineMode)) violations.push({ kind: 'engine_context_transport_mode_invalid', engine_id: id, mode: engineMode });
  if (engineTarget && !allowedTransportTargets.has(engineTarget)) violations.push({ kind: 'engine_structured_transport_target_invalid', engine_id: id, target: engineTarget });
  if (!Array.isArray(engine.authority_constraints) || engine.authority_constraints.length === 0) violations.push({ kind: 'engine_authority_constraints_missing', engine_id: id });
  if (!engine.discovery || typeof engine.discovery !== 'object') violations.push({ kind: 'engine_discovery_missing', engine_id: id });
  if (id !== 'infring_native' && engine.discovery?.custom_location_allowed !== true) violations.push({ kind: 'engine_custom_location_not_allowed', engine_id: id });
  if (id !== 'infring_native' && (!Array.isArray(engine.discovery?.env_vars) || engine.discovery.env_vars.length === 0)) violations.push({ kind: 'engine_discovery_env_vars_missing', engine_id: id });
  if (id !== 'infring_native') {
    if (!engine.install) violations.push({ kind: 'external_engine_install_metadata_missing', engine_id: id });
    if (engine.install && !engine.install.download_action_ref) violations.push({ kind: 'external_engine_download_action_ref_missing', engine_id: id });
  }
  if (['openclaw', 'hermes_agent'].includes(id)) {
    if (!engine.reference_repo_path || !exists(engine.reference_repo_path)) {
      violations.push({ kind: 'external_engine_reference_repo_missing', engine_id: id, path: engine.reference_repo_path || null });
    }
    if (!engine.private_schema_contract || !exists(engine.private_schema_contract)) {
      violations.push({ kind: 'external_engine_private_schema_contract_missing', engine_id: id, path: engine.private_schema_contract || null });
    } else {
      const privateSchema = readJson(engine.private_schema_contract);
      if (privateSchema.type !== 'agent_runtime_private_schema_contract') violations.push({ kind: 'external_engine_private_schema_type_wrong', engine_id: id, path: engine.private_schema_contract });
      if (privateSchema.engine_id !== id) violations.push({ kind: 'external_engine_private_schema_id_mismatch', engine_id: id, schema_engine_id: privateSchema.engine_id, path: engine.private_schema_contract });
      if (privateSchema.public_socket_contract !== socketPath) violations.push({ kind: 'external_engine_private_schema_socket_mismatch', engine_id: id, path: engine.private_schema_contract });
      if (privateSchema.private_schema_scope?.public_socket_schema_fork_allowed !== false) violations.push({ kind: 'external_engine_private_schema_allows_public_fork', engine_id: id, path: engine.private_schema_contract });
      if (privateSchema.private_schema_scope?.shell_direct_access_allowed !== false) violations.push({ kind: 'external_engine_private_schema_allows_shell_direct', engine_id: id, path: engine.private_schema_contract });
      if (!Array.isArray(privateSchema.discovery_schema?.health_probe_candidates) || privateSchema.discovery_schema.health_probe_candidates.length === 0) {
        violations.push({ kind: 'external_engine_private_schema_health_probes_missing', engine_id: id, path: engine.private_schema_contract });
      }
      if (!Array.isArray(privateSchema.structured_turn_mapping?.required_normalized_outputs) || !privateSchema.structured_turn_mapping.required_normalized_outputs.includes('turn.completed')) {
        violations.push({ kind: 'external_engine_private_schema_turn_mapping_incomplete', engine_id: id, path: engine.private_schema_contract });
      }
      if (id === 'hermes_agent') {
        if (!Array.isArray(privateSchema.structured_turn_mapping?.turn_submission_candidates) || !privateSchema.structured_turn_mapping.turn_submission_candidates.some((candidate: any) => candidate?.kind === 'safe_cli_oneshot_bridge' && candidate?.status === 'active_safe_non_mutating_bridge')) {
          violations.push({ kind: 'hermes_private_schema_safe_bridge_missing', engine_id: id, path: engine.private_schema_contract });
        }
        if (privateSchema.approval_and_tool_policy?.hermes_oneshot_auto_bypasses_native_approvals !== true) {
          violations.push({ kind: 'hermes_private_schema_oneshot_approval_risk_missing', engine_id: id, path: engine.private_schema_contract });
        }
        if (privateSchema.approval_and_tool_policy?.safe_cli_bridge_forbids_file_terminal_toolsets !== true) {
          violations.push({ kind: 'hermes_private_schema_safe_toolset_fuse_missing', engine_id: id, path: engine.private_schema_contract });
        }
        if (privateSchema.approval_and_tool_policy?.mutating_tool_bridge_status !== 'blocked_until_mediated_approval_bridge') {
          violations.push({ kind: 'hermes_private_schema_mutating_bridge_status_wrong', engine_id: id, path: engine.private_schema_contract });
        }
        if (privateSchema.discovery_schema?.provider_readiness_probe?.auth_required_status !== 'auth_required' || privateSchema.discovery_schema?.provider_readiness_probe?.selectable_when_auth_required !== false) {
          violations.push({ kind: 'hermes_private_schema_provider_readiness_probe_missing', engine_id: id, path: engine.private_schema_contract });
        }
        const defaultPaths = Array.isArray(privateSchema.discovery_schema?.default_paths) ? privateSchema.discovery_schema.default_paths : [];
        if (defaultPaths.some((row: string) => String(row).includes('.infring/external_runtimes'))) {
          violations.push({ kind: 'hermes_private_schema_managed_path_in_default_paths', engine_id: id, path: engine.private_schema_contract });
        }
        if (!Array.isArray(privateSchema.discovery_schema?.infring_managed_override_paths) || !privateSchema.discovery_schema.infring_managed_override_paths.some((row: string) => String(row).includes('.infring/external_runtimes/hermes_agent'))) {
          violations.push({ kind: 'hermes_private_schema_managed_override_path_missing', engine_id: id, path: engine.private_schema_contract });
        }
      }
    }
  }
  if (id === 'openclaw') {
    const managedWorkspace = engine.discovery?.managed_workspace_policy?.infring_managed_default_workspace || '';
    if (managedWorkspace === '~/.openclaw/workspace' || !String(managedWorkspace).includes('.infring/external_runtimes/openclaw')) {
      violations.push({ kind: 'openclaw_managed_workspace_not_isolated_from_infring_workspace', engine_id: id, managed_workspace: managedWorkspace });
    }
    if (engine.discovery?.managed_workspace_policy?.upstream_default_preserved_for_new_openclaw_instances !== true) {
      violations.push({ kind: 'openclaw_upstream_default_workspace_not_preserved', engine_id: id });
    }
  }
  if (['codex_cli', 'claude_code', 'grok_code', 'opencode'].includes(id)) {
    const install = engine.install || {};
    const commandLineInstall = install.command_line_install || {};
    if (install.preferred_install_method !== 'command_line') violations.push({ kind: 'engine_install_not_command_line', engine_id: id });
    if (commandLineInstall.permission_mode !== 'allowed') violations.push({ kind: 'engine_install_permission_not_allowed', engine_id: id });
    if (!Array.isArray(commandLineInstall.commands) || commandLineInstall.commands.length === 0) violations.push({ kind: 'engine_install_commands_missing', engine_id: id });
  }
}
if (!ids.has('infring_native')) violations.push({ kind: 'infring_native_missing', path: registryPath });
if (!ids.has('codex_cli')) violations.push({ kind: 'first_external_adapter_missing', path: registryPath });
if (!ids.has('claude_code')) violations.push({ kind: 'claude_code_missing', path: registryPath });
if (!ids.has('grok_code')) violations.push({ kind: 'grok_code_missing', path: registryPath });
if (!ids.has('opencode')) violations.push({ kind: 'opencode_missing', path: registryPath });
if (!ids.has('hermes_agent')) violations.push({ kind: 'hermes_agent_missing', path: registryPath });

const adapterRows = Array.isArray(adapterContracts.adapter_contracts) ? adapterContracts.adapter_contracts : [];
const adapterIds = new Set<string>();
const requiredAdapterFields = Array.isArray(adapterContracts.private_adapter_shape?.required_fields)
  ? adapterContracts.private_adapter_shape.required_fields
  : [];
for (const field of ['context_transport_mode', 'transport_migration_status', 'transitional_prompt_text_allowed', 'structured_transport_target']) {
  if (!requiredAdapterFields.includes(field)) violations.push({ kind: 'adapter_transport_required_field_missing', field, path: adapterContractsPath });
}
const transportMigrationRows: any[] = [];
for (const row of adapterRows) {
  const id = String(row.engine_id || '');
  if (!id) violations.push({ kind: 'adapter_contract_engine_id_missing', row });
  if (adapterIds.has(id)) violations.push({ kind: 'adapter_contract_duplicate', engine_id: id });
  adapterIds.add(id);
  for (const field of requiredAdapterFields) {
    if (row[field] === undefined || row[field] === null || row[field] === '') violations.push({ kind: 'adapter_contract_field_missing', engine_id: id, field });
  }
  if (row.public_socket_schema_fork_allowed !== false) violations.push({ kind: 'adapter_public_socket_fork_allowed', engine_id: id });
  if (row.raw_payload_default_allowed !== false) violations.push({ kind: 'adapter_raw_payload_default_allowed', engine_id: id });
  if (row.kernel_direct_access_allowed !== false) violations.push({ kind: 'adapter_kernel_direct_access_allowed', engine_id: id });
  if (row.shell_direct_access_allowed !== false) violations.push({ kind: 'adapter_shell_direct_access_allowed', engine_id: id });
  if (row.canonical_event_mapping_required !== true) violations.push({ kind: 'adapter_canonical_event_mapping_not_required', engine_id: id });
  if (row.health_projection_required !== true) violations.push({ kind: 'adapter_health_projection_not_required', engine_id: id });
  const mode = String(row.context_transport_mode || '');
  const target = String(row.structured_transport_target || '');
  transportMigrationRows.push({
    engine_id: id,
    context_transport_mode: mode,
    structured_transport_target: target,
    transport_migration_status: row.transport_migration_status || '',
    prompt_text_dependent: mode === 'prompt_text_compat',
    transitional_prompt_text_allowed: row.transitional_prompt_text_allowed === true,
  });
  if (!allowedTransportModes.has(mode)) violations.push({ kind: 'adapter_context_transport_mode_invalid', engine_id: id, mode });
  if (target && !allowedTransportTargets.has(target)) violations.push({ kind: 'adapter_structured_transport_target_invalid', engine_id: id, target });
  if (mode === 'prompt_text_compat') {
    if (row.transitional_prompt_text_allowed !== true) violations.push({ kind: 'adapter_prompt_text_not_marked_transitional', engine_id: id });
    if (target !== 'structured_json') violations.push({ kind: 'adapter_prompt_text_target_not_structured_json', engine_id: id, target });
    if (!row.prompt_text_compat_retirement_signal) violations.push({ kind: 'adapter_prompt_text_retirement_signal_missing', engine_id: id });
  } else if (row.transitional_prompt_text_allowed === true) {
    violations.push({ kind: 'adapter_prompt_text_allowed_without_prompt_text_mode', engine_id: id, mode });
  }
  if (id === 'infring_native' && row.engine_runtime_path !== 'orchestration/**') violations.push({ kind: 'native_engine_runtime_path_wrong', engine_id: id, actual: row.engine_runtime_path });
  if (!row.discovery || typeof row.discovery !== 'object') violations.push({ kind: 'adapter_discovery_missing', engine_id: id });
  if (id !== 'infring_native' && row.discovery?.custom_location_allowed !== true) violations.push({ kind: 'adapter_custom_location_not_allowed', engine_id: id });
  if (id === 'hermes_agent') {
    if (
      row.context_transport_mode !== 'bounded_cli_arg_context_envelope' ||
      row.structured_transport_target !== 'native_structured_context_api' ||
      row.transport_migration_status !== 'structured_source_safe_cli_arg_active'
    ) {
      violations.push({ kind: 'hermes_adapter_safe_bridge_transport_contract_wrong', engine_id: id, path: adapterContractsPath });
    }
    if (row.safe_bridge_policy?.bridge_mode !== 'cli_safe_oneshot' || row.safe_bridge_policy?.mutating_tool_bridge_ready !== false) {
      violations.push({ kind: 'hermes_adapter_safe_bridge_policy_missing', engine_id: id, path: adapterContractsPath });
    }
    if (!Array.isArray(row.safe_bridge_policy?.forbidden_toolsets) || !row.safe_bridge_policy.forbidden_toolsets.includes('terminal') || !row.safe_bridge_policy.forbidden_toolsets.includes('file')) {
      violations.push({ kind: 'hermes_adapter_safe_bridge_forbidden_toolsets_missing', engine_id: id, path: adapterContractsPath });
    }
    const defaultPaths = Array.isArray(row.discovery?.default_paths) ? row.discovery.default_paths : [];
    if (defaultPaths.some((item: string) => String(item).includes('.infring/external_runtimes'))) {
      violations.push({ kind: 'hermes_adapter_managed_path_in_default_paths', engine_id: id, path: adapterContractsPath });
    }
    if (!Array.isArray(row.discovery?.infring_managed_override_paths) || !row.discovery.infring_managed_override_paths.some((item: string) => String(item).includes('.infring/external_runtimes/hermes_agent'))) {
      violations.push({ kind: 'hermes_adapter_managed_override_path_missing', engine_id: id, path: adapterContractsPath });
    }
  }
  if (['openclaw', 'hermes_agent'].includes(id)) {
    if (!row.reference_repo_path || !exists(row.reference_repo_path)) violations.push({ kind: 'adapter_reference_repo_missing', engine_id: id, path: row.reference_repo_path || null });
    if (!row.private_schema_contract || !exists(row.private_schema_contract)) violations.push({ kind: 'adapter_private_schema_contract_missing', engine_id: id, path: row.private_schema_contract || null });
  }
  if (id !== 'infring_native' && (!Array.isArray(row.discovery?.authority_order) || !row.discovery.authority_order.includes('user_override') || !row.discovery.authority_order.includes('missing_installable'))) {
    violations.push({ kind: 'adapter_discovery_authority_order_incomplete', engine_id: id });
  }
  if (['codex_cli', 'claude_code', 'grok_code', 'opencode'].includes(id) && (!Array.isArray(row.discovery?.path_commands) || row.discovery.path_commands.length === 0)) violations.push({ kind: 'adapter_cli_path_commands_missing', engine_id: id });
  if (['openhands', 'openclaw', 'hermes_agent', 'openfang'].includes(id) && (!Array.isArray(row.discovery?.default_urls) || row.discovery.default_urls.length === 0)) violations.push({ kind: 'adapter_socket_default_urls_missing', engine_id: id });
}
for (const engine of engines) {
  const id = String(engine.engine_id || '');
  if (id && !adapterIds.has(id)) violations.push({ kind: 'adapter_contract_missing_for_engine', engine_id: id });
}
for (const id of adapterIds) {
  if (!ids.has(id)) violations.push({ kind: 'adapter_contract_unknown_engine', engine_id: id });
}

for (const event of socket.required_gateway_to_client_events || []) {
  if (!Array.isArray(event.required_fields) || !event.required_fields.includes('trace_id')) {
    violations.push({ kind: 'gateway_event_trace_id_missing', event_type: event.type });
  }
}
if (!Array.isArray(socket.required_gateway_to_client_events) || !socket.required_gateway_to_client_events.some((event) => event.type === 'permission.requested')) {
  violations.push({ kind: 'socket_permission_requested_event_missing', path: socketPath });
}

for (const forbidden of ['raw_tool_result', 'trace_body', 'workflow_graph', 'external_framework_transcript']) {
  if (!Array.isArray(socket.forbidden_default_payload_fields) || !socket.forbidden_default_payload_fields.includes(forbidden)) {
    violations.push({ kind: 'forbidden_payload_field_missing', field: forbidden });
  }
}

const routerPath = 'gateway/runtime/agent_runtime/agent_runtime_router.ts';
const nativePath = 'adapters/runtime/agent_engines/infring_native.ts';
const codexPath = 'adapters/runtime/agent_engines/codex_cli.ts';
const cliRuntimePath = 'adapters/runtime/agent_engines/cli_runtime_adapter.ts';
const claudePath = 'adapters/runtime/agent_engines/claude_code.ts';
const grokPath = 'adapters/runtime/agent_engines/grok_code.ts';
const openCodePath = 'adapters/runtime/agent_engines/opencode.ts';
const httpSocketRuntimePath = 'adapters/runtime/agent_engines/http_socket_runtime_adapter.ts';
const openclawPath = 'adapters/runtime/agent_engines/openclaw.ts';
const hermesAgentPath = 'adapters/runtime/agent_engines/hermes_agent.ts';
const liveTurnSmokePath = 'tests/tooling/scripts/ci/agent_runtime_cli_live_turn_smoke.ts';
const contextContinuityEvalPath = 'tests/tooling/scripts/ci/agent_runtime_context_continuity_eval.ts';
const routeApprovalLifecycleGuardPath = 'tests/tooling/scripts/ci/agent_runtime_route_approval_lifecycle_guard.ts';
const tracePath = 'gateway/runtime/agent_runtime/agent_runtime_trace_writer.ts';
const discoveryPath = 'adapters/runtime/agent_engines/discovery.ts';
const contextStorePath = 'gateway/runtime/agent_runtime/agent_runtime_context_store.ts';
const kernelContextBridgePath = 'gateway/runtime/agent_runtime/agent_runtime_kernel_context_bridge.ts';
const universalCoreToolsPath = 'gateway/runtime/agent_runtime/universal_core_tools.ts';
const approvalStorePath = 'gateway/runtime/agent_runtime/agent_runtime_approvals.ts';
const cliRuntimeAdapterPath = 'adapters/runtime/agent_engines/cli_runtime_adapter.ts';
const turnProjectionPath = 'gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts';
const contextPreviewPath = 'gateway/runtime/agent_runtime/agent_runtime_context_preview.ts';
const routeAssemblyPath = 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts';
const turnRoutesPath = 'gateway/runtime/agent_runtime/agent_runtime_turn_routes.ts';
const engineRoutesPath = 'gateway/runtime/agent_runtime/agent_runtime_engine_routes.ts';
const engineProjectionsPath = 'gateway/runtime/agent_runtime/agent_runtime_engine_projections.ts';
const contextPackPath = 'gateway/runtime/agent_runtime/agent_runtime_context_pack.ts';
const kernelContextMaterializerPath = 'core/layer2/memory/src/bin/agent_runtime_context_materializer.rs';
const dashboardPath = 'adapters/runtime/infring_dashboard.ts';
const chatSendPartPath = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/200-send-pipeline.part01.ts';
const chatRuntimeSelectorPartPath = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/090-init-hooks-and-shortcuts.part02.ts';
for (const rel of [routerPath, nativePath, codexPath, tracePath]) {
  if (!exists(rel)) violations.push({ kind: 'adapter_module_missing', path: rel });
}
if (!exists(discoveryPath)) violations.push({ kind: 'discovery_module_missing', path: discoveryPath });
if (!exists(contextStorePath)) violations.push({ kind: 'context_store_module_missing', path: contextStorePath });
if (!exists(kernelContextBridgePath)) violations.push({ kind: 'kernel_context_bridge_module_missing', path: kernelContextBridgePath });
if (!exists(universalCoreToolsPath)) violations.push({ kind: 'universal_core_tools_module_missing', path: universalCoreToolsPath });
if (!exists(approvalStorePath)) violations.push({ kind: 'approval_store_module_missing', path: approvalStorePath });
if (!exists(turnProjectionPath)) violations.push({ kind: 'turn_projection_module_missing', path: turnProjectionPath });
if (!exists(contextPreviewPath)) violations.push({ kind: 'context_preview_module_missing', path: contextPreviewPath });
if (!exists(routeAssemblyPath)) violations.push({ kind: 'route_assembly_module_missing', path: routeAssemblyPath });
if (!exists(turnRoutesPath)) violations.push({ kind: 'turn_routes_module_missing', path: turnRoutesPath });
if (!exists(engineRoutesPath)) violations.push({ kind: 'engine_routes_module_missing', path: engineRoutesPath });
if (!exists(engineProjectionsPath)) violations.push({ kind: 'engine_projections_module_missing', path: engineProjectionsPath });
if (!exists(contextPackPath)) violations.push({ kind: 'context_pack_module_missing', path: contextPackPath });
if (!exists(kernelContextMaterializerPath)) violations.push({ kind: 'kernel_context_materializer_bin_missing', path: kernelContextMaterializerPath });
if (!exists(cliRuntimePath)) violations.push({ kind: 'cli_runtime_module_missing', path: cliRuntimePath });
if (!exists(claudePath)) violations.push({ kind: 'claude_adapter_module_missing', path: claudePath });
if (!exists(grokPath)) violations.push({ kind: 'grok_adapter_module_missing', path: grokPath });
if (!exists(openCodePath)) violations.push({ kind: 'opencode_adapter_module_missing', path: openCodePath });
if (!exists(httpSocketRuntimePath)) violations.push({ kind: 'http_socket_runtime_module_missing', path: httpSocketRuntimePath });
if (!exists(openclawPath)) violations.push({ kind: 'openclaw_adapter_module_missing', path: openclawPath });
if (!exists(hermesAgentPath)) violations.push({ kind: 'hermes_agent_adapter_module_missing', path: hermesAgentPath });
if (!exists(liveTurnSmokePath)) violations.push({ kind: 'live_turn_smoke_script_missing', path: liveTurnSmokePath });
if (!exists(contextContinuityEvalPath)) violations.push({ kind: 'context_continuity_eval_script_missing', path: contextContinuityEvalPath });
if (exists(hermesAgentPath)) {
  const hermesAgentSource = fs.readFileSync(path.join(ROOT, hermesAgentPath), 'utf8');
  for (const marker of [
    'createHttpSocketRuntimeEngineAdapter',
    'createCliRuntimeEngineAdapter',
    'FORBIDDEN_SAFE_BRIDGE_TOOLSETS',
    'INFRING_AGENT_RUNTIME_HERMES_AGENT_LIVE',
    'INFRING_HERMES_AGENT_SAFE_TOOLSETS',
    'classifyHermesProviderReadiness',
    'agent_runtime_setup/hermes_agent_provider',
    "providerReadiness.provider_status === 'auth_required'",
    '--toolsets',
    '-z',
    'mutating_tool_bridge_ready: false',
  ]) {
    if (!hermesAgentSource.includes(marker)) violations.push({ kind: 'hermes_agent_safe_bridge_marker_missing', marker, path: hermesAgentPath });
  }
  for (const forbidden of ['terminal', 'file', 'debugging', 'hermes-cli', 'hermes-acp']) {
    if (!hermesAgentSource.includes(`'${forbidden}'`)) violations.push({ kind: 'hermes_agent_forbidden_toolset_not_guarded', forbidden, path: hermesAgentPath });
  }
}

if (exists(contextStorePath)) {
  const contextStoreSource = fs.readFileSync(path.join(ROOT, contextStorePath), 'utf8');
  for (const marker of [
    'TRANSITIONAL FALLBACK ONLY',
    'CONTEXT_STORE_AUTHORITY_CLASSIFICATION',
    "role: 'transitional_gateway_cache'",
    'canonical_authority: false',
    "canonical_authority_owner: 'kernel.layer2.memory'",
    "fallback_only_when: 'kernel_context_materializer_binary_unavailable'",
    "retirement_trigger: 'kernel_materializer_packaged_for_runtime'",
  ]) {
    if (!contextStoreSource.includes(marker)) violations.push({ kind: 'context_store_transitional_marker_missing', marker, path: contextStorePath });
  }
}
if (exists(kernelContextBridgePath)) {
  const kernelContextBridgeSource = fs.readFileSync(path.join(ROOT, kernelContextBridgePath), 'utf8');
  if (!kernelContextBridgeSource.includes('does not implement context semantics itself')) violations.push({ kind: 'kernel_context_bridge_semantics_boundary_missing', path: kernelContextBridgePath });
  if (!kernelContextBridgeSource.includes('INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_BIN') || !kernelContextBridgeSource.includes('INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO')) violations.push({ kind: 'kernel_context_bridge_discovery_controls_missing', path: kernelContextBridgePath });
  if (!kernelContextBridgeSource.includes("INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO || 'auto'")) violations.push({ kind: 'kernel_context_bridge_auto_cargo_default_missing', path: kernelContextBridgePath });
}
if (exists(cliRuntimePath)) {
  const cliSource = fs.readFileSync(path.join(ROOT, cliRuntimePath), 'utf8');
  if (!cliSource.includes('buildPromptWithContext') || !cliSource.includes('message.context_pack')) violations.push({ kind: 'cli_runtime_context_pack_injection_missing', path: cliRuntimePath });
  if (!cliSource.includes('renderUniversalToolGrantPromptSection') || !cliSource.includes('toolGrantSection')) violations.push({ kind: 'cli_runtime_universal_tool_prompt_missing', path: cliRuntimePath });
  if (!cliSource.includes('stableExternalSessionUuid')) violations.push({ kind: 'cli_runtime_stable_session_helper_missing', path: cliRuntimePath });
  for (const marker of structuredTransportContract.migration_policy?.prompt_text_compat_source_markers_required || []) {
    if (!cliSource.includes(marker)) violations.push({ kind: 'cli_runtime_prompt_text_transition_marker_missing', marker, path: cliRuntimePath });
  }
  if (!cliSource.includes('contextTransportMode') || !cliSource.includes('transportMigrationStatus')) {
    violations.push({ kind: 'cli_runtime_transport_mode_declaration_missing', path: cliRuntimePath });
  }
  for (const marker of ['resolveTurnTimeoutMs', 'cliRuntimeFailureText', 'timed_out', 'timeout_ms']) {
    if (!cliSource.includes(marker)) violations.push({ kind: 'cli_runtime_turn_outcome_marker_missing', marker, path: cliRuntimePath });
  }
  for (const marker of ['classifyCliRuntimeFailureCode', 'provider_quota_or_subscription_unavailable', 'provider_auth_required', 'provider_rate_limited']) {
    if (!cliSource.includes(marker)) violations.push({ kind: 'cli_runtime_provider_failure_classifier_missing', marker, path: cliRuntimePath });
  }
}
if (exists(codexPath)) {
  const codexSource = fs.readFileSync(path.join(ROOT, codexPath), 'utf8');
  if (!codexSource.includes('createCliRuntimeEngineAdapter')) violations.push({ kind: 'codex_cli_shared_adapter_missing', path: codexPath });
  if (codexSource.includes('--ephemeral')) violations.push({ kind: 'codex_cli_ephemeral_session_forbidden', path: codexPath });
}
if (exists(universalCoreToolsPath)) {
  const tools = require(path.join(ROOT, universalCoreToolsPath));
  if (!tools.UNIVERSAL_CORE_TOOL_SCOPE_CONTRACT || tools.UNIVERSAL_CORE_TOOL_SCOPE_CONTRACT.max_tool_count !== 6) {
    violations.push({ kind: 'universal_core_tool_scope_contract_missing_or_wrong', path: universalCoreToolsPath });
  }
  if (tools.UNIVERSAL_CORE_TOOL_SCOPE_CONTRACT?.workflow_tools_allowed !== false || tools.UNIVERSAL_CORE_TOOL_SCOPE_CONTRACT?.provider_specific_tools_allowed !== false) {
    violations.push({ kind: 'universal_core_tool_scope_allows_non_core_tools', path: universalCoreToolsPath });
  }
  const universalToolsSource = fs.readFileSync(path.join(ROOT, universalCoreToolsPath), 'utf8');
  for (const marker of ['UNIVERSAL_CORE_TOOL_SCOPE_CONTRACT', 'Keep this surface tiny, engine-agnostic, and proposal-first', 'Do not add workflow execution']) {
    if (!universalToolsSource.includes(marker)) violations.push({ kind: 'universal_core_tool_scope_marker_missing', marker, path: universalCoreToolsPath });
  }
  for (const exported of ['buildUniversalToolGrants', 'renderUniversalToolGrantPromptSection', 'normalizeUniversalToolProposal', 'evaluateUniversalToolPermission']) {
    if (typeof tools[exported] !== 'function') violations.push({ kind: 'universal_core_tools_export_missing', exported });
  }
  if (typeof tools.buildUniversalToolGrants === 'function' && typeof tools.normalizeUniversalToolProposal === 'function' && typeof tools.renderUniversalToolGrantPromptSection === 'function') {
    const grants = tools.buildUniversalToolGrants({ traceId: 'trace-tools', sessionId: 's', agentId: 'a', engineId: 'codex_cli' });
    const prompt = tools.renderUniversalToolGrantPromptSection(grants);
    const okProposal = tools.normalizeUniversalToolProposal({ type: 'infring_universal_tool_proposal', tool_id: 'memory.read', reason: 'need memory', arguments: { query: 'x' } }, grants);
    const badProposal = tools.normalizeUniversalToolProposal({ type: 'infring_universal_tool_proposal', tool_id: 'terminal.run', reason: 'bad', arguments: {} }, grants);
    const gatedProposal = tools.normalizeUniversalToolProposal({ type: 'infring_universal_tool_proposal', tool_id: 'memory.write_propose', reason: 'remember this', arguments: { summary: 'x' } }, grants);
    if (!Array.isArray(grants.tools) || grants.tools.length !== 6) violations.push({ kind: 'universal_core_tool_grant_count_wrong', count: grants.tools && grants.tools.length });
    if (!grants.permission_policy || grants.permission_policy.gatekeeper_kind !== 'user') violations.push({ kind: 'universal_core_tool_permission_policy_not_attached' });
    if (!prompt.includes('Universal InfRing core tools') || !prompt.includes('Proposal JSON shape') || !prompt.includes('memory.read')) violations.push({ kind: 'universal_core_tool_prompt_wrong' });
    if (!okProposal.ok || okProposal.type !== 'tool.proposed' || okProposal.engine_may_execute_directly !== false || okProposal.permission_status !== 'allowed_by_default_read_policy') violations.push({ kind: 'universal_core_tool_valid_proposal_not_normalized', result: okProposal });
    if (!gatedProposal.ok || gatedProposal.permission_requires_user_approval !== true || gatedProposal.permission_status !== 'requires_user_approval') violations.push({ kind: 'universal_core_tool_gated_proposal_not_marked', result: gatedProposal });
    if (badProposal.ok || badProposal.error_code !== 'universal_tool_not_granted') violations.push({ kind: 'universal_core_tool_unknown_proposal_not_denied', result: badProposal });
  }
}
if (!exists(cliRuntimeAdapterPath)) {
  violations.push({ kind: 'cli_runtime_adapter_missing', path: cliRuntimeAdapterPath });
} else {
  try {
    const cliAdapter = require(path.join(ROOT, cliRuntimeAdapterPath));
    if (typeof cliAdapter.parseCliActivityOutput !== 'function') {
      violations.push({ kind: 'cli_runtime_adapter_parse_export_missing', path: cliRuntimeAdapterPath });
    } else {
      const permissionText = 'Permission required: Create a simple standalone todo app in the workspace; Codex currently has read-only filesystem access.';
      const parsedPermission = cliAdapter.parseCliActivityOutput(
        permissionText,
        '',
        {
          message: {
            trace_id: 'trace-cli-permission-text-conformance',
            request_id: 'request-cli-permission-text-conformance',
            engine_id: 'codex_cli',
            session_id: 'session-cli-permission-text-conformance',
            turn_id: 'turn-cli-permission-text-conformance',
            working_directory: ROOT,
          },
        },
        'codex_cli',
      );
      const request = parsedPermission && parsedPermission.permission_request;
      if (
        !request ||
        request.type !== 'permission.requested' ||
        request.status !== 'paused_pending_approval' ||
        request.turn_status !== 'permission_required' ||
        request.resume_strategy !== 'grant_then_retry_next_turn' ||
        request.tool_id !== 'artifact.create_propose' ||
        !String(request.reason || '').includes('Permission required')
      ) {
        violations.push({
          kind: 'cli_runtime_permission_text_not_normalized_to_approval_pause',
          path: cliRuntimeAdapterPath,
          result: parsedPermission,
        });
      }
      const claudeWritePermissionText = 'Permission required: {"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_validation","name":"Write","input":{"file_path":"' + ROOT + '/tmp/approval_resume.txt","content":"approved after pause"}}]}}';
      const parsedClaudeWritePermission = cliAdapter.parseCliActivityOutput(
        claudeWritePermissionText,
        '',
        {
          message: {
            trace_id: 'trace-cli-claude-write-permission-conformance',
            request_id: 'request-cli-claude-write-permission-conformance',
            engine_id: 'claude_code',
            session_id: 'session-cli-claude-write-permission-conformance',
            turn_id: 'turn-cli-claude-write-permission-conformance',
            working_directory: ROOT,
          },
        },
        'claude_code',
      );
      const claudeWriteRequest = parsedClaudeWritePermission && parsedClaudeWritePermission.permission_request;
      if (
        !claudeWriteRequest ||
        claudeWriteRequest.type !== 'permission.requested' ||
        claudeWriteRequest.status !== 'paused_pending_approval' ||
        claudeWriteRequest.turn_status !== 'permission_required' ||
        claudeWriteRequest.resume_strategy !== 'gateway_apply_approved_effect' ||
        claudeWriteRequest.tool_id !== 'artifact.create_propose' ||
        claudeWriteRequest.proposal_arguments?.path !== 'tmp/approval_resume.txt' ||
        claudeWriteRequest.proposal_arguments?.content !== 'approved after pause'
      ) {
        violations.push({
          kind: 'cli_runtime_denied_write_json_not_normalized_to_artifact_proposal',
          path: cliRuntimeAdapterPath,
          result: parsedClaudeWritePermission,
        });
      }
    }
  } catch (error) {
    violations.push({ kind: 'cli_runtime_permission_text_probe_failed', path: cliRuntimeAdapterPath, error: String(error && error.message || error) });
  }
}

if (exists(approvalStorePath)) {
  const approvalRoot = path.join(ROOT, 'core', 'local', 'artifacts', 'agent-runtime-approval-conformance');
  const artifactRel = 'tmp/router-approval-lifecycle.txt';
  const artifactAbs = path.join(approvalRoot, artifactRel);
  try { fs.rmSync(approvalRoot, { recursive: true, force: true }); } catch {}
  try {
    const approvals = require(path.join(ROOT, approvalStorePath));
    if (typeof approvals.createAgentRuntimeApprovalStore !== 'function') {
      violations.push({ kind: 'approval_store_factory_missing', path: approvalStorePath });
    } else {
      const store = approvals.createAgentRuntimeApprovalStore({ root: approvalRoot });
      if (typeof store.recordAgentRuntimePendingApproval !== 'function' || typeof store.agentRuntimeApprovalDecisionProjection !== 'function') {
        violations.push({ kind: 'approval_store_lifecycle_methods_missing', path: approvalStorePath });
      } else {
        const pending = store.recordAgentRuntimePendingApproval({
          type: 'permission.requested',
          approval_id: 'approval_conformance_artifact_create',
          trace_id: 'trace-approval-conformance',
          request_id: 'request-approval-conformance',
          engine_id: 'codex_cli',
          session_id: 'session-approval-conformance',
          turn_id: 'turn-approval-conformance',
          tool_call_ref: 'tool-proposal/artifact.create_propose/trace-approval-conformance/turn-approval-conformance',
          tool_id: 'artifact.create_propose',
          capability: 'propose_artifact_create',
          reason: 'conformance approval lifecycle artifact write',
          argument_keys: ['path', 'mime_type', 'content'],
          proposal_arguments: {
            path: artifactRel,
            mime_type: 'text/plain',
            content: 'approval lifecycle conformance proof\n',
          },
          gatekeeper_kind: 'user',
          source: 'gateway_universal_tool_proposal_normalizer',
        });
        const decision = store.agentRuntimeApprovalDecisionProjection('trace-approval-conformance', 'approval_conformance_artifact_create', {
          decision: 'allow_once',
        });
        const wroteArtifact = fs.existsSync(artifactAbs) && fs.readFileSync(artifactAbs, 'utf8').includes('approval lifecycle conformance proof');
        if (
          !pending ||
          pending.turn_status !== 'permission_required' ||
          pending.status !== 'paused_pending_approval' ||
          !pending.resume_token ||
          !pending.proposal_arguments_ref ||
          pending.proposal_arguments ||
          !decision ||
          decision.ok !== true ||
          decision.pending_request_found !== true ||
          decision.resume_token !== pending.resume_token ||
          decision.durable_effect_executed !== true ||
          !decision.decision_receipt ||
          !decision.decision_receipt.receipt_hash ||
          !wroteArtifact
        ) {
          violations.push({ kind: 'approval_store_pause_decision_effect_lifecycle_broken', pending, decision, wroteArtifact });
        }
        const allowOnce = store.agentRuntimeApprovalDecisionProjection('trace-approval-conformance', 'approval_conformance_allow_once', {
          decision: 'allow_once',
          tool_id: 'memory.write_propose',
          engine_id: 'codex_cli',
          session_id: 'session-approval-conformance',
        });
        const allowOncePolicy = store.mergeAgentRuntimeApprovalPermissionPolicy({}, 'session-approval-conformance', 'codex_cli');
        const allowOncePolicyAfterConsume = store.mergeAgentRuntimeApprovalPermissionPolicy({}, 'session-approval-conformance', 'codex_cli');
        const alwaysPending = store.recordAgentRuntimePendingApproval({
          type: 'permission.requested',
          approval_id: 'approval_conformance_always_allow',
          trace_id: 'trace-approval-conformance',
          request_id: 'request-approval-conformance',
          engine_id: 'codex_cli',
          session_id: 'session-approval-conformance',
          turn_id: 'turn-approval-conformance-always',
          tool_call_ref: 'tool-proposal/memory.write_propose/trace-approval-conformance/turn-approval-conformance-always',
          tool_id: 'memory.write_propose',
          capability: 'propose_memory_write',
          reason: 'conformance always allow grant',
          argument_keys: ['summary'],
          proposal_arguments: { summary: 'always allow proof' },
          gatekeeper_kind: 'user',
          source: 'gateway_universal_tool_proposal_normalizer',
        });
        const alwaysDecision = store.agentRuntimeApprovalDecisionProjection('trace-approval-conformance', 'approval_conformance_always_allow', {
          decision: 'always_allow_tool_call',
        });
        const reloadedStore = approvals.createAgentRuntimeApprovalStore({ root: approvalRoot });
        const alwaysPolicy = reloadedStore.mergeAgentRuntimeApprovalPermissionPolicy({}, 'session-approval-conformance', 'codex_cli');
        const wrongSessionPolicy = reloadedStore.mergeAgentRuntimeApprovalPermissionPolicy({}, 'different-session', 'codex_cli');
        if (
          !allowOnce ||
          allowOnce.ok !== true ||
          !Array.isArray(allowOncePolicy.always_allowed_tool_calls) ||
          !allowOncePolicy.always_allowed_tool_calls.includes('memory.write_propose') ||
          (Array.isArray(allowOncePolicyAfterConsume.always_allowed_tool_calls) && allowOncePolicyAfterConsume.always_allowed_tool_calls.includes('memory.write_propose')) ||
          !alwaysPending ||
          !alwaysDecision ||
          alwaysDecision.ok !== true ||
          alwaysDecision.decision !== 'always_allow_tool_call' ||
          !Array.isArray(alwaysPolicy.always_allowed_tool_calls) ||
          !alwaysPolicy.always_allowed_tool_calls.includes('memory.write_propose') ||
          (Array.isArray(wrongSessionPolicy.always_allowed_tool_calls) && wrongSessionPolicy.always_allowed_tool_calls.includes('memory.write_propose'))
        ) {
          violations.push({
            kind: 'approval_store_always_allow_policy_lifecycle_broken',
            allowOnce,
            allowOncePolicy,
            allowOncePolicyAfterConsume,
            alwaysPending,
            alwaysDecision,
            alwaysPolicy,
            wrongSessionPolicy,
          });
        }
      }
    }
  } catch (error) {
    violations.push({ kind: 'approval_store_lifecycle_probe_failed', path: approvalStorePath, error: String(error && error.message || error) });
  } finally {
    try { fs.rmSync(approvalRoot, { recursive: true, force: true }); } catch {}
  }
}
if (!exists(routeApprovalLifecycleGuardPath)) violations.push({ kind: 'route_approval_lifecycle_guard_missing', path: routeApprovalLifecycleGuardPath });
if (exists(contextContinuityEvalPath)) {
  const evalSource = fs.readFileSync(path.join(ROOT, contextContinuityEvalPath), 'utf8');
  const activePromotionEngines = Array.isArray(registry.promotion_focus?.active_promotion_engines)
    ? registry.promotion_focus.active_promotion_engines
    : ['infring_native', 'codex_cli', 'claude_code'];
  for (const marker of ['materializeKernelAgentRuntimeContextPack', 'buildPromptWithContext', ...activePromotionEngines, 'brass-otter-713']) {
    if (!evalSource.includes(marker)) violations.push({ kind: 'context_continuity_eval_marker_missing', marker, path: contextContinuityEvalPath });
  }
}

if (exists(dashboardPath)) {
  const dashboardSource = fs.readFileSync(path.join(ROOT, dashboardPath), 'utf8');
  const turnProjectionSource = exists(turnProjectionPath) ? fs.readFileSync(path.join(ROOT, turnProjectionPath), 'utf8') : '';
  const contextPreviewSource = exists(contextPreviewPath) ? fs.readFileSync(path.join(ROOT, contextPreviewPath), 'utf8') : '';
  const routeAssemblySource = exists(routeAssemblyPath) ? fs.readFileSync(path.join(ROOT, routeAssemblyPath), 'utf8') : '';
  const turnRoutesSource = exists(turnRoutesPath) ? fs.readFileSync(path.join(ROOT, turnRoutesPath), 'utf8') : '';
  const engineRoutesSource = exists(engineRoutesPath) ? fs.readFileSync(path.join(ROOT, engineRoutesPath), 'utf8') : '';
  const engineProjectionsSource = exists(engineProjectionsPath) ? fs.readFileSync(path.join(ROOT, engineProjectionsPath), 'utf8') : '';
  const contextPackSource = exists(contextPackPath) ? fs.readFileSync(path.join(ROOT, contextPackPath), 'utf8') : '';
  const turnProjectionCombinedSource = `${dashboardSource}\n${turnProjectionSource}`;
  const contextProjectionCombinedSource = `${dashboardSource}\n${contextPreviewSource}`;
  const routeBoundarySource = `${dashboardSource}\n${routeAssemblySource}\n${turnRoutesSource}\n${engineRoutesSource}\n${engineProjectionsSource}`;
  const projectionBoundarySource = `${dashboardSource}\n${routeAssemblySource}\n${turnProjectionSource}\n${contextPreviewSource}\n${contextPackSource}`;
	  if (!routeBoundarySource.includes('/api/shell-socket/agent-runtime/turn')) violations.push({ kind: 'dashboard_agent_runtime_turn_route_missing', path: turnRoutesPath });
	  if (!routeBoundarySource.includes('agentRuntimeEngineInstallProjection')) violations.push({ kind: 'dashboard_agent_runtime_install_projection_missing', path: engineProjectionsPath });
	  if (!(routeBoundarySource.includes('agentRuntimeInstallMatch') || routeBoundarySource.includes('installMatch')) || !routeBoundarySource.includes('/install')) violations.push({ kind: 'dashboard_agent_runtime_install_route_missing', path: engineRoutesPath });
	  if (!projectionBoundarySource.includes('output_text') || !projectionBoundarySource.includes('display_text')) violations.push({ kind: 'dashboard_agent_runtime_formatted_output_projection_missing', path: turnProjectionPath });
	  if (!projectionBoundarySource.includes('buildAgentRuntimeContextPack') || !projectionBoundarySource.includes('AGENT_RUNTIME_CONTEXT_FANOUT_TARGET = 7')) violations.push({ kind: 'dashboard_agent_runtime_context_pack_builder_missing', path: contextPackPath });
	  if (!turnProjectionCombinedSource.includes('context_pack: contextPack')) violations.push({ kind: 'dashboard_agent_runtime_context_pack_not_submitted', path: turnProjectionPath });
	  if (!projectionBoundarySource.includes('ingestAgentRuntimeContextProjection') || !projectionBoundarySource.includes('materializeAgentRuntimeContextPack') || !projectionBoundarySource.includes('appendAgentRuntimeTurnAtoms')) violations.push({ kind: 'dashboard_agent_runtime_context_store_not_wired', path: routeAssemblyPath });
	  if (!contextProjectionCombinedSource.includes('materializeKernelAgentRuntimeContextPack') || !contextProjectionCombinedSource.includes('kernel_materializer_used')) violations.push({ kind: 'dashboard_agent_runtime_kernel_context_bridge_not_wired', path: contextPreviewPath });
	  for (const marker of ['failed_with_reason', 'timed_out_with_reason', 'status_code: 200']) {
	    if (!turnProjectionCombinedSource.includes(marker)) violations.push({ kind: 'dashboard_turn_outcome_projection_marker_missing', marker, path: turnProjectionPath });
	  }
	  for (const marker of ['agentRuntimePreTurnFailureProjection', 'classifyAgentRuntimePreTurnFailureCode', 'provider_quota_or_subscription_unavailable', 'provider_auth_required']) {
	    if (!turnProjectionCombinedSource.includes(marker)) violations.push({ kind: 'dashboard_pre_turn_failure_projection_marker_missing', marker, path: turnProjectionPath });
	  }
	  for (const factory of ['createCodexCliEngineAdapter', 'createClaudeCodeEngineAdapter', 'createGrokCodeEngineAdapter']) {
	    if (!dashboardSource.includes(factory)) violations.push({ kind: 'dashboard_agent_runtime_factory_missing', factory, path: dashboardPath });
	  }
	  for (const factory of ['createOpenClawEngineAdapter', 'createHermesAgentEngineAdapter']) {
	    if (!dashboardSource.includes(factory)) violations.push({ kind: 'dashboard_socket_runtime_factory_missing', factory, path: dashboardPath });
	  }
	  if (!dashboardSource.includes('createGatewayAgentRuntimeRouteAssembly') || !dashboardSource.includes('hermes_agent: createHermesAgentEngineAdapter')) violations.push({ kind: 'dashboard_hermes_live_dispatch_not_forwarded', path: dashboardPath });
}
if (exists(liveTurnSmokePath)) {
  const liveSmokeSource = fs.readFileSync(path.join(ROOT, liveTurnSmokePath), 'utf8');
  for (const marker of ['classifyTurnOutcome', 'timed_out_with_reason', 'failed_with_reason', 'silent_or_invalid', 'max_turn_seconds']) {
    if (!liveSmokeSource.includes(marker)) violations.push({ kind: 'live_turn_smoke_outcome_marker_missing', marker, path: liveTurnSmokePath });
  }
}
if (!exists(chatSendPartPath)) {
  violations.push({ kind: 'chat_send_part_missing', path: chatSendPartPath });
} else {
  const chatSendSource = fs.readFileSync(path.join(ROOT, chatSendPartPath), 'utf8');
  if (!chatSendSource.includes('/api/shell-socket/agent-runtime/turn')) violations.push({ kind: 'chat_send_runtime_turn_route_missing', path: chatSendPartPath });
  if (!chatSendSource.includes('isExternalAgentRuntimeEngineSelected')) violations.push({ kind: 'chat_send_external_runtime_guard_missing', path: chatSendPartPath });
  if (!chatSendSource.includes('!usesExternalRuntime')) violations.push({ kind: 'chat_send_native_model_preflight_not_bypassed_for_external_runtime', path: chatSendPartPath });
  if (!chatSendSource.includes('_sendAgentRuntimeSocketPayload')) violations.push({ kind: 'chat_send_runtime_socket_dispatch_missing', path: chatSendPartPath });
	  if (!chatSendSource.includes('display_text || res.output_text')) violations.push({ kind: 'chat_send_prefers_formatted_runtime_output_missing', path: chatSendPartPath });
	  if (!chatSendSource.includes('context_projection') || !chatSendSource.includes('rows: contextRows')) violations.push({ kind: 'chat_send_context_projection_missing', path: chatSendPartPath });
	  const runtimeDispatchBody = (chatSendSource.match(/async _sendAgentRuntimeSocketPayload[\s\S]+?\n    async _sendTerminalPayload/) || [''])[0];
  if (!runtimeDispatchBody.includes('isHtml: false') || !runtimeDispatchBody.includes('_typingVisual: false')) violations.push({ kind: 'chat_send_runtime_output_not_standard_markdown_message', path: chatSendPartPath });
  if (runtimeDispatchBody.includes('_queueFinalWordTypingRender')) violations.push({ kind: 'chat_send_runtime_output_uses_typewriter_renderer', path: chatSendPartPath });
}
if (!exists(chatRuntimeSelectorPartPath)) {
  violations.push({ kind: 'chat_runtime_selector_part_missing', path: chatRuntimeSelectorPartPath });
} else {
  const selectorSource = fs.readFileSync(path.join(ROOT, chatRuntimeSelectorPartPath), 'utf8');
  if (!selectorSource.includes("preferred === 'command_line'")) violations.push({ kind: 'chat_runtime_selector_cli_install_hint_not_terminal', path: chatRuntimeSelectorPartPath });
  if (!selectorSource.includes("url && preferred !== 'command_line'")) violations.push({ kind: 'chat_runtime_selector_browser_fallback_not_last_resort', path: chatRuntimeSelectorPartPath });
  if (!selectorSource.includes("['not_downloaded', 'not_configured', 'planned_adapter']")) violations.push({ kind: 'chat_runtime_selector_unavailable_statuses_not_explicit', path: chatRuntimeSelectorPartPath });
  if (!selectorSource.includes('installRuntimeEngine: function(row)')) violations.push({ kind: 'chat_runtime_selector_install_action_missing', path: chatRuntimeSelectorPartPath });
  if (!selectorSource.includes('/api/shell-socket/agent-runtime/engines/')) violations.push({ kind: 'chat_runtime_selector_install_route_missing', path: chatRuntimeSelectorPartPath });
}

if (exists(routerPath)) {
  const router = require(path.join(ROOT, routerPath));
  for (const exported of ['createAgentRuntimeRouter', 'loadAgentRuntimeEngineRegistry', 'normalizeGatewayEvent', 'hasForbiddenDefaultField']) {
    if (typeof router[exported] !== 'function') violations.push({ kind: 'router_export_missing', exported });
  }
  if (!router.ROUTER_SCOPE_CONTRACT || router.ROUTER_SCOPE_CONTRACT.router_routes_only !== true) {
    violations.push({ kind: 'router_scope_contract_export_missing_or_wrong', path: routerPath });
  }
  if (router.ROUTER_SCOPE_CONTRACT?.provider_specific_logic_belongs_in_adapter !== true || router.ROUTER_SCOPE_CONTRACT?.coordination_logic_belongs_outside_router !== true) {
    violations.push({ kind: 'router_scope_contract_boundary_missing', path: routerPath });
  }
  const routerSource = fs.readFileSync(path.join(ROOT, routerPath), 'utf8');
  for (const marker of ['ROUTER_SCOPE_CONTRACT', 'The router routes, normalizes, compacts, traces, and dispatches', 'Provider-specific behavior belongs in adapters']) {
    if (!routerSource.includes(marker)) violations.push({ kind: 'router_scope_marker_missing', marker, path: routerPath });
  }
  if (typeof router.normalizeGatewayEvent === 'function') {
    const replacement = router.normalizeGatewayEvent(
      { type: 'turn.complete', trace_id: 'trace-replaced', engine_id: 'infring_native', session_id: 's1' },
      { trace_id: 'trace-original', engine_id: 'infring_native', session_id: 's1' },
      'turn.complete',
    );
    if (replacement?.error_code !== 'agent_runtime_trace_id_replaced' || replacement?.trace_id !== 'trace-original') {
      violations.push({
        kind: 'router_trace_replacement_not_rejected',
        detail: 'normalizeGatewayEvent must reject adapter attempts to replace the canonical message trace_id.',
      });
    }
    const inherited = router.normalizeGatewayEvent(
      { type: 'turn.complete', engine_id: 'infring_native', session_id: 's1' },
      { trace_id: 'trace-original', engine_id: 'infring_native', session_id: 's1' },
      'turn.complete',
    );
    if (inherited?.trace_id !== 'trace-original' || inherited?.error_code) {
      violations.push({
        kind: 'router_trace_inheritance_broken',
        detail: 'normalizeGatewayEvent must preserve the canonical message trace_id when adapter events omit trace_id.',
      });
    }
    const proposed = router.normalizeGatewayEvent(
      { type: 'infring_universal_tool_proposal', tool_id: 'memory.read', reason: 'need continuity memory', arguments: { query: 'brass otter' } },
      {
        trace_id: 'trace-tools',
        request_id: 'request-tools',
        engine_id: 'codex_cli',
        session_id: 's1',
        turn_id: 't1',
        context_pack: {
          universal_tool_grants: {
            tools: [{ tool_id: 'memory.read' }],
          },
        },
      },
      'tool.proposed',
    );
    if (proposed?.type !== 'tool.proposed' || proposed?.tool_id !== 'memory.read' || proposed?.engine_may_execute_directly !== false || Array.isArray(proposed?.argument_keys) === false || proposed?.permission_status !== 'allowed_by_default_read_policy') {
      violations.push({ kind: 'router_universal_tool_proposal_not_normalized', result: proposed });
    }
    const approvalRequired = router.normalizeGatewayEvent(
      { type: 'infring_universal_tool_proposal', tool_id: 'memory.write_propose', reason: 'remember user preference', arguments: { summary: 'x' } },
      {
        trace_id: 'trace-tools',
        request_id: 'request-tools',
        engine_id: 'codex_cli',
        session_id: 's1',
        turn_id: 't1',
        context_pack: {
          universal_tool_grants: {
            permission_policy: { gatekeeper_kind: 'user', default_allow_read_tools: true, revoked_default_read_tools: [], always_allowed_tool_calls: [] },
            tools: [{ tool_id: 'memory.write_propose' }],
          },
        },
      },
      'tool.proposed',
    );
    if (approvalRequired?.permission_requires_user_approval !== true || approvalRequired?.permission_request?.type !== 'permission.requested' || !approvalRequired?.permission_request?.approval_route) {
      violations.push({ kind: 'router_universal_tool_permission_request_missing', result: approvalRequired });
    }
    if (
      approvalRequired?.permission_request?.turn_status !== 'permission_required' ||
      approvalRequired?.permission_request?.status !== 'paused_pending_approval' ||
      approvalRequired?.permission_request?.source !== 'gateway_universal_tool_proposal_normalizer' ||
      approvalRequired?.permission_request?.resume_strategy !== 'gateway_apply_approved_effect' ||
      approvalRequired?.permission_request?.proposal_arguments?.summary !== 'x'
    ) {
      violations.push({ kind: 'router_universal_tool_permission_pause_resume_payload_incomplete', result: approvalRequired });
    }
    const denied = router.normalizeGatewayEvent(
      { type: 'infring_universal_tool_proposal', tool_id: 'terminal.run', reason: 'bad', arguments: {} },
      {
        trace_id: 'trace-tools',
        request_id: 'request-tools',
        engine_id: 'codex_cli',
        session_id: 's1',
        turn_id: 't1',
        context_pack: {
          universal_tool_grants: {
            tools: [{ tool_id: 'memory.read' }],
          },
        },
      },
      'tool.proposed',
    );
    if (denied?.error_code !== 'universal_tool_not_granted') {
      violations.push({ kind: 'router_universal_tool_unknown_not_denied', result: denied });
    }
  }
}
if (exists(nativePath)) {
  const native = require(path.join(ROOT, nativePath));
  if (typeof native.createInfringNativeEngineAdapter !== 'function') violations.push({ kind: 'native_adapter_factory_missing' });
}
if (exists(codexPath)) {
  const codex = require(path.join(ROOT, codexPath));
  if (typeof codex.createCodexCliEngineAdapter !== 'function') violations.push({ kind: 'codex_adapter_factory_missing' });
  const codexSource = fs.readFileSync(path.join(ROOT, codexPath), 'utf8');
  if (!codexSource.includes('createCliRuntimeEngineAdapter')) violations.push({ kind: 'codex_adapter_shared_runtime_missing', path: codexPath });
}
if (exists(cliRuntimePath)) {
  const cliRuntime = require(path.join(ROOT, cliRuntimePath));
  if (typeof cliRuntime.createCliRuntimeEngineAdapter !== 'function') violations.push({ kind: 'cli_runtime_factory_missing' });
  if (typeof cliRuntime.stripTerminalControls !== 'function') violations.push({ kind: 'cli_runtime_terminal_control_stripper_missing' });
		  const cliSource = fs.readFileSync(path.join(ROOT, cliRuntimePath), 'utf8');
  if (!cliSource.includes('cleanDisplayString') || !cliSource.includes('output_text')) violations.push({ kind: 'cli_runtime_formatted_output_missing', path: cliRuntimePath });
  if (!cliSource.includes('permission_denials') || !cliSource.includes('external_cli_permission_denial_normalizer')) violations.push({ kind: 'cli_runtime_permission_denial_normalizer_missing', path: cliRuntimePath });
	  if (!cliSource.includes('cleanDisplayString') || !cliSource.includes('output_text') || !cliSource.includes('stripTerminalControls')) violations.push({ kind: 'cli_runtime_formatted_output_missing', path: cliRuntimePath });
	  if (typeof cliRuntime.buildPromptWithContext !== 'function') violations.push({ kind: 'cli_runtime_context_prompt_builder_missing', path: cliRuntimePath });
	  if (typeof cliRuntime.buildPromptWithContext === 'function') {
	    const prompt = cliRuntime.buildPromptWithContext({
	      type: 'agent_runtime_context_pack',
	      source_basis: 'core.layer2.memory.context_topology_projection',
	      source_authority: 'gateway_bounded_projection_pending_kernel_materializer_route',
	      session_id: 'session-ctx',
	      fanout_target: 7,
	      fragments: [
	        { kind: 'span', ref_id: 'span-1', level: 0, payload: { summary: 'prior todo context', coverage: { start_seq: 1, end_seq: 7 } } },
	        { kind: 'atom', ref_id: 'atom-8', level: 0, payload: { role: 'user', text_preview: 'previous user request', sequence_no: 8 } },
	      ],
	    }, 'current request');
	    if (!prompt.includes('Session continuity excerpt') || !prompt.includes('plain prior conversation context') || !prompt.includes('prior todo context') || !prompt.includes('Current user turn:') || !prompt.includes('current request')) {
	      violations.push({ kind: 'cli_runtime_context_prompt_builder_broken', path: cliRuntimePath });
	    }
	  }
	  if (typeof cliRuntime.stripTerminalControls === 'function') {
    const stripped = cliRuntime.stripTerminalControls('\u001b[32m```js\nconst x = 1;\n```\u001b[0m');
    if (stripped.includes('\u001b') || !stripped.includes('```js\nconst x = 1;\n```')) violations.push({ kind: 'cli_runtime_terminal_control_stripper_broken' });
  }
}
if (exists(claudePath)) {
  const claude = require(path.join(ROOT, claudePath));
  if (typeof claude.createClaudeCodeEngineAdapter !== 'function') violations.push({ kind: 'claude_adapter_factory_missing' });
}
if (exists(grokPath)) {
  const grok = require(path.join(ROOT, grokPath));
  if (typeof grok.createGrokCodeEngineAdapter !== 'function') violations.push({ kind: 'grok_adapter_factory_missing' });
}
if (exists(discoveryPath)) {
  const discovery = require(path.join(ROOT, discoveryPath));
  if (typeof discovery.resolveEngineDiscovery !== 'function') violations.push({ kind: 'discovery_resolver_missing' });
  if (typeof discovery.expandLocationTemplate !== 'function') violations.push({ kind: 'discovery_expand_template_missing' });
  if (typeof discovery.findCommandOnPath !== 'function') violations.push({ kind: 'discovery_path_probe_missing' });
  if (typeof discovery.resolveEngineDiscovery === 'function') {
    const row = adapterRows.find((entry) => entry.engine_id === 'codex_cli');
    const configured = discovery.resolveEngineDiscovery({ ...row, discovery: row?.discovery }, { command: '/tmp/codex-custom' });
    if (configured?.discovery_source !== 'user_override' || configured?.command !== '/tmp/codex-custom') violations.push({ kind: 'discovery_user_override_not_authoritative' });
    const envResolved = discovery.resolveEngineDiscovery({ ...row, discovery: row?.discovery }, { env: { PATH: '', INFRING_CODEX_CLI_PATH: '/tmp/codex-env' } });
    if (envResolved?.discovery_source !== 'environment_variable' || envResolved?.command !== '/tmp/codex-env') violations.push({ kind: 'discovery_env_var_not_authoritative' });
  }
}
if (exists(contextStorePath)) {
  const contextStore = require(path.join(ROOT, contextStorePath));
  for (const exported of ['ingestAgentRuntimeContextProjection', 'appendAgentRuntimeTurnAtoms', 'materializeAgentRuntimeContextPack']) {
    if (typeof contextStore[exported] !== 'function') violations.push({ kind: 'context_store_export_missing', exported, path: contextStorePath });
  }
  if (typeof contextStore.ingestAgentRuntimeContextProjection === 'function' && typeof contextStore.materializeAgentRuntimeContextPack === 'function') {
    const root = fs.mkdtempSync(path.join(require('node:os').tmpdir(), 'infring-context-store-'));
    contextStore.ingestAgentRuntimeContextProjection({
      root,
      sessionId: 'session-eval',
      agentId: 'agent-eval',
      projection: {
        rows: [
          { id: 'm1', role: 'user', text_preview: 'The project codename is brass otter.' },
          { id: 'm2', role: 'assistant', text_preview: 'I will remember brass otter as the project codename.' },
        ],
      },
    });
    contextStore.appendAgentRuntimeTurnAtoms({
      root,
      sessionId: 'session-eval',
      agentId: 'agent-eval',
      traceId: 'trace-eval',
      turnId: 'turn-eval',
      engineId: 'codex_cli',
      userText: 'Use the codename.',
      assistantText: 'Using brass otter.',
    });
    const pack = contextStore.materializeAgentRuntimeContextPack({ root, sessionId: 'session-eval', agentId: 'agent-eval' });
    const rendered = JSON.stringify(pack);
    if (pack?.fanout_target !== 7 || pack?.source_authority !== 'gateway_runtime_context_store_pending_kernel_materializer_route') violations.push({ kind: 'context_store_pack_metadata_wrong', path: contextStorePath });
    if (!rendered.includes('brass otter') || !Array.isArray(pack?.fragments) || pack.fragments.length === 0) violations.push({ kind: 'context_store_pack_missing_persisted_context', path: contextStorePath });
  }
}
if (exists(kernelContextBridgePath)) {
  const bridge = require(path.join(ROOT, kernelContextBridgePath));
  for (const exported of ['resolveKernelMaterializerCommand', 'materializeKernelAgentRuntimeContextPack']) {
    if (typeof bridge[exported] !== 'function') violations.push({ kind: 'kernel_context_bridge_export_missing', exported, path: kernelContextBridgePath });
  }
  if (typeof bridge.resolveKernelMaterializerCommand === 'function') {
    const previous = process.env.INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO;
    process.env.INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO = '1';
    const resolved = bridge.resolveKernelMaterializerCommand(ROOT);
    if (resolved?.mode !== 'cargo' || !Array.isArray(resolved?.args) || !resolved.args.includes('agent_runtime_context_materializer')) {
      violations.push({ kind: 'kernel_context_bridge_cargo_resolution_broken', path: kernelContextBridgePath });
    }
    if (previous === undefined) delete process.env.INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO;
    else process.env.INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO = previous;
  }
}
if (exists(kernelContextMaterializerPath)) {
  const rustSource = fs.readFileSync(path.join(ROOT, kernelContextMaterializerPath), 'utf8');
  for (const required of ['materialize_context_topology', 'append_context_atom', 'source_authority', 'kernel_materialize_context_topology_cli']) {
    if (!rustSource.includes(required)) violations.push({ kind: 'kernel_context_materializer_source_missing', required, path: kernelContextMaterializerPath });
  }
}
if (exists(tracePath)) {
  const trace = require(path.join(ROOT, tracePath));
  if (typeof trace.createAgentRuntimeTraceWriter !== 'function') violations.push({ kind: 'trace_writer_factory_missing' });
  if (typeof trace.compactEvent !== 'function') violations.push({ kind: 'trace_compact_event_missing' });
  if (typeof trace.compactEvent === 'function') {
    const compact = trace.compactEvent({
      trace_id: 'trace-agent-runtime-001',
      parent_span_id: 'span-parent',
      type: 'tool.completed',
      request_id: 'request-1',
      engine_id: 'infring_native',
      session_id: 'session-1',
      turn_id: 'turn-1',
      receipt_ref: 'receipt/ref',
      evidence_refs: ['evidence/ref'],
    });
    for (const field of ['schema_version', 'trace_id', 'span_id', 'parent_span_id', 'timestamp', 'source_domain', 'producer', 'authority_class', 'event_kind', 'subject', 'correlation', 'payload_schema', 'payload', 'evidence_refs', 'receipt_refs', 'severity', 'confidence']) {
      if (!compact || compact[field] == null) violations.push({ kind: 'trace_compact_universal_field_missing', field });
    }
    if (compact?.event_kind !== 'tool_call') violations.push({ kind: 'trace_compact_event_kind_wrong', event_kind: compact?.event_kind });
    if (compact?.correlation?.request_id !== 'request-1') violations.push({ kind: 'trace_compact_correlation_missing_request_id' });
    if (!Array.isArray(compact?.receipt_refs) || compact.receipt_refs[0] !== 'receipt/ref') violations.push({ kind: 'trace_compact_receipt_refs_missing' });
    if (!Array.isArray(compact?.evidence_refs) || compact.evidence_refs[0] !== 'evidence/ref') violations.push({ kind: 'trace_compact_evidence_refs_missing' });
  }
}

const payload = {
  ok: violations.length === 0,
  type: 'agent_runtime_engine_conformance_guard',
  generated_at: new Date().toISOString(),
  trace_id: `validation:agent-runtime:${Date.now()}`,
  registry_path: registryPath,
  socket_contract_path: socketPath,
  adapter_contracts_path: adapterContractsPath,
  context_pack_contract_path: contextPackContractPath,
  structured_transport_contract_path: structuredTransportContractPath,
  engine_count: engines.length,
  adapter_contract_count: Array.isArray(adapterContracts.adapter_contracts) ? adapterContracts.adapter_contracts.length : 0,
  transport_migration_report: {
    preferred_external_mode: 'structured_json',
    prompt_text_dependent_count: transportMigrationRows.filter((row) => row.prompt_text_dependent).length,
    structured_source_envelope_count: transportMigrationRows.filter((row) => (
      row.context_transport_mode === 'bounded_stdin_context_envelope' ||
      row.context_transport_mode === 'bounded_cli_arg_context_envelope'
    )).length,
    structured_target_count: transportMigrationRows.filter((row) => row.context_transport_mode === 'structured_json').length,
    native_bridge_count: transportMigrationRows.filter((row) => row.context_transport_mode === 'native_session_bridge').length,
    rows: transportMigrationRows,
  },
  violations,
};

fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
