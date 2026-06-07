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
const { loadAgentRuntimeEngineRegistry, createAgentRuntimeRouter } = require('../../gateway/runtime/agent_runtime/agent_runtime_router.ts');
const {
  ingestAgentRuntimeContextProjection,
  appendAgentRuntimeTurnAtoms,
  materializeAgentRuntimeContextPack,
  loadAgentRuntimeContextRows,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_context_store.ts');
const { materializeKernelAgentRuntimeContextPack } = require('../../gateway/runtime/agent_runtime/agent_runtime_kernel_context_bridge.ts');
const { buildUniversalToolGrants } = require('../../gateway/runtime/agent_runtime/universal_core_tools.ts');
const { createInfringNativeEngineAdapter } = require('./agent_engines/infring_native.ts');
const { createCodexCliEngineAdapter } = require('./agent_engines/codex_cli.ts');
const { createClaudeCodeEngineAdapter } = require('./agent_engines/claude_code.ts');
const { createGrokCodeEngineAdapter } = require('./agent_engines/grok_code.ts');
const { createOpenClawEngineAdapter } = require('./agent_engines/openclaw.ts');
const { createHermesAgentEngineAdapter } = require('./agent_engines/hermes_agent.ts');
const {
  createShellSocketAgentRuntimeOverlayRouteHandler,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_agent_runtime_overlay_routes.ts');
const {
  createShellSocketCoreRouteHandler,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_core_routes.ts');
const {
  createGatewaySystemRouteHandler,
} = require('../../gateway/runtime/gateway_system_routes.ts');
const {
  createGatewayDashboardSystemActionDispatcher,
} = require('../../gateway/runtime/gateway_system_actions.ts');
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
  writeGatewayJson: writeJson,
  writeGatewayJsonIfMissing: writeJsonIfMissing,
  appendGatewayJsonl: appendJsonl,
  deterministicGatewayReceiptHash: deterministicReceiptHash,
} = require('../../gateway/runtime/gateway_artifacts.ts');
const {
  gatewayStatusPayloadWithBootStage: statusPayloadWithBootStage,
  createGatewayDashboardVersionProjection,
} = require('../../gateway/runtime/gateway_status_projection.ts');
const {
  gatewayRequestTraceId: requestTraceId,
  gatewayRequestTraceBoundary: requestTraceBoundary,
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
  gatewayBackendBase: backendBase,
  fetchGatewayBackend: fetchBackend,
  fetchGatewayBackendJson: fetchBackendJson,
  postGatewayBackendJson: postBackendJson,
  gatewayBackendHealth: backendHealth,
  proxyGatewayHttpRequest,
  proxyGatewayUpgrade,
} = require('../../gateway/runtime/gateway_http_boundary.ts');
const {
  normalizeAgentRuntimeTurnInput,
} = require('../../gateway/runtime/agent_runtime_input_normalizer.ts');
const {
  createAgentRuntimeWorkspaceStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_workspace.ts');
const {
  createAgentRuntimeWorkspaceRouteHandler,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_workspace_routes.ts');
const {
  createAgentRuntimeApprovalStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_approvals.ts');
const {
  createAgentRuntimeApprovalRouteHandler,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_approval_routes.ts');
const {
  createAgentRuntimeReceiptStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_receipts.ts');
const {
  createAgentRuntimeTranscriptStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_transcripts.ts');
const {
  createAgentRuntimeSessionStateStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_session_state.ts');
const {
  createAgentRuntimeEngineProjectionStore,
  findAgentRuntimeEngine,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_engine_projections.ts');
const {
  createAgentRuntimeEngineRouteHandler,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_engine_routes.ts');
const {
  createAgentRuntimeTurnProjectionStore,
  sanitizeAgentRuntimeActivityEvent,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts');
const {
  createAgentRuntimeTurnRouteHandler,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_turn_routes.ts');
const {
  createAgentRuntimeContextPreviewProjectionStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_context_preview.ts');
const {
  AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
  buildAgentRuntimeContextPack,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_context_pack.ts');

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
const agentRuntimeWorkspaceStore = createAgentRuntimeWorkspaceStore({ root: ROOT, statusDir: STATUS_DIR });
const {
  normalizeAgentRuntimeWorkspacePath,
  loadAgentRuntimeWorkspace,
} = agentRuntimeWorkspaceStore;
const {
  handleAgentRuntimeWorkspaceRoute,
} = createAgentRuntimeWorkspaceRouteHandler({
  workspaceStore: agentRuntimeWorkspaceStore,
  readJsonBody,
  sendJson,
});
const agentRuntimeApprovalStore = createAgentRuntimeApprovalStore({ root: ROOT });
const {
  sanitizeAgentRuntimeProposalArguments,
  recordAgentRuntimePendingApproval,
  mergeAgentRuntimeApprovalPermissionPolicy,
} = agentRuntimeApprovalStore;
const {
  handleAgentRuntimeApprovalRoute,
} = createAgentRuntimeApprovalRouteHandler({
  approvalStore: agentRuntimeApprovalStore,
  readJsonBody,
  sendJson,
});
const agentRuntimeReceiptStore = createAgentRuntimeReceiptStore({ root: ROOT });
const {
  recordAgentRuntimeTurnReceipts,
} = agentRuntimeReceiptStore;
const agentRuntimeTranscriptStore = createAgentRuntimeTranscriptStore({ statusDir: STATUS_DIR });
const {
  appendAgentRuntimeTranscriptTurn,
} = agentRuntimeTranscriptStore;
const {
  handleShellSocketAgentRuntimeOverlayRoute,
} = createShellSocketAgentRuntimeOverlayRouteHandler({
  transcriptStore: agentRuntimeTranscriptStore,
  fetchBackendJson,
  sendJson,
});
const {
  handleShellSocketCoreRoute,
} = createShellSocketCoreRouteHandler({
  readJsonBody,
  sendJson,
  fetchBackend,
  fetchBackendJson,
  statusPayloadWithBootStage,
});
const {
  runDashboardSystemAction,
  dispatchDashboardSystemAction,
} = createGatewayDashboardSystemActionDispatcher({
  root: ROOT,
  env: () => process.env,
  invokeInfringOpsViaBridge,
  resolveBinary,
  spawnProcess: spawn,
});
const {
  handleGatewaySystemRoute,
} = createGatewaySystemRouteHandler({
  fetchBackendJson,
  fetchBackend,
  readJsonBody,
  sendJson,
  legacyHostFallback: (action, body) => {
    if (action === 'update') return runDashboardSystemAction('update', body);
    return dispatchDashboardSystemAction(action, body);
  },
  onHostShutdownAccepted: (body) => {
    const exitDelayMs = normalizeShutdownExitDelayMs(body && body.exit_delay_ms);
    scheduleDashboardHostExit(cleanup, exitDelayMs);
  },
});
const agentRuntimeSessionStateStore = createAgentRuntimeSessionStateStore({
  statusDir: STATUS_DIR,
  loadRegistry: () => loadAgentRuntimeEngineRegistry(ROOT),
  findEngine: findAgentRuntimeEngine,
});
const {
  loadAgentRuntimeSelection,
  saveAgentRuntimeSelection,
  agentRuntimeSteerProjection,
  drainAgentRuntimeSteeringInterventions,
} = agentRuntimeSessionStateStore;
const agentRuntimeEngineProjectionStore = createAgentRuntimeEngineProjectionStore({
  root: ROOT,
  loadRegistry: () => loadAgentRuntimeEngineRegistry(ROOT),
  createAdapterMap: createAgentRuntimeEngineAdapterMap,
  loadSelection: loadAgentRuntimeSelection,
  saveSelection: saveAgentRuntimeSelection,
});
const {
  handleAgentRuntimeEngineRoute,
} = createAgentRuntimeEngineRouteHandler({
  engineProjectionStore: agentRuntimeEngineProjectionStore,
  selectEngine: agentRuntimeEngineProjectionStore.agentRuntimeSelectionProjection,
  readJsonBody,
  sendJson,
});
const agentRuntimeTurnProjectionStore = createAgentRuntimeTurnProjectionStore({
  root: ROOT,
  contextFanoutTarget: AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
  normalizeAgentRuntimeTurnInput,
  loadAgentRuntimeEngineRegistry: () => loadAgentRuntimeEngineRegistry(ROOT),
  findAgentRuntimeEngine,
  loadAgentRuntimeWorkspace,
  createRouter: createDashboardAgentRuntimeRouter,
  sanitizeAgentRuntimeActivityEvent,
  appendAgentRuntimeTranscriptTurn,
  appendAgentRuntimeTurnAtoms,
  ingestAgentRuntimeContextProjection,
  loadAgentRuntimeContextRows,
  materializeKernelAgentRuntimeContextPack,
  materializeAgentRuntimeContextPack,
  buildAgentRuntimeContextPack,
  mergeAgentRuntimeApprovalPermissionPolicy,
  buildUniversalToolGrants,
  drainAgentRuntimeSteeringInterventions,
  sanitizeAgentRuntimeProposalArguments,
  recordAgentRuntimePendingApproval,
  recordAgentRuntimeTurnReceipts,
});
const agentRuntimeContextPreviewProjectionStore = createAgentRuntimeContextPreviewProjectionStore({
  root: ROOT,
  loadAgentRuntimeContextRows,
  materializeKernelAgentRuntimeContextPack,
  materializeAgentRuntimeContextPack,
  buildAgentRuntimeContextPack,
  buildUniversalToolGrants,
});
const {
  handleAgentRuntimeTurnRoute,
} = createAgentRuntimeTurnRouteHandler({
  turnProjectionStore: agentRuntimeTurnProjectionStore,
  contextPreviewProjectionStore: agentRuntimeContextPreviewProjectionStore,
  steer: agentRuntimeSteerProjection,
  createNativeOrchestrationClient: createGatewayNativeOrchestrationClient,
  readJsonBody,
  sendJson,
});

function createAgentRuntimeEngineAdapterMap(options = {}) {
  const liveDispatch = options.liveDispatch === true;
  const cwd = normalizeAgentRuntimeWorkspacePath(options.cwd || ROOT);
  return {
    infring_native: createInfringNativeEngineAdapter({
      liveDispatch,
      orchestrationClient: options.nativeOrchestrationClient || options.orchestrationClient,
    }),
    codex_cli: createCodexCliEngineAdapter({ liveDispatch, cwd }),
    claude_code: createClaudeCodeEngineAdapter({ liveDispatch, cwd }),
    grok_code: createGrokCodeEngineAdapter({ liveDispatch, cwd }),
    openclaw: createOpenClawEngineAdapter({ liveDispatch, cwd }),
    hermes_agent: createHermesAgentEngineAdapter({ liveDispatch, cwd }),
  };
}
function createDashboardAgentRuntimeRouter(options = {}) {
  const router = createAgentRuntimeRouter({ root: ROOT, disableTraceWriter: options.disableTraceWriter === true });
  const adapters = createAgentRuntimeEngineAdapterMap({
    liveDispatch: options.liveDispatch === true,
    nativeOrchestrationClient: options.nativeOrchestrationClient,
    cwd: options.cwd,
  });
  for (const [engineId, adapter] of Object.entries(adapters)) router.registerAdapter(engineId, adapter);
  return router;
}
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
function scheduleDashboardHostExit(cleanup, normalizedDelayMs) {
  const waitMs = normalizeShutdownExitDelayMs(normalizedDelayMs);
  setTimeout(() => {
    try { cleanup(); } catch {}
    setTimeout(() => {
      try { process.exit(0); } catch {}
    }, 0);
  }, waitMs);
}
async function runServe(flags) {
  assertDashboardSurfaceLocked();
  let dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR);
  if (!dashboardHtml.trim()) throw new Error('primary_dashboard_html_empty');
  const backend = {
    child: null,
    reused: false,
    ready: await backendHealth(flags, 1500),
    freshness: null,
    startup_error: '',
  };
  let backendStartPromise = null;
  if (!backend.ready) {
    backendStartPromise = ensureBackend(flags)
      .then((result) => {
        backend.child = result && result.child ? result.child : null;
        backend.reused = !!(result && result.reused);
        backend.ready = true;
        backend.freshness = result && result.freshness ? result.freshness : null;
        backend.startup_error = '';
        return result;
      })
      .catch((error) => {
        backend.ready = false;
        backend.startup_error = cleanText(error && error.message ? error.message : String(error), 200);
        return null;
      });
  } else {
    try {
      const result = await ensureBackend(flags);
      backend.child = result && result.child ? result.child : null;
      backend.reused = !!(result && result.reused);
      backend.ready = true;
      backend.freshness = result && result.freshness ? result.freshness : null;
      backend.startup_error = '';
    } catch (error) {
      backend.reused = true;
      backend.freshness = backendFreshnessSnapshot(flags);
      backend.startup_error = cleanText(error && error.message ? error.message : String(error), 200);
    }
  }
  const wsBridge = createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson });
  const status = {
    ok: true,
    type: 'infring_dashboard_server',
    ts: nowIso(),
    url: `http://${flags.host}:${flags.port}/dashboard`,
    host: flags.host,
    port: flags.port,
    refresh_ms: flags.refreshMs,
    team: flags.team,
    authority: 'primary_dashboard_ui_over_rust_core_api',
    dashboard_ui_mode_requested: flags.uiMode,
    dashboard_ui_mode_active: 'primary',
    backend_url: backendBase(flags),
    backend_reused: backend.reused,
    backend_ready: backend.ready,
    backend_freshness: backend.freshness,
    backend_start_pending: !!backendStartPromise,
    backend_start_error: '',
    ws_bridge_enabled: !!wsBridge.ws_enabled,
    ws_bridge_error: cleanText(wsBridge.ws_error || '', 120),
    dashboard_static_dir: path.basename(STATIC_DIR),
    status_path: path.relative(ROOT, STATUS_PATH),
  };
  function persistStatus() {
    status.backend_reused = backend.reused;
    status.backend_ready = backend.ready;
    status.backend_freshness = backend.freshness;
    status.backend_start_pending = !!backendStartPromise && !backend.ready && !backend.startup_error;
    status.backend_start_error = backend.startup_error;
    ensureDir(STATUS_DIR);
    writeJson(STATUS_PATH, status);
  }
  if (backendStartPromise) {
    backendStartPromise.finally(() => {
      try { persistStatus(); } catch {}
    });
  }
  const server = http.createServer(async (req, res) => {
    const requestUrl = new URL(req.url || '/', `http://${flags.host}:${flags.port}`);
    const pathname = requestUrl.pathname;
    const traceId = requestTraceId(req);
    try { res.setHeader('x-infring-trace-id', traceId); } catch {}
    try { res.setHeader('x-infring-trace-source', requestTraceBoundary(req).source || 'unknown'); } catch {}
    try {
      if ((req.method === 'GET' || req.method === 'HEAD') && (pathname === '/dashboard-classic' || pathname === '/dashboard-shell')) {
        res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' });
        res.end(req.method === 'HEAD' ? '' : 'dashboard_surface_retired');
        return;
      }
      if ((req.method === 'GET' || req.method === 'HEAD') && pathname === '/') {
        res.writeHead(302, { location: '/dashboard', 'cache-control': 'no-store' });
        res.end();
        return;
      }
      if (
        (req.method === 'GET' || req.method === 'HEAD') &&
        (pathname === '/dashboard' || pathname === '/dashboard/' || (pathname.startsWith('/dashboard/') && !path.extname(pathname)))
      ) {
        dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR) || dashboardHtml;
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(req.method === 'HEAD' ? '' : dashboardHtml);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/status') {
        const status = mergeDashboardVersionPayload(await statusPayloadWithBootStage(flags));
        return void sendJson(res, 200, status);
      }
      if (req.method === 'GET' && pathname === '/api/version') {
        const versionPayload = await fetchBackendJson(flags, '/api/version', 4000).catch(() => ({ ok: true }));
        return void sendJson(res, 200, mergeDashboardVersionPayload(versionPayload));
      }
      if (await handleGatewaySystemRoute({ req, res, pathname, requestUrl, traceId, flags })) return;
      if (await handleShellSocketAgentRuntimeOverlayRoute({ req, res, pathname, requestUrl, traceId, flags })) return;
      if (await handleAgentRuntimeTurnRoute({ req, res, pathname, traceId, flags })) return;
      if (await handleAgentRuntimeApprovalRoute({ req, res, pathname, traceId })) return;
      if (await handleAgentRuntimeEngineRoute({ req, res, pathname, traceId })) return;
      if (await handleAgentRuntimeWorkspaceRoute({ req, res, pathname, traceId })) return;
      if (await handleShellSocketCoreRoute({ req, res, pathname, requestUrl, traceId, flags })) return;
      if (req.method === 'GET') {
        const agentSessionsMatch = pathname.match(/^\/api\/agents\/([^/]+)\/sessions$/);
        if (agentSessionsMatch) {
          const rawAgentId = String(agentSessionsMatch[1] || '').trim();
          let decodedAgentId = rawAgentId;
          try { decodedAgentId = decodeURIComponent(rawAgentId); } catch {}
          const normalizedAgentId = cleanText(decodedAgentId, 120).toLowerCase();
          if (normalizedAgentId === 'system') {
            return void sendJson(res, 200, {
              ok: true,
              agent_id: 'system',
              sessions: [],
              system_thread: true,
            });
          }
        }
      }
      if (req.method === 'GET') {
        const asset = readPrimaryDashboardAsset(STATIC_DIR, pathname);
        if (asset) {
          res.writeHead(200, { 'content-type': asset.contentType, 'cache-control': 'no-store' });
          res.end(asset.body);
          return;
        }
      }
      if (pathname === '/healthz' || pathname.startsWith('/api/')) {
        return void await proxyGatewayHttpRequest(req, res, {
          apiHost: flags.apiHost,
          apiPort: flags.apiPort,
          requestTraceId,
        });
      }
      sendJson(res, 404, { ok: false, type: 'infring_dashboard_not_found', path: pathname });
    } catch (error) {
      const message = cleanText(error && error.message ? error.message : String(error), 260);
      const statusCode = message === 'request_body_invalid_json' || message === 'request_body_too_large' ? 400 : 500;
      sendJson(res, statusCode, { ok: false, type: 'infring_dashboard_request_error', trace_id: traceId, error: message });
    }
  });
  server.on('upgrade', (req, socket, head) => {
    if (wsBridge.tryHandle(req, socket, head)) return;
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    if (!pathname.startsWith('/api/')) { socket.destroy(); return; }
    proxyGatewayUpgrade(req, socket, head, {
      apiHost: flags.apiHost,
      apiPort: flags.apiPort,
      requestTraceId,
    });
  });
  server.on('clientError', (_error, socket) => {
    try { socket.destroy(); } catch {}
  });
  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    try { server.close(); } catch {}
    if (backend.child && backend.child.exitCode == null) { try { backend.child.kill('SIGTERM'); } catch {} }
  };
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);
  process.on('exit', cleanup);
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
  dispatchDashboardSystemAction,
  run,
  runDashboardSystemAction,
  scheduleDashboardHostExit,
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
