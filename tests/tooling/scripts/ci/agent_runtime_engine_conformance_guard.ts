#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');
const ROOT = process.cwd();
const registryPath = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const socketPath = 'validation/conformance/contracts/agent_runtime_socket_contract.json';
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

if (registry.socket_contract !== socketPath) violations.push({ kind: 'registry_socket_contract_mismatch', path: registryPath });
if (socket.canonical_endpoint?.canonical_route_pattern !== '/ws/agent-runtime') violations.push({ kind: 'socket_route_not_canonical', path: socketPath });
if (!socket.trace_identity_rule?.trace_id_required_on_every_message) violations.push({ kind: 'trace_id_not_required', path: socketPath });
if (!socket.trace_identity_rule?.gateway_router_rejects_adapter_trace_id_replacement) violations.push({ kind: 'trace_replacement_rejection_not_required', path: socketPath });
if (!socket.kernel_authority_invariant?.durable_effects_require_kernel_or_gateway_policy) violations.push({ kind: 'kernel_authority_not_invariant', path: socketPath });

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
  if (id !== 'infring_native') {
    if (!engine.install) violations.push({ kind: 'external_engine_install_metadata_missing', engine_id: id });
    if (engine.install && !engine.install.download_action_ref) violations.push({ kind: 'external_engine_download_action_ref_missing', engine_id: id });
  }
}
if (!ids.has('infring_native')) violations.push({ kind: 'infring_native_missing', path: registryPath });
if (!ids.has('codex_cli')) violations.push({ kind: 'first_external_adapter_missing', path: registryPath });

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
for (const rel of [routerPath, nativePath, codexPath, tracePath]) {
  if (!exists(rel)) violations.push({ kind: 'adapter_module_missing', path: rel });
}

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
    });
    for (const field of ['trace_id', 'span_id', 'parent_span_id', 'source_domain', 'producer', 'authority_class', 'event_kind', 'subject', 'correlation']) {
      if (!compact || compact[field] == null) violations.push({ kind: 'trace_compact_universal_field_missing', field });
    }
    if (compact?.event_kind !== 'tool_call') violations.push({ kind: 'trace_compact_event_kind_wrong', event_kind: compact?.event_kind });
    if (compact?.correlation?.request_id !== 'request-1') violations.push({ kind: 'trace_compact_correlation_missing_request_id' });
  }
}

const payload = {
  ok: violations.length === 0,
  type: 'agent_runtime_engine_conformance_guard',
  generated_at: new Date().toISOString(),
  trace_id: `validation:agent-runtime:${Date.now()}`,
  registry_path: registryPath,
  socket_contract_path: socketPath,
  engine_count: engines.length,
  violations,
};

fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
