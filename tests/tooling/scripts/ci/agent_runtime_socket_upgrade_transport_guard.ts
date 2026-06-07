#!/usr/bin/env node
/* eslint-disable no-console */

// Proves /ws/agent-runtime can be upgraded through the Gateway request boundary
// into the Gateway-owned Agent Runtime socket transport. This uses a local
// in-process HTTP server and raw TCP WebSocket frames so it does not depend on
// the legacy dashboard Shell or browser code.

'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs');
const http = require('node:http');
const net = require('node:net');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_socket_upgrade_transport_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-socket-upgrade-transport-scratch');
const ENGINE_ID = 'codex_cli';
const GUARD_TIMEOUT_MS = 15000;
const SOCKET_EVENT_TIMEOUT_MS = 5000;

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function createUpgradeProofAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'agent_runtime_socket_upgrade_transport_guard',
    }),
    start_session: async ({ message }) => ({ type: 'session.started', trace_id: message && message.trace_id, engine_id: engineId, session_id: message && message.session_id }),
    submit_turn: async ({ message }) => ({ type: 'turn.complete', trace_id: message && message.trace_id, engine_id: engineId, status: 'completed', output_text: 'socket upgrade proof turn completed.' }),
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, engine_id: engineId, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, engine_id: engineId, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, engine_id: engineId, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, engine_id: engineId, receipt_refs: [] }),
  };
}

function startServer() {
  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const { createGatewayDashboardHostRequestBoundary } = require(path.join(ROOT, 'gateway/runtime/gateway_dashboard_host_request_boundary.ts'));
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories: {
      [ENGINE_ID]: () => createUpgradeProofAdapter(ENGINE_ID),
    },
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
      if (typeof res.writeHead === 'function') {
        res.writeHead(statusCode, { 'content-type': 'application/json; charset=utf-8' });
        res.end(`${JSON.stringify(payload)}\n`);
      }
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });
  const boundary = createGatewayDashboardHostRequestBoundary({
    sendJson: (res, statusCode, payload) => {
      res.writeHead(statusCode, { 'content-type': 'application/json; charset=utf-8' });
      res.end(`${JSON.stringify(payload)}\n`);
    },
  });
  const server = http.createServer((_req, res) => {
    res.writeHead(404);
    res.end('not found');
  });
  server.on('upgrade', (req, socket, head) => {
    boundary.handleDashboardUpgrade({
      req,
      socket,
      head,
      agentRuntimeSocketTransport: assembly.agentRuntimeSocketTransport,
      flags: { host: '127.0.0.1', port: 0 },
      requestTraceId: () => '',
    });
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => resolve({ server, port: server.address().port }));
  });
}

function createSocketEventReader(socket, timeoutMs = SOCKET_EVENT_TIMEOUT_MS) {
  const { decodeAgentRuntimeSocketFrames } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_socket_transport.ts'));
  let raw = Buffer.alloc(0);
  let handshakeDone = false;
  let frameBuffer = Buffer.alloc(0);
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
  const failAll = (error) => {
    for (const waiter of [...waiters]) rejectWaiter(waiter, error);
  };
  const maybeResolveWaiters = (event) => {
    for (const waiter of [...waiters]) {
      let matched = false;
      try {
        matched = waiter.predicate(event, events);
      } catch (error) {
        rejectWaiter(waiter, error);
        continue;
      }
      if (matched) {
        clearTimeout(waiter.timer);
        removeWaiter(waiter);
        waiter.resolve({ event, events: [...events] });
      }
    }
  };
  const onData = (chunk) => {
    raw = Buffer.concat([raw, Buffer.from(chunk || '')]);
    if (!handshakeDone) {
      const marker = raw.indexOf(Buffer.from('\r\n\r\n'));
      if (marker < 0) return;
      const header = raw.subarray(0, marker + 4).toString('utf8');
      if (!/^HTTP\/1\.1 101\b/.test(header)) {
        failAll(new Error(`websocket_handshake_failed:${header.split(/\r?\n/)[0]}`));
        return;
      }
      handshakeDone = true;
      frameBuffer = raw.subarray(marker + 4);
      raw = Buffer.alloc(0);
    } else {
      frameBuffer = Buffer.concat([frameBuffer, Buffer.from(chunk || '')]);
    }
    const decoded = decodeAgentRuntimeSocketFrames(frameBuffer, { expectMasked: false });
    if (!decoded.ok) {
      failAll(new Error(decoded.error || 'server_frame_decode_failed'));
      return;
    }
    frameBuffer = decoded.remaining;
    for (const frame of decoded.frames) {
      if (frame.opcode !== 1) continue;
      let parsed = null;
      try { parsed = JSON.parse(frame.text || '{}'); } catch {}
      if (!parsed) continue;
      events.push(parsed);
      maybeResolveWaiters(parsed);
    }
  };
  const onError = (error) => failAll(error);
  const onClose = () => failAll(new Error('socket_closed_before_expected_event'));
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
          timer: setTimeout(() => {
            rejectWaiter(waiter, new Error(`socket_event_timeout:${label || 'event'}`));
          }, timeoutMs),
        };
        waiters.push(waiter);
      });
    },
    close() {
      socket.off('data', onData);
      socket.off('error', onError);
      socket.off('close', onClose);
      failAll(new Error('socket_reader_closed'));
    },
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  const { encodeAgentRuntimeSocketFrame } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_socket_transport.ts'));
  let server = null;
  let socket = null;
  let reader = null;
  const guardTimer = setTimeout(() => {
    const report = {
      ok: false,
      type: 'agent_runtime_socket_upgrade_transport_guard',
      error: `guard_timeout:${GUARD_TIMEOUT_MS}`,
    };
    ensureDir(OUT_JSON);
    fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.error(JSON.stringify(report, null, 2));
    try { if (socket) socket.destroy(); } catch {}
    try { if (server) server.close(() => {}); } catch {}
    process.exit(124);
  }, GUARD_TIMEOUT_MS);
  const started = await startServer();
  server = started.server;
  const port = started.port;
  socket = net.connect({ host: '127.0.0.1', port });
  const key = crypto.randomBytes(16).toString('base64');
  const traceId = `validation:agent-runtime-socket-upgrade:${Date.now()}`;
  const seen = [];
  let exitCode = 0;
  try {
    await new Promise((resolve, reject) => {
      socket.once('connect', resolve);
      socket.once('error', reject);
    });
    reader = createSocketEventReader(socket);
    const engineListPromise = reader.waitFor((event, events) => {
      seen.splice(0, seen.length, ...events);
      return event && event.type === 'engine.list.result';
    }, 'engine.list.result');
    socket.write([
      'GET /ws/agent-runtime HTTP/1.1',
      `Host: 127.0.0.1:${port}`,
      'Upgrade: websocket',
      'Connection: Upgrade',
      `Sec-WebSocket-Key: ${key}`,
      'Sec-WebSocket-Version: 13',
      '',
      '',
    ].join('\r\n'));
    socket.write(encodeAgentRuntimeSocketFrame(JSON.stringify({
      type: 'engine.list',
      trace_id: traceId,
      request_id: 'upgrade-transport-engine-list',
    }), { mask: true }));
    const engineList = await engineListPromise;
    const heartbeatPromise = reader.waitFor((event, events) => {
      seen.splice(0, seen.length, ...events);
      return event && event.type === 'heartbeat';
    }, 'heartbeat');
    socket.write(encodeAgentRuntimeSocketFrame(JSON.stringify({
      type: 'heartbeat',
      trace_id: traceId,
      request_id: 'upgrade-transport-heartbeat',
    }), { mask: true }));
    const heartbeat = await heartbeatPromise;
    const violations = [];
    if (!engineList.event || engineList.event.type !== 'engine.list.result') violations.push({ kind: 'engine_list_result_missing' });
    if (!heartbeat.event || heartbeat.event.type !== 'heartbeat') violations.push({ kind: 'heartbeat_missing' });
    if (engineList.event && !Array.isArray(engineList.event.engines)) violations.push({ kind: 'engine_list_rows_missing' });
    for (const event of seen) {
      if (event.trace_id && event.trace_id !== traceId) violations.push({ kind: 'trace_id_replaced', event_type: event.type, trace_id: event.trace_id });
    }
    const report = {
      ok: violations.length === 0,
      type: 'agent_runtime_socket_upgrade_transport_guard',
      generated_at: new Date().toISOString(),
      mode: 'local_gateway_request_boundary_websocket_upgrade',
      canonical_socket_route: '/ws/agent-runtime',
      port,
      event_types: seen.map((event) => clean(event && event.type, 120)).filter(Boolean),
      engine_count: Array.isArray(engineList.event && engineList.event.engines) ? engineList.event.engines.length : 0,
      violations,
    };
    ensureDir(OUT_JSON);
    fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) exitCode = 1;
  } finally {
    clearTimeout(guardTimer);
    try { if (reader) reader.close(); } catch {}
    try { if (socket) socket.destroy(); } catch {}
    if (server) await new Promise((resolve) => server.close(resolve));
  }
  return exitCode;
}

main().then((exitCode) => {
  process.exit(exitCode || 0);
}).catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_socket_upgrade_transport_guard',
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
