#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const SUMMARY_DIR = path.join(ROOT, 'validation/agent_runtime/proof_summaries');
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_proof_summary_guard_current.json');
const MAX_SUMMARY_BYTES = 64 * 1024;
const FORBIDDEN_RAW_KEYS = new Set([
  'activity_trace',
  'agent_activity_events',
  'conversation_tree',
  'full_transcript',
  'messages',
  'output_text',
  'prompt_text',
  'proposal_arguments',
  'raw_event',
  'raw_events',
  'raw_output',
  'raw_response',
  'raw_stderr',
  'raw_stdout',
  'raw_tool_input',
  'raw_tool_result',
  'trace_body',
  'transcript_rows',
]);

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function clean(value, max = 1000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function push(violations, kind, file, detail = '') {
  violations.push({ kind, file, detail: clean(detail, 2000) });
}

function readJson(filePath, violations) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    push(violations, 'proof_summary_json_invalid', path.relative(ROOT, filePath), String(error && error.message || error));
    return null;
  }
}

function fileSize(filePath) {
  try {
    return fs.statSync(filePath).size;
  } catch {
    return 0;
  }
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function scanRawKeys(value, violations, file, trail = '$') {
  if (!value || typeof value !== 'object') return;
  if (Array.isArray(value)) {
    value.forEach((row, idx) => scanRawKeys(row, violations, file, `${trail}[${idx}]`));
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    const normalized = clean(key, 240).toLowerCase();
    const childTrail = `${trail}.${key}`;
    if (FORBIDDEN_RAW_KEYS.has(normalized)) {
      push(violations, 'proof_summary_forbidden_raw_key', file, childTrail);
    }
    scanRawKeys(child, violations, file, childTrail);
  }
}

function validateSummary(filePath, summary, violations) {
  const relative = rel(filePath);
  if (!summary || typeof summary !== 'object') return;
  if (fileSize(filePath) > MAX_SUMMARY_BYTES) {
    push(violations, 'proof_summary_too_large', relative, `${fileSize(filePath)} bytes > ${MAX_SUMMARY_BYTES} bytes`);
  }
  scanRawKeys(summary, violations, relative);
  if (summary.type !== 'agent_runtime_live_proof_summary') {
    push(violations, 'proof_summary_type_invalid', relative, summary.type);
  }
  if (summary.version !== 1) {
    push(violations, 'proof_summary_version_invalid', relative, summary.version);
  }
  if (summary.owner !== 'validation.agent_runtime') {
    push(violations, 'proof_summary_owner_invalid', relative, summary.owner);
  }
  if (!Number.isFinite(Date.parse(clean(summary.generated_at, 200)))) {
    push(violations, 'proof_summary_generated_at_invalid', relative, summary.generated_at);
  }
  if (!/^[a-f0-9]{7,40}$/i.test(clean(summary.source_commit_observed, 80))) {
    push(violations, 'proof_summary_source_commit_missing', relative, summary.source_commit_observed);
  }
  const accounting = summary.proof_accounting && typeof summary.proof_accounting === 'object'
    ? summary.proof_accounting
    : {};
  if (accounting.classification !== 'substrate_platform_proof') {
    push(violations, 'proof_summary_accounting_classification_invalid', relative, accounting.classification);
  }
  if (accounting.not_native_intelligence_proof !== true) {
    push(violations, 'proof_summary_native_proof_boundary_missing', relative, 'Agent Runtime proof summaries must explicitly avoid counting external-engine success as native InfRing intelligence proof.');
  }
  if (!/does not prove native/i.test(clean(accounting.rule, 2000))) {
    push(violations, 'proof_summary_accounting_rule_too_weak', relative, accounting.rule);
  }

  const scope = summary.scope && typeof summary.scope === 'object' ? summary.scope : {};
  if (scope.raw_artifacts_local_only !== true) {
    push(violations, 'proof_summary_raw_artifact_policy_missing', relative, 'raw_artifacts_local_only must be true');
  }
  if (scope.live !== true) {
    push(violations, 'proof_summary_live_flag_missing', relative, 'live proof summaries must set scope.live=true');
  }
  if (scope.mode !== 'infring') {
    push(violations, 'proof_summary_mode_invalid', relative, scope.mode);
  }

  const sourceArtifacts = summary.source_artifacts && typeof summary.source_artifacts === 'object'
    ? Object.values(summary.source_artifacts)
    : [];
  for (const artifact of sourceArtifacts) {
    const artifactRef = clean(artifact, 1000);
    if (!artifactRef.startsWith('core/local/artifacts/')) {
      push(violations, 'proof_summary_source_artifact_not_local_ref', relative, artifactRef);
    }
  }

  const compactSummary = summary.summary && typeof summary.summary === 'object' ? summary.summary : {};
  if (Number(compactSummary.completed || 0) < 1) {
    push(violations, 'proof_summary_completed_count_missing', relative, `completed=${compactSummary.completed}`);
  }
  if (Number(compactSummary.failed || 0) !== 0) {
    push(violations, 'proof_summary_records_failed_run', relative, `failed=${compactSummary.failed}`);
  }
  const gapSummary = compactSummary.gap_summary && typeof compactSummary.gap_summary === 'object'
    ? compactSummary.gap_summary
    : {};
  if (Number(gapSummary.red || 0) !== 0) {
    push(violations, 'proof_summary_records_red_gap', relative, `red=${gapSummary.red}`);
  }

  const results = Array.isArray(summary.results) ? summary.results : [];
  if (!results.length) {
    push(violations, 'proof_summary_results_missing', relative, 'results must contain at least one framework proof row');
  } else if (Number(compactSummary.completed || 0) !== results.length) {
    push(violations, 'proof_summary_completed_count_mismatch', relative, `completed=${compactSummary.completed}; results=${results.length}`);
  }
  for (const row of results) {
    const frameworkId = clean(row && row.framework_id, 160) || 'unknown_framework';
    for (const field of [
      'task_id',
      'status',
      'projection_status',
      'semantic_signals',
      'approval_flow',
      'artifact_effect',
      'context_continuity',
      'parity',
    ]) {
      if (!clean(row && row[field], 500)) {
        push(violations, 'proof_summary_result_field_missing', relative, `${frameworkId}.${field}`);
      }
    }
    for (const field of [
      'approval_pause',
      'approval_decision_ok',
      'approval_resume_forwarded',
      'post_approval_resume_failure',
      'expected_artifacts_ok',
    ]) {
      if (typeof (row && row[field]) !== 'boolean') {
        push(violations, 'proof_summary_result_boolean_missing', relative, `${frameworkId}.${field}`);
      }
    }
    if (row && row.post_approval_resume_failure === true) {
      push(violations, 'proof_summary_result_post_resume_failure', relative, frameworkId);
    }
    if (row && row.status !== 'completed') {
      push(violations, 'proof_summary_result_status_not_completed', relative, `${frameworkId}:${row.status}`);
    }
    if (row && row.approval_pause !== true) {
      push(violations, 'proof_summary_result_approval_pause_missing', relative, frameworkId);
    }
    if (row && row.approval_decision_ok !== true) {
      push(violations, 'proof_summary_result_approval_decision_not_ok', relative, frameworkId);
    }
    if (row && row.approval_resume_forwarded !== true) {
      push(violations, 'proof_summary_result_approval_resume_not_forwarded', relative, frameworkId);
    }
    if (row && row.expected_artifacts_ok !== true) {
      push(violations, 'proof_summary_result_expected_artifact_not_ok', relative, frameworkId);
    }
    if (row && row.approval_flow !== 'pass') {
      push(violations, 'proof_summary_result_approval_flow_not_pass', relative, `${frameworkId}:${row.approval_flow}`);
    }
    if (row && row.artifact_effect !== 'pass') {
      push(violations, 'proof_summary_result_artifact_effect_not_pass', relative, `${frameworkId}:${row.artifact_effect}`);
    }
    if (row && row.context_continuity !== 'pass') {
      push(violations, 'proof_summary_result_context_continuity_not_pass', relative, `${frameworkId}:${row.context_continuity}`);
    }
    if (row && row.parity !== 'pass') {
      push(violations, 'proof_summary_result_parity_not_pass', relative, `${frameworkId}:${row.parity}`);
    }
  }

  const markdownPath = filePath.replace(/\.json$/i, '.md');
  if (!fs.existsSync(markdownPath)) {
    push(violations, 'proof_summary_markdown_pair_missing', relative, rel(markdownPath));
  } else if (fileSize(markdownPath) > MAX_SUMMARY_BYTES) {
    push(violations, 'proof_summary_markdown_too_large', relative, `${fileSize(markdownPath)} bytes > ${MAX_SUMMARY_BYTES} bytes`);
  }
}

function main() {
  const violations = [];
  if (!fs.existsSync(path.join(SUMMARY_DIR, 'README.md'))) {
    push(violations, 'proof_summary_readme_missing', rel(path.join(SUMMARY_DIR, 'README.md')));
  }
  let files = [];
  try {
    files = fs.readdirSync(SUMMARY_DIR)
      .filter((name) => /^agent_runtime_.*\.json$/.test(name))
      .map((name) => path.join(SUMMARY_DIR, name))
      .sort();
  } catch (error) {
    push(violations, 'proof_summary_directory_unreadable', rel(SUMMARY_DIR), String(error && error.message || error));
  }
  if (!files.length) {
    push(violations, 'proof_summary_json_missing', rel(SUMMARY_DIR), 'expected at least one agent_runtime_*.json proof summary');
  }
  for (const filePath of files) {
    validateSummary(filePath, readJson(filePath, violations), violations);
  }
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_proof_summary_guard',
    generated_at: new Date().toISOString(),
    summary_dir: rel(SUMMARY_DIR),
    summary_count: files.length,
    checked_summaries: files.map(rel),
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
