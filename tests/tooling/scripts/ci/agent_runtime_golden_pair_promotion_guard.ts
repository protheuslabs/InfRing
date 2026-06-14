#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_golden_pair_promotion_guard_current.json');
const SCORECARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_engine_scorecard_current.json');
const NATIVE_TRANSPORT_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_native_transport_probe_current.json');
const CODEX_LIVE_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_codex_app_server_live_acceptance_probe_current.json');
const CLAUDE_LIVE_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_claude_stream_json_live_acceptance_probe_current.json');
const CLAUDE_LIVE_WORK_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_live_work_eval_claude_native_probe.json');

const GOLDEN_PAIR = ['codex_cli', 'claude_code'];
const REQUIRED_CAPABILITIES = [
  'context_continuity',
  'live_adapter_evidence',
  'live_work_completion',
  'practical_usability_loop',
  'approval_pause',
  'durable_receipts',
  'activity_trace',
  'structured_transport',
  'transport_migration',
  'error_projection',
];

function clean(value, max = 1000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function readJson(filePath, violations) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    violations.push({
      kind: 'artifact_read_failed',
      path: rel(filePath),
      error: clean(error && error.message ? error.message : error, 1600),
    });
    return null;
  }
}

function rowsByEngine(rows) {
  const map = new Map();
  for (const row of Array.isArray(rows) ? rows : Object.values(rows || {})) {
    const engineId = clean(row && row.engine_id, 120);
    if (engineId) map.set(engineId, row);
  }
  return map;
}

function capStatus(row, capability) {
  return clean(row && row.capabilities && row.capabilities[capability] && row.capabilities[capability].status, 120);
}

function liveProbeReady(artifact) {
  if (!artifact || artifact.ok !== true || artifact.mapping_probe_ready !== true) return false;
  if (artifact.accepted_by_runtime === true && artifact.typed_turn_api_available === true) return true;
  return artifact.live_enabled === false && artifact.accepted_by_runtime === false;
}

function writeReport(report) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
}

function main() {
  const violations = [];
  const scorecard = readJson(SCORECARD_PATH, violations);
  const nativeTransport = readJson(NATIVE_TRANSPORT_PATH, violations);
  const codexLive = readJson(CODEX_LIVE_PATH, violations);
  const claudeLive = readJson(CLAUDE_LIVE_PATH, violations);
  const claudeLiveWork = readJson(CLAUDE_LIVE_WORK_PATH, violations);

  const scoreRows = rowsByEngine(scorecard && scorecard.rows);
  const nativeRows = rowsByEngine(nativeTransport && nativeTransport.rows);
  const goldenRows = [];

  for (const engineId of GOLDEN_PAIR) {
    const row = scoreRows.get(engineId);
    if (!row) {
      violations.push({ kind: 'golden_pair_scorecard_row_missing', engine_id: engineId });
      continue;
    }
    const classification = clean(row.classification, 160);
    const score = Number(row.score);
    if (classification !== 'daily_driver_candidate') {
      violations.push({ kind: 'golden_pair_not_daily_driver_candidate', engine_id: engineId, classification });
    }
    if (!Number.isFinite(score) || score < 0.9) {
      violations.push({ kind: 'golden_pair_score_too_low', engine_id: engineId, score });
    }
    for (const capability of REQUIRED_CAPABILITIES) {
      if (capStatus(row, capability) !== 'pass') {
        violations.push({
          kind: 'golden_pair_capability_not_pass',
          engine_id: engineId,
          capability,
          status: capStatus(row, capability),
        });
      }
    }
    goldenRows.push({
      engine_id: engineId,
      display_name: clean(row.display_name, 160),
      classification,
      score,
      practical_usability_loop: capStatus(row, 'practical_usability_loop'),
      structured_transport: capStatus(row, 'structured_transport'),
      approval_pause: capStatus(row, 'approval_pause'),
      durable_receipts: capStatus(row, 'durable_receipts'),
      activity_trace: capStatus(row, 'activity_trace'),
      error_projection: capStatus(row, 'error_projection'),
    });
  }

  for (const engineId of GOLDEN_PAIR) {
    const row = nativeRows.get(engineId);
    if (!row) {
      violations.push({ kind: 'golden_pair_native_transport_row_missing', engine_id: engineId });
      continue;
    }
    if (clean(row.probe_status, 160) !== 'native_transport_available') {
      violations.push({ kind: 'golden_pair_native_transport_unavailable', engine_id: engineId, probe_status: clean(row.probe_status, 160) });
    }
    if (row.typed_turn_api_available !== true) {
      violations.push({ kind: 'golden_pair_native_transport_typed_api_not_declared', engine_id: engineId });
    }
  }

  if (!liveProbeReady(codexLive)) {
    violations.push({
      kind: 'codex_app_server_live_probe_not_candidate_or_accepted',
      live_enabled: codexLive && codexLive.live_enabled,
      accepted_by_runtime: codexLive && codexLive.accepted_by_runtime,
      typed_turn_api_available: codexLive && codexLive.typed_turn_api_available,
    });
  }
  if (!liveProbeReady(claudeLive)) {
    violations.push({
      kind: 'claude_stream_json_live_probe_not_candidate_or_accepted',
      live_enabled: claudeLive && claudeLive.live_enabled,
      accepted_by_runtime: claudeLive && claudeLive.accepted_by_runtime,
      typed_turn_api_available: claudeLive && claudeLive.typed_turn_api_available,
    });
  }

  const claudeLiveWorkResult = claudeLiveWork && claudeLiveWork.results;
  if (
    !claudeLiveWork ||
    claudeLiveWork.ok !== true ||
    clean(claudeLiveWork.engine_id, 120) !== 'claude_code' ||
    clean(claudeLiveWorkResult && claudeLiveWorkResult.completion && claudeLiveWorkResult.completion.status, 120) !== 'completed' ||
    clean(claudeLiveWorkResult && claudeLiveWorkResult.approval_pause && claudeLiveWorkResult.approval_pause.status, 120) !== 'permission_required' ||
    !(claudeLiveWorkResult && claudeLiveWorkResult.approval_decision && claudeLiveWorkResult.approval_decision.ok === true)
  ) {
    violations.push({ kind: 'claude_native_live_work_probe_missing_or_incomplete' });
  }

  const nativeSummary = nativeTransport && nativeTransport.summary ? nativeTransport.summary : {};
  if (Number(nativeSummary.native_transport_available_count || 0) < 2) {
    violations.push({
      kind: 'golden_pair_native_transport_count_too_low',
      native_transport_available_count: Number(nativeSummary.native_transport_available_count || 0),
    });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_golden_pair_promotion_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    golden_pair: GOLDEN_PAIR,
    promotion_state: violations.length === 0
      ? 'golden_pair_practical_loop_pass_with_gated_native_transport'
      : 'golden_pair_promotion_blocked',
    accounting_rule: 'External-engine mediated work is substrate/platform proof only; it does not prove native InfRing intelligence.',
    summary: {
      scorecard_ok: !!(scorecard && scorecard.ok === true),
      native_transport_ok: !!(nativeTransport && nativeTransport.ok === true),
      native_transport_available_count: Number(nativeSummary.native_transport_available_count || 0),
      pending_probe_count: Number(nativeSummary.pending_probe_count || 0),
      codex_app_server_live_probe: codexLive ? {
        mode: clean(codexLive.mode, 160),
        mapping_probe_ready: codexLive.mapping_probe_ready === true,
        live_enabled: codexLive.live_enabled === true,
        accepted_by_runtime: codexLive.accepted_by_runtime === true,
        typed_turn_api_available: codexLive.typed_turn_api_available === true,
      } : null,
      claude_stream_json_live_probe: claudeLive ? {
        mode: clean(claudeLive.mode, 160),
        mapping_probe_ready: claudeLive.mapping_probe_ready === true,
        live_enabled: claudeLive.live_enabled === true,
        accepted_by_runtime: claudeLive.accepted_by_runtime === true,
        typed_turn_api_available: claudeLive.typed_turn_api_available === true,
      } : null,
      claude_native_live_work: claudeLiveWork ? {
        ok: claudeLiveWork.ok === true,
        classification: clean(claudeLiveWork.engine_results && claudeLiveWork.engine_results[0] && claudeLiveWork.engine_results[0].classification, 160),
        completion_status: clean(claudeLiveWorkResult && claudeLiveWorkResult.completion && claudeLiveWorkResult.completion.status, 160),
        approval_pause_status: clean(claudeLiveWorkResult && claudeLiveWorkResult.approval_pause && claudeLiveWorkResult.approval_pause.status, 160),
        approval_decision_ok: !!(claudeLiveWorkResult && claudeLiveWorkResult.approval_decision && claudeLiveWorkResult.approval_decision.ok === true),
      } : null,
    },
    rows: goldenRows,
    source_artifact_refs: [
      rel(SCORECARD_PATH),
      rel(NATIVE_TRANSPORT_PATH),
      rel(CODEX_LIVE_PATH),
      rel(CLAUDE_LIVE_PATH),
      rel(CLAUDE_LIVE_WORK_PATH),
    ],
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main();
