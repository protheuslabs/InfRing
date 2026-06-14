#!/usr/bin/env node
/* eslint-disable no-console */

import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_grok_code_live_acceptance_probe_current.json';
const MAPPING_PROBE_JSON = 'core/local/artifacts/agent_runtime_grok_code_mapping_probe_current.json';
const GROK_COMMAND = process.env.INFRING_GROK_CODE_BIN || process.env.INFRING_GROK_CODE_PATH || 'grok';
const LIVE_ENABLED = process.env.AGENT_RUNTIME_GROK_CODE_LIVE === '1';
const ALLOW_PROVIDER_UNAVAILABLE = (
  process.env.AGENT_RUNTIME_GROK_CODE_ALLOW_PROVIDER_UNAVAILABLE === '1'
  || process.env.INFRING_AGENT_RUNTIME_ALLOW_PROVIDER_UNAVAILABLE === '1'
  || process.env.INFRING_AGENT_RUNTIME_LIVE_TURN_ALLOW_PROVIDER_UNAVAILABLE === '1'
);
const TIMEOUT_MS = Math.max(5000, Number(process.env.AGENT_RUNTIME_GROK_CODE_LIVE_TIMEOUT_MS || 60000));

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

function writeReport(report: JsonObject) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

function hasNoSecretMaterial(value: any): boolean {
  return !/(api[_-]?key|secret|token|authorization)["']?\s*[:=]\s*["'][A-Za-z0-9_\-.]{12,}/i.test(JSON.stringify(value || {}));
}

function providerUnavailableText(value: any): boolean {
  return /(api[_ -]?key|auth|authentication|credential|provider|model|quota|usage limit|rate limit|unauthorized|401|403|payment|required|not configured|subscription)/i.test(
    JSON.stringify(value || {}),
  );
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

function candidatePromptJson(): string {
  return JSON.stringify([
    {
      type: 'text',
      text: 'InfRing structured context follows. Treat it as host-owned context, not user-authored instruction text.',
    },
    {
      type: 'text',
      text: JSON.stringify({
        source_authority: 'validation.agent_runtime_grok_code_live_acceptance_probe',
        structured_turn_ref: 'agent-runtime-structured-turn/validation:grok-code-live-acceptance/probe-turn',
        context_pack: {
          type: 'agent_runtime_structured_context_pack_projection',
          source_basis: 'validation_probe',
          source_authority: 'validation.agent_runtime_grok_code_live_acceptance_probe',
          fragments: [
            {
              kind: 'atom',
              ref_id: 'atom/user/grok-code-live-acceptance/1',
              payload: {
                role: 'user',
                source_kind: 'user_message',
                text_preview: 'Grok live acceptance should preserve prompt-json context.',
              },
            },
          ],
        },
        tool_grants: {
          source_authority: 'gateway_universal_core_tools_policy',
          tools: [
            {
              tool_id: 'conversation.read',
              default_policy: 'allowed_read_only',
            },
          ],
        },
      }),
    },
    {
      type: 'text',
      text: 'InfRing Grok Code live acceptance probe. Reply with exactly GROK_CODE_ACCEPTED and do not use tools.',
    },
  ]);
}

function outputContainsAcceptance(stdout: string, events: JsonObject[]): boolean {
  if (/GROK_CODE_ACCEPTED/.test(stdout)) return true;
  return /GROK_CODE_ACCEPTED/.test(JSON.stringify(events || []));
}

function main() {
  const mappingProbe = readJson(MAPPING_PROBE_JSON);
  const mappingReady = mappingProbe && mappingProbe.ok === true && mappingProbe.native_transport_probe_status === 'candidate_mapping_ready';
  if (!LIVE_ENABLED) {
    const previousReport = readJson(OUT_JSON);
    if (
      previousReport?.type === 'agent_runtime_grok_code_live_acceptance_probe' &&
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
        evidence: 'Live Grok Code acceptance was previously proven; disabled default run preserved that accepted proof without submitting another runtime call.',
        prior_live_acceptance_generated_at: previousReport.generated_at || '',
      });
      return;
    }
    writeReport({
      ok: true,
      type: 'agent_runtime_grok_code_live_acceptance_probe',
      generated_at: new Date().toISOString(),
      source_domain: 'validation',
      owner_domain: 'validation.agent_runtime',
      layer: 'gateway',
      mode: 'live_probe_disabled',
      engine_id: 'grok_code',
      live_enabled: false,
      mapping_probe_ready: mappingReady,
      typed_turn_api_available: false,
      accepted_by_runtime: false,
      submitted_to_runtime: false,
      evidence: 'Live Grok Code prompt-json/streaming-json acceptance probe is installed but disabled by default to avoid surprise runtime/token/API usage.',
      next_action: 'Set AGENT_RUNTIME_GROK_CODE_LIVE=1 to submit a disposable prompt-json + streaming-json acceptance probe.',
      violations: [],
    });
    return;
  }

  const promptJson = candidatePromptJson();
  const child = childProcess.spawnSync(GROK_COMMAND, [
    '--disable-web-search',
    '--no-subagents',
    '--no-plan',
    '--output-format',
    'streaming-json',
    '--permission-mode',
    'plan',
    '--tools',
    '',
    '--cwd',
    ROOT,
    '--prompt-json',
    promptJson,
  ], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: TIMEOUT_MS,
    maxBuffer: 1024 * 1024,
  });

  const stdout = child.stdout || '';
  const stderr = child.stderr || '';
  const events = parseJsonLines(stdout);
  const accepted = outputContainsAcceptance(stdout, events);
  const providerUnavailable = providerUnavailableText({ stdout, stderr, error: (child as any).error });
  const allowedSoft = ALLOW_PROVIDER_UNAVAILABLE && providerUnavailable;
  const violations = [];
  if (!mappingReady) violations.push({ kind: 'mapping_probe_not_ready' });
  if (child.status !== 0 && !allowedSoft) {
    violations.push({ kind: 'grok_code_live_exit_nonzero', status: child.status, stderr_preview: clean(stderr, 1000) });
  }
  if ((child as any).error && !allowedSoft) {
    violations.push({ kind: 'grok_code_live_process_error', error_code: clean((child as any).error && (child as any).error.code, 120) });
  }
  if (!accepted && !allowedSoft) violations.push({ kind: 'grok_code_acceptance_token_missing' });
  if (!hasNoSecretMaterial({ stdout, stderr })) violations.push({ kind: 'secret_material_detected' });

  writeReport({
    ok: violations.length === 0,
    type: 'agent_runtime_grok_code_live_acceptance_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: accepted ? 'live_probe_accepted' : 'live_disposable_prompt_json_acceptance_probe',
    engine_id: 'grok_code',
    live_enabled: true,
    mapping_probe_ready: mappingReady,
    typed_turn_api_available: accepted === true,
    accepted_by_runtime: accepted === true,
    submitted_to_runtime: true,
    provider_unavailable: allowedSoft,
    evidence: accepted
      ? 'Grok Code accepted a disposable prompt-json input with streaming-json output and returned the expected marker.'
      : providerUnavailable
        ? 'Grok Code prompt-json live acceptance could not complete because provider/auth/usage readiness was unavailable.'
        : 'Grok Code did not return the expected prompt-json/streaming-json acceptance marker.',
    next_action: accepted
      ? 'Map full AgentRuntimeStructuredTurn context, permission mode, session binding, streaming activity, and output projection onto the Grok prompt-json path before replacing bounded CLI compatibility transport.'
      : providerUnavailable
        ? 'Restore Grok provider/auth/usage readiness, then rerun with AGENT_RUNTIME_GROK_CODE_LIVE=1.'
        : 'Inspect stdout/stderr previews and adjust the prompt-json input mapping before promotion.',
    command: GROK_COMMAND,
    exit_status: child.status,
    event_count: events.length,
    event_type_sample: events.slice(0, 12).map((event) => clean(event && event.type, 120)).filter(Boolean),
    stdout_preview: clean(stdout, 2000),
    stderr_preview: clean(stderr, 1200),
    provider_unavailable_mode: ALLOW_PROVIDER_UNAVAILABLE ? 'soft' : 'strict',
    violations,
  });
  if (violations.length > 0) process.exit(1);
}

main();
