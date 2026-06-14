#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const GAP_MATRIX_PATH = path.join(ROOT, 'core/local/artifacts/agent_runtime_secondary_engine_gap_matrix_guard_current.json');
const CONTRACT_PATH = path.join(ROOT, 'validation/conformance/contracts/agent_runtime_graduation_baseline_contract.json');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_graduation_next_actions_guard_current.json');

function readinessArtifactRow(contract, engineId) {
  const rows = Array.isArray(contract && contract.secondary_readiness_artifacts)
    ? contract.secondary_readiness_artifacts
    : [];
  return rows.find((row) => clean(row && row.engine_id, 120) === engineId) || null;
}

function readinessArtifactPath(contract, engineId) {
  const match = readinessArtifactRow(contract, engineId);
  const artifactRef = clean(match && match.artifact_ref, 1000);
  return artifactRef ? path.join(ROOT, artifactRef) : '';
}

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
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function gapWeight(row) {
  return Number(row.red_gap_count || 0) * 100 + Number(row.yellow_gap_count || 0);
}

function actionBuckets(row) {
  const actions = Array.isArray(row.next_actions) ? row.next_actions.map((item) => clean(item, 500)).filter(Boolean) : [];
  const primitives = Array.isArray(row.missing_primitives)
    ? row.missing_primitives.map((item) => clean(item && item.primitive, 120)).filter(Boolean)
    : [];
  return {
    context: actions.filter((item) => /context|continuity/i.test(item)),
    live_adapter: actions.filter((item) => /live adapter|Gateway Agent Runtime route/i.test(item)),
    approval: actions.filter((item) => /approval|receipt|mutation/i.test(item)),
    activity: actions.filter((item) => /activity|dialog|trace/i.test(item)),
    failure: actions.filter((item) => /quota|auth|rate|network|failure/i.test(item)),
    transport: actions.filter((item) => /structured|transport|envelope|prompt/i.test(item)),
    primitives,
  };
}

function readinessFocus(engineId, readinessArtifact, fallback) {
  if (!readinessArtifact || readinessArtifact.ok !== true) return fallback;
  const readiness = readinessArtifact.readiness && typeof readinessArtifact.readiness === 'object'
    ? readinessArtifact.readiness
    : {};
  const status = clean(readiness.status, 160);
  const artifactActions = Array.isArray(readinessArtifact.next_actions)
    ? readinessArtifact.next_actions.map((item) => clean(item, 500)).filter(Boolean)
    : [];
  if (engineId === 'openclaw' && status === 'runtime_requirement_missing') {
    const requirement = clean(readiness.runtime_requirement, 160) || 'the required OpenClaw runtime';
    const current = clean(readiness.current_runtime, 160);
    return current
      ? `Install or switch to ${requirement} for OpenClaw; current runtime is ${current}.`
      : `Install or switch to ${requirement} for OpenClaw.`;
  }
  if (engineId === 'openclaw' && status === 'reference_checkout_entrypoint_available') {
    return 'Start the detected OpenClaw Gateway/socket service, then rerun live adapter readiness.';
  }
  if (engineId === 'openclaw' && status === 'installed_not_running') {
    return 'Start the installed OpenClaw service, then rerun live Gateway-mediated adapter evidence.';
  }
  if (engineId === 'openclaw' && status === 'available') {
    return 'Attach OpenClaw live turn submission behind Gateway and prove the full golden-pair baseline.';
  }
  if (status === 'ready_for_opt_in_live_probe') {
    return artifactActions[0] || `Run the opt-in live acceptance probe for ${engineId}.`;
  }
  if (status === 'live_probe_previously_accepted') {
    return artifactActions[0] || `Run Gateway-mediated live adapter evidence for ${engineId}.`;
  }
  if (status === 'sdk_package_missing' || status === 'runtime_command_missing' || status === 'reference_checkout_available' || status === 'provider_or_auth_unavailable') {
    return artifactActions[0] || fallback;
  }
  return fallback;
}

function readinessRemediation(contract, engineId, readinessArtifact) {
  const row = readinessArtifactRow(contract, engineId);
  const remediation = row && row.runtime_requirement_remediation && typeof row.runtime_requirement_remediation === 'object'
    ? row.runtime_requirement_remediation
    : null;
  const readiness = readinessArtifact && readinessArtifact.readiness && typeof readinessArtifact.readiness === 'object'
    ? readinessArtifact.readiness
    : {};
  const status = clean(readiness.status, 160);
  if (!remediation || status !== clean(remediation.applies_when_status, 160)) {
    return null;
  }
  return {
    required_runtime: clean(remediation.required_runtime, 160),
    operator_action_required: remediation.operator_action_required === true,
    commands: Array.isArray(remediation.commands)
      ? remediation.commands.map((command) => ({
          platforms: Array.isArray(command && command.platforms)
            ? command.platforms.map((item) => clean(item, 80)).filter(Boolean)
            : [],
          tool: clean(command && command.tool, 120),
          command: clean(command && command.command, 500),
          note: clean(command && command.note, 500),
        })).filter((command) => command.command)
      : [],
    after_runtime_ready: Array.isArray(remediation.after_runtime_ready)
      ? remediation.after_runtime_ready.map((item) => clean(item, 500)).filter(Boolean)
      : [],
  };
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
  const gapMatrix = readJson(GAP_MATRIX_PATH, violations);
  const contract = readJson(CONTRACT_PATH, violations);
  const rows = Array.isArray(gapMatrix && gapMatrix.secondary_rows) ? gapMatrix.secondary_rows : [];
  const baseline = contract && contract.current_baseline ? contract.current_baseline : {};
  const baselineEngines = Array.isArray(baseline.daily_driver_allowed_now) ? baseline.daily_driver_allowed_now.map((item) => clean(item, 120)) : [];

  if (gapMatrix && gapMatrix.ok !== true) violations.push({ kind: 'gap_matrix_not_ok' });
  if (contract && contract.status !== 'active') violations.push({ kind: 'graduation_contract_not_active' });
  if (!rows.length) violations.push({ kind: 'secondary_rows_missing' });

  const ranked = rows
    .map((row) => {
      const engineId = clean(row && row.engine_id, 120);
      const redGapCount = Number(row && row.red_gap_count || 0);
      const yellowGapCount = Number(row && row.yellow_gap_count || 0);
      const buckets = actionBuckets(row);
      const readinessPath = readinessArtifactPath(contract, engineId);
      const readinessArtifact = readOptionalJson(readinessPath);
      const remediation = readinessRemediation(contract, engineId, readinessArtifact);
      const nextActions = Array.isArray(row && row.next_actions)
        ? row.next_actions.map((item) => clean(item, 500)).filter(Boolean)
        : [];
      const recommendedFocus = readinessFocus(engineId, readinessArtifact, nextActions[0] || 'Re-run secondary gap matrix after adding evidence.');
      if (!engineId) violations.push({ kind: 'secondary_row_engine_id_missing' });
      if (!nextActions.length) violations.push({ kind: 'secondary_row_next_actions_missing', engine_id: engineId });
      if (baselineEngines.includes(engineId)) violations.push({ kind: 'baseline_engine_leaked_into_secondary_rows', engine_id: engineId });
      return {
        engine_id: engineId,
        display_name: clean(row && row.display_name, 160),
        classification: clean(row && row.classification, 160),
        score: Number.isFinite(Number(row && row.score)) ? Number(row.score) : null,
        red_gap_count: redGapCount,
        yellow_gap_count: yellowGapCount,
        graduation_distance: gapWeight(row),
        recommended_focus: recommendedFocus,
        next_actions: nextActions.slice(0, 4),
        readiness_diagnostic: readinessArtifact && readinessArtifact.ok === true ? {
          artifact_ref: readinessPath ? rel(readinessPath) : '',
          status: clean(readinessArtifact.readiness && readinessArtifact.readiness.status, 160),
          reason: clean(readinessArtifact.readiness && readinessArtifact.readiness.reason, 500),
          promotion_effect: readinessArtifact.promotion_effect || null,
        } : null,
        remediation,
        missing_primitives: Array.isArray(row && row.missing_primitives)
          ? row.missing_primitives.map((gap) => ({
              primitive: clean(gap && gap.primitive, 120),
              severity: clean(gap && gap.severity, 40),
              status: clean(gap && gap.status, 120),
            }))
          : [],
        action_buckets: buckets,
      };
    })
    .sort((a, b) => a.graduation_distance - b.graduation_distance || String(a.engine_id).localeCompare(String(b.engine_id)));

  const nearest = ranked.filter((row) => row.red_gap_count === 0).slice(0, 3);
  const blocked = ranked.filter((row) => row.red_gap_count > 0);
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_graduation_next_actions_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: rel(CONTRACT_PATH),
    rule: 'Secondary runtimes must expose compact next actions before they can graduate toward Codex/Claude-level interchangeability.',
    summary: {
      secondary_engine_count: rows.length,
      nearest_candidate_count: nearest.length,
      blocked_engine_count: blocked.length,
      nearest_candidates: nearest.map((row) => ({
        engine_id: row.engine_id,
        yellow_gap_count: row.yellow_gap_count,
        recommended_focus: row.recommended_focus,
      })),
      blocked_engines: blocked.map((row) => ({
        engine_id: row.engine_id,
        red_gap_count: row.red_gap_count,
        recommended_focus: row.recommended_focus,
      })),
    },
    ranked_next_actions: ranked,
    source_artifact_refs: [
      rel(GAP_MATRIX_PATH),
      rel(CONTRACT_PATH),
      ...ranked
        .map((row) => row.readiness_diagnostic && row.readiness_diagnostic.artifact_ref)
        .filter(Boolean),
    ],
    violations,
  };

  writeReport(report);
  if (!report.ok) process.exit(1);
}

main();
