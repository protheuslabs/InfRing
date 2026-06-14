#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_golden_pair_primitive_parity_guard_current.json');
const SCORECARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_engine_scorecard_current.json');
const GOLDEN_PAIR_GUARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_golden_pair_promotion_guard_current.json');
const PROMOTION_SPRAWL_GUARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_promotion_sprawl_guard_current.json');
const GRADUATION_CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');

const UPSTREAM_GUARD_ARTIFACTS = [
  ['context_continuity', 'core/local/artifacts/agent_runtime_context_continuity_eval_current.json'],
  ['context_switch_continuity', 'core/local/artifacts/agent_runtime_context_switch_continuity_guard_current.json'],
  ['transcript_reload_persistence', 'core/local/artifacts/agent_runtime_transcript_reload_persistence_guard_current.json'],
  ['activity_decision_trace_persistence', 'core/local/artifacts/agent_runtime_activity_decision_trace_persistence_guard_current.json'],
  ['turn_steering_contract', 'core/local/artifacts/agent_runtime_rendered_permission_steering_contract_guard_current.json'],
  ['model_projection_truth', 'core/local/artifacts/agent_runtime_model_projection_truth_guard_current.json'],
  ['command_execution', 'core/local/artifacts/agent_runtime_command_execution_guard_current.json'],
  ['hard_failure_projection', 'core/local/artifacts/agent_runtime_hard_failure_projection_guard_current.json'],
  ['working_directory_control', 'core/local/artifacts/agent_runtime_working_directory_control_guard_current.json'],
  ['native_transport_probe', 'core/local/artifacts/agent_runtime_native_transport_probe_current.json'],
];

const GOLDEN_PAIR = ['codex_cli', 'claude_code'];
const REQUIRED_SCORECARD_CAPABILITIES = [
  'context_continuity',
  'live_adapter_evidence',
  'live_work_completion',
  'real_work_replay',
  'practical_usability_loop',
  'approval_pause',
  'turn_steering_contract',
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

function readJson(filePath, violations, kind = 'artifact_read_failed') {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    violations.push({
      kind,
      path: rel(filePath),
      error: clean(error && error.message ? error.message : error, 1600),
    });
    return null;
  }
}

function scoreRows(scorecard) {
  const rows = scorecard && scorecard.rows;
  return Array.isArray(rows) ? rows : Object.values(rows || {});
}

function byEngine(rows) {
  const map = new Map();
  for (const row of rows || []) {
    const engineId = clean(row && row.engine_id, 160);
    if (engineId) map.set(engineId, row);
  }
  return map;
}

function cap(row, name) {
  const value = row && row.capabilities && row.capabilities[name];
  return {
    status: clean(value && value.status, 160),
    score: Number(value && value.score),
    evidence: clean(value && value.evidence, 500),
  };
}

function writeReport(report) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
}

function artifactOk(name, artifact, violations) {
  if (!artifact || artifact.ok !== true) {
    violations.push({ kind: 'upstream_primitive_guard_not_ok', primitive: name });
    return false;
  }
  return true;
}

function main() {
  const violations = [];
  const graduationContract = readJson(GRADUATION_CONTRACT_PATH, violations);
  const scorecard = readJson(SCORECARD_PATH, violations);
  const goldenPairGuard = readJson(GOLDEN_PAIR_GUARD_PATH, violations);
  const promotionSprawlGuard = readJson(PROMOTION_SPRAWL_GUARD_PATH, violations);
  const upstreamArtifacts = [];
  const rows = byEngine(scoreRows(scorecard));
  const parityRows = [];

  if (!graduationContract || graduationContract.status !== 'active') violations.push({ kind: 'graduation_contract_not_active' });
  const contractPrimitives = Array.isArray(graduationContract && graduationContract.baseline_primitives)
    ? graduationContract.baseline_primitives.map((primitive) => clean(primitive && primitive.id, 160)).filter(Boolean)
    : [];
  for (const capability of REQUIRED_SCORECARD_CAPABILITIES) {
    if (!contractPrimitives.includes(capability)) {
      violations.push({ kind: 'primitive_parity_capability_missing_from_graduation_contract', capability });
    }
  }
  if (scorecard && scorecard.ok !== true) violations.push({ kind: 'scorecard_not_ok' });
  if (goldenPairGuard && goldenPairGuard.ok !== true) violations.push({ kind: 'golden_pair_guard_not_ok' });
  if (promotionSprawlGuard && promotionSprawlGuard.ok !== true) violations.push({ kind: 'promotion_sprawl_guard_not_ok' });

  for (const [name, artifactPath] of UPSTREAM_GUARD_ARTIFACTS) {
    const artifact = readJson(path.join(ROOT, artifactPath), violations, 'upstream_primitive_artifact_missing');
    const ok = artifactOk(name, artifact, violations);
    upstreamArtifacts.push({
      primitive: name,
      ok,
      artifact_ref: artifactPath,
      type: clean(artifact && artifact.type, 200),
    });
  }

  for (const engineId of GOLDEN_PAIR) {
    const row = rows.get(engineId);
    if (!row) {
      violations.push({ kind: 'golden_pair_scorecard_row_missing', engine_id: engineId });
      continue;
    }
    const capabilitySummary = {};
    for (const capability of REQUIRED_SCORECARD_CAPABILITIES) {
      const value = cap(row, capability);
      capabilitySummary[capability] = value.status;
      if (value.status !== 'pass') {
        violations.push({
          kind: 'golden_pair_required_primitive_not_pass',
          engine_id: engineId,
          capability,
          status: value.status,
          evidence: value.evidence,
        });
      }
      if (!Number.isFinite(value.score) || value.score < 1) {
        violations.push({
          kind: 'golden_pair_required_primitive_score_not_full',
          engine_id: engineId,
          capability,
          score: Number.isFinite(value.score) ? value.score : null,
        });
      }
    }
    parityRows.push({
      engine_id: engineId,
      display_name: clean(row.display_name, 160),
      classification: clean(row.classification, 160),
      score: Number(row.score),
      capabilities: capabilitySummary,
    });
  }

  if (parityRows.length === GOLDEN_PAIR.length) {
    const left = parityRows[0];
    const right = parityRows[1];
    for (const capability of REQUIRED_SCORECARD_CAPABILITIES) {
      if (left.capabilities[capability] !== right.capabilities[capability]) {
        violations.push({
          kind: 'golden_pair_primitive_status_mismatch',
          capability,
          [left.engine_id]: left.capabilities[capability],
          [right.engine_id]: right.capabilities[capability],
        });
      }
    }
  }

  const summary = scorecard && scorecard.summary ? scorecard.summary : {};
  const sampledLive = Array.isArray(summary.sampled_live_work_engines) ? summary.sampled_live_work_engines : [];
  for (const engineId of GOLDEN_PAIR) {
    if (!sampledLive.includes(engineId)) {
      violations.push({ kind: 'golden_pair_not_sampled_in_live_work', engine_id: engineId });
    }
  }
  if (summary.hard_failure_injection_ok !== true) {
    violations.push({ kind: 'scorecard_hard_failure_injection_not_ok' });
  }
  if (summary.structured_transport_eval_ok !== true) {
    violations.push({ kind: 'scorecard_structured_transport_eval_not_ok' });
  }
  if (summary.transport_migration_ok !== true) {
    violations.push({ kind: 'scorecard_transport_migration_not_ok' });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_golden_pair_primitive_parity_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    rule: 'Codex and Claude Code must preserve the same practical Agent Runtime primitives before InfRing treats them as interchangeable daily-driver external runtimes.',
    golden_pair: GOLDEN_PAIR,
    required_scorecard_capabilities: REQUIRED_SCORECARD_CAPABILITIES,
    upstream_primitive_guards: upstreamArtifacts,
    rows: parityRows,
    summary: {
      sampled_live_work_engines: sampledLive.filter((engineId) => GOLDEN_PAIR.includes(engineId)),
      hard_failure_injection_ok: summary.hard_failure_injection_ok === true,
      structured_transport_eval_ok: summary.structured_transport_eval_ok === true,
      transport_migration_ok: summary.transport_migration_ok === true,
    },
    source_artifact_refs: [
      rel(GRADUATION_CONTRACT_PATH),
      rel(SCORECARD_PATH),
      rel(GOLDEN_PAIR_GUARD_PATH),
      rel(PROMOTION_SPRAWL_GUARD_PATH),
      ...UPSTREAM_GUARD_ARTIFACTS.map(([, artifactPath]) => artifactPath),
    ],
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main();
