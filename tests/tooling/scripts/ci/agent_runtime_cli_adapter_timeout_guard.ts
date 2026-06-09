#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_cli_adapter_timeout_guard_current.json');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const LAYER = 'gateway';
const { createAgentRuntimeTurnRouteHandler } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_turn_routes.ts'));
const { createCliRuntimeEngineAdapter } = require(path.join(ROOT, 'adapters/runtime/agent_engines/cli_runtime_adapter.ts'));

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function makeResponse() {
  let resolveEnd = null;
  const ended = new Promise((resolve) => {
    resolveEnd = resolve;
  });
  return {
    statusCode: 0,
    headers: null,
    body: '',
    writableEnded: false,
    destroyed: false,
    headersSent: false,
    writeHead(statusCode, headers) {
      this.statusCode = statusCode;
      this.headers = headers || {};
      this.headersSent = true;
    },
    end(chunk = '') {
      if (chunk) this.body += String(chunk);
      this.writableEnded = true;
      if (resolveEnd) resolveEnd();
    },
    waitForEnd(timeoutMs) {
      return Promise.race([
        ended,
        new Promise((resolve) => setTimeout(resolve, timeoutMs)),
      ]);
    },
  };
}

function parsePayload(res) {
  if (!res || !res.body) return null;
  try {
    return JSON.parse(res.body);
  } catch {
    return null;
  }
}

async function main() {
  const traceId = `validation:agent-runtime-cli-adapter-timeout:${Date.now()}`;
  const routeBody = {
    agent_id: 'agent-runtime-cli-adapter-timeout-guard',
    session_id: 'cli-adapter-timeout-session',
    conversation_id: 'cli-adapter-timeout-session',
    engine_id: 'codex_cli',
    message: 'cli adapter timeout guard fake runtime probe',
    input_text: 'cli adapter timeout guard fake runtime probe',
    capability_budget: {
      max_turn_seconds: 1,
    },
    test_probe: true,
  };
  const childScript = [
    'let tick = 0;',
    "process.stdout.write('{\"type\":\"turn.started\"}\\n');",
    "process.stderr.write('fake-cli-adapter-stderr-start\\n');",
    'setInterval(() => {',
    "  process.stdout.write('{\"type\":\"item.started\",\"item\":{\"type\":\"command\",\"command\":\"fake-' + tick + '\"}}\\n');",
    "  process.stderr.write('fake-cli-adapter-stderr-' + tick + '\\n');",
    '  tick += 1;',
    '}, 25);',
  ].join('\n');

  let projectionStarted = false;
  let adapterReturned = false;
  let adapterRun = null;
  const adapterActivity = [];
  const sent = [];
  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    command: process.execPath,
    commandFallback: process.execPath,
    liveDispatch: true,
    timeoutMs: 10000,
    versionArgs: ['--version'],
    runArgs: () => ['-e', childScript],
    runStdin: (prompt) => prompt,
    cwd: ROOT,
    afterRun: (run) => {
      adapterRun = run;
    },
  });

  const handler = createAgentRuntimeTurnRouteHandler({
    turnProjectionStore: {
      agentRuntimeTurnProjection: async (_traceId, body, options = {}) => {
        projectionStarted = true;
        const adapterMessage = {
          ...body,
          trace_id: traceId,
          request_id: `${traceId}:request`,
          turn_id: `${traceId}:turn`,
          engine_id: 'codex_cli',
          input: {
            text: body.input_text || body.message,
          },
          input_text: body.input_text || body.message,
          working_directory: ROOT,
          capability_budget: {
            max_turn_seconds: 10,
          },
          abort_signal: options.abortSignal,
        };
        const result = await adapter.stream_turn({
          message: adapterMessage,
          engine: { engine_id: 'codex_cli' },
          onActivity: (event) => {
            adapterActivity.push(event);
            if (typeof options.onActivity === 'function') options.onActivity(event);
          },
        });
        adapterReturned = true;
        return result;
      },
    },
    contextPreviewProjectionStore: {
      agentRuntimeContextPackPreviewProjection: async () => ({ ok: true }),
    },
    steer: () => ({ ok: true }),
    createNativeOrchestrationClient: () => ({}),
    readJsonBody: async () => routeBody,
    sendJson: (res, statusCode, payload) => {
      sent.push({ statusCode, payload });
      if (!res.headersSent) {
        res.writeHead(statusCode, {
          'content-type': 'application/json; charset=utf-8',
          'cache-control': 'no-store',
        });
      }
      res.end(`${JSON.stringify(payload, null, 2)}\n`);
    },
  });

  const req = { method: 'POST' };
  const res = makeResponse();
  const handledPromise = handler.handleAgentRuntimeTurnRoute({
    req,
    res,
    pathname: '/api/shell-socket/agent-runtime/turn',
    traceId,
    flags: {},
  });
  handledPromise.catch(() => {});
  await res.waitForEnd(2500);
  await new Promise((resolve) => setTimeout(resolve, 1500));

  const payload = parsePayload(res) || sent[0] && sent[0].payload || null;
  const activityEvents = payload && Array.isArray(payload.activity_events)
    ? payload.activity_events
    : [];
  const launchActivityProjected = activityEvents.some((event) =>
    event &&
    event.provider_event_type === 'external_cli.launch' &&
    String(event.display_text || '').includes('Launching codex_cli CLI')
  );
  const timeoutActivityProjected = activityEvents.some((event) =>
    event &&
    event.provider_event_type === 'turn.timeout' &&
    event.status === 'timed_out'
  );
  const adapterSawCommandActivity = adapterActivity.some((event) =>
    event &&
    (event.provider_event_type === 'external_cli.launch' ||
      String(event.display_text || '').includes('fake-'))
  );
  const adapterStoppedBeforeOwnTimeout = !!(
    adapterReturned &&
    adapterRun &&
    adapterRun.timed_out !== true
  );

  const violations = [];
  if (!projectionStarted) violations.push('projection_not_started');
  if (!res.writableEnded) violations.push('route_timeout_response_not_sent');
  if (res.statusCode !== 504) violations.push(`unexpected_status_code:${res.statusCode}`);
  if (!payload || payload.error_code !== 'agent_runtime_turn_route_timeout') {
    violations.push(`unexpected_error_code:${payload && payload.error_code || 'missing'}`);
  }
  if (!payload || payload.terminal_outcome !== 'timed_out_with_reason') {
    violations.push(`unexpected_terminal_outcome:${payload && payload.terminal_outcome || 'missing'}`);
  }
  if (!launchActivityProjected) violations.push('cli_adapter_launch_activity_missing_from_timeout_payload');
  if (!timeoutActivityProjected) violations.push('timeout_activity_missing_from_timeout_payload');
  if (!adapterSawCommandActivity) violations.push('cli_adapter_activity_not_observed');
  if (!adapterStoppedBeforeOwnTimeout) violations.push('cli_adapter_not_aborted_before_internal_timeout');
  if (sent.length !== 1) violations.push(`unexpected_send_count:${sent.length}`);

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_cli_adapter_timeout_guard',
    type: 'agent_runtime_cli_adapter_timeout_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    trace_id: traceId,
    route: '/api/shell-socket/agent-runtime/turn',
    status_code: res.statusCode,
    error_code: payload && payload.error_code || null,
    terminal_outcome: payload && payload.terminal_outcome || null,
    projection_started: projectionStarted,
    route_response_sent: res.writableEnded,
    adapter_returned_after_abort: adapterReturned,
    adapter_run_timed_out: adapterRun ? adapterRun.timed_out === true : null,
    adapter_run_exit_code: adapterRun ? adapterRun.exit_code : null,
    adapter_activity_count: adapterActivity.length,
    cli_adapter_launch_activity_projected: launchActivityProjected,
    timeout_activity_projected: timeoutActivityProjected,
    send_count: sent.length,
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  ensureDir(OUT_JSON);
  const report = {
    ok: false,
    guard: 'agent_runtime_cli_adapter_timeout_guard',
    type: 'agent_runtime_cli_adapter_timeout_guard',
    generated_at: new Date().toISOString(),
    source_domain: SOURCE_DOMAIN,
    owner_domain: OWNER_DOMAIN,
    layer: LAYER,
    policy_path: POLICY_PATH,
    error: error && error.stack || String(error),
  };
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
