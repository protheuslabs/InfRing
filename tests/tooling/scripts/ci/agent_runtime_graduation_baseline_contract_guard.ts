#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');
const PACKAGE_PATH = path.join(ROOT, 'package.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_graduation_baseline_contract_guard_current.json');

const REQUIRED_DAILY_DRIVER_BASELINE = ['infring_native', 'codex_cli', 'claude_code'];
const REQUIRED_GOLDEN_PAIR = ['codex_cli', 'claude_code'];
const REQUIRED_PRIMITIVES = [
  'registered',
  'adapter_contract',
  'discovery_metadata',
  'model_catalog_metadata',
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
const REQUIRED_RED_PRIMITIVES = [
  'context_continuity',
  'live_adapter_evidence',
  'live_work_completion',
  'practical_usability_loop',
  'approval_pause',
  'turn_steering_contract',
  'durable_receipts',
  'activity_trace',
  'error_projection',
];
const REQUIRED_BLOCKED_VALUES = [
  'daily_driver_candidate',
  'golden_pair_equivalent',
  'baseline_covered',
  'reliable',
  'promoted',
  'production_ready',
];
const REQUIRED_UPSTREAM_ARTIFACTS = [
  'context_continuity',
  'context_switch_continuity',
  'transcript_reload_persistence',
  'activity_decision_trace_persistence',
  'turn_steering_contract',
  'model_projection_truth',
  'command_execution',
  'hard_failure_projection',
  'working_directory_control',
  'native_transport_probe',
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
      kind: 'contract_read_or_parse_failed',
      path: rel(filePath),
      error: clean(error && error.message ? error.message : error, 1600),
    });
    return null;
  }
}

function pushMissing(violations, kind, value) {
  violations.push({ kind, value: clean(value, 400) });
}

function readOptionalJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function requireIncludes(violations, kind, rows, required) {
  const set = new Set(Array.isArray(rows) ? rows.map((row) => clean(row, 200)).filter(Boolean) : []);
  for (const value of required) {
    if (!set.has(value)) pushMissing(violations, kind, value);
  }
}

function requireUniqueIds(violations, kind, rows) {
  const seen = new Set();
  for (const row of Array.isArray(rows) ? rows : []) {
    const id = clean(row && row.id, 200);
    if (!id) {
      violations.push({ kind: `${kind}_id_missing` });
      continue;
    }
    if (seen.has(id)) violations.push({ kind: `${kind}_id_duplicate`, id });
    seen.add(id);
  }
  return seen;
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
  const contract = readJson(CONTRACT_PATH, violations);
  const baseline = contract && contract.current_baseline ? contract.current_baseline : {};
  const rule = contract && contract.graduation_rule ? contract.graduation_rule : {};
  const secondaryEvalSamples = contract && contract.secondary_eval_samples && typeof contract.secondary_eval_samples === 'object'
    ? contract.secondary_eval_samples
    : {};
  const accounting = contract && contract.proof_accounting ? contract.proof_accounting : {};
  const primitives = Array.isArray(contract && contract.baseline_primitives) ? contract.baseline_primitives : [];
  const upstreamArtifacts = Array.isArray(contract && contract.upstream_primitive_artifacts)
    ? contract.upstream_primitive_artifacts
    : [];
  const secondaryReadinessArtifacts = Array.isArray(contract && contract.secondary_readiness_artifacts)
    ? contract.secondary_readiness_artifacts
    : [];
  const secondaryPromotionPlan = contract && contract.secondary_promotion_plan_artifact && typeof contract.secondary_promotion_plan_artifact === 'object'
    ? contract.secondary_promotion_plan_artifact
    : null;
  const forbiddenShortcuts = Array.isArray(contract && contract.forbidden_shortcuts)
    ? contract.forbidden_shortcuts.map((row) => clean(row, 1000)).filter(Boolean)
    : [];
  const packageJson = readOptionalJson(PACKAGE_PATH) || {};
  const packageScripts = packageJson && packageJson.scripts && typeof packageJson.scripts === 'object'
    ? packageJson.scripts
    : {};

  if (contract) {
    if (contract.type !== 'agent_runtime_graduation_baseline_contract') {
      violations.push({ kind: 'contract_type_invalid', value: clean(contract.type, 200) });
    }
    if (contract.version !== 1) {
      violations.push({ kind: 'contract_version_invalid', value: contract.version });
    }
    if (contract.owner !== 'validation.agent_runtime') {
      violations.push({ kind: 'contract_owner_invalid', value: clean(contract.owner, 200) });
    }
    if (contract.layer !== 'gateway') {
      violations.push({ kind: 'contract_layer_invalid', value: clean(contract.layer, 200) });
    }
    if (contract.status !== 'active') {
      violations.push({ kind: 'contract_status_not_active', value: clean(contract.status, 200) });
    }
  }

  if (accounting.classification !== 'substrate_platform_proof') {
    violations.push({ kind: 'proof_accounting_classification_invalid', value: clean(accounting.classification, 240) });
  }
  if (accounting.not_native_intelligence_proof !== true) {
    violations.push({ kind: 'proof_accounting_native_boundary_missing' });
  }
  if (!/does not prove native/i.test(clean(accounting.rule, 2000))) {
    violations.push({ kind: 'proof_accounting_rule_too_weak', value: clean(accounting.rule, 500) });
  }

  if (baseline.native_reference_engine !== 'infring_native') {
    violations.push({ kind: 'native_reference_engine_invalid', value: clean(baseline.native_reference_engine, 200) });
  }
  requireIncludes(violations, 'golden_pair_engine_missing', baseline.golden_external_pair, REQUIRED_GOLDEN_PAIR);
  requireIncludes(violations, 'daily_driver_baseline_engine_missing', baseline.daily_driver_allowed_now, REQUIRED_DAILY_DRIVER_BASELINE);
  const dailyDriverAllowed = new Set(Array.isArray(baseline.daily_driver_allowed_now) ? baseline.daily_driver_allowed_now.map((item) => clean(item, 200)) : []);
  const contextContinuitySecondarySamples = Array.isArray(secondaryEvalSamples.context_continuity)
    ? secondaryEvalSamples.context_continuity.map((item) => clean(item, 200)).filter(Boolean)
    : [];
  const liveAdapterReadinessSecondarySamples = Array.isArray(secondaryEvalSamples.live_adapter_readiness)
    ? secondaryEvalSamples.live_adapter_readiness.map((item) => clean(item, 200)).filter(Boolean)
    : [];
  for (const engineId of [...contextContinuitySecondarySamples, ...liveAdapterReadinessSecondarySamples]) {
    if (dailyDriverAllowed.has(engineId)) {
      violations.push({ kind: 'secondary_eval_sample_in_daily_driver_baseline', engine_id: engineId });
    }
  }
  if (contextContinuitySecondarySamples.length > 0 && !/without becoming promotion-eligible/i.test(clean(secondaryEvalSamples.rule, 1000))) {
    violations.push({ kind: 'secondary_eval_sample_promotion_boundary_rule_missing' });
  }

  requireIncludes(violations, 'blocked_status_value_missing', rule.blocked_status_values, REQUIRED_BLOCKED_VALUES);
  requireIncludes(violations, 'blocked_classification_missing', rule.blocked_classifications, REQUIRED_BLOCKED_VALUES);
  if (Number(rule.minimum_daily_driver_score) < 0.9) {
    violations.push({ kind: 'minimum_daily_driver_score_too_low', value: rule.minimum_daily_driver_score });
  }
  if (Number(rule.required_primitive_score) !== 1) {
    violations.push({ kind: 'required_primitive_score_must_be_one', value: rule.required_primitive_score });
  }

  const primitiveIds = requireUniqueIds(violations, 'baseline_primitive', primitives);
  for (const primitiveId of REQUIRED_PRIMITIVES) {
    if (!primitiveIds.has(primitiveId)) pushMissing(violations, 'baseline_primitive_missing', primitiveId);
  }
  for (const primitive of primitives) {
    const id = clean(primitive && primitive.id, 200);
    const severity = clean(primitive && primitive.severity, 40);
    if (!['red', 'yellow'].includes(severity)) {
      violations.push({ kind: 'baseline_primitive_severity_invalid', id, severity });
    }
    if (!clean(primitive && primitive.description, 500)) {
      violations.push({ kind: 'baseline_primitive_description_missing', id });
    }
    if (REQUIRED_RED_PRIMITIVES.includes(id) && severity !== 'red') {
      violations.push({ kind: 'baseline_primitive_red_severity_required', id, severity });
    }
  }

  const upstreamIds = requireUniqueIds(violations, 'upstream_primitive_artifact', upstreamArtifacts);
  for (const artifactId of REQUIRED_UPSTREAM_ARTIFACTS) {
    if (!upstreamIds.has(artifactId)) pushMissing(violations, 'upstream_primitive_artifact_missing', artifactId);
  }
  for (const artifact of upstreamArtifacts) {
    const id = clean(artifact && artifact.id, 200);
    const artifactRef = clean(artifact && artifact.artifact_ref, 1000);
    if (!artifactRef.startsWith('core/local/artifacts/')) {
      violations.push({ kind: 'upstream_primitive_artifact_ref_not_local', id, artifact_ref: artifactRef });
    }
  }
  const secondaryReadinessIds = requireUniqueIds(violations, 'secondary_readiness_artifact', secondaryReadinessArtifacts);
  if (!secondaryReadinessIds.has('openclaw_live_adapter_readiness')) {
    violations.push({ kind: 'secondary_readiness_artifact_openclaw_missing' });
  }
  for (const artifact of secondaryReadinessArtifacts) {
    const id = clean(artifact && artifact.id, 200);
    const engineId = clean(artifact && artifact.engine_id, 200);
    const artifactRef = clean(artifact && artifact.artifact_ref, 1000);
    const ruleText = clean(artifact && artifact.rule, 1000);
    if (!engineId) violations.push({ kind: 'secondary_readiness_artifact_engine_missing', id });
    if (dailyDriverAllowed.has(engineId)) {
      violations.push({ kind: 'secondary_readiness_artifact_engine_is_daily_driver', id, engine_id: engineId });
    }
    if (!artifactRef.startsWith('core/local/artifacts/')) {
      violations.push({ kind: 'secondary_readiness_artifact_ref_not_local', id, artifact_ref: artifactRef });
    }
    if (!/diagnostic only/i.test(ruleText) || !/must not count/i.test(ruleText)) {
      violations.push({ kind: 'secondary_readiness_artifact_boundary_rule_missing', id });
    }
    const remediation = artifact && artifact.runtime_requirement_remediation && typeof artifact.runtime_requirement_remediation === 'object'
      ? artifact.runtime_requirement_remediation
      : null;
    if (id === 'openclaw_live_adapter_readiness') {
      if (!remediation) {
        violations.push({ kind: 'openclaw_runtime_requirement_remediation_missing' });
      } else {
        if (clean(remediation.applies_when_status, 120) !== 'runtime_requirement_missing') {
          violations.push({ kind: 'openclaw_runtime_requirement_remediation_status_invalid', value: clean(remediation.applies_when_status, 120) });
        }
        if (!/^node>=22\.19\.0$/.test(clean(remediation.required_runtime, 120))) {
          violations.push({ kind: 'openclaw_runtime_requirement_remediation_runtime_invalid', value: clean(remediation.required_runtime, 120) });
        }
        if (remediation.operator_action_required !== true) {
          violations.push({ kind: 'openclaw_runtime_requirement_remediation_operator_action_required_missing' });
        }
        const commands = Array.isArray(remediation.commands) ? remediation.commands : [];
        if (commands.length < 2) {
          violations.push({ kind: 'openclaw_runtime_requirement_remediation_commands_too_few', count: commands.length });
        }
        for (const command of commands) {
          const commandText = clean(command && command.command, 500);
          if (!commandText.includes('22.19.0')) {
            violations.push({ kind: 'openclaw_runtime_requirement_remediation_command_missing_version', command: commandText });
          }
          if (/(curl|irm|Invoke-WebRequest|sudo|rm\s+-rf)/i.test(commandText)) {
            violations.push({ kind: 'openclaw_runtime_requirement_remediation_command_too_privileged', command: commandText });
          }
        }
        const afterRuntimeReady = Array.isArray(remediation.after_runtime_ready) ? remediation.after_runtime_ready : [];
        if (!afterRuntimeReady.some((item) => /Gateway/i.test(clean(item, 500)))) {
          violations.push({ kind: 'openclaw_runtime_requirement_remediation_after_ready_gateway_step_missing' });
        }
      }
    }
  }

  if (!secondaryPromotionPlan) {
    violations.push({ kind: 'secondary_promotion_plan_artifact_missing' });
  } else {
    const id = clean(secondaryPromotionPlan.id, 200);
    const artifactRef = clean(secondaryPromotionPlan.artifact_ref, 1000);
    const guardRef = clean(secondaryPromotionPlan.guard_ref, 1000);
    const commandRef = clean(secondaryPromotionPlan.command_ref, 240);
    const ruleText = clean(secondaryPromotionPlan.rule, 1200);
    const promotionEffect = secondaryPromotionPlan.promotion_effect && typeof secondaryPromotionPlan.promotion_effect === 'object'
      ? secondaryPromotionPlan.promotion_effect
      : {};
    if (id !== 'secondary_runtime_ordered_promotion_plan') {
      violations.push({ kind: 'secondary_promotion_plan_id_invalid', id });
    }
    if (artifactRef !== 'core/local/artifacts/agent_runtime_secondary_promotion_plan_guard_current.json') {
      violations.push({ kind: 'secondary_promotion_plan_artifact_ref_invalid', artifact_ref: artifactRef });
    }
    if (guardRef !== 'tests/tooling/scripts/ci/agent_runtime_secondary_promotion_plan_guard.ts') {
      violations.push({ kind: 'secondary_promotion_plan_guard_ref_invalid', guard_ref: guardRef });
    }
    if (!fs.existsSync(path.join(ROOT, guardRef))) {
      violations.push({ kind: 'secondary_promotion_plan_guard_file_missing', guard_ref: guardRef });
    }
    if (commandRef !== 'ops:agent-runtime:secondary-promotion-plan:guard') {
      violations.push({ kind: 'secondary_promotion_plan_command_ref_invalid', command_ref: commandRef });
    }
    if (!packageScripts[commandRef]) {
      violations.push({ kind: 'secondary_promotion_plan_command_missing_from_package_scripts', command_ref: commandRef });
    }
    if (!/compact ordered promotion plan/i.test(ruleText) || !/before operators spend live usage/i.test(ruleText)) {
      violations.push({ kind: 'secondary_promotion_plan_rule_too_weak', value: ruleText });
    }
    if (promotionEffect.changes_daily_driver_eligibility !== false) {
      violations.push({ kind: 'secondary_promotion_plan_daily_driver_boundary_missing' });
    }
    if (promotionEffect.counts_as_golden_pair_equivalent !== false) {
      violations.push({ kind: 'secondary_promotion_plan_golden_pair_boundary_missing' });
    }
    if (promotionEffect.counts_as_native_intelligence_proof !== false) {
      violations.push({ kind: 'secondary_promotion_plan_native_proof_boundary_missing' });
    }
  }

  if (forbiddenShortcuts.length < 3) {
    violations.push({ kind: 'forbidden_shortcuts_too_few', count: forbiddenShortcuts.length });
  }
  if (!forbiddenShortcuts.some((row) => /shell/i.test(row))) {
    violations.push({ kind: 'forbidden_shortcuts_shell_boundary_missing' });
  }
  if (!forbiddenShortcuts.some((row) => /native InfRing intelligence/i.test(row))) {
    violations.push({ kind: 'forbidden_shortcuts_native_proof_boundary_missing' });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_graduation_baseline_contract_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    contract_ref: rel(CONTRACT_PATH),
    summary: {
      golden_external_pair: Array.isArray(baseline.golden_external_pair) ? baseline.golden_external_pair : [],
      daily_driver_allowed_now: Array.isArray(baseline.daily_driver_allowed_now) ? baseline.daily_driver_allowed_now : [],
      secondary_context_continuity_samples: contextContinuitySecondarySamples,
      secondary_live_adapter_readiness_samples: liveAdapterReadinessSecondarySamples,
      baseline_primitive_count: primitives.length,
      red_primitive_count: primitives.filter((row) => clean(row && row.severity, 40) === 'red').length,
      upstream_primitive_artifact_count: upstreamArtifacts.length,
      secondary_readiness_artifact_count: secondaryReadinessArtifacts.length,
      secondary_promotion_plan_artifact: secondaryPromotionPlan
        ? clean(secondaryPromotionPlan.artifact_ref, 1000)
        : '',
      forbidden_shortcut_count: forbiddenShortcuts.length,
    },
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main();
