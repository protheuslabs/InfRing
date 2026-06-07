#!/usr/bin/env node
'use strict';

const { execSync } = require('child_process');

const REPO_ROOT = execSync('git rev-parse --show-toplevel', { encoding: 'utf8' }).trim();

const CODE_EXT_RE = /\.(rs|ts|js|py|c|cc|cpp|h|hpp|html|css|sh|ps1)$/;
const CORE_DISALLOWED_RE = /\.(ts|js|py|sh|ps1|html|css)$/;
const CLIENT_NATIVE_RE = /\.(rs|c|cc|cpp|h|hpp)$/;
const CORE_ALLOWED_NONRUST_PREFIXES = [
  'core/layer1/memory_runtime/adaptive/'
];
const REQUIRED_CORE_ALLOWED_NONRUST_PREFIXES = [
  'core/layer1/memory_runtime/adaptive/'
];
const CLIENT_ALLOWED_NATIVE_PREFIXES = [
  'client/pure-workspace/'
];
const REQUIRED_CLIENT_ALLOWED_NATIVE_PREFIXES = [
  'client/pure-workspace/'
];
const EXEMPT_CODE_ROOTS = new Set([
  'adapters',
  'apps',
  'benchmarks',
  'deploy',
  'docs',
  'examples',
  'gateway',
  'install',
  'observability',
  'orchestration',
  'packages',
  'planes',
  'proofs',
  'references',
  'scripts',
  'setup',
  'shell',
  'surface',
  'tests',
  'tools',
  'validation',
  'xtask'
]);
const REQUIRED_EXEMPT_CODE_ROOTS = [
  'adapters',
  'apps',
  'benchmarks',
  'deploy',
  'docs',
  'examples',
  'gateway',
  'install',
  'observability',
  'orchestration',
  'packages',
  'planes',
  'proofs',
  'references',
  'scripts',
  'setup',
  'shell',
  'surface',
  'tests',
  'tools',
  'validation',
  'xtask'
];

function loadTrackedFiles() {
  const out = execSync('git ls-files', {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024
  });
  return out.split('\n').map((v) => v.trim()).filter(Boolean);
}

function firstSegment(relPath) {
  const idx = relPath.indexOf('/');
  if (idx < 0) return relPath;
  return relPath.slice(0, idx);
}

function printViolation(title, rows) {
  if (!rows || rows.length === 0) return;
  console.log('');
  console.log(`LAYER RULE VIOLATION: ${title}`);
  for (const row of rows) console.log(row);
}

function duplicateValues(values) {
  const counts = new Map();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([value]) => value)
    .sort();
}

function isCanonicalPrefix(prefix, expectedRoot) {
  if (typeof prefix !== 'string') return false;
  if (!prefix) return false;
  if (prefix.trim() !== prefix) return false;
  if (prefix.includes('\\')) return false;
  if (!prefix.startsWith(`${expectedRoot}/`)) return false;
  if (!prefix.endsWith('/')) return false;
  if (prefix.includes('//')) return false;
  if (prefix.startsWith('./') || prefix.startsWith('../')) return false;
  return true;
}

function isCanonicalPath(pathToken) {
  if (typeof pathToken !== 'string') return false;
  if (!pathToken) return false;
  if (pathToken.trim() !== pathToken) return false;
  if (pathToken.includes('\\')) return false;
  if (pathToken.includes('//')) return false;
  if (pathToken.startsWith('/') || pathToken.startsWith('./') || pathToken.startsWith('../')) return false;
  const segments = pathToken.split('/');
  if (segments.some((segment) => !segment || segment === '.' || segment === '..')) return false;
  return true;
}

function isCanonicalRootToken(token) {
  return typeof token === 'string' && /^[a-z0-9._-]+$/.test(token);
}

function hasFlag(args, name) {
  const normalized = String(name || '').replace(/^-+/, '');
  return args.some((arg) => {
    const value = String(arg || '').replace(/^-+/, '');
    return value === normalized;
  });
}

function main() {
  const files = loadTrackedFiles();
  const policyFailures = [];
  const violations = [];

  const coreAllowed = [...CORE_ALLOWED_NONRUST_PREFIXES];
  if (coreAllowed.length === 0) {
    policyFailures.push({
      reason: 'core_allowed_nonrust_prefixes_empty',
    });
  }
  const duplicateCoreAllowed = duplicateValues(coreAllowed);
  if (duplicateCoreAllowed.length > 0) {
    policyFailures.push({
      reason: 'core_allowed_nonrust_prefixes_duplicate',
      markers: duplicateCoreAllowed,
    });
  }
  const nonCanonicalCoreAllowed = coreAllowed.filter((row) => !isCanonicalPrefix(row, 'core'));
  if (nonCanonicalCoreAllowed.length > 0) {
    policyFailures.push({
      reason: 'core_allowed_nonrust_prefixes_noncanonical',
      markers: nonCanonicalCoreAllowed.sort(),
    });
  }
  const rootBroadCoreAllowed = coreAllowed.filter((row) => row === 'core/');
  if (rootBroadCoreAllowed.length > 0) {
    policyFailures.push({
      reason: 'core_allowed_nonrust_prefixes_root_broad',
      markers: rootBroadCoreAllowed,
    });
  }
  const sortedCoreAllowed = [...coreAllowed].sort((left, right) => left.localeCompare(right));
  if (sortedCoreAllowed.join('|') !== coreAllowed.join('|')) {
    policyFailures.push({
      reason: 'core_allowed_nonrust_prefixes_order_drift',
      markers: coreAllowed,
    });
  }
  const missingRequiredCoreAllowed = REQUIRED_CORE_ALLOWED_NONRUST_PREFIXES.filter((row) => !coreAllowed.includes(row));
  if (missingRequiredCoreAllowed.length > 0) {
    policyFailures.push({
      reason: 'core_allowed_nonrust_prefixes_required_missing',
      markers: missingRequiredCoreAllowed.sort(),
    });
  }

  const clientAllowed = [...CLIENT_ALLOWED_NATIVE_PREFIXES];
  if (clientAllowed.length === 0) {
    policyFailures.push({
      reason: 'client_allowed_native_prefixes_empty',
    });
  }
  const duplicateClientAllowed = duplicateValues(clientAllowed);
  if (duplicateClientAllowed.length > 0) {
    policyFailures.push({
      reason: 'client_allowed_native_prefixes_duplicate',
      markers: duplicateClientAllowed,
    });
  }
  const nonCanonicalClientAllowed = clientAllowed.filter((row) => !isCanonicalPrefix(row, 'client'));
  if (nonCanonicalClientAllowed.length > 0) {
    policyFailures.push({
      reason: 'client_allowed_native_prefixes_noncanonical',
      markers: nonCanonicalClientAllowed.sort(),
    });
  }
  const rootBroadClientAllowed = clientAllowed.filter((row) => row === 'client/');
  if (rootBroadClientAllowed.length > 0) {
    policyFailures.push({
      reason: 'client_allowed_native_prefixes_root_broad',
      markers: rootBroadClientAllowed,
    });
  }
  const sortedClientAllowed = [...clientAllowed].sort((left, right) => left.localeCompare(right));
  if (sortedClientAllowed.join('|') !== clientAllowed.join('|')) {
    policyFailures.push({
      reason: 'client_allowed_native_prefixes_order_drift',
      markers: clientAllowed,
    });
  }
  const missingRequiredClientAllowed = REQUIRED_CLIENT_ALLOWED_NATIVE_PREFIXES.filter((row) => !clientAllowed.includes(row));
  if (missingRequiredClientAllowed.length > 0) {
    policyFailures.push({
      reason: 'client_allowed_native_prefixes_required_missing',
      markers: missingRequiredClientAllowed.sort(),
    });
  }

  const exemptRoots = [...EXEMPT_CODE_ROOTS];
  if (exemptRoots.length === 0) {
    policyFailures.push({
      reason: 'exempt_code_roots_empty',
    });
  }
  const nonCanonicalExemptRoots = exemptRoots.filter((row) => !isCanonicalRootToken(row));
  if (nonCanonicalExemptRoots.length > 0) {
    policyFailures.push({
      reason: 'exempt_code_roots_noncanonical',
      markers: nonCanonicalExemptRoots.sort(),
    });
  }
  const slashBearingExemptRoots = exemptRoots.filter((row) => row.includes('/'));
  if (slashBearingExemptRoots.length > 0) {
    policyFailures.push({
      reason: 'exempt_code_roots_contains_slash',
      markers: slashBearingExemptRoots.sort(),
    });
  }
  const sortedExemptRoots = [...exemptRoots].sort((left, right) => left.localeCompare(right));
  if (sortedExemptRoots.join('|') !== exemptRoots.join('|')) {
    policyFailures.push({
      reason: 'exempt_code_roots_order_drift',
      markers: exemptRoots,
    });
  }
  const missingRequiredExemptRoots = REQUIRED_EXEMPT_CODE_ROOTS.filter((row) => !EXEMPT_CODE_ROOTS.has(row));
  if (missingRequiredExemptRoots.length > 0) {
    policyFailures.push({
      reason: 'exempt_code_roots_required_missing',
      markers: missingRequiredExemptRoots.sort(),
    });
  }

  if (files.length === 0) {
    policyFailures.push({
      reason: 'tracked_files_empty',
    });
  }
  const duplicateTrackedFiles = duplicateValues(files);
  if (duplicateTrackedFiles.length > 0) {
    policyFailures.push({
      reason: 'tracked_files_duplicate',
      markers: duplicateTrackedFiles,
    });
  }
  const nonCanonicalTrackedFiles = files.filter((row) => !isCanonicalPath(row));
  if (nonCanonicalTrackedFiles.length > 0) {
    policyFailures.push({
      reason: 'tracked_files_noncanonical',
      markers: nonCanonicalTrackedFiles.sort(),
    });
  }
  const dotSegmentTrackedFiles = files.filter((row) => row.includes('/./') || row.includes('/../'));
  if (dotSegmentTrackedFiles.length > 0) {
    policyFailures.push({
      reason: 'tracked_files_dot_segment_drift',
      markers: dotSegmentTrackedFiles.sort(),
    });
  }
  const whitespaceTrackedFiles = files.filter((row) => /\s/.test(row));
  if (whitespaceTrackedFiles.length > 0) {
    policyFailures.push({
      reason: 'tracked_files_whitespace',
      markers: whitespaceTrackedFiles.sort(),
    });
  }

  const badRoots = files
    .filter((p) => CODE_EXT_RE.test(p))
    .filter((p) => p.includes('/'))
    .filter((p) => {
      const seg = firstSegment(p);
      if (seg === 'core' || seg === 'client' || seg === 'surface') return false;
      if (seg.startsWith('.')) return false;
      return !EXEMPT_CODE_ROOTS.has(seg);
    })
    .sort();
  if (badRoots.length > 0) {
    violations.push({
      reason: 'source_code_paths_outside_core_surface_client',
      paths: badRoots,
    });
  }

  const coreDisallowed = files
    .filter((p) => p.startsWith('core/'))
    .filter((p) => CORE_DISALLOWED_RE.test(p))
    .filter((p) => !CORE_ALLOWED_NONRUST_PREFIXES.some((prefix) => p.startsWith(prefix)))
    .sort();
  if (coreDisallowed.length > 0) {
    violations.push({
      reason: 'non_core_language_files_in_core',
      paths: coreDisallowed,
    });
  }

  const clientNative = files
    .filter((p) => p.startsWith('client/'))
    .filter((p) => CLIENT_NATIVE_RE.test(p))
    .filter((p) => !CLIENT_ALLOWED_NATIVE_PREFIXES.some((prefix) => p.startsWith(prefix)))
    .sort();
  if (clientNative.length > 0) {
    violations.push({
      reason: 'native_files_in_client',
      paths: clientNative,
    });
  }

  const violationPaths = violations.flatMap((row) => row.paths || []);
  const duplicateViolationPaths = duplicateValues(violationPaths);
  if (duplicateViolationPaths.length > 0) {
    policyFailures.push({
      reason: 'violation_paths_duplicate',
      markers: duplicateViolationPaths,
    });
  }
  const nonCanonicalViolationPaths = violationPaths.filter((row) => !isCanonicalPath(row));
  if (nonCanonicalViolationPaths.length > 0) {
    policyFailures.push({
      reason: 'violation_paths_noncanonical',
      markers: nonCanonicalViolationPaths.sort(),
    });
  }

  const fail = policyFailures.length > 0 || violations.length > 0;
  const payload = {
    type: 'layer_rulebook_check',
    policy_failures: policyFailures,
    violations,
    summary: {
      tracked_file_count: files.length,
      policy_failure_count: policyFailures.length,
      violation_count: violations.length,
      violation_path_count: violationPaths.length,
      pass: !fail,
    },
  };

  if (hasFlag(process.argv.slice(2), 'json')) {
    console.log(JSON.stringify(payload, null, 2));
  }
  for (const policyFailure of policyFailures) {
    printViolation(`policy_contract:${policyFailure.reason}`, policyFailure.markers || []);
  }
  for (const violation of violations) {
    printViolation(violation.reason, violation.paths);
  }
  if (fail) {
    process.exit(1);
  }

  console.log('Layer rulebook checks passed.');
}

main();
