#!/usr/bin/env node
/* eslint-disable no-console */

// Proves public Agent Runtime cancel events leave a compact, persisted,
// user-visible transcript outcome for the primary external engines.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_stop_cancel_semantics_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-stop-cancel-semantics-scratch');
const AGENT_ID = 'agent-runtime-stop-cancel-agent';
const ENGINES = ['codex_cli', 'claude_code'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'state'),
    adapterFactories: {},
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
    const sessionId = `agent-runtime-stop-cancel-${engineId}`;
    const turnId = `stop-cancel-turn-${index + 1}-${engineId}`;
    const traceId = `validation:agent-runtime-stop-cancel:${engineId}:${Date.now()}:${index + 1}`;
    const emitted = [];
    const result = await assembly.handleAgentRuntimeSocketMessage({
      type: 'turn.cancel',
      trace_id: traceId,
      request_id: `stop-cancel-request-${index + 1}`,
      engine_id: engineId,
      agent_id: AGENT_ID,
      session_id: sessionId,
      turn_id: turnId,
      reason: 'User pressed stop while the external runtime was working.',
    }, (event) => emitted.push(event));
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
    const rows = merged && merged.message_window && Array.isArray(merged.message_window.rows)
      ? merged.message_window.rows
      : [];
    const assistantRows = rows.filter((row) => clean(row && row.role, 40) === 'assistant');
    const cancelRows = assistantRows.filter((row) => {
      const joined = [
        row && row.status,
        row && row.text,
        row && row.content_preview,
        row && row.agent_runtime_worked_label,
      ].map((item) => clean(item, 1000).toLowerCase()).join(' ');
      return /cancel|stopped|stop/.test(joined);
    });
    const completeEvent = emitted.find((event) => event && event.type === 'turn.complete');
    results.push({
      engine_id: engineId,
      socket_result_ok: result && result.ok === true,
      emitted_turn_complete: !!completeEvent,
      emitted_status: clean(completeEvent && completeEvent.status, 120),
      emitted_terminal_outcome: clean(completeEvent && completeEvent.terminal_outcome, 120),
      transcript_overlay_present: !!(merged && merged.agent_runtime_transcript_overlay),
      transcript_row_count: rows.length,
      cancelled_assistant_row_count: cancelRows.length,
      cancelled_row_status: clean(cancelRows[0] && cancelRows[0].status, 120),
      cancelled_row_text: clean(cancelRows[0] && (cancelRows[0].text || cancelRows[0].content_preview), 1000),
      worked_label: clean(cancelRows[0] && cancelRows[0].agent_runtime_worked_label, 120),
      activity_trace_present: !!(cancelRows[0] && cancelRows[0].activity_trace),
      payload_bytes: Buffer.byteLength(JSON.stringify(merged || {}), 'utf8'),
      ok: !!(
        result && result.ok === true &&
        completeEvent && completeEvent.status === 'cancelled' &&
        merged && merged.agent_runtime_transcript_overlay &&
        cancelRows.length === 1 &&
        clean(cancelRows[0] && cancelRows[0].status, 120) === 'cancelled' &&
        /stopped|cancel/.test(clean(cancelRows[0] && cancelRows[0].text, 1000).toLowerCase()) &&
        Buffer.byteLength(JSON.stringify(merged || {}), 'utf8') <= 65536
      ),
    });
  }

  const violations = [];
  for (const row of results) {
    if (!row.ok) {
      violations.push({
        kind: 'cancel_turn_did_not_persist_visible_stopped_outcome',
        engine_id: row.engine_id,
        emitted_status: row.emitted_status,
        transcript_row_count: row.transcript_row_count,
        cancelled_assistant_row_count: row.cancelled_assistant_row_count,
        cancelled_row_status: row.cancelled_row_status,
        payload_bytes: row.payload_bytes,
      });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_stop_cancel_semantics_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_public_socket_cancel_transcript_persistence',
    engines_tested: ENGINES,
    results,
    budget: {
      default_gateway_budget_bytes: 65536,
      max_payload_bytes: Math.max(0, ...results.map((row) => row.payload_bytes)),
    },
    policy: {
      shell_replaceability_policy: 'Shell sends turn.cancel and renders Gateway events; Gateway persists the stopped outcome.',
      transcript_policy: 'Stopped turns must survive reload/history as compact assistant transcript rows.',
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
    type: 'agent_runtime_stop_cancel_semantics_guard',
    generated_at: new Date().toISOString(),
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['stop_cancel_semantics_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
