#!/usr/bin/env node
/* eslint-disable no-console */

import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_claude_stream_json_live_acceptance_probe_current.json';
const MAPPING_PROBE_JSON = 'core/local/artifacts/agent_runtime_claude_stream_json_mapping_probe_current.json';
const CLAUDE_COMMAND = process.env.INFRING_CLAUDE_CODE_BIN || process.env.INFRING_CLAUDE_CODE_PATH || 'claude';
const LIVE_ENABLED = process.env.AGENT_RUNTIME_CLAUDE_STREAM_JSON_LIVE === '1';
const ALLOW_PROVIDER_UNAVAILABLE = (
  process.env.AGENT_RUNTIME_CLAUDE_STREAM_JSON_ALLOW_PROVIDER_UNAVAILABLE === '1'
  || process.env.INFRING_AGENT_RUNTIME_ALLOW_PROVIDER_UNAVAILABLE === '1'
  || process.env.INFRING_AGENT_RUNTIME_LIVE_TURN_ALLOW_PROVIDER_UNAVAILABLE === '1'
);
const TIMEOUT_MS = Math.max(5000, Number(process.env.AGENT_RUNTIME_CLAUDE_STREAM_JSON_LIVE_TIMEOUT_MS || 60000));

function clean(value: any, max = 1000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function readJson(rel: string): JsonObject {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
  } catch {
    return {};
  }
}

function hasNoSecretMaterial(value: any): boolean {
  return !/(api[_-]?key|secret|token|authorization)["']?\s*[:=]\s*["'][A-Za-z0-9_\-.]{12,}/i.test(JSON.stringify(value || {}));
}

function parseJsonLines(text: string): JsonObject[] {
  return String(text || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return { type: 'unparsed_line', preview: clean(line, 400) };
      }
    });
}

function candidateInput(): string {
  return [
    JSON.stringify({
      type: 'user',
      message: {
        role: 'user',
        content: [
          {
            type: 'text',
            text: 'InfRing live acceptance probe: reply with exactly CLAUDE_STREAM_JSON_ACCEPTED and do not use tools.',
          },
        ],
      },
    }),
    '',
  ].join('\n');
}

function writeReport(report: JsonObject) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

function main() {
  const mappingProbe = readJson(MAPPING_PROBE_JSON);
  const mappingReady = mappingProbe && mappingProbe.ok === true && mappingProbe.native_transport_probe_status === 'candidate_mapping_ready';
  if (!LIVE_ENABLED) {
    const previousReport = readJson(OUT_JSON);
    if (
      previousReport?.type === 'agent_runtime_claude_stream_json_live_acceptance_probe' &&
      previousReport?.mode === 'live_probe_accepted' &&
      previousReport?.accepted_by_runtime === true &&
      previousReport?.typed_turn_api_available === true
    ) {
      writeReport({
        ...previousReport,
        generated_at: new Date().toISOString(),
        mode: 'live_probe_disabled_with_prior_acceptance',
        live_enabled: false,
        submitted_to_runtime: false,
        evidence: 'Live Claude stream-json acceptance was previously proven; disabled default run preserved that accepted proof without submitting another runtime call.',
        prior_live_acceptance_generated_at: previousReport.generated_at || '',
      });
      return;
    }

    writeReport({
      ok: true,
      type: 'agent_runtime_claude_stream_json_live_acceptance_probe',
      generated_at: new Date().toISOString(),
      source_domain: 'validation',
      owner_domain: 'validation.agent_runtime',
      layer: 'gateway',
      mode: 'live_probe_disabled',
      engine_id: 'claude_code',
      live_enabled: false,
      mapping_probe_ready: mappingReady,
      typed_turn_api_available: false,
      accepted_by_runtime: false,
      submitted_to_runtime: false,
      evidence: 'Live Claude Code stream-json acceptance probe is installed but disabled by default to avoid surprise token/API usage.',
      next_action: 'Set AGENT_RUNTIME_CLAUDE_STREAM_JSON_LIVE=1 to submit a disposable stream-json acceptance probe.',
      violations: [],
    });
    return;
  }

  const stdin = candidateInput();
  const child = childProcess.spawnSync(CLAUDE_COMMAND, [
    '--print',
    '--input-format',
    'stream-json',
    '--output-format',
    'stream-json',
    '--verbose',
    '--permission-mode',
    'plan',
    '--tools',
    '',
  ], {
    cwd: ROOT,
    input: stdin,
    encoding: 'utf8',
    timeout: TIMEOUT_MS,
    maxBuffer: 1024 * 1024,
  });

  const outputEvents = parseJsonLines(child.stdout || '');
  const stderrPreview = clean(child.stderr || '', 1200);
  const outputText = clean(outputEvents.map((event) => JSON.stringify(event)).join('\n'), 5000);
  const accepted = /CLAUDE_STREAM_JSON_ACCEPTED/.test(outputText);
  const authFailure = /(authentication_failed|Invalid authentication credentials|api_error_status\\\":401|401 Invalid authentication credentials)/i.test(outputText);
  const providerUnavailable = authFailure;
  const violations = [];
  if (!mappingReady) violations.push({ kind: 'mapping_probe_not_ready' });
  if (child.status !== 0) violations.push({ kind: 'claude_stream_json_live_exit_nonzero', status: child.status, stderr_preview: stderrPreview });
  if ((child as any).error) violations.push({ kind: 'claude_stream_json_live_process_error', error_code: clean((child as any).error && (child as any).error.code, 120) });
  if (!accepted) violations.push({ kind: 'claude_stream_json_acceptance_token_missing' });
  if (authFailure) violations.push({ kind: 'claude_stream_json_auth_required' });
  if (!hasNoSecretMaterial({ stdout: child.stdout, stderr: child.stderr })) violations.push({ kind: 'secret_material_detected' });
  const allowedSoftFailures = new Set(
    ALLOW_PROVIDER_UNAVAILABLE && providerUnavailable
      ? ['claude_stream_json_live_exit_nonzero', 'claude_stream_json_acceptance_token_missing', 'claude_stream_json_auth_required']
      : []
  );

  writeReport({
    ok: violations.every((entry) => !(entry && entry.kind && !allowedSoftFailures.has(entry.kind))),
    type: 'agent_runtime_claude_stream_json_live_acceptance_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'live_disposable_stream_json_acceptance_probe',
    engine_id: 'claude_code',
    live_enabled: true,
    mapping_probe_ready: mappingReady,
    typed_turn_api_available: violations.length === 0,
    provider_unavailable: ALLOW_PROVIDER_UNAVAILABLE && providerUnavailable,
    accepted_by_runtime: accepted,
    submitted_to_runtime: true,
    command: CLAUDE_COMMAND,
    exit_status: child.status,
    event_count: outputEvents.length,
    evidence: accepted
      ? 'Claude Code accepted a disposable stream-json input probe and returned the expected marker.'
      : authFailure
        ? 'Claude stream-json probe could not authenticate to Claude endpoints.'
        : 'Claude Code did not return the expected stream-json acceptance marker.',
    next_action: authFailure
      ? 'Authenticate Claude CLI in this environment (CLAUDE_API_KEY / claude auth) and rerun with AGENT_RUNTIME_CLAUDE_STREAM_JSON_LIVE=1.'
      : accepted
      ? 'Promote Claude stream-json from candidate mapping to typed transport acceptance evidence, then map full AgentRuntimeStructuredTurn records.'
      : 'Inspect stderr/stdout previews and adjust the stream-json input mapping before promotion.',
    stdout_event_type_sample: outputEvents.slice(0, 8).map((event) => clean(event && event.type, 120)).filter(Boolean),
    stderr_preview: stderrPreview,
    provider_unavailable_mode: ALLOW_PROVIDER_UNAVAILABLE ? 'soft' : 'strict',
    violations,
  });
  if (violations.some((entry) => !(entry && entry.kind && allowedSoftFailures.has(entry.kind)))) {
    process.exit(1);
  }
}

main();
