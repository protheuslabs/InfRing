import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execSync, spawnSync } from 'node:child_process';
import { afterEach, describe, expect, test } from 'vitest';

const ROOT = process.cwd();
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const GUARD_PATH = path.join(ROOT, 'tests/tooling/scripts/ci/churn_guard.ts');

function writeTrackedFile(repoRoot: string, relativePath: string, contents = `${relativePath}\n`) {
  const absolutePath = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
  fs.writeFileSync(absolutePath, contents);
}

function createFixtureRepo(): string {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'churn-guard-swarm-'));
  execSync('git init -q', { cwd: repoRoot });
  execSync('git config user.email "codex@example.com"', { cwd: repoRoot });
  execSync('git config user.name "Codex"', { cwd: repoRoot });

  [
    'core/layer0/ops/src/swarm_runtime.rs',
    'client/runtime/systems/autonomy/swarm_sessions_bridge.ts',
    'core/layer0/ops/tests/v9_swarm_runtime_integration.rs',
    'tests/client-memory-tools/swarm_sessions_bridge.test.ts',
    'tests/tooling/scripts/ci/swarm_protocol_audit_runner.ts',
    'docs/workspace/SRS.md',
    'docs/client/requirements/REQ-38-agent-orchestration-hardening.md',
  ].forEach((relativePath) => writeTrackedFile(repoRoot, relativePath));

  execSync('git add .', { cwd: repoRoot });
  execSync('git commit -qm "fixture"', { cwd: repoRoot });
  return repoRoot;
}

function runGuard(repoRoot: string, args: string[] = []) {
  const result = spawnSync(process.execPath, [ENTRYPOINT, GUARD_PATH, ...args], {
    cwd: repoRoot,
    encoding: 'utf8',
  });
  return {
    status: result.status,
    stdout: result.stdout,
    stderr: result.stderr,
  };
}

function parseJsonOutput(text: string) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed);
  } catch {}
  const lines = trimmed.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    const candidate = lines.slice(index).join('\n').trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

const tempDirs: string[] = [];

afterEach(() => {
  while (tempDirs.length > 0) {
    const repoRoot = tempDirs.pop();
    if (repoRoot) {
      fs.rmSync(repoRoot, { recursive: true, force: true });
    }
  }
});

describe('swarm companion churn guard', () => {
  test('fails when swarm runtime changes without tests or docs', () => {
    const repoRoot = createFixtureRepo();
    tempDirs.push(repoRoot);
    fs.appendFileSync(path.join(repoRoot, 'core/layer0/ops/src/swarm_runtime.rs'), '// drift\n');

    const result = runGuard(repoRoot, ['--strict=1']);
    expect(result.status).toBe(1);

    const payload = parseJsonOutput(result.stdout) ?? parseJsonOutput(result.stderr);
    expect(payload).not.toBeNull();
    expect(payload.summary.swarm_surface_churn).toBe(1);
    expect(payload.summary.swarm_companion_gaps).toBe(2);
  });

  test('passes commit gate when swarm code, tests, and docs move together', () => {
    const repoRoot = createFixtureRepo();
    tempDirs.push(repoRoot);
    fs.appendFileSync(path.join(repoRoot, 'core/layer0/ops/src/swarm_runtime.rs'), '// runtime\n');
    fs.appendFileSync(
      path.join(repoRoot, 'tests/client-memory-tools/swarm_sessions_bridge.test.ts'),
      '// test\n',
    );
    fs.appendFileSync(path.join(repoRoot, 'docs/workspace/SRS.md'), '\n- V6-SWARM-033\n');
    execSync('git add .', { cwd: repoRoot });

    const result = runGuard(repoRoot, [
      '--strict=1',
      '--commit-gate=1',
      '--allow-governance-doc-churn=1',
    ]);
    expect(result.status).toBe(0);

    const payload = parseJsonOutput(result.stdout) ?? parseJsonOutput(result.stderr);
    expect(payload).not.toBeNull();
    expect(payload.summary.swarm_surface_churn).toBe(2);
    expect(payload.summary.governance_doc_churn).toBe(1);
    expect(payload.summary.swarm_companion_gaps).toBe(0);
    expect(payload.summary.commit_gate).toBe(true);
    expect(payload.summary.pass).toBe(true);
  });

  test('passes commit gate when the audit runner is the swarm-test companion', () => {
    const repoRoot = createFixtureRepo();
    tempDirs.push(repoRoot);
    fs.appendFileSync(path.join(repoRoot, 'core/layer0/ops/src/swarm_runtime.rs'), '// audit-hardening\n');
    fs.appendFileSync(
      path.join(repoRoot, 'tests/tooling/scripts/ci/swarm_protocol_audit_runner.ts'),
      '// audit proof\n',
    );
    fs.appendFileSync(
      path.join(repoRoot, 'docs/client/requirements/REQ-38-agent-orchestration-hardening.md'),
      '\n- audit runner companion proof\n',
    );
    execSync('git add .', { cwd: repoRoot });

    const result = runGuard(repoRoot, [
      '--strict=1',
      '--commit-gate=1',
      '--allow-governance-doc-churn=1',
    ]);
    expect(result.status).toBe(0);

    const payload = parseJsonOutput(result.stdout) ?? parseJsonOutput(result.stderr);
    expect(payload).not.toBeNull();
    expect(payload.summary.swarm_surface_churn).toBe(2);
    expect(payload.summary.swarm_companion_gaps).toBe(0);
    expect(payload.summary.commit_gate_pass).toBe(true);
    expect(payload.summary.pass).toBe(true);
  });

  test('commit gate evaluates staged contents instead of unrelated unstaged churn', () => {
    const repoRoot = createFixtureRepo();
    tempDirs.push(repoRoot);
    fs.appendFileSync(path.join(repoRoot, 'docs/workspace/SRS.md'), '\n- staged governance note\n');
    execSync('git add docs/workspace/SRS.md', { cwd: repoRoot });
    fs.mkdirSync(path.join(repoRoot, 'local/workspace/memory'), { recursive: true });
    fs.writeFileSync(path.join(repoRoot, 'local/workspace/memory/dashboard-ui-noise.log'), 'unstaged\n');

    const result = runGuard(repoRoot, [
      '--strict=1',
      '--commit-gate=1',
      '--allow-governance-doc-churn=1',
    ]);
    expect(result.status).toBe(0);

    const payload = parseJsonOutput(result.stdout) ?? parseJsonOutput(result.stderr);
    expect(payload).not.toBeNull();
    expect(payload.summary.scope).toBe('staged');
    expect(payload.summary.total).toBe(1);
    expect(payload.summary.commit_gate_pass).toBe(true);
    expect(payload.summary.pass).toBe(true);
  });
});
