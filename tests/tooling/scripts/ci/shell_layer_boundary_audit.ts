#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const IGNORED_DIR_NAMES = new Set([
  'node_modules',
  '.svelte-kit',
  'dist',
  'build',
  'coverage',
  '.vite',
  '.turbo',
]);

function parseArgs(argv) {
  const common = parseStrictOutArgs(argv, {});
  const out = {
    policy: cleanText(readFlag(argv, 'policy') || 'client/runtime/config/shell_layer_boundary_policy.json', 400),
    out: cleanText(common.out || '', 400),
    strict: common.strict,
  };
  return out;
}

function walk(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    if (ent.isDirectory() && (IGNORED_DIR_NAMES.has(ent.name) || ent.name.startsWith('.'))) {
      continue;
    }
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) walk(p, out);
    else if (/\.(ts|js)$/.test(ent.name)) out.push(p);
  }
  return out;
}

function rel(p) {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function startsWithAny(value, roots) {
  return roots.some((root) => value.startsWith(root));
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const policyPath = path.resolve(ROOT, args.policy);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));

  const revision = currentRevision(ROOT);

  const scanRoots = Array.isArray(policy.scan_roots) ? policy.scan_roots : [];
  const wrapperRequiredRoots = Array.isArray(policy.wrapper_required_roots)
    ? policy.wrapper_required_roots
    : [];
  const wrapperMarkers = Array.isArray(policy.wrapper_markers) ? policy.wrapper_markers : [];
  const wrapperForbiddenMarkers = Array.isArray(policy.wrapper_forbidden_markers)
    ? policy.wrapper_forbidden_markers
    : [];
  const allowedNonWrapper = new Set(
    (Array.isArray(policy.allowed_non_wrapper_paths) ? policy.allowed_non_wrapper_paths : []).map((v) =>
      String(v).replace(/\\/g, '/'),
    ),
  );
  const allowedNonWrapperRoots = Array.isArray(policy.allowed_non_wrapper_roots)
    ? policy.allowed_non_wrapper_roots.map((v) => String(v).replace(/\\/g, '/'))
    : [];
  const allowedNonWrapperBudgetExemptRoots = Array.isArray(policy.allowed_non_wrapper_budget_exempt_roots)
    ? policy.allowed_non_wrapper_budget_exempt_roots.map((v) => String(v).replace(/\\/g, '/'))
    : [];

  const files = [];
  for (const scanRoot of scanRoots) {
    files.push(...walk(path.resolve(ROOT, scanRoot)));
  }

  let wrapperCount = 0;
  const allowedNonWrapperPaths = [];
  const allowedNonWrapperBudgetedPaths = [];
  const allowedNonWrapperBudgetExemptPaths = [];
  const allowedNonWrapperForbiddenMarkerPaths = [];
  const violations = [];

  for (const abs of files) {
    const rp = rel(abs);
    if (rp.includes('/tests/') || rp.includes('/__tests__/')) continue;
    if (!startsWithAny(rp, wrapperRequiredRoots)) continue;
    const source = fs.readFileSync(abs, 'utf8');
    const hasWrapperMarker = wrapperMarkers.some((m) => source.includes(String(m)));
    const forbiddenWrapperMarkers = wrapperForbiddenMarkers
      .filter((m) => source.includes(String(m)))
      .map((m) => String(m));
    const isWrapper = hasWrapperMarker && forbiddenWrapperMarkers.length === 0;
    if (isWrapper) {
      wrapperCount += 1;
      continue;
    }
    if (allowedNonWrapper.has(rp) || startsWithAny(rp, allowedNonWrapperRoots)) {
      allowedNonWrapperPaths.push(rp);
      if (startsWithAny(rp, allowedNonWrapperBudgetExemptRoots)) {
        allowedNonWrapperBudgetExemptPaths.push(rp);
      } else {
        allowedNonWrapperBudgetedPaths.push(rp);
      }
      if (forbiddenWrapperMarkers.length > 0) {
        allowedNonWrapperForbiddenMarkerPaths.push({
          file: rp,
          forbidden_wrapper_markers: forbiddenWrapperMarkers,
        });
      }
      continue;
    }
    violations.push({
      file: rp,
      reason:
        forbiddenWrapperMarkers.length > 0
          ? 'wrapper_contains_forbidden_logic_marker'
          : 'non_wrapper_in_wrapper_required_root',
      forbidden_wrapper_markers: forbiddenWrapperMarkers,
    });
  }

  const rawAllowedLimit = Number(policy.max_allowed_non_wrapper_count);
  const allowedLimit =
    Number.isFinite(rawAllowedLimit) && rawAllowedLimit >= 0
      ? rawAllowedLimit
      : Number.MAX_SAFE_INTEGER;
  const limitOk = allowedNonWrapperBudgetedPaths.length <= allowedLimit;
  if (!limitOk) {
    violations.push({
      file: '*',
      reason: 'allowed_non_wrapper_count_exceeds_policy_limit',
      detail: `${allowedNonWrapperBudgetedPaths.length} > ${allowedLimit}`,
    });
  }

  const payload = {
    type: 'shell_layer_boundary_audit',
    generated_at: new Date().toISOString(),
    revision,
    policy_path: rel(policyPath),
    summary: {
      scanned_files: files.length,
      wrapper_count: wrapperCount,
      allowed_non_wrapper_count: allowedNonWrapperPaths.length,
      allowed_non_wrapper_budgeted_count: allowedNonWrapperBudgetedPaths.length,
      allowed_non_wrapper_budget_exempt_count: allowedNonWrapperBudgetExemptPaths.length,
      allowed_non_wrapper_limit: allowedLimit,
      allowed_non_wrapper_forbidden_marker_count: allowedNonWrapperForbiddenMarkerPaths.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    allowed_non_wrapper_paths: allowedNonWrapperPaths.sort(),
    allowed_non_wrapper_budgeted_paths: allowedNonWrapperBudgetedPaths.sort(),
    allowed_non_wrapper_budget_exempt_paths: allowedNonWrapperBudgetExemptPaths.sort(),
    allowed_non_wrapper_forbidden_marker_paths: allowedNonWrapperForbiddenMarkerPaths,
    violations,
  };

  process.exit(
    emitStructuredResult(payload, {
      outPath: args.out || '',
      strict: args.strict,
      ok: violations.length === 0,
    }),
  );
}

main();
