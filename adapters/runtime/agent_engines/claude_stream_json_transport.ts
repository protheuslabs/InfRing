#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::claude-stream-json-transport.
//
// Adapter-private translator for Claude Code stream-json input/output. Gateway
// remains the authority for context construction, route admission, approvals,
// receipts, and projection bounds; this module only maps an already-built Agent
// Runtime turn onto Claude's private stream-json CLI transport.

'use strict';

const {
  nativeDirectMutationGrantActive,
  parseCliActivityOutput,
  selectedRuntimeModelArg,
  spawnActivityCapture,
} = require('./cli_runtime_adapter.ts');

function cleanText(value, max = 2000) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
    .replace(/\r\n/g, '\n')
    .trim()
    .slice(0, max);
}

function messageMeta(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  return {
    trace_id: cleanText(message.trace_id, 200),
    request_id: cleanText(message.request_id, 200),
    engine_id: cleanText(message.engine_id || 'claude_code', 120),
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

function compactProviderDiagnostics(value, max = 2000) {
  const lines = String(value || '').split(/\r?\n/).map((line) => cleanText(line, 600)).filter(Boolean);
  const out = [];
  const seen = new Set();
  for (const line of lines) {
    const projected = line
      .replace(/\/Users\/[^/\s]+/g, '/Users/[redacted]')
      .replace(/"path":"[^"]+"/g, '"path":"[redacted]"');
    const key = projected.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(projected);
    if (out.length >= 8) break;
  }
  return out.join('\n').slice(0, max);
}

function claudeNativePermissionArgs(ctx) {
  const mutationGrant = nativeDirectMutationGrantActive(ctx);
  return [
    '--permission-mode',
    mutationGrant ? 'acceptEdits' : 'default',
    ...(mutationGrant ? ['--allowedTools', 'Read,Write,Edit,Bash'] : []),
  ];
}

function claudeStreamJsonArgs(ctx) {
  const modelArg = selectedRuntimeModelArg(ctx, ['claude_code', 'claude', 'anthropic']);
  return [
    '--print',
    '--input-format',
    'stream-json',
    '--output-format',
    'stream-json',
    '--verbose',
    '--include-partial-messages',
    '--include-hook-events',
    ...claudeNativePermissionArgs(ctx),
    ...(modelArg ? ['--model', modelArg] : []),
  ];
}

function buildClaudeStreamJsonInput(prompt) {
  const frame = {
    type: 'user',
    message: {
      role: 'user',
      content: cleanText(prompt, 24000),
    },
    parent_tool_use_id: null,
  };
  return `${JSON.stringify(frame)}\n`;
}

async function runClaudeStreamJsonStructuredTurn(ctx, options = {}) {
  const command = cleanText(options.command || 'claude', 500);
  const cwd = cleanText(options.cwd || process.cwd(), 1000) || process.cwd();
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 60000, 300000));
  const activityEvents = [
    activity(ctx, 'claude_stream_json.native_transport.launch', 'Starting Claude Code stream-json native transport.', 'running'),
  ];
  const run = await spawnActivityCapture(command, claudeStreamJsonArgs(ctx), {
    cwd,
    timeoutMs,
    maxOutputBytes: 262144,
    stdin: buildClaudeStreamJsonInput(options.prompt || ''),
    ctx,
    engineId: 'claude_code',
    onActivity: (event) => {
      if (event) activityEvents.push(event);
      if (ctx && typeof ctx.onActivity === 'function') ctx.onActivity(event);
    },
    abortSignal: options.abortSignal,
  });
  const parsed = parseCliActivityOutput(run.stdout, run.stderr, ctx, 'claude_code');
  const outputText = cleanText(parsed && parsed.output_text, 24000);
  const stderrPreview = compactProviderDiagnostics(run.stderr, 2000);
  const failedReason = outputText || stderrPreview || (run.timed_out ? 'Claude Code stream-json native turn timed out.' : 'Claude Code stream-json native turn failed.');
  if (!run.ok) {
    return {
      status: run.timed_out ? 'timed_out' : 'failed',
      error_code: run.timed_out ? 'claude_code_native_transport_timeout' : 'claude_code_native_transport_failed',
      reason: failedReason,
      retryable: run.timed_out === true,
      output_text: failedReason,
      activity_events: activityEvents.slice(-80),
      stderr_preview: stderrPreview,
      exit_code: run.exit_code,
      timed_out: run.timed_out === true,
      native_transport_mapping_status: 'native_transport_adapter_path_failed',
      permission_request: parsed && parsed.permission_request,
    };
  }
  activityEvents.push(activity(ctx, 'claude_stream_json.native_transport.completed', 'Claude Code stream-json native turn completed.'));
  return {
    status: 'completed',
    error_code: '',
    reason: '',
    retryable: false,
    output_text: outputText || 'Claude Code completed the native stream-json turn without emitting bounded assistant text.',
    activity_events: (activityEvents.concat(Array.isArray(parsed && parsed.activity_events) ? parsed.activity_events : [])).slice(-80),
    stderr_preview: stderrPreview,
    exit_code: run.exit_code,
    timed_out: run.timed_out === true,
    native_transport_mapping_status: 'native_transport_adapter_path_active',
    permission_request: parsed && parsed.permission_request,
  };
}

module.exports = {
  buildClaudeStreamJsonInput,
  claudeStreamJsonArgs,
  runClaudeStreamJsonStructuredTurn,
};
