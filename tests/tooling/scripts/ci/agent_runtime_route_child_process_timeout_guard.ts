#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const childProcess = require('child_process');
const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_route_child_process_timeout_guard_current.json');
const SOURCE_DOMAIN = 'validation';
const OWNER_DOMAIN = 'validation.agent_runtime';
const POLICY_PATH = 'validation/conformance/contracts/agent_runtime_turn_outcome_contract.json';
const LAYER = 'gateway';
const { createAgentRuntimeTurnRouteHandler } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_turn_routes.ts'));

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

function stopChild(child) {
  if (!child || child.killed || child.exitCode != null || child.signalCode != null) return;
  try { child.kill('SIGTERM'); } catch {}
  setTimeout(() => {
    if (!child || child.killed || child.exitCode != null || child.signalCode != null) return;
    try { child.kill('SIGKILL'); } catch {}
  }, 500).unref();
}

async function main() {
  const traceId = `validation:agent-runtime-route-child-process-timeout:${Date.now()}`;
  const body = {
    agent_id: 'agent-runtime-route-child-process-timeout-guard',
    session_id: 'route-child-process-timeout-session',
    conversation_id: 'route-child-process-timeout-session',
    engine_id: 'codex_cli',
    message: 'route child process timeout guard delayed cli probe',
    input_text: 'route child process timeout guard delayed cli probe',
    capability_budget: {
      max_turn_seconds: 1,
    },
    test_probe: true,
  };

  let projectionStarted = false;
  let childStarted = false;
  let abortSeen = false;
  let childExitedAfterAbort = false;
  let stdoutChunks = 0;
  let stderrChunks = 0;
  const sent = [];
  const children = new Set();
  const childScript = [
    'let tick = 0;',
    "process.stdout.write('fake-cli-start\\n');",
    "process.stderr.write('fake-cli-stderr-start\\n');",
    'setInterval(() => {',
    "  process.stdout.write('fake-cli-stdout-' + tick + ' '.repeat(256) + '\\n');",
    "  process.stderr.write('fake-cli-stderr-' + tick + '\\n');",
    '  tick += 1;',
    '}, 25);',
  ].join('\n');

  const handler = createAgentRuntimeTurnRouteHandler({
    turnProjectionStore: {
      agentRuntimeTurnProjection: async (_traceId, _body, options = {}) => {
        projectionStarted = true;
        const child = childProcess.spawn(process.execPath, ['-e', childScript], {
          cwd: ROOT,
          stdio: ['ignore', 'pipe', 'pipe'],
        });
        childStarted = true;
        children.add(child);
        child.stdout.on('data', () => {
          stdoutChunks += 1;
        });
        child.stderr.on('data', () => {
          stderrChunks += 1;
        });
        child.on('close', () => {
          children.delete(child);
          if (abortSeen) childExitedAfterAbort = true;
        });
        if (options.abortSignal) {
          options.abortSignal.addEventListener('abort', () => {
            abortSeen = true;
            stopChild(child);
          }, { once: true });
        }
        if (typeof options.onActivity === 'function') {
          options.onActivity({
            type: 'agent_activity_event',
            trace_id: traceId,
            engine_id: 'codex_cli',
            activity_kind: 'runtime_activity',
            provider_event_type: 'external_cli.launch',
            source: 'agent_runtime_route_child_process_timeout_guard_fake_cli',
            status: 'running',
            display_text: 'Fake CLI child process launched and is intentionally streaming past the route budget.',
          });
        }
        return new Promise(() => {});
      },
    },
    contextPreviewProjectionStore: {
      agentRuntimeContextPackPreviewProjection: async () => ({ ok: true }),
    },
    steer: () => ({ ok: true }),
    createNativeOrchestrationClient: () => ({}),
    readJsonBody: async () => body,
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
  await new Promise((resolve) => setTimeout(resolve, 750));
  for (const child of Array.from(children)) stopChild(child);
  await new Promise((resolve) => setTimeout(resolve, 250));

  const payload = parsePayload(res) || sent[0] && sent[0].payload || null;
  const activityEvents = payload && Array.isArray(payload.activity_events)
    ? payload.activity_events
    : [];
  const launchedActivityProjected = activityEvents.some((event) =>
    event &&
    event.provider_event_type === 'external_cli.launch' &&
    String(event.display_text || '').includes('Fake CLI child process launched')
  );
  const timeoutActivityProjected = activityEvents.some((event) =>
    event &&
    event.provider_event_type === 'turn.timeout' &&
    event.status === 'timed_out'
  );

  const violations = [];
  if (!projectionStarted) violations.push('projection_not_started');
  if (!childStarted) violations.push('child_process_not_started');
  if (!res.writableEnded) violations.push('route_timeout_response_not_sent');
  if (res.statusCode !== 504) violations.push(`unexpected_status_code:${res.statusCode}`);
  if (!payload || payload.error_code !== 'agent_runtime_turn_route_timeout') {
    violations.push(`unexpected_error_code:${payload && payload.error_code || 'missing'}`);
  }
  if (!payload || payload.terminal_outcome !== 'timed_out_with_reason') {
    violations.push(`unexpected_terminal_outcome:${payload && payload.terminal_outcome || 'missing'}`);
  }
  if (!abortSeen) violations.push('projection_abort_signal_not_fired');
  if (!childExitedAfterAbort) violations.push('child_process_not_stopped_after_abort');
  if (stdoutChunks < 1) violations.push('stdout_not_observed');
  if (stderrChunks < 1) violations.push('stderr_not_observed');
  if (!launchedActivityProjected) violations.push('adapter_launch_activity_missing_from_timeout_payload');
  if (!timeoutActivityProjected) violations.push('timeout_activity_missing_from_timeout_payload');
  if (sent.length !== 1) violations.push(`unexpected_send_count:${sent.length}`);

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_route_child_process_timeout_guard',
    type: 'agent_runtime_route_child_process_timeout_guard',
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
    child_process_started: childStarted,
    projection_abort_signal_fired: abortSeen,
    child_process_stopped_after_abort: childExitedAfterAbort,
    stdout_chunks_observed: stdoutChunks,
    stderr_chunks_observed: stderrChunks,
    adapter_launch_activity_projected: launchedActivityProjected,
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
    guard: 'agent_runtime_route_child_process_timeout_guard',
    type: 'agent_runtime_route_child_process_timeout_guard',
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
