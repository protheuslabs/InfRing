#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const SCORECARD_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_engine_scorecard_current.json');
const PRIMITIVE_PARITY_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_golden_pair_primitive_parity_guard_current.json');
const PROMOTION_SPRAWL_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_promotion_sprawl_guard_current.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_secondary_engine_gap_matrix_guard_current.json');
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

function cap(row, name) {
  const value = row && row.capabilities && row.capabilities[name];
  return {
    status: clean(value && value.status, 160),
    score: Number(value && value.score),
    evidence: clean(value && value.evidence, 500),
  };
}

function missingPrimitives(row, baselinePrimitives) {
  const gaps = [];
  for (const primitive of baselinePrimitives) {
    const primitiveId = clean(primitive && primitive.id, 160);
    if (!primitiveId) continue;
    const value = cap(row, primitiveId);
    if (value.status !== 'pass') {
      gaps.push({
        primitive: primitiveId,
        status: value.status || 'missing',
        score: Number.isFinite(value.score) ? value.score : null,
        severity: clean(primitive && primitive.severity, 40) === 'red' ? 'red' : 'yellow',
        evidence: value.evidence,
      });
    }
  }
  return gaps;
}

function nextActionsFor(gaps) {
  const primitives = new Set(gaps.map((gap) => gap.primitive));
  const actions = [];
  if (primitives.has('context_continuity')) {
    actions.push('Prove bounded context pack continuity across at least two turns.');
  }
  if (primitives.has('live_adapter_evidence') || primitives.has('live_work_completion')) {
    actions.push('Run live adapter evidence through the Gateway Agent Runtime route, not only native CLI fixtures.');
  }
  if (primitives.has('approval_pause') || primitives.has('durable_receipts')) {
    actions.push('Map write/mutation attempts into Gateway approval pause plus decision receipts.');
  }
  if (primitives.has('activity_trace')) {
    actions.push('Normalize native activity/tool/dialog events into bounded RuntimeActivityProjection rows.');
  }
  if (primitives.has('error_projection')) {
    actions.push('Classify quota/auth/rate/network/missing-runtime failures into visible chat outcomes.');
  }
  if (primitives.has('structured_transport') || primitives.has('transport_migration')) {
    actions.push('Identify native structured turn/session transport or explicitly keep bounded-envelope compatibility mode.');
  }
  if (!actions.length) {
    actions.push('Re-run the golden-pair-equivalent practical loop and confirm no hidden parity gaps remain.');
  }
  return actions;
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
  const primitiveParity = readJson(PRIMITIVE_PARITY_PATH, violations);
  const promotionSprawl = readJson(PROMOTION_SPRAWL_PATH, violations);
  const contractBaseline = graduationContract && graduationContract.current_baseline ? graduationContract.current_baseline : {};
  const baselineEngines = new Set(Array.isArray(contractBaseline.daily_driver_allowed_now) ? contractBaseline.daily_driver_allowed_now : []);
  const goldenPair = Array.isArray(contractBaseline.golden_external_pair) ? contractBaseline.golden_external_pair : [];
  const baselinePrimitives = Array.isArray(graduationContract && graduationContract.baseline_primitives)
    ? graduationContract.baseline_primitives
    : [];
  const rows = scoreRows(scorecard);
  const secondaryRows = [];
  const baselineRows = [];

  if (!graduationContract || graduationContract.status !== 'active') violations.push({ kind: 'graduation_contract_not_active' });
  if (!baselinePrimitives.length) violations.push({ kind: 'graduation_contract_primitives_missing' });
  if (scorecard && scorecard.ok !== true) violations.push({ kind: 'scorecard_not_ok' });
  if (primitiveParity && primitiveParity.ok !== true) violations.push({ kind: 'primitive_parity_guard_not_ok' });
  if (promotionSprawl && promotionSprawl.ok !== true) violations.push({ kind: 'promotion_sprawl_guard_not_ok' });

  for (const row of rows) {
    const engineId = clean(row && row.engine_id, 160);
    if (!engineId) continue;
    const classification = clean(row.classification, 160);
    const score = Number(row.score);
    const gaps = missingPrimitives(row, baselinePrimitives);
    const redGapCount = gaps.filter((gap) => gap.severity === 'red').length;
    const yellowGapCount = gaps.filter((gap) => gap.severity === 'yellow').length;
    const record = {
      engine_id: engineId,
      display_name: clean(row.display_name, 160),
      classification,
      score: Number.isFinite(score) ? score : null,
      interchangeable_with_golden_pair: gaps.length === 0,
      red_gap_count: redGapCount,
      yellow_gap_count: yellowGapCount,
      missing_primitives: gaps,
      next_actions: nextActionsFor(gaps),
    };

    if (baselineEngines.has(engineId)) {
      baselineRows.push(record);
    } else {
      secondaryRows.push(record);
    }
  }

  for (const engineId of goldenPair) {
    const baseline = baselineRows.find((row) => row.engine_id === engineId);
    if (!baseline) {
      violations.push({ kind: 'golden_pair_baseline_row_missing', engine_id: engineId });
    } else if (!baseline.interchangeable_with_golden_pair) {
      violations.push({
        kind: 'golden_pair_baseline_has_primitive_gaps',
        engine_id: engineId,
        red_gap_count: baseline.red_gap_count,
        yellow_gap_count: baseline.yellow_gap_count,
      });
    }
  }

  for (const row of secondaryRows) {
    if (row.interchangeable_with_golden_pair) {
      violations.push({
        kind: 'secondary_engine_matches_golden_pair_without_promotion_review',
        engine_id: row.engine_id,
        classification: row.classification,
      });
    }
    if (!row.next_actions.length) {
      violations.push({ kind: 'secondary_engine_next_actions_missing', engine_id: row.engine_id });
    }
  }

  const redEngines = secondaryRows
    .filter((row) => row.red_gap_count > 0)
    .map((row) => row.engine_id);
  const nearestCandidates = secondaryRows
    .filter((row) => row.red_gap_count === 0)
    .sort((a, b) => a.yellow_gap_count - b.yellow_gap_count || String(a.engine_id).localeCompare(String(b.engine_id)))
    .map((row) => ({
      engine_id: row.engine_id,
      yellow_gap_count: row.yellow_gap_count,
      next_actions: row.next_actions,
    }));

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_secondary_engine_gap_matrix_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    rule: 'Every non-baseline runtime must keep an explicit gap matrix until it proves golden-pair-equivalent Agent Runtime primitives.',
    baseline_engines: [...baselineEngines],
    golden_pair: goldenPair,
    graduation_contract_ref: rel(GRADUATION_CONTRACT_PATH),
    baseline_primitives: baselinePrimitives.map((primitive) => clean(primitive && primitive.id, 160)).filter(Boolean),
    summary: {
      secondary_engine_count: secondaryRows.length,
      secondary_with_red_gaps: redEngines.length,
      secondary_with_no_red_gaps: nearestCandidates.length,
      red_gap_engines: redEngines,
      nearest_candidates: nearestCandidates,
    },
    secondary_rows: secondaryRows,
    source_artifact_refs: [
      rel(SCORECARD_PATH),
      rel(PRIMITIVE_PARITY_PATH),
      rel(PROMOTION_SPRAWL_PATH),
      rel(GRADUATION_CONTRACT_PATH),
    ],
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main();
