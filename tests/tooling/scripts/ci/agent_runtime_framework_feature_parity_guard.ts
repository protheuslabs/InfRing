#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const CONTRACT_PATH = path.join(ROOT, 'validation', 'conformance', 'contracts', 'agent_runtime_framework_feature_parity_contract.json');
const REGISTRY_PATH = path.join(ROOT, 'validation', 'conformance', 'contracts', 'agent_runtime_engine_registry.json');
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_framework_feature_parity_guard_current.json');

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function cleanText(value, maxLen = 6000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function fieldKey(engineId, dimensionId, field) {
  return `${cleanText(engineId, 120)}:${cleanText(dimensionId, 120)}:${cleanText(field, 120)}`;
}

function main() {
  const contract = readJson(CONTRACT_PATH);
  const registry = readJson(REGISTRY_PATH);
  const traceId = `validation:agent-runtime-feature-parity:${Date.now()}`;
  const violations = [];
  const warnings = [];

  if (!contract || contract.type !== 'agent_runtime_framework_feature_parity_contract') {
    violations.push('contract_missing_or_wrong_type');
  }
  if (!registry || registry.feature_parity_contract !== 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json') {
    violations.push('engine_registry_feature_parity_contract_ref_missing');
  }

  const allowedStatuses = new Set(Array.isArray(contract.parity_status_values) ? contract.parity_status_values : []);
  const dimensions = Array.isArray(contract.feature_dimensions) ? contract.feature_dimensions : [];
  const requiredDimensions = dimensions
    .filter((dimension) => dimension && dimension.required_for_primary_external_engines === true)
    .map((dimension) => cleanText(dimension.dimension_id, 160))
    .filter(Boolean);
  const dimensionSet = new Set(dimensions.map((dimension) => cleanText(dimension && dimension.dimension_id, 160)).filter(Boolean));
  const primaryEngines = (((contract.promotion_scope || {}).primary_external_engines) || []).map((engineId) => cleanText(engineId, 160)).filter(Boolean);
  const registryPrimary = ((((registry.validation_focus_policy || {}).primary_external_engines) || [])).map((engineId) => cleanText(engineId, 160)).filter(Boolean);
  const matrix = contract.engine_parity_matrix || {};

  if (primaryEngines.length < 2) violations.push('primary_external_engines_missing_or_too_small');
  for (const engineId of primaryEngines) {
    if (!registryPrimary.includes(engineId)) warnings.push(`primary_engine_not_in_registry_validation_focus:${engineId}`);
  }
  if (!requiredDimensions.includes('context_continuity')) violations.push('required_dimension_context_continuity_missing');
  if (!requiredDimensions.includes('permission_pause_resume')) violations.push('required_dimension_permission_pause_resume_missing');
  if (!requiredDimensions.includes('hard_failure_projection')) violations.push('required_dimension_hard_failure_projection_missing');
  if (!requiredDimensions.includes('activity_decision_trace')) violations.push('required_dimension_activity_decision_trace_missing');

  const engineSummaries = [];
  for (const engineId of primaryEngines) {
    const entry = matrix[engineId];
    if (!entry) {
      violations.push(`engine_matrix_missing:${engineId}`);
      continue;
    }
    const featureStatus = entry.feature_status || {};
    const statusCounts = {};
    const gaps = [];
    const partials = [];
    for (const dimensionId of requiredDimensions) {
      if (!dimensionSet.has(dimensionId)) violations.push(`dimension_id_not_declared:${dimensionId}`);
      const row = featureStatus[dimensionId];
      if (!row) {
        violations.push(`feature_status_missing:${engineId}:${dimensionId}`);
        continue;
      }
      const status = cleanText(row.status, 80);
      statusCounts[status] = (statusCounts[status] || 0) + 1;
      if (!allowedStatuses.has(status)) violations.push(`feature_status_invalid:${engineId}:${dimensionId}:${status || 'missing'}`);
      for (const field of contract.required_status_fields || []) {
        const value = cleanText(row[field], 1000);
        if (!value) violations.push(`feature_status_field_missing:${fieldKey(engineId, dimensionId, field)}`);
      }
      if (status === 'gap') gaps.push({ dimension_id: dimensionId, next_action: cleanText(row.next_action, 1000) });
      if (status === 'partial') partials.push({ dimension_id: dimensionId, next_action: cleanText(row.next_action, 1000) });
      if (status === 'preserved' && cleanText(row.evidence_ref, 1000) === 'not_yet_proven') {
        violations.push(`preserved_without_evidence_ref:${engineId}:${dimensionId}`);
      }
    }
    for (const dimensionId of Object.keys(featureStatus)) {
      if (!dimensionSet.has(dimensionId)) warnings.push(`feature_status_unknown_dimension:${engineId}:${dimensionId}`);
    }
    engineSummaries.push({
      engine_id: engineId,
      display_name: entry.display_name || engineId,
      promotion_state: entry.current_promotion_state || 'unknown',
      status_counts: statusCounts,
      blocking_gaps: gaps,
      partials,
    });
  }

  const promotionReady = engineSummaries.length === primaryEngines.length
    && engineSummaries.every((summary) => (summary.status_counts.partial || 0) === 0 && (summary.status_counts.gap || 0) === 0);

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_framework_feature_parity_guard',
    type: 'agent_runtime_framework_feature_parity_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    contract_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    registry_path: 'validation/conformance/contracts/agent_runtime_engine_registry.json',
    trace_id: traceId,
    primary_external_engines: primaryEngines,
    required_dimension_count: requiredDimensions.length,
    promotion_ready: promotionReady,
    promotion_blocked_reason: promotionReady ? null : 'required feature dimensions still include partial or gap statuses',
    engine_summaries: engineSummaries,
    warnings,
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
