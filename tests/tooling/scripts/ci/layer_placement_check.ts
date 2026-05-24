#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { readFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

function shell(cmd) {
  return execSync(cmd, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

function parseArgs(argv) {
  const out = {
    baseRef: '',
    policyPath: 'client/runtime/config/layer_placement_policy.json',
    strict: true,
    allTracked: false,
    outJson: '',
    outMarkdown: '',
  };
  for (const arg of argv) {
    if (arg.startsWith('--base-ref=')) out.baseRef = arg.slice('--base-ref='.length);
    else if (arg.startsWith('--policy=')) out.policyPath = arg.slice('--policy='.length);
    else if (arg.startsWith('--strict=')) out.strict = !['0', 'false'].includes(arg.slice('--strict='.length).toLowerCase());
    else if (arg.startsWith('--all-tracked=')) {
      out.allTracked = ['1', 'true', 'yes'].includes(arg.slice('--all-tracked='.length).toLowerCase());
    } else if (arg.startsWith('--out-json=')) out.outJson = arg.slice('--out-json='.length);
    else if (arg.startsWith('--out-markdown=')) out.outMarkdown = arg.slice('--out-markdown='.length);
  }
  return out;
}

function resolveBaseRef(explicitBaseRef) {
  if (explicitBaseRef) return explicitBaseRef;
  if (process.env.GITHUB_BASE_REF) return `origin/${process.env.GITHUB_BASE_REF}`;
  try {
    return shell('git rev-parse --verify HEAD~1');
  } catch {
    return shell('git rev-parse --verify HEAD');
  }
}

function shellOrEmpty(cmd) {
  try {
    return shell(cmd);
  } catch {
    return '';
  }
}

function changedFiles(baseRef) {
  const fileSet = new Set();
  for (const diff of [
    baseRef ? shellOrEmpty(`git diff --name-only --diff-filter=ACMR ${baseRef}...HEAD`) : '',
    shellOrEmpty('git diff --cached --name-only --diff-filter=ACMR'),
    shellOrEmpty('git diff --name-only --diff-filter=ACMR'),
  ]) {
    if (!diff) continue;
    for (const file of diff.split('\n').map((v) => v.trim()).filter(Boolean)) {
      if (existsSync(file)) fileSet.add(file);
    }
  }
  return [...fileSet];
}

function trackedFiles() {
  try {
    const rows = shell('git ls-files');
    if (!rows) return [];
    return rows
      .split('\n')
      .map((v) => v.trim())
      .filter(Boolean)
      .filter((file) => existsSync(file));
  } catch {
    return [];
  }
}

function changedAddedLines(baseRef, file) {
  const addedLines = [];
  for (const diff of [
    baseRef ? shellOrEmpty(`git diff --unified=0 --diff-filter=ACMR ${baseRef}...HEAD -- ${JSON.stringify(file)}`) : '',
    shellOrEmpty(`git diff --cached --unified=0 --diff-filter=ACMR -- ${JSON.stringify(file)}`),
    shellOrEmpty(`git diff --unified=0 --diff-filter=ACMR -- ${JSON.stringify(file)}`),
  ]) {
    if (!diff) continue;
    addedLines.push(
      ...diff
        .split('\n')
        .filter((line) => line.startsWith('+') && !line.startsWith('+++'))
        .map((line) => line.slice(1)),
    );
  }
  return addedLines.join('\n');
}

function startsWithAny(path, prefixes) {
  return prefixes.some((prefix) => path.startsWith(prefix));
}

function hasAnyMarkerInHeader(content, markers, maxLines) {
  const header = content.split('\n').slice(0, maxLines).join('\n');
  return markers.some((m) => header.includes(m));
}

function hasAnyMarker(content, markers) {
  return markers.some((m) => content.includes(m));
}

function markerAppears(content, marker, caseInsensitive) {
  if (caseInsensitive) {
    return content.toLowerCase().includes(String(marker).toLowerCase());
  }
  return content.includes(marker);
}

function isSourceFile(path) {
  return /\.(ts|js|rs)$/.test(path);
}

function requiresOwnershipHeader(path) {
  if (!isSourceFile(path)) return false;
  if (path.startsWith('core/layer0/') || path.startsWith('core/layer1/') || path.startsWith('core/layer2/')) return true;
  if (path.startsWith('orchestration/')) return true;
  if (path.startsWith('client/runtime/systems/')) return true;
  if (path.startsWith('apps/')) return true;
  return false;
}

function ensureParent(filePath) {
  if (!filePath) return;
  mkdirSync(dirname(resolve(filePath)), { recursive: true });
}

function writeArtifacts(outJson, outMarkdown, receipt) {
  if (outJson) {
    ensureParent(outJson);
    writeFileSync(resolve(outJson), `${JSON.stringify(receipt, null, 2)}\n`);
  }
  if (outMarkdown) {
    ensureParent(outMarkdown);
    const lines = [
      '# Layer Placement Policy Check',
      '',
      `- ok: ${receipt.ok}`,
      `- scope: ${receipt.scope}`,
      `- policy: ${receipt.policy}`,
      `- scanned_files: ${receipt.scanned_files}`,
      `- violations: ${receipt.violations_count}`,
      '',
    ];
    if (receipt.violations.length === 0) {
      lines.push('- none');
    } else {
      for (const violation of receipt.violations) {
        lines.push(
          `- ${violation.type}: ${violation.file}${violation.marker ? ` [${violation.marker}]` : ''} — ${violation.hint}`,
        );
      }
    }
    writeFileSync(resolve(outMarkdown), `${lines.join('\n')}\n`);
  }
}

function run() {
  const args = parseArgs(process.argv.slice(2));
  const baseRef = args.allTracked ? '' : resolveBaseRef(args.baseRef);
  const policyFile = resolve(args.policyPath);
  const policy = JSON.parse(readFileSync(policyFile, 'utf8'));
  const files = args.allTracked ? trackedFiles() : changedFiles(baseRef);

  const violations = [];
  for (const file of files) {
    if (!isSourceFile(file)) continue;
    const content = readFileSync(file, 'utf8');

    if (requiresOwnershipHeader(file)) {
      const ok = hasAnyMarkerInHeader(
        content,
        policy.ownership_markers ?? ['Layer ownership:', 'App ownership:'],
        Number(policy.ownership_header_scan_lines ?? 12),
      );
      if (!ok) {
        violations.push({
          type: 'missing_ownership_header',
          file,
          hint: 'Add "Layer ownership:" or "App ownership:" in the first 12 lines.',
        });
      }
    }

    if (startsWithAny(file, policy.authority_client_roots ?? [])) {
      const wrapperOk = hasAnyMarker(content, policy.wrapper_markers ?? []);
      if (!wrapperOk) {
        violations.push({
          type: 'authority_logic_in_client',
          file,
          hint: 'Authority paths in client/runtime/systems must remain thin wrappers.',
        });
      }
    }

    for (const rule of policy.forbidden_concept_markers_by_prefix ?? []) {
      const pathPrefix = String(rule.path_prefix || '');
      if (!pathPrefix || !file.startsWith(pathPrefix)) continue;
      if ((rule.allow_exact_paths ?? []).includes(file)) continue;
      if (startsWithAny(file, rule.allow_path_prefixes ?? [])) continue;
      const addedContent = args.allTracked ? content : changedAddedLines(baseRef, file);
      if (!addedContent) continue;

      for (const marker of rule.markers ?? []) {
        if (!markerAppears(addedContent, marker, rule.case_insensitive !== false)) continue;
        violations.push({
          type: 'forbidden_layer_concept_marker',
          file,
          marker,
          path_prefix: pathPrefix,
          hint:
            rule.hint ||
            'This concept is not allowed in this layer by the placement policy.',
        });
      }
    }

    for (const rule of policy.forbidden_path_markers_by_prefix ?? []) {
      const pathPrefix = String(rule.path_prefix || '');
      if (!pathPrefix || !file.startsWith(pathPrefix)) continue;
      if ((rule.allow_exact_paths ?? []).includes(file)) continue;
      if (startsWithAny(file, rule.allow_path_prefixes ?? [])) continue;

      for (const marker of rule.markers ?? []) {
        if (!markerAppears(file, marker, rule.case_insensitive !== false)) continue;
        violations.push({
          type: 'forbidden_layer_path_marker',
          file,
          marker,
          path_prefix: pathPrefix,
          hint:
            rule.hint ||
            'This path shape is not allowed in this layer by the placement policy.',
        });
      }
    }
  }

  const receipt = {
    ok: violations.length === 0,
    type: 'layer_placement_policy_check',
    policy: args.policyPath,
    contract_doc: policy.contract_doc || '',
    scope: args.allTracked ? 'all_tracked' : 'changed_files',
    base_ref: baseRef || null,
    scanned_files: files.length,
    violations_count: violations.length,
    violations,
  };

  writeArtifacts(args.outJson, args.outMarkdown, receipt);
  console.log(JSON.stringify(receipt, null, 2));
  if (args.strict && violations.length > 0) process.exit(1);
}

run();
