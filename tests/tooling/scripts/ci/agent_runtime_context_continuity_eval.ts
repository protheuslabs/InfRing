#!/usr/bin/env node
/* eslint-disable no-console */

// Deterministic engine-switch context continuity eval. This does not launch
// external providers; it proves that Gateway/router messages can carry the same
// Kernel-materialized context pack across native and external engine IDs, and
// that prompt/structured compatibility rendering exposes the remembered fact to
// every selected engine.

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const GRADUATION_BASELINE_CONTRACT_PATH = 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json';
const outPath = 'core/local/artifacts/agent_runtime_context_continuity_eval_current.json';
const continuityFact = 'continuity-key: brass-otter-713';
const expectedAnswer = 'brass-otter-713';

function cleanString(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function load(rel) {
  return require(path.join(ROOT, rel));
}

function readJson(rel) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
}

function write(payload) {
  fs.mkdirSync(path.dirname(path.join(ROOT, outPath)), { recursive: true });
  fs.writeFileSync(path.join(ROOT, outPath), `${JSON.stringify(payload, null, 2)}\n`);
}

function argValue(name) {
  const prefix = `${name}=`;
  const direct = process.argv.find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(name);
  if (idx >= 0 && process.argv[idx + 1]) return process.argv[idx + 1];
  return '';
}

function unique(values) {
  const out = [];
  const seen = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const item = cleanString(value, 160);
    if (!item || seen.has(item)) continue;
    seen.add(item);
    out.push(item);
  }
  return out;
}

function resolveEngineScope() {
  const registry = readJson(REGISTRY_PATH);
  const graduationContract = readJson(GRADUATION_BASELINE_CONTRACT_PATH);
  const registryEngines = unique((Array.isArray(registry.engines) ? registry.engines : []).map((row) => row && row.engine_id));
  const focus = registry.validation_focus_policy && typeof registry.validation_focus_policy === 'object'
    ? registry.validation_focus_policy
    : {};
  const secondaryEvalSamples = graduationContract.secondary_eval_samples && typeof graduationContract.secondary_eval_samples === 'object'
    ? graduationContract.secondary_eval_samples
    : {};
  const activePromotionEngines = unique(focus.active_promotion_engines);
  const contextContinuitySecondaryEngines = unique(secondaryEvalSamples.context_continuity);
  const requested = cleanString(argValue('--engines') || process.env.INFRING_AGENT_RUNTIME_CONTEXT_CONTINUITY_ENGINES || '', 2000);
  if (requested) {
    const normalized = requested.toLowerCase();
    if (['all', 'registry'].includes(normalized)) {
      return {
        engines: registryEngines,
        source: normalized,
        broad_registry_sample: true,
      };
    }
    if (['active', 'active_promotion_engines', 'golden', 'golden-pair'].includes(normalized)) {
      return {
        engines: activePromotionEngines.length ? activePromotionEngines : ['infring_native', 'codex_cli', 'claude_code'],
        source: normalized,
        broad_registry_sample: false,
      };
    }
    return {
      engines: unique(requested.split(',')),
      source: 'explicit_list',
      broad_registry_sample: false,
    };
  }
  return {
    engines: unique([
      ...(activePromotionEngines.length ? activePromotionEngines : ['infring_native', 'codex_cli', 'claude_code']),
      ...contextContinuitySecondaryEngines,
    ]),
    source: contextContinuitySecondaryEngines.length
      ? 'active_promotion_engines_plus_secondary_context_samples'
      : 'active_promotion_engines',
    broad_registry_sample: false,
  };
}

function baseEvent(ctx, type) {
  const message = (ctx && ctx.message) || {};
  const engine = (ctx && ctx.engine) || {};
  return {
    type,
    trace_id: cleanString(message.trace_id, 200),
    request_id: cleanString(message.request_id, 200),
    engine_id: cleanString(message.engine_id || engine.engine_id, 120),
    session_id: cleanString(message.session_id, 200),
    turn_id: cleanString(message.turn_id, 200),
  };
}

function collectTextFromPack(pack) {
  const fragments = Array.isArray(pack && pack.fragments) ? pack.fragments : [];
  const parts = [];
  for (const fragment of fragments) {
    const payload = fragment && fragment.payload && typeof fragment.payload === 'object' ? fragment.payload : {};
    parts.push(payload.text_preview, payload.summary, payload.source_ref);
  }
  return parts.map((row) => cleanString(row, 1200)).filter(Boolean).join('\n');
}

function makeContinuityProbeAdapter(engineId, buildPromptWithContext) {
  return {
    async health_check(ctx) {
      return {
        ...baseEvent(ctx, 'engine.health.result'),
        status: 'available',
        engine_kind: 'deterministic_context_probe',
        discovery_source: 'validation_fixture',
      };
    },
    async start_session(ctx) {
      return { ...baseEvent(ctx, 'session.started'), status: 'started' };
    },
    async submit_turn(ctx) {
      const message = (ctx && ctx.message) || {};
      const pack = message.context_pack || {};
      const current = message.input && (message.input.text || message.input.prompt || message.input.message) || '';
      const prompt = buildPromptWithContext(pack, current);
      const packText = collectTextFromPack(pack);
      const hasFactInPack = packText.includes(expectedAnswer);
      const hasFactInPrompt = prompt.includes(expectedAnswer);
      const hasUniversalTools = prompt.includes('Universal InfRing core tools') && prompt.includes('memory.read');
      return {
        ...baseEvent(ctx, 'turn.complete'),
        status: hasFactInPack && hasFactInPrompt && hasUniversalTools ? 'completed' : 'failed',
        output_preview: hasFactInPack && hasFactInPrompt && hasUniversalTools
          ? `${engineId} continuity ok: ${expectedAnswer}`
          : `${engineId} continuity missing: pack=${hasFactInPack} prompt=${hasFactInPrompt} tools=${hasUniversalTools}`,
        result_ref: `artifact/context-continuity/${engineId}`,
        receipt_ref: `receipt/context-continuity/${engineId}`,
        context_probe: {
          has_fact_in_pack: hasFactInPack,
          has_fact_in_prompt: hasFactInPrompt,
          has_universal_tools: hasUniversalTools,
          source_authority: cleanString(pack.source_authority, 160),
          source_basis: cleanString(pack.source_basis, 160),
          fragment_count: Array.isArray(pack.fragments) ? pack.fragments.length : 0,
        },
      };
    },
    async stream_events(ctx) {
      return { ...baseEvent(ctx, 'heartbeat'), status: 'adapter_ready' };
    },
    async cancel_turn(ctx) {
      return { ...baseEvent(ctx, 'turn.cancelled'), status: 'cancelled' };
    },
    async collect_artifacts(ctx) {
      return { ...baseEvent(ctx, 'artifact.created'), artifact_ref: `artifact/context-continuity/${engineId}` };
    },
    async emit_receipts(ctx) {
      return { ...baseEvent(ctx, 'receipt.created'), receipt_ref: `receipt/context-continuity/${engineId}` };
    },
  };
}

async function main() {
  const { createAgentRuntimeRouter } = load('gateway/runtime/agent_runtime/agent_runtime_router.ts');
  const { materializeKernelAgentRuntimeContextPack, resolveKernelMaterializerCommand } = load('gateway/runtime/agent_runtime/agent_runtime_kernel_context_bridge.ts');
  const { buildPromptWithContext } = load('adapters/runtime/agent_engines/cli_runtime_adapter.ts');
  const { buildUniversalToolGrants } = load('gateway/runtime/agent_runtime/universal_core_tools.ts');

  const commandSpec = resolveKernelMaterializerCommand(ROOT);
  const atoms = [
    {
      id: 'turn-001',
      role: 'user',
      source_kind: 'interaction_unit',
      source_ref: 'conversation/turn/001',
      text_preview: `The shared runtime memory fact is ${continuityFact}. Remember it across engine switches.`,
      token_count: 32,
    },
    {
      id: 'turn-002',
      role: 'assistant',
      source_kind: 'interaction_unit',
      source_ref: 'conversation/turn/002',
      text_preview: `Acknowledged. I will preserve ${continuityFact} as the engine-switch continuity key.`,
      token_count: 28,
    },
  ];

  const kernel = await materializeKernelAgentRuntimeContextPack({
    root: ROOT,
    sessionId: 'agent-runtime-context-continuity-eval',
    agentId: 'validation-agent',
    atoms,
    budgetTokens: 1600,
    timeoutMs: 30000,
  });

  const violations = [];
  if (!commandSpec) violations.push({ kind: 'kernel_materializer_command_unavailable' });
  if (!kernel.ok || !kernel.context_pack) violations.push({ kind: 'kernel_context_pack_materialization_failed', reason: kernel.reason || null });

  const contextPack = kernel.context_pack || {};
  contextPack.universal_tool_grants = buildUniversalToolGrants({
    traceId: 'trace-context-continuity',
    sessionId: 'agent-runtime-context-continuity-eval',
    agentId: 'validation-agent',
    engineId: 'runtime-switch',
  });
  const packText = collectTextFromPack(contextPack);
  if (!packText.includes(expectedAnswer)) {
    violations.push({ kind: 'kernel_context_pack_missing_continuity_fact', source_authority: contextPack.source_authority || null });
  }
  const promptPreview = buildPromptWithContext(contextPack, 'What is the shared runtime memory fact?');
  if (!promptPreview.includes(expectedAnswer)) violations.push({ kind: 'adapter_prompt_missing_continuity_fact' });
  if (!promptPreview.includes('Universal InfRing core tools') || !promptPreview.includes('memory.read')) violations.push({ kind: 'adapter_prompt_missing_universal_tool_grants' });

  const router = createAgentRuntimeRouter({ root: ROOT, disableTraceWriter: true });
  const engineScope = resolveEngineScope();
  const engines = engineScope.engines;
  if (!engines.length) violations.push({ kind: 'engine_scope_empty', source: engineScope.source });
  const results = [];
  for (const engineId of engines) {
    router.registerAdapter(engineId, makeContinuityProbeAdapter(engineId, buildPromptWithContext));
    const base = {
      type: 'turn.submit',
      trace_id: `trace-context-continuity-${engineId}`,
      request_id: `request-context-continuity-${engineId}`,
      engine_id: engineId,
      session_id: 'agent-runtime-context-continuity-eval',
      turn_id: `turn-context-continuity-${engineId}`,
      input: { text: 'What is the shared runtime memory fact?' },
      context_pack: contextPack,
      scope: { workspace: 'none', mutation_allowed: false, tools_allowed: false },
      capability_budget: { tool_calls: 0, file_reads: 0, file_writes: 0, command_runs: 0 },
    };
    const turn = await router.submitTurn(base);
    const ok = turn.type === 'turn.complete' && turn.status === 'completed' && cleanString(turn.output_preview, 1000).includes(expectedAnswer);
    if (!ok) violations.push({ kind: 'engine_context_continuity_failed', engine_id: engineId, output_preview: turn.output_preview || null });
    results.push({
      engine_id: engineId,
      ok,
      turn_type: turn.type,
      turn_status: turn.status,
      output_preview: cleanString(turn.output_preview, 1000),
      context_probe: turn.context_probe || null,
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'agent_runtime_context_continuity_eval',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_no_provider_dispatch',
    kernel_materializer_mode: commandSpec ? commandSpec.mode : 'unavailable',
    kernel_materializer_auto: commandSpec ? commandSpec.auto === true : false,
    expected_answer: expectedAnswer,
    context_source_authority: contextPack.source_authority || null,
    context_fragment_count: Array.isArray(contextPack.fragments) ? contextPack.fragments.length : 0,
    engine_scope_source: engineScope.source,
    broad_registry_sample: engineScope.broad_registry_sample,
    engines_tested: engines,
    results,
    violations,
  };
  write(payload);
  console.log(JSON.stringify(payload, null, 2));
  if (!payload.ok) process.exit(1);
}

main().catch((err) => {
  const payload = {
    ok: false,
    type: 'agent_runtime_context_continuity_eval',
    error: cleanString(err && err.stack ? err.stack : err, 6000),
  };
  write(payload);
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
});
