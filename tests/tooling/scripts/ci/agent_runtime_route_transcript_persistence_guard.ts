#!/usr/bin/env node
/* eslint-disable no-console */

// Proves public Agent Runtime turn routes persist native and external engine
// turns through the same Gateway-owned transcript/context path. Provider CLIs
// are intentionally replaced with deterministic adapters so this remains a
// route contract guard rather than a flaky integration smoke.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_route_transcript_persistence_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-route-transcript-persistence-scratch');
const AGENT_ID = 'agent-runtime-route-transcript-persistence-agent';
const SESSION_ID = 'agent-runtime-route-transcript-persistence-session';
const CONTINUITY_KEY = 'route-transcript-key: violet-raven-318';
const ENGINES = ['infring_native', 'codex_cli', 'claude_code'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function rowText(rows) {
  return (Array.isArray(rows) ? rows : [])
    .map((row) => clean(row && (row.text || row.content || row.content_preview || row.summary), 1200))
    .filter(Boolean)
    .join('\n');
}

function rowsForEngine(rows, engineId) {
  return (Array.isArray(rows) ? rows : [])
    .filter((row) => clean(row && row.agent_runtime_engine_id, 120) === engineId);
}

function contextStatePath() {
  return path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}.json`);
}

function createDeterministicAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'route_transcript_persistence_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => ({
      type: 'turn.complete',
      trace_id: message && message.trace_id,
      request_id: message && message.request_id,
      engine_id: engineId,
      session_id: message && message.session_id,
      turn_id: message && message.turn_id,
      status: 'completed',
      output_text: `${engineId} persisted the shared continuity key ${CONTINUITY_KEY}.`,
      output_preview: `${engineId} persisted ${CONTINUITY_KEY}.`,
      result_ref: `artifact/route-transcript/${engineId}`,
      receipt_ref: `receipt/route-transcript/${engineId}`,
      activity_events: [
        {
          type: 'agent_activity_event',
          activity_kind: 'decision_dialog',
          provider_event_type: 'deterministic.decision_dialog',
          status: 'completed',
          display_text: `${engineId} decided to preserve the route transcript continuity key before writing the final response.`,
          engine_id: engineId,
          trace_id: message && message.trace_id,
          session_id: message && message.session_id,
          turn_id: message && message.turn_id,
        },
        {
          type: 'agent_activity_event',
          activity_kind: 'activity',
          provider_event_type: 'deterministic.turn.persisted',
          status: 'completed',
          display_text: `${engineId} completed deterministic route persistence turn.`,
          engine_id: engineId,
          trace_id: message && message.trace_id,
          session_id: message && message.session_id,
          turn_id: message && message.turn_id,
        },
      ],
      structured_activity: true,
    }),
    stream_events: async ({ message }) => ({
      type: 'heartbeat',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'ok',
    }),
    cancel_turn: async ({ message }) => ({
      type: 'turn.cancelled',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'cancelled',
    }),
    collect_artifacts: async ({ message }) => ({
      type: 'artifact.list',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      artifacts: [],
    }),
    emit_receipts: async ({ message }) => ({
      type: 'receipt.created',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      receipt_refs: [`receipt/route-transcript/${engineId}`],
    }),
  };
}

async function submitTurn(assembly, engineId, index) {
  const res = makeResponse();
  const traceId = `validation:agent-runtime-route-transcript:${engineId}:${Date.now()}:${index}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: SESSION_ID,
        conversation_id: SESSION_ID,
        engine_id: engineId,
        turn_id: `route-transcript-turn-${index}-${engineId}`,
        message: `Ask ${engineId} to preserve ${CONTINUITY_KEY}.`,
        input_text: `Ask ${engineId} to preserve ${CONTINUITY_KEY}.`,
        working_directory: ROOT,
        test_probe: true,
      },
    },
    res,
    pathname: '/api/agent-runtime/turn',
    traceId,
    flags: {},
  });
  return {
    engine_id: engineId,
    handled,
    status_code: res.statusCode,
    status: clean(res.payload && res.payload.status, 120),
    ok: !!(handled && res.statusCode === 200 && res.payload && res.payload.status === 'completed'),
    payload: res.payload,
  };
}

async function previewContext(assembly) {
  const res = makeResponse();
  const traceId = `validation:agent-runtime-route-transcript-preview:${Date.now()}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: SESSION_ID,
        engine_id: 'codex_cli',
      },
    },
    res,
    pathname: '/api/agent-runtime/context-pack/preview',
    traceId,
    flags: {},
  });
  return {
    handled,
    status_code: res.statusCode,
    payload: res.payload,
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(contextStatePath(), { force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of ENGINES) adapterFactories[engineId] = () => createDeterministicAdapter(engineId);
  const sent = [];
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories,
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
      sent.push({ statusCode, payload });
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });

  const turnResults = [];
  for (let index = 0; index < ENGINES.length; index += 1) {
    turnResults.push(await submitTurn(assembly, ENGINES[index], index + 1));
  }

  const merged = assembly.agentRuntimeTranscriptStore.mergeAgentRuntimeTranscriptPayload({
    type: 'session_projection',
    session_id: SESSION_ID,
    message_window: {
      rows: [],
      total_count: 0,
      before_cursor: null,
      after_cursor: null,
    },
  }, {
    agentId: AGENT_ID,
    sessionId: SESSION_ID,
    limit: 80,
  });
  const rows = merged && merged.message_window && Array.isArray(merged.message_window.rows)
    ? merged.message_window.rows
    : [];
  const preview = await previewContext(assembly);
  const previewFragments = preview.payload && Array.isArray(preview.payload.fragments) ? preview.payload.fragments : [];
  const previewText = rowText(previewFragments);
  const mergedText = rowText(rows);
  const assistantRows = rows.filter((row) => clean(row && row.role, 40) === 'assistant');
  const violations = [];

  if (!turnResults.every((row) => row.ok)) {
    violations.push({
      kind: 'public_turn_route_failed',
      failed_engines: turnResults.filter((row) => !row.ok).map((row) => ({
        engine_id: row.engine_id,
        handled: row.handled,
        status_code: row.status_code,
        status: row.status,
      })),
    });
  }
  if (!merged || !merged.agent_runtime_transcript_overlay) violations.push({ kind: 'transcript_overlay_missing_after_public_turns' });
  if (rows.length !== ENGINES.length * 2) {
    violations.push({ kind: 'unexpected_public_route_transcript_row_count', expected: ENGINES.length * 2, actual: rows.length });
  }
  if (Number(merged && merged.agent_runtime_transcript_overlay && merged.agent_runtime_transcript_overlay.row_count) !== rows.length) {
    violations.push({
      kind: 'raw_transcript_overlay_contains_duplicate_rows',
      overlay_row_count: Number(merged && merged.agent_runtime_transcript_overlay && merged.agent_runtime_transcript_overlay.row_count) || 0,
      projected_row_count: rows.length,
    });
  }
  for (const engineId of ENGINES) {
    if (rowsForEngine(rows, engineId).length !== 2) {
      violations.push({ kind: 'engine_public_route_rows_missing_or_extra', engine_id: engineId, row_count: rowsForEngine(rows, engineId).length });
    }
  }
  if (!mergedText.includes(CONTINUITY_KEY)) violations.push({ kind: 'continuity_key_missing_from_transcript_overlay' });
  if (!rows.every((row) => row && row.projection_owner === 'gateway.runtime.agent_runtime_transcript')) {
    violations.push({ kind: 'transcript_projection_owner_not_gateway' });
  }
  if (!rows.every((row) => clean(row && row.trace_id, 200))) violations.push({ kind: 'transcript_trace_id_missing' });
  if (!assistantRows.every((row) => row && row.activity_trace && row.activity_trace.collapse_label && /^Worked for /.test(String(row.activity_trace.collapse_label)))) {
    violations.push({ kind: 'assistant_worked_for_trace_not_persisted' });
  }
  if (!assistantRows.every((row) => row && Array.isArray(row.tools) && row.tools.some((tool) => tool && tool.agent_runtime_decision_dialog === true))) {
    violations.push({ kind: 'assistant_decision_tool_not_persisted' });
  }
  if (!assistantRows.every((row) => row && Array.isArray(row.agent_activity_events) && row.agent_activity_events.some((event) => event && event.activity_kind === 'decision_dialog'))) {
    violations.push({ kind: 'assistant_decision_dialog_event_not_persisted' });
  }
  if (Buffer.byteLength(JSON.stringify(merged), 'utf8') > 65536) {
    violations.push({
      kind: 'public_route_transcript_projection_exceeds_budget',
      byte_length: Buffer.byteLength(JSON.stringify(merged), 'utf8'),
    });
  }
  if (!preview.handled || preview.status_code !== 200 || !(preview.payload && preview.payload.ok)) {
    violations.push({
      kind: 'context_preview_route_failed_after_public_turns',
      handled: preview.handled,
      status_code: preview.status_code,
    });
  }
  if (!previewText.includes(CONTINUITY_KEY)) violations.push({ kind: 'continuity_key_missing_from_context_preview' });
  if (Number(preview.payload && preview.payload.row_count) < ENGINES.length * 2) {
    violations.push({
      kind: 'context_preview_row_count_too_low',
      expected_min: ENGINES.length * 2,
      actual: Number(preview.payload && preview.payload.row_count) || 0,
    });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_route_transcript_persistence_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_public_gateway_route',
    engine_scope_source: 'active_promotion_engines',
    engines_tested: ENGINES,
    turn_results: turnResults.map((row) => ({
      engine_id: row.engine_id,
      handled: row.handled,
      status_code: row.status_code,
      status: row.status,
      ok: row.ok,
    })),
    transcript_overlay: merged && merged.agent_runtime_transcript_overlay ? merged.agent_runtime_transcript_overlay : null,
    transcript_row_count: rows.length,
    context_preview: {
      handled: preview.handled,
      status_code: preview.status_code,
      ok: !!(preview.payload && preview.payload.ok),
      row_count: Number(preview.payload && preview.payload.row_count) || 0,
      fragment_count: previewFragments.length,
      source_authority: clean(preview.payload && preview.payload.source_authority, 200),
    },
    budget: {
      transcript_projection_bytes: Buffer.byteLength(JSON.stringify(merged), 'utf8'),
      default_gateway_budget_bytes: 65536,
    },
    sent_count: sent.length,
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
    type: 'agent_runtime_route_transcript_persistence_guard',
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(contextStatePath(), { force: true }); } catch {}
  process.exit(1);
});
