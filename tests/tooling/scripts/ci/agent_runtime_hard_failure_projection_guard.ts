#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external runtimes surface hard provider/runtime failures as
// visible, classified Gateway terminal outcomes. This covers the practical
// "agent vanished" class: quota, auth, rate limit, network, and missing runtime
// failures must become failed_with_reason with user-facing text and next actions.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_hard_failure_projection_guard_current.json');
const SCRATCH_ROOT = path.join(ROOT, 'core/local/artifacts/agent-runtime-hard-failure-projection-scratch');
const AGENT_ID = 'agent-runtime-hard-failure-projection-agent';
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

const FAILURE_CASES = [
  {
    case_id: 'quota',
    stderr: (engineId) => engineId === 'codex_cli'
      ? "You've hit your usage limit for GPT-5.3-Codex-Spark. Switch to another model now, or try again at 1:49 PM."
      : 'Claude Code subscription is unavailable because account usage is exhausted.',
    expected_code_suffix: 'provider_quota_or_subscription_unavailable',
    must_include: (engineId) => engineId === 'codex_cli'
      ? ['usage limit', 'GPT-5.3-Codex-Spark', '1:49 PM']
      : ['Claude Code subscription is unavailable', 'usage is exhausted'],
    must_exclude: () => [],
  },
  {
    case_id: 'auth',
    stderr: () => 'Failed to authenticate. API Error: 401 Invalid authentication credentials',
    expected_code_suffix: 'provider_auth_required',
    must_include: () => ['401', 'Invalid authentication credentials'],
    must_exclude: () => [],
  },
  {
    case_id: 'rate_limit',
    stderr: () => 'Provider API Error: 429 Too many requests. Rate limit exceeded.',
    expected_code_suffix: 'provider_rate_limited',
    must_include: () => ['429', 'Rate limit'],
    must_exclude: () => [],
  },
  {
    case_id: 'network',
    stderr: () => 'Network failure: ENOTFOUND provider.example.test while connecting to runtime transport.',
    expected_code_suffix: 'provider_network_unavailable',
    must_include: () => ['ENOTFOUND', 'Network failure'],
    must_exclude: () => [],
  },
  {
    case_id: 'plugin_noise_quota',
    engines: ['claude_code'],
    stderr: () => [
      JSON.stringify('rust-analyzer-lsp@claude-plugins-official'),
      'Claude Code subscription is unavailable because account usage is exhausted.',
    ].join('\n'),
    expected_code_suffix: 'provider_quota_or_subscription_unavailable',
    must_include: () => ['Claude Code subscription is unavailable', 'usage is exhausted'],
    must_exclude: () => ['rust-analyzer-lsp@claude-plugins-official'],
  },
  {
    case_id: 'missing_runtime',
    expected_code_suffix: 'runtime_not_available',
    must_include: (engineId) => [engineId, 'command not found'],
    must_exclude: () => [],
    missing_runtime: true,
  },
];

function caseAppliesToEngine(row, engineId) {
  return !Array.isArray(row.engines) || row.engines.includes(engineId);
}

function createCliFailureAdapter(engineId, failureCase) {
  const { createCliRuntimeEngineAdapter } = require(path.join(
    ROOT,
    'adapters/runtime/agent_engines/cli_runtime_adapter.ts',
  ));
  const stderrText = failureCase.stderr(engineId);
  const childScript = [
    `process.stderr.write(${JSON.stringify(stderrText)} + '\\n');`,
    'process.exit(7);',
  ].join('\n');
  return createCliRuntimeEngineAdapter({
    engineId,
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    liveEnvVar: `INFRING_HARD_FAILURE_${engineId.toUpperCase()}_LIVE`,
    contextTransportMode: 'bounded_stdin_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'hard_failure_projection_fixture',
    versionArgs: ['--version'],
    timeoutMs: 8000,
    runArgs: () => ['-e', childScript],
    runStdin: () => '',
  });
}

function createMissingRuntimeAdapter(engineId) {
  const base = (message, type) => ({
    type,
    trace_id: message && message.trace_id,
    request_id: message && message.request_id,
    engine_id: engineId,
    session_id: message && message.session_id,
    turn_id: message && message.turn_id,
  });
  return {
    health_check: async ({ message }) => ({
      ...base(message, 'engine.health.result'),
      status: 'unavailable',
      error_code: `${engineId}_runtime_not_available`,
      reason: `${engineId} command not found: ${engineId === 'codex_cli' ? 'codex' : 'claude'}`,
      discovery_source: 'hard_failure_projection_missing_runtime_fixture',
      retryable: true,
    }),
    start_session: async ({ message }) => ({ ...base(message, 'session.started'), status: 'unavailable' }),
    submit_turn: async ({ message }) => ({ ...base(message, 'turn.complete'), status: 'failed', reason: 'missing runtime' }),
    stream_events: async ({ message }) => ({ ...base(message, 'heartbeat'), status: 'unavailable' }),
    cancel_turn: async ({ message }) => ({ ...base(message, 'turn.cancelled'), status: 'cancel_requested' }),
    collect_artifacts: async ({ message }) => ({ ...base(message, 'artifact.list'), artifacts: [] }),
    emit_receipts: async ({ message }) => ({ ...base(message, 'receipt.created'), receipt_refs: [] }),
  };
}

function createAssembly(engineId, failureCase) {
  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(
    ROOT,
    'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts',
  ));
  return createGatewayAgentRuntimeRouteAssembly({
    root: SCRATCH_ROOT,
    statusDir: path.join(SCRATCH_ROOT, 'state', engineId, failureCase.case_id),
    adapterFactories: {
      [engineId]: () => failureCase.missing_runtime
        ? createMissingRuntimeAdapter(engineId)
        : createCliFailureAdapter(engineId, failureCase),
    },
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });
}

async function runCase(engineId, failureCase, index) {
  const assembly = createAssembly(engineId, failureCase);
  const res = makeResponse();
  const sessionId = `agent-runtime-hard-failure-${engineId}-${failureCase.case_id}`;
  const turnId = `hard-failure-${failureCase.case_id}-${index}-${engineId}`;
  const traceId = `validation:agent-runtime-hard-failure:${engineId}:${failureCase.case_id}:${Date.now()}:${index}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: sessionId,
        conversation_id: sessionId,
        engine_id: engineId,
        turn_id: turnId,
        message: `${engineId} ${failureCase.case_id} hard failure projection probe.`,
        input_text: `${engineId} ${failureCase.case_id} hard failure projection probe.`,
        working_directory: SCRATCH_ROOT,
        test_probe: true,
        capability_budget: { max_turn_seconds: 5 },
      },
    },
    res,
    pathname: '/api/agent-runtime/turn',
    traceId,
    flags: {},
  });
  return summarizeCase(engineId, failureCase, handled, res);
}

function summarizeCase(engineId, failureCase, handled, res) {
  const payload = res.payload || {};
  const displayText = clean(payload.display_text || payload.output_text || payload.text || payload.reason, 4000);
  const outputPreview = clean(payload.output_preview, 4000);
  const errorCode = clean(payload.error_code, 200);
  const nextActions = Array.isArray(payload.next_actions) ? payload.next_actions.map((item) => clean(item, 500)).filter(Boolean) : [];
  const traceRows = payload.activity_trace && Array.isArray(payload.activity_trace.rows) ? payload.activity_trace.rows : [];
  const mustInclude = failureCase.must_include(engineId);
  const mustExclude = failureCase.must_exclude(engineId);
  const included = mustInclude.every((text) => displayText.includes(text) || outputPreview.includes(text));
  const excluded = mustExclude.every((text) => !displayText.includes(text) && !outputPreview.includes(text));
  const expectedCode = `${engineId}_${failureCase.expected_code_suffix}`;
  return {
    engine_id: engineId,
    case_id: failureCase.case_id,
    handled: handled === true,
    status_code: res.statusCode,
    turn_status: clean(payload.status, 120),
    ok_field: payload.ok === false,
    error_code: errorCode,
    expected_error_code: expectedCode,
    retryable: payload.retryable === true,
    display_text: displayText,
    output_preview: outputPreview,
    next_action_count: nextActions.length,
    next_actions: nextActions,
    activity_trace_present: !!payload.activity_trace,
    activity_trace_error_row_present: traceRows.some((row) =>
      row && row.activity_kind === 'error' && clean(row.title || row.display_text || row.text, 1000)
    ),
    must_include: mustInclude,
    must_exclude: mustExclude,
    provider_text_preserved: included,
    provider_noise_filtered: excluded,
    raw_stack_not_leaked: !/at .*\(|node:internal|UnhandledPromise|Traceback/.test(`${displayText}\n${outputPreview}`),
    ok: !!(
      handled === true &&
      res.statusCode === 200 &&
      payload.ok === false &&
      payload.status === 'failed_with_reason' &&
      errorCode === expectedCode &&
      displayText &&
      included &&
      excluded &&
      nextActions.length >= 2 &&
      payload.activity_trace &&
      traceRows.some((row) => row && row.activity_kind === 'error') &&
      !/at .*\(|node:internal|UnhandledPromise|Traceback/.test(`${displayText}\n${outputPreview}`)
    ),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(SCRATCH_ROOT, { recursive: true });
  writeScratchRegistry();

  const results = [];
  let index = 0;
  for (const engineId of ENGINES) {
    for (const failureCase of FAILURE_CASES) {
      if (!caseAppliesToEngine(failureCase, engineId)) continue;
      index += 1;
      results.push(await runCase(engineId, failureCase, index));
    }
  }

  const violations = results
    .filter((row) => !row.ok)
    .map((row) => ({
      kind: 'hard_failure_projection_failed',
      engine_id: row.engine_id,
      case_id: row.case_id,
      status_code: row.status_code,
      turn_status: row.turn_status,
      error_code: row.error_code,
      expected_error_code: row.expected_error_code,
      provider_text_preserved: row.provider_text_preserved,
      provider_noise_filtered: row.provider_noise_filtered,
      next_action_count: row.next_action_count,
      activity_trace_present: row.activity_trace_present,
      display_text: row.display_text,
      output_preview: row.output_preview,
    }));

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_hard_failure_projection_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_public_route_hard_failures',
    engines_tested: ENGINES,
    cases_tested: results.map((row) => `${row.engine_id}:${row.case_id}`),
    results,
    policy: {
      shell_replaceability_policy: 'The guard uses Gateway turn projection only; Shell receives terminal outcomes and next actions.',
      adapter_policy: 'Adapters classify provider/runtime failure text and filter provider metadata noise.',
      user_visibility_policy: 'Quota/auth/rate/network/missing-runtime failures must be visible chat terminal states.',
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
    type: 'agent_runtime_hard_failure_projection_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['hard_failure_projection_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_ROOT, { recursive: true, force: true }); } catch {}
  process.exit(1);
});
