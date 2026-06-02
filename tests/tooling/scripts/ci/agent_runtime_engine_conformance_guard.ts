#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
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
