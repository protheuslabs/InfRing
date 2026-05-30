#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (command-runner-first anti-sprawl guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/command_runner_first_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const registry = JSON.parse(fs.readFileSync(path.join(root, policy.registry_path), 'utf8'));
const runner = fs.readFileSync(path.join(root, policy.runner_path), 'utf8');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const entries = Array.isArray(registry.entries) ? registry.entries : [];
const scripts = Object.keys(pkg.scripts || {});
const violations = [];
for (const token of policy.required_runner_capabilities || []) {
  if (!runner.includes(token)) violations.push({ kind: 'runner_missing_capability_token', token });
}
for (const entry of entries) {
  for (const field of policy.required_registry_metadata || []) {
    if (entry[field] === undefined || entry[field] === null || String(entry[field]).trim() === '') {
      violations.push({ kind: 'command_registry_entry_missing_metadata', id: entry.id || 'unknown', field });
    }
  }
}
if (policy.policy?.compact_operator_registry_is_canonical) {
  const operatorEntries = entries.filter((entry) => entry.operator_surface === true);
  const minimum = Number(policy.minimum_operator_registry_entries || 0);
  if (operatorEntries.length < minimum) {
    violations.push({ kind: 'operator_registry_below_minimum', operator_entries: operatorEntries.length, expected_minimum: minimum });
  }
  if (!runner.includes('packageScripts')) {
    violations.push({ kind: 'runner_missing_package_script_backing_lookup' });
  }
} else if (entries.length < scripts.length) {
  violations.push({ kind: 'registry_smaller_than_package_scripts', registry_entries: entries.length, package_scripts: scripts.length });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'command_runner_first_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  registry_entries: entries.length,
  registry_role: policy.policy?.compact_operator_registry_is_canonical ? 'compact_operator_surface' : 'package_script_mirror',
  package_scripts: scripts.length,
  violations
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/command_runner_first_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
