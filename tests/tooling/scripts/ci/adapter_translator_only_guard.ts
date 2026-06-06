#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/conformance/contracts/adapter_translator_only_policy.json';
const gatewayPolicyPath = 'validation/conformance/contracts/gateway_adapter_invariants_policy.json';
const defaultOutJson = 'core/local/artifacts/adapter_translator_only_guard_current.json';
const defaultOutMarkdown = 'local/workspace/reports/ADAPTER_TRANSLATOR_ONLY_GUARD_CURRENT.md';

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

function walk(dir: string): string[] {
  const full = path.join(ROOT, dir);
  if (!fs.existsSync(full)) return [];
  const out: string[] = [];
  for (const name of fs.readdirSync(full)) {
    const rel = path.join(dir, name).replace(/\\/g, '/');
    const stat = fs.statSync(path.join(ROOT, rel));
    if (stat.isDirectory()) out.push(...walk(rel));
    else out.push(rel);
  }
  return out;
}

function matchesPattern(rel: string, pattern: string): boolean {
  const clean = String(pattern || '').replace(/\\/g, '/');
  if (!clean) return false;
  if (clean.endsWith('/**')) return rel === clean.slice(0, -3) || rel.startsWith(clean.slice(0, -2));
  return rel === clean;
}

function push(list: any[], kind: string, rel: string, detail: string, extra: Record<string, unknown> = {}): void {
  list.push({ kind, path: rel, detail, ...extra });
}

const violations: any[] = [];
const debt: any[] = [];
const policy = json(policyPath);
const gatewayPolicy = exists(gatewayPolicyPath) ? json(gatewayPolicyPath) : {};

if (policy.type !== 'adapter_translator_only_policy') {
  push(violations, 'wrong_policy_type', policyPath, 'Adapter translator-only policy must use type=adapter_translator_only_policy.');
}
if (!String(policy.canonical_rule || '').includes('adapters/** translates')) {
  push(violations, 'canonical_rule_missing_translation_clause', policyPath, 'Policy must explicitly state that adapters translate rather than own authority.');
}
if (policy.todo_id !== 'ADAPTER-TRANSLATOR-ONLY-GUARD') {
  push(violations, 'todo_id_missing', policyPath, 'Policy must reference ADAPTER-TRANSLATOR-ONLY-GUARD.');
}

const translatorRoots = Array.isArray(policy.translator_roots) ? policy.translator_roots : [];
const legacyRows = Array.isArray(policy.declared_legacy_non_translator_paths)
  ? policy.declared_legacy_non_translator_paths
  : [];
const shimRows = Array.isArray(gatewayPolicy.declared_legacy_compatibility_shims)
  ? gatewayPolicy.declared_legacy_compatibility_shims
  : [];
const shimPaths = new Set(shimRows.map((row: any) => String(row && row.path || '')).filter(Boolean));
const forbiddenSignals = Array.isArray(policy.forbidden_translator_signals)
  ? policy.forbidden_translator_signals
  : [];
const signalExemptions = Array.isArray(policy.signal_exemptions) ? policy.signal_exemptions : [];

for (const row of translatorRoots) {
  if (!String(row && row.path || '').startsWith('adapters/')) {
    push(violations, 'translator_root_outside_adapters', policyPath, `${row?.path || 'missing'} must be under adapters/**.`);
  }
  if (!String(row && row.role || '').trim()) {
    push(violations, 'translator_root_missing_role', policyPath, `${row?.path || 'missing'} must define a role.`);
  }
}

for (const row of legacyRows) {
  const rel = String(row && row.path || '');
  if (!rel) {
    push(violations, 'legacy_row_missing_path', policyPath, 'Every declared legacy adapter row needs a path.');
    continue;
  }
  if (!String(row.retirement_todo || '').trim()) push(violations, 'legacy_row_missing_retirement_todo', policyPath, rel);
  if (!String(row.allowed_until || '').trim()) push(violations, 'legacy_row_missing_allowed_until', policyPath, rel);
  if (!rel.endsWith('/**') && !exists(rel)) push(violations, 'legacy_file_missing', rel, 'Declared legacy adapter file does not exist.');
  debt.push({
    kind: 'declared_legacy_non_translator_adapter',
    path: rel,
    detail: `retirement_todo=${row.retirement_todo || 'missing'} allowed_until=${row.allowed_until || 'missing'}`,
  });
}

const adapterFiles = walk('adapters').filter((rel) => /\.(ts|js)$/.test(rel));
const classified = {
  translator: 0,
  legacy: 0,
  shim: 0,
};

for (const rel of adapterFiles) {
  const isShim = shimPaths.has(rel);
  const legacy = legacyRows.find((row: any) => matchesPattern(rel, String(row && row.path || '')));
  const translator = translatorRoots.find((row: any) => matchesPattern(rel, String(row && row.path || '')));
  if (isShim) {
    classified.shim += 1;
    debt.push({ kind: 'declared_gateway_compatibility_shim', path: rel, detail: 'Shim is governed by gateway_adapter_invariants_guard.' });
    continue;
  }
  if (legacy) {
    classified.legacy += 1;
    continue;
  }
  if (!translator) {
    push(violations, 'adapter_file_unclassified', rel, 'Adapter file must be classified as translator, declared legacy debt, or declared compatibility shim.');
    continue;
  }
  classified.translator += 1;
  const text = read(rel);
  for (const signal of forbiddenSignals) {
    const id = String(signal && signal.id || 'forbidden_signal');
    const pattern = String(signal && signal.pattern || '');
    if (!pattern) continue;
    const exempt = signalExemptions.some((row: any) =>
      String(row && row.signal_id || '') === id && matchesPattern(rel, String(row && row.path || '')),
    );
    if (exempt) continue;
    const re = new RegExp(pattern, 'i');
    if (re.test(text)) {
      push(
        violations,
        'translator_file_owns_forbidden_signal',
        rel,
        String(signal.detail || 'Translator file owns forbidden non-translator behavior.'),
        { signal_id: id },
      );
    }
  }
}

const pkg = exists('package.json') ? json('package.json') : {};
if (!String(pkg.scripts?.['ops:adapter:translator-only:guard'] || '').includes('adapter_translator_only_guard.ts')) {
  push(violations, 'package_script_missing', 'package.json', 'ops:adapter:translator-only:guard must execute adapter_translator_only_guard.ts.');
}

const gateRegistryPath = 'tests/tooling/config/tooling_gate_registry.json';
if (exists(gateRegistryPath)) {
  const gateRegistry = read(gateRegistryPath);
  if (!gateRegistry.includes('ops:adapter-translator-only:registered')) {
    push(violations, 'tooling_gate_registry_missing', gateRegistryPath, 'Guard must be registered in tooling_gate_registry.');
  }
} else {
  push(violations, 'tooling_gate_registry_missing_file', gateRegistryPath, 'Tooling gate registry is missing.');
}

const assurancePath = 'validation/conformance/contracts/assurance_validation_registry.json';
if (exists(assurancePath)) {
  const assurance = read(assurancePath);
  if (!assurance.includes('ops:adapter:translator-only:guard')) {
    push(violations, 'assurance_registry_missing_source_ref', assurancePath, 'Assurance registry must include adapter translator-only guard as boundary conformance evidence.');
  }
} else {
  push(violations, 'assurance_registry_missing_file', assurancePath, 'Assurance validation registry is missing.');
}

const todoPath = 'docs/workspace/todo/todo_registry.json';
const todoArchivePath = 'docs/workspace/todo/todo_archive_registry.json';
const todoText = `${exists(todoPath) ? read(todoPath) : ''}\n${exists(todoArchivePath) ? read(todoArchivePath) : ''}`;
if (!todoText.includes('ADAPTER-TRANSLATOR-ONLY-GUARD')) {
  push(violations, 'todo_missing', todoPath, 'TODO or TODO archive must retain ADAPTER-TRANSLATOR-ONLY-GUARD as the historical work item behind this guard.');
}

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'adapter_translator_only_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  summary: {
    adapter_files: adapterFiles.length,
    translator_files: classified.translator,
    declared_legacy_files: classified.legacy,
    declared_gateway_shims: classified.shim,
    violations: violations.length,
    debt: debt.length,
  },
  violations,
  debt,
};

const outJson = flag('out-json', defaultOutJson);
const outMarkdown = flag('out-markdown', defaultOutMarkdown);
fs.mkdirSync(path.dirname(path.join(ROOT, outJson)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outJson), `${JSON.stringify(payload, null, 2)}\n`);

const markdown = [
  '# Adapter Translator-Only Guard',
  '',
  `Generated: ${payload.generated_at}`,
  `Status: ${payload.ok ? 'PASS' : 'FAIL'}`,
  '',
  '## Summary',
  `- adapter_files: ${payload.summary.adapter_files}`,
  `- translator_files: ${payload.summary.translator_files}`,
  `- declared_legacy_files: ${payload.summary.declared_legacy_files}`,
  `- declared_gateway_shims: ${payload.summary.declared_gateway_shims}`,
  '',
  '## Violations',
  violations.length ? violations.map((row) => `- ${row.kind}: ${row.path} - ${row.detail}`).join('\n') : '- none',
  '',
  '## Declared Debt',
  debt.length ? debt.map((row) => `- ${row.kind}: ${row.path} - ${row.detail}`).join('\n') : '- none',
  '',
].join('\n');
fs.mkdirSync(path.dirname(path.join(ROOT, outMarkdown)), { recursive: true });
fs.writeFileSync(path.join(ROOT, outMarkdown), markdown);

console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
