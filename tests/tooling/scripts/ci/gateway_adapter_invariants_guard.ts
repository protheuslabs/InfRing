#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/conformance/contracts/gateway_adapter_invariants_policy.json';
const defaultOutJson = 'core/local/artifacts/gateway_adapter_invariants_guard_current.json';
const defaultOutMarkdown = 'local/workspace/reports/GATEWAY_ADAPTER_INVARIANTS_GUARD_CURRENT.md';

function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const index = process.argv.indexOf(`--${name}`);
  if (index >= 0 && index + 1 < process.argv.length) return process.argv[index + 1];
  return fallback;
}

function read(rel: string): string {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function exists(rel: string): boolean {
  return fs.existsSync(path.join(ROOT, rel));
}

function json(rel: string): any {
  return JSON.parse(read(rel));
}

function walk(dir: string): string[] {
  const full = path.join(ROOT, dir);
  if (!fs.existsSync(full)) return [];
  const out: string[] = [];
  for (const name of fs.readdirSync(full)) {
    const rel = path.join(dir, name).replace(/\\/g, '/');
    const stat = fs.statSync(path.join(ROOT, rel));
    if (stat.isDirectory()) out.push(...walk(rel));
    else out.push(rel);
  }
  return out;
}

const violations: any[] = [];
const debt: any[] = [];
const policy = json(policyPath);

function push(kind: string, rel: string, detail: string): void {
  violations.push({ kind, path: rel, detail });
}

function noteDebt(kind: string, rel: string, detail: string): void {
  debt.push({ kind, path: rel, detail });
}

if (policy.type !== 'gateway_physical_domain_invariants_policy') {
  push('wrong_policy_type', policyPath, 'Gateway invariants policy must define physical-domain ownership, not gateway-adapter equivalence.');
}
if (policy.canonical_gateway_domain !== 'gateway/**') {
  push('missing_canonical_gateway_domain', policyPath, 'Gateway canonical implementation domain must be gateway/**.');
}
if (!String(policy.adapter_domain_rule || '').includes('translator-only')) {
  push('adapter_domain_rule_not_translator_only', policyPath, 'Adapters must be declared translator-only behind Gateway sockets.');
}

const requiredDocs = [
  {
    path: 'ARCHITECTURE.md',
    tokens: [
      'Gateway boundary membrane (`gateway/**`)',
      '`adapters/**` is translator-only',
      'The dashboard, CLI, SDK, and future shells must connect to the system through Gateway sockets.',
    ],
    forbidden: [
      '| Gateways | `adapters/**`',
    ],
  },
  {
    path: 'README.md',
    tokens: [
      '`gateway/**` is the canonical implementation path',
      '`adapters/**` is not a Gateway implementation domain',
      'the dashboard must remain a Shell surface that connects to Gateway sockets',
    ],
    forbidden: [
      'Gateway Layer: `adapters/**` path compatibility',
      '`adapters/**` is the implementation path',
    ],
  },
  {
    path: 'docs/workspace/gateway_ingress_egress_policy.md',
    tokens: [
      "Gateway is InfRing's skin.",
      'The canonical implementation domain for Gateway boundary behavior is `gateway/**`.',
      'New Gateway policy under `adapters/**` is forbidden.',
      'The dashboard is a Shell. It must connect through Gateway sockets',
    ],
    forbidden: [],
  },
];

for (const doc of requiredDocs) {
  const text = exists(doc.path) ? read(doc.path) : '';
  if (!text) {
    push('required_doc_missing', doc.path, 'Required Gateway physical-domain document is missing.');
    continue;
  }
  for (const token of doc.tokens) {
    if (!text.includes(token)) push('required_doc_token_missing', doc.path, token);
  }
  for (const token of doc.forbidden) {
    if (text.includes(token)) push('forbidden_legacy_doc_token_present', doc.path, token);
  }
}

const canonicalNormalizer = 'gateway/runtime/agent_runtime_input_normalizer.ts';
if (!exists(canonicalNormalizer)) {
  push('gateway_input_normalizer_missing', canonicalNormalizer, 'Gateway ingress normalization must live under gateway/**.');
}
const canonicalSocketModules = Array.isArray(policy.canonical_gateway_socket_modules)
  ? policy.canonical_gateway_socket_modules
  : [];
for (const rel of canonicalSocketModules) {
  if (!exists(rel)) push('canonical_gateway_socket_module_missing', rel, 'Declared Gateway socket module is missing.');
  if (!String(rel).startsWith('gateway/')) {
    push('canonical_gateway_socket_module_outside_gateway', policyPath, `${rel} must live under gateway/**.`);
  }
}
const canonicalAgentRuntimeModules = Array.isArray(policy.canonical_gateway_agent_runtime_modules)
  ? policy.canonical_gateway_agent_runtime_modules
  : [];
for (const rel of canonicalAgentRuntimeModules) {
  if (!exists(rel)) push('canonical_gateway_agent_runtime_module_missing', rel, 'Declared Gateway Agent Runtime module is missing.');
  if (!String(rel).startsWith('gateway/')) {
    push('canonical_gateway_agent_runtime_module_outside_gateway', policyPath, `${rel} must live under gateway/**.`);
  }
}

const declaredLegacyHosts = Array.isArray(policy.declared_legacy_compatibility_hosts)
  ? policy.declared_legacy_compatibility_hosts
  : [];
const declaredHostPaths = new Set<string>();
for (const host of declaredLegacyHosts) {
  const rel = String(host && host.path || '');
  if (!rel) {
    push('legacy_host_missing_path', policyPath, 'Every declared legacy Gateway host needs a path.');
    continue;
  }
  declaredHostPaths.add(rel);
  if (!exists(rel)) push('legacy_host_file_missing', rel, 'Declared legacy host path does not exist.');
  if (!String(host.retirement_todo || '').trim()) push('legacy_host_missing_retirement_todo', policyPath, rel);
  if (!String(host.allowed_until || '').trim()) push('legacy_host_missing_allowed_until', policyPath, rel);
  noteDebt('declared_legacy_gateway_host', rel, `retirement_todo=${host.retirement_todo || 'missing'} allowed_until=${host.allowed_until || 'missing'}`);
}

const adapterFiles = walk('adapters').filter((rel) => /\.(ts|js|json|md)$/.test(rel));
const gatewayPolicySignals = [
  /Gateway is InfRing['’]s skin/,
  /canonical implementation domain for Gateway/,
  /Gateway ingress normalization/,
  /Gateway-owned Agent Runtime input normalization/,
  /source_authority:\s*['"]gateway_agent_runtime_input_normalizer['"]/,
  /INFRING_AGENT_RUNTIME_LARGE_TEXT_ATTACHMENT_/,
  /function\s+shouldMaterialize.*LargeText/,
  /function\s+materialize.*LargeTextAttachment/,
];

for (const rel of adapterFiles) {
  const text = read(rel);
  const hasGatewayPolicySignal = gatewayPolicySignals.some((pattern) => pattern.test(text));
  if (!hasGatewayPolicySignal) continue;
  if (declaredHostPaths.has(rel)) {
    noteDebt('legacy_adapter_gateway_policy_signal', rel, 'Gateway policy signal remains in a declared legacy host.');
    continue;
  }
  push('gateway_policy_under_adapter_domain', rel, 'Gateway policy must live under gateway/** or be declared as expiring legacy host debt.');
}

const dashboardPath = 'adapters/runtime/infring_dashboard.ts';
if (exists(dashboardPath)) {
  const dashboard = read(dashboardPath);
  if (!dashboard.includes("require('../../gateway/runtime/sockets/agent_ws/agent_ws_bridge.ts')")) {
    push('dashboard_not_using_gateway_agent_ws_bridge', dashboardPath, 'Legacy dashboard host must delegate Agent WebSocket bridge behavior to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/agent_runtime_router.ts')")) {
    push('dashboard_not_using_gateway_agent_runtime_router', dashboardPath, 'Legacy dashboard host must delegate Agent Runtime routing to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/sockets/shell_socket/shell_socket_agent_runtime_overlay_routes.ts')")) {
    push('dashboard_not_using_gateway_shell_socket_agent_runtime_overlay_routes', dashboardPath, 'Legacy dashboard host must delegate Shell Socket Agent Runtime overlay routes to gateway/**.');
  }
  if (!/handleShellSocketAgentRuntimeOverlayRoute\s*\(/.test(dashboard)) {
    push('dashboard_shell_socket_agent_runtime_overlay_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned Shell Socket Agent Runtime overlay route handler.');
  }
  if (/legacyAgentSessionMatch|mergeAgentRuntimeTranscriptPayload\s*\(|agentRuntimeTranscriptFilterFromShellSocketPath\s*\(|shellSocketChatProjection\s*\(/.test(dashboard)) {
    push('dashboard_owns_shell_socket_agent_runtime_overlay', dashboardPath, 'Legacy dashboard host must not shape Shell Socket Agent Runtime transcript overlay responses directly.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/agent_runtime_approval_routes.ts')")) {
    push('dashboard_not_using_gateway_approval_routes', dashboardPath, 'Legacy dashboard host must delegate Agent Runtime approval decision routes to gateway/**.');
  }
  if (!/handleAgentRuntimeApprovalRoute\s*\(/.test(dashboard)) {
    push('dashboard_approval_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned approval route handler.');
  }
  if (/agentRuntimeApprovalDecisionProjection\s*\(/.test(dashboard)) {
    push('dashboard_owns_approval_route_projection', dashboardPath, 'Legacy dashboard host must not call approval decision projections directly.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/agent_runtime_engine_routes.ts')")) {
    push('dashboard_not_using_gateway_engine_routes', dashboardPath, 'Legacy dashboard host must delegate Agent Runtime engine menu/selection routes to gateway/**.');
  }
  if (!dashboard.includes('handleAgentRuntimeEngineRoute')) {
    push('dashboard_engine_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned engine route handler.');
  }
  if (/agentRuntimeEnginesProjection\s*\(|agentRuntimeEngineInstallProjection\s*\(/.test(dashboard)) {
    push('dashboard_owns_engine_route_projection', dashboardPath, 'Legacy dashboard host must not call engine route projections directly.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/agent_runtime_turn_routes.ts')")) {
    push('dashboard_not_using_gateway_turn_routes', dashboardPath, 'Legacy dashboard host must delegate Agent Runtime turn/stream/steer/context-preview routes to gateway/**.');
  }
  if (!/handleAgentRuntimeTurnRoute\s*\(/.test(dashboard)) {
    push('dashboard_turn_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned turn route handler.');
  }
  if (/agentRuntimeTurnProjection\s*\(|agentRuntimeContextPackPreviewProjection\s*\(/.test(dashboard)) {
    push('dashboard_owns_turn_route_projection', dashboardPath, 'Legacy dashboard host must not call turn/context-preview route projections directly.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/agent_runtime_workspace_routes.ts')")) {
    push('dashboard_not_using_gateway_workspace_routes', dashboardPath, 'Legacy dashboard host must delegate Agent Runtime workspace routes to gateway/**.');
  }
  if (!dashboard.includes('handleAgentRuntimeWorkspaceRoute')) {
    push('dashboard_workspace_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned workspace route handler.');
  }
  if (/agentRuntimeWorkspaceProjection\s*\(|agentRuntimeWorkspacePickerProjection\s*\(/.test(dashboard)) {
    push('dashboard_owns_workspace_route_projection', dashboardPath, 'Legacy dashboard host must not call workspace route projections directly.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/universal_core_tools.ts')")) {
    push('dashboard_not_using_gateway_universal_tools', dashboardPath, 'Legacy dashboard host must delegate universal tool grant policy to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime_input_normalizer.ts')")) {
    push('dashboard_not_using_gateway_normalizer', dashboardPath, 'Legacy dashboard host must delegate input normalization to gateway/**.');
  }
  if (/function\s+normalizeAgentRuntimeTurnInput|function\s+materializeAgentRuntimeLargeTextAttachment|INFRING_AGENT_RUNTIME_LARGE_TEXT_ATTACHMENT_/.test(dashboard)) {
    push('dashboard_owns_gateway_input_policy', dashboardPath, 'Legacy dashboard host must not own large-input Gateway policy implementation.');
  }
}

const sentinelPolicyPath = 'observability/sentinel/usability_reliability_simplicity_enforcement_policy.json';
if (!read(sentinelPolicyPath).includes('gateway_adapter_physical_inversion')) {
  push('sentinel_missing_gateway_inversion_signal', sentinelPolicyPath, 'Sentinel must watch for Gateway/Adapter physical inversion.');
}

const todo = exists('docs/workspace/todo/todo_registry.json') ? read('docs/workspace/todo/todo_registry.json') : '';
if (!todo.includes('GATEWAY-PHYSICAL-REROOT')) {
  push('missing_gateway_reroot_todo', 'docs/workspace/todo/todo_registry.json', 'Gateway physical re-root debt must be present on the live TODO board.');
}

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'gateway_physical_domain_invariants_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  canonical_gateway_domain: 'gateway/**',
  adapter_domain_rule: 'translator_only',
  violations,
  debt,
};

const outJson = flag('out-json', defaultOutJson);
const outMarkdown = flag('out-markdown', defaultOutMarkdown);
fs.mkdirSync(path.dirname(path.join(ROOT, outJson)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outJson), `${JSON.stringify(payload, null, 2)}\n`);

const markdown = [
  '# Gateway Physical Domain Invariants Guard',
  '',
  `Generated: ${payload.generated_at}`,
  `Status: ${payload.ok ? 'PASS' : 'FAIL'}`,
  '',
  '## Violations',
  violations.length ? violations.map((row) => `- ${row.kind}: ${row.path} - ${row.detail}`).join('\n') : '- none',
  '',
  '## Declared Debt',
  debt.length ? debt.map((row) => `- ${row.kind}: ${row.path} - ${row.detail}`).join('\n') : '- none',
  '',
].join('\n');
fs.mkdirSync(path.dirname(path.join(ROOT, outMarkdown)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outMarkdown), markdown);

console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
