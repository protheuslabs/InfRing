#!/usr/bin/env node
/* eslint-disable no-console */

// Proves native and external Agent Runtime engines share the same Gateway-owned
// per-turn working-directory semantics. Approved artifact effects must resolve
// relative to the explicit turn working_directory, not the repo root, process
// cwd, dashboard workspace default, or adapter-specific state.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_working_directory_parity_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-working-directory-parity-scratch');
const STATUS_DIR = path.join(SCRATCH_DIR, 'state');
const EXPLICIT_WORKSPACE = path.join(SCRATCH_DIR, 'explicit-workspace');
const DEFAULT_WORKSPACE = path.join(SCRATCH_DIR, 'default-workspace');
const ENGINES = ['infring_native', 'codex_cli', 'claude_code', 'grok_code', 'openclaw', 'hermes_agent'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function createProposalAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'working_directory_parity_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => {
      const turnId = clean(message && message.turn_id, 120) || 'turn';
      const artifactRel = `${engineId}/${turnId}.txt`;
      return {
        type: 'infring_universal_tool_proposal',
        trace_id: message && message.trace_id,
        request_id: message && message.request_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: message && message.turn_id,
        tool_id: 'artifact.create_propose',
        reason: `working directory parity write for ${engineId}`,
        arguments: {
          path: artifactRel,
          mime_type: 'text/plain',
          content: `working-directory-parity ${engineId} ${turnId}\n`,
        },
      };
    },
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, engine_id: engineId, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, engine_id: engineId, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, engine_id: engineId, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, engine_id: engineId, receipt_refs: [`receipt/working-directory-parity/${engineId}`] }),
  };
}

async function submitAndApprove(assembly, engineId, index) {
  const turnId = `working-dir-turn-${index}-${engineId}`;
  const artifactRel = `${engineId}/${turnId}.txt`;
  const expectedAbs = path.join(EXPLICIT_WORKSPACE, artifactRel);
  const forbiddenRootAbs = path.join(ROOT, artifactRel);
  const forbiddenDefaultAbs = path.join(DEFAULT_WORKSPACE, artifactRel);
  const traceId = `validation:agent-runtime-working-directory:${engineId}:${Date.now()}:${index}`;
  const turnRes = makeResponse();
  const turnHandled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: 'agent-runtime-working-directory-parity-agent',
        session_id: `agent-runtime-working-directory-parity-${engineId}`,
        conversation_id: `agent-runtime-working-directory-parity-${engineId}`,
        engine_id: engineId,
        turn_id: turnId,
        message: `Ask ${engineId} to write a working directory parity artifact.`,
        input_text: `Ask ${engineId} to write a working directory parity artifact.`,
        working_directory: EXPLICIT_WORKSPACE,
        test_probe: true,
      },
    },
    res: turnRes,
    pathname: '/api/shell-socket/agent-runtime/turn',
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
    })
    : false;
  const decisionPayload = decisionRes.payload || {};
  const expectedText = `working-directory-parity ${engineId} ${turnId}`;
  const expectedExists = fs.existsSync(expectedAbs) && fs.readFileSync(expectedAbs, 'utf8').includes(expectedText);
  const rootLeak = fs.existsSync(forbiddenRootAbs);
  const defaultLeak = fs.existsSync(forbiddenDefaultAbs);
  return {
    engine_id: engineId,
    turn_id: turnId,
    artifact_rel_path: artifactRel,
    expected_path: expectedAbs,
    forbidden_root_path: forbiddenRootAbs,
    forbidden_default_path: forbiddenDefaultAbs,
    turn: {
      handled: turnHandled === true,
      status_code: turnRes.statusCode,
      status: clean(turnPayload.status, 120),
      pending_permission: turnPayload.pending_permission === true,
      approval_pause_active: turnPayload.approval_pause_active === true,
      approval_pause: turnPayload.approval_pause === true,
      request_paused: !!(
        request &&
        request.status === 'paused_pending_approval' &&
        request.turn_status === 'permission_required'
      ),
      request_source: clean(request && request.source, 160),
      shell_projection_bounded: !!(request && request.proposal_arguments_ref && !request.proposal_arguments),
    },
    decision: {
      handled: decisionHandled === true,
      status_code: decisionRes.statusCode,
      ok: decisionPayload.ok === true,
      pending_request_found: decisionPayload.pending_request_found === true,
      durable_effect_executed: decisionPayload.durable_effect_executed === true,
      effect_working_directory: clean(decisionPayload.execution_result && decisionPayload.execution_result.working_directory, 1000),
    },
    filesystem: {
      expected_exists: expectedExists,
      root_leak: rootLeak,
      default_workspace_leak: defaultLeak,
    },
    ok: !!(
      turnHandled === true &&
      turnRes.statusCode === 200 &&
      turnPayload.status === 'permission_required' &&
      turnPayload.pending_permission === true &&
      turnPayload.approval_pause_active === true &&
      request &&
      request.status === 'paused_pending_approval' &&
      request.turn_status === 'permission_required' &&
      request.proposal_arguments_ref &&
      !request.proposal_arguments &&
      decisionHandled === true &&
      decisionRes.statusCode === 200 &&
      decisionPayload.ok === true &&
      decisionPayload.pending_request_found === true &&
      decisionPayload.durable_effect_executed === true &&
      expectedExists &&
      !rootLeak &&
      !defaultLeak
    ),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(EXPLICIT_WORKSPACE, { recursive: true });
  fs.mkdirSync(DEFAULT_WORKSPACE, { recursive: true });

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of ENGINES) adapterFactories[engineId] = () => createProposalAdapter(engineId);
  const sent = [];
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: STATUS_DIR,
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
    results.push(await submitAndApprove(assembly, ENGINES[index], index + 1));
  }

  const violations = [];
  for (const result of results) {
    if (!result.ok) {
      violations.push({
        kind: 'engine_working_directory_parity_failed',
        engine_id: result.engine_id,
        turn: result.turn,
        decision: result.decision,
        filesystem: result.filesystem,
      });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_working_directory_parity_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_public_gateway_route_approval_matrix',
    policy: {
      explicit_turn_working_directory_wins: true,
      approved_artifacts_must_not_resolve_against_repo_root: true,
      native_and_external_engines_share_same_route_semantics: true,
    },
    engines_tested: ENGINES,
    scratch_dir: SCRATCH_DIR,
    explicit_workspace: EXPLICIT_WORKSPACE,
    default_workspace: DEFAULT_WORKSPACE,
    results,
    sent_count: sent.length,
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  console.error(error && error.stack || error);
  process.exit(1);
});
