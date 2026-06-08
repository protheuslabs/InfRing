#!/usr/bin/env node
/* eslint-disable no-console */
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const POLICY_PATH = 'validation/conformance/contracts/proof_ledger_separation_policy.json';
const OUT_JSON = 'core/local/artifacts/proof_ledger_separation_guard_current.json';

function rel(filePath) {
  return filePath.replace(/\\/g, '/');
}

function read(filePath) {
  return fs.readFileSync(path.join(ROOT, filePath), 'utf8');
}

function exists(filePath) {
  return fs.existsSync(path.join(ROOT, filePath));
}

function json(filePath) {
  return JSON.parse(read(filePath));
}

function push(violations, kind, file, detail) {
  violations.push({
    kind,
    file: rel(file),
    detail: String(detail == null ? '' : detail).replace(/\s+/g, ' ').trim().slice(0, 2000),
  });
}

function includesAll(violations, file, needles, kind) {
  if (!exists(file)) {
    push(violations, `${kind}_file_missing`, file, 'required file is missing');
    return;
  }
  const text = read(file);
  for (const needle of needles) {
    if (!text.includes(needle)) {
      push(violations, kind, file, `missing required text: ${needle}`);
    }
  }
}

const violations = [];
const policy = exists(POLICY_PATH) ? json(POLICY_PATH) : {};

if (!exists(POLICY_PATH)) {
  push(violations, 'policy_missing', POLICY_PATH, 'proof ledger separation policy must exist');
}
if (policy.type !== 'proof_ledger_separation_policy') {
  push(violations, 'policy_type_invalid', POLICY_PATH, policy.type);
}

const ledgers = Array.isArray(policy.ledgers) ? policy.ledgers : [];
const substrate = ledgers.find((row) => row && row.id === 'agent_runtime_substrate_platform');
const native = ledgers.find((row) => row && row.id === 'infring_native_intelligence');
if (!substrate || substrate.classification !== 'substrate_platform_proof') {
  push(violations, 'substrate_ledger_missing', POLICY_PATH, 'agent_runtime_substrate_platform must classify as substrate_platform_proof');
}
if (!native || native.classification !== 'native_intelligence_runtime_proof') {
  push(violations, 'native_ledger_missing', POLICY_PATH, 'infring_native_intelligence must classify as native_intelligence_runtime_proof');
}
if (!Array.isArray(policy.invariants) || !policy.invariants.join('\n').includes('External-engine mediated success is substrate/platform proof')) {
  push(violations, 'ledger_invariant_missing', POLICY_PATH, 'policy must state the external-engine/native proof boundary');
}

for (const source of Array.isArray(policy.required_sources) ? policy.required_sources : []) {
  if (!exists(source)) push(violations, 'required_source_missing', source, 'required proof-accounting source is missing');
}

const harness = exists('validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json')
  ? json('validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json')
  : {};
const accounting = harness.proof_accounting_rule || {};
if (accounting.external_engine_success !== 'substrate_platform_proof') {
  push(violations, 'task_harness_external_accounting_invalid', 'validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json', accounting.external_engine_success);
}
if (accounting.infring_native_success !== 'native_intelligence_runtime_proof') {
  push(violations, 'task_harness_native_accounting_invalid', 'validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json', accounting.infring_native_success);
}
if (accounting.must_not_conflate_ledgers !== true) {
  push(violations, 'task_harness_conflation_guard_missing', 'validation/agent_runtime/task_harness/agent_runtime_task_harness_contract.json', 'must_not_conflate_ledgers must be true');
}

includesAll(violations, 'validation/agent_runtime/task_harness/README.md', [
  'External-engine success is substrate proof, not native InfRing intelligence proof.',
  'Native InfRing promotion targets must remain on a separate scoreboard.',
], 'task_harness_readme_boundary_missing');

includesAll(violations, 'validation/agent_runtime/proof_summaries/README.md', [
  'substrate/platform proof',
  'not native InfRing intelligence proof',
], 'proof_summary_readme_boundary_missing');

includesAll(violations, 'tests/tooling/scripts/ci/agent_runtime_proof_summary_guard.ts', [
  "accounting.classification !== 'substrate_platform_proof'",
  'not_native_intelligence_proof',
  'does not prove native',
], 'proof_summary_guard_boundary_missing');

includesAll(violations, 'docs/workspace/native_coding_useful_work_eval_v1.md', [
  'Infring-native',
  'Default promotion target: `19/20` passes.',
  'Codex subagent runs can validate prompts and judges, but they are not capability evidence for Infring-native coding.',
], 'native_coding_eval_boundary_missing');

includesAll(violations, 'docs/workspace/coding_workflow_eval_results.md', [
  'Promotion target: `19/20` Infring-native passes.',
  'Codex subagent runs may validate prompt and judge shape only; they do not prove native Infring coding capability.',
], 'native_coding_scoreboard_boundary_missing');

const fixture = exists('validation/evals/fixtures/native_coding_useful_work_eval_v1.json')
  ? json('validation/evals/fixtures/native_coding_useful_work_eval_v1.json')
  : {};
const target = fixture.promotion_target || {};
if (Number(target.attempts) !== 20 || Number(target.required_pass_count) !== 19) {
  push(violations, 'native_fixture_promotion_target_invalid', 'validation/evals/fixtures/native_coding_useful_work_eval_v1.json', JSON.stringify(target));
}
if (target.worker_runtime !== 'infring_native_agent_only') {
  push(violations, 'native_fixture_worker_runtime_invalid', 'validation/evals/fixtures/native_coding_useful_work_eval_v1.json', target.worker_runtime);
}

const summaryDir = path.join(ROOT, 'validation/agent_runtime/proof_summaries');
const proofSummaryFiles = fs.existsSync(summaryDir)
  ? fs.readdirSync(summaryDir).filter((name) => name.endsWith('.json')).map((name) => `validation/agent_runtime/proof_summaries/${name}`)
  : [];
for (const file of proofSummaryFiles) {
  const summary = json(file);
  const proofAccounting = summary.proof_accounting || {};
  if (proofAccounting.classification !== 'substrate_platform_proof') {
    push(violations, 'proof_summary_classification_invalid', file, proofAccounting.classification);
  }
  if (proofAccounting.not_native_intelligence_proof !== true) {
    push(violations, 'proof_summary_not_native_flag_missing', file, 'not_native_intelligence_proof must be true');
  }
}

const pkg = exists('package.json') ? json('package.json') : {};
if (!String(pkg.scripts && pkg.scripts['ops:proof-ledger-separation:guard'] || '').includes('proof_ledger_separation_guard.ts')) {
  push(violations, 'package_script_missing', 'package.json', 'ops:proof-ledger-separation:guard must execute proof_ledger_separation_guard.ts');
}

const coordination = exists('tests/tooling/scripts/ci/agent_runtime_framework_coordination_guard.ts')
  ? read('tests/tooling/scripts/ci/agent_runtime_framework_coordination_guard.ts')
  : '';
if (!coordination.includes('proof_ledger_separation_guard.ts')) {
  push(violations, 'coordination_guard_missing', 'tests/tooling/scripts/ci/agent_runtime_framework_coordination_guard.ts', 'framework coordination guard must include proof ledger separation');
}

const payload = {
  trace_id: `validation:${new Date().toISOString()}:${process.pid}`,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'proof_ledger_separation_guard',
  generated_at: new Date().toISOString(),
  summary: {
    required_sources: Array.isArray(policy.required_sources) ? policy.required_sources.length : 0,
    proof_summary_files: proofSummaryFiles.length,
    substrate_ledger_declared: Boolean(substrate),
    native_ledger_declared: Boolean(native),
    violations: violations.length,
  },
  violations,
};

fs.mkdirSync(path.dirname(path.join(ROOT, OUT_JSON)), { recursive: true });
fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
