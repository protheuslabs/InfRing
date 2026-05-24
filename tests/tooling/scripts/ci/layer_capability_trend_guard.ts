#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/sentinel (Layer capability trend guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/layer_capability_trend_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];

if (!report) violations.push({ kind: 'layer_capability_trend_report_missing', path: policy.report_path });
if (report && Buffer.byteLength(JSON.stringify(report), 'utf8') > Number(policy.budgets?.max_report_bytes || 131072)) {
  violations.push({
    kind: 'layer_capability_trend_report_too_large',
    bytes: Buffer.byteLength(JSON.stringify(report), 'utf8'),
    max: Number(policy.budgets?.max_report_bytes || 131072),
  });
}
if (report && !['bootstrap', 'improving', 'flat', 'regressing'].includes(report.status)) {
  violations.push({ kind: 'layer_capability_trend_status_invalid', status: report.status });
}
if (report && report.status === 'regressing') {
  violations.push({
    kind: 'layer_capability_trend_regressing',
    actual: Number(report.summary?.layer0_capability_debt_total || 0),
    delta_from_previous: Number(report.delta?.layer0_capability_debt_from_previous || 0),
  });
}
if (report && Number(report.summary?.changed_file_layer0_capability_violations || 0) > 0) {
  violations.push({
    kind: 'layer_capability_changed_file_regression_present',
    actual: Number(report.summary?.changed_file_layer0_capability_violations || 0),
  });
}

const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'observability',
  ok: violations.length === 0,
  type: 'layer_capability_trend_guard',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  report_path: policy.report_path,
  violations,
};

const outPath = path.join(root, 'core/local/artifacts/layer_capability_trend_guard_current.json');
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
