#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external runtimes preserve both agent decision/dialog text and
// tool activity through the public Gateway turn route and a fresh transcript
// projection. This is the user-facing legibility contract for using Codex and
// Claude interchangeably through InfRing without adding cognition to Shell.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(
  ROOT,
  'core/local/artifacts/agent_runtime_activity_decision_trace_persistence_guard_current.json',
);
const SCRATCH_DIR = path.join(
  ROOT,
  'core/local/artifacts/agent-runtime-activity-decision-trace-persistence-scratch',
);
const AGENT_ID = 'agent-runtime-activity-decision-trace-agent';
const ENGINES = ['codex_cli', 'claude_code'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function createActivityDecisionAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'activity_decision_trace_persistence_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => {
      const turnId = clean(message && message.turn_id, 200);
      const decisionText = `${engineId} decision dialog: inspect context, run one tool action, then answer.`;
      const commandText = engineId === 'claude_code'
        ? 'npm test -- --watch=false'
        : '/bin/zsh -lc "pwd"';
      const filePath = `/tmp/${engineId}-activity-decision-trace.txt`;
      const answer = `${engineId} completed with preserved decision dialog and tool activity.`;
      return {
        type: 'turn.complete',
        trace_id: message && message.trace_id,
        request_id: message && message.request_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: turnId,
        status: 'completed',
        output_text: answer,
        output_preview: answer,
        activity_events: [
          {
            type: 'agent_activity_event',
            activity_kind: 'decision_dialog',
            provider_event_type: `fixture.${engineId}.decision_dialog`,
            status: 'completed',
            display_text: decisionText,
            text: decisionText,
            engine_id: engineId,
            trace_id: message && message.trace_id,
            session_id: message && message.session_id,
            turn_id: turnId,
          },
          {
            type: 'agent_activity_event',
            activity_kind: 'command',
            provider_event_type: `fixture.${engineId}.command.started`,
            status: 'running',
            display_text: `Reading workspace with command: ${commandText}`,
            command: commandText,
            engine_id: engineId,
            trace_id: message && message.trace_id,
            session_id: message && message.session_id,
            turn_id: turnId,
          },
          {
            type: 'agent_activity_event',
            activity_kind: 'file_change',
            provider_event_type: `fixture.${engineId}.file_change.completed`,
            status: 'completed',
            display_text: `Wrote activity trace fixture: ${filePath}`,
            path: filePath,
            engine_id: engineId,
            trace_id: message && message.trace_id,
            session_id: message && message.session_id,
            turn_id: turnId,
          },
        ],
        structured_activity: true,
      };
    },
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
      receipt_refs: [`receipt/activity-decision-trace/${engineId}`],
    }),
  };
}

async function submitTurn(assembly, engineId, index) {
  const res = makeResponse();
  const sessionId = `agent-runtime-activity-decision-trace-${engineId}`;
  const traceId = `validation:agent-runtime-activity-decision-trace:${engineId}:${Date.now()}:${index}`;
  const turnId = `activity-decision-trace-${index}-${engineId}`;
  const prompt = `${engineId} prove decision dialog and tool activity survive projection.`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: sessionId,
        conversation_id: sessionId,
        engine_id: engineId,
        turn_id: turnId,
        message: prompt,
        input_text: prompt,
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
    session_id: sessionId,
    turn_id: turnId,
    prompt,
    handled,
    status_code: res.statusCode,
    payload: res.payload || {},
  };
}

function projectRows(assembly, sessionId) {
  const merged = assembly.agentRuntimeTranscriptStore.mergeAgentRuntimeTranscriptPayload({
    type: 'session_projection',
    session_id: sessionId,
    message_window: {
      rows: [],
      total_count: 0,
      before_cursor: null,
      after_cursor: null,
    },
  }, {
    agentId: AGENT_ID,
    sessionId,
    limit: 20,
  });
  return {
    merged,
    rows: merged && merged.message_window && Array.isArray(merged.message_window.rows)
      ? merged.message_window.rows
      : [],
  };
}

function traceRowsFrom(value) {
  if (!value || typeof value !== 'object') return [];
  const trace = value.activity_trace && typeof value.activity_trace === 'object'
    ? value.activity_trace
    : value.agent_runtime_activity_trace && typeof value.agent_runtime_activity_trace === 'object'
      ? value.agent_runtime_activity_trace
      : {};
  if (Array.isArray(trace.rows)) return trace.rows;
  if (Array.isArray(value.agent_runtime_trace_rows)) return value.agent_runtime_trace_rows;
  if (Array.isArray(value.agent_runtime_activity_trace_rows)) return value.agent_runtime_activity_trace_rows;
  const tools = Array.isArray(value.tools) ? value.tools : [];
  for (const tool of tools) {
    if (tool && Array.isArray(tool.agent_runtime_trace_rows)) return tool.agent_runtime_trace_rows;
  }
  return [];
}

function traceLabelFrom(value) {
  if (!value || typeof value !== 'object') return '';
  const trace = value.activity_trace && typeof value.activity_trace === 'object'
    ? value.activity_trace
    : value.agent_runtime_activity_trace && typeof value.agent_runtime_activity_trace === 'object'
      ? value.agent_runtime_activity_trace
      : {};
  return clean(trace.collapse_label || value.agent_runtime_worked_label || '', 120);
}

function rowText(row) {
  return clean(row && (row.title || row.display_text || row.text || row.summary || row.result), 4000);
}

function rowKind(row) {
  return clean(row && (row.activity_kind || row.kind || row.type), 160);
}

function decisionTextsFrom(value) {
  const out = [];
  if (!value || typeof value !== 'object') return out;
  if (value.agent_runtime_decision_dialog_text) out.push(clean(value.agent_runtime_decision_dialog_text, 4000));
  const tools = Array.isArray(value.tools) ? value.tools : [];
  for (const tool of tools) {
    if (tool && tool.agent_runtime_decision_dialog_text) {
      out.push(clean(tool.agent_runtime_decision_dialog_text, 4000));
    }
  }
  for (const row of traceRowsFrom(value)) {
    if (rowKind(row) === 'decision_dialog') out.push(rowText(row));
  }
  return out.filter(Boolean);
}

function summarizeProjection(engineId, turn, projection) {
  const expectedDecision = `${engineId} decision dialog: inspect context, run one tool action, then answer.`;
  const rows = projection.rows;
  const assistantRows = rows.filter((row) => clean(row && row.role, 40) === 'assistant');
  const assistant = assistantRows.find((row) => clean(row && (row.text || row.content || row.content_preview), 4000).includes('preserved decision dialog and tool activity')) || null;
  const routeTraceRows = traceRowsFrom(turn.payload);
  const projectedTraceRows = traceRowsFrom(assistant);
  const routeDecisionTexts = decisionTextsFrom(turn.payload);
  const projectedDecisionTexts = decisionTextsFrom(assistant);
  const routeVisibleRows = routeTraceRows.filter((row) => row && row.display_in_thinking_bubble !== false);
  const projectedVisibleRows = projectedTraceRows.filter((row) => row && row.display_in_thinking_bubble !== false);
  const projectedTitles = projectedTraceRows.map(rowText).filter(Boolean);
  const routeTitles = routeTraceRows.map(rowText).filter(Boolean);
  const projectedKinds = projectedTraceRows.map(rowKind).filter(Boolean);
  const routeKinds = routeTraceRows.map(rowKind).filter(Boolean);
  const projectedPayloadBytes = Buffer.byteLength(JSON.stringify(projection.merged || {}), 'utf8');
  const routePayloadBytes = Buffer.byteLength(JSON.stringify(turn.payload || {}), 'utf8');
  const projectedHasToolRow = projectedTraceRows.some((row) => {
    const kind = rowKind(row);
    const text = rowText(row).toLowerCase();
    return kind !== 'decision_dialog' && /(command|file_change|tool|read|wrote|writ|command)/.test(`${kind} ${text}`);
  });
  const routeHasToolRow = routeTraceRows.some((row) => {
    const kind = rowKind(row);
    const text = rowText(row).toLowerCase();
    return kind !== 'decision_dialog' && /(command|file_change|tool|read|wrote|writ|command)/.test(`${kind} ${text}`);
  });
  return {
    engine_id: engineId,
    handled: turn.handled,
    status_code: turn.status_code,
    turn_status: clean(turn.payload && turn.payload.status, 120),
    assistant_row_present: !!assistant,
    route_trace_label: traceLabelFrom(turn.payload),
    projected_trace_label: traceLabelFrom(assistant),
    route_trace_row_count: routeTraceRows.length,
    projected_trace_row_count: projectedTraceRows.length,
    route_visible_row_count: routeVisibleRows.length,
    projected_visible_row_count: projectedVisibleRows.length,
    route_kinds: routeKinds,
    projected_kinds: projectedKinds,
    route_titles: routeTitles,
    projected_titles: projectedTitles,
    route_decision_texts: routeDecisionTexts,
    projected_decision_texts: projectedDecisionTexts,
    route_has_decision_dialog: routeDecisionTexts.some((text) => text.includes(expectedDecision)),
    projected_has_decision_dialog: projectedDecisionTexts.some((text) => text.includes(expectedDecision)),
    route_has_tool_row: routeHasToolRow,
    projected_has_tool_row: projectedHasToolRow,
    projected_payload_bytes: projectedPayloadBytes,
    route_payload_bytes: routePayloadBytes,
    ok: !!(
      turn.handled &&
      turn.status_code === 200 &&
      clean(turn.payload && turn.payload.status, 120) === 'completed' &&
      assistant &&
      /^Worked for \d/.test(traceLabelFrom(turn.payload)) &&
      /^Worked for \d/.test(traceLabelFrom(assistant)) &&
      routeDecisionTexts.some((text) => text.includes(expectedDecision)) &&
      projectedDecisionTexts.some((text) => text.includes(expectedDecision)) &&
      routeHasToolRow &&
      projectedHasToolRow &&
      routeVisibleRows.length >= 2 &&
      projectedVisibleRows.length >= 2 &&
      projectedPayloadBytes <= 65536
    ),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of ENGINES) adapterFactories[engineId] = () => createActivityDecisionAdapter(engineId);
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories,
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });

  const results = [];
  for (let index = 0; index < ENGINES.length; index += 1) {
    const engineId = ENGINES[index];
    const turn = await submitTurn(assembly, engineId, index + 1);
    const projection = projectRows(assembly, turn.session_id);
    results.push(summarizeProjection(engineId, turn, projection));
  }

  const violations = [];
  for (const row of results) {
    if (row.ok) continue;
    violations.push({
      kind: 'activity_decision_trace_persistence_failed',
      engine_id: row.engine_id,
      handled: row.handled,
      status_code: row.status_code,
      turn_status: row.turn_status,
      assistant_row_present: row.assistant_row_present,
      route_has_decision_dialog: row.route_has_decision_dialog,
      projected_has_decision_dialog: row.projected_has_decision_dialog,
      route_has_tool_row: row.route_has_tool_row,
      projected_has_tool_row: row.projected_has_tool_row,
      projected_payload_bytes: row.projected_payload_bytes,
      route_titles: row.route_titles,
      projected_titles: row.projected_titles,
    });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_activity_decision_trace_persistence_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_public_route_reload_projection',
    engines_tested: ENGINES,
    results,
    budget: {
      default_gateway_budget_bytes: 65536,
      max_projected_payload_bytes: Math.max(0, ...results.map((row) => row.projected_payload_bytes)),
      max_route_payload_bytes: Math.max(0, ...results.map((row) => row.route_payload_bytes)),
    },
    policy: {
      shell_replaceability_policy: 'The guard reads Gateway turn/projection payloads only; Shell rendering remains projection-only.',
      interchangeability_policy: 'Codex and Claude must preserve both decision/dialog text and tool activity in the same compact transcript trace shape.',
    },
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_activity_decision_trace_persistence_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['activity_decision_trace_persistence_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
