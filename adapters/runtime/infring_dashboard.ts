#!/usr/bin/env tsx
// Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.

const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { createHash } = require('node:crypto');
const { spawn } = require('node:child_process');
const {
  ROOT,
  invokeInfringOpsViaBridge,
  resolveBinary,
  runInfringOps,
} = require('./run_infring_ops.ts');
const { buildPrimaryDashboardHtml, hasPrimaryDashboardUi, readBuildVersionInfo, readPrimaryDashboardAsset } = require('./dashboard_asset_router.ts');
const { createAgentWsBridge } = require('./agent_ws_bridge.ts');
const { loadAgentRuntimeEngineRegistry } = require('./agent_engines/agent_runtime_router.ts');
const { createCodexCliEngineAdapter } = require('./agent_engines/codex_cli.ts');
const {
  isShellSocketChatProjectionPath,
  shellSocketChatProjection,
} = require('./shell_socket_chat_projection.ts');
const {
  isShellSocketCommandIngressPath,
  shellSocketCommandIngress,
} = require('./shell_socket_command_ingress.ts');
const {
  isShellSocketStatusProjectionPath,
  shellSocketStatusProjection,
} = require('./shell_socket_status_projection.ts');
const {
  isShellSocketEvalIssueIngressPath,
  shellSocketEvalIssueIngress,
} = require('./shell_socket_eval_issue_ingress.ts');
const {
  isShellSocketLifecycleIngressPath,
  shellSocketLifecycleIngress,
} = require('./shell_socket_lifecycle_ingress.ts');
const {
  backendFreshnessSnapshot: backendFreshnessSnapshotFromProcess,
  backendSpawnEnv: backendSpawnEnvForRoot,
  shouldRestartStaleBackend,
} = require('./dashboard_backend_freshness.ts');

const DASHBOARD_DIR = path.resolve(ROOT, 'client', 'runtime', 'systems', 'ui');
const CANONICAL_STATIC_DIR = path.resolve(DASHBOARD_DIR, 'infring_static');
const STATIC_DIR = CANONICAL_STATIC_DIR;
const FORBIDDEN_ALT_DASHBOARD_DIRS = [
  path.resolve(DASHBOARD_DIR, 'legacy_dashboard'),
  path.resolve(DASHBOARD_DIR, 'reference_runtime_dashboard'),
  path.resolve(DASHBOARD_DIR, 'control_runtime_dashboard'),
  path.resolve(DASHBOARD_DIR, 'dashboard_legacy'),
  path.resolve(DASHBOARD_DIR, 'deprecated_dashboard'),
];
const SIBLING_ALT_DASHBOARD_PATTERN = /(legacy|reference_runtime|control_runtime|deprecated)/i;
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
const STATUS_SNAPSHOT_PATH = path.resolve(STATUS_DIR, 'latest_snapshot.json');
const TROUBLESHOOTING_DIR = path.resolve(STATUS_DIR, 'troubleshooting');
const TROUBLESHOOTING_RECENT_WORKFLOWS_PATH = path.resolve(TROUBLESHOOTING_DIR, 'recent_workflows.json');
const TROUBLESHOOTING_EVAL_QUEUE_PATH = path.resolve(TROUBLESHOOTING_DIR, 'eval_queue.json');
const TROUBLESHOOTING_ISSUE_OUTBOX_PATH = path.resolve(TROUBLESHOOTING_DIR, 'issue_outbox.json');
const TROUBLESHOOTING_LATEST_SNAPSHOT_PATH = path.resolve(TROUBLESHOOTING_DIR, 'latest_snapshot.json');
const TROUBLESHOOTING_SNAPSHOT_HISTORY_PATH = path.resolve(TROUBLESHOOTING_DIR, 'snapshot_history.jsonl');
const TROUBLESHOOTING_LATEST_EVAL_REPORT_PATH = path.resolve(TROUBLESHOOTING_DIR, 'latest_eval_report.json');
const TROUBLESHOOTING_DEFAULT_EVAL_MODEL = 'gpt-5.4';
const TROUBLESHOOTING_MAX_RECENT = 10;
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const DEFAULT_TEAM = 'ops';
const DEFAULT_REFRESH_MS = 2000;
const DEFAULT_BACKEND_READY_TIMEOUT_MS = 120000;
const BACKEND_PORT_OFFSET = 1000;
const DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS = 180;
const DASHBOARD_SHUTDOWN_EXIT_DELAY_MIN_MS = 80;
const DASHBOARD_SHUTDOWN_EXIT_DELAY_MAX_MS = 5000;
const HOP_BY_HOP = new Set(['connection', 'host', 'keep-alive', 'proxy-authenticate', 'proxy-authorization', 'te', 'trailers', 'transfer-encoding', 'upgrade']);

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function isTransientSocketError(error) {
  const code = cleanText(error && error.code ? error.code : '', 40);
  return code === 'ECONNRESET' || code === 'EPIPE' || code === 'ERR_STREAM_PREMATURE_CLOSE';
}
function ignoreStreamErrors(stream) {
  if (!stream || typeof stream.on !== 'function') return;
  if (stream.__infringIgnoreErrorsInstalled) return;
  stream.__infringIgnoreErrorsInstalled = true;
  stream.on('error', () => {});
}
function parsePositiveInt(value, fallback, min = 1, max = 65535) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}

function projectAgentRuntimeEngineRow(engine, health) {
  const row = engine && typeof engine === 'object' ? engine : {};
  const install = row.install && typeof row.install === 'object' ? row.install : {};
  const engineId = cleanEngineId(row.engine_id);
  const healthStatus = cleanText(health && health.status ? health.status : '', 80);
  const registryStatus = cleanText(row.status || '', 80);
  const nativeReady = engineId === 'infring_native' && registryStatus === 'adapter_seam_ready';
  const status = healthStatus || (nativeReady ? 'available' : (registryStatus || 'unknown'));
  const selectable = status === 'available' || status === 'adapter_ready' || nativeReady;
  const downloadAvailable = install.download_available === true || (health && health.download_available === true);
  return {
    engine_id: engineId,
    display_name: cleanText(row.display_name || engineId, 120),
    engine_kind: cleanText(row.engine_kind || '', 120),
    transport_kind: cleanText(row.transport_kind || '', 120),
    status,
    selectable,
    capabilities: Array.isArray(row.capabilities) ? row.capabilities.map((item) => cleanText(item, 120)).filter(Boolean).slice(0, 12) : [],
    download_available: !!downloadAvailable,
    download_action_ref: cleanText(install.download_action_ref || (health && health.download_action_ref) || '', 240),
    preferred_install_method: cleanText(install.preferred_install_method || '', 80),
    command_line_hint: cleanText(install.command_line_hint || '', 500),
    browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
    display_when_missing: cleanText(install.display_when_missing || (downloadAvailable ? 'download_icon' : ''), 80),
    version_preview: cleanText(health && health.version_preview ? health.version_preview : '', 240),
  };
}

async function agentRuntimeEnginesProjection(traceId) {
  const info = loadAgentRuntimeEngineRegistry(ROOT);
  const engines = Array.isArray(info.engines) ? info.engines : [];
  const codexAdapter = createCodexCliEngineAdapter({ liveDispatch: false });
  const rows = [];
  for (const engine of engines) {
    const engineId = cleanEngineId(engine && engine.engine_id);
    let health = null;
    if (engineId === 'codex_cli') {
      health = await codexAdapter.health_check({
        message: {
          trace_id: traceId,
          request_id: `agent-runtime-menu:${Date.now()}`,
          engine_id: 'codex_cli',
          session_id: 'dashboard-menu',
        },
        engine,
      }).catch((error) => ({
        status: 'not_downloaded',
        download_available: true,
        reason: cleanText(error && error.message ? error.message : error, 200),
      }));
    }
    rows.push(projectAgentRuntimeEngineRow(engine, health));
  }
  return {
    ok: true,
    type: 'agent_runtime_engines_projection',
    trace_id: traceId,
    socket_route: '/ws/agent-runtime',
    selected_default_engine_id: 'infring_native',
    engines: rows,
  };
}
function normalizeShutdownExitDelayMs(value) {
  const num = Number(value);
  if (!Number.isFinite(num)) return DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS;
  if (!Number.isSafeInteger(Math.floor(num))) return DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS;
  return Math.max(
    DASHBOARD_SHUTDOWN_EXIT_DELAY_MIN_MS,
    Math.min(DASHBOARD_SHUTDOWN_EXIT_DELAY_MAX_MS, Math.floor(num)),
  );
}
function normalizeArgs(argv = process.argv.slice(2)) { return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : []; }
function defaultApiPort(port) {
  if (port + BACKEND_PORT_OFFSET <= 65535) return port + BACKEND_PORT_OFFSET;
  if (port - BACKEND_PORT_OFFSET >= 1) return port - BACKEND_PORT_OFFSET;
  return port === 65535 ? 65534 : port + 1;
}
function parseFlags(argv = []) {
  const out = {
    mode: 'serve',
    host: DEFAULT_HOST,
    port: DEFAULT_PORT,
    team: DEFAULT_TEAM,
    refreshMs: DEFAULT_REFRESH_MS,
    pretty: true,
    apiHost: '',
    apiPort: 0,
    apiReadyTimeoutMs: DEFAULT_BACKEND_READY_TIMEOUT_MS,
    uiMode: cleanText(process.env.INFRING_DASHBOARD_UI || 'primary', 24).toLowerCase(),
  };
  let modeSet = false;
  for (const token of argv) {
    const value = String(token || '').trim();
    if (!value) continue;
    if (!modeSet && !value.startsWith('--')) { out.mode = value.toLowerCase(); modeSet = true; continue; }
    if (value.startsWith('--host=')) out.host = cleanText(value.slice(7), 100) || DEFAULT_HOST;
    else if (value.startsWith('--port=')) out.port = parsePositiveInt(value.slice(7), DEFAULT_PORT);
    else if (value.startsWith('--team=')) out.team = cleanText(value.slice(7), 80) || DEFAULT_TEAM;
    else if (value.startsWith('--refresh-ms=')) out.refreshMs = parsePositiveInt(value.slice(13), DEFAULT_REFRESH_MS, 800, 60000);
    else if (value.startsWith('--api-host=')) out.apiHost = cleanText(value.slice(11), 100);
    else if (value.startsWith('--backend-host=')) out.apiHost = cleanText(value.slice(15), 100);
    else if (value.startsWith('--api-port=')) out.apiPort = parsePositiveInt(value.slice(11), 0);
    else if (value.startsWith('--backend-port=')) out.apiPort = parsePositiveInt(value.slice(15), 0);
    else if (value.startsWith('--api-ready-timeout-ms=')) out.apiReadyTimeoutMs = parsePositiveInt(value.slice(23), DEFAULT_BACKEND_READY_TIMEOUT_MS, 1500, 300000);
    else if (value.startsWith('--ui=')) out.uiMode = cleanText(value.slice(5), 24).toLowerCase();
    else if (value === '--pretty=0' || value === '--pretty=false') out.pretty = false;
  }
  out.uiMode = 'primary';
  out.apiHost = out.apiHost || out.host;
  out.apiPort = out.apiPort || defaultApiPort(out.port);
  if (out.apiPort === out.port) out.apiPort = defaultApiPort(out.port + 1);
  return out;
}
function ensureDir(dirPath) { fs.mkdirSync(dirPath, { recursive: true }); }
function writeJson(filePath, value) { ensureDir(path.dirname(filePath)); fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8'); }
function writeJsonIfMissing(filePath, value) {
  if (fs.existsSync(filePath)) return false;
  writeJson(filePath, value);
  return true;
}
function appendJsonl(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}
function deterministicReceiptHash(value) {
  try {
    return createHash('sha256').update(JSON.stringify(value)).digest('hex');
  } catch {
    return '';
  }
}
function readRecentActionRows(limit = TROUBLESHOOTING_MAX_RECENT) {
  const historyPath = path.resolve(STATUS_DIR, 'actions', 'history.jsonl');
  let raw = '';
  try {
    raw = fs.readFileSync(historyPath, 'utf8');
  } catch {
    return [];
  }
  const lines = raw
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (!lines.length) return [];
  const out = [];
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    let parsed = null;
    try {
      parsed = JSON.parse(lines[idx]);
    } catch {
      parsed = null;
    }
    if (!parsed || cleanText(parsed.action || '', 80) !== 'app.chat') continue;
    out.push(parsed);
    if (out.length >= limit) break;
  }
  return out.reverse();
}
function summarizeBootstrapActionRow(actionRow, previousSummary) {
  const laneOk = actionRow?.ok === true;
  const laneStatus = Number.isFinite(Number(actionRow?.lane_status))
    ? Number(actionRow.lane_status)
    : laneOk
      ? 0
      : 1;
  const payload = actionRow && typeof actionRow.payload === 'object' && actionRow.payload
    ? actionRow.payload
    : {};
  const input = cleanText(payload.input || payload.message || payload.prompt || '', 240);
  const tools = Array.isArray(actionRow?.tool_receipts)
    ? actionRow.tool_receipts
    : Array.isArray(payload?.tool_receipts)
      ? payload.tool_receipts
      : [];
  const toolSummary = tools
    .slice(0, 3)
    .map((row) => cleanText(row?.name || row?.tool || '', 40))
    .filter(Boolean)
    .join(',');
  const laneLabel = laneOk ? 'lane_ok' : `lane_fail(${laneStatus})`;
  const inputLabel = input ? `input:${cleanText(input, 64)}` : 'input:empty';
  const toolLabel = toolSummary ? `tools:${toolSummary}` : 'tools:none';
  const summary = `${laneLabel};${inputLabel};${toolLabel}`;
  if (!previousSummary) return summary;
  return `${summary};prev:${cleanText(previousSummary, 120)}`;
}
function bootstrapRecentWorkflowEntries() {
  const actionRows = readRecentActionRows(TROUBLESHOOTING_MAX_RECENT);
  const entries = [];
  let previousSummary = '';
  for (let index = 0; index < actionRows.length; index += 1) {
    const row = actionRows[index] || {};
    const payload = row && typeof row.payload === 'object' && row.payload ? row.payload : {};
    const laneOk = row?.ok === true;
    const laneStatus = Number.isFinite(Number(row?.lane_status))
      ? Number(row.lane_status)
      : laneOk
        ? 0
        : 1;
    const summary = summarizeBootstrapActionRow(row, previousSummary);
    previousSummary = summary;
    const entry = {
      workflow_id: cleanText(row?.id || `wf_${index + 1}`, 120) || `wf_${index + 1}`,
      source_sequence: index + 1,
      ts: cleanText(row?.ts || nowIso(), 80),
      lane_ok: laneOk,
      lane_status: laneStatus,
      error_code: cleanText(row?.error_code || row?.error || '', 120).toLowerCase(),
      exchange: {
        user: cleanText(payload.input || payload.message || payload.prompt || '', 1600),
        assistant: cleanText(row?.response || payload.response || '', 2000),
        tool_receipts: Array.isArray(row?.tool_receipts)
          ? row.tool_receipts.slice(0, 12)
          : [],
      },
      process_summary: {
        previous: cleanText(index === 0 ? '' : entries[index - 1]?.process_summary?.current || '', 360),
        current: cleanText(summary, 360),
        source: 'snapshot_compat_bootstrap',
      },
      metadata: {
        source: 'snapshot_compat_bootstrap',
      },
    };
    entry.receipt_hash = deterministicReceiptHash(entry);
    entries.push(entry);
  }
  return entries;
}
function writeBridgeOutput(out) {
  if (!out || typeof out !== 'object') return 1;
  if (out.stdout) process.stdout.write(String(out.stdout));
  if (out.stderr) process.stderr.write(String(out.stderr));
  if (out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  const status = Number(out.status);
  return Number.isFinite(status) ? status : 1;
}
function bootstrapTroubleshootingFromSnapshot(snapshotPayload) {
  const payload = snapshotPayload && typeof snapshotPayload === 'object' ? snapshotPayload : {};
  const seededEntries = bootstrapRecentWorkflowEntries();
  writeJsonIfMissing(TROUBLESHOOTING_RECENT_WORKFLOWS_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_recent_workflows',
    ts: nowIso(),
    entries: seededEntries,
    receipt_hash: deterministicReceiptHash({
      entries: seededEntries,
      type: 'dashboard_troubleshooting_recent_workflows',
    }),
  });
  writeJsonIfMissing(TROUBLESHOOTING_EVAL_QUEUE_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_eval_queue',
    ts: nowIso(),
    items: [],
    receipt_hash: deterministicReceiptHash({
      items: [],
      type: 'dashboard_troubleshooting_eval_queue',
    }),
  });
  writeJsonIfMissing(TROUBLESHOOTING_ISSUE_OUTBOX_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_issue_outbox',
    ts: nowIso(),
    items: [],
    receipt_hash: deterministicReceiptHash({
      items: [],
      type: 'dashboard_troubleshooting_issue_outbox',
    }),
  });
  if (!fs.existsSync(TROUBLESHOOTING_LATEST_SNAPSHOT_PATH)) {
    const failureCount = seededEntries.filter((row) => row?.lane_ok !== true).length;
    const snapshot = {
      ok: true,
      type: 'dashboard_troubleshooting_snapshot',
      snapshot_id: `snap_${Date.now().toString(36)}`,
      trigger: 'runtime_bootstrap_compat',
      ts: nowIso(),
      failure_count: failureCount,
      entry_count: seededEntries.length,
      entries: seededEntries,
      metadata: {
        source: 'dashboard_snapshot_compat_bootstrap',
        snapshot_receipt_hash: cleanText(payload.receipt_hash || '', 160),
      },
    };
    snapshot.receipt_hash = deterministicReceiptHash(snapshot);
    writeJson(TROUBLESHOOTING_LATEST_SNAPSHOT_PATH, snapshot);
    appendJsonl(TROUBLESHOOTING_SNAPSHOT_HISTORY_PATH, snapshot);
  }
  writeJsonIfMissing(TROUBLESHOOTING_LATEST_EVAL_REPORT_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_eval_report',
    ts: nowIso(),
    status: 'idle',
    reason: 'runtime_bootstrap_compat',
    model: TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
    model_source: 'strong_default_bootstrap',
    strong_default_model: TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
    entry_count: seededEntries.length,
    issues: [],
    summary: 'Eval runtime is initialized and waiting for failure snapshots.',
    receipt_hash: deterministicReceiptHash({
      status: 'idle',
      model: TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
      entry_count: seededEntries.length,
      type: 'dashboard_troubleshooting_eval_report',
    }),
  });
}
function runSnapshotWithCompatBootstrap(args, options) {
  const out = invokeInfringOpsViaBridge(['dashboard-ui', ...args], options);
  if (!out) {
    const status = runInfringOps(['dashboard-ui', ...args], options);
    if (Number(status) === 0 && fs.existsSync(STATUS_SNAPSHOT_PATH)) {
      try {
        const fallbackPayload = JSON.parse(fs.readFileSync(STATUS_SNAPSHOT_PATH, 'utf8'));
        if (fallbackPayload && typeof fallbackPayload === 'object') {
          bootstrapTroubleshootingFromSnapshot(fallbackPayload);
        }
      } catch {}
    }
    return status;
  }
  const parsedPayload = out.payload && typeof out.payload === 'object'
    ? out.payload
    : parseLastJson(out.stdout || '');
  if (parsedPayload && typeof parsedPayload === 'object') {
    bootstrapTroubleshootingFromSnapshot(parsedPayload);
    if (!out.payload) out.payload = parsedPayload;
  }
  return writeBridgeOutput(out);
}
function discoverSiblingAltDashboardSurfaces() {
  const out = [];
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return out; }
  for (const entry of rows) {
    if (!entry || typeof entry.isDirectory !== 'function' || !entry.isDirectory()) continue;
    const dirPath = path.resolve(DASHBOARD_DIR, String(entry.name || ''));
    if (!dirPath || dirPath === STATIC_DIR) continue;
    const dirName = path.basename(dirPath);
    const hasInlineDashboardRoot = hasPrimaryDashboardUi(dirPath);
    const hasBuildIndex = fs.existsSync(path.resolve(dirPath, 'build', 'index.html'));
    const hasIndexHtml = fs.existsSync(path.resolve(dirPath, 'index.html'));
    if (SIBLING_ALT_DASHBOARD_PATTERN.test(dirName) || hasInlineDashboardRoot || hasBuildIndex || hasIndexHtml) out.push(dirPath);
  }
  return out;
}
function assertNoAlternateDashboardSurfaces() {
  const found = new Set();
  FORBIDDEN_ALT_DASHBOARD_DIRS.filter((dirPath) => fs.existsSync(dirPath)).forEach((dirPath) => found.add(dirPath));
  discoverSiblingAltDashboardSurfaces().forEach((dirPath) => found.add(dirPath));
  if (found.size === 0) return;
  const labels = Array.from(found).map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
  throw new Error(`forbidden_dashboard_surface_present:${labels}`);
}
function assertSingleDashboardRoot() {
  if (!hasPrimaryDashboardUi(STATIC_DIR)) throw new Error('primary_dashboard_ui_missing');
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return; }
  const duplicateRoots = rows
    .filter((entry) => entry && typeof entry.isDirectory === 'function' && entry.isDirectory())
    .map((entry) => path.resolve(DASHBOARD_DIR, String(entry.name || '')))
    .filter((dirPath) => dirPath !== STATIC_DIR && hasPrimaryDashboardUi(dirPath));
  if (!duplicateRoots.length) return;
  const labels = duplicateRoots.map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
  throw new Error(`multiple_dashboard_roots_detected:${labels}`);
}
function assertDashboardSurfaceLocked() {
  assertNoAlternateDashboardSurfaces();
  assertSingleDashboardRoot();
}
function backendBase(flags) { return `http://${flags.apiHost}:${flags.apiPort}`; }
async function sleep(ms) { await new Promise((resolve) => setTimeout(resolve, ms)); }
async function fetchBackend(flags, pathname, init = {}, timeoutMs = 15000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try { return await fetch(`${backendBase(flags)}${pathname}`, { ...init, signal: controller.signal }); }
  finally { clearTimeout(timer); }
}
async function fetchBackendJson(flags, pathname, timeoutMs = 15000, traceId = '') {
  const cleanTraceId = sanitizeTraceId(traceId);
  const init = cleanTraceId
    ? { cache: 'no-store', headers: { 'x-infring-trace-id': cleanTraceId } }
    : { cache: 'no-store' };
  const res = await fetchBackend(flags, pathname, init, timeoutMs);
  if (!res.ok) throw new Error(`backend_http_${pathname}_${res.status}`);
  return res.json();
}
async function backendHealth(flags, timeoutMs = 5000) {
  try { return (await fetchBackend(flags, '/healthz', {}, timeoutMs)).ok; } catch { return false; }
}
function backendSpawnEnv() { return backendSpawnEnvForRoot(ROOT, process.env); }
function backendFreshnessSnapshot(flags) {
  return backendFreshnessSnapshotFromProcess(flags, { root: ROOT, resolveBinary, env: backendSpawnEnv() });
}
async function waitForBackendDown(flags, timeoutMs = 6000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!(await backendHealth(flags, 800))) return true;
    await sleep(150);
  }
  return !(await backendHealth(flags, 800));
}
async function stopStaleBackend(flags, freshness) {
  const rows = freshness && Array.isArray(freshness.listener_pids) ? freshness.listener_pids : [];
  const pids = rows.map((row) => Number(row && row.pid)).filter((pid) => Number.isInteger(pid) && pid > 0);
  if (!pids.length) return false;
  for (const pid of pids) {
    try { process.kill(pid, 'SIGTERM'); } catch {}
  }
  if (await waitForBackendDown(flags)) return true;
  for (const pid of pids) {
    try { process.kill(pid, 'SIGKILL'); } catch {}
  }
  return waitForBackendDown(flags);
}
async function statusPayloadWithBootStage(flags) {
  const startedAt = Date.now();
  const healthOk = await backendHealth(flags, 1200);
  if (!healthOk) {
    return {
      ok: false,
      error: 'backend_unreachable',
      connected: false,
      connection_state: 'disconnected',
      boot_stage: 'backend_unreachable',
      backend_health_ok: false,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
  try {
    const status = await fetchBackendJson(flags, '/api/status', 1800);
    const base = (status && typeof status === 'object') ? status : {};
    const connected = base.connected !== false;
    const degraded = !!base.degraded || base.ok === false;
    const out = {
      ...base,
      ok: connected,
      connected,
      degraded,
      connection_state: connected ? 'connected' : 'disconnected',
      boot_stage: cleanText(base.boot_stage || base.last_stage || (degraded ? 'status_degraded' : 'ready'), 60),
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
    };
    if (!out.error && degraded) out.error = 'status_degraded';
    return out;
  } catch {
    return {
      ok: true,
      degraded: true,
      warning: 'status_unavailable',
      connected: true,
      connection_state: 'connected',
      boot_stage: 'backend_ready_status_probe_timeout',
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
}
function spawnBackend(flags) {
  const laneArgs = ['dashboard-ui', 'serve', `--host=${flags.apiHost}`, `--port=${flags.apiPort}`, `--team=${flags.team}`, `--refresh-ms=${flags.refreshMs}`];
  const env = backendSpawnEnv();
  const bin = resolveBinary({ env });
  if (!bin) throw new Error('dashboard_backend_binary_missing');
  const child = spawn(bin, laneArgs, { cwd: ROOT, env, stdio: ['ignore', 'pipe', 'pipe'] });
  if (child.stdout) child.stdout.on('data', (chunk) => process.stdout.write(chunk));
  if (child.stderr) child.stderr.on('data', (chunk) => process.stderr.write(chunk));
  return child;
}
async function ensureBackend(flags) {
  if (await backendHealth(flags, 1500)) {
    const freshness = backendFreshnessSnapshot(flags);
    if (!freshness.stale) return { child: null, reused: true, freshness };
    if (!shouldRestartStaleBackend()) return { child: null, reused: true, freshness };
    const stopped = await stopStaleBackend(flags, freshness);
    if (!stopped) return { child: null, reused: true, freshness: { ...freshness, restart_failed: true } };
  }
  const child = spawnBackend(flags);
  const deadline = Date.now() + flags.apiReadyTimeoutMs;
  while (Date.now() < deadline) {
    if (await backendHealth(flags, 1500)) return { child, reused: false, freshness: backendFreshnessSnapshot(flags) };
    if (child.exitCode != null) throw new Error(`dashboard_backend_exit:${child.exitCode}`);
    await sleep(250);
  }
  try { child.kill('SIGTERM'); } catch {}
  throw new Error('dashboard_backend_timeout');
}
function sanitizeTraceId(value) {
  const raw = Array.isArray(value) ? value[0] : value;
  const clean = String(raw || '').trim();
  return /^[A-Za-z0-9_.:-]{8,160}$/.test(clean) ? clean : '';
}
function requestTraceId(req) {
  if (req.__infringTraceId) return req.__infringTraceId;
  const rawHeader = req.headers && (req.headers['x-infring-trace-id'] || req.headers['traceparent']);
  const existing = sanitizeTraceId(rawHeader);
  const minted = `trace_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 14)}`;
  req.__infringTraceId = existing || minted;
  req.__infringTraceBoundary = {
    trace_id: req.__infringTraceId,
    source: existing ? 'incoming_header' : 'gateway_minted',
    gateway_boundary: 'adapters.runtime.infring_dashboard',
  };
  return req.__infringTraceId;
}
function requestTraceBoundary(req) {
  requestTraceId(req);
  return req.__infringTraceBoundary || { trace_id: req.__infringTraceId || '', source: 'unknown' };
}
function sendJson(res, statusCode, value) {
  res.writeHead(statusCode, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
  res.end(`${JSON.stringify(value, null, 2)}\n`);
}
function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}
function readJsonBody(req, maxBytes = 65536) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0;
    ignoreStreamErrors(req);
    req.on('data', (chunk) => {
      const next = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      total += next.length;
      if (total > maxBytes) {
        reject(new Error('request_body_too_large'));
        return;
      }
      chunks.push(next);
    });
    req.on('end', () => {
      if (!chunks.length) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'));
      } catch {
        reject(new Error('request_body_invalid_json'));
      }
    });
    req.on('error', reject);
  });
}
function currentDashboardBuildInfo() {
  return readBuildVersionInfo(STATIC_DIR);
}
function mergeDashboardVersionPayload(payload) {
  const base = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  const build = currentDashboardBuildInfo();
  const version = cleanText(build && build.version, 120) || '0.0.0';
  const tag = cleanText(build && build.tag, 120) || `v${version}`;
  const source = cleanText(build && build.source, 80) || 'fallback_default';
  return {
    ...base,
    ok: base.ok !== false,
    version,
    tag,
    version_tag: tag,
    source,
    version_source: source,
    platform: base.platform || process.platform,
    arch: base.arch || process.arch,
  };
}
function filteredHeaders(headers, host, traceId = '') {
  const out = {};
  for (const [key, value] of Object.entries(headers || {})) {
    if (!value || HOP_BY_HOP.has(String(key).toLowerCase())) continue;
    out[key] = value;
  }
  out.host = host;
  if (traceId) out['x-infring-trace-id'] = traceId;
  return out;
}
function dashboardSystemActionArgs(action, payload = {}) {
  const normalized = cleanText(action, 40).toLowerCase();
  const body = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  if (normalized === 'restart') return ['restart', '--json'];
  if (normalized === 'shutdown') return ['stop', '--json'];
  if (normalized === 'update') {
    const args = ['update', '--json'];
    if (body.force === true) args.push('--force');
    if (body.apply !== false) args.push('--apply');
    return args;
  }
  throw new Error(`unknown_dashboard_system_action:${normalized}`);
}
function dashboardSystemActionEnv() {
  return {
    ...process.env,
    INFRING_ROOT: ROOT,
    INFRING_OPS_ALLOW_STALE: process.env.INFRING_OPS_ALLOW_STALE || '1',
    INFRING_NPM_ALLOW_STALE: process.env.INFRING_NPM_ALLOW_STALE || '1',
  };
}
function runDashboardSystemAction(action, payload = {}) {
  const args = dashboardSystemActionArgs(action, payload);
  const run =
    invokeInfringOpsViaBridge(args, {
      allowProcessFallback: false,
      unknownDomainFallback: false,
    }) || {
      status: 1,
      stdout: '',
      stderr: 'resident_ipc_bridge_unavailable',
      payload: null,
    };
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  const receipt = (run && run.payload && typeof run.payload === 'object') ? run.payload : parseLastJson(run.stdout);
  const ok = status === 0 && (!receipt || receipt.ok !== false);
  const error = ok
    ? ''
    : cleanText(
        (receipt && receipt.error) || run.stderr || run.stdout || `${cleanText(action, 40).toLowerCase()}_failed`,
        260,
      );
  return {
    ok,
    type: 'dashboard_system_action',
    action: cleanText(action, 40).toLowerCase(),
    command: args[0],
    args: args.slice(1),
    exit_code: status,
    payload: receipt || null,
    error,
  };
}
function dispatchDashboardSystemAction(action, payload = {}) {
  const args = dashboardSystemActionArgs(action, payload);
  const env = dashboardSystemActionEnv();
  const bin = resolveBinary({ env });
  if (!bin) {
    return {
      ok: false,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: '',
      args: args.slice(1),
      error: 'dashboard_backend_binary_missing',
    };
  }
  try {
    const child = spawn(bin, args, {
      cwd: ROOT,
      env,
      detached: true,
      stdio: 'ignore',
    });
    if (child && typeof child.unref === 'function') child.unref();
    return {
      ok: true,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: path.basename(bin),
      args: args.slice(1),
      dispatch_mode: 'detached_subprocess',
      pid: Number(child && child.pid) || 0,
      payload: null,
      error: '',
    };
  } catch (error) {
    return {
      ok: false,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: path.basename(String(bin || '')),
      args: args.slice(1),
      error: cleanText(error && error.message ? error.message : String(error), 260),
    };
  }
}
function scheduleDashboardHostExit(cleanup, normalizedDelayMs = DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS) {
  const waitMs = normalizeShutdownExitDelayMs(normalizedDelayMs);
  setTimeout(() => {
    try { cleanup(); } catch {}
    setTimeout(() => {
      try { process.exit(0); } catch {}
    }, 0);
  }, waitMs);
}
function proxyToBackend(req, res, flags) {
  return new Promise((resolve, reject) => {
    ignoreStreamErrors(req);
    ignoreStreamErrors(res);
    ignoreStreamErrors(req.socket);
    ignoreStreamErrors(res.socket);
    const upstream = http.request({ host: flags.apiHost, port: flags.apiPort, method: req.method || 'GET', path: req.url || '/', headers: filteredHeaders(req.headers, `${flags.apiHost}:${flags.apiPort}`, requestTraceId(req)) }, (upstreamRes) => {
      ignoreStreamErrors(upstreamRes);
      ignoreStreamErrors(upstreamRes.socket);
      res.writeHead(upstreamRes.statusCode || 502, upstreamRes.headers);
      upstreamRes.pipe(res);
      upstreamRes.on('end', resolve);
      upstreamRes.on('error', reject);
    });
    ignoreStreamErrors(upstream);
    upstream.on('error', reject);
    req.pipe(upstream);
  });
}
function proxyUpgrade(req, socket, head, flags) {
  ignoreStreamErrors(req);
  ignoreStreamErrors(req.socket);
  ignoreStreamErrors(socket);
  const upstream = http.request({
    host: flags.apiHost,
    port: flags.apiPort,
    path: req.url || '/',
    headers: { ...filteredHeaders(req.headers, `${flags.apiHost}:${flags.apiPort}`, requestTraceId(req)), connection: 'Upgrade', upgrade: req.headers.upgrade || 'websocket' },
  });
  upstream.on('upgrade', (upstreamRes, upstreamSocket, upstreamHead) => {
    ignoreStreamErrors(upstreamRes);
    ignoreStreamErrors(upstreamSocket);
    const headerLines = [`HTTP/1.1 ${upstreamRes.statusCode || 101} ${upstreamRes.statusMessage || 'Switching Protocols'}`];
    for (const [key, value] of Object.entries(upstreamRes.headers || {})) {
      if (Array.isArray(value)) value.forEach((entry) => headerLines.push(`${key}: ${entry}`));
      else if (value != null) headerLines.push(`${key}: ${value}`);
    }
    socket.write(`${headerLines.join('\r\n')}\r\n\r\n`);
    if (head && head.length) upstreamSocket.write(head);
    if (upstreamHead && upstreamHead.length) socket.write(upstreamHead);
    upstreamSocket.pipe(socket).pipe(upstreamSocket);
  });
  upstream.on('response', (upstreamRes) => {
    ignoreStreamErrors(upstreamRes);
    socket.write(`HTTP/1.1 ${upstreamRes.statusCode || 502} ${upstreamRes.statusMessage || 'Bad Gateway'}\r\nConnection: close\r\n\r\n`);
    upstreamRes.pipe(socket);
  });
  upstream.on('error', () => { try { socket.destroy(); } catch {} });
  upstream.end();
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
      if (req.method === 'GET' && pathname === '/api/system/release-check') {
        const qs = requestUrl.search || '';
        const payload = await fetchBackendJson(flags, `/api/update/check${qs}`, 5000).catch(() => ({
          ok: true,
          update_available: false,
        }));
        return void sendJson(res, 200, payload);
      }
      if (req.method === 'GET' && (pathname === '/api/shell-socket/agent-runtime/engines' || pathname === '/api/agent-runtime/engines')) {
        const payload = await agentRuntimeEnginesProjection(traceId).catch((error) => ({
          ok: false,
          type: 'agent_runtime_engines_projection_error',
          trace_id: traceId,
          error: cleanText(error && error.message ? error.message : error, 240),
          engines: [],
        }));
        return void sendJson(res, payload.ok === false ? 503 : 200, payload);
      }
      if (req.method === 'GET' && isShellSocketChatProjectionPath(pathname)) {
        const result = await shellSocketChatProjection({ flags, requestUrl, traceId, fetchBackendJson });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'GET' && isShellSocketStatusProjectionPath(pathname)) {
        const result = await shellSocketStatusProjection({ flags, traceId, fetchBackendJson, statusPayloadWithBootStage });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'POST' && isShellSocketCommandIngressPath(pathname)) {
        const body = await readJsonBody(req, 65536);
        const result = await shellSocketCommandIngress({ flags, requestUrl, traceId, body, fetchBackend });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'POST' && isShellSocketEvalIssueIngressPath(pathname)) {
        const body = await readJsonBody(req, 65536);
        const result = await shellSocketEvalIssueIngress({ flags, traceId, body, fetchBackend });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'POST' && isShellSocketLifecycleIngressPath(pathname)) {
        const body = await readJsonBody(req, 65536);
        const result = await shellSocketLifecycleIngress({ flags, requestUrl, traceId, body, fetchBackend });
        return void sendJson(res, result.status, result.payload);
      }
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
      if (req.method === 'GET' && pathname === '/api/config') {
        const config = await fetchBackendJson(flags, '/api/config', 8000).catch(() => ({ ok: false, error: 'config_unavailable' }));
        return void sendJson(res, 200, config);
      }
      if (req.method === 'GET' && pathname === '/api/config/schema') {
        const schema = await fetchBackendJson(flags, '/api/config/schema', 8000).catch(() => ({ ok: true, sections: {} }));
        return void sendJson(res, 200, schema);
      }
      if (req.method === 'GET' && pathname === '/api/auth/check') {
        const auth = await fetchBackendJson(flags, '/api/auth/check', 8000).catch(() => ({ ok: true, mode: 'none', authenticated: true, user: 'operator' }));
        return void sendJson(res, 200, auth);
      }
      if (req.method === 'POST' && pathname === '/api/system/restart') {
        const body = await readJsonBody(req);
        const result = dispatchDashboardSystemAction('restart', body);
        return void sendJson(res, result.ok ? 200 : 500, result);
      }
      if (req.method === 'POST' && pathname === '/api/system/update') {
        const body = await readJsonBody(req);
        try {
          const upstream = await fetchBackend(flags, '/api/system/update', {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(body || {})
          }, body && body.apply === false ? 8000 : 3500);
          const text = await upstream.text();
          let payload = {};
          try {
            payload = text ? JSON.parse(text) : {};
          } catch {
            payload = {};
          }
          return void sendJson(
            res,
            upstream.status || ((payload && payload.ok === false) ? 400 : 200),
            payload && typeof payload === 'object' ? payload : { ok: upstream.ok }
          );
        } catch (_) {
          const result = runDashboardSystemAction('update', body);
          return void sendJson(res, result.ok ? 200 : 500, result);
        }
      }
      if (req.method === 'POST' && pathname === '/api/system/shutdown') {
        const body = await readJsonBody(req);
        const result = dispatchDashboardSystemAction('shutdown', body);
        sendJson(res, result.ok ? 200 : 500, result);
        if (result.ok) {
          const exitDelayMs = normalizeShutdownExitDelayMs(body && body.exit_delay_ms);
          scheduleDashboardHostExit(cleanup, exitDelayMs);
        }
        return;
      }
      if (req.method === 'GET') {
        const asset = readPrimaryDashboardAsset(STATIC_DIR, pathname);
        if (asset) {
          res.writeHead(200, { 'content-type': asset.contentType, 'cache-control': 'no-store' });
          res.end(asset.body);
          return;
        }
      }
      if (pathname === '/healthz' || pathname.startsWith('/api/')) return void await proxyToBackend(req, res, flags);
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
    proxyUpgrade(req, socket, head, flags);
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
  dashboardSystemActionArgs,
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
