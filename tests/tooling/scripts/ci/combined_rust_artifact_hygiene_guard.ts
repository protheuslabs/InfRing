#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const ROOT = process.cwd();
type Json = Record<string, any>;
type Violation = { kind: string; path: string; detail: string };
type ArtifactRow = {
  path: string;
  domain: string;
  kind: string;
  bytes: number;
  lines: number;
  parent: string;
  classification: string;
  reference_class: string;
  reference_count: number;
  sample_references: string[];
  new_since_baseline: boolean;
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
function abs(rel: string): string { return path.join(ROOT, rel); }
function readJson(rel: string, fallback: Json = {}): Json {
  try { return JSON.parse(fs.readFileSync(abs(rel), 'utf8')); } catch { return fallback; }
}
function ensureDir(rel: string): void { fs.mkdirSync(path.dirname(abs(rel)), { recursive: true }); }
function gitFiles(): string[] {
  return execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }).split(/\r?\n/).filter(Boolean);
}
function isGeneratedRust(p: string): boolean {
  return p.endsWith('000-combined.rs') || p.includes('.combined_parts/') || p.includes('.combined_parts');
}
function artifactKind(p: string): string {
  if (p.endsWith('000-combined.rs')) return 'combined_rust_file';
  if (p.includes('.combined_parts/')) return 'combined_parts_member';
  if (p.includes('.combined_parts')) return 'combined_parts_artifact';
  return 'generated_rust_artifact';
}
function domainFor(p: string): string {
  if (p.startsWith('core/')) return 'core';
  if (p.startsWith('validation/')) return 'validation';
  if (p.startsWith('observability/')) return 'observability';
  if (p.startsWith('tests/')) return 'validation';
  return p.split('/')[0] || 'unknown';
}
function parentFor(p: string): string {
  const idx = p.indexOf('.combined_parts/');
  if (idx >= 0) return p.slice(0, idx + '.combined_parts'.length);
  return path.dirname(p);
}
function statsFor(rel: string): { bytes: number; lines: number } {
  try {
    const raw = fs.readFileSync(abs(rel), 'utf8');
    return { bytes: Buffer.byteLength(raw), lines: raw.split(/\r?\n/).length };
  } catch {
    return { bytes: 0, lines: 0 };
  }
}
function referenceNeedles(rel: string): string[] {
  const needles = new Set<string>([rel]);
  const marker = '.combined_parts/';
  const idx = rel.indexOf(marker);
  if (idx >= 0) {
    needles.add(rel.slice(rel.lastIndexOf('/', idx) + 1));
    needles.add(rel.slice(idx + marker.length));
  }
  needles.add(path.basename(rel));
  return [...needles].filter((needle) => needle.length > 8);
}
let referenceLineCache: string[] | null = null;
function allReferenceLines(): string[] {
  if (referenceLineCache) return referenceLineCache;
  try {
    const out = execFileSync('git', [
      'grep', '-n', '-E', 'combined_parts|000-combined\\.rs', '--',
      '.',
      ':(exclude)validation/reports/**',
      ':(exclude)core/local/artifacts/**',
      ':(exclude)local/workspace/reports/**',
    ], { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    referenceLineCache = out.split(/\r?\n/).filter(Boolean);
  } catch {
    referenceLineCache = [];
  }
  return referenceLineCache;
}
function compactReference(row: string): string {
  return row.length > 360 ? `${row.slice(0, 357)}...` : row;
}
function referencesFor(rel: string): string[] {
  const seen = new Set<string>();
  const needles = referenceNeedles(rel);
  for (const row of allReferenceLines()) {
    if (row.startsWith(`${rel}:`)) continue;
    for (const needle of needles) {
      if (row.includes(needle)) {
        seen.add(compactReference(row));
        break;
      }
    }
    if (seen.size >= 25) break;
  }
  return [...seen];
}
function referenceClass(refs: string[]): string {
  if (refs.some((row) => /^core\/.*\.rs:\d+:.*include!/.test(row) || /^validation\/tests\/rust\/.*\.rs:\d+:.*include!/.test(row))) return 'live_rust_module_reference';
  if (refs.some((row) => row.startsWith('validation/') || row.startsWith('tests/'))) return 'validation_or_test_reference';
  if (refs.some((row) => row.startsWith('docs/') || row.startsWith('validation/conformance/') || row.startsWith('validation/scorecards/'))) return 'docs_or_policy_reference';
  if (refs.length > 0) return 'other_text_reference';
  return 'unreferenced_by_text_scan';
}
function classify(rel: string, refs: string[]): { classification: string; reference_class: string } {
  const refClass = referenceClass(refs);
  if (refClass === 'live_rust_module_reference') return { classification: 'required_live_mirror', reference_class: refClass };
  if (rel.startsWith('validation/tests/') && refClass === 'validation_or_test_reference') return { classification: 'validation_fixture_mirror', reference_class: refClass };
  if (refClass === 'docs_or_policy_reference') return { classification: 'policy_or_doc_reference_only', reference_class: refClass };
  if (refClass === 'other_text_reference') return { classification: 'other_referenced_artifact', reference_class: refClass };
  return { classification: 'unreferenced_residue_candidate', reference_class: refClass };
}
function countBy(rows: ArtifactRow[], key: keyof ArtifactRow): Record<string, number> {
  const out: Record<string, number> = {};
  for (const row of rows) out[String(row[key])] = (out[String(row[key])] || 0) + 1;
  return out;
}
function validateRows(rows: any[], required: string[], rel: string, violations: Violation[]): void {
  rows.forEach((row, idx) => {
    for (const field of required) if (!(field in row)) violations.push({ kind: 'combined_row_field_missing', path: rel, detail: `row ${idx} missing ${field}` });
  });
}

function main(): void {
  const strict = boolFlag('strict', true);
  const policyPath = flag('policy', 'validation/conformance/contracts/combined_rust_artifact_hygiene_policy.json');
  const outJson = flag('out-json', 'core/local/artifacts/combined_rust_artifact_hygiene_guard_current.json');
  const outMd = flag('out-markdown', 'local/workspace/reports/COMBINED_RUST_ARTIFACT_HYGIENE_GUARD_CURRENT.md');
  const policy = readJson(policyPath);
  const invPath = String(policy.baseline_artifacts?.inventory_json || '');
  const mapPath = String(policy.baseline_artifacts?.reference_map_json || '');
  const inventory = invPath ? readJson(invPath, { artifacts: [] }) : { artifacts: [] };
  const refMap = mapPath ? readJson(mapPath, { references: [] }) : { references: [] };
  const current = gitFiles().filter(isGeneratedRust).sort();
  const inventoryPaths = new Set((inventory.artifacts || []).map((row: any) => String(row.path)));
  const mapPaths = new Set((refMap.references || []).map((row: any) => String(row.path)));
  const violations: Violation[] = [];
  if (policy.type !== 'combined_rust_artifact_hygiene_policy') violations.push({ kind: 'combined_policy_type_invalid', path: policyPath, detail: 'Wrong policy type.' });
  validateRows(inventory.artifacts || [], policy.inventory_required_fields || [], invPath, violations);
  validateRows(refMap.references || [], policy.reference_map_required_fields || [], mapPath, violations);

  const rows: ArtifactRow[] = current.map((rel) => {
    const refs = referencesFor(rel);
    const classified = classify(rel, refs);
    const st = statsFor(rel);
    return {
      path: rel,
      domain: domainFor(rel),
      kind: artifactKind(rel),
      bytes: st.bytes,
      lines: st.lines,
      parent: parentFor(rel),
      classification: classified.classification,
      reference_class: classified.reference_class,
      reference_count: refs.length,
      sample_references: refs.slice(0, 5),
      new_since_baseline: !inventoryPaths.has(rel) || !mapPaths.has(rel),
    };
  });

  const staleInventory = [...inventoryPaths].filter((p) => !current.includes(String(p))).map(String).sort();
  const newSinceBaseline = rows.filter((row) => row.new_since_baseline);
  const unreferenced = rows.filter((row) => row.classification === 'unreferenced_residue_candidate');
  const allowedClasses = new Set(policy.allowed_classifications || []);
  for (const row of rows) if (!allowedClasses.has(row.classification)) violations.push({ kind: 'generated_artifact_classification_not_allowed', path: row.path, detail: row.classification });
  if (policy.fail_on_stale_inventory === true) for (const p of staleInventory) violations.push({ kind: 'combined_inventory_stale_artifact', path: invPath, detail: p });
  if (policy.fail_on_new_since_baseline === true) for (const row of newSinceBaseline) violations.push({ kind: 'combined_inventory_missing_current_artifact', path: invPath, detail: row.path });
  const maxUnreferenced = Number(policy.thresholds?.max_unreferenced_residue_candidates ?? Number.POSITIVE_INFINITY);
  if (unreferenced.length > maxUnreferenced) violations.push({ kind: 'generated_artifact_unreferenced_residue_budget_exceeded', path: policyPath, detail: `${unreferenced.length} > ${maxUnreferenced}` });

  const payload = {
    ok: violations.length === 0,
    type: 'combined_rust_artifact_hygiene_guard',
    generated_at: new Date().toISOString(),
    strict,
    current_artifact_count: current.length,
    inventory_artifact_count: Number(inventory.artifact_count || (inventory.artifacts || []).length || 0),
    reference_map_artifact_count: Number(refMap.artifact_count || (refMap.references || []).length || 0),
    classification_summary: countBy(rows, 'classification'),
    reference_summary: countBy(rows, 'reference_class'),
    domain_summary: countBy(rows, 'domain'),
    kind_summary: countBy(rows, 'kind'),
    new_since_baseline_count: newSinceBaseline.length,
    stale_inventory_count: staleInventory.length,
    unreferenced_residue_candidate_count: unreferenced.length,
    required_live_mirror_count: rows.filter((row) => row.classification === 'required_live_mirror').length,
    validation_fixture_mirror_count: rows.filter((row) => row.classification === 'validation_fixture_mirror').length,
    policy_or_doc_reference_only_count: rows.filter((row) => row.classification === 'policy_or_doc_reference_only').length,
    new_since_baseline_sample: newSinceBaseline.slice(0, 10),
    unreferenced_residue_sample: unreferenced.slice(0, 10),
    stale_inventory_sample: staleInventory.slice(0, 10),
    artifact_sample: rows.slice(0, 10),
    violations,
  };
  ensureDir(outJson);
  fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd);
  fs.writeFileSync(abs(outMd), `# Combined Rust Artifact Hygiene Guard\n\n- ok: ${payload.ok}\n- current_artifact_count: ${payload.current_artifact_count}\n- required_live_mirror_count: ${payload.required_live_mirror_count}\n- validation_fixture_mirror_count: ${payload.validation_fixture_mirror_count}\n- policy_or_doc_reference_only_count: ${payload.policy_or_doc_reference_only_count}\n- unreferenced_residue_candidate_count: ${payload.unreferenced_residue_candidate_count}\n- new_since_baseline_count: ${payload.new_since_baseline_count}\n- stale_inventory_count: ${payload.stale_inventory_count}\n- violations: ${violations.length}\n\n## Classification summary\n\n${Object.entries(payload.classification_summary).map(([k, v]) => `- ${k}: ${v}`).join('\n') || '- none'}\n\n## Next action\n\nClassify and retire unreferenced residue candidates only after owner review; do not delete required live mirrors.\n`);
  console.log(JSON.stringify({
    ok: payload.ok,
    type: payload.type,
    generated_at: payload.generated_at,
    current_artifact_count: payload.current_artifact_count,
    classification_summary: payload.classification_summary,
    new_since_baseline_count: payload.new_since_baseline_count,
    stale_inventory_count: payload.stale_inventory_count,
    unreferenced_residue_candidate_count: payload.unreferenced_residue_candidate_count,
    required_live_mirror_count: payload.required_live_mirror_count,
    violations: payload.violations,
    artifact_paths: [outJson, outMd],
  }, null, 2));
  if (strict && !payload.ok) process.exit(1);
}
main();
