#!/usr/bin/env node
/* eslint-disable no-console */

// Proves the canonical Agent Runtime socket semantics are live inside Gateway
// without requiring the legacy dashboard host. This guard intentionally talks
// to gateway/runtime/agent_runtime/agent_runtime_socket_handler.ts through the
// route assembly, not to Shell code and not to adapter-host HTTP shims.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_socket_live_handler_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-socket-live-handler-scratch');
const DASHBOARD_HOST_PATH = path.join(ROOT, 'adapters/runtime/infring_dashboard.ts');
const AGENT_ID = 'agent-runtime-socket-live-handler-agent';
const SESSION_ID = 'agent-runtime-socket-live-handler-session';
const ENGINE_ID = 'codex_cli';

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function safe(value) {
  return clean(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, '') || 'value';
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function artifactRelPath() {
  return `core/local/artifacts/agent-runtime-socket-live-handler-scratch/${safe(ENGINE_ID)}/socket-proof.txt`;
}

function createSocketProofAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'agent_runtime_socket_live_handler_guard',
      version_preview: `${engineId} socket proof adapter`,
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      session_id: message && message.session_id,
      receipt_ref: `receipt/socket-proof-session/${engineId}`,
    }),
    submit_turn: async ({ message, onActivity }) => {
      if (typeof onActivity === 'function') {
        onActivity({
          type: 'agent_activity_event',
          activity_kind: 'command',
          provider_event_type: 'socket-proof.command.started',
          status: 'started',
          display_text: 'Checking socket proof workspace.',
        });
        onActivity({
          type: 'agent_activity_event',
          activity_kind: 'file_change',
          provider_event_type: 'socket-proof.file.completed',
          status: 'completed',
          display_text: `Preparing artifact proposal: ${artifactRelPath()}`,
        });
      }
      return {
        type: 'infring_universal_tool_proposal',
        trace_id: message && message.trace_id,
        request_id: message && message.request_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: message && message.turn_id,
        tool_id: 'artifact.create_propose',
        reason: 'Create socket proof artifact through Gateway approval.',
        arguments: {
          path: artifactRelPath(),
          mime_type: 'text/plain',
          content: `socket live handler proof for ${engineId}\n`,
        },
      };
    },
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, engine_id: engineId, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, engine_id: engineId, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, engine_id: engineId, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, engine_id: engineId, receipt_refs: [] }),
  };
}

async function emitMessage(handler, message) {
  const events = [];
  const result = await handler.handleAgentRuntimeSocketMessage(message, (event) => events.push(event), { flags: {} });
  return { result, events };
}

function eventTypes(events) {
  return events.map((event) => clean(event && event.type, 120)).filter(Boolean);
}

function dashboardHostUpgradeWiring() {
  let source = '';
  try {
    source = fs.readFileSync(DASHBOARD_HOST_PATH, 'utf8');
  } catch (error) {
    return {
      ok: false,
      status: 'missing_host_source',
      error: clean(error && error.message ? error.message : error, 400),
    };
  }
  const assemblyExportsTransport = /createGatewayAgentRuntimeRouteAssembly\([\s\S]*?\)[\s\S]*?agentRuntimeSocketTransport/.test(source) ||
    /\{\s*[\s\S]*?agentRuntimeSocketTransport[\s\S]*?\}\s*=\s*createGatewayAgentRuntimeRouteAssembly\(/.test(source);
  const upgradePassesTransport = /handleDashboardUpgrade\(\{\s*[\s\S]*?agentRuntimeSocketTransport[\s\S]*?\}\)/.test(source);
  return {
    ok: assemblyExportsTransport && upgradePassesTransport,
    status: assemblyExportsTransport && upgradePassesTransport ? 'live' : 'pending',
    assembly_exports_transport: assemblyExportsTransport,
    upgrade_passes_transport: upgradePassesTransport,
    host_path: path.relative(ROOT, DASHBOARD_HOST_PATH),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories: {
      [ENGINE_ID]: () => createSocketProofAdapter(ENGINE_ID),
    },
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });
  const handler = assembly.agentRuntimeSocketHandler;
  const allEvents = [];
  const traceId = `validation:agent-runtime-socket-live:${Date.now()}`;

  for (const message of [
    { type: 'engine.list', trace_id: traceId, request_id: 'socket-engine-list' },
    { type: 'engine.health', trace_id: traceId, request_id: 'socket-engine-health', engine_id: ENGINE_ID },
    { type: 'session.start', trace_id: traceId, request_id: 'socket-session-start', engine_id: ENGINE_ID, session_id: SESSION_ID, scope: { workspace: ROOT } },
    { type: 'heartbeat', trace_id: traceId, request_id: 'socket-heartbeat' },
    {
      type: 'turn.submit',
      trace_id: traceId,
      request_id: 'socket-turn-submit',
      engine_id: ENGINE_ID,
      agent_id: AGENT_ID,
      session_id: SESSION_ID,
      turn_id: 'socket-turn-1',
      input: { text: 'Create the socket proof artifact.' },
      scope: { workspace: ROOT },
      capability_budget: { max_default_response_bytes: 65536 },
      working_directory: ROOT,
    },
  ]) {
    const { events } = await emitMessage(handler, message);
    allEvents.push(...events);
  }

  const permission = allEvents.find((event) => event && event.type === 'permission.requested');
  if (permission && permission.approval_id) {
    const { events } = await emitMessage(handler, {
      type: 'permission.decision',
      trace_id: traceId,
      request_id: 'socket-permission-decision',
      engine_id: ENGINE_ID,
      session_id: SESSION_ID,
      turn_id: 'socket-turn-1',
      approval_id: permission.approval_id,
      decision: 'allow_once',
    });
    allEvents.push(...events);
  }

  for (const message of [
    { type: 'turn.cancel', trace_id: traceId, request_id: 'socket-turn-cancel', engine_id: ENGINE_ID, session_id: SESSION_ID, turn_id: 'socket-turn-cancelled' },
    { type: 'unknown.event', trace_id: traceId, request_id: 'socket-unknown', engine_id: ENGINE_ID, session_id: SESSION_ID },
  ]) {
    const { events } = await emitMessage(handler, message);
    allEvents.push(...events);
  }

  const types = eventTypes(allEvents);
  const required = [
    'engine.list.result',
    'engine.health.result',
    'session.started',
    'tool.started',
    'tool.proposed',
    'permission.requested',
    'assistant.delta',
    'tool.completed',
    'artifact.created',
    'receipt.created',
    'heartbeat',
    'turn.complete',
    'error',
  ];
  const violations = [];
  const hostWiring = dashboardHostUpgradeWiring();
  if (!hostWiring.ok) violations.push({ kind: 'host_upgrade_transport_not_wired', host_wiring: hostWiring });
  for (const type of required) {
    if (!types.includes(type)) violations.push({ kind: 'missing_socket_event_type', type });
  }
  if (!permission || permission.status !== 'paused_pending_approval') {
    violations.push({ kind: 'permission_pause_not_projected', permission: permission || null });
  }
  const artifactPath = path.join(ROOT, artifactRelPath());
  const artifactExists = fs.existsSync(artifactPath);
  if (!artifactExists) violations.push({ kind: 'approved_socket_artifact_not_created', path: artifactRelPath() });
  for (const event of allEvents) {
    if (event && event.trace_id && event.trace_id !== traceId) {
      violations.push({ kind: 'trace_id_replaced', event_type: event.type, trace_id: event.trace_id });
    }
    const serialized = JSON.stringify(event || {});
    if (/(raw_tool_result|external_framework_transcript|adapter_process_dump|authorization_state|policy_decision)/.test(serialized)) {
      violations.push({ kind: 'forbidden_socket_default_payload_field', event_type: event && event.type });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_socket_live_handler_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_gateway_socket_handler',
    canonical_socket_route: '/ws/agent-runtime',
    host_upgrade_wiring: hostWiring.status,
    host_upgrade_wiring_probe: hostWiring,
    engine_id: ENGINE_ID,
    trace_id: traceId,
    event_count: allEvents.length,
    event_types: types,
    permission_pause_projected: !!permission,
    approved_artifact_created: artifactExists,
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_socket_live_handler_guard',
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
