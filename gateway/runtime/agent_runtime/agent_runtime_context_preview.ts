#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::context-preview.
//
// Gateway owns bounded Agent Runtime context-pack preview projection. The legacy
// dashboard host may still expose the HTTP route while it is being retired, but
// context preview shaping and Kernel/fallback materializer selection belong to
// the Gateway membrane.

'use strict';

const {
  resolveAgentRuntimeEngineId,
} = require('./agent_runtime_engine_identity.ts');

function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }

function defaultContextPack(input = {}) {
  return {
    source_basis: 'gateway_runtime_default_context_pack',
    source_authority: 'gateway.runtime.agent_runtime_context_preview',
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
      tools: [],
      source_authority: 'gateway.runtime.agent_runtime_context_preview',
    },
    trace_id: cleanText(input.traceId, 200),
    agent_id: cleanText(input.agentId, 160),
    session_id: cleanText(input.sessionId, 200),
  };
}

function normalizeContextPack(pack, fallback) {
  const contextPack = pack && typeof pack === 'object' ? pack : fallback;
  if (!Array.isArray(contextPack.fragments)) contextPack.fragments = [];
  if (!contextPack.frontier || typeof contextPack.frontier !== 'object') contextPack.frontier = {};
  for (const key of ['hot_atom_refs', 'warm_span_refs', 'cool_span_refs', 'cold_span_refs']) {
    if (!Array.isArray(contextPack.frontier[key])) contextPack.frontier[key] = [];
  }
  if (!contextPack.universal_tool_grants || typeof contextPack.universal_tool_grants !== 'object') {
    contextPack.universal_tool_grants = { tools: [], source_authority: 'gateway.runtime.agent_runtime_context_preview' };
  }
  if (!Array.isArray(contextPack.universal_tool_grants.tools)) contextPack.universal_tool_grants.tools = [];
  return contextPack;
}

function projectContextFragment(fragment) {
  const payload = fragment && fragment.payload && typeof fragment.payload === 'object' ? fragment.payload : {};
  return {
    fragment_id: cleanText(fragment && fragment.fragment_id, 200),
    kind: cleanText(fragment && fragment.kind, 40),
    ref_id: cleanText(fragment && fragment.ref_id, 200),
    level: Number(fragment && fragment.level) || 0,
    source_kind: cleanText(payload.source_kind || payload.record_type, 120),
    speaker_label: cleanText(payload.speaker_label || payload.role, 120),
    role: cleanText(payload.role, 40),
    source_ref: cleanText(payload.source_ref, 240),
    summary: cleanDisplayText(payload.text_preview || payload.summary || '', 800),
  };
}

function projectFallbackContextRow(row) {
  const source = row && typeof row === 'object' ? row : {};
  const sourceRef = cleanText(source.source_ref || source.id, 240);
  return {
    fragment_id: cleanText(source.id || `gateway_fallback_${sourceRef}`, 200),
    kind: 'atom',
    ref_id: cleanText(source.id || sourceRef, 200),
    level: 0,
    source_kind: cleanText(source.source_kind || source.record_type, 120),
    speaker_label: cleanText(source.speaker_label || source.role, 120),
    role: cleanText(source.role, 40),
    source_ref: sourceRef,
    summary: cleanDisplayText(source.text_preview || source.summary || source.content_preview || '', 800),
    source_authority: cleanText(source.source_authority, 160),
    gateway_fallback_overlay: true,
  };
}

function mergeFreshGatewayContextRows(projectedFragments, fallbackRows) {
  const out = Array.isArray(projectedFragments) ? projectedFragments.slice(-24) : [];
  const seen = new Set(out.flatMap((row) => [
    cleanText(row && row.source_ref, 240),
    cleanText(row && row.ref_id, 200),
    cleanDisplayText(row && row.summary, 800),
  ]).filter(Boolean));
  for (const row of (Array.isArray(fallbackRows) ? fallbackRows : []).slice(-12)) {
    const kind = cleanText(row && (row.source_kind || row.record_type), 120);
    const authority = cleanText(row && row.source_authority, 160);
    const shouldOverlay = kind === 'tool_receipt' || authority === 'gateway_agent_runtime_approval_effect_context';
    if (!shouldOverlay) continue;
    const projected = projectFallbackContextRow(row);
    const key = cleanText(projected.source_ref || projected.ref_id || projected.summary, 240);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    out.push(projected);
  }
  return out.slice(-24);
}

function createAgentRuntimeContextPreviewProjectionStore(deps = {}) {
  const ROOT = deps.root || process.cwd();
  const loadAgentRuntimeContextRows = deps.loadAgentRuntimeContextRows || (() => []);
  const materializeKernelAgentRuntimeContextPack = deps.materializeKernelAgentRuntimeContextPack || (async () => ({ ok: false, reason: 'kernel_materializer_unavailable' }));
  const materializeAgentRuntimeContextPack = deps.materializeAgentRuntimeContextPack || (() => defaultContextPack({}));
  const buildAgentRuntimeContextPack = deps.buildAgentRuntimeContextPack || defaultContextPack;
  const buildUniversalToolGrants = deps.buildUniversalToolGrants || (() => ({ tools: [], source_authority: 'gateway.runtime.agent_runtime_context_preview' }));
  const loadAgentRuntimeSelection = typeof deps.loadAgentRuntimeSelection === 'function'
    ? deps.loadAgentRuntimeSelection
    : null;

  async function agentRuntimeContextPackPreviewProjection(traceId, body) {
    const engineId = resolveAgentRuntimeEngineId(body, {
      loadSelection: loadAgentRuntimeSelection,
      defaultEngineId: 'infring_native',
    });
    const agentId = cleanText(body && body.agent_id, 160) || 'default';
    const sessionId = cleanText(body && body.session_id, 200) || `shell_${agentId}`;
    const fallbackContextRows = loadAgentRuntimeContextRows({ root: ROOT, sessionId, agentId });
    const kernelContext = await materializeKernelAgentRuntimeContextPack({
      root: ROOT,
      sessionId,
      agentId,
      traceId,
      atoms: fallbackContextRows,
      timeoutMs: 8000,
    }).catch((error) => ({
      ok: false,
      reason: cleanText(error && error.message ? error.message : error, 240),
    }));
    const contextPack = kernelContext && kernelContext.ok && kernelContext.context_pack
      ? kernelContext.context_pack
      : await Promise.resolve(materializeAgentRuntimeContextPack({
        root: ROOT,
        sessionId,
        agentId,
        traceId,
      })).catch(() => buildAgentRuntimeContextPack({ body, agentId, sessionId, traceId }));
    contextPack.universal_tool_grants = buildUniversalToolGrants({
      traceId,
      sessionId,
      agentId,
      engineId,
      permissionPolicy: body && body.permission_policy,
    });
    const fragments = Array.isArray(contextPack.fragments) ? contextPack.fragments : [];
    const projectedFragments = mergeFreshGatewayContextRows(
      fragments.slice(-24).map(projectContextFragment),
      fallbackContextRows,
    );
    return {
      ok: true,
      type: 'agent_runtime_context_pack_preview',
      trace_id: traceId,
      engine_id: engineId,
      agent_id: agentId,
      session_id: sessionId,
      source_basis: cleanText(contextPack.source_basis, 160),
      source_authority: cleanText(contextPack.source_authority, 200),
      row_count: Number(contextPack.row_count) || 0,
      raw_row_count: Number(contextPack.raw_row_count) || Number(contextPack.row_count) || 0,
      dedupe_policy: contextPack.dedupe_policy || null,
      kernel_materializer_used: !!(kernelContext && kernelContext.ok),
      kernel_materializer_mode: cleanText(kernelContext && kernelContext.command_mode, 40),
      frontier: {
        hot_atom_count: Array.isArray(contextPack.frontier && contextPack.frontier.hot_atom_refs) ? contextPack.frontier.hot_atom_refs.length : 0,
        warm_span_count: Array.isArray(contextPack.frontier && contextPack.frontier.warm_span_refs) ? contextPack.frontier.warm_span_refs.length : 0,
        cool_span_count: Array.isArray(contextPack.frontier && contextPack.frontier.cool_span_refs) ? contextPack.frontier.cool_span_refs.length : 0,
        cold_span_count: Array.isArray(contextPack.frontier && contextPack.frontier.cold_span_refs) ? contextPack.frontier.cold_span_refs.length : 0,
        pressure_state: cleanText(contextPack.frontier && contextPack.frontier.pressure_state, 80),
      },
      fragments: projectedFragments,
      universal_tool_count: Array.isArray(contextPack.universal_tool_grants && contextPack.universal_tool_grants.tools)
        ? contextPack.universal_tool_grants.tools.length
        : 0,
    };
  }

  async function safeAgentRuntimeContextPackPreviewProjection(traceId, body) {
    const projection = await agentRuntimeContextPackPreviewProjection(traceId, body);
    if (projection && projection.context_pack && typeof projection.context_pack === 'object') {
      projection.context_pack = normalizeContextPack(projection.context_pack, defaultContextPack({ traceId }));
    }
    return projection;
  }

  return {
    agentRuntimeContextPackPreviewProjection: safeAgentRuntimeContextPackPreviewProjection,
  };
}

module.exports = {
  createAgentRuntimeContextPreviewProjectionStore,
};
