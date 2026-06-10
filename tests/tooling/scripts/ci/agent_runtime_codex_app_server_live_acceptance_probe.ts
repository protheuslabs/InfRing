#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';
import childProcess from 'node:child_process';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_codex_app_server_live_acceptance_probe_current.json';
const MAPPING_PROBE_JSON = 'core/local/artifacts/agent_runtime_codex_app_server_mapping_probe_current.json';
const LIVE_ENABLED = process.env.AGENT_RUNTIME_CODEX_APP_SERVER_LIVE === '1';
const CODEX_COMMAND = process.env.INFRING_CODEX_CLI_BIN || process.env.INFRING_CODEX_CLI_PATH || 'codex';
const LIVE_TIMEOUT_MS = Number(process.env.AGENT_RUNTIME_CODEX_APP_SERVER_LIVE_TIMEOUT_MS || 45000);

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

function buildRequest(id: string, method: string, params: JsonObject = {}): JsonObject {
  return { id, method, params };
}

function parseJsonLines(chunk: string): JsonObject[] {
  const out: JsonObject[] = [];
  for (const line of chunk.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('{')) continue;
    try {
      out.push(JSON.parse(trimmed));
    } catch {
      // Ignore non-JSON diagnostic lines; they are retained in stderr/stdout previews.
    }
  }
  return out;
}

function findResponse(messages: JsonObject[], id: string): JsonObject | null {
  return messages.find((message) => String(message?.id || '') === id && ('result' in message || 'error' in message)) || null;
}

function waitForResponse(
  proc: childProcess.ChildProcessWithoutNullStreams,
  messages: JsonObject[],
  id: string,
  timeoutMs: number,
): Promise<JsonObject | null> {
  const started = Date.now();
  return new Promise((resolve) => {
    const timer = setInterval(() => {
      const response = findResponse(messages, id);
      if (response) {
        clearInterval(timer);
        resolve(response);
        return;
      }
      if (Date.now() - started >= timeoutMs || proc.exitCode !== null) {
        clearInterval(timer);
        resolve(findResponse(messages, id));
      }
    }, 50);
  });
}

function sendJson(proc: childProcess.ChildProcessWithoutNullStreams, message: JsonObject) {
  proc.stdin.write(`${JSON.stringify(message)}\n`);
}

function extractThreadId(response: JsonObject | null, messages: JsonObject[]): string {
  const candidates = [
    response?.result?.threadId,
    response?.result?.thread_id,
    response?.result?.id,
    response?.result?.thread?.id,
    ...messages.map((message) => message?.params?.threadId),
    ...messages.map((message) => message?.params?.thread_id),
  ];
  return clean(candidates.find(Boolean) || '', 200);
}

function summarizeMessages(messages: JsonObject[]): JsonObject[] {
  return messages.slice(0, 80).map((message) => ({
    id: message?.id == null ? '' : String(message.id),
    method: clean(message?.method || '', 200),
    has_result: Object.prototype.hasOwnProperty.call(message || {}, 'result'),
    has_error: Object.prototype.hasOwnProperty.call(message || {}, 'error'),
    error_code: clean(message?.error?.code || '', 200),
    error_message: clean(message?.error?.message || '', 500),
    result_keys: message?.result && typeof message.result === 'object' ? Object.keys(message.result).slice(0, 12) : [],
    params_keys: message?.params && typeof message.params === 'object' ? Object.keys(message.params).slice(0, 12) : [],
  }));
}

async function runLiveAcceptance(selectedPath: JsonObject): Promise<JsonObject> {
  const proc = childProcess.spawn(CODEX_COMMAND, ['app-server', '--stdio'], {
    cwd: ROOT,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  const messages: JsonObject[] = [];
  let stdout = '';
  let stderr = '';
  let exitCode: number | null = null;
  let exitSignal: NodeJS.Signals | null = null;
  proc.stdout.on('data', (chunk) => {
    const text = String(chunk || '');
    stdout += text;
    messages.push(...parseJsonLines(text));
  });
  proc.stderr.on('data', (chunk) => {
    stderr += String(chunk || '');
  });
  proc.on('exit', (code, signal) => {
    exitCode = code;
    exitSignal = signal;
  });

  const killTimer = setTimeout(() => {
    try { proc.kill('SIGTERM'); } catch {}
  }, LIVE_TIMEOUT_MS + 2000);

  try {
    const initializeId = 'infring-codex-live-initialize';
    sendJson(proc, buildRequest(initializeId, 'initialize', {
      clientInfo: {
        name: 'infring-agent-runtime-validation',
        title: 'InfRing Agent Runtime Validation',
        version: '0.0.0-validation',
      },
      capabilities: {
        experimentalApi: true,
        requestAttestation: false,
        optOutNotificationMethods: [],
      },
    }));
    const initializeResponse = await waitForResponse(proc, messages, initializeId, Math.min(10000, LIVE_TIMEOUT_MS));
    sendJson(proc, { method: 'initialized' });

    const threadStartId = 'infring-codex-live-thread-start';
    sendJson(proc, buildRequest(threadStartId, 'thread/start', {
      cwd: ROOT,
      ephemeral: true,
      approvalPolicy: 'never',
      approvalsReviewer: 'user',
      sandbox: 'read-only',
      runtimeWorkspaceRoots: [ROOT],
      baseInstructions: 'InfRing validation probe: answer compactly and do not mutate files.',
      developerInstructions: 'This is a disposable structured transport acceptance probe. Do not write files or run long commands.',
    }));
    const threadResponse = await waitForResponse(proc, messages, threadStartId, Math.min(15000, LIVE_TIMEOUT_MS));
    const threadId = extractThreadId(threadResponse, messages);

    if (!threadId) {
      return {
        accepted: false,
        reason: 'thread_start_no_thread_id',
        initialize_response_received: !!initializeResponse,
        thread_response_received: !!threadResponse,
        stdout_preview: clean(stdout, 2000),
        stderr_preview: clean(stderr, 2000),
        message_summary: summarizeMessages(messages),
        process_exit_code: exitCode,
        process_exit_signal: exitSignal,
        messages_seen: messages.length,
      };
    }

    const turnStartId = 'infring-codex-live-turn-start';
    sendJson(proc, buildRequest(turnStartId, 'turn/start', {
      threadId,
      cwd: ROOT,
      approvalPolicy: 'never',
      sandboxPolicy: {
        type: 'readOnly',
        networkAccess: false,
      },
      input: [
        {
          type: 'text',
          text: 'InfRing structured transport acceptance probe. Reply with exactly: CODEX_APP_SERVER_ACCEPTED',
        },
      ],
      additionalContext: {
        'infring://context/validation-probe': {
          kind: 'application',
          value: 'InfRing supplied this context through Codex app-server additionalContext, not prompt preamble hydration.',
        },
      },
      responsesapiClientMetadata: {
        infring_trace_id: 'validation:agent-runtime:codex-app-server-live-acceptance',
      },
    }));
    const turnResponse = await waitForResponse(proc, messages, turnStartId, LIVE_TIMEOUT_MS);
    const joinedMessages = JSON.stringify(messages);
    const accepted =
      !!turnResponse &&
      !turnResponse.error &&
      (!!turnResponse.result?.turn || /CODEX_APP_SERVER_ACCEPTED|turn\/started|TurnStarted/i.test(`${joinedMessages}\n${stdout}`));
    return {
      accepted,
      reason: accepted ? 'typed_thread_and_turn_submission_accepted' : 'turn_start_not_accepted',
      initialize_response_received: !!initializeResponse && !initializeResponse.error,
      thread_response_received: !!threadResponse && !threadResponse.error,
      turn_response_received: !!turnResponse && !turnResponse.error,
      thread_id_present: !!threadId,
      selected_live_acceptance_path: selectedPath,
      stdout_preview: clean(stdout, 2000),
      stderr_preview: clean(stderr, 2000),
      message_summary: summarizeMessages(messages),
      process_exit_code: exitCode,
      process_exit_signal: exitSignal,
      messages_seen: messages.length,
    };
  } finally {
    clearTimeout(killTimer);
    try { proc.stdin.end(); } catch {}
    try { proc.kill('SIGTERM'); } catch {}
  }
}

async function main() {
  const mappingProbe = readJson(MAPPING_PROBE_JSON);
  const mappingReady = mappingProbe && mappingProbe.ok === true && mappingProbe.native_transport_probe_status === 'candidate_mapping_ready';
  const selectedPath = mappingProbe?.schema_methods?.selected_live_acceptance_path || {};

  if (!LIVE_ENABLED) {
    const previousReport = readJson(OUT_JSON);
    if (
      previousReport?.type === 'agent_runtime_codex_app_server_live_acceptance_probe' &&
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
        evidence: 'Live Codex app-server acceptance was previously proven; disabled default run preserved that accepted proof without submitting another runtime call.',
        prior_live_acceptance_generated_at: previousReport.generated_at || '',
      });
      return;
    }
    writeReport({
      ok: true,
      type: 'agent_runtime_codex_app_server_live_acceptance_probe',
      generated_at: new Date().toISOString(),
      source_domain: 'validation',
      owner_domain: 'validation.agent_runtime',
      layer: 'gateway',
      mode: 'live_probe_disabled',
      engine_id: 'codex_cli',
      live_enabled: false,
      mapping_probe_ready: mappingReady,
      typed_turn_api_available: false,
      accepted_by_runtime: false,
      submitted_to_runtime: false,
      selected_live_acceptance_path: {
        status: clean(selectedPath.status || 'candidate_method_surface_unknown'),
        session_method: clean(selectedPath.session_method || ''),
        turn_method: clean(selectedPath.turn_method || ''),
        approval_callback_methods: Array.isArray(selectedPath.approval_callback_methods)
          ? selectedPath.approval_callback_methods.map((method: any) => clean(method, 200)).filter(Boolean)
          : [],
      },
      evidence: 'Live Codex app-server acceptance probe is installed but disabled by default to avoid surprise runtime/token/API usage.',
      next_action: selectedPath.turn_method
        ? `Set AGENT_RUNTIME_CODEX_APP_SERVER_LIVE=1 only after implementing disposable ${clean(selectedPath.session_method || 'session')} + ${clean(selectedPath.turn_method)} submission against Codex app-server.`
        : 'Run the mapping probe first so the live probe can select a schema-derived app-server method surface.',
      violations: [],
    });
    return;
  }

  const liveResult = await runLiveAcceptance({
    status: clean(selectedPath.status || 'candidate_method_surface_unknown'),
    session_method: clean(selectedPath.session_method || ''),
    turn_method: clean(selectedPath.turn_method || ''),
    approval_callback_methods: Array.isArray(selectedPath.approval_callback_methods)
      ? selectedPath.approval_callback_methods.map((method: any) => clean(method, 200)).filter(Boolean)
      : [],
  });
  if (liveResult.accepted) {
    writeReport({
      ok: true,
      type: 'agent_runtime_codex_app_server_live_acceptance_probe',
      generated_at: new Date().toISOString(),
      source_domain: 'validation',
      owner_domain: 'validation.agent_runtime',
      layer: 'gateway',
      mode: 'live_probe_accepted',
      engine_id: 'codex_cli',
      live_enabled: true,
      mapping_probe_ready: mappingReady,
      typed_turn_api_available: true,
      accepted_by_runtime: true,
      submitted_to_runtime: true,
      evidence: 'Codex app-server accepted a disposable typed initialize + thread/start + turn/start JSON-RPC sequence.',
      live_result: liveResult,
      violations: [],
    });
    return;
  }

  writeReport({
    ok: false,
    type: 'agent_runtime_codex_app_server_live_acceptance_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'live_probe_rejected_or_incomplete',
    engine_id: 'codex_cli',
    live_enabled: true,
    mapping_probe_ready: mappingReady,
    typed_turn_api_available: false,
    accepted_by_runtime: false,
    submitted_to_runtime: true,
    evidence: 'Codex app-server live acceptance did not complete the disposable typed JSON-RPC sequence.',
    next_action: 'Inspect live_result stdout/stderr previews and adjust the minimal initialize + thread/start + turn/start handshake.',
    live_result: liveResult,
    violations: [
      {
        kind: 'codex_app_server_live_acceptance_not_accepted',
        detail: 'Live submission was explicitly enabled but Codex app-server did not accept the disposable typed turn sequence.',
      },
    ],
  });
  process.exit(1);
}

main().catch((error) => {
  writeReport({
    ok: false,
    type: 'agent_runtime_codex_app_server_live_acceptance_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'live_probe_error',
    engine_id: 'codex_cli',
    live_enabled: LIVE_ENABLED,
    mapping_probe_ready: false,
    typed_turn_api_available: false,
    accepted_by_runtime: false,
    submitted_to_runtime: LIVE_ENABLED,
    evidence: 'Codex app-server live acceptance probe crashed before producing an accepted result.',
    error: clean(error?.message || error, 2000),
    violations: [
      {
        kind: 'codex_app_server_live_acceptance_probe_error',
        detail: clean(error?.message || error, 2000),
      },
    ],
  });
  process.exit(1);
});
