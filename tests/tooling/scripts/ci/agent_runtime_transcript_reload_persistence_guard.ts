#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external runtimes persist user-visible transcript outcomes
// through the Gateway path: completed turns, failed turns, and stopped turns
// must survive a fresh transcript projection/reload.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_transcript_reload_persistence_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-transcript-reload-persistence-scratch');
const AGENT_ID = 'agent-runtime-transcript-reload-agent';
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

function createPersistenceAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'transcript_reload_persistence_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => {
      const turnId = clean(message && message.turn_id, 200);
      const isFailure = /failure/.test(turnId);
      const status = isFailure ? 'failed' : 'completed';
      const output = isFailure
        ? `${engineId} visible failure persisted after reload.`
        : `${engineId} visible assistant answer persisted after reload.`;
      return {
        type: 'turn.complete',
        trace_id: message && message.trace_id,
        request_id: message && message.request_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: turnId,
        status,
        error_code: isFailure ? `${engineId}_synthetic_failure` : '',
        reason: isFailure ? `${engineId} synthetic failure reason.` : '',
        retryable: isFailure,
        output_text: output,
        output_preview: output,
        activity_events: [
          {
            type: 'agent_activity_event',
            activity_kind: 'decision_dialog',
            provider_event_type: isFailure ? 'deterministic.failure_dialog' : 'deterministic.answer_dialog',
            status,
            display_text: isFailure
              ? `${engineId} decided to stop with a visible classified failure.`
              : `${engineId} decided to answer with persisted context.`,
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
      receipt_refs: [`receipt/transcript-reload/${engineId}`],
    }),
  };
}

async function submitTurn(assembly, engineId, sessionId, kind, index) {
  const res = makeResponse();
  const traceId = `validation:agent-runtime-transcript-reload:${engineId}:${kind}:${Date.now()}:${index}`;
  const turnId = `transcript-reload-${kind}-${index}-${engineId}`;
  const prompt = `${engineId} ${kind} prompt should survive reload.`;
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
    kind,
    turn_id: turnId,
    prompt,
    handled,
    status_code: res.statusCode,
    status: clean(res.payload && res.payload.status, 120),
    output_preview: clean(res.payload && (res.payload.output_preview || res.payload.output_text || res.payload.text), 1000),
    ok: !!(handled && res.statusCode === 200),
  };
}

async function cancelTurn(assembly, engineId, sessionId, index) {
  const traceId = `validation:agent-runtime-transcript-reload:${engineId}:cancel:${Date.now()}:${index}`;
  const turnId = `transcript-reload-cancel-${index}-${engineId}`;
  const emitted = [];
  const result = await assembly.handleAgentRuntimeSocketMessage({
    type: 'turn.cancel',
    trace_id: traceId,
    request_id: `transcript-reload-cancel-${index}`,
    engine_id: engineId,
    agent_id: AGENT_ID,
    session_id: sessionId,
    turn_id: turnId,
    reason: 'User stopped the turn during transcript reload proof.',
  }, (event) => emitted.push(event));
  return {
    kind: 'cancel',
    turn_id: turnId,
    socket_result_ok: !!(result && result.ok),
    emitted_status: clean((emitted.find((event) => event && event.type === 'turn.complete') || {}).status, 120),
    ok: !!(result && result.ok),
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
    limit: 40,
  });
  return {
    merged,
    rows: merged && merged.message_window && Array.isArray(merged.message_window.rows)
      ? merged.message_window.rows
      : [],
  };
}

function rowIncludes(row, text) {
  return clean(row && (row.text || row.content || row.content_preview), 4000).includes(text);
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of ENGINES) adapterFactories[engineId] = () => createPersistenceAdapter(engineId);
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
    const sessionId = `agent-runtime-transcript-reload-${engineId}`;
    const completed = await submitTurn(assembly, engineId, sessionId, 'completed', index + 1);
    const failed = await submitTurn(assembly, engineId, sessionId, 'failure', index + 1);
    const cancelled = await cancelTurn(assembly, engineId, sessionId, index + 1);
    const projection = projectRows(assembly, sessionId);
    const rows = projection.rows;
    const assistantRows = rows.filter((row) => clean(row && row.role, 40) === 'assistant');
    const userRows = rows.filter((row) => clean(row && row.role, 40) === 'user');
    const completedAssistant = assistantRows.find((row) => rowIncludes(row, `${engineId} visible assistant answer persisted after reload.`));
    const failedAssistant = assistantRows.find((row) => rowIncludes(row, `${engineId} visible failure persisted after reload.`));
    const stoppedAssistant = assistantRows.find((row) => /stopped|cancel/.test(clean(row && row.text, 1000).toLowerCase()));
    const payloadBytes = Buffer.byteLength(JSON.stringify(projection.merged || {}), 'utf8');
    results.push({
      engine_id: engineId,
      completed_turn: completed,
      failed_turn: failed,
      cancelled_turn: cancelled,
      transcript_overlay_present: !!(projection.merged && projection.merged.agent_runtime_transcript_overlay),
      transcript_row_count: rows.length,
      user_row_count: userRows.length,
      assistant_row_count: assistantRows.length,
      completed_assistant_status: clean(completedAssistant && completedAssistant.status, 120),
      failed_assistant_status: clean(failedAssistant && failedAssistant.status, 120),
      stopped_assistant_status: clean(stoppedAssistant && stoppedAssistant.status, 120),
      completed_worked_label: clean(completedAssistant && completedAssistant.agent_runtime_worked_label, 120),
      failed_worked_label: clean(failedAssistant && failedAssistant.agent_runtime_worked_label, 120),
      stopped_worked_label: clean(stoppedAssistant && stoppedAssistant.agent_runtime_worked_label, 120),
      completed_activity_trace: !!(completedAssistant && completedAssistant.activity_trace),
      failed_activity_trace: !!(failedAssistant && failedAssistant.activity_trace),
      stopped_activity_trace: !!(stoppedAssistant && stoppedAssistant.activity_trace),
      prompt_rows_persisted: userRows.some((row) => rowIncludes(row, completed.prompt)) && userRows.some((row) => rowIncludes(row, failed.prompt)),
      payload_bytes: payloadBytes,
      ok: !!(
        completed.ok &&
        failed.ok &&
        cancelled.ok &&
        projection.merged && projection.merged.agent_runtime_transcript_overlay &&
        userRows.some((row) => rowIncludes(row, completed.prompt)) &&
        userRows.some((row) => rowIncludes(row, failed.prompt)) &&
        completedAssistant &&
        failedAssistant &&
        stoppedAssistant &&
        clean(completedAssistant.status, 120) === 'completed' &&
        clean(failedAssistant.status, 120) === 'failed_with_reason' &&
        clean(stoppedAssistant.status, 120) === 'cancelled' &&
        completedAssistant.activity_trace &&
        failedAssistant.activity_trace &&
        stoppedAssistant.activity_trace &&
        payloadBytes <= 65536
      ),
    });
  }

  const violations = [];
  for (const row of results) {
    if (!row.ok) {
      violations.push({
        kind: 'external_runtime_transcript_reload_persistence_failed',
        engine_id: row.engine_id,
        prompt_rows_persisted: row.prompt_rows_persisted,
        completed_assistant_status: row.completed_assistant_status,
        failed_assistant_status: row.failed_assistant_status,
        stopped_assistant_status: row.stopped_assistant_status,
        completed_activity_trace: row.completed_activity_trace,
        failed_activity_trace: row.failed_activity_trace,
        stopped_activity_trace: row.stopped_activity_trace,
        payload_bytes: row.payload_bytes,
      });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_transcript_reload_persistence_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_reload_projection',
    engines_tested: ENGINES,
    results,
    budget: {
      default_gateway_budget_bytes: 65536,
      max_payload_bytes: Math.max(0, ...results.map((row) => row.payload_bytes)),
    },
    policy: {
      shell_replaceability_policy: 'Reload proof uses Gateway transcript projection, not dashboard visual state.',
      interchangeability_policy: 'Codex and Claude must persist the same completed, failed, and stopped outcome shapes.',
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
    type: 'agent_runtime_transcript_reload_persistence_guard',
    generated_at: new Date().toISOString(),
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['transcript_reload_persistence_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
