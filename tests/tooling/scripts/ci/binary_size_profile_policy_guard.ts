#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/conformance/contracts/binary_size_profile_policy.json';
const defaultOutJson = 'core/local/artifacts/binary_size_profile_policy_guard_current.json';

function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const index = process.argv.indexOf(`--${name}`);
  if (index >= 0 && index + 1 < process.argv.length) return process.argv[index + 1];
  return fallback;
}

function read(rel: string): string {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function exists(rel: string): boolean {
  return fs.existsSync(path.join(ROOT, rel));
}

function json(rel: string): any {
  return JSON.parse(read(rel));
}

function push(list: any[], kind: string, rel: string, detail: string): void {
  list.push({ kind, path: rel, detail });
}

const violations: any[] = [];

if (!exists(policyPath)) {
  push(violations, 'policy_missing', policyPath, 'Binary size profile policy must exist.');
}

const policy = exists(policyPath) ? json(policyPath) : {};
const profiles = Array.isArray(policy.profiles) ? policy.profiles : [];
const richHost = profiles.find((row: any) => row && row.id === 'rich_host_static');
const tinyPure = profiles.find((row: any) => row && row.id === 'tiny_pure_edge');

if (policy.type !== 'binary_size_profile_policy') {
  push(violations, 'wrong_policy_type', policyPath, 'Policy must use type=binary_size_profile_policy.');
}
if (!richHost) {
  push(violations, 'rich_host_profile_missing', policyPath, 'Policy must declare rich_host_static profile.');
} else {
  if (Number(richHost.max_mb) !== 35) {
    push(violations, 'rich_host_max_not_35_mb', policyPath, 'rich_host_static.max_mb must document the current 35 MB gate.');
  }
  if (!Array.isArray(richHost.not_claimed_as) || !richHost.not_claimed_as.includes('tiny_pure_edge')) {
    push(violations, 'rich_host_not_claimed_scope_missing', policyPath, 'rich_host_static must explicitly say it is not tiny_pure_edge proof.');
  }
}
if (!tinyPure) {
  push(violations, 'tiny_pure_profile_missing', policyPath, 'Policy must declare tiny_pure_edge profile.');
} else {
  if (tinyPure.satisfied_by_rich_host_gate !== false) {
    push(violations, 'tiny_pure_satisfied_by_rich_gate', policyPath, 'tiny_pure_edge must not be satisfied by the rich host 35 MB gate.');
  }
  if (!Array.isArray(tinyPure.required_future_evidence) || tinyPure.required_future_evidence.length < 5) {
    push(violations, 'tiny_pure_future_evidence_too_weak', policyPath, 'tiny_pure_edge must require separate evidence before promotion.');
  }
}

const workflowChecks = [
  {
    path: '.github/workflows/infringd-static-size-gate.yml',
    needles: [
      'Enforce rich-host static size gate (0.5 MB - 35 MB)',
      'rich-host 0.5 MB - 35 MB CI gate',
      'not the tiny/pure edge profile gate',
    ],
  },
  {
    path: '.github/workflows/release.yml',
    needles: [
      'Enforce rich-host static infringd size gate (35 MB)',
      'rich-host 35 MB CI gate',
      'infringd rich-host static binary',
    ],
  },
  {
    path: '.github/workflows/release-security-artifacts.yml',
    needles: [
      'rich-host 35 MB release gate',
      'not tiny/pure edge proof',
    ],
  },
];

for (const check of workflowChecks) {
  if (!exists(check.path)) {
    push(violations, 'workflow_missing', check.path, 'Referenced size workflow is missing.');
    continue;
  }
  const text = read(check.path);
  for (const needle of check.needles) {
    if (!text.includes(needle)) {
      push(violations, 'workflow_missing_size_profile_language', check.path, `Missing required language: ${needle}`);
    }
  }
}

const releasePolicyGatePath = 'tests/tooling/scripts/ci/release_policy_gate.ts';
if (exists(releasePolicyGatePath)) {
  const releasePolicyGate = read(releasePolicyGatePath);
  if (!releasePolicyGate.includes('Enforce rich-host static infringd size gate (35 MB)')) {
    push(violations, 'release_policy_gate_missing_rich_host_needle', releasePolicyGatePath, 'Release policy guard must check the rich-host size-gate label.');
  }
} else {
  push(violations, 'release_policy_gate_missing', releasePolicyGatePath, 'Release policy gate script is missing.');
}

const pkgPath = 'package.json';
if (exists(pkgPath)) {
  const pkg = json(pkgPath);
  if (!String(pkg.scripts?.['ops:binary-size-profile:guard'] || '').includes('binary_size_profile_policy_guard.ts')) {
    push(violations, 'package_script_missing', pkgPath, 'ops:binary-size-profile:guard must execute binary_size_profile_policy_guard.ts.');
  }
} else {
  push(violations, 'package_missing', pkgPath, 'package.json is missing.');
}

const payload = {
  trace_id: `validation:${new Date().toISOString()}:${process.pid}`,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'binary_size_profile_policy_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  summary: {
    profile_count: profiles.length,
    rich_host_max_mb: richHost?.max_mb ?? null,
    tiny_pure_satisfied_by_rich_host_gate: tinyPure?.satisfied_by_rich_host_gate ?? null,
    checked_workflows: workflowChecks.length,
    violations: violations.length,
  },
  violations,
};

const outJson = flag('out-json', defaultOutJson);
fs.mkdirSync(path.dirname(path.join(ROOT, outJson)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outJson), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
