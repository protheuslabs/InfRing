#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external runtimes can switch engines while preserving
// InfRing-owned bounded context. The continuity key is introduced by Codex,
// then Claude and Codex must recover it from Gateway context packs without the
// key being repeated in their current user prompts.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_context_switch_continuity_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-context-switch-continuity-scratch');
const AGENT_ID = 'agent-runtime-context-switch-agent';
const SESSION_ID = 'agent-runtime-context-switch-session';
const CONTINUITY_KEY = 'context-switch-key: jade-lynx-884';
const EXPECTED_VALUE = 'jade-lynx-884';

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function collectPackText(pack) {
  const fragments = Array.isArray(pack && pack.fragments) ? pack.fragments : [];
  const parts = [];
  for (const fragment of fragments) {
    const payload = fragment && fragment.payload && typeof fragment.payload === 'object' ? fragment.payload : {};
    parts.push(payload.text_preview, payload.summary, payload.source_ref, fragment.ref_id);
  }
  return parts.map((item) => clean(item, 1600)).filter(Boolean).join('\n');
}

function createContextSwitchAdapter(engineId, observations) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'context_switch_continuity_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => {
      const pack = message && message.context_pack && typeof message.context_pack === 'object' ? message.context_pack : {};
      const currentText = clean(message && message.input && message.input.text, 24000);
      const packText = collectPackText(pack);
      const isSeedTurn = currentText.includes(CONTINUITY_KEY);
      const hasFactInPack = packText.includes(EXPECTED_VALUE);
      const currentPromptLeaksFact = !isSeedTurn && currentText.includes(EXPECTED_VALUE);
      const hasAtomsOrSpans = Array.isArray(pack.fragments) && pack.fragments.length > 0;
      const hasUniversalTools = !!(pack.universal_tool_grants && Array.isArray(pack.universal_tool_grants.tools));
      const observed = {
        engine_id: engineId,
        turn_id: clean(message && message.turn_id, 200),
        is_seed_turn: isSeedTurn,
        current_prompt_leaks_fact: currentPromptLeaksFact,
        has_fact_in_pack: hasFactInPack,
        has_atoms_or_spans: hasAtomsOrSpans,
        has_universal_tools: hasUniversalTools,
        row_count: Number(pack.row_count) || 0,
        raw_row_count: Number(pack.raw_row_count) || Number(pack.row_count) || 0,
        fragment_count: Array.isArray(pack.fragments) ? pack.fragments.length : 0,
        source_basis: clean(pack.source_basis, 160),
        source_authority: clean(pack.source_authority, 200),
      };
      observations.push(observed);
      const ok = isSeedTurn || (hasFactInPack && !currentPromptLeaksFact && hasAtomsOrSpans && hasUniversalTools);
      const output = isSeedTurn
        ? `${engineId} seeded ${CONTINUITY_KEY}.`
        : ok
          ? `${engineId} recovered ${EXPECTED_VALUE} from InfRing context after engine switch.`
          : `${engineId} continuity failed: pack=${hasFactInPack} leaked=${currentPromptLeaksFact} fragments=${observed.fragment_count} tools=${hasUniversalTools}`;
      return {
        type: 'turn.complete',
        trace_id: message && message.trace_id,
        request_id: message && message.request_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: message && message.turn_id,
        status: ok ? 'completed' : 'failed',
        error_code: ok ? '' : `${engineId}_context_switch_continuity_failed`,
        reason: ok ? '' : 'Gateway context pack did not preserve cross-engine continuity.',
        retryable: !ok,
        output_text: output,
        output_preview: output,
        activity_events: [
          {
            type: 'agent_activity_event',
            activity_kind: 'decision_dialog',
            provider_event_type: 'deterministic.context_switch_continuity',
            status: ok ? 'completed' : 'failed',
            display_text: isSeedTurn
              ? `${engineId} introduced the continuity key for later engines.`
              : `${engineId} inspected bounded InfRing context after an engine switch.`,
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
      receipt_refs: [`receipt/context-switch-continuity/${engineId}`],
    }),
  };
}

async function submitTurn(assembly, row, index) {
  const res = makeResponse();
  const traceId = `validation:agent-runtime-context-switch:${row.engine_id}:${Date.now()}:${index}`;
  const turnId = `context-switch-${index}-${row.engine_id}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: SESSION_ID,
        conversation_id: SESSION_ID,
        engine_id: row.engine_id,
        turn_id: turnId,
        message: row.prompt,
        input_text: row.prompt,
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
    engine_id: row.engine_id,
    turn_id: turnId,
    prompt_contains_fact: row.prompt.includes(EXPECTED_VALUE),
    handled,
    status_code: res.statusCode,
    status: clean(res.payload && res.payload.status, 120),
    output_preview: clean(res.payload && (res.payload.output_preview || res.payload.output_text || res.payload.text), 1200),
    payload_bytes: Buffer.byteLength(JSON.stringify(res.payload || {}), 'utf8'),
    ok: !!(handled && res.statusCode === 200 && res.payload && res.payload.status === 'completed'),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}.json`), { force: true }); } catch {}

  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const observations = [];
  const adapterFactories = {
    codex_cli: () => createContextSwitchAdapter('codex_cli', observations),
    claude_code: () => createContextSwitchAdapter('claude_code', observations),
  };
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

  const turns = [
    {
      engine_id: 'codex_cli',
      prompt: `Remember this InfRing shared context fact for later engine switches: ${CONTINUITY_KEY}.`,
    },
    {
      engine_id: 'claude_code',
      prompt: 'What was the shared context fact from the previous engine? Answer from InfRing context only.',
    },
    {
      engine_id: 'codex_cli',
      prompt: 'After switching back, what continuity value did the other engine preserve? Answer from InfRing context only.',
    },
  ];
  const results = [];
  for (let index = 0; index < turns.length; index += 1) {
    results.push(await submitTurn(assembly, turns[index], index + 1));
  }

  const nonSeedObservations = observations.filter((row) => !row.is_seed_turn);
  const violations = [];
  if (!results.every((row) => row.ok)) {
    violations.push({
      kind: 'context_switch_turn_failed',
      failed_turns: results.filter((row) => !row.ok).map((row) => ({
        engine_id: row.engine_id,
        status: row.status,
        output_preview: row.output_preview,
      })),
    });
  }
  if (nonSeedObservations.length !== 2) violations.push({ kind: 'expected_two_non_seed_switch_observations', actual: nonSeedObservations.length });
  for (const observed of nonSeedObservations) {
    if (!observed.has_fact_in_pack) violations.push({ kind: 'continuity_fact_missing_from_context_pack', engine_id: observed.engine_id });
    if (observed.current_prompt_leaks_fact) violations.push({ kind: 'continuity_fact_leaked_by_current_prompt', engine_id: observed.engine_id });
    if (!observed.has_atoms_or_spans) violations.push({ kind: 'context_pack_missing_atoms_or_spans', engine_id: observed.engine_id });
    if (!observed.has_universal_tools) violations.push({ kind: 'context_pack_missing_universal_tool_grants', engine_id: observed.engine_id });
    if (observed.row_count > 49 || observed.fragment_count > 48) {
      violations.push({
        kind: 'context_pack_not_bounded',
        engine_id: observed.engine_id,
        row_count: observed.row_count,
        fragment_count: observed.fragment_count,
      });
    }
  }
  for (const result of results) {
    if (result.payload_bytes > 65536) violations.push({ kind: 'turn_projection_exceeds_gateway_budget', engine_id: result.engine_id, payload_bytes: result.payload_bytes });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_context_switch_continuity_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_public_route_engine_switch_context_pack',
    engines_tested: ['codex_cli', 'claude_code', 'codex_cli'],
    results,
    observations,
    budget: {
      default_gateway_budget_bytes: 65536,
      max_payload_bytes: Math.max(0, ...results.map((row) => row.payload_bytes)),
    },
    policy: {
      shell_cognition_policy: 'Shell submits user turns only; Gateway-owned context packs carry continuity across runtimes.',
      engine_private_memory_policy: 'Continuity proof requires the later runtime to recover the key from InfRing context, not its own private session.',
      interchangeability_policy: 'Codex and Claude must receive the same bounded context shape across engine switches.',
    },
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}.json`), { force: true }); } catch {}
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_context_switch_continuity_guard',
    generated_at: new Date().toISOString(),
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['context_switch_continuity_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  try { fs.rmSync(path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}.json`), { force: true }); } catch {}
  process.exit(1);
});
