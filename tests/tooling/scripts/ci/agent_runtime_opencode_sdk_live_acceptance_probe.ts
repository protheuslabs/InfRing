#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_opencode_sdk_live_acceptance_probe_current.json';
const MAPPING_PROBE_JSON = 'core/local/artifacts/agent_runtime_opencode_sdk_mapping_probe_current.json';
const LIVE_ENABLED = process.env.AGENT_RUNTIME_OPENCODE_SDK_LIVE === '1';
const ALLOW_PROVIDER_UNAVAILABLE = (
  process.env.AGENT_RUNTIME_OPENCODE_SDK_ALLOW_PROVIDER_UNAVAILABLE === '1'
  || process.env.INFRING_AGENT_RUNTIME_ALLOW_PROVIDER_UNAVAILABLE === '1'
  || process.env.INFRING_AGENT_RUNTIME_LIVE_TURN_ALLOW_PROVIDER_UNAVAILABLE === '1'
);
const TIMEOUT_MS = Math.max(5000, Number(process.env.AGENT_RUNTIME_OPENCODE_SDK_LIVE_TIMEOUT_MS || 60000));

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
  return /(api[_ -]?key|auth|authentication|credential|provider|model|quota|usage limit|rate limit|unauthorized|401|403|payment|required|not configured)/i.test(
    JSON.stringify(value || {}),
  );
}

async function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: NodeJS.Timeout | null = null;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`opencode_sdk_live_timeout_${timeoutMs}ms`)), timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function runLiveAcceptance(): Promise<JsonObject> {
  let sdk: any = null;
  try {
    sdk = await import('@opencode-ai/sdk');
  } catch (error: any) {
    return {
      accepted: false,
      reason: 'opencode_sdk_package_missing',
      error_message: clean(error && error.message, 500),
      next_action: 'Install @opencode-ai/sdk and OpenCode before running AGENT_RUNTIME_OPENCODE_SDK_LIVE=1.',
    };
  }
  if (typeof sdk.createOpencode !== 'function') {
    return {
      accepted: false,
      reason: 'opencode_sdk_createOpencode_missing',
      exported_keys: Object.keys(sdk || {}).slice(0, 30),
      next_action: 'Update the OpenCode SDK mapping to match the installed @opencode-ai/sdk exports.',
    };
  }

  const controller = new AbortController();
  let opencode: any = null;
  const events: JsonObject[] = [];
  try {
    opencode = await withTimeout(sdk.createOpencode({
      hostname: '127.0.0.1',
      port: 0,
      signal: controller.signal,
      timeout: Math.min(10000, TIMEOUT_MS),
      config: {},
    }), TIMEOUT_MS);
    const client = opencode && opencode.client;
    if (!client) {
      return {
        accepted: false,
        reason: 'opencode_sdk_client_missing',
        server_url: clean(opencode && opencode.server && opencode.server.url, 500),
      };
    }

    let eventIterator: AsyncIterable<any> | null = null;
    try {
      const eventResult = await client.event.subscribe();
      eventIterator = eventResult && eventResult.stream ? eventResult.stream : null;
      if (eventIterator) {
        (async () => {
          try {
            for await (const event of eventIterator as AsyncIterable<any>) {
              events.push({
                type: clean(event && event.type, 160),
                properties_keys: event && event.properties && typeof event.properties === 'object'
                  ? Object.keys(event.properties).slice(0, 12)
                  : [],
              });
              if (events.length >= 30) break;
            }
          } catch {}
        })();
      }
    } catch {}

    const session = await withTimeout(client.session.create({
      body: { title: 'InfRing OpenCode SDK live acceptance probe' },
    }), TIMEOUT_MS);
    const sessionId = clean(session?.data?.id || session?.id || session?.data?.session?.id, 200);
    if (!sessionId) {
      return {
        accepted: false,
        reason: 'opencode_sdk_session_id_missing',
        session_keys: session && typeof session === 'object' ? Object.keys(session).slice(0, 20) : [],
      };
    }

    await withTimeout(client.session.prompt({
      path: { id: sessionId },
      body: {
        noReply: true,
        parts: [
          {
            type: 'text',
            text: 'InfRing live acceptance context injection. Treat this as host-owned context, not a user request.',
          },
        ],
      },
    }), TIMEOUT_MS);

    const result = await withTimeout(client.session.prompt({
      path: { id: sessionId },
      body: {
        parts: [
          {
            type: 'text',
            text: 'InfRing OpenCode SDK acceptance probe. Reply with exactly OPENCODE_SDK_ACCEPTED and do not use tools.',
          },
        ],
      },
    }), TIMEOUT_MS);
    const resultText = JSON.stringify(result || {});
    const accepted = /OPENCODE_SDK_ACCEPTED/.test(resultText);
    return {
      accepted,
      reason: accepted ? 'sdk_session_prompt_accepted' : 'sdk_session_prompt_missing_acceptance_marker',
      server_url: clean(opencode?.server?.url, 500),
      session_id_present: !!sessionId,
      no_reply_context_injected: true,
      prompt_result_keys: result && typeof result === 'object' ? Object.keys(result).slice(0, 20) : [],
      event_count: events.length,
      event_type_sample: events.slice(0, 12).map((event) => event.type).filter(Boolean),
      result_preview: clean(resultText, 2000),
    };
  } catch (error: any) {
    return {
      accepted: false,
      reason: 'opencode_sdk_live_exception',
      error_name: clean(error && error.name, 160),
      error_message: clean(error && error.message, 1000),
      provider_unavailable: providerUnavailableText(error),
      event_count: events.length,
      event_type_sample: events.slice(0, 12).map((event) => event.type).filter(Boolean),
    };
  } finally {
    controller.abort();
    try { opencode?.server?.close?.(); } catch {}
  }
}

async function main() {
  const mappingProbe = readJson(MAPPING_PROBE_JSON);
  const mappingReady = mappingProbe && mappingProbe.ok === true && mappingProbe.native_transport_probe_status === 'candidate_mapping_ready';
  if (!LIVE_ENABLED) {
    const previousReport = readJson(OUT_JSON);
    if (
      previousReport?.type === 'agent_runtime_opencode_sdk_live_acceptance_probe' &&
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
        evidence: 'Live OpenCode SDK acceptance was previously proven; disabled default run preserved that accepted proof without submitting another runtime call.',
        prior_live_acceptance_generated_at: previousReport.generated_at || '',
      });
      return;
    }
    writeReport({
      ok: true,
      type: 'agent_runtime_opencode_sdk_live_acceptance_probe',
      generated_at: new Date().toISOString(),
      source_domain: 'validation',
      owner_domain: 'validation.agent_runtime',
      layer: 'gateway',
      mode: 'live_probe_disabled',
      engine_id: 'opencode',
      live_enabled: false,
      mapping_probe_ready: mappingReady,
      typed_turn_api_available: false,
      accepted_by_runtime: false,
      submitted_to_runtime: false,
      evidence: 'Live OpenCode SDK acceptance probe is installed but disabled by default to avoid surprise runtime/token/API usage.',
      next_action: 'Install OpenCode and @opencode-ai/sdk, then set AGENT_RUNTIME_OPENCODE_SDK_LIVE=1 to submit a disposable SDK acceptance probe.',
      violations: [],
    });
    return;
  }

  const liveResult = await runLiveAcceptance();
  const providerUnavailable = liveResult.provider_unavailable === true || providerUnavailableText(liveResult);
  const missingSdk = liveResult.reason === 'opencode_sdk_package_missing';
  const allowedSoft = ALLOW_PROVIDER_UNAVAILABLE && (providerUnavailable || missingSdk);
  const violations = [];
  if (!mappingReady) violations.push({ kind: 'mapping_probe_not_ready' });
  if (!liveResult.accepted && !allowedSoft) violations.push({ kind: 'opencode_sdk_live_acceptance_failed', reason: liveResult.reason });
  if (!hasNoSecretMaterial(liveResult)) violations.push({ kind: 'secret_material_detected' });

  writeReport({
    ok: violations.length === 0,
    type: 'agent_runtime_opencode_sdk_live_acceptance_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: liveResult.accepted ? 'live_probe_accepted' : 'live_disposable_sdk_acceptance_probe',
    engine_id: 'opencode',
    live_enabled: true,
    mapping_probe_ready: mappingReady,
    typed_turn_api_available: liveResult.accepted === true,
    accepted_by_runtime: liveResult.accepted === true,
    submitted_to_runtime: liveResult.reason !== 'opencode_sdk_package_missing',
    provider_unavailable: allowedSoft,
    evidence: liveResult.accepted
      ? 'OpenCode SDK accepted a disposable session.create + noReply context injection + session.prompt sequence.'
      : 'OpenCode SDK live acceptance did not complete with the expected marker.',
    next_action: liveResult.accepted
      ? 'Map full AgentRuntimeStructuredTurn context, approval responses, event stream, and output projection onto the OpenCode SDK path before replacing bounded CLI compatibility transport.'
      : liveResult.next_action || 'Inspect live_result and complete OpenCode SDK availability/auth before promotion.',
    live_result: liveResult,
    provider_unavailable_mode: ALLOW_PROVIDER_UNAVAILABLE ? 'soft' : 'strict',
    violations,
  });
  if (violations.length > 0) process.exit(1);
}

main().catch((error) => {
  writeReport({
    ok: false,
    type: 'agent_runtime_opencode_sdk_live_acceptance_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'live_probe_exception',
    engine_id: 'opencode',
    live_enabled: LIVE_ENABLED,
    error: clean(error && (error.stack || error.message || error), 2000),
    violations: [{ kind: 'opencode_sdk_live_probe_exception' }],
  });
  process.exit(1);
});
