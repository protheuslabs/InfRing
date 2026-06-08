#!/usr/bin/env tsx
// Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.

const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { spawn } = require('node:child_process');
const {
  ROOT,
  invokeInfringOpsViaBridge,
  resolveBinary,
  runInfringOps,
} = require('./run_infring_ops.ts');
const { buildPrimaryDashboardHtml, hasPrimaryDashboardUi, readBuildVersionInfo, readPrimaryDashboardAsset } = require('./dashboard_asset_router.ts');
const { createAgentWsBridge } = require('../../gateway/runtime/sockets/agent_ws/agent_ws_bridge.ts');
const {
  createGatewayAgentRuntimeRouteAssembly,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts');
const {
  materializeAgentRuntimeContextPack,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_context_store.ts');
const {
  materializeKernelAgentRuntimeContextPack,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_kernel_context_bridge.ts');
const { createInfringNativeEngineAdapter } = require('./agent_engines/infring_native.ts');
const { createCodexCliEngineAdapter } = require('./agent_engines/codex_cli.ts');
const { createClaudeCodeEngineAdapter } = require('./agent_engines/claude_code.ts');
const { createGrokCodeEngineAdapter } = require('./agent_engines/grok_code.ts');
const { createOpenCodeEngineAdapter } = require('./agent_engines/opencode.ts');
const { createOpenClawEngineAdapter } = require('./agent_engines/openclaw.ts');
const { createHermesAgentEngineAdapter } = require('./agent_engines/hermes_agent.ts');
const {
  createGatewayShellSocketCoreRouteAssembly,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_core_route_assembly.ts');
const {
  createGatewaySystemRouteAssembly,
} = require('../../gateway/runtime/gateway_system_route_assembly.ts');
const {
  createGatewayHostLifecycleController,
} = require('../../gateway/runtime/gateway_host_lifecycle.ts');
const {
  createGatewayDashboardHostStatusController,
} = require('../../gateway/runtime/gateway_dashboard_host_status.ts');
const {
  createGatewayDashboardStaticResponseController,
} = require('../../gateway/runtime/gateway_dashboard_static_responses.ts');
const {
  createGatewayDashboardHostRequestBoundary,
} = require('../../gateway/runtime/gateway_dashboard_host_request_boundary.ts');
const {
  normalizeGatewayShutdownExitDelayMs: normalizeShutdownExitDelayMs,
  normalizeGatewayArgs: normalizeArgs,
  parseGatewayHostFlags: parseFlags,
} = require('../../gateway/runtime/gateway_host_config.ts');
const {
  createGatewayDashboardSurfaceLock,
} = require('../../gateway/runtime/gateway_dashboard_surface_lock.ts');
const {
  createGatewayBackendHostLauncher,
} = require('../../gateway/runtime/gateway_backend_host_launcher.ts');
const {
  createGatewayBackendStartupStateController,
} = require('../../gateway/runtime/gateway_backend_startup_state.ts');
const {
  createGatewayTroubleshootingBootstrap,
} = require('../../gateway/runtime/gateway_troubleshooting_bootstrap.ts');
const {
  createGatewayNativeOrchestrationClient,
} = require('../../gateway/runtime/gateway_native_orchestration_client.ts');
const {
  sleepGatewayMs: sleep,
  gatewayNowIso: nowIso,
} = require('../../gateway/runtime/gateway_timing.ts');
const {
  waitForGatewayBackendDown: waitForBackendDown,
  stopStaleGatewayBackend: stopStaleBackend,
} = require('../../gateway/runtime/gateway_backend_lifecycle.ts');
const {
  createGatewayDashboardVersionProjection,
} = require('../../gateway/runtime/gateway_status_projection.ts');
const {
  gatewayRequestTraceId: requestTraceId,
  sanitizeGatewayTraceId: sanitizeTraceId,
} = require('../../gateway/runtime/gateway_trace_boundary.ts');
const {
  stripGatewayTerminalControls: stripTerminalControls,
  cleanGatewayText: cleanText,
  cleanGatewayDisplayText: cleanDisplayText,
  cleanGatewayEngineId: cleanEngineId,
  cleanGatewayApprovalId: cleanApprovalId,
  cleanGatewayPathText: cleanPathText,
} = require('../../gateway/runtime/gateway_text_boundary.ts');
const {
  sendGatewayJson: sendJson,
  readGatewayJsonBody: readJsonBody,
  isGatewayTransientSocketError: isTransientSocketError,
  fetchGatewayBackend: fetchBackend,
  fetchGatewayBackendJson: fetchBackendJson,
  postGatewayBackendJson: postBackendJson,
  gatewayBackendHealth: backendHealth,
} = require('../../gateway/runtime/gateway_http_boundary.ts');

const DASHBOARD_DIR = path.resolve(ROOT, 'client', 'runtime', 'systems', 'ui');
const CANONICAL_STATIC_DIR = path.resolve(DASHBOARD_DIR, 'infring_static');
const STATIC_DIR = CANONICAL_STATIC_DIR;
const STATUS_DIR = path.resolve(
  ROOT,
  'client',
  'runtime',
  'local',
  'state',
  'ui',
  'infring_dashboard',
);
const STATUS_PATH = path.resolve(STATUS_DIR, 'server_status.json');
const {
  currentDashboardBuildInfo,
  mergeDashboardVersionPayload,
} = createGatewayDashboardVersionProjection({
  staticDir: STATIC_DIR,
  readBuildVersionInfo,
  platform: process.platform,
  arch: process.arch,
});
const {
  assertDashboardSurfaceLocked,
} = createGatewayDashboardSurfaceLock({
  dashboardDir: DASHBOARD_DIR,
  staticDir: STATIC_DIR,
  hasPrimaryDashboardUi,
});
const dashboardHostStatus = createGatewayDashboardHostStatusController({
  root: ROOT,
  staticDir: STATIC_DIR,
  statusPath: STATUS_PATH,
});
const dashboardStaticResponses = createGatewayDashboardStaticResponseController({
  staticDir: STATIC_DIR,
  buildPrimaryDashboardHtml,
  readPrimaryDashboardAsset,
  fetchBackendJson,
  sendJson,
  mergeDashboardVersionPayload,
});
const dashboardRequestBoundary = createGatewayDashboardHostRequestBoundary({
  sendJson,
});
function projectDashboardAgentRuntimeContextAuthority(kernelContext) {
  return {
    type: 'agent_runtime_context_authority_projection',
    source_authority: 'gateway.runtime.agent_runtime_route_assembly',
    kernel_materializer_used: !!(kernelContext && kernelContext.ok && kernelContext.context_pack),
  };
}
const {
  createGatewayHostCleanup,
  scheduleGatewayHostExit,
} = createGatewayHostLifecycleController({
  normalizeExitDelayMs: normalizeShutdownExitDelayMs,
  setTimeout,
  exitProcess: (code) => process.exit(code),
});
const {
  handleAgentRuntimeWorkspaceRoute,
  handleAgentRuntimeApprovalRoute,
  handleShellSocketAgentRuntimeOverlayRoute,
  handleAgentRuntimeEngineRoute,
  handleAgentRuntimeTurnRoute,
  agentRuntimeSocketTransport,
} = createGatewayAgentRuntimeRouteAssembly({
  root: ROOT,
  statusDir: STATUS_DIR,
  readJsonBody,
  sendJson,
  fetchBackendJson,
  createNativeOrchestrationClient: createGatewayNativeOrchestrationClient,
  materializeKernelAgentRuntimeContextPack,
  materializeAgentRuntimeContextPack,
  projectContextAuthority: projectDashboardAgentRuntimeContextAuthority,
  adapterFactories: {
    infring_native: createInfringNativeEngineAdapter,
    codex_cli: createCodexCliEngineAdapter,
    claude_code: createClaudeCodeEngineAdapter,
    grok_code: createGrokCodeEngineAdapter,
    opencode: createOpenCodeEngineAdapter,
    openclaw: createOpenClawEngineAdapter,
    hermes_agent: createHermesAgentEngineAdapter,
  },
});
const {
  handleShellSocketCoreRoute,
} = createGatewayShellSocketCoreRouteAssembly({
  readJsonBody,
  sendJson,
  fetchBackend,
  fetchBackendJson,
});
const {
  handleGatewaySystemRoute,
} = createGatewaySystemRouteAssembly({
  root: ROOT,
  env: () => process.env,
  invokeInfringOpsViaBridge,
  resolveBinary,
  spawnProcess: spawn,
  fetchBackendJson,
  fetchBackend,
  readJsonBody,
  sendJson,
  onHostShutdownAccepted: (body) => {
    scheduleGatewayHostExit(cleanup, body && body.exit_delay_ms);
  },
});

const {
  runSnapshotWithCompatBootstrap,
} = createGatewayTroubleshootingBootstrap({
  root: ROOT,
  statusDir: STATUS_DIR,
  invokeInfringOpsViaBridge,
  runInfringOps,
  stdout: process.stdout,
  stderr: process.stderr,
});
const {
  backendFreshnessSnapshot,
  ensureBackend,
} = createGatewayBackendHostLauncher({
  root: ROOT,
  resolveBinary,
  spawnProcess: spawn,
  env: () => process.env,
  backendHealth,
  stdout: process.stdout,
  stderr: process.stderr,
});
const {
  createBackendStartupState,
} = createGatewayBackendStartupStateController({
  backendHealth,
  ensureBackend,
  backendFreshnessSnapshot,
});
async function runServe(flags) {
  assertDashboardSurfaceLocked();
  const dashboardHtml = dashboardStaticResponses.loadDashboardHtml();
  if (!dashboardHtml.trim()) throw new Error('primary_dashboard_html_empty');
  const { backend, backendStartPromise } = await createBackendStartupState(flags);
  const wsBridge = createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson });
  const status = dashboardHostStatus.createDashboardHostStatus({
    flags,
    backend,
    backendStartPromise,
    wsBridge,
  });
  const persistStatus = () => dashboardHostStatus.persistDashboardHostStatus(status, {
    backend,
    backendStartPromise,
  });
  if (backendStartPromise) {
    backendStartPromise.finally(() => {
      try { persistStatus(); } catch {}
    });
  }
  const server = http.createServer(async (req, res) => {
    const requestUrl = new URL(req.url || '/', `http://${flags.host}:${flags.port}`);
    const pathname = requestUrl.pathname;
    const traceId = requestTraceId(req);
    dashboardRequestBoundary.applyTraceHeaders(req, res, traceId);
    try {
      if (await dashboardStaticResponses.handleGatewayDashboardStaticRoute({ req, res, pathname, flags })) return;
      if (await handleGatewaySystemRoute({ req, res, pathname, requestUrl, traceId, flags })) return;
      if (await handleShellSocketAgentRuntimeOverlayRoute({ req, res, pathname, requestUrl, traceId, flags })) return;
      if (await handleAgentRuntimeTurnRoute({ req, res, pathname, traceId, flags })) return;
      if (await handleAgentRuntimeApprovalRoute({ req, res, pathname, traceId })) return;
      if (await handleAgentRuntimeEngineRoute({ req, res, pathname, traceId })) return;
      if (await handleAgentRuntimeWorkspaceRoute({ req, res, pathname, traceId })) return;
      if (await handleShellSocketCoreRoute({ req, res, pathname, requestUrl, traceId, flags })) return;
      if (await dashboardRequestBoundary.proxyDashboardBackendRoute({ req, res, pathname, flags, requestTraceId })) return;
      dashboardRequestBoundary.sendDashboardNotFound(res, pathname);
    } catch (error) {
      dashboardRequestBoundary.sendDashboardRequestError(res, error, traceId);
    }
  });
  server.on('upgrade', (req, socket, head) => {
    dashboardRequestBoundary.handleDashboardUpgrade({
      req,
      socket,
      head,
      agentRuntimeSocketTransport,
      wsBridge,
      flags,
      requestTraceId,
    });
  });
  server.on('clientError', dashboardRequestBoundary.handleClientError);
  const cleanup = createGatewayHostCleanup({
    server,
    backend,
    processTarget: process,
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(flags.port, flags.host, () => {
      server.off('error', reject);
      persistStatus();
      console.log(JSON.stringify(status, null, 2));
      console.log(`Dashboard listening at ${status.url}`);
      resolve(null);
    });
  });
}
async function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const flags = parseFlags(args);
  if (flags.mode === 'serve' || flags.mode === 'web') { await runServe(flags); return null; }
  const opsOptions = {
    unknownDomainFallback: true,
    env: {
      INFRING_OPS_USE_PREBUILT: process.env.INFRING_OPS_USE_PREBUILT || '0',
      INFRING_OPS_LOCAL_TIMEOUT_MS: process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000',
    },
  };
  if (flags.mode === 'snapshot') return runSnapshotWithCompatBootstrap(args, opsOptions);
  return runInfringOps(['dashboard-ui', ...args], opsOptions);
}
module.exports = {
  cleanText,
  backendFreshnessSnapshot,
  currentDashboardBuildInfo,
  isTransientSocketError,
  mergeDashboardVersionPayload,
  normalizeArgs,
  parseFlags,
  run,
  scheduleGatewayHostExit,
};
if (require.main === module) {
  process.on('uncaughtException', (error) => {
    if (isTransientSocketError(error)) {
      console.error(cleanText(`dashboard_host_socket:${error.code || 'unknown'}`, 280));
      return;
    }
    console.error(cleanText(error && error.message ? error.message : String(error), 280));
    process.exitCode = 1;
  });
  Promise.resolve(run(process.argv.slice(2)))
    .then((exitCode) => { if (typeof exitCode === 'number') process.exitCode = exitCode; })
    .catch((error) => { console.error(cleanText(error && error.message ? error.message : String(error), 280)); process.exitCode = 1; });
}
