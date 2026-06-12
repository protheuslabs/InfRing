#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external CLI runtimes cannot convert a blocked mutation into a
// dead-end final answer. If Codex/Claude-style CLI output exits successfully
// with an embedded InfRing universal proposal, the shared adapter plus Gateway
// route must project permission_required, persist a pending request, and apply
// the approved artifact only after a Gateway approval decision.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(
  ROOT,
  'core/local/artifacts/agent_runtime_permission_pause_resume_normalization_guard_current.json',
);
const SCRATCH_ROOT = path.join(
  ROOT,
  'core/local/artifacts/agent-runtime-permission-pause-resume-normalization-scratch',
);
const AGENT_ID = 'agent-runtime-permission-normalization-agent';
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

function writeScratchRegistry() {
  const registryPath = path.join(
    SCRATCH_ROOT,
    'validation/conformance/contracts/agent_runtime_engine_registry.json',
  );
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
  const fixturePath = path.join(SCRATCH_ROOT, 'fixtures/permission-proposal-cli.js');
  ensureDir(fixturePath);
  fs.writeFileSync(fixturePath, `#!/usr/bin/env node
'use strict';
const engineId = process.argv[2] || 'external_cli';
const artifactPath = 'tmp/' + engineId + '-approved-from-proposal.txt';
console.log('I could not write directly because the runtime needs host approval.');
console.log(JSON.stringify({
  type: 'infring_universal_tool_proposal',
  tool_id: 'artifact.create_propose',
  reason: 'Create a permission normalization artifact for ' + engineId + '.',
  arguments: {
    path: artifactPath,
    mime_type: 'text/plain',
    content: 'permission normalization artifact for ' + engineId + '\\n'
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
    liveEnvVar: `INFRING_PERMISSION_NORMALIZATION_${engineId.toUpperCase()}_LIVE`,
    contextTransportMode: 'bounded_stdin_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'permission_normalization_fixture',
    versionArgs: ['--version'],
    timeoutMs: 8000,
    runArgs: () => [fixtureCliPath, engineId],
    runStdin: () => '',
    stableShadowWorkspaceKey: (ctx) => `permission-normalization-${engineId}-${clean(ctx && ctx.message && ctx.message.session_id, 120)}`,
  });
}

async function submitTurn(assembly, engineId, index) {
  const res = makeResponse();
  const sessionId = `agent-runtime-permission-normalization-${engineId}`;
  const turnId = `permission-normalization-${index}-${engineId}`;
  const traceId = `validation:agent-runtime-permission-normalization:${engineId}:${Date.now()}:${index}`;
  const prompt = `${engineId} create a tiny artifact and request approval if needed.`;
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
    traceId: `validation:agent-runtime-permission-normalization-decision:${approvalId}`,
    flags: {},
  });
  return {
    handled,
    status_code: res.statusCode,
    payload: res.payload || {},
  };
}

function artifactFor(engineId) {
  return path.join(SCRATCH_ROOT, 'tmp', `${engineId}-approved-from-proposal.txt`);
}

function summarizeTurn(turn, decision) {
  const payload = turn.payload || {};
  const pending = payload.pending_permission_request || payload.permission_request || null;
  const approvalPause = payload.approval_pause || null;
  const artifactPath = artifactFor(turn.engine_id);
  let artifactText = '';
  try { artifactText = fs.readFileSync(artifactPath, 'utf8'); } catch {}
  const text = clean(payload.text || payload.display_text || payload.output_text, 4000);
  const pendingPreview = clean(pending && pending.proposal_arguments_preview, 1200);
  const decisionPayload = decision && decision.payload || {};
  return {
    engine_id: turn.engine_id,
    handled: turn.handled,
    status_code: turn.status_code,
    turn_status: clean(payload.status, 120),
    text,
    pending_present: !!pending,
    pending_status: clean(pending && pending.status, 120),
    pending_turn_status: clean(pending && pending.turn_status, 120),
    pending_resume_token_present: !!(pending && pending.resume_token),
    pending_arguments_hidden: !!(pending && pending.proposal_arguments === undefined),
    pending_arguments_ref_present: !!(pending && pending.proposal_arguments_ref),
    pending_preview: pendingPreview,
    approval_pause_present: !!approvalPause,
    approval_pause_resume_token_matches: !!(
      approvalPause &&
      pending &&
      clean(approvalPause.resume_token, 260) === clean(pending.resume_token, 260)
    ),
    raw_proposal_not_user_text: !text.includes('infring_universal_tool_proposal'),
    decision_ok: !!(decision && decision.handled && decision.status_code === 200 && decisionPayload.ok === true),
    decision_resumed: decisionPayload.resumed === true,
    decision_receipt_present: !!(decisionPayload.decision_receipt_ref && decisionPayload.decision_receipt),
    durable_effect_executed: decisionPayload.durable_effect_executed === true,
    artifact_written: artifactText.includes(`permission normalization artifact for ${turn.engine_id}`),
    artifact_path: artifactPath,
    ok: !!(
      turn.handled &&
      turn.status_code === 200 &&
      clean(payload.status, 120) === 'permission_required' &&
      /^Permission required:/.test(text) &&
      pending &&
      pending.status === 'paused_pending_approval' &&
      pending.turn_status === 'permission_required' &&
      pending.resume_token &&
      pending.proposal_arguments === undefined &&
      pending.proposal_arguments_ref &&
      approvalPause &&
      clean(approvalPause.resume_token, 260) === clean(pending.resume_token, 260) &&
      !text.includes('infring_universal_tool_proposal') &&
      decision &&
      decision.handled &&
      decision.status_code === 200 &&
      decisionPayload.ok === true &&
      decisionPayload.resumed === true &&
      decisionPayload.decision_receipt_ref &&
      decisionPayload.durable_effect_executed === true &&
      artifactText.includes(`permission normalization artifact for ${turn.engine_id}`)
    ),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(SCRATCH_ROOT, { recursive: true });
  writeScratchRegistry();
  const fixtureCliPath = writeFixtureCli();

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
  for (let index = 0; index < ENGINES.length; index += 1) {
    const engineId = ENGINES[index];
    const turn = await submitTurn(assembly, engineId, index + 1);
    const pending = turn.payload && (turn.payload.pending_permission_request || turn.payload.permission_request);
    const decision = pending ? await approvePending(assembly, pending) : null;
    results.push(summarizeTurn(turn, decision));
  }

  const violations = [];
  for (const row of results) {
    if (row.ok) continue;
    violations.push({
      kind: 'permission_pause_resume_normalization_failed',
      engine_id: row.engine_id,
      status_code: row.status_code,
      turn_status: row.turn_status,
      pending_present: row.pending_present,
      pending_status: row.pending_status,
      pending_turn_status: row.pending_turn_status,
      pending_resume_token_present: row.pending_resume_token_present,
      pending_arguments_hidden: row.pending_arguments_hidden,
      approval_pause_present: row.approval_pause_present,
      raw_proposal_not_user_text: row.raw_proposal_not_user_text,
      decision_ok: row.decision_ok,
      durable_effect_executed: row.durable_effect_executed,
      artifact_written: row.artifact_written,
      text: row.text,
      pending_preview: row.pending_preview,
    });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_permission_pause_resume_normalization_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_cli_final_answer_proposal_pause_resume',
    engines_tested: ENGINES,
    results,
    policy: {
      shell_replaceability_policy: 'The guard uses Gateway turn and approval routes only; Shell never reconstructs proposal arguments.',
      adapter_policy: 'Adapters translate embedded external CLI proposal text into Gateway permission requests; they do not approve or apply effects.',
      durable_effect_policy: 'Artifact write happens only after Gateway approval decision emits receipt.',
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
    type: 'agent_runtime_permission_pause_resume_normalization_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['permission_pause_resume_normalization_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
