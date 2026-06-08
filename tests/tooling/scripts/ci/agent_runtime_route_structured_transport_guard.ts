#!/usr/bin/env node
/* eslint-disable no-console */

// Proves public Agent Runtime turn routes deliver a Gateway-owned structured
// turn envelope to every runtime engine. Prompt text compatibility may exist,
// but it must derive from the structured turn and remain explicitly marked as
// transitional.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_route_structured_transport_guard_current.json');
const SCRATCH_DIR = path.join(ROOT, 'core/local/artifacts/agent-runtime-route-structured-transport-scratch');
const SESSION_ID = 'agent-runtime-route-structured-transport-session';
const AGENT_ID = 'agent-runtime-route-structured-transport-agent';
const ENGINES = ['infring_native', 'codex_cli', 'claude_code', 'grok_code', 'opencode', 'openclaw', 'hermes_agent'];
const INSTALLABLE_ENGINES = ['codex_cli', 'claude_code', 'grok_code', 'opencode', 'openclaw', 'hermes_agent'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  return { statusCode: 0, payload: null };
}

function contextStatePath(engineId) {
  return path.join(ROOT, 'core/local/state/agent_runtime/context', `${SESSION_ID}-${engineId}.json`);
}

function hasNoSecrets(value) {
  const text = JSON.stringify(value || {});
  return !/(api[_-]?key|secret|token)["']?\s*[:=]\s*["'][A-Za-z0-9_\-.]{12,}/i.test(text);
}

function createCaptureAdapter(engineId, captures) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'available',
      discovery_source: 'route_structured_transport_guard',
    }),
    start_session: async ({ message }) => ({
      type: 'session.started',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'started',
    }),
    submit_turn: async ({ message }) => {
      captures[engineId] = message;
      return {
        type: 'turn.complete',
        trace_id: message && message.trace_id,
        engine_id: engineId,
        session_id: message && message.session_id,
        turn_id: message && message.turn_id,
        status: 'completed',
        output_text: `${engineId} structured transport accepted.`,
        output_preview: `${engineId} structured transport accepted.`,
        result_ref: `artifact/route-structured/${engineId}`,
        receipt_ref: `receipt/route-structured/${engineId}`,
        activity_events: [
          {
            type: 'agent_activity_event',
            activity_kind: 'activity',
            provider_event_type: 'structured_transport.accepted',
            status: 'completed',
            display_text: `${engineId} received a structured Gateway turn.`,
          },
        ],
        structured_activity: true,
        activity_event_count: 1,
      };
    },
    stream_events: async ({ message }) => ({ type: 'heartbeat', trace_id: message && message.trace_id, engine_id: engineId, status: 'ok' }),
    cancel_turn: async ({ message }) => ({ type: 'turn.cancelled', trace_id: message && message.trace_id, engine_id: engineId, status: 'cancelled' }),
    collect_artifacts: async ({ message }) => ({ type: 'artifact.list', trace_id: message && message.trace_id, engine_id: engineId, artifacts: [] }),
    emit_receipts: async ({ message }) => ({ type: 'receipt.created', trace_id: message && message.trace_id, engine_id: engineId, receipt_refs: [] }),
  };
}

function createUnavailableInstallAdapter(engineId) {
  return {
    health_check: async ({ message }) => ({
      type: 'engine.health.result',
      trace_id: message && message.trace_id,
      engine_id: engineId,
      status: 'not_downloaded',
      download_available: true,
      download_action_ref: `agent_runtime_download/${engineId}`,
      discovery_source: 'route_install_permission_guard',
      reason: `${engineId} intentionally unavailable in install permission guard.`,
    }),
  };
}

async function submitTurn(assembly, engineId, index) {
  const res = makeResponse();
  const sessionId = `${SESSION_ID}-${engineId}`;
  const handled = await assembly.handleAgentRuntimeTurnRoute({
    req: {
      method: 'POST',
      __body: {
        agent_id: AGENT_ID,
        session_id: sessionId,
        engine_id: engineId,
        turn_id: `route-structured-turn-${index}-${engineId}`,
        message: `Check structured transport for ${engineId}.`,
        input_text: `Check structured transport for ${engineId}.`,
        working_directory: ROOT,
        model_provider_context: {
          provider: engineId === 'claude_code' ? 'anthropic' : engineId === 'grok_code' ? 'xai' : engineId === 'opencode' ? 'openrouter' : 'openai',
          model: engineId === 'claude_code' ? 'claude-sonnet-4-6' : engineId === 'grok_code' ? 'grok-code-fast-1' : engineId === 'opencode' ? 'openrouter/auto' : 'gpt-5.5',
          qualified_model_ref: engineId === 'claude_code' ? 'anthropic/claude-sonnet-4-6' : engineId === 'grok_code' ? 'xai/grok-code-fast-1' : engineId === 'opencode' ? 'openrouter/openrouter/auto' : 'openai/gpt-5.5',
          secrets_included: false,
        },
      },
    },
    res,
    pathname: '/api/agent-runtime/turn',
    traceId: `validation:agent-runtime-route-structured:${engineId}:${Date.now()}`,
    flags: {},
  });
  return {
    engine_id: engineId,
    handled,
    status_code: res.statusCode,
    status: clean(res.payload && res.payload.status, 120),
    ok: !!(handled && res.statusCode === 200 && res.payload && res.payload.status === 'completed'),
  };
}

async function submitInstallBlocked(assembly, engineId, index) {
  const res = makeResponse();
  const handled = await assembly.handleAgentRuntimeEngineRoute({
    req: {
      method: 'POST',
      __body: {
        reason: 'validation guard must not execute installers without approval',
      },
    },
    res,
    pathname: `/api/shell-socket/agent-runtime/engines/${encodeURIComponent(engineId)}/install`,
    traceId: `validation:agent-runtime-route-install:${engineId}:${Date.now()}`,
    flags: {},
  });
  return {
    engine_id: engineId,
    handled,
    status_code: res.statusCode,
    status: clean(res.payload && res.payload.status, 120),
    ok: !!(handled && res.statusCode === 403 && res.payload && res.payload.status === 'permission_required'),
    stdout_preview_present: !!(res.payload && res.payload.stdout_preview),
    stderr_preview_present: !!(res.payload && res.payload.stderr_preview),
    command_line_hint_present: !!(res.payload && res.payload.command_line_hint),
    type: clean(res.payload && res.payload.type, 120),
  };
}

async function main() {
  try { fs.rmSync(SCRATCH_DIR, { recursive: true, force: true }); } catch {}
  for (const engineId of ENGINES) {
    try { fs.rmSync(contextStatePath(engineId), { force: true }); } catch {}
  }

  const captures = {};
  const { createGatewayAgentRuntimeRouteAssembly } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_route_assembly.ts'));
  const adapterFactories = {};
  for (const engineId of ENGINES) adapterFactories[engineId] = () => createCaptureAdapter(engineId, captures);
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

  const turnResults = [];
  for (let index = 0; index < ENGINES.length; index += 1) {
    turnResults.push(await submitTurn(assembly, ENGINES[index], index + 1));
  }

  const previousInstallAllowed = process.env.INFRING_AGENT_RUNTIME_INSTALL_ALLOWED;
  process.env.INFRING_AGENT_RUNTIME_INSTALL_ALLOWED = '0';
  const installAdapterFactories = {};
  for (const engineId of INSTALLABLE_ENGINES) installAdapterFactories[engineId] = () => createUnavailableInstallAdapter(engineId);
  const installAssembly = createGatewayAgentRuntimeRouteAssembly({
    root: ROOT,
    statusDir: path.join(SCRATCH_DIR, 'install-state'),
    adapterFactories: installAdapterFactories,
    readJsonBody: async (req) => (req && req.__body) || {},
    sendJson: (res, statusCode, payload) => {
      res.statusCode = statusCode;
      res.payload = payload;
    },
    fetchBackendJson: async () => ({}),
    createNativeOrchestrationClient: () => ({}),
  });
  const installResults = [];
  for (let index = 0; index < INSTALLABLE_ENGINES.length; index += 1) {
    installResults.push(await submitInstallBlocked(installAssembly, INSTALLABLE_ENGINES[index], index + 1));
  }
  if (previousInstallAllowed == null) delete process.env.INFRING_AGENT_RUNTIME_INSTALL_ALLOWED;
  else process.env.INFRING_AGENT_RUNTIME_INSTALL_ALLOWED = previousInstallAllowed;

  const violations = [];
  if (!turnResults.every((row) => row.ok)) {
    violations.push({ kind: 'public_route_structured_turn_submission_failed', failed: turnResults.filter((row) => !row.ok) });
  }
  if (!installResults.every((row) => row.ok)) {
    violations.push({ kind: 'install_route_permission_gate_failed', failed: installResults.filter((row) => !row.ok) });
  }
  for (const row of installResults) {
    if (row.stdout_preview_present || row.stderr_preview_present) {
      violations.push({ kind: 'install_route_executed_or_leaked_process_output_while_permission_denied', engine_id: row.engine_id });
    }
    if (!row.command_line_hint_present) {
      violations.push({ kind: 'install_route_permission_denied_missing_command_line_hint', engine_id: row.engine_id });
    }
  }
  const captureSummaries = [];
  for (const engineId of ENGINES) {
    const message = captures[engineId] || {};
    const structuredTurn = message.structured_turn && typeof message.structured_turn === 'object' ? message.structured_turn : null;
    const contextPack = message.context_pack && typeof message.context_pack === 'object' ? message.context_pack : {};
    const structuredProjection = contextPack.structured_transport && typeof contextPack.structured_transport === 'object' ? contextPack.structured_transport : {};
    const summary = {
      engine_id: engineId,
      structured_turn_type: structuredTurn && structuredTurn.type,
      structured_turn_ref: structuredTurn && structuredTurn.structured_turn_ref,
      transport_mode: structuredTurn && structuredTurn.transport_mode,
      transport_target: structuredTurn && structuredTurn.transport_target,
      prompt_text_compat_allowed: !!(structuredTurn && structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.allowed),
      prompt_text_compat_rule: clean(structuredTurn && structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.rule, 240),
      context_projection_ref: structuredProjection.structured_turn_ref || '',
      secrets_included: structuredTurn ? !hasNoSecrets(structuredTurn) : true,
      model_provider_secrets_included: !!(structuredTurn && structuredTurn.model_provider_context && structuredTurn.model_provider_context.secrets_included),
    };
    captureSummaries.push(summary);
    if (!structuredTurn || structuredTurn.type !== 'AgentRuntimeStructuredTurn') violations.push({ kind: 'structured_turn_missing', engine_id: engineId });
    if (structuredTurn && structuredTurn.source_authority !== 'gateway.runtime.agent_runtime_structured_transport') violations.push({ kind: 'structured_turn_wrong_authority', engine_id: engineId });
    if (structuredTurn && structuredTurn.transport_mode !== 'structured_json') violations.push({ kind: 'structured_turn_wrong_transport_mode', engine_id: engineId, transport_mode: structuredTurn.transport_mode });
    if (structuredTurn && structuredTurn.transport_target !== 'structured_json') violations.push({ kind: 'structured_turn_wrong_transport_target', engine_id: engineId, transport_target: structuredTurn.transport_target });
    if (structuredTurn && (!structuredTurn.prompt_text_compatibility || structuredTurn.prompt_text_compatibility.allowed !== true)) {
      violations.push({ kind: 'prompt_text_compatibility_not_explicitly_transitional', engine_id: engineId });
    }
    if (structuredTurn && !/transitional/i.test(clean(structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.rule, 500))) {
      violations.push({ kind: 'prompt_text_compatibility_rule_not_transitional', engine_id: engineId });
    }
    if (!structuredProjection || structuredProjection.structured_turn_ref !== (structuredTurn && structuredTurn.structured_turn_ref)) {
      violations.push({ kind: 'context_pack_structured_turn_ref_missing_or_mismatched', engine_id: engineId });
    }
    if (!hasNoSecrets(structuredTurn)) violations.push({ kind: 'structured_turn_secret_material_leaked', engine_id: engineId });
    if (structuredTurn && structuredTurn.model_provider_context && structuredTurn.model_provider_context.secrets_included !== false) {
      violations.push({ kind: 'structured_turn_model_provider_secret_flag_wrong', engine_id: engineId });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_route_structured_transport_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_public_gateway_route_matrix',
    engines_tested: ENGINES,
    install_engines_tested: INSTALLABLE_ENGINES,
    turn_results: turnResults,
    install_permission_results: installResults,
    captures: captureSummaries,
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
    type: 'agent_runtime_route_structured_transport_guard',
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
