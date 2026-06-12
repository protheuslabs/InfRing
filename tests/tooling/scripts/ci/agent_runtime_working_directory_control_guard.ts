#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external CLI runtimes honor InfRing working-directory control:
// approved relative artifact proposals apply inside the selected workspace, and
// escaping/out-of-tree artifact proposals fail closed at the Gateway approval
// boundary. The fake CLI exercises the shared Codex/Claude CLI adapter path.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_working_directory_control_guard_current.json');
const SCRATCH_ROOT = path.join(ROOT, 'core/local/artifacts/agent-runtime-working-directory-control-scratch');
const SELECTED_WORKSPACE = path.join(SCRATCH_ROOT, 'selected-workspace');
const OUTSIDE_WORKSPACE = path.join(SCRATCH_ROOT, 'outside-workspace');
const AGENT_ID = 'agent-runtime-working-directory-control-agent';
const ENGINES = ['codex_cli', 'claude_code'];
const CASES = ['inside', 'escape'];

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

function proposalRelPath(engineId, caseId) {
  if (caseId === 'escape') return `../outside-workspace/${engineId}-escape-should-not-write.txt`;
  return `tmp/${engineId}-selected-workspace-apply.txt`;
}

function insideArtifactPath(engineId) {
  return path.join(SELECTED_WORKSPACE, 'tmp', `${engineId}-selected-workspace-apply.txt`);
}

function outsideArtifactPath(engineId) {
  return path.join(OUTSIDE_WORKSPACE, `${engineId}-escape-should-not-write.txt`);
}

function expectedInsideContent(engineId) {
  return `selected workspace artifact for ${engineId}\n`;
}

function writeFixtureCli() {
  const fixturePath = path.join(SCRATCH_ROOT, 'fixtures/working-directory-control-cli.js');
  ensureDir(fixturePath);
  fs.writeFileSync(fixturePath, `#!/usr/bin/env node
'use strict';
const engineId = process.argv[2] || 'external_cli';
const caseId = process.argv[3] || 'inside';
const relPath = caseId === 'escape'
  ? '../outside-workspace/' + engineId + '-escape-should-not-write.txt'
  : 'tmp/' + engineId + '-selected-workspace-apply.txt';
const content = caseId === 'escape'
  ? 'escape artifact should never be written for ' + engineId + '\\n'
  : 'selected workspace artifact for ' + engineId + '\\n';
console.log('Proposing ' + caseId + ' artifact from cwd=' + process.cwd());
console.log(JSON.stringify({
  type: 'infring_universal_tool_proposal',
  tool_id: 'artifact.create_propose',
  reason: 'Working-directory control ' + caseId + ' proposal for ' + engineId + '.',
  arguments: {
    path: relPath,
    mime_type: 'text/plain',
    content
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
    liveEnvVar: `INFRING_WORKING_DIRECTORY_CONTROL_${engineId.toUpperCase()}_LIVE`,
    contextTransportMode: 'bounded_stdin_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'working_directory_control_fixture',
    versionArgs: ['--version'],
    timeoutMs: 8000,
    runArgs: (_prompt, ctx) => {
      const turnId = clean(ctx && ctx.message && ctx.message.turn_id, 200);
      const caseId = turnId.includes('-escape-') ? 'escape' : 'inside';
      return [fixtureCliPath, engineId, caseId];
    },
    runStdin: () => '',
    stableShadowWorkspaceKey: (ctx) => `working-directory-control-${engineId}-${clean(ctx && ctx.message && ctx.message.session_id, 120)}`,
  });
}

async function submitTurn(assembly, engineId, caseId, index) {
  const res = makeResponse();
  const sessionId = `agent-runtime-working-directory-control-${engineId}`;
  const turnId = `working-directory-control-${caseId}-${index}-${engineId}`;
  const traceId = `validation:agent-runtime-working-directory-control:${engineId}:${caseId}:${Date.now()}:${index}`;
  const prompt = `${engineId} ${caseId} working directory control artifact proposal.`;
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
        working_directory: SELECTED_WORKSPACE,
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
    handled,
    status_code: res.statusCode,
    payload: res.payload || {},
  };
}

async function approvePending(assembly, pending, caseId) {
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
    traceId: `validation:agent-runtime-working-directory-control-decision:${caseId}:${approvalId}`,
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
  const insidePath = insideArtifactPath(turn.engine_id);
  const outsidePath = outsideArtifactPath(turn.engine_id);
  let insideText = '';
  let outsideText = '';
  try { insideText = fs.readFileSync(insidePath, 'utf8'); } catch {}
  try { outsideText = fs.readFileSync(outsidePath, 'utf8'); } catch {}
  const expectedInside = expectedInsideContent(turn.engine_id);
  const isInsideCase = turn.case_id === 'inside';
  const text = clean(payload.text || payload.display_text || payload.output_text, 4000);
  const baseChecks = !!(
    turn.handled &&
    turn.status_code === 200 &&
    clean(payload.status, 120) === 'permission_required' &&
    pending &&
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
    decisionPayload.pending_request_found === true &&
    decisionPayload.decision_receipt_ref &&
    decisionPayload.decision_receipt
  );
  const insideOk = isInsideCase && !!(
    decisionPayload.durable_effect_executed === true &&
    execution.effect === 'artifact_written' &&
    execution.working_directory === SELECTED_WORKSPACE &&
    execution.path === proposalRelPath(turn.engine_id, 'inside') &&
    clean(execution.result_ref, 500) === `artifact/${proposalRelPath(turn.engine_id, 'inside')}` &&
    insideText === expectedInside &&
    !outsideText
  );
  const escapeOk = !isInsideCase && !!(
    decisionPayload.durable_effect_executed === false &&
    execution.type === 'agent_runtime_approval_effect_error' &&
    execution.error === 'artifact_path_escapes_workspace' &&
    !outsideText
  );
  return {
    engine_id: turn.engine_id,
    case_id: turn.case_id,
    turn_status: clean(payload.status, 120),
    text,
    pending_present: !!pending,
    pending_path_preview: clean(pending && pending.proposal_arguments_preview, 1200),
    pending_arguments_hidden: !!(pending && pending.proposal_arguments === undefined),
    approval_pause_active: payload.approval_pause_active === true,
    decision_ok: decisionPayload.ok === true,
    decision_receipt_present: !!(decisionPayload.decision_receipt_ref && decisionPayload.decision_receipt),
    durable_effect_executed: decisionPayload.durable_effect_executed === true,
    execution_type: clean(execution.type, 120),
    execution_effect: clean(execution.effect, 120),
    execution_error: clean(execution.error, 240),
    execution_working_directory: clean(execution.working_directory, 1200),
    execution_path: clean(execution.path, 1200),
    inside_artifact_written: insideText === expectedInside,
    outside_artifact_written: !!outsideText,
    selected_workspace: SELECTED_WORKSPACE,
    outside_workspace: OUTSIDE_WORKSPACE,
    ok: baseChecks && (insideOk || escapeOk),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(SELECTED_WORKSPACE, { recursive: true });
  fs.mkdirSync(OUTSIDE_WORKSPACE, { recursive: true });
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
  let index = 0;
  for (const engineId of ENGINES) {
    for (const caseId of CASES) {
      index += 1;
      const turn = await submitTurn(assembly, engineId, caseId, index);
      const pending = turn.payload && (turn.payload.pending_permission_request || turn.payload.permission_request);
      const decision = pending ? await approvePending(assembly, pending, caseId) : null;
      results.push(summarizeCase(turn, decision));
    }
  }

  const violations = results
    .filter((row) => !row.ok)
    .map((row) => ({
      kind: 'working_directory_control_failed',
      engine_id: row.engine_id,
      case_id: row.case_id,
      turn_status: row.turn_status,
      pending_present: row.pending_present,
      decision_ok: row.decision_ok,
      durable_effect_executed: row.durable_effect_executed,
      execution_working_directory: row.execution_working_directory,
      execution_path: row.execution_path,
      execution_error: row.execution_error,
      inside_artifact_written: row.inside_artifact_written,
      outside_artifact_written: row.outside_artifact_written,
    }));

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_working_directory_control_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_cli_selected_workspace_and_escape_rejection',
    engines_tested: ENGINES,
    cases_tested: CASES,
    selected_workspace: SELECTED_WORKSPACE,
    outside_workspace: OUTSIDE_WORKSPACE,
    results,
    policy: {
      shell_replaceability_policy: 'The guard uses Gateway turn and approval routes only; Shell never decides cwd authority.',
      selected_workspace_policy: 'Approved relative artifact effects resolve under the explicit turn working_directory.',
      escape_policy: 'Out-of-tree proposal paths fail closed and do not write outside the selected workspace.',
      adapter_policy: 'Adapters translate CLI proposal text; Gateway approval policy resolves and enforces workspace boundaries.',
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
    type: 'agent_runtime_working_directory_control_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['working_directory_control_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
