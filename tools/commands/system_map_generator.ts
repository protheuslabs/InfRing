#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

type Entry = {
  id: string;
  subsystem: string;
  layer: string;
  owner: string;
  purpose: string;
  inbound: string[];
  outbound: string[];
  failure_mode: string;
  health_check: string;
  srs: string[];
};

type Policy = {
  registry_path: string;
  outputs: {
    markdown_path: string;
    latest_path: string;
    history_path: string;
  };
};

const ROOT = process.cwd();
const POLICY_PATH = 'client/runtime/config/system_map_generator_policy.json';
const REQUIRED_SUBSYSTEMS = [
  'orchestration_control_plane',
  'gateway_membrane',
  'shell_projection',
  'validation_domain',
  'observability_domain',
  'governance_domain',
  'kernel_sentinel',
  'layer3_os_personality',
];

function readJson<T>(rel: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, rel), 'utf8')) as T;
}

function writeText(rel: string, text: string): void {
  const target = path.resolve(ROOT, rel);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, text, 'utf8');
}

function appendText(rel: string, text: string): void {
  const target = path.resolve(ROOT, rel);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.appendFileSync(target, text, 'utf8');
}

function normalizeOwner(owner: string): string {
  return owner
    .replace(/\bcore\b/g, 'kernel')
    .replace(/\bclient\b/g, 'shell');
}

function normalizeLayer(layer: string): string {
  return layer
    .replace(/^Client Ops$/, 'Shell Ops')
    .replace(/^Client Cognition$/, 'Shell Cognition')
    .replace(/^Cross-Plane$/, 'Conduit');
}

function normalizeText(value: string): string {
  return value
    .replace(/\bclient surfaces\b/gi, 'shell surfaces')
    .replace(/\bclient lane\b/gi, 'shell lane')
    .replace(/\bcore authority\b/gi, 'Kernel authority')
    .replace(/\bCore\b/g, 'Kernel');
}

function normalizeEntry(entry: Entry): Entry {
  return {
    ...entry,
    subsystem: normalizeText(entry.subsystem),
    layer: normalizeLayer(entry.layer),
    owner: normalizeOwner(entry.owner),
    purpose: normalizeText(entry.purpose),
    inbound: entry.inbound.map(normalizeText),
    outbound: entry.outbound.map(normalizeText),
  };
}

function validateEntry(entry: Entry): string[] {
  const missing = ['id', 'subsystem', 'layer', 'owner', 'purpose', 'failure_mode', 'health_check']
    .filter((field) => !String((entry as any)[field] || '').trim());
  if (!Array.isArray(entry.inbound) || entry.inbound.length === 0) missing.push('inbound');
  if (!Array.isArray(entry.outbound) || entry.outbound.length === 0) missing.push('outbound');
  if (!Array.isArray(entry.srs) || entry.srs.length === 0) missing.push('srs');
  const violations = missing.map((field) => `${entry.id || '<missing-id>'}: missing ${field}`);
  const staleValues = [entry.layer, entry.owner, entry.purpose, ...entry.inbound, ...entry.outbound];
  for (const value of staleValues) {
    if (/\bClient (?:Ops|Cognition)\b|\bclient surfaces\b|\bclient lane\b|\bcore authority\b/i.test(value)) {
      violations.push(`${entry.id || '<missing-id>'}: stale system-map ownership label "${value}"`);
    }
  }
  if (entry.owner === 'client' || entry.owner.includes('+client')) {
    violations.push(`${entry.id || '<missing-id>'}: owner must use canonical shell/kernel terms, not client`);
  }
  return violations;
}

function validate(entries: Entry[]): string[] {
  const violations = entries.flatMap(validateEntry);
  const ids = new Set<string>();
  for (const entry of entries) {
    if (ids.has(entry.id)) violations.push(`${entry.id}: duplicate id`);
    ids.add(entry.id);
  }
  for (const required of REQUIRED_SUBSYSTEMS) {
    if (!ids.has(required)) violations.push(`${required}: required current-domain subsystem missing`);
  }
  return violations;
}

function layerCounts(entries: Entry[]): Record<string, number> {
  return entries.reduce((acc, entry) => {
    acc[entry.layer] = (acc[entry.layer] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);
}

function mdList(values: string[]): string {
  return values.map((value) => normalizeText(value)).join('; ');
}

function renderMarkdown(entries: Entry[], generatedAt: string, registryPath: string): string {
  const normalized = entries.map(normalizeEntry);
  const counts = Object.entries(layerCounts(normalized)).sort(([a], [b]) => a.localeCompare(b));
  const lines = [
    '# System Map',
    '',
    `Generated: ${generatedAt}`,
    '',
    `This map is generated from \`${registryPath}\` via \`system_map_generator\` and is the canonical quick-reference for subsystem purpose, ownership, and health checks.`,
    '',
    '## Layer Coverage',
    '',
    '| Layer | Subsystems |',
    '|---|---:|',
    ...counts.map(([layer, count]) => `| ${layer} | ${count} |`),
    '',
    '## Subsystem Map',
    '',
    '| Subsystem | Layer | Purpose | Owner | Inputs | Outputs | Failure Mode | Health Check | SRS |',
    '|---|---|---|---|---|---|---|---|---|',
    ...normalized.map((entry) => [
      entry.subsystem,
      entry.layer,
      entry.purpose,
      entry.owner,
      mdList(entry.inbound),
      mdList(entry.outbound),
      entry.failure_mode,
      `\`${entry.health_check}\``,
      entry.srs.map((srs) => `\`${srs}\``).join(', '),
    ].join(' | ')).map((row) => `| ${row} |`),
    '',
  ];
  return lines.join('\n');
}

function loadPolicy(): Policy {
  return readJson<Policy>(POLICY_PATH);
}

function readFlag(name: string): string {
  const prefix = `--${name}=`;
  const arg = process.argv.find((value) => value.startsWith(prefix));
  return arg ? arg.slice(prefix.length) : '';
}

function run(apply: boolean): number {
  const policy = loadPolicy();
  const registryPath = readFlag('registry-path') || policy.registry_path;
  const registry = readJson<{ entries: Entry[] }>(registryPath);
  const entries = registry.entries || [];
  const violations = validate(entries);
  const generatedAt = new Date().toISOString();
  const normalized = entries.map(normalizeEntry);
  const payload = {
    ok: violations.length === 0,
    type: 'system_map_generator',
    generated_at: generatedAt,
    registry_path: registryPath,
    markdown_path: policy.outputs.markdown_path,
    entries: normalized.length,
    layer_counts: layerCounts(normalized),
    violations,
  };
  if (apply) {
    writeText(policy.outputs.markdown_path, renderMarkdown(entries, generatedAt, registryPath));
    writeText(policy.outputs.latest_path, `${JSON.stringify(payload, null, 2)}\n`);
    appendText(policy.outputs.history_path, `${JSON.stringify(payload)}\n`);
  }
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  return payload.ok ? 0 : 1;
}

function status(): number {
  const policy = loadPolicy();
  const latestPath = path.resolve(ROOT, policy.outputs.latest_path);
  if (!fs.existsSync(latestPath)) {
    process.stdout.write(`${JSON.stringify({ ok: false, type: 'system_map_status', reason: 'missing_latest' }, null, 2)}\n`);
    return 1;
  }
  const latest = readJson<Record<string, unknown>>(policy.outputs.latest_path);
  process.stdout.write(`${JSON.stringify({ ok: Boolean(latest.ok), type: 'system_map_status', latest }, null, 2)}\n`);
  return latest.ok ? 0 : 1;
}

function main(): number {
  const command = process.argv[2] || 'status';
  if (command === 'run') return run(process.argv.includes('--apply=1'));
  if (command === 'status') return status();
  process.stderr.write('Usage: system_map_generator.ts run [--apply=1] | status\n');
  return 2;
}

process.exit(main());
