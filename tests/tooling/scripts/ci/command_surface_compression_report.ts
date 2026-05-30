#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/reports (command-surface anti-entropy report generator)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const registryPath = path.join(root, 'tools/commands/command_registry.json');
const outPath = path.join(root, 'validation/reports/command_surface_compression_report_2026-05-10.json');
const registry = JSON.parse(fs.readFileSync(registryPath, 'utf8'));
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const commands = Array.isArray(registry.entries) ? registry.entries : (Array.isArray(registry.commands) ? registry.commands : []);
const packageScriptIds = Object.keys(pkg.scripts || {});
const counts = { total: commands.length, package_scripts: packageScriptIds.length, hidden_package_script_backing_entries: Math.max(0, packageScriptIds.length - commands.length), by_domain: {}, by_work_gate: {}, by_lifecycle: {}, by_prefix: {} };
for (const command of commands) {
  const id = String(command.id || 'unknown');
  const domain = String(command.domain || 'unclassified');
  const workGate = String(command.work_gate || 'unclassified');
  const lifecycle = String(command.lifecycle || 'unclassified');
  const prefix = id.split(':')[0] || 'unknown';
  counts.by_domain[domain] = (counts.by_domain[domain] || 0) + 1;
  counts.by_work_gate[workGate] = (counts.by_work_gate[workGate] || 0) + 1;
  counts.by_lifecycle[lifecycle] = (counts.by_lifecycle[lifecycle] || 0) + 1;
  counts.by_prefix[prefix] = (counts.by_prefix[prefix] || 0) + 1;
}
const prefixRows = Object.entries(counts.by_prefix)
  .map(([prefix, count]) => ({ prefix, count }))
  .sort((a, b) => b.count - a.count || a.prefix.localeCompare(b.prefix));
const retirementCandidates = commands
  .filter((command) => String(command.lifecycle || '').match(/legacy|deprecated|compat/i) || String(command.work_gate || '').match(/advisory|manual|legacy/i))
  .slice(0, 100)
  .map((command) => ({ id: command.id, domain: command.domain, work_gate: command.work_gate, lifecycle: command.lifecycle, owner: command.owner || 'unassigned' }));
const hiddenBackingPrefixes = packageScriptIds
  .filter((id) => !commands.some((command) => command.id === id))
  .reduce((acc, id) => {
    const prefix = id.split(':')[0] || 'unknown';
    acc[prefix] = (acc[prefix] || 0) + 1;
    return acc;
  }, {});
const payload = {
  trace_id: `validation:${new Date().toISOString()}:${process.pid}`,
  span_id: `span:command_surface_compression:${process.pid}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'command_surface_compression_report',
  generated_at: new Date().toISOString(),
  registry_path: registryPath,
  registry_role: registry.registry_role || 'package_script_mirror',
  counts,
  largest_prefixes: prefixRows.slice(0, 30),
  largest_hidden_backing_prefixes: Object.entries(hiddenBackingPrefixes).map(([prefix, count]) => ({ prefix, count })).sort((a, b) => b.count - a.count || a.prefix.localeCompare(b.prefix)).slice(0, 30),
  retirement_candidate_count: retirementCandidates.length,
  retirement_candidates_sample: retirementCandidates.slice(0, 25),
  recommendations: [
    'Prefer command-runner-first entrypoints over direct package script expansion.',
    'Retire or hide advisory/manual/legacy commands from default operator surfaces.',
    'Collapse prefixes with overlapping ownership before adding new command families.',
    'Require owner, lifecycle, work_gate, and domain metadata on every new command.'
  ]
};
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: 'command_surface_compression_report', output_path: outPath, total: counts.total }, null, 2));
