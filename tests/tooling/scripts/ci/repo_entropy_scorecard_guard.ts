import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;

const root = process.cwd();
const policyPath = path.join(root, 'validation/scorecards/repo_entropy_scorecard_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Json;
const reportRelPath = String(policy.report_path || 'core/local/artifacts/repo_entropy_scorecard_current.json');
const reportPath = path.join(root, reportRelPath);
const report = JSON.parse(fs.readFileSync(reportPath, 'utf8')) as Json;
const dimensions = Array.isArray(report.dimensions) ? report.dimensions as Json[] : [];
const violations: string[] = [];

for (const name of (policy.required_dimensions as string[]) || []) {
  if (!dimensions.some((row) => row.name === name)) violations.push(`missing_dimension_${name}`);
}
if (!report.trace_id) violations.push('missing_trace_id');
if (report.source_domain !== 'validation') violations.push('wrong_source_domain');
if (report.type !== 'repo_entropy_scorecard') violations.push('wrong_report_type');
if (!report.summary || typeof report.summary !== 'object') violations.push('missing_summary');
if (!Array.isArray(report.artifact_paths) || report.artifact_paths.length < 2) violations.push('missing_artifact_paths');

const requiredSummaryKeys = [
  'dirty_paths',
  'dirty_added',
  'dirty_modified',
  'dirty_deleted',
  'dirty_untracked',
  'npm_scripts',
  'command_entries',
  'compat_command_entries',
  'operator_surface_entries',
  'workflow_files',
  'required_ci_checks',
  'core_local_artifacts',
  'core_local_artifact_bytes',
  'artifact_cleanup_candidates',
  'effective_loc',
  'effective_loc_delta',
  'effective_loc_delta_pct',
  'guard_scripts',
  'gate_registry_entries',
  'duplicate_surface_roots',
  'tracked_generated_artifacts',
  'installer_monolith_bytes',
  'test_scaffold_debt',
  'test_lifecycle_overdue',
  'real_work_missing_ready_lanes',
  'real_work_missing_user_visible_lanes',
];
for (const key of requiredSummaryKeys) {
  if (typeof report.summary?.[key] !== 'number') violations.push(`missing_numeric_summary_${key}`);
}
const trackedMetrics = Array.isArray(report.tracked_metrics) ? report.tracked_metrics.map(String) : [];
if (trackedMetrics.length === 0) violations.push('missing_tracked_metrics');
for (const key of Array.isArray(policy.tracked_metrics) ? policy.tracked_metrics.map(String) : []) {
  if (!trackedMetrics.includes(key)) violations.push(`tracked_metric_missing_${key}`);
}
const trendDeltas = report.trend_deltas && typeof report.trend_deltas === 'object' ? report.trend_deltas as Json : {};
if (policy?.policy?.history_deltas_required === true && Object.keys(trendDeltas).length === 0) {
  violations.push('missing_trend_deltas');
}
for (const key of trackedMetrics) {
  const row = trendDeltas[key];
  if (!row || typeof row !== 'object') {
    violations.push(`missing_trend_delta_${key}`);
    continue;
  }
  if (typeof row.current !== 'number') violations.push(`trend_delta_missing_current_${key}`);
  if (typeof row.previous !== 'number') violations.push(`trend_delta_missing_previous_${key}`);
  if (typeof row.delta !== 'number') violations.push(`trend_delta_missing_delta_${key}`);
}
const topEntropyDrivers = Array.isArray(report.top_entropy_drivers) ? report.top_entropy_drivers as Json[] : [];
if (policy?.policy?.top_entropy_drivers_required === true && topEntropyDrivers.length === 0) {
  violations.push('missing_top_entropy_drivers');
}
for (const row of topEntropyDrivers) {
  if (!row.name || !row.metric_key || !row.severity || typeof row.value !== 'number') {
    violations.push(`top_entropy_driver_invalid_${row.name || 'unknown'}`);
  }
}

for (const row of dimensions) {
  if (!row.metric_key) violations.push(`dimension_missing_metric_key_${row.name || 'unknown'}`);
  if (typeof row.value !== 'number') violations.push(`dimension_missing_numeric_value_${row.name || 'unknown'}`);
  if (!['pass', 'white', 'yellow', 'red'].includes(String(row.severity))) violations.push(`dimension_invalid_severity_${row.name || 'unknown'}`);
  if ((row.severity === 'red' || row.severity === 'yellow') && (!Array.isArray(row.next_actions) || row.next_actions.length === 0)) {
    violations.push(`dimension_missing_next_actions_${row.name || 'unknown'}`);
  }
}

const redDimensions = Array.isArray(report.red_dimensions) ? report.red_dimensions : [];
if (redDimensions.length > 0 && !dimensions.some((row) => row.severity === 'red')) violations.push('red_dimensions_mismatch');
const artifactDimension = dimensions.find((row) => row.name === 'artifact_pressure');
if (
  policy?.policy?.artifact_pressure_must_include_retention_candidates === true &&
  typeof artifactDimension?.details?.artifact_cleanup_candidates !== 'number'
) {
  violations.push('artifact_pressure_missing_retention_candidates');
}
const worktreeDimension = dimensions.find((row) => row.name === 'worktree_churn');
if (
  policy?.policy?.dirty_churn_must_include_status_breakdown === true &&
  (typeof worktreeDimension?.details?.dirty_added !== 'number' ||
    typeof worktreeDimension?.details?.dirty_modified !== 'number' ||
    typeof worktreeDimension?.details?.dirty_deleted !== 'number' ||
    typeof worktreeDimension?.details?.dirty_untracked !== 'number')
) {
  violations.push('worktree_churn_missing_status_breakdown');
}
if (policy?.policy?.blob_pressure_must_cover_generated_installer_tests_and_real_work === true) {
  for (const name of ['generated_artifact_residue', 'installer_monolith', 'test_lifecycle_debt', 'real_work_gap']) {
    if (!dimensions.some((row) => row.name === name)) violations.push(`missing_blob_pressure_dimension_${name}`);
  }
}

const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:repo-entropy-scorecard-guard`;
const result = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'repo_entropy_scorecard_guard',
  generated_at: generatedAt,
  ok: violations.length === 0,
  policy_path: path.relative(root, policyPath).replace(/\\/g, '/'),
  report_path: reportRelPath,
  scorecard_severity: report.severity || null,
  entropy_score: report.entropy_score || 0,
  tracked_metrics: trackedMetrics,
  top_entropy_drivers: topEntropyDrivers,
  red_dimensions: report.red_dimensions || [],
  yellow_dimensions: report.yellow_dimensions || [],
  violation_count: violations.length,
  violations,
};

const out = path.join(root, 'core/local/artifacts/repo_entropy_scorecard_guard_current.json');
fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
