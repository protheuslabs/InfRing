#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');
const GAP_MATRIX_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_secondary_engine_gap_matrix_guard_current.json');
const READINESS_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_secondary_live_adapter_readiness_guard_current.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_secondary_promotion_plan_guard_current.json');
const ALLOWED_STAGE_IDS = [
  'needs_gateway_mediated_live_adapter_evidence',
  'ready_for_operator_opt_in_native_probe',
  'setup_reference_runtime_first',
  'install_sdk_dependency_first',
  'install_or_configure_runtime_first',
  'restore_provider_auth_or_usage_first',
  'fix_native_mapping_probe_first',
  'repair_live_probe_artifact_first',
  'generate_or_repair_readiness_artifacts_first',
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

function readOptionalJson(filePath) {
  if (!filePath) return null;
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, filePath), 'utf8'));
  } catch {
    return null;
  }
}

function unique(values) {
  const out = [];
  const seen = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const item = clean(value, 160);
    if (!item || seen.has(item)) continue;
    seen.add(item);
    out.push(item);
  }
  return out;
}

function duplicates(values) {
  const seen = new Set();
  const dupes = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const item = clean(value, 160);
    if (!item) continue;
    if (seen.has(item)) dupes.add(item);
    seen.add(item);
  }
  return [...dupes].sort();
}

function readinessRows(readiness) {
  return Array.isArray(readiness && readiness.rows) ? readiness.rows : [];
}

function readinessStatus(row) {
  return clean(row && row.readiness && row.readiness.status, 160);
}

function readinessReason(row) {
  return clean(row && row.readiness && row.readiness.reason, 700);
}

function readinessAction(row) {
  const actions = Array.isArray(row && row.next_actions)
    ? row.next_actions.map((item) => clean(item, 700)).filter(Boolean)
    : [];
  return actions[0] || '';
}

function stageFor(status) {
  if (status === 'live_probe_previously_accepted') return 'needs_gateway_mediated_live_adapter_evidence';
  if (status === 'ready_for_opt_in_live_probe') return 'ready_for_operator_opt_in_native_probe';
  if (status === 'reference_checkout_available') return 'setup_reference_runtime_first';
  if (status === 'sdk_package_missing') return 'install_sdk_dependency_first';
  if (status === 'runtime_command_missing') return 'install_or_configure_runtime_first';
  if (status === 'provider_or_auth_unavailable') return 'restore_provider_auth_or_usage_first';
  if (status === 'mapping_not_ready') return 'fix_native_mapping_probe_first';
  if (status === 'live_probe_needs_attention') return 'repair_live_probe_artifact_first';
  return 'generate_or_repair_readiness_artifacts_first';
}

function stageRank(stage) {
  const ranks = {
    needs_gateway_mediated_live_adapter_evidence: 10,
    ready_for_operator_opt_in_native_probe: 20,
    setup_reference_runtime_first: 30,
    install_sdk_dependency_first: 40,
    install_or_configure_runtime_first: 50,
    restore_provider_auth_or_usage_first: 60,
    fix_native_mapping_probe_first: 70,
    repair_live_probe_artifact_first: 80,
    generate_or_repair_readiness_artifacts_first: 90,
  };
  return ranks[stage] || 99;
}

function gapWeight(row) {
  return Number(row && row.red_gap_count || 0) * 100 + Number(row && row.yellow_gap_count || 0);
}

function gapActions(row) {
  return Array.isArray(row && row.next_actions)
    ? row.next_actions.map((item) => clean(item, 500)).filter(Boolean).slice(0, 3)
    : [];
}

function artifactRefFor(contract, engineId) {
  const artifacts = Array.isArray(contract && contract.secondary_readiness_artifacts)
    ? contract.secondary_readiness_artifacts
    : [];
  const row = artifacts.find((item) => clean(item && item.engine_id, 160) === engineId);
  return clean(row && row.artifact_ref, 500);
}

function main() {
  const violations = [];
  const contract = readJson(CONTRACT_PATH, violations);
  const gapMatrix = readJson(GAP_MATRIX_PATH, violations);
  const readiness = readJson(READINESS_PATH, violations);

  if (contract && contract.status !== 'active') violations.push({ kind: 'graduation_contract_not_active' });
  if (gapMatrix && gapMatrix.ok !== true) violations.push({ kind: 'secondary_gap_matrix_not_ok' });
  if (readiness && readiness.ok !== true) violations.push({ kind: 'secondary_readiness_not_ok' });

  const baseline = contract && contract.current_baseline ? contract.current_baseline : {};
  const planContract = contract && contract.secondary_promotion_plan_artifact && typeof contract.secondary_promotion_plan_artifact === 'object'
    ? contract.secondary_promotion_plan_artifact
    : {};
  const dailyDriverIds = new Set(unique(baseline.daily_driver_allowed_now));
  const contractStageIds = unique(planContract.allowed_stages);
  const rawSecondaryReadinessSamples = Array.isArray(
    contract &&
      contract.secondary_eval_samples &&
      contract.secondary_eval_samples.live_adapter_readiness,
  )
    ? contract.secondary_eval_samples.live_adapter_readiness.map((engineId) => clean(engineId, 160)).filter(Boolean)
    : [];
  const duplicateSecondarySamples = duplicates(rawSecondaryReadinessSamples);
  const secondaryIds = unique(rawSecondaryReadinessSamples).filter((engineId) => !dailyDriverIds.has(engineId));
  const secondaryArtifactEngineIds = unique(
    Array.isArray(contract && contract.secondary_readiness_artifacts)
      ? contract.secondary_readiness_artifacts.map((row) => row && row.engine_id)
      : [],
  ).filter((engineId) => !dailyDriverIds.has(engineId));

  for (const stageId of ALLOWED_STAGE_IDS) {
    if (!contractStageIds.includes(stageId)) {
      violations.push({ kind: 'secondary_promotion_stage_missing_from_contract', stage_id: stageId });
    }
  }
  for (const stageId of contractStageIds) {
    if (!ALLOWED_STAGE_IDS.includes(stageId)) {
      violations.push({ kind: 'secondary_promotion_stage_unknown_in_contract', stage_id: stageId });
    }
  }

  if (!rawSecondaryReadinessSamples.length) {
    violations.push({ kind: 'secondary_live_adapter_readiness_samples_missing' });
  }
  for (const engineId of rawSecondaryReadinessSamples) {
    if (dailyDriverIds.has(engineId)) {
      violations.push({ kind: 'daily_driver_engine_in_secondary_promotion_plan_scope', engine_id: engineId });
    }
  }
  for (const engineId of duplicateSecondarySamples) {
    violations.push({ kind: 'secondary_live_adapter_readiness_sample_duplicate', engine_id: engineId });
  }
  for (const engineId of secondaryIds) {
    if (!secondaryArtifactEngineIds.includes(engineId)) {
      violations.push({ kind: 'secondary_readiness_artifact_missing_for_sample', engine_id: engineId });
    }
  }
  for (const engineId of secondaryArtifactEngineIds) {
    if (!secondaryIds.includes(engineId)) {
      violations.push({ kind: 'secondary_readiness_artifact_not_in_promotion_plan_scope', engine_id: engineId });
    }
  }

  const gapRows = Array.isArray(gapMatrix && gapMatrix.secondary_rows) ? gapMatrix.secondary_rows : [];
  const readyRows = readinessRows(readiness);
  const plans = secondaryIds.map((engineId) => {
    const gapRow = gapRows.find((row) => clean(row && row.engine_id, 160) === engineId) || null;
    const readyRow = readyRows.find((row) => clean(row && row.engine_id, 160) === engineId) || null;
    const artifactRef = artifactRefFor(contract, engineId);
    const perEngineReadiness = readOptionalJson(artifactRef);
    const readinessSource = readyRow || perEngineReadiness;
    const status = readinessStatus(readinessSource);
    const stage = stageFor(status);
    if (!ALLOWED_STAGE_IDS.includes(stage)) {
      violations.push({ kind: 'secondary_promotion_plan_stage_unknown', engine_id: engineId, stage });
    }
    if (!gapRow) violations.push({ kind: 'secondary_gap_row_missing', engine_id: engineId });
    if (!readinessSource) violations.push({ kind: 'secondary_readiness_row_missing', engine_id: engineId, artifact_ref: artifactRef });
    if (!readinessAction(readinessSource)) violations.push({ kind: 'secondary_readiness_action_missing', engine_id: engineId, status });
    return {
      engine_id: engineId,
      display_name: clean((gapRow && gapRow.display_name) || (readinessSource && readinessSource.display_name), 160),
      current_stage: stage,
      stage_rank: stageRank(stage),
      readiness_status: status,
      readiness_reason: readinessReason(readinessSource),
      recommended_next_action: readinessAction(readinessSource) || gapActions(gapRow)[0] || 'Regenerate readiness and gap matrix artifacts.',
      secondary_gap_counts: {
        red: Number(gapRow && gapRow.red_gap_count || 0),
        yellow: Number(gapRow && gapRow.yellow_gap_count || 0),
        weight: gapWeight(gapRow),
      },
      promotion_effect: {
        daily_driver_eligible_now: false,
        counts_as_golden_pair_equivalent: false,
        counts_as_native_intelligence_proof: false,
        rule: 'Secondary readiness only identifies the next safe step. It does not promote a runtime.',
      },
      artifact_refs: [
        artifactRef,
        clean(readinessSource && readinessSource.readiness && readinessSource.readiness.live_probe_ref, 500),
      ].filter(Boolean),
      gap_next_actions: gapActions(gapRow),
    };
  }).sort((a, b) => a.stage_rank - b.stage_rank || a.secondary_gap_counts.weight - b.secondary_gap_counts.weight || a.engine_id.localeCompare(b.engine_id));

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_secondary_promotion_plan_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: rel(CONTRACT_PATH),
    rule: 'Secondary runtimes must have a compact ordered promotion plan before operators spend live usage or install dependencies.',
    summary: {
      secondary_engine_count: secondaryIds.length,
      planned_engine_count: plans.length,
      duplicate_secondary_sample_count: duplicateSecondarySamples.length,
      stage_taxonomy_count: contractStageIds.length,
      nearest_next_step: plans[0] ? {
        engine_id: plans[0].engine_id,
        current_stage: plans[0].current_stage,
        recommended_next_action: plans[0].recommended_next_action,
      } : null,
      stages: plans.map((row) => ({
        engine_id: row.engine_id,
        current_stage: row.current_stage,
        readiness_status: row.readiness_status,
      })),
    },
    plans,
    source_artifact_refs: [
      rel(CONTRACT_PATH),
      rel(GAP_MATRIX_PATH),
      rel(READINESS_PATH),
      ...plans.flatMap((row) => row.artifact_refs),
    ],
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
  if (!report.ok) process.exit(1);
}

main();
