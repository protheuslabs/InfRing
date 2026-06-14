#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CHAT = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts';
const DEFAULT_PARTS_README = 'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/README.md';
const DEFAULT_POLICY = 'docs/workspace/shell_source_of_truth_policy.md';
const DEFAULT_INVENTORY = 'core/local/artifacts/shell_duplicate_ts_inventory_current.json';
const DEFAULT_DECOMPOSITION_MANIFEST = 'client/runtime/config/shell_chat_decomposition_manifest.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_chat_surface_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_CHAT_SURFACE_OWNERSHIP_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  requireClosed: boolean;
  outJson: string;
  outMarkdown: string;
  chatPath: string;
  partsReadmePath: string;
  policyPath: string;
  inventoryPath: string;
  manifestPath: string;
};

type Violation = {
  kind: string;
  path?: string;
  token?: string;
  detail: string;
};

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    requireClosed: parseBool(readFlag(argv, 'require-closed'), false),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    chatPath: cleanText(readFlag(argv, 'chat') || DEFAULT_CHAT, 400),
    partsReadmePath: cleanText(readFlag(argv, 'parts-readme') || DEFAULT_PARTS_README, 400),
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    inventoryPath: cleanText(readFlag(argv, 'inventory') || DEFAULT_INVENTORY, 400),
    manifestPath: cleanText(readFlag(argv, 'manifest') || DEFAULT_DECOMPOSITION_MANIFEST, 400),
  };
}

function readText(relPath: string): string {
  return readFileSync(resolve(ROOT, relPath), 'utf8');
}

function requireExists(relPath: string, violations: Violation[], kind: string, detail: string): boolean {
  if (existsSync(resolve(ROOT, relPath))) return true;
  violations.push({ kind, path: relPath, detail });
  return false;
}

function requireTokens(relPath: string, source: string, tokens: string[], kind: string, detail: string): Violation[] {
  return tokens
    .filter((token) => !source.includes(token))
    .map((token) => ({ kind, path: relPath, token, detail }));
}

function normalizeRelPath(value: string): string {
  return cleanText(value, 500).replace(/\\/g, '/').replace(/^\.\//, '');
}

function lineCount(source: string): number {
  if (!source) return 0;
  return source.split(/\r?\n/).length;
}

function listTsFilesRecursive(relDir: string): string[] {
  const root = resolve(ROOT, relDir);
  if (!existsSync(root)) return [];
  const out: string[] = [];
  function walk(absDir: string, relPrefix: string) {
    for (const entry of readdirSync(absDir, { withFileTypes: true })) {
      const relPath = `${relPrefix}/${entry.name}`.replace(/^\/+/, '');
      const absPath = resolve(absDir, entry.name);
      if (entry.isDirectory()) {
        walk(absPath, relPath);
        continue;
      }
      if (entry.isFile() && relPath.endsWith('.ts')) out.push(relPath);
    }
  }
  walk(root, normalizeRelPath(relDir));
  return out.sort();
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Chat Surface Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- inventory_counterparts: ${payload.summary.inventory_counterparts}`);
  lines.push(`- inventory_duplicate_loc_estimate: ${payload.summary.inventory_duplicate_loc_estimate}`);
  lines.push(`- closure_state: ${payload.summary.closure_state}`);
  lines.push(`- part_target_max_loc: ${payload.summary.part_target_max_loc}`);
  lines.push(`- part_files: ${payload.summary.part_files}`);
  lines.push(`- oversized_part_files: ${payload.summary.oversized_part_files}`);
  lines.push(`- untracked_oversized_part_files: ${payload.summary.untracked_oversized_part_files}`);
  lines.push('');
  lines.push('## Closure');
  lines.push(`- require_closed: ${payload.inputs.require_closed}`);
  lines.push(`- closure_required: ${payload.closure.closure_required}`);
  lines.push(`- state: ${payload.closure.state}`);
  lines.push(`- counterpart_paths: ${payload.closure.counterpart_paths}`);
  lines.push(`- duplicate_loc_estimate: ${payload.closure.duplicate_loc_estimate}`);
  lines.push('- next_actions:');
  if (!payload.closure.next_actions.length) lines.push('  - none');
  for (const action of payload.closure.next_actions) lines.push(`  - ${action}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) {
    lines.push(`- ${violation.kind}: ${violation.path || 'unknown'} ${violation.token || ''}`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv = process.argv.slice(2)): number {
  const args = readArgs(argv);
  const violations: Violation[] = [];

  const pathsReady = [
    requireExists(args.chatPath, violations, 'missing_chat_surface', 'The canonical assembled chat runtime surface must exist.'),
    requireExists(args.partsReadmePath, violations, 'missing_chat_parts_readme', 'The chat parts decomposition README must exist.'),
    requireExists(args.policyPath, violations, 'missing_shell_source_policy', 'The shell source-of-truth policy must exist.'),
    requireExists(args.inventoryPath, violations, 'missing_duplicate_inventory', 'The duplicate-surface inventory artifact must exist before ownership can be validated.'),
    requireExists(args.manifestPath, violations, 'missing_chat_decomposition_manifest', 'The chat decomposition manifest must exist before oversized shard debt can be governed.'),
  ].every(Boolean);

  let inventoryCounterparts = 0;
  let inventoryDuplicateLocEstimate = 0;
  let closureState = 'not_evaluated';
  const closureNextActions: string[] = [];
  let partTargetMaxLoc = 0;
  let partFileCount = 0;
  let oversizedPartFileCount = 0;
  let untrackedOversizedPartFileCount = 0;

  if (pathsReady) {
    const chat = readText(args.chatPath);
    violations.push(
      ...requireTokens(
        args.chatPath,
        chat,
        [
          'Canonical Shell source-of-truth: assembled runtime chat surface.',
          'Decomposition debt lives under ./chat.ts.parts/**',
        ],
        'chat_surface_missing_canonical_marker',
        'The assembled chat runtime file must declare itself as the canonical Shell source-of-truth.',
      ),
    );

    const readme = readText(args.partsReadmePath);
    violations.push(
      ...requireTokens(
        args.partsReadmePath,
        readme,
        [
          '# `chat.ts.parts`',
          'Canonical runtime surface: `../chat.ts`',
          'Status: decomposition debt only',
          'Migration manifest:',
          'runtime ownership stays with `../chat.ts`',
          'new oversized parts must be declared in the migration manifest',
        ],
        'chat_parts_readme_missing_marker',
        'The chat parts directory must explicitly declare that it is non-canonical decomposition debt.',
      ),
    );

    const policy = readText(args.policyPath);
    violations.push(
      ...requireTokens(
        args.policyPath,
        policy,
        [
          'canonical assembled files that are still the runtime entry surface during migration, such as `app.ts` and `pages/chat.ts`',
          '- `pages/chat.ts` and `pages/chat.ts.parts/**` are one logical surface, not two',
          'client/runtime/config/shell_chat_decomposition_manifest.json',
        ],
        'shell_policy_missing_chat_ownership_rule',
        'The shell source-of-truth policy must explicitly classify the chat assembled surface and parts mirror.',
      ),
    );

    const manifest = JSON.parse(readText(args.manifestPath));
    const manifestCanonicalPath = normalizeRelPath(String(manifest && manifest.canonical_runtime_surface || ''));
    const manifestPartsRoot = normalizeRelPath(String(manifest && manifest.parts_root || ''));
    if (manifestCanonicalPath !== normalizeRelPath(args.chatPath)) {
      violations.push({
        kind: 'chat_decomposition_manifest_canonical_mismatch',
        path: args.manifestPath,
        detail: `Manifest canonical surface is ${manifestCanonicalPath || 'missing'}, expected ${args.chatPath}.`,
      });
    }
    const expectedPartsRoot = normalizeRelPath(args.partsReadmePath).replace(/\/README\.md$/, '');
    if (manifestPartsRoot !== expectedPartsRoot) {
      violations.push({
        kind: 'chat_decomposition_manifest_parts_root_mismatch',
        path: args.manifestPath,
        detail: `Manifest parts root is ${manifestPartsRoot || 'missing'}, expected ${expectedPartsRoot}.`,
      });
    }
    const sizePolicy = manifest && manifest.part_size_policy && typeof manifest.part_size_policy === 'object'
      ? manifest.part_size_policy
      : {};
    partTargetMaxLoc = Math.max(0, Number(sizePolicy.target_max_loc || 0) || 0);
    if (partTargetMaxLoc <= 0) {
      violations.push({
        kind: 'chat_decomposition_manifest_missing_part_size_target',
        path: args.manifestPath,
        detail: 'The chat decomposition manifest must define part_size_policy.target_max_loc.',
      });
    }
    const allowedOversizedRows = Array.isArray(sizePolicy.allowed_oversized_parts) ? sizePolicy.allowed_oversized_parts : [];
    const allowedOversizedParts = new Set<string>();
    for (const row of allowedOversizedRows) {
      const partPath = normalizeRelPath(String(row && row.path || ''));
      if (!partPath) continue;
      allowedOversizedParts.add(partPath);
      if (!String(row && row.reason || '').trim() || !String(row && row.planned_extraction || '').trim()) {
        violations.push({
          kind: 'chat_decomposition_manifest_incomplete_oversized_part',
          path: partPath,
          detail: 'Each allowed oversized part must include both reason and planned_extraction.',
        });
      }
    }
    const partFiles = manifestPartsRoot ? listTsFilesRecursive(manifestPartsRoot) : [];
    partFileCount = partFiles.length;
    const actualPartSet = new Set(partFiles);
    for (const allowedPart of allowedOversizedParts) {
      if (!actualPartSet.has(allowedPart)) {
        violations.push({
          kind: 'chat_decomposition_manifest_stale_oversized_part',
          path: allowedPart,
          detail: 'Allowed oversized part is no longer present and should be removed from the manifest.',
        });
      }
    }
    if (partTargetMaxLoc > 0) {
      for (const partPath of partFiles) {
        const loc = lineCount(readText(partPath));
        if (loc <= partTargetMaxLoc) continue;
        oversizedPartFileCount += 1;
        if (!allowedOversizedParts.has(partPath)) {
          untrackedOversizedPartFileCount += 1;
          violations.push({
            kind: 'chat_part_exceeds_target_without_manifest_entry',
            path: partPath,
            detail: `Part has ${loc} LOC, target is ${partTargetMaxLoc}; add an explicit extraction plan or split it.`,
          });
        }
      }
    }

    const inventory = JSON.parse(readText(args.inventoryPath));
    const groups = Array.isArray(inventory && inventory.duplicate_groups) ? inventory.duplicate_groups : [];
    const chatGroup = groups.find(
      (row: any) =>
        row &&
        row.kind === 'assembled_vs_parts' &&
        row.canonical_path === args.chatPath,
    );
    if (!chatGroup) {
      violations.push({
        kind: 'duplicate_inventory_missing_chat_group',
        path: args.inventoryPath,
        detail: 'The duplicate-surface inventory must classify chat.ts against chat.ts.parts/** as one logical surface.',
      });
    } else {
      inventoryCounterparts = Array.isArray(chatGroup.counterpart_paths) ? chatGroup.counterpart_paths.length : 0;
      inventoryDuplicateLocEstimate = Number(chatGroup.duplicate_loc_estimate || 0);
      if (inventoryCounterparts <= 0) {
        violations.push({
          kind: 'duplicate_inventory_chat_group_empty',
          path: args.inventoryPath,
          detail: 'The duplicate-surface inventory found chat.ts but no chat.ts.parts/** counterparts.',
        });
      }
      if (inventoryCounterparts === 0 && inventoryDuplicateLocEstimate === 0) {
        closureState = 'closed';
      } else {
        closureState = 'decomposition_debt_open';
        closureNextActions.push('Convert chat.ts.parts/** shards into real imported modules or delete them after the runtime no longer needs mirrored decomposition debt.');
        closureNextActions.push('Keep chat.ts as the canonical assembled entry only until the module graph proves parity.');
        closureNextActions.push('Do not mark SHELL-CLEANUP complete while counterpart_paths or duplicate_loc_estimate remain non-zero.');
      }
    }
  }

  if (args.requireClosed && closureState !== 'closed') {
    violations.push({
      kind: 'chat_surface_closure_incomplete',
      path: args.chatPath,
      detail: `Shell chat surface is ${closureState}; ${inventoryCounterparts} counterpart paths and ${inventoryDuplicateLocEstimate} duplicate LOC remain.`,
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'shell_chat_surface_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      chat_path: args.chatPath,
      parts_readme_path: args.partsReadmePath,
      policy_path: args.policyPath,
      inventory_path: args.inventoryPath,
      manifest_path: args.manifestPath,
      require_closed: args.requireClosed,
    },
    summary: {
      violations: violations.length,
      inventory_counterparts: inventoryCounterparts,
      inventory_duplicate_loc_estimate: inventoryDuplicateLocEstimate,
      closure_state: closureState,
      closure_next_action_count: closureNextActions.length,
      part_target_max_loc: partTargetMaxLoc,
      part_files: partFileCount,
      oversized_part_files: oversizedPartFileCount,
      untracked_oversized_part_files: untrackedOversizedPartFileCount,
    },
    closure: {
      closure_required: true,
      state: closureState,
      counterpart_paths: inventoryCounterparts,
      duplicate_loc_estimate: inventoryDuplicateLocEstimate,
      next_actions: closureNextActions,
    },
    violations,
  };

  writeTextArtifact(args.outMarkdown, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
