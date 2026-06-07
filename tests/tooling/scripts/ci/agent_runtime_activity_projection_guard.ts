#!/usr/bin/env node
/* eslint-disable no-console */

// Verifies Gateway compacts provider-shaped/raw JSON activity events into
// bounded user-facing trace rows before any Shell sees them.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_activity_projection_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-activity-projection-scratch');
const SESSION_ID = 'agent-runtime-activity-projection-session';
const AGENT_ID = 'agent-runtime-activity-projection-agent';

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function contextStatePath() {
  return path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}.json`);
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function createRawActivityAdapter() {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      status: 'available',
      discovery_source: 'activity_projection_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      status: 'started',
    }),
    submit_turn: async ({ message }) => ({
      type: 'turn.complete',
      trace_id: message && message.trace_id,
      engine_id: message && message.engine_id,
      session_id: message && message.session_id,
      turn_id: message && message.turn_id,
      status: 'completed',
      output_text: 'Activity projection guard completed.',
      output_preview: 'Activity projection guard completed.',
      activity_events: [
        {
          type: 'agent_activity_event',
          activity_kind: 'started',
          status: 'running',
          display_text: JSON.stringify({ type: 'thread.started', thread_id: 'thread-activity-projection' }),
        },
        {
          type: 'agent_activity_event',
          activity_kind: 'command',
          status: 'running',
          display_text: JSON.stringify({
            type: 'item.started',
            item: {
              type: 'command',
              command: '/bin/zsh -lc "pwd"',
            },
          }),
        },
        {
          type: 'agent_activity_event',
          activity_kind: 'file_change',
          status: 'completed',
          display_text: JSON.stringify({
            type: 'item.completed',
            item: {
              type: 'file_change',
              path: '/tmp/activity-projection.txt',
            },
          }),
        },
        {
          type: 'agent_activity_event',
          activity_kind: 'completed',
          status: 'completed',
          display_text: JSON.stringify({ type: 'turn.completed', usage: { input_tokens: 7, output_tokens: 3 } }),
        },
      ],
      activity_event_count: 4,
      structured_activity: true,
    }),
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, receipt_refs: [] }),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(contextStatePath(), { force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories: {
      codex_cli: () => createRawActivityAdapter(),
    },
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });

  const res = makeResponse();
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: SESSION_ID,
        engine_id: 'codex_cli',
        message: 'run activity projection guard',
        input_text: 'run activity projection guard',
        working_directory: ROOT,
      },
    },
    res,
    pathname: '/api/agent-runtime/turn',
    traceId: `validation:agent-runtime-activity-projection:${Date.now()}`,
    flags: {},
  });

  const payload = res.payload || {};
  const trace = payload.activity_trace && typeof payload.activity_trace === 'object' ? payload.activity_trace : {};
  const rows = Array.isArray(trace.rows) ? trace.rows : [];
  const titles = rows.map((row) => clean(row && row.title, 1000)).filter(Boolean);
  const violations = [];

  if (!handled || res.statusCode !== 200 || payload.status !== 'completed') {
    violations.push({ kind: 'activity_projection_turn_route_failed', handled, status_code: res.statusCode, status: clean(payload.status, 120) });
  }
  if (!trace || trace.type !== 'agent_runtime_activity_trace_projection') violations.push({ kind: 'activity_trace_projection_missing' });
  if (trace.collapsed_by_default !== true) violations.push({ kind: 'activity_trace_not_collapsed_by_default' });
  if (!/^Worked for \d/.test(clean(trace.collapse_label, 120))) violations.push({ kind: 'activity_trace_missing_worked_label', label: clean(trace.collapse_label, 120) });
  if (!titles.length) violations.push({ kind: 'activity_trace_rows_missing' });
  if (titles.some((title) => /^[{[]/.test(title))) violations.push({ kind: 'raw_json_activity_title_leaked', titles });
  if (titles.some((title) => /\"type\"|thread_id|input_tokens|output_tokens/.test(title))) violations.push({ kind: 'raw_provider_payload_detail_leaked', titles });
  for (const expected of [
    'Runtime thread started.',
    'Working on command: /bin/zsh -lc "pwd"',
    'Completed file change: /tmp/activity-projection.txt',
    'Runtime completed the turn.',
  ]) {
    if (!titles.includes(expected)) violations.push({ kind: 'semantic_activity_title_missing', expected, titles });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_activity_projection_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_public_gateway_route',
    handled,
    status_code: res.statusCode,
    turn_status: clean(payload.status, 120),
    trace: {
      collapsed_by_default: trace.collapsed_by_default === true,
      collapse_label: clean(trace.collapse_label, 120),
      row_count: rows.length,
      titles,
    },
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(contextStatePath(), { force: true }); } catch {}
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_activity_projection_guard',
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(contextStatePath(), { force: true }); } catch {}
  process.exit(1);
});
