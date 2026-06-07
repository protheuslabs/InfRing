#!/usr/bin/env node
'use strict';

const childProcess = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const ARTIFACT_PATH = path.join(
  ROOT,
  'core/local/artifacts/agent_runtime_framework_coordination_guard_current.json',
);
const GATE_TIMEOUT_MS = Number.parseInt(
  process.env.AGENT_RUNTIME_FRAMEWORK_COORDINATION_GATE_TIMEOUT_MS || '60000',
  10,
);
const PREVIEW_LIMIT = Number.parseInt(
  process.env.AGENT_RUNTIME_FRAMEWORK_COORDINATION_PREVIEW_CHARS || '2400',
  10,
);

const GATES = [
  {
    id: 'approval_lifecycle',
    script: 'tests/tooling/scripts/ci/agent_runtime_route_approval_lifecycle_guard.ts',
  },
  {
    id: 'context_continuity',
    script: 'tests/tooling/scripts/ci/agent_runtime_context_continuity_eval.ts',
  },
  {
    id: 'transcript_persistence_parity',
    script: 'tests/tooling/scripts/ci/agent_runtime_transcript_persistence_parity_guard.ts',
  },
  {
    id: 'route_transcript_persistence',
    script: 'tests/tooling/scripts/ci/agent_runtime_route_transcript_persistence_guard.ts',
  },
  {
    id: 'activity_projection',
    script: 'tests/tooling/scripts/ci/agent_runtime_activity_projection_guard.ts',
  },
  {
    id: 'model_projection',
    script: 'tests/tooling/scripts/ci/agent_runtime_model_projection_guard.ts',
  },
  {
    id: 'route_structured_transport',
    script: 'tests/tooling/scripts/ci/agent_runtime_route_structured_transport_guard.ts',
  },
  {
    id: 'real_work_replay',
    script: 'tests/tooling/scripts/ci/agent_runtime_real_work_replay_guard.ts',
  },
];

function preview(value) {
  const text = String(value || '').trim();
  if (text.length <= PREVIEW_LIMIT) {
    return text;
  }
  return `${text.slice(0, PREVIEW_LIMIT)}... [truncated ${text.length - PREVIEW_LIMIT} chars]`;
}

function parseJsonCandidate(text) {
  const source = String(text || '').trim();
  if (!source) {
    return null;
  }

  const first = source.indexOf('{');
  const last = source.lastIndexOf('}');
  if (first < 0 || last <= first) {
    return null;
  }

  try {
    return JSON.parse(source.slice(first, last + 1));
  } catch {
    return null;
  }
}

function runGate(gate) {
  const startedAt = Date.now();
  const child = childProcess.spawnSync(
    process.execPath,
    [ENTRYPOINT, gate.script],
    {
      cwd: ROOT,
      encoding: 'utf8',
      maxBuffer: 4 * 1024 * 1024,
      timeout: Number.isFinite(GATE_TIMEOUT_MS) && GATE_TIMEOUT_MS > 0
        ? GATE_TIMEOUT_MS
        : 60000,
    },
  );

  const parsed = parseJsonCandidate(child.stdout);
  const timedOut = child.error && child.error.code === 'ETIMEDOUT';
  const ok = child.status === 0 && !timedOut && (!parsed || parsed.ok !== false);

  return {
    id: gate.id,
    script: gate.script,
    ok,
    status: child.status,
    signal: child.signal || null,
    timed_out: Boolean(timedOut),
    duration_ms: Date.now() - startedAt,
    parsed_ok: parsed && typeof parsed.ok === 'boolean' ? parsed.ok : null,
    stdout_preview: preview(child.stdout),
    stderr_preview: preview(child.stderr),
    error: child.error
      ? {
          code: child.error.code || null,
          message: child.error.message || String(child.error),
        }
      : null,
  };
}

function main() {
  const startedAt = Date.now();
  const gateResults = GATES.map(runGate);
  const failures = gateResults.filter((gate) => !gate.ok);
  const report = {
    ok: failures.length === 0,
    generated_at: new Date().toISOString(),
    kind: 'agent_runtime_framework_coordination_guard',
    version: 1,
    policy: {
      purpose:
        'Roll up the external-framework coordination proof surface without creating a second source of truth.',
      shell_cognition_policy:
        'No dashboard cognition: this guard only invokes Gateway/Validation proof scripts.',
      timeout_ms_per_gate: GATE_TIMEOUT_MS,
    },
    summary: {
      gate_count: gateResults.length,
      pass_count: gateResults.length - failures.length,
      failure_count: failures.length,
      duration_ms: Date.now() - startedAt,
    },
    gates: gateResults,
    failures: failures.map((gate) => ({
      id: gate.id,
      script: gate.script,
      status: gate.status,
      signal: gate.signal,
      timed_out: gate.timed_out,
      stderr_preview: gate.stderr_preview,
      stdout_preview: gate.stdout_preview,
    })),
  };

  fs.mkdirSync(path.dirname(ARTIFACT_PATH), { recursive: true });
  fs.writeFileSync(ARTIFACT_PATH, `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (!report.ok) {
    process.exitCode = 1;
  }
}

main();
