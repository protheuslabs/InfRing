#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/dependency_boundary_manifest.json');

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

type LayerRuleViolation = {
  file: string;
  source_layer: string;
  spec: string;
  resolved: string;
  target_layer: string;
};

type ConduitViolation = {
  file: string;
  forbidden_pattern: string;
};

type MissingImport = {
  file: string;
  spec: string;
};

function nowIso(): string {
  return new Date().toISOString();
}

function toText(value: unknown, max = 400): string {
  return String(value ?? '').trim().slice(0, max);
}

function parseArgs(argv: string[]): Record<string, unknown> {
  const out: Record<string, unknown> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) {
      (out._ as string[]).push(tok);
      continue;
    }
    const eq = tok.indexOf('=');
    if (eq >= 0) {
      out[tok.slice(2, eq)] = tok.slice(eq + 1);
      continue;
    }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function isCanonicalRelativePath(value: string): boolean {
  if (!value) return false;
  if (value.startsWith('/') || value.startsWith('\\')) return false;
  if (value.includes('..') || value.includes('\\') || value.includes('//')) return false;
  return /^[A-Za-z0-9._/\-]+$/.test(value);
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return value.toLowerCase().endsWith(suffix.toLowerCase());
}

function isCanonicalToken(value: string): boolean {
  return /^[a-z0-9][a-z0-9_]*$/.test(value);
}

function readJson(filePath: string, fallback: unknown): unknown {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function ensureDir(filePath: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function writeJson(filePath: string, value: unknown): void {
  ensureDir(filePath);
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath: string, value: unknown): void {
  ensureDir(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function listFiles(baseDir: string, extSet: Set<string>, excludeContains: string[]): string[] {
  if (!fs.existsSync(baseDir)) return [];
  const out: string[] = [];
  const stack: string[] = [baseDir];
  while (stack.length) {
    const cur = stack.pop() as string;
    for (const ent of fs.readdirSync(cur, { withFileTypes: true })) {
      const abs = path.join(cur, ent.name);
      const relPath = rel(abs);
      if (excludeContains.some((token) => relPath.includes(token))) continue;
      if (ent.isDirectory()) {
        stack.push(abs);
      } else if (ent.isFile() && extSet.has(path.extname(abs))) {
        out.push(abs);
      }
    }
  }
  return out.sort();
}

function detectLayer(relPath: string, layers: Record<string, unknown>): string | null {
  for (const [layer, roots] of Object.entries(layers || {})) {
    for (const root of (Array.isArray(roots) ? roots : [])) {
      const normalized = String(root).replace(/\\/g, '/').replace(/\/+$/, '');
      if (relPath === normalized || relPath.startsWith(`${normalized}/`)) return layer;
    }
  }
  return null;
}

function parseSpecs(source: string): string[] {
  const specs: string[] = [];
  const regex = /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = regex.exec(source)) != null) {
    specs.push(match[1]);
  }
  return specs;
}

function resolveLocalSpec(fromFile: string, spec: string): string | null {
  const base = path.resolve(path.dirname(fromFile), spec);
  const stemCandidates: string[] = [];
  const ext = path.extname(base);
  if (ext === '.js' || ext === '.mjs' || ext === '.cjs') {
    stemCandidates.push(base.slice(0, -ext.length));
  }
  const candidates: string[] = [
    base,
    `${base}.ts`,
    `${base}.js`,
    `${base}.mjs`,
    path.join(base, 'index.ts'),
    path.join(base, 'index.js'),
    path.join(base, 'index.mjs'),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
  }
  for (const stem of stemCandidates) {
    const stemChecks = [
      stem,
      `${stem}.ts`,
      `${stem}.js`,
      `${stem}.mjs`,
      path.join(stem, 'index.ts'),
      path.join(stem, 'index.js'),
      path.join(stem, 'index.mjs'),
    ];
    for (const candidate of stemChecks) {
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
    }
  }
  return null;
}

function isVirtualGeneratedLocalImport(spec: string): boolean {
  return spec === './$types' || spec.endsWith('/$types');
}

export function run(rawArgs: Record<string, unknown> = {}): Record<string, unknown> {
  const strictRaw = String(rawArgs.strict ?? '0').toLowerCase();
  const strict = strictRaw === '1' || strictRaw === 'true' || strictRaw === 'yes';
  const checks: CheckRow[] = [];

  const policyRaw = readJson(POLICY_PATH, null);
  const policy = isPlainObject(policyRaw) ? policyRaw : {};
  const policyRel = rel(POLICY_PATH);
  checks.push({
    id: 'dependency_boundary_policy_path_canonical_contract',
    ok: isCanonicalRelativePath(policyRel),
    detail: policyRel,
  });
  checks.push({
    id: 'dependency_boundary_policy_exists_contract',
    ok: fs.existsSync(POLICY_PATH),
    detail: policyRel,
  });
  checks.push({
    id: 'dependency_boundary_policy_payload_object_contract',
    ok: isPlainObject(policyRaw),
    detail: `type=${Array.isArray(policyRaw) ? 'array' : typeof policyRaw}`,
  });
  checks.push({
    id: 'dependency_boundary_policy_version_present_contract',
    ok: typeof policy.version === 'string' && toText(policy.version, 120).length > 0,
    detail: String(policy.version || ''),
  });

  const scan = isPlainObject(policy.scan) ? policy.scan : {};
  const includeDirs = Array.isArray(scan.include_dirs) ? scan.include_dirs.map((v) => String(v)) : [];
  const includeExt = new Set((Array.isArray(scan.include_ext) ? scan.include_ext : ['.ts', '.js']).map((v) => String(v)));
  const excludeContains = Array.isArray(scan.exclude_contains) ? scan.exclude_contains.map((v) => String(v)) : [];
  const includeDirsUnique = new Set(includeDirs).size === includeDirs.length;
  const includeDirsCanonical = includeDirs.every((dir) => isCanonicalRelativePath(dir));
  const includeExtList = Array.from(includeExt.values());
  const includeExtCanonical = includeExtList.every((ext) => /^\.[A-Za-z0-9]+$/.test(ext));
  const excludeContainsUnique = new Set(excludeContains).size === excludeContains.length;
  const excludeContainsNonEmpty = excludeContains.every((token) => toText(token, 160).length > 0);
  checks.push({
    id: 'dependency_boundary_scan_include_dirs_unique_canonical_nonempty_contract',
    ok: includeDirs.length > 0 && includeDirsUnique && includeDirsCanonical,
    detail: `count=${includeDirs.length};unique=${new Set(includeDirs).size}`,
  });
  checks.push({
    id: 'dependency_boundary_scan_include_ext_token_nonempty_contract',
    ok: includeExtList.length > 0 && includeExtCanonical,
    detail: includeExtList.join(','),
  });
  checks.push({
    id: 'dependency_boundary_scan_exclude_contains_shape_contract',
    ok: excludeContainsUnique && excludeContainsNonEmpty,
    detail: `count=${excludeContains.length};unique=${new Set(excludeContains).size}`,
  });

  const files: string[] = [];
  for (const dir of includeDirs) {
    files.push(...listFiles(path.join(ROOT, dir), includeExt, excludeContains));
  }
  const filesRel = files.map((filePath) => rel(filePath)).sort((left, right) => left.localeCompare(right));
  const filesSorted = filesRel.every((filePath, index) => index === 0 || filePath.localeCompare(filesRel[index - 1]) >= 0);
  const filesUnique = new Set(filesRel).size === filesRel.length;
  checks.push({
    id: 'dependency_boundary_scanned_files_sorted_unique_contract',
    ok: filesSorted && filesUnique,
    detail: `count=${filesRel.length};unique=${new Set(filesRel).size}`,
  });

  const layers = isPlainObject(policy.layers) ? policy.layers : {};
  const layerNames = Object.keys(layers);
  const layerNamesCanonical = layerNames.every((name) => isCanonicalToken(name));
  const layerRootsCanonical = layerNames.every((layer) => {
    const roots = Array.isArray(layers[layer]) ? layers[layer] : [];
    return roots.length > 0
      && roots.every((root) => isCanonicalRelativePath(String(root)));
  });
  checks.push({
    id: 'dependency_boundary_layers_nonempty_contract',
    ok: layerNames.length > 0,
    detail: `count=${layerNames.length}`,
  });
  checks.push({
    id: 'dependency_boundary_layer_names_token_contract',
    ok: layerNamesCanonical,
    detail: layerNames.join(','),
  });
  checks.push({
    id: 'dependency_boundary_layer_roots_canonical_nonempty_contract',
    ok: layerRootsCanonical,
    detail: `count=${layerNames.length}`,
  });

  const enforceLayers = new Set((Array.isArray(policy.enforce_layers) ? policy.enforce_layers : []).map((v) => String(v)));
  const enforceLayerList = Array.from(enforceLayers.values());
  const enforceLayersSubset = enforceLayerList.length > 0
    && enforceLayerList.every((layer) => layerNames.includes(layer));
  checks.push({
    id: 'dependency_boundary_enforce_layers_subset_nonempty_contract',
    ok: enforceLayersSubset,
    detail: `count=${enforceLayerList.length}`,
  });

  const allowImports = isPlainObject(policy.allow_imports) ? policy.allow_imports : {};
  const allowImportKeysSubset = Object.keys(allowImports).every((layer) => layerNames.includes(String(layer)));
  const allowImportTargetsSubset = Object.values(allowImports).every((targets) => {
    const list = Array.isArray(targets) ? targets : [];
    return list.every((target) => layerNames.includes(String(target)));
  });
  checks.push({
    id: 'dependency_boundary_allow_import_keys_subset_contract',
    ok: allowImportKeysSubset,
    detail: `count=${Object.keys(allowImports).length}`,
  });
  checks.push({
    id: 'dependency_boundary_allow_import_targets_subset_contract',
    ok: allowImportTargetsSubset,
    detail: `count=${Object.keys(allowImports).length}`,
  });

  const layerViolations: LayerRuleViolation[] = [];
  const conduitViolations: ConduitViolation[] = [];
  const missingLocalImports: MissingImport[] = [];
  const conduit = isPlainObject(policy.conduit_boundary) ? policy.conduit_boundary : {};
  const conduitAllow = new Set((conduit.allowlisted_files || []).map((v) => String(v).replace(/\\/g, '/')));
  const conduitExtSet = new Set((conduit.include_ext || ['.ts', '.js']).map((v) => String(v)));
  const conduitDirs = Array.isArray(conduit.include_dirs) ? conduit.include_dirs : [];
  const conduitExcludes = Array.isArray(conduit.exclude_contains) ? conduit.exclude_contains : [];
  const forbiddenPatterns = Array.isArray(conduit.forbidden_patterns) ? conduit.forbidden_patterns : [];
  const conduitRoots = conduitDirs.map((d) => String(d).replace(/\\/g, '/').replace(/\/+$/, ''));
  const conduitAllowList = Array.from(conduitAllow.values());
  const conduitAllowListUnique = new Set(conduitAllowList).size === conduitAllowList.length;
  const conduitAllowListCanonical = conduitAllowList.every((entry) => isCanonicalRelativePath(entry));
  const forbiddenPatternsUnique = new Set(forbiddenPatterns.map((token) => String(token))).size === forbiddenPatterns.length;
  const forbiddenPatternsNonEmpty = forbiddenPatterns.every((token) => toText(token, 260).length > 0);
  checks.push({
    id: 'dependency_boundary_conduit_dirs_canonical_nonempty_contract',
    ok: conduitDirs.length > 0 && conduitRoots.every((root) => isCanonicalRelativePath(root)),
    detail: `count=${conduitDirs.length}`,
  });
  checks.push({
    id: 'dependency_boundary_conduit_allowlist_unique_canonical_contract',
    ok: conduitAllowListUnique && conduitAllowListCanonical,
    detail: `count=${conduitAllowList.length};unique=${new Set(conduitAllowList).size}`,
  });
  checks.push({
    id: 'dependency_boundary_conduit_forbidden_patterns_shape_contract',
    ok: forbiddenPatterns.length > 0 && forbiddenPatternsUnique && forbiddenPatternsNonEmpty,
    detail: `count=${forbiddenPatterns.length};unique=${new Set(forbiddenPatterns.map((token) => String(token))).size}`,
  });

  for (const filePath of files) {
    const relPath = rel(filePath);
    const source = fs.readFileSync(filePath, 'utf8');
    const sourceLayer = detectLayer(relPath, layers as Record<string, unknown>);
    const specs = parseSpecs(source);

    for (const spec of specs) {
      if (!spec.startsWith('.')) continue;
      if (isVirtualGeneratedLocalImport(spec)) continue;
      const resolved = resolveLocalSpec(filePath, spec);
      if (!resolved) {
        missingLocalImports.push({ file: relPath, spec });
        continue;
      }
      const targetRel = rel(resolved);
      const targetLayer = detectLayer(targetRel, layers as Record<string, unknown>);
      if (!sourceLayer || !targetLayer) continue;
      if (!enforceLayers.has(sourceLayer)) continue;
      const allowed = new Set((allowImports[sourceLayer] || []).map((v) => String(v)));
      if (!allowed.has(targetLayer)) {
        layerViolations.push({
          file: relPath,
          source_layer: sourceLayer,
          spec,
          resolved: targetRel,
          target_layer: targetLayer,
        });
      }
    }

    const inConduitScope =
      conduitRoots.some((root) => relPath === root || relPath.startsWith(`${root}/`)) &&
      conduitExtSet.has(path.extname(relPath)) &&
      !conduitExcludes.some((token) => relPath.includes(token));
    if (!inConduitScope || conduitAllow.has(relPath)) continue;
    for (const token of forbiddenPatterns) {
      if (!token) continue;
      if (source.includes(token)) {
        conduitViolations.push({ file: relPath, forbidden_pattern: token });
      }
    }
  }

  const violations = [...layerViolations, ...conduitViolations];
  const missingCount = missingLocalImports.length;
  const layerViolationShape = layerViolations.every((row) => {
    return (
      isCanonicalRelativePath(row.file)
      && isCanonicalToken(row.source_layer)
      && isCanonicalToken(row.target_layer)
      && isCanonicalRelativePath(row.resolved)
      && toText(row.spec, 260).length > 0
    );
  });
  const conduitViolationShape = conduitViolations.every((row) => {
    return isCanonicalRelativePath(row.file) && toText(row.forbidden_pattern, 260).length > 0;
  });
  const missingImportShape = missingLocalImports.every((row) => {
    return isCanonicalRelativePath(row.file) && toText(row.spec, 260).length > 0;
  });
  checks.push({
    id: 'dependency_boundary_violation_row_shape_contract',
    ok: layerViolationShape && conduitViolationShape,
    detail: `layer=${layerViolations.length};conduit=${conduitViolations.length}`,
  });
  checks.push({
    id: 'dependency_boundary_missing_import_row_shape_contract',
    ok: missingImportShape,
    detail: `count=${missingLocalImports.length}`,
  });

  const latestPath = path.join(
    ROOT,
    (policy.paths && policy.paths.latest_path) ||
      'client/runtime/local/state/ops/dependency_boundary_guard/latest.json',
  );
  const receiptsPath = path.join(
    ROOT,
    (policy.paths && policy.paths.receipts_path) ||
      'client/runtime/local/state/ops/dependency_boundary_guard/receipts.jsonl',
  );
  const latestRel = rel(latestPath);
  const receiptsRel = rel(receiptsPath);
  checks.push({
    id: 'dependency_boundary_artifact_paths_canonical_distinct_suffix_contract',
    ok:
      isCanonicalRelativePath(latestRel)
      && isCanonicalRelativePath(receiptsRel)
      && latestRel !== receiptsRel
      && hasCaseInsensitiveSuffix(latestRel, 'latest.json')
      && hasCaseInsensitiveSuffix(receiptsRel, '.jsonl'),
    detail: `${latestRel}|${receiptsRel}`,
  });

  const checkFailures = checks.filter((row) => !row.ok).length;
  const baseOk = violations.length === 0 && missingCount === 0 && checkFailures === 0;
  const out = {
    ok: strict ? baseOk : true,
    pass: baseOk,
    type: 'dependency_boundary_guard',
    ts: nowIso(),
    strict,
    check_count: checks.length,
    check_failure_count: checkFailures,
    checks,
    scanned_files: files.length,
    layer_violations: layerViolations,
    conduit_violations: conduitViolations,
    missing_local_imports: missingLocalImports,
  };
  writeJson(latestPath, out);
  appendJsonl(receiptsPath, out);

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  if (strict && !baseOk) process.exit(1);
  return out;
}

if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'check');
  if (cmd !== 'check' && cmd !== 'run') {
    process.stderr.write(`dependency_boundary_guard: unsupported command '${cmd}'\n`);
    process.exit(2);
  }
  run(args);
}
