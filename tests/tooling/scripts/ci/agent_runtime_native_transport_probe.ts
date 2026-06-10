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
const BOUNDED_ENVELOPE_MODES = new Set([
  'bounded_stdin_context_envelope',
  'bounded_cli_arg_context_envelope',
]);

function clean(value: any, max = 500): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
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
  if (engineId !== 'claude_code') return null;
  const probe = readJson(CLAUDE_STREAM_JSON_MAPPING_PROBE);
  return probe && probe.type === 'agent_runtime_claude_stream_json_mapping_probe' ? probe : null;
}

function liveAcceptanceProbeForEngine(engineId: string): JsonObject | null {
  if (engineId === 'codex_cli') {
    const probe = readJson(CODEX_APP_SERVER_LIVE_ACCEPTANCE_PROBE);
    return probe && probe.type === 'agent_runtime_codex_app_server_live_acceptance_probe' ? probe : null;
  }
  if (engineId !== 'claude_code') return null;
  const probe = readJson(CLAUDE_STREAM_JSON_LIVE_ACCEPTANCE_PROBE);
  return probe && probe.type === 'agent_runtime_claude_stream_json_live_acceptance_probe' ? probe : null;
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
    source_authority: liveAccepted
      ? engineId === 'codex_cli'
        ? 'validation.agent_runtime_codex_app_server_live_acceptance_probe'
        : 'validation.agent_runtime_claude_stream_json_live_acceptance_probe'
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
      ? engineId === 'codex_cli'
        ? CODEX_APP_SERVER_MAPPING_PROBE
        : CLAUDE_STREAM_JSON_MAPPING_PROBE
      : '',
    live_acceptance_probe_ref: liveAcceptanceProbe
      ? engineId === 'codex_cli'
        ? CODEX_APP_SERVER_LIVE_ACCEPTANCE_PROBE
        : CLAUDE_STREAM_JSON_LIVE_ACCEPTANCE_PROBE
      : '',
    next_action: applies && !(typedTurnApiAvailable || liveAccepted)
      ? mappingReady
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
    },
    rows,
    warnings: pendingRows.map((row) => ({
      kind: 'upstream_native_transport_probe_pending',
      engine_id: row.engine_id,
      next_action: row.next_action,
    })),
    violations: [],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

main();
