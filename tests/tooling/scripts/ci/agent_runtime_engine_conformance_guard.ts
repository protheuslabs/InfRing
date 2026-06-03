#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');
const ROOT = process.cwd();
const registryPath = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const socketPath = 'validation/conformance/contracts/agent_runtime_socket_contract.json';
const adapterContractsPath = 'validation/conformance/contracts/agent_runtime_adapter_contracts.json';
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

if (registry.socket_contract !== socketPath) violations.push({ kind: 'registry_socket_contract_mismatch', path: registryPath });
if (socket.canonical_endpoint?.canonical_route_pattern !== '/ws/agent-runtime') violations.push({ kind: 'socket_route_not_canonical', path: socketPath });
if (!socket.trace_identity_rule?.trace_id_required_on_every_message) violations.push({ kind: 'trace_id_not_required', path: socketPath });
if (!socket.trace_identity_rule?.gateway_router_rejects_adapter_trace_id_replacement) violations.push({ kind: 'trace_replacement_rejection_not_required', path: socketPath });
if (!socket.kernel_authority_invariant?.durable_effects_require_kernel_or_gateway_policy) violations.push({ kind: 'kernel_authority_not_invariant', path: socketPath });
if (registry.private_adapter_contracts !== adapterContractsPath) violations.push({ kind: 'registry_private_adapter_contracts_mismatch', path: registryPath });
if (socket.private_adapter_contracts !== adapterContractsPath) violations.push({ kind: 'socket_private_adapter_contracts_mismatch', path: socketPath });
if (adapterContracts.engine_registry !== registryPath) violations.push({ kind: 'adapter_contract_registry_mismatch', path: adapterContractsPath });
if (adapterContracts.public_socket_contract !== socketPath) violations.push({ kind: 'adapter_contract_public_socket_mismatch', path: adapterContractsPath });
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
for (const engine of engines) {
  const id = String(engine.engine_id || '');
  if (!id) violations.push({ kind: 'engine_id_missing', engine });
  if (ids.has(id)) violations.push({ kind: 'engine_id_duplicate', engine_id: id });
  ids.add(id);
  for (const field of ['display_name', 'engine_kind', 'implementation_path', 'transport_kind', 'status', 'health_contract', 'adapter_boundary']) {
    if (!engine[field]) violations.push({ kind: 'engine_field_missing', engine_id: id, field });
  }
  if (!Array.isArray(engine.capabilities) || engine.capabilities.length === 0) violations.push({ kind: 'engine_capabilities_missing', engine_id: id });
  if (!Array.isArray(engine.authority_constraints) || engine.authority_constraints.length === 0) violations.push({ kind: 'engine_authority_constraints_missing', engine_id: id });
  if (!engine.discovery || typeof engine.discovery !== 'object') violations.push({ kind: 'engine_discovery_missing', engine_id: id });
  if (id !== 'infring_native' && engine.discovery?.custom_location_allowed !== true) violations.push({ kind: 'engine_custom_location_not_allowed', engine_id: id });
  if (id !== 'infring_native' && (!Array.isArray(engine.discovery?.env_vars) || engine.discovery.env_vars.length === 0)) violations.push({ kind: 'engine_discovery_env_vars_missing', engine_id: id });
  if (id !== 'infring_native') {
    if (!engine.install) violations.push({ kind: 'external_engine_install_metadata_missing', engine_id: id });
    if (engine.install && !engine.install.download_action_ref) violations.push({ kind: 'external_engine_download_action_ref_missing', engine_id: id });
  }
}
if (!ids.has('infring_native')) violations.push({ kind: 'infring_native_missing', path: registryPath });
if (!ids.has('codex_cli')) violations.push({ kind: 'first_external_adapter_missing', path: registryPath });

const adapterRows = Array.isArray(adapterContracts.adapter_contracts) ? adapterContracts.adapter_contracts : [];
const adapterIds = new Set<string>();
const requiredAdapterFields = Array.isArray(adapterContracts.private_adapter_shape?.required_fields)
  ? adapterContracts.private_adapter_shape.required_fields
  : [];
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
  if (id === 'infring_native' && row.engine_runtime_path !== 'orchestration/**') violations.push({ kind: 'native_engine_runtime_path_wrong', engine_id: id, actual: row.engine_runtime_path });
  if (!row.discovery || typeof row.discovery !== 'object') violations.push({ kind: 'adapter_discovery_missing', engine_id: id });
  if (id !== 'infring_native' && row.discovery?.custom_location_allowed !== true) violations.push({ kind: 'adapter_custom_location_not_allowed', engine_id: id });
  if (id !== 'infring_native' && (!Array.isArray(row.discovery?.authority_order) || !row.discovery.authority_order.includes('user_override') || !row.discovery.authority_order.includes('missing_installable'))) {
    violations.push({ kind: 'adapter_discovery_authority_order_incomplete', engine_id: id });
  }
  if (['codex_cli', 'claude_code'].includes(id) && (!Array.isArray(row.discovery?.path_commands) || row.discovery.path_commands.length === 0)) violations.push({ kind: 'adapter_cli_path_commands_missing', engine_id: id });
  if (['openhands', 'openclaw', 'openfang'].includes(id) && (!Array.isArray(row.discovery?.default_urls) || row.discovery.default_urls.length === 0)) violations.push({ kind: 'adapter_socket_default_urls_missing', engine_id: id });
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

for (const forbidden of ['raw_tool_result', 'trace_body', 'workflow_graph', 'external_framework_transcript']) {
  if (!Array.isArray(socket.forbidden_default_payload_fields) || !socket.forbidden_default_payload_fields.includes(forbidden)) {
    violations.push({ kind: 'forbidden_payload_field_missing', field: forbidden });
  }
}

const routerPath = 'adapters/runtime/agent_engines/agent_runtime_router.ts';
const nativePath = 'adapters/runtime/agent_engines/infring_native.ts';
const codexPath = 'adapters/runtime/agent_engines/codex_cli.ts';
const tracePath = 'adapters/runtime/agent_engines/agent_runtime_trace_writer.ts';
const discoveryPath = 'adapters/runtime/agent_engines/discovery.ts';
for (const rel of [routerPath, nativePath, codexPath, tracePath]) {
  if (!exists(rel)) violations.push({ kind: 'adapter_module_missing', path: rel });
}
if (!exists(discoveryPath)) violations.push({ kind: 'discovery_module_missing', path: discoveryPath });

if (exists(routerPath)) {
  const router = require(path.join(ROOT, routerPath));
  for (const exported of ['createAgentRuntimeRouter', 'loadAgentRuntimeEngineRegistry', 'normalizeGatewayEvent', 'hasForbiddenDefaultField']) {
    if (typeof router[exported] !== 'function') violations.push({ kind: 'router_export_missing', exported });
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
  }
}
if (exists(nativePath)) {
  const native = require(path.join(ROOT, nativePath));
  if (typeof native.createInfringNativeEngineAdapter !== 'function') violations.push({ kind: 'native_adapter_factory_missing' });
}
if (exists(codexPath)) {
  const codex = require(path.join(ROOT, codexPath));
  if (typeof codex.createCodexCliEngineAdapter !== 'function') violations.push({ kind: 'codex_adapter_factory_missing' });
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
  engine_count: engines.length,
  adapter_contract_count: Array.isArray(adapterContracts.adapter_contracts) ? adapterContracts.adapter_contracts.length : 0,
  violations,
};

fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
