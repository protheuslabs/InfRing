#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const policyPath = 'docs/workspace/AGENTIC_IMPLEMENTATION_LOOP.md';
const outJson = readArg('--out-json') || 'core/local/artifacts/agentic_loop_doctrine_guard_current.json';
const outMarkdown = readArg('--out-markdown') || 'local/workspace/reports/AGENTIC_LOOP_DOCTRINE_GUARD_CURRENT.md';
const strict = readBoolArg('--strict');

type Check = {
  id: string;
  path: string;
  ok: boolean;
  missing: string[];
};

const checks: Check[] = [
  requireFileContains('root_agents_md', 'AGENTS.md', [
    'AGENTIC_IMPLEMENTATION_LOOP.md',
    'Objective',
    'Smallest slice',
    'Allowed lanes',
    'Forbidden lanes',
    'Evidence',
    'Promotion rule',
    'Promotion state',
  ]),
  requireFileContains('loop_doctrine', policyPath, [
    'bounded agentic loops',
    'vision context',
    'checkpoint objective',
    'validation and purity filters',
    'promote',
    'retry',
    'reject',
  ]),
  requireFileContains('next_version_crosslink', 'docs/workspace/NEXT_VERSION_VISION.md', [
    'AGENTIC_IMPLEMENTATION_LOOP.md',
    'single large prompts',
  ]),
  requireFileContains('todo_readme_template_link', 'docs/workspace/todo/README.md', [
    'AGENTIC_LOOP_ITEM_TEMPLATE.md',
    'Objective',
    'Smallest slice',
    'Allowed lanes',
    'Forbidden lanes',
    'Evidence',
    'Promotion rule',
  ]),
  requireFileContains('todo_item_template', 'docs/workspace/todo/AGENTIC_LOOP_ITEM_TEMPLATE.md', [
    'Checkpoint',
    'Objective',
    'Smallest slice',
    'Allowed lanes',
    'Forbidden lanes',
    'Boundary constraints',
    'Evidence',
    'Promotion rule',
    'Rollback plan',
  ]),
  requireFileContains('sentinel_policy', 'observability/sentinel/sentinel_agentic_loop_enforcement_policy.md', [
    'agentic_loop_gap',
    'checkpoint_objective',
    'owner_guess',
    'boundary_risk',
    'evidence_gap',
    'promotion_state',
  ]),
];

const violations = checks
  .filter((check) => !check.ok)
  .flatMap((check) => check.missing.map((missing) => `${check.id}:${missing}`));

const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:agentic-loop-doctrine-guard`;
const result = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'agentic_loop_doctrine_guard',
  generated_at: generatedAt,
  ok: violations.length === 0,
  strict,
  policy_path: policyPath,
  checked_paths: checks.map((check) => check.path),
  checks,
  violation_count: violations.length,
  violations,
};

writeFile(outJson, `${JSON.stringify(result, null, 2)}\n`);
writeFile(outMarkdown, renderMarkdown(result));
console.log(JSON.stringify(result, null, 2));
if (strict && violations.length > 0) process.exitCode = 1;

function requireFileContains(id: string, relPath: string, needles: string[]): Check {
  const absPath = path.join(root, relPath);
  if (!fs.existsSync(absPath)) {
    return { id, path: relPath, ok: false, missing: ['file_missing'] };
  }
  const text = fs.readFileSync(absPath, 'utf8');
  const missing = needles.filter((needle) => !text.includes(needle));
  return { id, path: relPath, ok: missing.length === 0, missing };
}

function readArg(name: string): string | null {
  const prefix = `${name}=`;
  const inline = process.argv.find((arg) => arg.startsWith(prefix));
  if (inline) return inline.slice(prefix.length);
  const idx = process.argv.indexOf(name);
  if (idx >= 0 && process.argv[idx + 1]) return process.argv[idx + 1];
  return null;
}

function readBoolArg(name: string): boolean {
  const value = readArg(name);
  if (value !== null) return value === '1' || value === 'true';
  return process.argv.includes(name);
}

function writeFile(relPath: string, body: string): void {
  const absPath = path.join(root, relPath);
  fs.mkdirSync(path.dirname(absPath), { recursive: true });
  fs.writeFileSync(absPath, body);
}

function renderMarkdown(result: Record<string, any>): string {
  const lines = [
    '# Agentic Loop Doctrine Guard',
    '',
    `Generated: ${result.generated_at}`,
    `Status: ${result.ok ? 'pass' : 'fail'}`,
    `Policy: ${result.policy_path}`,
    '',
    '## Checks',
    '',
    '| Check | Path | Status | Missing |',
    '|---|---|---|---|',
  ];
  for (const check of result.checks as Check[]) {
    lines.push(`| ${check.id} | ${check.path} | ${check.ok ? 'pass' : 'fail'} | ${check.missing.join(', ') || 'none'} |`);
  }
  if (result.violations.length > 0) {
    lines.push('', '## Violations', '');
    for (const violation of result.violations as string[]) {
      lines.push(`- ${violation}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}
