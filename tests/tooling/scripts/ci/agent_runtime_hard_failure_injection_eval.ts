#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_hard_failure_injection_eval_current.json';
const RECEIPT_JSONL = 'core/local/artifacts/agent_runtime_hard_failure_injection_receipts.jsonl';

function clean(value: any, max = 500): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function requireTs(rel: string): any {
  return require(path.join(ROOT, rel));
}

function minimalContextPack(traceId: string, sessionId: string, agentId: string): JsonObject {
  return {
    source_basis: 'agent_runtime_hard_failure_injection_eval',
    source_authority: 'assurance.validation.agent_runtime_hard_failure_injection_eval',
    trace_id: traceId,
    session_id: sessionId,
    agent_id: agentId,
    row_count: 0,
    fanout_target: 7,
    fragments: [],
    frontier: {
      hot_atom_refs: [],
      warm_span_refs: [],
      cool_span_refs: [],
      cold_span_refs: [],
    },
    universal_tool_grants: {
      source_authority: 'assurance.validation.agent_runtime_hard_failure_injection_eval',
      tools: [],
    },
  };
}

function hasBoundedActivityTrace(row: JsonObject): boolean {
  const trace = row && row.activity_trace && typeof row.activity_trace === 'object' ? row.activity_trace : null;
  return !!(
    trace &&
    trace.type === 'agent_runtime_activity_trace_projection' &&
    trace.collapsed_by_default === true &&
    /^Worked for \d/.test(String(trace.collapse_label || '')) &&
    Array.isArray(trace.rows) &&
    trace.rows.length > 0 &&
    trace.rows.length <= 48 &&
    trace.rows.every((activity: JsonObject) => activity && clean(activity.title, 1000) && !clean(activity.title, 1000).startsWith('{'))
  );
}

function expectedSuffix(kind: string): string {
  if (kind === 'auth') return 'provider_auth_required';
  if (kind === 'quota') return 'provider_quota_or_subscription_unavailable';
  if (kind === 'rate_limit') return 'provider_rate_limited';
  if (kind === 'network') return 'provider_network_unavailable';
  if (kind === 'runtime_missing') return 'runtime_not_available';
  if (kind === 'timeout') return 'turn_timeout';
  if (kind === 'payload_budget') return 'agent_runtime_payload_budget_exceeded';
  if (kind === 'transport') return 'agent_runtime_transport_failure';
  return kind;
}

function buildStoreForScenario(scenario: JsonObject) {
  const { createAgentRuntimeTurnProjectionStore } = requireTs('gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts');
  const { createAgentRuntimeReceiptStore } = requireTs('gateway/runtime/agent_runtime/agent_runtime_receipts.ts');
  const receiptStore = createAgentRuntimeReceiptStore({
    root: ROOT,
    receiptPath: path.join(ROOT, RECEIPT_JSONL),
    maxReceipts: 200,
  });
  return createAgentRuntimeTurnProjectionStore({
    root: ROOT,
    contextFanoutTarget: 7,
    loadAgentRuntimeEngineRegistry: () => ({
      engines: [
        {
          engine_id: scenario.engine_id,
          display_name: 'Injected Failure Engine',
          engine_kind: 'external_cli_adapter',
          status: 'adapter_seam_ready',
          implementation_path: 'tests/tooling/scripts/ci/agent_runtime_hard_failure_injection_eval.ts',
        },
      ],
    }),
    findAgentRuntimeEngine: (registry: JsonObject, engineId: string) => (registry.engines || []).find((row: JsonObject) => row.engine_id === engineId),
    loadAgentRuntimeWorkspace: () => ({ workspace_dir: ROOT, active_workspace: ROOT, display_label: 'workspace' }),
    createRouter: () => ({
      healthCheck: async () => scenario.health,
      startSession: async () => ({ ok: true }),
      submitTurn: async () => scenario.turn,
      streamTurn: async () => scenario.turn,
    }),
    sanitizeAgentRuntimeActivityEvent: (event: JsonObject, index: number, defaults: JsonObject) => ({
      ...event,
      sequence_no: Number(event.sequence_no || index + 1) || index + 1,
      engine_id: event.engine_id || defaults.engineId,
      trace_id: event.trace_id || defaults.traceId,
      session_id: event.session_id || defaults.sessionId,
      turn_id: event.turn_id || defaults.turnId,
    }),
    appendAgentRuntimeTranscriptTurn: () => {},
    appendAgentRuntimeTurnAtoms: () => {},
    ingestAgentRuntimeContextProjection: () => {},
    loadAgentRuntimeContextRows: () => [],
    materializeKernelAgentRuntimeContextPack: async () => ({ ok: false, reason: 'eval_uses_gateway_context_fallback' }),
    materializeAgentRuntimeContextPack: ({ traceId, sessionId, agentId }: JsonObject) => minimalContextPack(traceId, sessionId, agentId),
    buildAgentRuntimeContextPack: ({ traceId, sessionId, agentId }: JsonObject) => minimalContextPack(traceId, sessionId, agentId),
    mergeAgentRuntimeApprovalPermissionPolicy: (policy: JsonObject) => policy || {},
    buildUniversalToolGrants: () => ({
      source_authority: 'assurance.validation.agent_runtime_hard_failure_injection_eval',
      tools: [],
    }),
    recordAgentRuntimeTurnReceipts: receiptStore.recordAgentRuntimeTurnReceipts,
  });
}

async function runScenario(scenario: JsonObject): Promise<JsonObject> {
  const traceId = `trace-hard-failure-${scenario.kind}-${Date.now()}`;
  const sessionId = `hard-failure-${scenario.kind}`;
  const turnId = `turn-${scenario.kind}`;
  const store = buildStoreForScenario(scenario);
  const projection = await store.agentRuntimeTurnProjection(traceId, {
    engine_id: scenario.engine_id,
    agent_id: 'agent-runtime-hard-failure-eval',
    session_id: sessionId,
    turn_id: turnId,
    message: `Trigger ${scenario.kind} failure projection.`,
    input_text: `Trigger ${scenario.kind} failure projection.`,
  });
  const statusOk = scenario.kind === 'timeout'
    ? projection.status === 'timed_out_with_reason'
    : projection.status === 'failed_with_reason';
  const expected = expectedSuffix(scenario.kind);
  const errorCode = clean(projection.error_code, 200).toLowerCase();
  const errorOk = scenario.kind === 'timeout'
    ? errorCode.includes('timeout')
    : errorCode.endsWith(expected) || errorCode.includes(expected);
  const result = {
    kind: scenario.kind,
    engine_id: scenario.engine_id,
    ok: !!(
      projection &&
      projection.status_code === 200 &&
      statusOk &&
      errorOk &&
      clean(projection.display_text, 1200) &&
      Array.isArray(projection.receipt_refs) &&
      projection.receipt_refs.length >= 3 &&
      hasBoundedActivityTrace(projection) &&
      Array.isArray(projection.next_actions) &&
      projection.next_actions.length > 0
    ),
    status_code: projection.status_code,
    status: projection.status,
    error_code: projection.error_code,
    display_text_present: !!clean(projection.display_text, 1200),
    receipt_refs: Array.isArray(projection.receipt_refs) ? projection.receipt_refs.length : 0,
    activity_trace: hasBoundedActivityTrace(projection),
    next_actions: Array.isArray(projection.next_actions) ? projection.next_actions.length : 0,
  };
  return result;
}

async function main() {
  try { fs.rmSync(path.join(ROOT, RECEIPT_JSONL), { force: true }); } catch {}
  const baseEngine = 'injected_failure_engine';
  const scenarios = [
    {
      kind: 'auth',
      engine_id: baseEngine,
      health: { status: 'unavailable', reason: 'API key missing; login required before use.' },
    },
    {
      kind: 'quota',
      engine_id: baseEngine,
      health: { status: 'unavailable', reason: 'Subscription cancelled; quota exhausted and billing unavailable.' },
    },
    {
      kind: 'rate_limit',
      engine_id: baseEngine,
      health: { status: 'unavailable', reason: '429 too many requests; provider rate limit exceeded.' },
    },
    {
      kind: 'network',
      engine_id: baseEngine,
      health: { status: 'unavailable', reason: 'Network transport unavailable: ECONNRESET socket hang up.' },
    },
    {
      kind: 'runtime_missing',
      engine_id: baseEngine,
      health: { status: 'unavailable', reason: 'Command not found; runtime binary not installed.' },
    },
    {
      kind: 'timeout',
      engine_id: baseEngine,
      health: { status: 'available', version_preview: 'injected ok' },
      turn: {
        type: 'turn.complete',
        status: 'failed',
        timed_out: true,
        timeout_ms: 1234,
        error_code: 'turn_timeout',
        reason: 'Injected runtime timeout after Gateway turn budget.',
        output_text: 'Injected runtime timeout after Gateway turn budget.',
      },
    },
    {
      kind: 'payload_budget',
      engine_id: baseEngine,
      health: { status: 'available', version_preview: 'injected ok' },
      turn: {
        type: 'turn.complete',
        status: 'failed',
        error_code: 'agent_runtime_payload_budget_exceeded',
        reason: 'Agent runtime event exceeded default Gateway payload budget.',
        output_text: 'Agent runtime event exceeded default Gateway payload budget.',
      },
    },
    {
      kind: 'transport',
      engine_id: baseEngine,
      health: { status: 'available', version_preview: 'injected ok' },
      turn: {
        type: 'turn.complete',
        status: 'failed',
        error_code: 'agent_runtime_transport_failure',
        reason: 'Runtime socket transport closed before final response.',
        output_text: 'Runtime socket transport closed before final response.',
      },
    },
  ];
  const results = [];
  for (const scenario of scenarios) {
    results.push(await runScenario(scenario));
  }
  const report = {
    ok: results.every((row) => row.ok),
    type: 'agent_runtime_hard_failure_injection_eval',
    generated_at: new Date().toISOString(),
    turn_outcome_contract: 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json',
    receipt_probe_path: RECEIPT_JSONL,
    summary: {
      scenario_count: results.length,
      passed: results.filter((row) => row.ok).length,
      failed: results.filter((row) => !row.ok).length,
    },
    results,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_hard_failure_injection_eval',
    error: clean(error && error.stack ? error.stack : error, 4000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
