#!/usr/bin/env node
/* eslint-disable no-console */

// Proves approved artifact proposals from the primary external CLI runtimes
// become real files through Gateway approval decisions with effect receipts.
// This keeps durable artifact authority in Gateway while adapters only
// translate external runtime proposal text.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_artifact_proposal_apply_guard_current.json');
const SCRATCH_ROOT = path.join(ROOT, 'core/local/artifacts/agent-runtime-artifact-proposal-apply-scratch');
const AGENT_ID = 'agent-runtime-artifact-proposal-apply-agent';
const ENGINES = ['codex_cli', 'claude_code'];
const CASES = ['create', 'edit'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function artifactRel(engineId, caseId) {
  return `tmp/${engineId}-${caseId}-proposal-apply.txt`;
}

function artifactPath(engineId, caseId) {
  return path.join(SCRATCH_ROOT, artifactRel(engineId, caseId));
}

function expectedContent(engineId, caseId) {
  return `artifact proposal apply ${caseId} for ${engineId}\n`;
}

function writeScratchRegistry() {
  const registryPath = path.join(SCRATCH_ROOT, 'validation/conformance/contracts/agent_runtime_engine_registry.json');
  ensureDir(registryPath);
  fs.writeFileSync(registryPath, `${JSON.stringify({
    type: 'agent_runtime_engine_registry',
    engines: ENGINES.map((engineId) => ({
      engine_id: engineId,
      display_name: engineId === 'codex_cli' ? 'Codex' : 'Claude Code',
      status: 'available',
      runtime_kind: 'external_cli_adapter',
      adapter_id: engineId,
    })),
  }, null, 2)}\n`, 'utf8');
}

function writeFixtureCli() {
  const fixturePath = path.join(SCRATCH_ROOT, 'fixtures/artifact-proposal-cli.js');
  ensureDir(fixturePath);
  fs.writeFileSync(fixturePath, `#!/usr/bin/env node
'use strict';
const engineId = process.argv[2] || 'external_cli';
const caseId = process.argv[3] || 'create';
const artifactPath = 'tmp/' + engineId + '-' + caseId + '-proposal-apply.txt';
console.log('The runtime is proposing an artifact ' + caseId + ' through InfRing.');
console.log(JSON.stringify({
  type: 'infring_universal_tool_proposal',
  tool_id: 'artifact.create_propose',
  reason: 'Apply ' + caseId + ' artifact proposal for ' + engineId + '.',
  arguments: {
    path: artifactPath,
    mime_type: 'text/plain',
    content: 'artifact proposal apply ' + caseId + ' for ' + engineId + '\\n'
  }
}));
`, 'utf8');
  try { fs.chmodSync(fixturePath, 0o755); } catch {}
  return fixturePath;
}

function createFixtureAdapter(engineId, fixtureCliPath) {
  const { createCliRuntimeEngineAdapter } = require(path.join(
    ROOT,
    'adapters/runtime/agent_engines/cli_runtime_adapter.ts',
  ));
  return createCliRuntimeEngineAdapter({
    engineId,
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    liveEnvVar: `INFRING_ARTIFACT_PROPOSAL_APPLY_${engineId.toUpperCase()}_LIVE`,
    contextTransportMode: 'bounded_stdin_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'artifact_proposal_apply_fixture',
    versionArgs: ['--version'],
    timeoutMs: 8000,
    runArgs: (_prompt, ctx) => {
      const turnId = clean(ctx && ctx.message && ctx.message.turn_id, 200);
      const caseId = turnId.includes('-edit-') ? 'edit' : 'create';
      return [fixtureCliPath, engineId, caseId];
    },
    runStdin: () => '',
    stableShadowWorkspaceKey: (ctx) => `artifact-proposal-apply-${engineId}-${clean(ctx && ctx.message && ctx.message.session_id, 120)}`,
  });
}

async function submitTurn(assembly, engineId, caseId, index) {
  const res = makeResponse();
  const sessionId = `agent-runtime-artifact-proposal-apply-${engineId}`;
  const turnId = `artifact-proposal-apply-${caseId}-${index}-${engineId}`;
  const traceId = `validation:agent-runtime-artifact-proposal-apply:${engineId}:${caseId}:${Date.now()}:${index}`;
  const prompt = `${engineId} ${caseId} an artifact through an InfRing approval proposal.`;
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
        working_directory: SCRATCH_ROOT,
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
    case_id: caseId,
    session_id: sessionId,
    turn_id: turnId,
    trace_id: traceId,
    prompt,
    handled,
    status_code: res.statusCode,
    payload: res.payload || {},
  };
}

async function approvePending(assembly, pending) {
  const res = makeResponse();
  const approvalId = clean(pending && pending.approval_id, 260);
  const handled = await assembly.handleAgentRuntimeApprovalRoute({
    req: {
      method: 'POST',
      __body: {
        decision: 'allow_once',
        gatekeeper_kind: 'user',
      },
    },
    res,
    pathname: `/api/shell-socket/approvals/${encodeURIComponent(approvalId)}/decision`,
    traceId: `validation:agent-runtime-artifact-proposal-apply-decision:${approvalId}`,
    flags: {},
  });
  return {
    handled,
    status_code: res.statusCode,
    payload: res.payload || {},
  };
}

function summarizeCase(turn, decision) {
  const payload = turn.payload || {};
  const pending = payload.pending_permission_request || payload.permission_request || null;
  const decisionPayload = decision && decision.payload || {};
  const execution = decisionPayload.execution_result || {};
  const target = artifactPath(turn.engine_id, turn.case_id);
  let content = '';
  try { content = fs.readFileSync(target, 'utf8'); } catch {}
  const expected = expectedContent(turn.engine_id, turn.case_id);
  const text = clean(payload.text || payload.display_text || payload.output_text, 4000);
  return {
    engine_id: turn.engine_id,
    case_id: turn.case_id,
    handled: turn.handled,
    status_code: turn.status_code,
    turn_status: clean(payload.status, 120),
    text,
    pending_present: !!pending,
    pending_tool_id: clean(pending && pending.tool_id, 120),
    pending_status: clean(pending && pending.status, 120),
    pending_turn_status: clean(pending && pending.turn_status, 120),
    pending_resume_token_present: !!(pending && pending.resume_token),
    pending_arguments_hidden: !!(pending && pending.proposal_arguments === undefined),
    pending_arguments_ref_present: !!(pending && pending.proposal_arguments_ref),
    pending_preview_contains_path: clean(pending && pending.proposal_arguments_preview, 1200).includes(artifactRel(turn.engine_id, turn.case_id)),
    approval_pause_active: payload.approval_pause_active === true,
    decision_ok: !!(decision && decision.handled && decision.status_code === 200 && decisionPayload.ok === true),
    decision_receipt_present: !!(decisionPayload.decision_receipt_ref && decisionPayload.decision_receipt),
    decision_receipt_hash_present: !!(decisionPayload.decision_receipt && decisionPayload.decision_receipt.receipt_hash),
    durable_effect_executed: decisionPayload.durable_effect_executed === true,
    execution_effect: clean(execution.effect, 120),
    execution_path: clean(execution.path, 500),
    execution_bytes: Number(execution.bytes) || 0,
    execution_sha256_present: /^[a-f0-9]{64}$/.test(clean(execution.sha256, 80)),
    execution_result_ref: clean(execution.result_ref, 500),
    execution_receipt_ref: clean(execution.receipt_ref, 500),
    artifact_written: content === expected,
    artifact_path: target,
    ok: !!(
      turn.handled &&
      turn.status_code === 200 &&
      clean(payload.status, 120) === 'permission_required' &&
      /^Permission required:/.test(text) &&
      pending &&
      pending.tool_id === 'artifact.create_propose' &&
      pending.status === 'paused_pending_approval' &&
      pending.turn_status === 'permission_required' &&
      pending.resume_token &&
      pending.proposal_arguments === undefined &&
      pending.proposal_arguments_ref &&
      payload.approval_pause_active === true &&
      decision &&
      decision.handled &&
      decision.status_code === 200 &&
      decisionPayload.ok === true &&
      decisionPayload.decision_receipt_ref &&
      decisionPayload.decision_receipt &&
      decisionPayload.durable_effect_executed === true &&
      execution.effect === 'artifact_written' &&
      execution.path === artifactRel(turn.engine_id, turn.case_id) &&
      Number(execution.bytes) === Buffer.byteLength(expected, 'utf8') &&
      /^[a-f0-9]{64}$/.test(clean(execution.sha256, 80)) &&
      clean(execution.result_ref, 500) === `artifact/${artifactRel(turn.engine_id, turn.case_id)}` &&
      clean(execution.receipt_ref, 500).startsWith('receipt/agent-runtime-approval/') &&
      content === expected
    ),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(SCRATCH_ROOT, { recursive: true });
  writeScratchRegistry();
  const fixtureCliPath = writeFixtureCli();

  for (const engineId of ENGINES) {
    ensureDir(artifactPath(engineId, 'edit'));
    fs.writeFileSync(artifactPath(engineId, 'edit'), `old edit content for ${engineId}\n`, 'utf8');
  }

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(
    ROOT,
    'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts',
  ));
  const adapterFactories = {};
  for (const engineId of ENGINES) {
    adapterFactories[engineId] = () => createFixtureAdapter(engineId, fixtureCliPath);
  }
  const assembly = createGatewayAgentRuntimeRouteAssembly({
    root: SCRATCH_ROOT,
    statusDir: path.join(SCRATCH_ROOT, 'state'),
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
  let index = 0;
  for (const engineId of ENGINES) {
    for (const caseId of CASES) {
      index += 1;
      const turn = await submitTurn(assembly, engineId, caseId, index);
      const pending = turn.payload && (turn.payload.pending_permission_request || turn.payload.permission_request);
      const decision = pending ? await approvePending(assembly, pending) : null;
      results.push(summarizeCase(turn, decision));
    }
  }

  const violations = results
    .filter((row) => !row.ok)
    .map((row) => ({
      kind: 'artifact_proposal_apply_failed',
      engine_id: row.engine_id,
      case_id: row.case_id,
      turn_status: row.turn_status,
      pending_present: row.pending_present,
      pending_tool_id: row.pending_tool_id,
      decision_ok: row.decision_ok,
      durable_effect_executed: row.durable_effect_executed,
      execution_effect: row.execution_effect,
      execution_path: row.execution_path,
      execution_bytes: row.execution_bytes,
      execution_sha256_present: row.execution_sha256_present,
      artifact_written: row.artifact_written,
      text: row.text,
    }));

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_artifact_proposal_apply_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_cli_create_edit_proposal_approval_apply',
    engines_tested: ENGINES,
    cases_tested: CASES,
    results,
    policy: {
      shell_replaceability_policy: 'The guard uses Gateway turn and approval routes only; Shell never applies proposal payloads.',
      adapter_policy: 'Adapters translate external CLI proposal text; approval and artifact effects remain Gateway-owned.',
      durable_effect_policy: 'Create/edit artifact writes require approval decision receipts and verified effect receipts.',
    },
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_artifact_proposal_apply_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['artifact_proposal_apply_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
