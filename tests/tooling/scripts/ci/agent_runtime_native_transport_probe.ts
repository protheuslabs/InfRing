#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const OUT_JSON = 'core/local/artifacts/agent_runtime_native_transport_probe_current.json';
const CODEX_APP_SERVER_MAPPING_PROBE = 'core/local/artifacts/agent_runtime_codex_app_server_mapping_probe_current.json';
const CODEX_APP_SERVER_LIVE_ACCEPTANCE_PROBE = 'core/local/artifacts/agent_runtime_codex_app_server_live_acceptance_probe_current.json';
const CLAUDE_STREAM_JSON_MAPPING_PROBE = 'core/local/artifacts/agent_runtime_claude_stream_json_mapping_probe_current.json';
const CLAUDE_STREAM_JSON_LIVE_ACCEPTANCE_PROBE = 'core/local/artifacts/agent_runtime_claude_stream_json_live_acceptance_probe_current.json';
const OPENCODE_SDK_MAPPING_PROBE = 'core/local/artifacts/agent_runtime_opencode_sdk_mapping_probe_current.json';
const OPENCODE_SDK_LIVE_ACCEPTANCE_PROBE = 'core/local/artifacts/agent_runtime_opencode_sdk_live_acceptance_probe_current.json';
const HERMES_AGENT_MAPPING_PROBE = 'core/local/artifacts/agent_runtime_hermes_agent_mapping_probe_current.json';
const HERMES_AGENT_LIVE_ACCEPTANCE_PROBE = 'core/local/artifacts/agent_runtime_hermes_agent_live_acceptance_probe_current.json';
const GROK_CODE_MAPPING_PROBE = 'core/local/artifacts/agent_runtime_grok_code_mapping_probe_current.json';
const GROK_CODE_LIVE_ACCEPTANCE_PROBE = 'core/local/artifacts/agent_runtime_grok_code_live_acceptance_probe_current.json';
const BOUNDED_ENVELOPE_MODES = new Set([
  'bounded_stdin_context_envelope',
  'bounded_cli_arg_context_envelope',
]);
const REVIEW_WARNING_WINDOW_DAYS = Number.parseInt(
  process.env.AGENT_RUNTIME_BOUNDED_ENVELOPE_REVIEW_WARNING_DAYS || '45',
  10,
);
const GOLDEN_PAIR_ENGINES = new Set(['codex_cli', 'claude_code']);

function clean(value: any, max = 500): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function parseDate(value: any): Date | null {
  const text = clean(value, 40);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) return null;
  const date = new Date(`${text}T00:00:00.000Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function daysUntil(value: any): number | null {
  const date = parseDate(value);
  if (!date) return null;
  const now = new Date();
  const todayMs = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  return Math.ceil((date.getTime() - todayMs) / 86400000);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function readJson(rel: string): JsonObject {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
  } catch {
    return {};
  }
}

function mappingProbeForEngine(engineId: string): JsonObject | null {
  if (engineId === 'codex_cli') {
    const probe = readJson(CODEX_APP_SERVER_MAPPING_PROBE);
    return probe && probe.type === 'agent_runtime_codex_app_server_mapping_probe' ? probe : null;
  }
  if (engineId === 'claude_code') {
    const probe = readJson(CLAUDE_STREAM_JSON_MAPPING_PROBE);
    return probe && probe.type === 'agent_runtime_claude_stream_json_mapping_probe' ? probe : null;
  }
  if (engineId === 'opencode') {
    const probe = readJson(OPENCODE_SDK_MAPPING_PROBE);
    return probe && probe.type === 'agent_runtime_opencode_sdk_mapping_probe' ? probe : null;
  }
  if (engineId === 'hermes_agent') {
    const probe = readJson(HERMES_AGENT_MAPPING_PROBE);
    return probe && probe.type === 'agent_runtime_hermes_agent_mapping_probe' ? probe : null;
  }
  if (engineId === 'grok_code') {
    const probe = readJson(GROK_CODE_MAPPING_PROBE);
    return probe && probe.type === 'agent_runtime_grok_code_mapping_probe' ? probe : null;
  }
  return null;
}

function liveAcceptanceProbeForEngine(engineId: string): JsonObject | null {
  if (engineId === 'codex_cli') {
    const probe = readJson(CODEX_APP_SERVER_LIVE_ACCEPTANCE_PROBE);
    return probe && probe.type === 'agent_runtime_codex_app_server_live_acceptance_probe' ? probe : null;
  }
  if (engineId === 'claude_code') {
    const probe = readJson(CLAUDE_STREAM_JSON_LIVE_ACCEPTANCE_PROBE);
    return probe && probe.type === 'agent_runtime_claude_stream_json_live_acceptance_probe' ? probe : null;
  }
  if (engineId === 'opencode') {
    const probe = readJson(OPENCODE_SDK_LIVE_ACCEPTANCE_PROBE);
    return probe && probe.type === 'agent_runtime_opencode_sdk_live_acceptance_probe' ? probe : null;
  }
  if (engineId === 'hermes_agent') {
    const probe = readJson(HERMES_AGENT_LIVE_ACCEPTANCE_PROBE);
    return probe && probe.type === 'agent_runtime_hermes_agent_live_acceptance_probe' ? probe : null;
  }
  if (engineId === 'grok_code') {
    const probe = readJson(GROK_CODE_LIVE_ACCEPTANCE_PROBE);
    return probe && probe.type === 'agent_runtime_grok_code_live_acceptance_probe' ? probe : null;
  }
  return null;
}

function mappingProbeRefForEngine(engineId: string): string {
  if (engineId === 'codex_cli') return CODEX_APP_SERVER_MAPPING_PROBE;
  if (engineId === 'claude_code') return CLAUDE_STREAM_JSON_MAPPING_PROBE;
  if (engineId === 'opencode') return OPENCODE_SDK_MAPPING_PROBE;
  if (engineId === 'hermes_agent') return HERMES_AGENT_MAPPING_PROBE;
  if (engineId === 'grok_code') return GROK_CODE_MAPPING_PROBE;
  return '';
}

function liveAcceptanceProbeRefForEngine(engineId: string): string {
  if (engineId === 'codex_cli') return CODEX_APP_SERVER_LIVE_ACCEPTANCE_PROBE;
  if (engineId === 'claude_code') return CLAUDE_STREAM_JSON_LIVE_ACCEPTANCE_PROBE;
  if (engineId === 'opencode') return OPENCODE_SDK_LIVE_ACCEPTANCE_PROBE;
  if (engineId === 'hermes_agent') return HERMES_AGENT_LIVE_ACCEPTANCE_PROBE;
  if (engineId === 'grok_code') return GROK_CODE_LIVE_ACCEPTANCE_PROBE;
  return '';
}

function liveAcceptanceSourceAuthorityForEngine(engineId: string): string {
  if (engineId === 'codex_cli') return 'validation.agent_runtime_codex_app_server_live_acceptance_probe';
  if (engineId === 'claude_code') return 'validation.agent_runtime_claude_stream_json_live_acceptance_probe';
  if (engineId === 'opencode') return 'validation.agent_runtime_opencode_sdk_live_acceptance_probe';
  if (engineId === 'hermes_agent') return 'validation.agent_runtime_hermes_agent_live_acceptance_probe';
  if (engineId === 'grok_code') return 'validation.agent_runtime_grok_code_live_acceptance_probe';
  return 'validation.agent_runtime_native_transport_probe';
}

function probeRow(engine: JsonObject): JsonObject {
  const engineId = clean(engine.engine_id, 120);
  const contextMode = clean(engine.context_transport_mode, 120);
  const target = clean(engine.structured_transport_target, 120);
  const envelope = engine.structured_source_envelope && typeof engine.structured_source_envelope === 'object'
    ? engine.structured_source_envelope
    : {};
  const declaredProbe = envelope.native_transport_probe && typeof envelope.native_transport_probe === 'object'
    ? envelope.native_transport_probe
    : {};
  const declaredStatus = clean(declaredProbe.status, 120);
  const typedTurnApiAvailable = declaredStatus === 'passed' ||
    declaredStatus === 'available' ||
    declaredStatus === 'native_transport_available' ||
    declaredProbe.typed_turn_api_available === true;
  const declaredEvidence = clean(declaredProbe.evidence, 1000);
  const declaredNextAction = clean(declaredProbe.next_action || declaredProbe.next_step, 1000);
  const reviewBy = clean(envelope.review_by, 40);
  const reviewDaysRemaining = daysUntil(reviewBy);
  const isGoldenPair = GOLDEN_PAIR_ENGINES.has(engineId);
  const mappingProbe = mappingProbeForEngine(engineId);
  const mappingReady = !!(mappingProbe && mappingProbe.ok === true && mappingProbe.native_transport_probe_status === 'candidate_mapping_ready');
  const liveAcceptanceProbe = liveAcceptanceProbeForEngine(engineId);
  const liveAccepted = !!(liveAcceptanceProbe && liveAcceptanceProbe.ok === true && liveAcceptanceProbe.typed_turn_api_available === true);
  const applies = BOUNDED_ENVELOPE_MODES.has(contextMode);
  const status = !applies
    ? 'not_applicable'
    : typedTurnApiAvailable || liveAccepted
      ? 'native_transport_available'
      : mappingReady
        ? 'candidate_mapping_ready'
      : declaredStatus
        ? declaredStatus
        : 'pending';
  return {
    engine_id: engineId,
    applies,
    context_transport_mode: contextMode,
    structured_transport_target: target,
    probe_status: status,
    typed_turn_api_available: !!(typedTurnApiAvailable || liveAccepted),
    promotion_scope: isGoldenPair ? 'golden_pair_runtime' : 'secondary_runtime',
    review_by: reviewBy,
    review_days_remaining: reviewDaysRemaining,
    warning_window_days: REVIEW_WARNING_WINDOW_DAYS,
    source_authority: liveAccepted
      ? liveAcceptanceSourceAuthorityForEngine(engineId)
      : declaredStatus
      ? 'validation.conformance.agent_runtime_engine_registry.structured_source_envelope.native_transport_probe'
      : 'validation.agent_runtime_native_transport_probe',
    evidence: liveAccepted
      ? clean(liveAcceptanceProbe.evidence, 1000)
      : typedTurnApiAvailable
      ? 'Registry declares an upstream native typed turn/session/context API is available.'
      : mappingReady
        ? clean(mappingProbe.evidence, 1000)
      : declaredEvidence
        ? declaredEvidence
      : applies
        ? 'No upstream native typed turn/session/context API has been recorded; bounded CLI/stdin envelope remains compatibility transport.'
        : 'Engine does not use a bounded CLI/stdin envelope.',
    candidate_mapping_probe_ref: mappingReady
      ? mappingProbeRefForEngine(engineId)
      : '',
    live_acceptance_probe_ref: liveAcceptanceProbe ? liveAcceptanceProbeRefForEngine(engineId) : '',
    next_action: applies && !(typedTurnApiAvailable || liveAccepted)
      ? liveAcceptanceProbe && liveAcceptanceProbe.next_action
        ? clean(liveAcceptanceProbe.next_action, 1000)
        : mappingReady
        ? clean(mappingProbe.next_action, 1000)
        : declaredNextAction || 'Probe upstream runtime documentation or local runtime capabilities for a stable typed turn/session/context API that can accept AgentRuntimeStructuredTurn without bounded prompt rendering.'
      : '',
  };
}

function main() {
  const registry = readJson(REGISTRY_PATH);
  const engines = Array.isArray(registry.engines) ? registry.engines : [];
  const rows = engines.map(probeRow);
  const applicableRows = rows.filter((row) => row.applies);
  const pendingRows = applicableRows.filter((row) => row.typed_turn_api_available !== true);
  const rankedNextActions = pendingRows.map((row) => ({
    engine_id: row.engine_id,
    priority: row.promotion_scope === 'golden_pair_runtime' ? 'red' : 'yellow',
    promotion_scope: row.promotion_scope,
    review_by: row.review_by,
    days_remaining: row.review_days_remaining,
    next_action: row.next_action,
    source_path: REGISTRY_PATH,
  })).sort((a, b) => {
    const priorityOrder: Record<string, number> = { red: 0, yellow: 1 };
    const pa = priorityOrder[a.priority] ?? 9;
    const pb = priorityOrder[b.priority] ?? 9;
    if (pa !== pb) return pa - pb;
    const da = typeof a.days_remaining === 'number' ? a.days_remaining : Number.POSITIVE_INFINITY;
    const db = typeof b.days_remaining === 'number' ? b.days_remaining : Number.POSITIVE_INFINITY;
    if (da !== db) return da - db;
    return String(a.engine_id).localeCompare(String(b.engine_id));
  });
  const report = {
    ok: true,
    type: 'agent_runtime_native_transport_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    registry_path: REGISTRY_PATH,
    mode: 'deterministic_registry_native_transport_probe',
    summary: {
      engine_count: rows.length,
      bounded_envelope_engine_count: applicableRows.length,
      native_transport_available_count: applicableRows.filter((row) => row.typed_turn_api_available === true).length,
      pending_probe_count: pendingRows.length,
      pending_golden_pair_probe_count: pendingRows.filter((row) => row.promotion_scope === 'golden_pair_runtime').length,
      pending_secondary_probe_count: pendingRows.filter((row) => row.promotion_scope === 'secondary_runtime').length,
    },
    rows,
    ranked_next_actions: rankedNextActions,
    warnings: pendingRows.map((row) => ({
      kind: 'upstream_native_transport_probe_pending',
      engine_id: row.engine_id,
      path: REGISTRY_PATH,
      detail: `${row.engine_id} still uses bounded envelope compatibility transport until a stable native typed turn/session/context API is proven.`,
      days_remaining: row.review_days_remaining,
      warning_window_days: REVIEW_WARNING_WINDOW_DAYS,
      priority: row.promotion_scope === 'golden_pair_runtime' ? 'red' : 'yellow',
      next_action: row.next_action,
    })),
    violations: [],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

main();
