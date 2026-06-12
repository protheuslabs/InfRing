#!/usr/bin/env node
/* eslint-disable no-console */

// Proves large pasted text is normalized by Gateway into bounded attachment
// refs before primary external runtimes receive a turn. This is public-route
// evidence for framework interchangeability: Codex and Claude should read the
// same InfRing-owned context ref shape without Shell prompt stuffing.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_attachment_context_bridge_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-attachment-context-bridge-scratch');
const ENGINES = ['codex_cli', 'claude_code'];
const SECRET = 'ATTACHMENT_BRIDGE_SECRET=violet-context-ref-729';

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function largePastedText(engineId) {
  const lines = [
    `Synthetic oversized pasted context for ${engineId}.`,
    SECRET,
  ];
  for (let index = 0; index < 180; index += 1) {
    lines.push(`bridge filler ${engineId} ${index}: this line exists only to exceed the large-text attachment threshold.`);
  }
  return lines.join('\n');
}

function attachmentRowsFromMessage(message) {
  const pack = message && message.context_pack && typeof message.context_pack === 'object' ? message.context_pack : {};
  const refs = pack.runtime_attachment_refs && typeof pack.runtime_attachment_refs === 'object' ? pack.runtime_attachment_refs : {};
  return Array.isArray(refs.attachments) ? refs.attachments : [];
}

function createDeterministicAttachmentAdapter(engineId, observations) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'attachment_context_bridge_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => {
      const attachments = attachmentRowsFromMessage(message);
      const first = attachments[0] || {};
      const readPath = clean(first.local_read_path || first.read_path, 1200);
      let fileText = '';
      let readError = '';
      try {
        fileText = readPath ? fs.readFileSync(readPath, 'utf8') : '';
      } catch (error) {
        readError = clean(error && error.message ? error.message : error, 500);
      }
      const normalizedInputText = clean(message && message.input && message.input.text, 2000);
      const observed = {
        engine_id: engineId,
        attachment_count: attachments.length,
        first_attachment: {
          filename: clean(first.filename, 240),
          source_kind: clean(first.source_kind, 120),
          source_authority: clean(first.source_authority, 160),
          size_bytes: Number(first.size_bytes) || 0,
          stored_bytes: Number(first.stored_bytes) || 0,
          local_read_path_present: !!readPath,
          read_error: readError,
        },
        normalized_input_text: normalizedInputText,
        normalized_input_contains_secret: normalizedInputText.includes(SECRET),
        file_contains_secret: fileText.includes(SECRET),
        file_length: fileText.length,
        context_pack_has_runtime_attachment_refs: attachments.length > 0,
      };
      observations.push(observed);
      const status = observed.file_contains_secret && !observed.normalized_input_contains_secret ? 'completed' : 'failed';
      const output = status === 'completed'
        ? `${engineId} read ${SECRET} from Gateway attachment ref.`
        : `${engineId} failed attachment bridge: count=${attachments.length} read_error=${readError || 'none'} normalized_leak=${observed.normalized_input_contains_secret}`;
      return {
        type: 'turn.complete',
        trace_id: message && message.trace_id,
        request_id: message && message.request_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: message && message.turn_id,
        status,
        output_text: output,
        output_preview: output,
        activity_events: [
          {
            type: 'agent_activity_event',
            activity_kind: 'decision_dialog',
            provider_event_type: 'deterministic.attachment_ref_read',
            status,
            display_text: `${engineId} inspected Gateway runtime_attachment_refs and read the pasted context from the provided ref.`,
            engine_id: engineId,
            trace_id: message && message.trace_id,
            session_id: message && message.session_id,
            turn_id: message && message.turn_id,
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
      receipt_refs: [`receipt/attachment-context-bridge/${engineId}`],
    }),
  };
}

async function submitLargePasteTurn(assembly, engineId, index) {
  const res = makeResponse();
  const traceId = `validation:agent-runtime-attachment-context:${engineId}:${Date.now()}:${index}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: 'agent-runtime-attachment-context-bridge-agent',
        session_id: `agent-runtime-attachment-context-bridge-${engineId}`,
        conversation_id: `agent-runtime-attachment-context-bridge-${engineId}`,
        engine_id: engineId,
        turn_id: `attachment-context-bridge-turn-${index}-${engineId}`,
        message: largePastedText(engineId),
        input_text: largePastedText(engineId),
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
    output_preview: clean(res.payload && (res.payload.output_preview || res.payload.output_text || res.payload.text), 1200),
    payload_bytes: Buffer.byteLength(JSON.stringify(res.payload || {}), 'utf8'),
    payload_contains_secret: JSON.stringify(res.payload || {}).includes(SECRET),
    ok: !!(handled && res.statusCode === 200 && res.payload && res.payload.status === 'completed'),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const observations = [];
  const adapterFactories = {};
  for (const engineId of ENGINES) {
    adapterFactories[engineId] = () => createDeterministicAttachmentAdapter(engineId, observations);
  }

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

  const results = [];
  for (let index = 0; index < ENGINES.length; index += 1) {
    results.push(await submitLargePasteTurn(assembly, ENGINES[index], index + 1));
  }

  const violations = [];
  if (!results.every((row) => row.ok)) {
    violations.push({
      kind: 'large_pasted_text_turn_failed',
      failed_engines: results.filter((row) => !row.ok).map((row) => ({
        engine_id: row.engine_id,
        status_code: row.status_code,
        status: row.status,
        output_preview: row.output_preview,
      })),
    });
  }
  for (const engineId of ENGINES) {
    const observed = observations.find((row) => row.engine_id === engineId);
    if (!observed) {
      violations.push({ kind: 'adapter_did_not_receive_turn', engine_id: engineId });
      continue;
    }
    if (!observed.context_pack_has_runtime_attachment_refs) violations.push({ kind: 'runtime_attachment_refs_missing', engine_id: engineId });
    if (!observed.first_attachment.local_read_path_present) violations.push({ kind: 'attachment_local_read_path_missing', engine_id: engineId });
    if (!observed.file_contains_secret) violations.push({ kind: 'attachment_file_secret_not_readable', engine_id: engineId });
    if (observed.normalized_input_contains_secret) violations.push({ kind: 'large_pasted_text_stuffed_into_normalized_input', engine_id: engineId });
    if (observed.first_attachment.size_bytes < 6000) violations.push({ kind: 'attachment_size_did_not_exceed_large_text_threshold', engine_id: engineId });
  }
  for (const result of results) {
    if (result.payload_bytes > 65536) violations.push({ kind: 'turn_projection_exceeds_gateway_budget', engine_id: result.engine_id, payload_bytes: result.payload_bytes });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_attachment_context_bridge_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_public_gateway_route_large_paste_attachment_refs',
    engines_tested: ENGINES,
    results,
    observations,
    sent_count: sent.length,
    budget: {
      default_gateway_budget_bytes: 65536,
      max_payload_bytes: Math.max(0, ...results.map((row) => row.payload_bytes)),
    },
    policy: {
      shell_cognition_policy: 'Shell may pass user input/attachments, but Gateway owns large-text materialization and attachment-ref projection.',
      adapter_policy: 'Adapters receive bounded refs and translate them for private runtimes; they do not decide attachment policy.',
      interchangeability_policy: 'Primary external runtimes must receive the same Gateway attachment-ref shape.',
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
    type: 'agent_runtime_attachment_context_bridge_guard',
    generated_at: new Date().toISOString(),
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['attachment_context_bridge_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
