#!/usr/bin/env node
/* eslint-disable no-console */

// Deterministic "real work" replay for the Agent Runtime coordinating face.
// This is not a provider smoke test. It proves that every selected runtime
// engine can drive a useful artifact workflow through the public Gateway turn
// route, approval gate, durable effect, transcript overlay, and context reload.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_real_work_replay_guard_current.json');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json';
const LAYER = 'gateway';
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-real-work-replay-scratch');
const ENGINE_REGISTRY_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_engine_registry.json');
const AGENT_ID = 'agent-runtime-real-work-replay-agent';
const SESSION_ID = 'agent-runtime-real-work-replay-session';

function loadRegistryRows() {
  try {
    const registry = JSON.parse(fs.readFileSync(ENGINE_REGISTRY_PATH, 'utf8'));
    const rows = Array.isArray(registry && registry.engines) ? registry.engines : [];
    return rows.filter((row) => row && clean(row.engine_id, 120));
  } catch {
    return [];
  }
}

const ENGINE_ROWS = loadRegistryRows();
const ENGINES = ENGINE_ROWS.map((row) => clean(row.engine_id, 120)).filter(Boolean);

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function safeEngineId(value) {
  return clean(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, '') || 'engine';
}

function contextStatePath(engineId) {
  return path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}-${safeEngineId(engineId)}.json`);
}

function artifactRelPath(engineId) {
  return `core/local/artifacts/agent-runtime-real-work-replay-scratch/apps/${safeEngineId(engineId)}/todo-app.html`;
}

function artifactHtml(engineId) {
  const label = clean(engineId, 80);
  return [
    '<!doctype html>',
    '<html lang="en">',
    '<head>',
    '  <meta charset="utf-8" />',
    '  <meta name="viewport" content="width=device-width, initial-scale=1" />',
    `  <title>InfRing ${label} Todo Replay</title>`,
    '  <style>',
    '    body { font-family: sans-serif; max-width: 42rem; margin: 3rem auto; padding: 0 1rem; }',
    '    form { display: flex; gap: .5rem; }',
    '    input { flex: 1; padding: .7rem; }',
    '    button { padding: .7rem 1rem; }',
    '    li.done span { text-decoration: line-through; color: #666; }',
    '  </style>',
    '</head>',
    '<body>',
    `  <h1>${label} Todo Replay</h1>`,
    '  <form id="todo-form">',
    '    <input id="todo-input" placeholder="Add a task" autocomplete="off" />',
    '    <button>Add</button>',
    '  </form>',
    '  <ul id="todo-list"></ul>',
    '  <script>',
    '    const form = document.getElementById("todo-form");',
    '    const input = document.getElementById("todo-input");',
    '    const list = document.getElementById("todo-list");',
    '    form.addEventListener("submit", (event) => {',
    '      event.preventDefault();',
    '      const text = input.value.trim();',
    '      if (!text) return;',
    '      const item = document.createElement("li");',
    '      const check = document.createElement("input");',
    '      check.type = "checkbox";',
    '      const label = document.createElement("span");',
    '      label.textContent = text;',
    '      const remove = document.createElement("button");',
    '      remove.textContent = "Remove";',
    '      check.addEventListener("change", () => item.classList.toggle("done", check.checked));',
    '      remove.addEventListener("click", () => item.remove());',
    '      item.append(check, label, remove);',
    '      list.append(item);',
    '      input.value = "";',
    '      input.focus();',
    '    });',
    '  </script>',
    '</body>',
    '</html>',
    '',
  ].join('\n');
}

function registryRowFor(engineId) {
  return ENGINE_ROWS.find((row) => clean(row && row.engine_id, 120) === engineId) || {};
}

function isExpectedPlannedUnavailable(engineId, turnPayload) {
  const row = registryRowFor(engineId);
  const status = clean(row && row.status, 160);
  const output = clean(turnPayload && turnPayload.output_preview, 1000).toLowerCase();
  return status.includes('planned_adapter') &&
    clean(turnPayload && turnPayload.status, 120) === 'failed_with_reason' &&
    (output.includes('no live adapter') || output.includes('unavailable'));
}

function createRealWorkAdapter(engineId) {
  const artifactPath = artifactRelPath(engineId);
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'real_work_replay_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => ({
      type: 'infring_universal_tool_proposal',
      trace_id: message && message.trace_id,
      request_id: message && message.request_id,
      engine_id: engineId,
      session_id: message && message.session_id,
      turn_id: message && message.turn_id,
      tool_id: 'artifact.create_propose',
      reason: `Create a small Todo app artifact for ${engineId} through InfRing Gateway approval.`,
      arguments: {
        path: artifactPath,
        mime_type: 'text/html',
        content: artifactHtml(engineId),
      },
    }),
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, engine_id: engineId, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, engine_id: engineId, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, engine_id: engineId, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, engine_id: engineId, receipt_refs: [] }),
  };
}

function artifactQuality(relPath, engineId) {
  const abs = path.join(ROOT, relPath);
  let content = '';
  try { content = fs.readFileSync(abs, 'utf8'); } catch {}
  const checks = {
    exists: fs.existsSync(abs),
    has_html: /<!doctype html>|<html/i.test(content),
    has_form: /<form\b/i.test(content),
    has_input: /<input\b/i.test(content),
    has_button: /<button\b/i.test(content),
    has_event_listener: /addEventListener\(/.test(content),
    names_engine: content.includes(clean(engineId, 80)),
    byte_length: Buffer.byteLength(content, 'utf8'),
  };
  checks.ok = checks.exists &&
    checks.has_html &&
    checks.has_form &&
    checks.has_input &&
    checks.has_button &&
    checks.has_event_listener &&
    checks.names_engine &&
    checks.byte_length > 1000;
  return checks;
}

async function runEngineReplay(assembly, engineId, index) {
  const traceId = `validation:agent-runtime-real-work:${safeEngineId(engineId)}:${Date.now()}:${index}`;
  const sessionId = `${SESSION_ID}-${safeEngineId(engineId)}`;
  const turnRes = makeResponse();
  const turnHandled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: sessionId,
        conversation_id: sessionId,
        engine_id: engineId,
        turn_id: `real-work-turn-${index}-${safeEngineId(engineId)}`,
        message: `Create a very small todo app for ${engineId}.`,
        input_text: `Create a very small todo app for ${engineId}.`,
        working_directory: ROOT,
      },
    },
    res: turnRes,
    pathname: '/api/agent-runtime/turn',
    traceId,
    flags: {},
  });

  const turnPayload = turnRes.payload || {};
  const request = turnPayload.pending_permission_request || turnPayload.permission_request || null;
  const approvalRoute = clean(request && request.approval_route, 500);
  const decisionRes = makeResponse();
  const decisionHandled = approvalRoute
    ? await assembly.handleAgentRuntimeApprovalRoute({
      req: { method: 'POST', __body: { decision: 'allow_once' } },
      res: decisionRes,
      pathname: approvalRoute,
      traceId,
      flags: {},
    })
    : false;

  const relPath = artifactRelPath(engineId);
  const quality = artifactQuality(relPath, engineId);
  const replayOk = !!(
    turnHandled === true &&
    turnRes.statusCode === 200 &&
    turnPayload.status === 'permission_required' &&
    turnPayload.pending_permission === true &&
    request &&
    request.status === 'paused_pending_approval' &&
    request.turn_status === 'permission_required' &&
    request.source === 'gateway_universal_tool_proposal_normalizer' &&
    request.proposal_arguments_ref &&
    !request.proposal_arguments &&
    decisionHandled === true &&
    decisionRes.statusCode === 200 &&
    decisionRes.payload &&
    decisionRes.payload.ok === true &&
    decisionRes.payload.durable_effect_executed === true &&
    decisionRes.payload.pending_request_found === true &&
    quality.ok
  );
  const expectedUnavailable = isExpectedPlannedUnavailable(engineId, turnPayload);
  return {
    engine_id: engineId,
    registry_status: clean(registryRowFor(engineId).status, 160),
    trace_id: traceId,
    session_id: sessionId,
    artifact_rel_path: relPath,
    turn: {
      handled: turnHandled,
      status_code: turnRes.statusCode,
      status: clean(turnPayload.status, 120),
      pending_permission: turnPayload.pending_permission === true,
      approval_pause: !!turnPayload.approval_pause,
      activity_trace: !!(turnPayload.activity_trace && turnPayload.activity_trace.type === 'agent_runtime_activity_trace_projection'),
      output_preview: clean(turnPayload.output_preview, 500),
    },
    permission_request: {
      present: !!request,
      status: clean(request && request.status, 120),
      turn_status: clean(request && request.turn_status, 120),
      source: clean(request && request.source, 160),
      shell_projection_bounded: !!(request && request.proposal_arguments_ref && !request.proposal_arguments),
      approval_route: approvalRoute,
    },
    decision: {
      handled: decisionHandled,
      status_code: decisionRes.statusCode,
      ok: decisionRes.payload && decisionRes.payload.ok === true,
      durable_effect_executed: decisionRes.payload && decisionRes.payload.durable_effect_executed === true,
      pending_request_found: decisionRes.payload && decisionRes.payload.pending_request_found === true,
      decision_receipt_hash_present: !!(decisionRes.payload && decisionRes.payload.decision_receipt && decisionRes.payload.decision_receipt.receipt_hash),
    },
    artifact_quality: quality,
    expected_unavailable: expectedUnavailable,
    replay_status: replayOk ? 'passed' : expectedUnavailable ? 'expected_planned_adapter_unavailable' : 'failed',
    ok: replayOk,
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of ENGINES) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of ENGINES) adapterFactories[engineId] = () => createRealWorkAdapter(engineId);
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
    results.push(await runEngineReplay(assembly, ENGINES[index], index + 1));
  }

  const previewEngine = ENGINES.includes('codex_cli') ? 'codex_cli' : ENGINES[0];
  const merged = assembly.agentRuntimeTranscriptStore.mergeAgentRuntimeTranscriptPayload({
    type: 'session_projection',
    session_id: `${SESSION_ID}-${safeEngineId(previewEngine)}`,
    message_window: { rows: [], total_count: 0 },
  }, {
    agentId: AGENT_ID,
    sessionId: `${SESSION_ID}-${safeEngineId(previewEngine)}`,
    limit: 80,
  });
  const previewRes = makeResponse();
  const previewHandled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: `${SESSION_ID}-${safeEngineId(previewEngine)}`,
        engine_id: previewEngine,
      },
    },
    res: previewRes,
    pathname: '/api/agent-runtime/context-pack/preview',
    traceId: `validation:agent-runtime-real-work-preview:${Date.now()}`,
    flags: {},
  });

  const violations = [];
  for (const result of results) {
    if (!result.ok && !result.expected_unavailable) {
      violations.push({
        kind: 'engine_real_work_replay_failed',
        engine_id: result.engine_id,
        turn: result.turn,
        permission_request: result.permission_request,
        decision: result.decision,
        artifact_quality: result.artifact_quality,
      });
    }
  }
  const transcriptRows = merged && merged.message_window && Array.isArray(merged.message_window.rows)
    ? merged.message_window.rows
    : [];
  if (!merged || !merged.agent_runtime_transcript_overlay || transcriptRows.length !== 2) {
    violations.push({ kind: 'real_work_transcript_overlay_missing', row_count: transcriptRows.length });
  }
  if (Number(merged && merged.agent_runtime_transcript_overlay && merged.agent_runtime_transcript_overlay.row_count) !== 2) {
    violations.push({
      kind: 'real_work_transcript_overlay_cross_session_bleed',
      overlay_row_count: Number(merged && merged.agent_runtime_transcript_overlay && merged.agent_runtime_transcript_overlay.row_count) || 0,
    });
  }
  if (!previewHandled || previewRes.statusCode !== 200 || !(previewRes.payload && previewRes.payload.ok)) {
    violations.push({ kind: 'real_work_context_preview_failed', handled: previewHandled, status_code: previewRes.statusCode });
  }
  if (Number(previewRes.payload && previewRes.payload.row_count) < 2) {
    violations.push({ kind: 'real_work_context_preview_rows_missing', row_count: Number(previewRes.payload && previewRes.payload.row_count) || 0 });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_real_work_replay_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    mode: 'deterministic_public_gateway_route_approval_artifact_replay',
    engine_registry_path: 'validation/conformance/contracts/agent_runtime_engine_registry.json',
    engines_tested: ENGINES,
    successful_engine_count: results.filter((row) => row.ok).length,
    expected_unavailable_count: results.filter((row) => row.expected_unavailable).length,
    results,
    transcript_probe: {
      overlay: merged && merged.agent_runtime_transcript_overlay ? merged.agent_runtime_transcript_overlay : null,
      row_count: transcriptRows.length,
    },
    context_preview_probe: {
      handled: previewHandled,
      status_code: previewRes.statusCode,
      ok: !!(previewRes.payload && previewRes.payload.ok),
      row_count: Number(previewRes.payload && previewRes.payload.row_count) || 0,
      fragment_count: Array.isArray(previewRes.payload && previewRes.payload.fragments) ? previewRes.payload.fragments.length : 0,
    },
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of ENGINES) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_real_work_replay_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of ENGINES) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }
  process.exit(1);
});
