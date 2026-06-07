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
  {
    path: 'docs/workspace/system_action_authority_policy.md',
    tokens: [
      'System actions are OS/runtime authority, not Gateway authority.',
      'Gateway may own external route admission and bounded response projection for system controls',
      'Core / ops system-action authority',
      'Gateway system control routes forward to Core/ops authority.',
    ],
    forbidden: [
      'Gateway owns restart authority',
      'Gateway owns shutdown authority',
      'Gateway owns update authority',
    ],
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

const gatewaySystemRoutesPath = 'gateway/runtime/gateway_system_routes.ts';
if (exists(gatewaySystemRoutesPath)) {
  const gatewaySystemRoutes = read(gatewaySystemRoutesPath);
  if (/spawn\s*\(|resolveBinary\s*\(|invokeInfringOpsViaBridge\s*\(|runInfringOps\s*\(|process\.exit\s*\(|dispatchDashboardSystemAction\s*\(|runDashboardSystemAction\s*\(/.test(gatewaySystemRoutes)) {
    push('gateway_system_route_owns_os_authority', gatewaySystemRoutesPath, 'Gateway system routes may wrap/forward system controls, but must not spawn binaries, call resident IPC directly, or exit the host process.');
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

const declaredLegacyShims = Array.isArray(policy.declared_legacy_compatibility_shims)
  ? policy.declared_legacy_compatibility_shims
  : [];
const declaredShimPaths = new Set<string>();
const shimImplementationSignals = [
  /function\s+\w+/,
  /class\s+\w+/,
  /export\s+(?:async\s+)?function/,
  /createServer\s*\(/,
  /new\s+WebSocketServer\s*\(/,
  /res\.writeHead\s*\(/,
  /req\.url/,
  /pathname\s*===/,
  /payload_budget/i,
  /permission_gate/i,
  /source_authority\s*:/,
];
for (const shim of declaredLegacyShims) {
  const rel = String(shim && shim.path || '');
  const canonicalPath = String(shim && shim.canonical_path || '');
  if (!rel) {
    push('legacy_shim_missing_path', policyPath, 'Every declared legacy Gateway shim needs a path.');
    continue;
  }
  declaredShimPaths.add(rel);
  if (!rel.startsWith('adapters/')) {
    push('legacy_shim_outside_adapter_domain', policyPath, `${rel} must live under adapters/** while it is declared compatibility debt.`);
  }
  if (!canonicalPath.startsWith('gateway/')) {
    push('legacy_shim_canonical_path_not_gateway', policyPath, `${rel} must point at a gateway/** canonical path.`);
  }
  if (!String(shim.retirement_todo || '').trim()) push('legacy_shim_missing_retirement_todo', policyPath, rel);
  if (!String(shim.allowed_until || '').trim()) push('legacy_shim_missing_allowed_until', policyPath, rel);
  if (!exists(rel)) {
    push('legacy_shim_file_missing', rel, 'Declared legacy shim path does not exist.');
  } else {
    const text = read(rel);
    const canonicalBase = path.basename(canonicalPath);
    if (!exists(canonicalPath)) push('legacy_shim_canonical_file_missing', canonicalPath, `${rel} points at a missing canonical Gateway module.`);
    if (!text.includes(canonicalPath) && !text.includes(canonicalBase)) {
      push('legacy_shim_missing_canonical_delegate', rel, `Compatibility shim must delegate to ${canonicalPath}.`);
    }
    if (!/Compatibility shim/i.test(text)) {
      push('legacy_shim_not_labeled', rel, 'Compatibility shim must label itself as compatibility debt.');
    }
    if (shimImplementationSignals.some((pattern) => pattern.test(text))) {
      push('legacy_shim_contains_implementation_logic', rel, 'Compatibility shim must stay a thin Gateway delegate and must not grow route, policy, socket, or payload implementation logic.');
    }
  }
  noteDebt('declared_legacy_gateway_shim', rel, `canonical_path=${canonicalPath || 'missing'} retirement_todo=${shim.retirement_todo || 'missing'} allowed_until=${shim.allowed_until || 'missing'}`);
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
  if (declaredShimPaths.has(rel)) {
    noteDebt('legacy_adapter_gateway_policy_signal', rel, 'Gateway policy signal remains in a declared compatibility shim.');
    continue;
  }
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
  if (!dashboard.includes("require('../../gateway/runtime/gateway_system_routes.ts')")) {
    push('dashboard_not_using_gateway_system_routes', dashboardPath, 'Legacy dashboard host must delegate Gateway system routes to gateway/**.');
  }
  if (!/handleGatewaySystemRoute\s*\(/.test(dashboard)) {
    push('dashboard_gateway_system_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned system route handler.');
  }
  if (/pathname\s*===\s*['"]\/api\/system\/release-check['"]|pathname\s*===\s*['"]\/api\/config['"]|pathname\s*===\s*['"]\/api\/config\/schema['"]|pathname\s*===\s*['"]\/api\/auth\/check['"]|pathname\s*===\s*['"]\/api\/system\/restart['"]|pathname\s*===\s*['"]\/api\/system\/update['"]|pathname\s*===\s*['"]\/api\/system\/shutdown['"]|\/api\/update\/check\$\{qs\}/.test(dashboard)) {
    push('dashboard_owns_gateway_system_release_check', dashboardPath, 'Legacy dashboard host must not shape Gateway system release-check route directly.');
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
  if (!dashboard.includes("require('../../gateway/runtime/sockets/shell_socket/shell_socket_core_routes.ts')")) {
    push('dashboard_not_using_gateway_shell_socket_core_routes', dashboardPath, 'Legacy dashboard host must delegate Shell Socket status/command/eval/lifecycle routes to gateway/**.');
  }
  if (!/handleShellSocketCoreRoute\s*\(/.test(dashboard)) {
    push('dashboard_shell_socket_core_route_handler_missing', dashboardPath, 'Legacy dashboard host must call the Gateway-owned Shell Socket core route handler.');
  }
  if (/isShellSocket(?:CommandIngress|StatusProjection|EvalIssueIngress|LifecycleIngress)Path|shellSocket(?:CommandIngress|StatusProjection|EvalIssueIngress|LifecycleIngress)\s*\(/.test(dashboard)) {
    push('dashboard_owns_shell_socket_core_routes', dashboardPath, 'Legacy dashboard host must not shape Shell Socket status/command/eval/lifecycle routes directly.');
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
  if (/function\s+(?:agentRuntimeSelectionProjection|agentRuntimeSteerProjection|readAgentRuntimeSteeringRecords|sanitizeAgentRuntimeActivityEvent|classifyAgentRuntimePreTurnFailureCode|agentRuntimePreTurnFailureProjection)\b/.test(dashboard)) {
    push('dashboard_owns_agent_runtime_projection_helpers', dashboardPath, 'Legacy dashboard host must not define Agent Runtime selection, steering, activity, or pre-turn failure projection helpers; those belong under gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_trace_boundary.ts')")) {
    push('dashboard_not_using_gateway_trace_boundary', dashboardPath, 'Legacy dashboard host must delegate trace admission and boundary metadata to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_text_boundary.ts')")) {
    push('dashboard_not_using_gateway_text_boundary', dashboardPath, 'Legacy dashboard host must delegate boundary text normalization to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_timing.ts')")) {
    push('dashboard_not_using_gateway_timing', dashboardPath, 'Legacy dashboard host must delegate Gateway lifecycle timing primitives to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_artifacts.ts')")) {
    push('dashboard_not_using_gateway_artifacts', dashboardPath, 'Legacy dashboard host must delegate local artifact and receipt primitives to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_backend_lifecycle.ts')")) {
    push('dashboard_not_using_gateway_backend_lifecycle', dashboardPath, 'Legacy dashboard host must delegate backend lifecycle wait/stop decisions to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_backend_freshness.ts')")) {
    push('dashboard_not_using_gateway_backend_freshness', dashboardPath, 'Legacy dashboard host must delegate backend freshness detection to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_status_projection.ts')")) {
    push('dashboard_not_using_gateway_status_projection', dashboardPath, 'Legacy dashboard host must delegate status/version projections to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_system_actions.ts')")) {
    push('dashboard_not_using_gateway_system_actions', dashboardPath, 'Legacy dashboard host must delegate system action argument/env/result projection to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_host_config.ts')")) {
    push('dashboard_not_using_gateway_host_config', dashboardPath, 'Legacy dashboard host must delegate CLI flag parsing and host config defaults to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_dashboard_surface_lock.ts')")) {
    push('dashboard_not_using_gateway_dashboard_surface_lock', dashboardPath, 'Legacy dashboard host must delegate dashboard surface lock policy to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_backend_host_launcher.ts')")) {
    push('dashboard_not_using_gateway_backend_host_launcher', dashboardPath, 'Legacy dashboard host must delegate backend launch orchestration to gateway/**.');
  }
  if (/function\s+(?:sanitizeTraceId|requestTraceId|requestTraceBoundary)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_trace_boundary', dashboardPath, 'Legacy dashboard host must not define Gateway trace boundary helpers locally.');
  }
  if (/function\s+(?:sleep|nowIso)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_timing', dashboardPath, 'Legacy dashboard host must not define Gateway lifecycle timing primitives locally.');
  }
  if (/function\s+(?:ensureDir|writeJson|writeJsonIfMissing|appendJsonl|appendBoundedJsonl|deterministicReceiptHash)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_artifacts', dashboardPath, 'Legacy dashboard host must not define Gateway artifact or receipt primitives locally.');
  }
  if (/function\s+(?:stripTerminalControls|cleanText|cleanDisplayText|cleanEngineId|cleanApprovalId|cleanPathText)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_text_boundary', dashboardPath, 'Legacy dashboard host must not define Gateway text boundary helpers locally.');
  }
  if (/gateway_boundary:\s*['"]adapters\.runtime\.infring_dashboard['"]/.test(dashboard)) {
    push('dashboard_labels_adapter_as_gateway_boundary', dashboardPath, 'Gateway trace boundary metadata must identify gateway/** authority, not adapter host paths.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_http_boundary.ts')")) {
    push('dashboard_not_using_gateway_http_boundary', dashboardPath, 'Legacy dashboard host must delegate JSON response, bounded body parsing, and proxy header filtering to gateway/**.');
  }
  if (!dashboard.includes('fetchGatewayBackend') || !dashboard.includes('fetchGatewayBackendJson') || !dashboard.includes('postGatewayBackendJson')) {
    push('dashboard_not_using_gateway_backend_fetch_helpers', dashboardPath, 'Legacy dashboard host must delegate backend fetch/client helpers to gateway/**.');
  }
  if (!dashboard.includes('gatewayBackendHealth')) {
    push('dashboard_not_using_gateway_backend_health', dashboardPath, 'Legacy dashboard host must delegate backend health probing to gateway/**.');
  }
  if (!dashboard.includes('isGatewayTransientSocketError')) {
    push('dashboard_not_using_gateway_socket_error_classifier', dashboardPath, 'Legacy dashboard host must delegate transient socket error classification to gateway/**.');
  }
  if (!dashboard.includes('proxyGatewayHttpRequest') || !dashboard.includes('proxyGatewayUpgrade')) {
    push('dashboard_not_using_gateway_proxy_helpers', dashboardPath, 'Legacy dashboard host must delegate backend HTTP and WebSocket proxy mechanics to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/gateway_native_orchestration_client.ts')")) {
    push('dashboard_not_using_gateway_native_orchestration_client', dashboardPath, 'Legacy dashboard host must construct native orchestration clients through gateway/**.');
  }
  if (/function\s+(?:sendJson|readJsonBody|filteredHeaders|ignoreStreamErrors|isTransientSocketError|backendBase|backendHealth|fetchBackend|fetchBackendJson|postBackendJson|proxyToBackend|proxyUpgrade|createGatewayNativeOrchestrationClient)\b|const\s+HOP_BY_HOP\b|http\.request\s*\(/.test(dashboard)) {
    push('dashboard_owns_gateway_http_boundary', dashboardPath, 'Legacy dashboard host must not define Gateway HTTP boundary helpers locally.');
  }
  if (/function\s+(?:backendFreshnessSnapshot|waitForBackendDown|stopStaleBackend|statusPayloadWithBootStage|currentDashboardBuildInfo|mergeDashboardVersionPayload)\b|dashboard_backend_freshness\.ts/.test(dashboard)) {
    push('dashboard_owns_gateway_lifecycle_or_status_projection', dashboardPath, 'Legacy dashboard host must not define Gateway backend lifecycle, freshness, or status/version projections locally.');
  }
  if (/function\s+(?:backendSpawnEnv|spawnBackend|ensureBackend)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_backend_host_launcher', dashboardPath, 'Legacy dashboard host must not define Gateway backend host launcher helpers locally.');
  }
  if (/function\s+(?:dashboardSystemActionArgs|dashboardSystemActionEnv|runDashboardSystemAction|dispatchDashboardSystemAction)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_system_actions', dashboardPath, 'Legacy dashboard host must not define Gateway system action helpers locally.');
  }
  if (/function\s+(?:parsePositiveInt|normalizeShutdownExitDelayMs|normalizeArgs|defaultApiPort|parseFlags)\b|const\s+(?:DEFAULT_HOST|DEFAULT_PORT|DEFAULT_TEAM|DEFAULT_REFRESH_MS|DEFAULT_BACKEND_READY_TIMEOUT_MS|BACKEND_PORT_OFFSET|DASHBOARD_SHUTDOWN_EXIT_DELAY_(?:DEFAULT|MIN|MAX)_MS)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_host_config', dashboardPath, 'Legacy dashboard host must not define Gateway host config parsing or defaults locally.');
  }
  if (/function\s+(?:discoverSiblingAltDashboardSurfaces|assertNoAlternateDashboardSurfaces|assertSingleDashboardRoot|assertDashboardSurfaceLocked)\b|const\s+(?:FORBIDDEN_ALT_DASHBOARD_DIRS|SIBLING_ALT_DASHBOARD_PATTERN)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_dashboard_surface_lock', dashboardPath, 'Legacy dashboard host must not define Gateway dashboard surface lock policy locally.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/universal_core_tools.ts')")) {
    push('dashboard_not_using_gateway_universal_tools', dashboardPath, 'Legacy dashboard host must delegate universal tool grant policy to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime_input_normalizer.ts')")) {
    push('dashboard_not_using_gateway_normalizer', dashboardPath, 'Legacy dashboard host must delegate input normalization to gateway/**.');
  }
  if (!dashboard.includes("require('../../gateway/runtime/agent_runtime/agent_runtime_context_pack.ts')")) {
    push('dashboard_not_using_gateway_context_pack', dashboardPath, 'Legacy dashboard host must delegate Agent Runtime context-pack construction to gateway/**.');
  }
  if (/function\s+normalizeAgentRuntimeTurnInput|function\s+materializeAgentRuntimeLargeTextAttachment|INFRING_AGENT_RUNTIME_LARGE_TEXT_ATTACHMENT_/.test(dashboard)) {
    push('dashboard_owns_gateway_input_policy', dashboardPath, 'Legacy dashboard host must not own large-input Gateway policy implementation.');
  }
  if (/function\s+(?:buildAgentRuntimeContextPack|estimateContextTokens|cleanContextRole|contextRowText|contextRef|buildSpan)\b|const\s+AGENT_RUNTIME_CONTEXT_(?:HOT_TAIL_COUNT|MAX_ROWS|ROW_TEXT_MAX)\b/.test(dashboard)) {
    push('dashboard_owns_gateway_context_pack', dashboardPath, 'Legacy dashboard host must not define Agent Runtime context-pack construction locally.');
  }
}

const sentinelPolicyPath = 'observability/sentinel/usability_reliability_simplicity_enforcement_policy.json';
if (!read(sentinelPolicyPath).includes('gateway_adapter_physical_inversion')) {
  push('sentinel_missing_gateway_inversion_signal', sentinelPolicyPath, 'Sentinel must watch for Gateway/Adapter physical inversion.');
}

const traceRegistryPath = 'observability/traces/domain_trace_extension_registry.json';
if (exists(traceRegistryPath)) {
  const traceRegistry = read(traceRegistryPath);
  if (/adapters\/runtime\/shell_socket_/.test(traceRegistry)) {
    push('observability_uses_legacy_adapter_shell_socket_source', traceRegistryPath, 'Gateway Shell Socket trace sources must point at gateway/** canonical modules, not adapter compatibility shims.');
  }
} else {
  push('trace_registry_missing', traceRegistryPath, 'Trace extension registry must exist so Gateway physical ownership can be observed.');
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
