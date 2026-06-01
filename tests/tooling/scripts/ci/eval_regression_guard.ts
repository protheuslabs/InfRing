#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type RequiredEvalArtifact = {
  id: string;
  path: string;
  required_ok: boolean;
};

const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_regression_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_regression_guard_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_REGRESSION_GUARD_CURRENT.md';

const REQUIRED_EVAL_ARTIFACTS: RequiredEvalArtifact[] = [
  {
    id: 'eval_agent_chat_monitor_guard',
    path: 'core/local/artifacts/eval_agent_chat_monitor_guard_current.json',
    required_ok: true,
  },
  {
    id: 'eval_monitor_slo',
    path: 'core/local/artifacts/eval_monitor_slo_current.json',
    required_ok: true,
  },
  {
    id: 'eval_quality_metrics',
    path: 'core/local/artifacts/eval_quality_metrics_current.json',
    required_ok: true,
  },
  {
    id: 'eval_reviewer_feedback_weekly',
    path: 'core/local/artifacts/eval_reviewer_feedback_weekly_current.json',
    required_ok: true,
  },
  {
    id: 'eval_judge_human_agreement',
    path: 'core/local/artifacts/eval_judge_human_agreement_current.json',
    required_ok: true,
  },
  {
    id: 'eval_adversarial_guard',
    path: 'core/local/artifacts/eval_adversarial_guard_current.json',
    required_ok: true,
  },
  {
    id: 'eval_issue_filing_guard',
    path: 'core/local/artifacts/eval_issue_filing_guard_current.json',
    required_ok: true,
  },
  {
    id: 'eval_issue_resolution',
    path: 'core/local/artifacts/eval_issue_resolution_current.json',
    required_ok: true,
  },
  {
    id: 'eval_quality_gate_v1',
    path: 'core/local/artifacts/eval_quality_gate_v1_current.json',
    required_ok: true,
  },
  {
    id: 'eval_runtime_authority_guard',
    path: 'core/local/artifacts/eval_runtime_authority_guard_current.json',
    required_ok: true,
  },
  {
    id: 'eval_autopilot_guard',
    path: 'core/local/artifacts/eval_autopilot_guard_current.json',
    required_ok: true,
  },
  {
    id: 'eval_feedback_router',
    path: 'core/local/artifacts/eval_feedback_router_current.json',
    required_ok: true,
  },
];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    artifactRoot: cleanText(readFlag(argv, 'artifact-root') || process.cwd(), 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(absPath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(absPath, 'utf8'));
  } catch {
    return null;
  }
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function isOkPayload(payload: any): boolean {
  if (!isPlainObject(payload)) return false;
  if (Object.prototype.hasOwnProperty.call(payload, 'ok')) return payload.ok === true;
  if (Object.prototype.hasOwnProperty.call(payload, 'pass')) return payload.pass === true;
  if (isPlainObject(payload.summary) && Object.prototype.hasOwnProperty.call(payload.summary, 'pass')) {
    return payload.summary.pass === true;
  }
  return true;
}

function isReleaseAcceptableReadinessBlock(row: RequiredEvalArtifact, payload: any): boolean {
  if (!isPlainObject(payload)) return false;
  if (row.id === 'eval_quality_gate_v1') {
    return (
      payload.summary?.quality_ok === true &&
      payload.summary?.quality_evaluation_mode === 'insufficient_signal' &&
      payload.summary?.rsi_promotion_blocked === true
    );
  }
  if (row.id === 'eval_autopilot_guard') {
    return payload.summary?.autopilot_ready === false;
  }
  return false;
}

function arrayLength(payload: any, key: string): number {
  return Array.isArray(payload?.[key]) ? payload[key].length : 0;
}

function artifactStatus(root: string, row: RequiredEvalArtifact) {
  const absPath = path.resolve(root, row.path);
  const exists = fs.existsSync(absPath);
  const payload = exists ? readJson(absPath) : null;
  const payloadObject = isPlainObject(payload);
  const ok = !row.required_ok || isOkPayload(payload) || isReleaseAcceptableReadinessBlock(row, payload);
  const thresholdViolations = payloadObject ? arrayLength(payload, 'threshold_violations') : 0;
  const regressionViolations = payloadObject ? arrayLength(payload, 'regression_violations') : 0;
  const failures = payloadObject ? arrayLength(payload, 'failures') : 0;
  const releaseReadinessBlockAccepted = payloadObject && payload?.ok === false && ok;
  const pass = exists && payloadObject && ok && thresholdViolations === 0 && regressionViolations === 0 && failures === 0;
  return {
    id: row.id,
    path: row.path,
    exists,
    payload_object: payloadObject,
    ok,
    threshold_violations: thresholdViolations,
    regression_violations: regressionViolations,
    failure_count: failures,
    release_readiness_block_accepted: releaseReadinessBlockAccepted,
    pass,
  };
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Regression Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- required_artifacts: ${Number(report.summary?.required_artifacts || 0)}`);
  lines.push(`- passing_artifacts: ${Number(report.summary?.passing_artifacts || 0)}`);
  lines.push(`- failing_artifacts: ${Number(report.summary?.failing_artifacts || 0)}`);
  lines.push('');
  lines.push('| artifact | exists | payload | ok | regressions | failures | pass |');
  lines.push('| --- | :---: | :---: | :---: | :---: | :---: | :---: |');
  for (const row of Array.isArray(report.artifacts) ? report.artifacts : []) {
    lines.push(
      `| ${cleanText(row.path || row.id || 'unknown', 220)} | ${row.exists ? 'yes' : 'no'} | ${
        row.payload_object ? 'yes' : 'no'
      } | ${row.ok ? 'yes' : 'no'} | ${Number(row.regression_violations || 0)} | ${
        Number(row.failure_count || 0)
      } | ${row.pass ? 'yes' : 'no'} |`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const artifactRoot = path.resolve(root, args.artifactRoot);
  const artifacts = REQUIRED_EVAL_ARTIFACTS.map((row) => artifactStatus(artifactRoot, row));
  const failures = artifacts
    .filter((row) => !row.pass)
    .map((row) => ({
      id: 'eval_release_artifact_not_passing',
      artifact: row.id,
      detail: `${row.path}: exists=${row.exists};payload_object=${row.payload_object};ok=${row.ok};threshold_violations=${row.threshold_violations};regression_violations=${row.regression_violations};failures=${row.failure_count}`,
    }));
  const pass = failures.length === 0;
  const report = {
    ok: pass,
    type: 'eval_regression_guard',
    generated_at: new Date().toISOString(),
    artifact_root: artifactRoot,
    summary: {
      required_artifacts: artifacts.length,
      passing_artifacts: artifacts.filter((row) => row.pass).length,
      failing_artifacts: failures.length,
      eval_release_gate: pass ? 'pass' : 'fail',
    },
    artifacts,
    failures,
    artifact_paths: [args.outPath, args.outLatestPath, args.markdownPath],
  };

  writeJsonArtifact(path.resolve(root, args.outLatestPath), report);
  writeTextArtifact(path.resolve(root, args.markdownPath), renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
