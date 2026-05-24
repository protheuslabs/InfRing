#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/reports (Layer capability trend report)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/layer_capability_trend_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const sourceReportPath = path.join(root, policy.source_report_path);
const changedFileReportPath = path.join(root, policy.changed_file_report_path);
const sampleStorePath = path.join(root, policy.sample_store_path);

function readJsonIfExists(filePath) {
  if (!fs.existsSync(filePath)) return null;
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function collectCounts(violations, prefix) {
  const counts = {
    total: 0,
    missing_ownership_header: 0,
    authority_logic_in_client: 0,
    forbidden_layer_concept_marker: 0,
    forbidden_layer_path_marker: 0,
  };
  for (const violation of Array.isArray(violations) ? violations : []) {
    if (prefix && (!violation.file || !String(violation.file).startsWith(prefix))) continue;
    counts.total += 1;
    if (counts[violation.type] !== undefined) counts[violation.type] += 1;
  }
  return counts;
}

function deriveSummary(sourceReport, changedFileReport) {
  const allViolations = Array.isArray(sourceReport?.violations) ? sourceReport.violations : [];
  const layer0Counts = collectCounts(allViolations, 'core/layer0/');
  const totalCounts = collectCounts(allViolations, '');
  const changedLayer0Counts = collectCounts(changedFileReport?.violations || [], 'core/layer0/');
  return {
    scanned_files: Number(sourceReport?.scanned_files || 0),
    total_violations: Number(sourceReport?.violations_count || 0),
    missing_ownership_headers: totalCounts.missing_ownership_header,
    client_authority_wrapper_violations: totalCounts.authority_logic_in_client,
    layer0_total_violations: layer0Counts.total,
    layer0_missing_ownership_headers: layer0Counts.missing_ownership_header,
    layer0_capability_path_debt: layer0Counts.forbidden_layer_path_marker,
    layer0_capability_concept_debt: layer0Counts.forbidden_layer_concept_marker,
    layer0_capability_debt_total:
      layer0Counts.forbidden_layer_path_marker + layer0Counts.forbidden_layer_concept_marker,
    changed_file_ok: Boolean(changedFileReport?.ok),
    changed_file_violations: Number(changedFileReport?.violations_count || 0),
    changed_file_layer0_capability_violations:
      changedLayer0Counts.forbidden_layer_path_marker + changedLayer0Counts.forbidden_layer_concept_marker,
  };
}

function readSamples() {
  if (!fs.existsSync(sampleStorePath)) return [];
  const out = [];
  for (const line of fs.readFileSync(sampleStorePath, 'utf8').split(/\r?\n/)) {
    if (!line.trim()) continue;
    try {
      out.push(JSON.parse(line));
    } catch {}
  }
  return out;
}

const sourceReport = readJsonIfExists(sourceReportPath);
const changedFileReport = readJsonIfExists(changedFileReportPath);
const summary = deriveSummary(sourceReport, changedFileReport);
const previousSamples = readSamples();
const previous = previousSamples.length ? previousSamples[previousSamples.length - 1] : null;
const bestHistoricalDebt = previousSamples.length
  ? previousSamples.reduce((best, row) => Math.min(best, Number(row.summary?.layer0_capability_debt_total || Infinity)), Infinity)
  : summary.layer0_capability_debt_total;
const deltaFromPrevious = previous
  ? summary.layer0_capability_debt_total - Number(previous.summary?.layer0_capability_debt_total || 0)
  : 0;
const deltaFromBest = summary.layer0_capability_debt_total - Number(bestHistoricalDebt || 0);

let status = 'bootstrap';
if (previous) {
  if (deltaFromPrevious < 0) status = 'improving';
  else if (deltaFromPrevious > 0) status = 'regressing';
  else status = 'flat';
}

const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const sample = {
  trace_id: traceId,
  generated_at: new Date().toISOString(),
  summary,
};

fs.mkdirSync(path.dirname(sampleStorePath), { recursive: true });
fs.appendFileSync(sampleStorePath, `${JSON.stringify(sample)}\n`);

const refreshedSamples = readSamples();
const recentSamples = refreshedSamples.slice(-Math.max(1, Number(policy.budgets?.max_recent_samples_in_report || 12)));
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'observability',
  type: 'layer_capability_trend_report',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  source_report_path: policy.source_report_path,
  changed_file_report_path: policy.changed_file_report_path,
  status,
  focus_metrics: Array.isArray(policy.focus_metrics) ? policy.focus_metrics : [],
  summary,
  delta: {
    layer0_capability_debt_from_previous: deltaFromPrevious,
    layer0_capability_debt_from_best: deltaFromBest,
  },
  best_historical_layer0_capability_debt: bestHistoricalDebt,
  previous_sample_generated_at: previous?.generated_at || null,
  recent_samples: recentSamples.map((row) => ({
    generated_at: row.generated_at || '',
    layer0_capability_debt_total: Number(row.summary?.layer0_capability_debt_total || 0),
    changed_file_layer0_capability_violations: Number(row.summary?.changed_file_layer0_capability_violations || 0),
  })),
  next_action:
    summary.changed_file_layer0_capability_violations > 0
      ? 'remove or relocate newly touched Layer 0 capability debt before merging'
      : status === 'regressing'
        ? 'reverse the latest increase in Layer 0 capability debt or formally reclassify the affected surfaces'
        : 'continue extracting Layer 0 capability debt upward and keep changed-file regressions at zero',
};

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(
  JSON.stringify(
    {
      ok: true,
      type: 'layer_capability_trend_report',
      report_path: policy.report_path,
      status,
      layer0_capability_debt_total: summary.layer0_capability_debt_total,
      changed_file_layer0_capability_violations: summary.changed_file_layer0_capability_violations,
    },
    null,
    2,
  ),
);
