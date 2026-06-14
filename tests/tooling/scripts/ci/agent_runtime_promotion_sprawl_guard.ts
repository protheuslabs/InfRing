#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const SCORECARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_engine_scorecard_current.json');
const GOLDEN_PAIR_GUARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_golden_pair_promotion_guard_current.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_promotion_sprawl_guard_current.json');
const GRADUATION_CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');

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

function scoreRows(scorecard) {
  const rows = scorecard && scorecard.rows;
  return Array.isArray(rows) ? rows : Object.values(rows || {});
}

function capStatus(row, name) {
  return clean(row && row.capabilities && row.capabilities[name] && row.capabilities[name].status, 160);
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
  const graduationContract = readJson(GRADUATION_CONTRACT_PATH, violations);
  const scorecard = readJson(SCORECARD_PATH, violations);
  const goldenPairGuard = readJson(GOLDEN_PAIR_GUARD_PATH, violations);
  const contractBaseline = graduationContract && graduationContract.current_baseline ? graduationContract.current_baseline : {};
  const contractRule = graduationContract && graduationContract.graduation_rule ? graduationContract.graduation_rule : {};
  const dailyDriverAllowed = new Set(Array.isArray(contractBaseline.daily_driver_allowed_now) ? contractBaseline.daily_driver_allowed_now : []);
  const externalGoldenPair = new Set(Array.isArray(contractBaseline.golden_external_pair) ? contractBaseline.golden_external_pair : []);
  const promotedClassifications = new Set(Array.isArray(contractRule.blocked_classifications) ? contractRule.blocked_classifications : []);
  const blockedStatusValues = new Set(Array.isArray(contractRule.blocked_status_values) ? contractRule.blocked_status_values : []);
  const minimumDailyDriverScore = Number(contractRule.minimum_daily_driver_score || 0.9);
  const rows = scoreRows(scorecard);
  const reviewed = [];

  if (!graduationContract || graduationContract.status !== 'active') {
    violations.push({ kind: 'graduation_contract_not_active' });
  }
  for (const engineId of ['infring_native', 'codex_cli', 'claude_code']) {
    if (!dailyDriverAllowed.has(engineId)) {
      violations.push({ kind: 'graduation_contract_daily_driver_baseline_missing', engine_id: engineId });
    }
  }
  if (scorecard && scorecard.ok !== true) {
    violations.push({ kind: 'scorecard_not_ok' });
  }
  if (goldenPairGuard && goldenPairGuard.ok !== true) {
    violations.push({ kind: 'golden_pair_guard_not_ok' });
  }

  for (const engineId of externalGoldenPair) {
    const row = rows.find((candidate) => clean(candidate && candidate.engine_id, 160) === engineId);
    if (!row) {
      violations.push({ kind: 'golden_pair_scorecard_row_missing', engine_id: engineId });
      continue;
    }
    if (clean(row.classification, 160) !== 'daily_driver_candidate') {
      violations.push({
        kind: 'golden_pair_not_daily_driver_candidate',
        engine_id: engineId,
        classification: clean(row.classification, 160),
      });
    }
    if (capStatus(row, 'practical_usability_loop') !== 'pass') {
      violations.push({
        kind: 'golden_pair_practical_loop_not_pass',
        engine_id: engineId,
        status: capStatus(row, 'practical_usability_loop'),
      });
    }
  }

  for (const row of rows) {
    const engineId = clean(row && row.engine_id, 160);
    if (!engineId) continue;
    const classification = clean(row.classification, 160);
    const status = clean(row.status, 160);
    const score = Number(row.score);
    const isAllowed = dailyDriverAllowed.has(engineId);
    const isPromoted = promotedClassifications.has(classification) || blockedStatusValues.has(status);
    const practicalLoop = capStatus(row, 'practical_usability_loop');
    const liveAdapter = capStatus(row, 'live_adapter_evidence');
    const approvalPause = capStatus(row, 'approval_pause');
    const durableReceipts = capStatus(row, 'durable_receipts');
    const activityTrace = capStatus(row, 'activity_trace');
    const errorProjection = capStatus(row, 'error_projection');

    reviewed.push({
      engine_id: engineId,
      classification,
      score: Number.isFinite(score) ? score : null,
      promotion_allowed_now: isAllowed,
      practical_usability_loop: practicalLoop,
      live_adapter_evidence: liveAdapter,
      approval_pause: approvalPause,
      durable_receipts: durableReceipts,
      activity_trace: activityTrace,
      error_projection: errorProjection,
    });

    if (!isAllowed && isPromoted) {
      violations.push({
        kind: 'secondary_runtime_promoted_without_golden_equivalent_lane',
        engine_id: engineId,
        classification,
        status,
      });
    }
    if (!isAllowed && Number.isFinite(score) && score >= minimumDailyDriverScore) {
      violations.push({
        kind: 'secondary_runtime_score_implies_daily_driver_without_promotion_lane',
        engine_id: engineId,
        score,
      });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_promotion_sprawl_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    rule: 'Only InfRing Native plus the Codex/Claude golden external pair may be promoted as daily-driver candidates until another runtime has golden-pair-equivalent practical-loop proof.',
    graduation_contract_ref: rel(GRADUATION_CONTRACT_PATH),
    allowed_daily_driver_engines: [...dailyDriverAllowed],
    external_golden_pair: [...externalGoldenPair],
    reviewed_engine_count: reviewed.length,
    reviewed,
    source_artifact_refs: [
      rel(SCORECARD_PATH),
      rel(GOLDEN_PAIR_GUARD_PATH),
      rel(GRADUATION_CONTRACT_PATH),
    ],
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main();
