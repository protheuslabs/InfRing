#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const SOCKET_CONTRACT = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_socket_contract.json');
const ROUTER_SCOPE_CONTRACT = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_router_scope_contract.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_shell_replaceability_guard_current.json');

const REQUIRED_CLIENT_EVENTS = new Set([
  'engine.list',
  'engine.health',
  'session.start',
  'turn.submit',
  'turn.cancel',
]);

const REQUIRED_GATEWAY_EVENTS = new Set([
  'engine.list.result',
  'engine.health.result',
  'session.started',
  'assistant.delta',
  'tool.proposed',
  'permission.requested',
  'tool.started',
  'tool.completed',
  'artifact.created',
  'receipt.created',
  'error',
  'heartbeat',
  'turn.complete',
]);

const REQUIRED_SOCKET_INVARIANTS = [
  'shell_to_gateway_only',
  'no_shell_to_adapter_direct_socket',
  'no_shell_to_engine_direct_socket',
  'no_shell_to_orchestration_direct_agent_turn',
  'no_shell_to_kernel_direct_agent_turn',
  'external_engines_enter_gateway_only',
  'gateway_remains_external_ambiguity_firewall',
  'dashboard_is_shell_not_gateway_host',
  'adapters_are_translator_only',
];

const REQUIRED_ROUTER_RESPONSIBILITIES = new Set([
  'load_engine_registry',
  'register_adapter',
  'select_adapter_by_engine_id',
  'validate_adapter_shape',
  'normalize_public_gateway_events',
  'reject_trace_id_replacement',
  'reject_forbidden_default_payload_fields',
  'compact_oversized_gateway_events',
  'write_compact_trace_events',
  'dispatch_adapter_method',
]);

const FORBIDDEN_ROUTER_RESPONSIBILITIES = new Set([
  'plan_workflows',
  'execute_workflows',
  'own_conversation_memory',
  'own_kernel_authority',
  'own_shell_state',
  'implement_provider_business_logic',
  'implement_research_tools',
  'execute_terminal_commands',
  'perform_direct_file_mutations',
  'make_policy_approval_decisions',
  'score_eval_results',
  'interpret_sentinel_findings',
]);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function clean(value, max = 400) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function listEventTypes(rows) {
  const out = [];
  const seen = new Set();
  for (const row of Array.isArray(rows) ? rows : []) {
    const type = clean(row && row.type, 120);
    if (!type || seen.has(type)) continue;
    seen.add(type);
    out.push(type);
  }
  return out;
}

function missingFrom(required, actual) {
  const actualSet = new Set(actual);
  return [...required].filter((item) => !actualSet.has(item));
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function main() {
  const socket = readJson(SOCKET_CONTRACT);
  const router = readJson(ROUTER_SCOPE_CONTRACT);
  const failures = [];

  const endpoint = socket.canonical_endpoint || {};
  if (clean(endpoint.owner_of_boundary) !== 'gateway') {
    failures.push({ kind: 'socket_boundary_owner_not_gateway', value: endpoint.owner_of_boundary || null });
  }
  if (clean(endpoint.canonical_route_pattern) !== '/ws/agent-runtime') {
    failures.push({ kind: 'canonical_socket_route_mismatch', value: endpoint.canonical_route_pattern || null });
  }
  if (clean(endpoint.target_domain) !== 'gateway.agent_runtime_router') {
    failures.push({ kind: 'socket_target_domain_mismatch', value: endpoint.target_domain || null });
  }
  if (!['contracted_not_live', 'live'].includes(clean(endpoint.implementation_status))) {
    failures.push({ kind: 'socket_implementation_status_unknown', value: endpoint.implementation_status || null });
  }

  const invariant = socket.gateway_only_invariant || {};
  for (const key of REQUIRED_SOCKET_INVARIANTS) {
    if (invariant[key] !== true) failures.push({ kind: 'gateway_only_invariant_not_true', invariant: key, value: invariant[key] ?? null });
  }
  if (clean(invariant.gateway_physical_domain) !== 'gateway/**') {
    failures.push({ kind: 'gateway_physical_domain_mismatch', value: invariant.gateway_physical_domain || null });
  }

  const clientEvents = listEventTypes(socket.required_client_to_gateway_events);
  const gatewayEvents = listEventTypes(socket.required_gateway_to_client_events);
  for (const event of missingFrom(REQUIRED_CLIENT_EVENTS, clientEvents)) failures.push({ kind: 'missing_client_to_gateway_event', event });
  for (const event of missingFrom(REQUIRED_GATEWAY_EVENTS, gatewayEvents)) failures.push({ kind: 'missing_gateway_to_client_event', event });

  const routerPath = clean(router.router_path, 240);
  const toolsPath = clean(router.universal_tools_path, 240);
  if (!routerPath.startsWith('gateway/runtime/agent_runtime/')) failures.push({ kind: 'router_not_in_gateway_domain', router_path: routerPath });
  if (!toolsPath.startsWith('gateway/runtime/agent_runtime/')) failures.push({ kind: 'universal_tools_not_in_gateway_domain', universal_tools_path: toolsPath });

  const allowedResponsibilities = new Set(Array.isArray(router.allowed_router_responsibilities) ? router.allowed_router_responsibilities : []);
  const forbiddenResponsibilities = new Set(Array.isArray(router.forbidden_router_responsibilities) ? router.forbidden_router_responsibilities : []);
  for (const item of REQUIRED_ROUTER_RESPONSIBILITIES) {
    if (!allowedResponsibilities.has(item)) failures.push({ kind: 'missing_allowed_router_responsibility', responsibility: item });
  }
  for (const item of FORBIDDEN_ROUTER_RESPONSIBILITIES) {
    if (!forbiddenResponsibilities.has(item)) failures.push({ kind: 'missing_forbidden_router_responsibility', responsibility: item });
  }

  const report = {
    ok: failures.length === 0,
    type: 'agent_runtime_shell_replaceability_guard',
    generated_at: new Date().toISOString(),
    policy: {
      shell_replaceability_requires_gateway_socket_contract: true,
      dashboard_must_not_be_gateway_host: true,
      adapters_are_translator_only: true,
      router_must_stay_thin_gateway_domain: true,
    },
    maturity: {
      canonical_socket_route: clean(endpoint.canonical_route_pattern),
      implementation_status: clean(endpoint.implementation_status),
      live_socket_required_for_completion: true,
      current_stage: clean(endpoint.implementation_status) === 'live'
        ? 'live_socket_gateway'
        : 'contract_enforced_socket_pending',
    },
    coverage: {
      client_to_gateway_events: clientEvents,
      gateway_to_client_events: gatewayEvents,
      router_path: routerPath,
      universal_tools_path: toolsPath,
    },
    failures,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
