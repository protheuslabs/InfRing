#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

// Proves the canonical Agent Runtime WebSocket works against a real running
// Gateway host process. Unlike the deterministic in-process transport guard,
// this guard connects to the live dashboard/Gateway port and fails clearly if
// the host is not running or not wired to /ws/agent-runtime.

const crypto = require('node:crypto');
const fs = require('node:fs');
const http = require('node:http');
const net = require('node:net');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_socket_live_gateway_guard_current.json');
const STATUS_PATH = path.join(ROOT, 'client/runtime/local/state/ui/infring_dashboard/server_status.json');
const DEFAULT_HOST = process.env.INFRING_GATEWAY_HOST || '127.0.0.1';
const DEFAULT_PORT = Number(process.env.INFRING_GATEWAY_PORT || process.env.INFRING_DASHBOARD_PORT || 4173);
const TIMEOUT_MS = Number(process.env.INFRING_AGENT_RUNTIME_LIVE_GATEWAY_TIMEOUT_MS || 7000);
const SERVER_FRAME_MAX_BYTES = Number(process.env.INFRING_AGENT_RUNTIME_LIVE_GATEWAY_SERVER_FRAME_MAX_BYTES || 262144);

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function probeDashboardRoute(target, routePath = '/dashboard') {
  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      resolve({ ok: false, status_code: null, error: 'dashboard_route_timeout' });
    }, Math.min(Math.max(600, TIMEOUT_MS), 3000));
    const req = http.get({
      host: target.host,
      port: target.port,
      path: routePath,
      timeout: Math.min(Math.max(500, TIMEOUT_MS), 2000),
    }, (res) => {
      res.resume();
      const status = Number(res.statusCode || 0);
      clearTimeout(timeout);
      resolve({ ok: status >= 200 && status < 400, status_code: status });
    });
    req.on('error', (error) => {
      clearTimeout(timeout);
      resolve({ ok: false, status_code: null, error: clean(error && error.message ? error.message : error, 1200) });
    });
  });
}

function readStatusTarget() {
  if (process.env.INFRING_GATEWAY_HOST || process.env.INFRING_GATEWAY_PORT || process.env.INFRING_DASHBOARD_PORT) {
    return { host: DEFAULT_HOST, port: DEFAULT_PORT, source: 'explicit_env' };
  }
  try {
    const status = JSON.parse(fs.readFileSync(STATUS_PATH, 'utf8'));
    const url = status && status.url ? new URL(status.url) : null;
    const backendUrl = status && typeof status.backend_url === 'string' ? status.backend_url : null;
    const backendUrlObj = backendUrl ? new URL(backendUrl) : null;
    const host = clean((status && status.host) || (url && url.hostname) || (backendUrlObj && backendUrlObj.hostname) || DEFAULT_HOST, 120) || DEFAULT_HOST;
    const backendPort = backendUrlObj ? Number(backendUrlObj.port) : NaN;
    const port = Number((status && status.port) || (url && url.port) || DEFAULT_PORT);
    return {
      host,
      port,
      source: 'dashboard_status_file',
      status_path: path.relative(ROOT, STATUS_PATH),
      backend_url: backendUrl || undefined,
      status_pid: Number(status && status.process_id),
      backend_port: Number.isFinite(backendPort) ? backendPort : undefined,
    };
  } catch {
    return { host: DEFAULT_HOST, port: DEFAULT_PORT, source: 'default_or_env' };
  }
}

function writeReport(report) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
}

function fail(kind, extra = {}) {
  const report = {
    ok: false,
    type: 'agent_runtime_socket_live_gateway_guard',
    generated_at: new Date().toISOString(),
    failure_kind: kind,
    ...extra,
  };
  writeReport(report);
  process.exit(1);
}

async function waitForConnect(socket) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('connect_timeout')), TIMEOUT_MS);
    socket.once('connect', () => {
      clearTimeout(timer);
      resolve(null);
    });
    socket.once('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

function createLiveReader(socket) {
  const { decodeAgentRuntimeSocketFrames } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_socket_transport.ts'));
  let raw = Buffer.alloc(0);
  let frameBuffer = Buffer.alloc(0);
  let handshakeDone = false;
  const events = [];
  const waiters = [];
  const removeWaiter = (waiter) => {
    const index = waiters.indexOf(waiter);
    if (index >= 0) waiters.splice(index, 1);
  };
  const rejectWaiter = (waiter, error) => {
    clearTimeout(waiter.timer);
    removeWaiter(waiter);
    waiter.reject(error);
  };
  const rejectAll = (error) => {
    for (const waiter of [...waiters]) rejectWaiter(waiter, error);
  };
  const maybeResolve = (event) => {
    for (const waiter of [...waiters]) {
      let matched = false;
      try { matched = waiter.predicate(event, events); } catch (error) { rejectWaiter(waiter, error); continue; }
      if (!matched) continue;
      clearTimeout(waiter.timer);
      removeWaiter(waiter);
      waiter.resolve({ event, events: [...events] });
    }
  };
  const onData = (chunk) => {
    raw = Buffer.concat([raw, Buffer.from(chunk || '')]);
    if (!handshakeDone) {
      const marker = raw.indexOf(Buffer.from('\r\n\r\n'));
      if (marker < 0) return;
      const header = raw.subarray(0, marker + 4).toString('utf8');
      if (!/^HTTP\/1\.1 101\b/.test(header)) {
        rejectAll(new Error(`websocket_handshake_failed:${header.split(/\r?\n/)[0]}`));
        return;
      }
      handshakeDone = true;
      frameBuffer = raw.subarray(marker + 4);
      raw = Buffer.alloc(0);
    } else {
      frameBuffer = Buffer.concat([frameBuffer, Buffer.from(chunk || '')]);
    }
    const decoded = decodeAgentRuntimeSocketFrames(frameBuffer, {
      expectMasked: false,
      maxBytes: SERVER_FRAME_MAX_BYTES,
    });
    if (!decoded.ok) {
      rejectAll(new Error(decoded.error || 'server_frame_decode_failed'));
      return;
    }
    frameBuffer = decoded.remaining;
    for (const frame of decoded.frames) {
      if (frame.opcode !== 1) continue;
      let parsed = null;
      try { parsed = JSON.parse(frame.text || '{}'); } catch {}
      if (!parsed) continue;
      events.push(parsed);
      maybeResolve(parsed);
    }
  };
  const onError = (error) => rejectAll(error);
  const onClose = () => rejectAll(new Error('socket_closed_before_expected_event'));
  socket.on('data', onData);
  socket.on('error', onError);
  socket.on('close', onClose);
  return {
    waitFor(predicate, label) {
      return new Promise((resolve, reject) => {
        for (const event of events) {
          if (predicate(event, events)) {
            resolve({ event, events: [...events] });
            return;
          }
        }
        const waiter = {
          predicate,
          resolve,
          reject,
          timer: setTimeout(() => rejectWaiter(waiter, new Error(`socket_event_timeout:${label}`)), TIMEOUT_MS),
        };
        waiters.push(waiter);
      });
    },
    close() {
      socket.off('data', onData);
      socket.off('error', onError);
      socket.off('close', onClose);
      rejectAll(new Error('reader_closed'));
    },
  };
}

async function main() {
  const { encodeAgentRuntimeSocketFrame } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_socket_transport.ts'));
  const initialTarget = readStatusTarget();
  const explicitEnv = !!(process.env.INFRING_GATEWAY_HOST || process.env.INFRING_GATEWAY_PORT || process.env.INFRING_DASHBOARD_PORT);
  const fallbackTargets = [];
  if (
    !explicitEnv
    && initialTarget.source === 'dashboard_status_file'
    && Number.isFinite(initialTarget.port)
    && initialTarget.port !== DEFAULT_PORT
  ) {
    fallbackTargets.push({
      host: DEFAULT_HOST,
      port: DEFAULT_PORT,
      source: 'default_port_fallback',
      status_path: initialTarget.status_path,
    });
  }
  if (
    !explicitEnv
    && initialTarget.source === 'dashboard_status_file'
    && Number.isFinite(initialTarget.backend_port)
    && initialTarget.backend_port > 0
    && initialTarget.backend_port !== initialTarget.port
  ) {
    fallbackTargets.push({
      host: initialTarget.host,
      port: initialTarget.backend_port,
      source: 'backend_port_fallback',
      status_path: initialTarget.status_path,
      status_pid: initialTarget.status_pid,
      backend_port: initialTarget.backend_port,
      backend_url: initialTarget.backend_url,
      skip_route_probe: true,
      backend_probe_only: true,
    });
  }

  const candidates = [initialTarget, ...fallbackTargets];
  let target = null;
  let reader = null;
  let socket = null;
  let connectFailure = null;

  for (const candidate of candidates) {
    if (!Number.isFinite(candidate.port) || candidate.port <= 0) {
      connectFailure = { kind: 'invalid_gateway_port', candidate };
      continue;
    }
    if (Number.isFinite(candidate.status_pid) && candidate.status_pid > 0) {
      try {
        process.kill(candidate.status_pid, 0);
      } catch (error) {
        candidate.stale_status_pid = {
          checked: true,
          pid: candidate.status_pid,
          error: clean(error && error.message ? error.message : error, 1800),
        };
      }
    }
    const routeProbe = candidate.skip_route_probe
      ? { ok: true, status_code: 0, note: 'skipped_for_candidate' }
      : await probeDashboardRoute(candidate, candidate.backend_probe_only ? '/' : '/dashboard');
    if (!routeProbe.ok) {
      connectFailure = {
        kind: 'route_probe_failed',
        candidate,
        route_probe: routeProbe,
      };
      continue;
    }
    const nextSocket = net.connect({ host: candidate.host, port: candidate.port });
    try {
      await waitForConnect(nextSocket);
      target = candidate;
      socket = nextSocket;
      break;
    } catch (error) {
      connectFailure = {
        kind: 'connect_failed',
        candidate,
        error: clean(error && error.stack ? error.stack : error, 1800),
      };
      try {
        nextSocket.destroy();
      } catch {}
    }
  }

  if (!target || !socket) {
    fail(connectFailure && connectFailure.kind ? connectFailure.kind : 'no_connectable_gateway_target', {
      candidates,
      connectFailure,
      target: initialTarget,
      recovery: 'Start/restart the Gateway and verify it is serving /dashboard before rerunning npm run -s ops:agent-runtime:socket-live-gateway:guard',
    });
  }

  const traceId = `validation:agent-runtime-live-gateway:${Date.now()}`;
  const requestId = `live-gateway-${Date.now()}`;
  try {
    reader = createLiveReader(socket);
    socket.write([
      'GET /ws/agent-runtime HTTP/1.1',
      `Host: ${target.host}:${target.port}`,
      'Upgrade: websocket',
      'Connection: Upgrade',
      `Sec-WebSocket-Key: ${crypto.randomBytes(16).toString('base64')}`,
      'Sec-WebSocket-Version: 13',
      '',
      '',
    ].join('\r\n'));
    const engineListPromise = reader.waitFor((event) => event && event.type === 'engine.list.result', 'engine.list.result');
    socket.write(encodeAgentRuntimeSocketFrame(JSON.stringify({
      type: 'engine.list',
      trace_id: traceId,
      request_id: `${requestId}-engine-list`,
    }), { mask: true }));
    const engineList = await engineListPromise;
    const heartbeatPromise = reader.waitFor((event) => event && event.type === 'heartbeat', 'heartbeat');
    socket.write(encodeAgentRuntimeSocketFrame(JSON.stringify({
      type: 'heartbeat',
      trace_id: traceId,
      request_id: `${requestId}-heartbeat`,
    }), { mask: true }));
    const heartbeat = await heartbeatPromise;
    const events = [...(engineList.events || []), ...(heartbeat.events || [])];
    const violations = [];
    if (!Array.isArray(engineList.event && engineList.event.engines)) violations.push({ kind: 'engine_rows_missing' });
    if (heartbeat.event && heartbeat.event.trace_id && heartbeat.event.trace_id !== traceId) violations.push({ kind: 'heartbeat_trace_id_replaced', trace_id: heartbeat.event.trace_id });
    for (const event of events) {
      if (event && event.trace_id && event.trace_id !== traceId) violations.push({ kind: 'trace_id_replaced', event_type: event.type, trace_id: event.trace_id });
    }
    const report = {
      ok: violations.length === 0,
      type: 'agent_runtime_socket_live_gateway_guard',
      generated_at: new Date().toISOString(),
      mode: 'live_gateway_websocket_client',
      canonical_socket_route: '/ws/agent-runtime',
      target,
      server_frame_max_bytes: SERVER_FRAME_MAX_BYTES,
      trace_id: traceId,
      event_types: [...new Set(events.map((event) => clean(event && event.type, 120)).filter(Boolean))],
      engine_count: Array.isArray(engineList.event && engineList.event.engines) ? engineList.event.engines.length : 0,
      violations,
    };
    writeReport(report);
    if (!report.ok) process.exitCode = 1;
  } catch (error) {
    fail('live_gateway_socket_probe_failed', {
      target,
      canonical_socket_route: '/ws/agent-runtime',
      error: clean(error && error.stack ? error.stack : error, 6000),
      recovery: 'Start or restart the Gateway, then rerun npm run -s ops:agent-runtime:socket-live-gateway:guard',
    });
  } finally {
    try { if (reader) reader.close(); } catch {}
    try { if (socket) socket.destroy(); } catch {}
  }
}

main();
