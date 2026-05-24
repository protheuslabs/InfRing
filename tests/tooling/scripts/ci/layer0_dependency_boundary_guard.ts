#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();

type Policy = {
  schema_id?: string;
  schema_version?: string;
  layer0_root?: string;
  forbidden_runtime_dependency_names?: string[];
  allow_manifest_prefixes?: string[];
  out_json?: string;
  out_markdown?: string;
};

type Violation = {
  kind: string;
  manifest: string;
  dependency: string;
  section: string;
  detail: string;
};

function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}

function boolFlag(name: string, fallback = false): boolean {
  const raw = flag(name, fallback ? '1' : '0');
  return raw === '1' || raw === 'true';
}

function abs(rel: string): string {
  return path.join(ROOT, rel);
}

function ensureDir(rel: string): void {
  fs.mkdirSync(path.dirname(abs(rel)), { recursive: true });
}

function readJson(rel: string): Policy {
  return JSON.parse(fs.readFileSync(abs(rel), 'utf8')) as Policy;
}

function walkCargoTomls(rootRel: string): string[] {
  const out: string[] = [];
  const rootAbs = abs(rootRel);
  if (!fs.existsSync(rootAbs)) return out;
  const stack = [rootAbs];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.name === 'target' || entry.name === '.git' || entry.name === 'node_modules') continue;
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.isFile() && entry.name === 'Cargo.toml') {
        out.push(path.relative(ROOT, full).replace(/\\/g, '/'));
      }
    }
  }
  return out.sort();
}

function parseRuntimeDependencies(manifestText: string): Array<{ section: string; dependency: string }> {
  const rows: Array<{ section: string; dependency: string }> = [];
  let section = '';
  for (const rawLine of manifestText.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const sectionMatch = line.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      section = sectionMatch[1].trim();
      continue;
    }
    const isRuntimeSection =
      section === 'dependencies' ||
      (/^target\.[^.]+\.(dependencies)$/.test(section) || /^target\..+\.dependencies$/.test(section));
    if (!isRuntimeSection) continue;
    const depMatch = line.match(/^([A-Za-z0-9_.-]+)\s*=/);
    if (!depMatch) continue;
    rows.push({
      section,
      dependency: depMatch[1].trim(),
    });
  }
  return rows;
}

function toMarkdown(payload: any): string {
  const lines = [
    '# Layer0 Dependency Boundary Guard',
    '',
    `- ok: ${payload.ok}`,
    `- policy: ${payload.policy_path}`,
    `- scanned_manifests: ${payload.scanned_manifests}`,
    `- runtime_dependencies_scanned: ${payload.runtime_dependencies_scanned}`,
    `- violations: ${payload.violations.length}`,
    '',
  ];
  if (!payload.violations.length) {
    lines.push('- none');
  } else {
    for (const violation of payload.violations) {
      lines.push(
        `- ${violation.kind}: ${violation.manifest} [${violation.dependency}] (${violation.section}) - ${violation.detail}`,
      );
    }
  }
  return `${lines.join('\n')}\n`;
}

function main(): void {
  const strict = boolFlag('strict', true);
  const policyPath = flag('policy', 'tests/tooling/config/layer0_dependency_boundary_policy.json');
  const policy = readJson(policyPath);
  const outJson = flag('out-json', policy.out_json || 'core/local/artifacts/layer0_dependency_boundary_guard_current.json');
  const outMarkdown = flag('out-markdown', policy.out_markdown || 'local/workspace/reports/LAYER0_DEPENDENCY_BOUNDARY_GUARD_CURRENT.md');
  const layer0Root = String(policy.layer0_root || 'core/layer0');
  const forbidden = new Set(
    (Array.isArray(policy.forbidden_runtime_dependency_names) ? policy.forbidden_runtime_dependency_names : [])
      .map((value) => String(value).trim().toLowerCase())
      .filter(Boolean),
  );
  const allowPrefixes = (Array.isArray(policy.allow_manifest_prefixes) ? policy.allow_manifest_prefixes : [])
    .map((value) => String(value).trim())
    .filter(Boolean);

  const manifests = walkCargoTomls(layer0Root);
  const violations: Violation[] = [];
  let runtimeDependenciesScanned = 0;

  for (const manifest of manifests) {
    if (allowPrefixes.some((prefix) => manifest.startsWith(prefix))) continue;
    const text = fs.readFileSync(abs(manifest), 'utf8');
    for (const row of parseRuntimeDependencies(text)) {
      runtimeDependenciesScanned += 1;
      const dep = row.dependency.toLowerCase();
      if (!forbidden.has(dep)) continue;
      violations.push({
        kind: 'layer0_forbidden_runtime_dependency',
        manifest,
        dependency: row.dependency,
        section: row.section,
        detail:
          'Layer 0 should not take a direct runtime dependency on browser/web HTTP client crates; move this capability upward or hide it behind a higher-layer contract.',
      });
    }
  }

  const traceId = `observability:${new Date().toISOString()}:layer0-dependency-boundary`;
  const payload = {
    trace_id: traceId,
    span_id: `span:${traceId}`,
    parent_span_id: null,
    source_domain: 'observability',
    ok: violations.length === 0,
    type: 'layer0_dependency_boundary_guard',
    generated_at: new Date().toISOString(),
    policy_path: policyPath,
    layer0_root: layer0Root,
    scanned_manifests: manifests.length,
    runtime_dependencies_scanned: runtimeDependenciesScanned,
    forbidden_runtime_dependency_names: [...forbidden].sort(),
    violations,
  };

  ensureDir(outJson);
  fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMarkdown);
  fs.writeFileSync(abs(outMarkdown), toMarkdown(payload));
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !payload.ok) process.exit(1);
}

main();
