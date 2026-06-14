#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::codex-app-server-transport.
//
// Adapter-private translator for Codex app-server JSON-RPC. Gateway remains the
// authority for route admission, context construction, approvals, receipts, and
// projection bounds; this module only maps an already-built Agent Runtime turn
// onto Codex's private native transport.

'use strict';

const childProcess = require('child_process');

function cleanText(value, max = 2000) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
    .replace(/\r\n/g, '\n')
    .trim()
    .slice(0, max);
}

function compactJson(value, max = 12000) {
  try {
    return JSON.stringify(value || {}, null, 2).slice(0, max);
  } catch {
    return '';
  }
}

function compactProviderDiagnostics(value, max = 2000) {
  const lines = String(value || '').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const out = [];
  const seen = new Set();
  for (const line of lines) {
    let projected = '';
    try {
      const parsed = JSON.parse(line);
      const level = cleanText(parsed && parsed.level, 40);
      const target = cleanText(parsed && parsed.target, 120);
      const message = cleanText(parsed && parsed.fields && parsed.fields.message, 300);
      projected = [level, target, message].filter(Boolean).join(' ');
    } catch {
      projected = cleanText(line.replace(/"path":"[^"]+"/g, '"path":"[redacted]"'), 500);
    }
    if (!projected) continue;
    const key = projected.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(projected);
    if (out.length >= 8) break;
  }
  return out.join('\n').slice(0, max);
}

function messageMeta(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  return {
    trace_id: cleanText(message.trace_id, 200),
    request_id: cleanText(message.request_id, 200),
    engine_id: cleanText(message.engine_id || 'codex_cli', 120),
    session_id: cleanText(message.session_id, 200),
    turn_id: cleanText(message.turn_id, 200),
  };
}

function activity(ctx, providerEventType, displayText, status = 'done') {
  return {
    type: 'agent_activity_event',
    ...messageMeta(ctx),
    activity_kind: 'runtime_activity',
    provider_event_type: providerEventType,
    status,
    display_text: cleanText(displayText, 1000),
  };
}

function parseJsonLines(text) {
  const out = [];
  for (const line of String(text || '').split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('{')) continue;
    try {
      out.push(JSON.parse(trimmed));
    } catch {}
  }
  return out;
}

function appendBounded(current, chunk, maxBytes = 65536) {
  const next = Buffer.concat([current, Buffer.from(chunk || '')]);
  return next.length > maxBytes ? next.subarray(next.length - maxBytes) : next;
}

function buildRequest(id, method, params = {}) {
  return { id, method, params };
}

function sendJson(child, message) {
  child.stdin.write(`${JSON.stringify(message)}\n`);
}

function findResponse(messages, id) {
  return messages.find((message) => String(message && message.id || '') === id && ('result' in message || 'error' in message)) || null;
}

function waitForResponse(child, messages, id, timeoutMs) {
  const started = Date.now();
  return new Promise((resolve) => {
    const timer = setInterval(() => {
      const response = findResponse(messages, id);
      if (response) {
        clearInterval(timer);
        resolve(response);
        return;
      }
      if (Date.now() - started >= timeoutMs || child.exitCode !== null) {
        clearInterval(timer);
        resolve(findResponse(messages, id));
      }
    }, 50);
  });
}

function waitForTurnCompletion(child, messages, timeoutMs) {
  const started = Date.now();
  return new Promise((resolve) => {
    const timer = setInterval(() => {
      if (messages.some((message) => message && message.method === 'turn/completed')) {
        clearInterval(timer);
        resolve(true);
        return;
      }
      if (Date.now() - started >= timeoutMs || child.exitCode !== null) {
        clearInterval(timer);
        resolve(false);
      }
    }, 100);
  });
}

function extractThreadId(response, messages) {
  const candidates = [
    response && response.result && response.result.threadId,
    response && response.result && response.result.thread_id,
    response && response.result && response.result.id,
    response && response.result && response.result.thread && response.result.thread.id,
    ...messages.map((message) => message && message.params && message.params.threadId),
    ...messages.map((message) => message && message.params && message.params.thread_id),
  ];
  return cleanText(candidates.find(Boolean) || '', 200);
}

function collectTextFragments(value, out = [], depth = 0) {
  if (depth > 6 || out.length > 24 || value == null) return out;
  if (typeof value === 'string') {
    const text = cleanText(value, 4000);
    if (
      text &&
      text.length > 2 &&
      !/^(thread\/|turn\/|initialized|initialize|request|response)$/i.test(text) &&
      !/^[a-z0-9_.:/-]{1,80}$/i.test(text)
    ) {
      out.push(text);
    }
    return out;
  }
  if (Array.isArray(value)) {
    for (const item of value) collectTextFragments(item, out, depth + 1);
    return out;
  }
  if (typeof value === 'object') {
    for (const key of ['text', 'content', 'message', 'output', 'summary', 'delta', 'value']) {
      if (Object.prototype.hasOwnProperty.call(value, key)) collectTextFragments(value[key], out, depth + 1);
    }
    for (const key of Object.keys(value)) {
      if (['id', 'method', 'type', 'role', 'threadId', 'thread_id', 'turn_id', 'request_id'].includes(key)) continue;
      collectTextFragments(value[key], out, depth + 1);
    }
  }
  return out;
}

function outputTextFromMessages(messages, fallback) {
  const agentDeltas = messages
    .filter((message) => message && message.method === 'item/agentMessage/delta')
    .map((message) => cleanText(message && message.params && message.params.delta, 4000))
    .filter(Boolean);
  if (agentDeltas.length) {
    return agentDeltas.join('').trim().slice(0, 24000);
  }
  const fragments = [];
  for (const message of messages.slice(-120)) collectTextFragments(message, fragments);
  const unique = [];
  const seen = new Set();
  for (const fragment of fragments) {
    const key = fragment.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(fragment);
  }
  const text = unique.slice(-8).join('\n\n').trim();
  return text || cleanText(fallback, 4000);
}

function additionalContextFromTurn(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  const contextPack = message.context_pack && typeof message.context_pack === 'object' ? message.context_pack : {};
  const structuredProjection = contextPack.structured_transport && typeof contextPack.structured_transport === 'object'
    ? contextPack.structured_transport
    : {};
  return {
    'infring://context/pack': {
      kind: 'application',
      value: compactJson({
        source_authority: contextPack.source_authority || contextPack.source_basis || 'gateway_agent_runtime_context_pack',
        row_count: contextPack.row_count,
        fanout_target: contextPack.fanout_target,
        frontier: contextPack.frontier,
        fragments: Array.isArray(contextPack.fragments) ? contextPack.fragments.slice(-12) : [],
      }),
    },
    'infring://context/structured-turn-ref': {
      kind: 'application',
      value: cleanText(structuredProjection.structured_turn_ref || message.structured_turn_ref || '', 1000),
    },
    'infring://runtime/universal-tool-grants': {
      kind: 'application',
      value: compactJson(contextPack.universal_tool_grants || message.universal_tool_grants || {}),
    },
  };
}

async function runCodexAppServerStructuredTurn(ctx, options = {}) {
  const command = cleanText(options.command || 'codex', 500);
  const prompt = cleanText(options.prompt || '', 24000);
  const cwd = cleanText(options.cwd || process.cwd(), 1000) || process.cwd();
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 60000, 300000));
  const meta = messageMeta(ctx);
  const child = childProcess.spawn(command, ['app-server', '--stdio'], {
    cwd,
    env: { ...process.env },
    shell: false,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  const messages = [];
  let stdout = Buffer.alloc(0);
  let stderr = Buffer.alloc(0);
  let exitCode = null;
  const activityEvents = [
    activity(ctx, 'codex_app_server.native_transport.launch', 'Starting Codex app-server native transport.', 'running'),
  ];
  child.stdout.on('data', (chunk) => {
    stdout = appendBounded(stdout, chunk);
    messages.push(...parseJsonLines(String(chunk || '')));
  });
  child.stderr.on('data', (chunk) => {
    stderr = appendBounded(stderr, chunk);
  });
  child.on('exit', (code) => {
    exitCode = code;
  });
  const killTimer = setTimeout(() => {
    try { child.kill('SIGTERM'); } catch {}
  }, timeoutMs + 2000);
  try {
    const initializeId = `${meta.turn_id || meta.request_id || 'turn'}:initialize`;
    sendJson(child, buildRequest(initializeId, 'initialize', {
      clientInfo: {
        name: 'infring-agent-runtime',
        title: 'InfRing Agent Runtime',
        version: '0.0.0-adapter',
      },
      capabilities: {
        experimentalApi: true,
        requestAttestation: false,
        optOutNotificationMethods: [],
      },
    }));
    const initializeResponse = await waitForResponse(child, messages, initializeId, Math.min(10000, timeoutMs));
    sendJson(child, { method: 'initialized' });
    if (!initializeResponse || initializeResponse.error) {
      const reason = cleanText(initializeResponse && initializeResponse.error && initializeResponse.error.message, 1000) ||
        'Codex app-server did not accept initialize.';
      return {
        status: 'failed',
        error_code: 'codex_cli_native_transport_initialize_failed',
        reason,
        output_text: reason,
        activity_events: activityEvents,
        stderr_preview: compactProviderDiagnostics(stderr.toString('utf8'), 2000),
        exit_code: exitCode,
        native_transport_mapping_status: 'initialize_failed',
      };
    }
    activityEvents.push(activity(ctx, 'codex_app_server.native_transport.initialized', 'Codex app-server initialized.'));

    const threadStartId = `${meta.turn_id || meta.request_id || 'turn'}:thread-start`;
    sendJson(child, buildRequest(threadStartId, 'thread/start', {
      cwd,
      ephemeral: false,
      approvalPolicy: 'never',
      approvalsReviewer: 'user',
      sandbox: 'read-only',
      runtimeWorkspaceRoots: [cwd],
      baseInstructions: 'InfRing Agent Runtime native transport. Follow the user turn and do not mutate files unless the host has granted write authority.',
      developerInstructions: 'Use InfRing-supplied additionalContext as bounded context refs. Durable effects remain governed by InfRing approvals and receipts.',
    }));
    const threadResponse = await waitForResponse(child, messages, threadStartId, Math.min(15000, timeoutMs));
    const threadId = extractThreadId(threadResponse, messages);
    if (!threadId || threadResponse && threadResponse.error) {
      const reason = cleanText(threadResponse && threadResponse.error && threadResponse.error.message, 1000) ||
        'Codex app-server did not return a thread id.';
      return {
        status: 'failed',
        error_code: 'codex_cli_native_transport_thread_failed',
        reason,
        output_text: reason,
        activity_events: activityEvents,
        stderr_preview: compactProviderDiagnostics(stderr.toString('utf8'), 2000),
        exit_code: exitCode,
        native_transport_mapping_status: 'thread_start_failed',
      };
    }
    activityEvents.push(activity(ctx, 'codex_app_server.native_transport.thread_started', 'Codex app-server thread started.'));

    const turnStartId = `${meta.turn_id || meta.request_id || 'turn'}:turn-start`;
    sendJson(child, buildRequest(turnStartId, 'turn/start', {
      threadId,
      cwd,
      approvalPolicy: 'never',
      sandboxPolicy: {
        type: 'readOnly',
        networkAccess: false,
      },
      input: [{ type: 'text', text: prompt }],
      additionalContext: additionalContextFromTurn(ctx),
      responsesapiClientMetadata: {
        infring_trace_id: meta.trace_id,
        infring_request_id: meta.request_id,
        infring_session_id: meta.session_id,
        infring_turn_id: meta.turn_id,
      },
    }));
    const turnResponse = await waitForResponse(child, messages, turnStartId, Math.min(timeoutMs, 60000));
    const turnCompleted = await waitForTurnCompletion(child, messages, Math.min(timeoutMs, 60000));
    const stderrText = compactProviderDiagnostics(stderr.toString('utf8'), 2000);
    if (!turnResponse || turnResponse.error) {
      const reason = cleanText(turnResponse && turnResponse.error && turnResponse.error.message, 1000) ||
        stderrText ||
        'Codex app-server did not accept the native turn.';
      return {
        status: 'failed',
        error_code: 'codex_cli_native_transport_turn_failed',
        reason,
        output_text: reason,
        activity_events: activityEvents,
        stderr_preview: stderrText,
        exit_code: exitCode,
        native_transport_mapping_status: 'turn_start_failed',
      };
    }
    activityEvents.push(activity(
      ctx,
      turnCompleted
        ? 'codex_app_server.native_transport.turn_completed'
        : 'codex_app_server.native_transport.turn_submitted',
      turnCompleted
        ? 'Codex app-server native turn completed.'
        : 'Codex app-server native turn submitted; completion event was not observed before the bounded timeout.',
    ));
    const outputText = outputTextFromMessages(
      messages,
      turnCompleted
        ? 'Codex app-server completed the native structured turn without emitting bounded assistant text.'
        : 'Codex app-server accepted the native structured turn but did not complete before the bounded wait window.',
    );
    return {
      status: turnCompleted ? 'completed' : 'timed_out',
      error_code: turnCompleted ? '' : 'codex_cli_native_transport_completion_not_observed',
      reason: turnCompleted ? '' : 'Codex app-server accepted the native structured turn but did not emit turn/completed before the bounded wait window.',
      retryable: !turnCompleted,
      output_text: outputText,
      activity_events: activityEvents,
      stderr_preview: stderrText,
      exit_code: exitCode,
      native_transport_mapping_status: 'native_transport_adapter_path_active',
    };
  } finally {
    clearTimeout(killTimer);
    try { child.stdin.end(); } catch {}
    try { child.kill('SIGTERM'); } catch {}
  }
}

module.exports = {
  runCodexAppServerStructuredTurn,
};
