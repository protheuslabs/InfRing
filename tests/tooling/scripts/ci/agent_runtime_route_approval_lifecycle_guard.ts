#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_route_approval_lifecycle_guard_current.json');
const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function createFakeApprovalRuntimeAdapter(artifactRel, artifactText) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      status: 'available',
      discovery_source: 'route_approval_lifecycle_guard_fake_adapter',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      status: 'started',
    }),
    submit_turn: async ({ message }) => ({
      type: 'infring_universal_tool_proposal',
      trace_id: message && message.trace_id,
      request_id: message && message.request_id,
      engine_id: message && message.engine_id,
      session_id: message && message.session_id,
      turn_id: message && message.turn_id,
      tool_id: 'artifact.create_propose',
      reason: 'route-level approval lifecycle artifact write',
      arguments: {
        path: artifactRel,
        mime_type: 'text/plain',
        content: artifactText,
      },
    }),
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, receipt_refs: [] }),
  };
}

async function main() {
  const scratchDir = path.join(ROOT, 'core', 'local', 'artifacts', 'agent-runtime-route-approval-lifecycle-scratch');
  const artifactRel = 'core/local/artifacts/agent-runtime-route-approval-lifecycle-scratch/tmp/route-level-approval.txt';
  const artifactAbs = path.join(ROOT, artifactRel);
  const artifactText = 'route approval lifecycle proof\n';
  try { fs.rmSync(scratchDir, { recursive: true, force: true }); } catch {}

  const sent = [];
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(scratchDir, 'state'),
    adapterFactories: {
      codex_cli: () => createFakeApprovalRuntimeAdapter(artifactRel, artifactText),
    },
    readJsonBody: async (req) => req && req.__body || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
      sent.push({ statusCode, payload });
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });

  const traceId = `validation:agent-runtime-route-approval:${Date.now()}`;
  const turnReq = {
    method: 'POST',
    __body: {
      agent_id: 'agent-runtime-route-approval-lifecycle-guard',
      session_id: 'route-approval-session',
      conversation_id: 'route-approval-session',
      engine_id: 'codex_cli',
      message: 'create the route-level approval artifact',
      input_text: 'create the route-level approval artifact',
      working_directory: scratchDir,
      test_probe: true,
    },
  };
  const turnRes = makeResponse();
  const turnHandled = await assembly.handleAgentRuntimeTurnRoute({
    req: turnReq,
    res: turnRes,
    pathname: '/api/shell-socket/agent-runtime/turn',
    traceId,
    flags: {},
  });

  const turnPayload = turnRes.payload || {};
  const request = turnPayload.pending_permission_request || turnPayload.permission_request || null;
  const permissionActivity = Array.isArray(turnPayload.agent_activity_events)
    ? turnPayload.agent_activity_events.find((row) =>
      row &&
      row.provider_event_type === 'permission.requested' &&
      row.status === 'paused_pending_approval' &&
      String(row.display_text || '').includes('Permission required:')
    )
    : null;
  const approvalRoute = request && request.approval_route || '';
  const decisionReq = { method: 'POST', __body: { decision: 'allow_once' } };
  const decisionRes = makeResponse();
  const decisionHandled = approvalRoute
    ? await assembly.handleAgentRuntimeApprovalRoute({
      req: decisionReq,
      res: decisionRes,
      pathname: approvalRoute,
      traceId,
    })
    : false;

  const decisionPayload = decisionRes.payload || {};
  const wroteArtifact = fs.existsSync(artifactAbs) && fs.readFileSync(artifactAbs, 'utf8').includes(artifactText.trim());
  const ok = !!(
    turnHandled === true &&
    turnRes.statusCode === 200 &&
    turnPayload.status === 'permission_required' &&
    turnPayload.pending_permission === true &&
    turnPayload.approval_pause &&
    request &&
    request.turn_status === 'permission_required' &&
    request.status === 'paused_pending_approval' &&
    request.source === 'gateway_universal_tool_proposal_normalizer' &&
    request.resume_strategy === 'gateway_apply_approved_effect' &&
    request.proposal_arguments_ref &&
    !request.proposal_arguments &&
    permissionActivity &&
    decisionHandled === true &&
    decisionRes.statusCode === 200 &&
    decisionPayload.ok === true &&
    decisionPayload.pending_request_found === true &&
    decisionPayload.durable_effect_executed === true &&
    decisionPayload.resume_token === request.resume_token &&
    decisionPayload.decision_receipt &&
    decisionPayload.decision_receipt.receipt_hash &&
    wroteArtifact
  );

  const report = {
    ok,
    type: 'agent_runtime_route_approval_lifecycle_guard',
    generated_at: new Date().toISOString(),
    trace_id: traceId,
    route_probe: {
      turn_handled: turnHandled,
      turn_status_code: turnRes.statusCode,
      turn_status: clean(turnPayload.status, 120),
      pending_permission: turnPayload.pending_permission === true,
      approval_route: clean(approvalRoute, 300),
      pending_request_source: clean(request && request.source, 160),
      resume_strategy: clean(request && request.resume_strategy, 160),
      shell_projection_bounded: !!(request && request.proposal_arguments_ref && !request.proposal_arguments),
      permission_activity_emitted: !!permissionActivity,
    },
    decision_probe: {
      decision_handled: decisionHandled,
      decision_status_code: decisionRes.statusCode,
      ok: decisionPayload.ok === true,
      pending_request_found: decisionPayload.pending_request_found === true,
      durable_effect_executed: decisionPayload.durable_effect_executed === true,
      resume_token_continuity: !!(request && decisionPayload.resume_token === request.resume_token),
      decision_receipt_ref: clean(decisionPayload.decision_receipt_ref, 300),
      decision_receipt_hash_present: !!(decisionPayload.decision_receipt && decisionPayload.decision_receipt.receipt_hash),
      wrote_artifact: wroteArtifact,
    },
    failures: [],
  };
  if (!ok) {
    report.failures.push({
      kind: 'route_level_approval_pause_decision_effect_lifecycle_broken',
      turn_payload_status: clean(turnPayload.status, 120),
      decision_payload_status: clean(decisionPayload.type || decisionPayload.error, 240),
      sent_count: sent.length,
    });
  }

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(scratchDir, { recursive: true, force: true }); } catch {}
  if (!ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_route_approval_lifecycle_guard',
    generated_at: new Date().toISOString(),
    failures: [{ kind: 'route_level_approval_lifecycle_probe_failed', error: String(error && error.stack || error && error.message || error) }],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
