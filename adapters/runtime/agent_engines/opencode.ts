#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::opencode.
//
// OpenCode is a CLI-backed external runtime seam. InfRing discovers and runs it
// through Gateway adapter projection only; Shell must not probe or execute it.

'use strict';

const fs = require('fs');
const path = require('path');
const { createCliRuntimeEngineAdapter, selectedRuntimeModelArg } = require('./cli_runtime_adapter.ts');

const DIRECT_NATIVE_MUTATION_GRANTS = new Set(['direct_file_write', 'native.direct_file_write', 'filesystem.direct_write']);
const opencodeNativeSessionByInfRingSession = new Map();
const OPENCODE_SESSION_STORE_PATH = path.resolve(
  process.cwd(),
  'core/local/state/observability/agent_runtime/opencode_native_sessions.json',
);

function infRingSessionKey(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  return String(
    message.session_id ||
      message.conversation_id ||
      message.thread_id ||
      (ctx && (ctx.session_id || ctx.conversation_id || ctx.thread_id)) ||
      '',
  ).trim();
}

function readOpenCodeSessionStore() {
  try {
    const parsed = JSON.parse(fs.readFileSync(OPENCODE_SESSION_STORE_PATH, 'utf8'));
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function writeOpenCodeSessionStore(store) {
  try {
    fs.mkdirSync(path.dirname(OPENCODE_SESSION_STORE_PATH), { recursive: true });
    fs.writeFileSync(OPENCODE_SESSION_STORE_PATH, `${JSON.stringify(store, null, 2)}\n`, 'utf8');
  } catch {}
}

function rememberOpenCodeNativeSession(key, sessionId) {
  if (!key || !sessionId) return;
  opencodeNativeSessionByInfRingSession.set(key, sessionId);
  const store = readOpenCodeSessionStore();
  store[key] = {
    session_id: sessionId,
    updated_at: new Date().toISOString(),
    source: 'opencode_adapter_native_session_bridge',
  };
  writeOpenCodeSessionStore(store);
}

function lookupOpenCodeNativeSession(key) {
  if (!key) return '';
  const cached = opencodeNativeSessionByInfRingSession.get(key);
  if (cached) return cached;
  const row = readOpenCodeSessionStore()[key];
  const sessionId = String(row && row.session_id || '').trim();
  if (sessionId) opencodeNativeSessionByInfRingSession.set(key, sessionId);
  return sessionId;
}

function shouldRetryOpenCodeNoAssistantOutput(result) {
  const row = result && typeof result === 'object' ? result : {};
  const reason = String(row.reason || row.output_text || row.output_preview || '').toLowerCase();
  return row.status === 'failed' &&
    String(row.error_code || '') === 'opencode_turn_failed' &&
    reason.includes('completed without a usable assistant response') &&
    row.permission_request == null &&
    row.timed_out !== true;
}

function opencodeRetryEvent(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  return {
    type: 'agent_activity_event',
    activity_kind: 'activity',
    provider_event_type: 'opencode.no_assistant_output.retry',
    source: 'opencode_adapter_retry_policy',
    sequence_no: 0,
    item_id: 'opencode-no-assistant-output-retry',
    status: 'completed',
    text: 'OpenCode exited without assistant text; retrying once in the same native session.',
    display_text: 'OpenCode exited without assistant text; retrying once in the same native session.',
    engine_id: 'opencode',
    trace_id: String(message.trace_id || ''),
    session_id: String(message.session_id || ''),
    turn_id: String(message.turn_id || ''),
  };
}

function captureOpenCodeNativeSession(run, ctx) {
  const key = infRingSessionKey(ctx);
  if (!key || !run || !run.stdout) return;
  for (const line of String(run.stdout || '').split(/\r?\n/)) {
    const text = line.trim();
    if (!text) continue;
    let row = null;
    try { row = JSON.parse(text); } catch { continue; }
    const sessionId = String(
      (row && row.sessionID) ||
        (row && row.sessionId) ||
        (row && row.session_id) ||
        (row && row.part && (row.part.sessionID || row.part.sessionId || row.part.session_id)) ||
        '',
    ).trim();
    if (!sessionId) continue;
    rememberOpenCodeNativeSession(key, sessionId);
    if (opencodeNativeSessionByInfRingSession.size > 200) {
      const firstKey = opencodeNativeSessionByInfRingSession.keys().next().value;
      if (firstKey) opencodeNativeSessionByInfRingSession.delete(firstKey);
    }
    return;
  }
}

function mutationGrantActive(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  const approvalPolicy = String(message.approval_policy || '').trim();
  if (approvalPolicy === 'simulate_allow' || approvalPolicy === 'manual_or_simulate_allow') return true;
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  return always.some((toolId) => DIRECT_NATIVE_MUTATION_GRANTS.has(String(toolId || '').trim()));
}

function runtimeAttachmentReadPaths(ctx) {
  const refs = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.runtime_attachment_refs;
  const rows = refs && Array.isArray(refs.attachments) ? refs.attachments : [];
  const out = [];
  const seen = new Set();
  for (const row of rows) {
    const item = row && typeof row === 'object' ? row : {};
    const readPath = String(item.local_read_path || item.read_path || '').trim();
    if (!readPath || seen.has(readPath)) continue;
    seen.add(readPath);
    out.push(readPath);
    if (out.length >= 8) break;
  }
  return out;
}

function runtimeWorkingDirectory(ctx) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  return String(
    message.working_directory ||
      message.current_working_directory ||
      message.present_working_directory ||
      message.cwd ||
      '',
  ).trim();
}

function createOpenCodeEngineAdapter(options = {}) {
  const adapter = createCliRuntimeEngineAdapter({
    engineId: 'opencode',
    contextTransportMode: 'bounded_cli_arg_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'structured_source_bounded_cli_arg_active',
    command: options.command || process.env.INFRING_OPENCODE_BIN || process.env.INFRING_OPENCODE_PATH,
    commandFallback: 'opencode',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_OPENCODE_LIVE',
    downloadActionRef: 'agent_runtime_download/opencode',
    artifactKind: 'opencode_result_projection',
    receiptKind: 'opencode_adapter_receipt',
    versionArgs: ['--version'],
    modelDiscovery: {
      kind: 'opencode_models_command',
      args: ['models'],
      source: 'opencode_models_command',
      freshnessAuthority: 'opencode_models_command',
      timeoutMs: 8000,
      maxOutputBytes: 1048576,
    },
    runArgs: (prompt, ctx) => {
      const modelArg = selectedRuntimeModelArg(ctx, ['opencode', 'openrouter', 'openai', 'anthropic', 'xai', 'google', 'gemini']);
      const mutationGrant = mutationGrantActive(ctx);
      const attachmentPaths = runtimeAttachmentReadPaths(ctx);
      const cwd = runtimeWorkingDirectory(ctx);
      const nativeSessionId = lookupOpenCodeNativeSession(infRingSessionKey(ctx));
      return [
        'run',
        '--format',
        'json',
        ...(cwd ? ['--dir', cwd] : []),
        ...(nativeSessionId ? ['--session', nativeSessionId] : []),
        ...(modelArg ? ['--model', modelArg] : []),
        ...(mutationGrant ? ['--dangerously-skip-permissions'] : []),
        prompt,
        ...attachmentPaths.flatMap((readPath) => ['--file', readPath]),
      ];
    },
    promptBuilder: ({ ctx, current }) => {
      void ctx;
      return current;
    },
    stableShadowWorkspaceKey: (ctx) => infRingSessionKey(ctx),
    afterRun: captureOpenCodeNativeSession,
    ...options,
  });
  return {
    ...adapter,
    async submit_turn(ctx) {
      const first = await adapter.submit_turn(ctx);
      if (!shouldRetryOpenCodeNoAssistantOutput(first)) return first;
      const second = await adapter.submit_turn(ctx);
      const retryEvent = opencodeRetryEvent(ctx);
      return {
        ...second,
        opencode_no_assistant_output_retry_attempted: true,
        activity_events: [
          ...(Array.isArray(first.activity_events) ? first.activity_events : []),
          retryEvent,
          ...(Array.isArray(second.activity_events) ? second.activity_events : []),
        ].slice(-80),
      };
    },
  };
}

module.exports = {
  createOpenCodeEngineAdapter,
};
