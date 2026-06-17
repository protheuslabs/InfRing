#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::turn-projection.
//
// Gateway owns the Shell-facing Agent Runtime turn projection. The legacy
// dashboard host may still serve HTTP while it is being retired, but turn
// normalization, context materialization, activity projection, permission pause
// shaping, and bounded output packaging belong to the Gateway membrane.

'use strict';

const path = require('node:path');
const { normalizeAgentRuntimeTurnInput: defaultNormalizeAgentRuntimeTurnInput } = require('../agent_runtime_input_normalizer.ts');
const { buildUniversalToolGrants: defaultBuildUniversalToolGrants } = require('./universal_core_tools.ts');
const { buildAgentRuntimeStructuredTurn: defaultBuildAgentRuntimeStructuredTurn } = require('./agent_runtime_structured_transport.ts');
const {
  resolveAgentRuntimeEngineId,
} = require('./agent_runtime_engine_identity.ts');

const DEFAULT_CONTEXT_FANOUT_TARGET = 7;
const AGENT_RUNTIME_FALLBACK_CONTEXT_ROW_LIMIT = 24;

function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanApprovalId(value) { return cleanText(value, 260).replace(/[^a-zA-Z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanReceiptComponent(value, maxLen = 200) { return cleanText(value, maxLen).replace(/[^A-Za-z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, '') || 'unknown'; }
function cleanWorkingDirectory(value) {
  const raw = String(value == null ? '' : value).replace(/\0/g, '').trim();
  if (!raw || raw.startsWith('~')) return '';
  return path.resolve(raw);
}

function agentRuntimeContextRowKey(row) {
  if (!row || typeof row !== 'object') return '';
  const stableRef = cleanText(row.source_ref || row.ref || row.id || row.message_id || '', 240);
  if (stableRef) return stableRef;
  const role = cleanText(row.role || row.origin_kind || row.speaker || '', 64).toLowerCase();
  const kind = cleanText(row.source_kind || row.record_type || row.type || '', 96).toLowerCase();
  const text = cleanDisplayText(row.text_preview || row.preview || row.text || row.content || '', 800)
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .trim();
  return role || kind || text ? `${role}:${kind}:${text}` : '';
}

function boundAgentRuntimeFallbackContextRows(rows) {
  const sourceRows = Array.isArray(rows) ? rows : [];
  const selected = [];
  const seen = new Set();
  for (let idx = sourceRows.length - 1; idx >= 0; idx -= 1) {
    const row = sourceRows[idx];
    const key = agentRuntimeContextRowKey(row);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    selected.push(row);
    if (selected.length >= AGENT_RUNTIME_FALLBACK_CONTEXT_ROW_LIMIT) break;
  }
  return selected.reverse();
}

function parseRawProviderActivityText(value) {
  const text = cleanDisplayText(value, 12000);
  if (!text || !/^[{[]/.test(text)) return null;
  try {
    const parsed = JSON.parse(text);
    return parsed && typeof parsed === 'object' ? parsed : null;
  } catch {
    return null;
  }
}

function firstActivityText(source, keys, maxLen = 500) {
  const obj = source && typeof source === 'object' ? source : {};
  for (const key of keys) {
    const value = obj[key];
    if (Array.isArray(value)) {
      const joined = value.map((item) => cleanText(item, 120)).filter(Boolean).join(' ');
      if (joined) return cleanDisplayText(joined, maxLen);
    } else if (value && typeof value === 'object') {
      continue;
    } else {
      const text = cleanDisplayText(value, maxLen);
      if (text) return text;
    }
  }
  return '';
}

function activityStatusPrefix(status) {
  const cleaned = cleanText(status, 80).toLowerCase();
  if (/complete|done|success/.test(cleaned)) return 'done';
  if (/fail|error/.test(cleaned)) return 'failed';
  return 'running';
}

function compactRawProviderActivityText(raw, event = {}) {
  const row = raw && typeof raw === 'object' ? raw : {};
  const item = row.item && typeof row.item === 'object' ? row.item : row;
  const input = item.input && typeof item.input === 'object'
    ? item.input
    : row.input && typeof row.input === 'object'
      ? row.input
      : {};
  const args = item.args && typeof item.args === 'object'
    ? item.args
    : row.args && typeof row.args === 'object'
      ? row.args
      : {};
  const toolObj = item.tool && typeof item.tool === 'object'
    ? item.tool
    : row.tool && typeof row.tool === 'object'
      ? row.tool
      : {};
  const providerType = cleanText(row.type || row.event_type || row.provider_event_type || event.provider_event_type, 160);
  const itemType = cleanText(item.type || item.kind || item.name || '', 120).toLowerCase();
  const status = cleanText(row.status || row.state || item.status || item.state || event.status, 80);
  const prefix = activityStatusPrefix(status);
  const command = firstActivityText(item, ['command', 'cmd', 'shell_command', 'argv', 'args'], 800) ||
    firstActivityText(input, ['command', 'cmd', 'shell_command', 'argv', 'args'], 800) ||
    firstActivityText(args, ['command', 'cmd', 'shell_command', 'argv', 'args'], 800) ||
    firstActivityText(row, ['command', 'cmd', 'shell_command', 'argv', 'args'], 800);
  if (command || /command|exec|shell/.test(itemType || providerType)) {
    const target = command || 'shell command';
    return prefix === 'failed' ? `failed running ${target}` : prefix === 'done' ? `ran ${target}` : `running ${target}`;
  }
  const pathTarget = firstActivityText(item, ['path', 'file', 'filename', 'file_path', 'target_path', 'target', 'uri'], 500) ||
    firstActivityText(input, ['path', 'file', 'filename', 'file_path', 'target_path', 'target', 'uri'], 500) ||
    firstActivityText(args, ['path', 'file', 'filename', 'file_path', 'target_path', 'target', 'uri'], 500) ||
    firstActivityText(row, ['path', 'file', 'filename', 'file_path', 'target_path', 'target', 'uri'], 500);
  if (pathTarget || /file|edit|patch|change|write/.test(itemType || providerType)) {
    const target = pathTarget || 'file';
    return prefix === 'failed' ? `failed writing ${target}` : prefix === 'done' ? `wrote ${target}` : `writing ${target}`;
  }
  const tool = firstActivityText(item, ['tool_id', 'tool', 'name', 'function'], 300) ||
    firstActivityText(toolObj, ['tool_id', 'name', 'function'], 300) ||
    firstActivityText(input, ['tool_id', 'tool', 'name', 'function'], 300) ||
    firstActivityText(args, ['tool_id', 'tool', 'name', 'function'], 300) ||
    firstActivityText(row, ['tool_id', 'tool', 'name', 'function'], 300);
  if (tool || /tool|function/.test(itemType || providerType)) {
    const target = tool || 'tool call';
    return prefix === 'failed' ? `failed running ${target}` : prefix === 'done' ? `ran ${target}` : `running ${target}`;
  }
  const query = firstActivityText(item, ['query', 'pattern', 'search'], 500) ||
    firstActivityText(input, ['query', 'pattern', 'search'], 500) ||
    firstActivityText(args, ['query', 'pattern', 'search'], 500) ||
    firstActivityText(row, ['query', 'pattern', 'search'], 500);
  if (query || /search|grep|rg/.test(itemType || providerType)) {
    const target = query || 'workspace search';
    return prefix === 'failed' ? `failed searching ${target}` : prefix === 'done' ? `searched ${target}` : `searching ${target}`;
  }
  const provider = providerType.toLowerCase();
  if (provider.includes('thread.started')) return 'Runtime thread started.';
  if (provider.includes('turn.started')) return 'Runtime turn started.';
  if (provider.includes('turn.completed')) return 'Runtime completed the turn.';
  if (providerType) return `Runtime event: ${providerType}`;
  return '';
}

function shouldDisplayActivityInThinkingBubble(event, displayText, providerEventType) {
  const row = event && typeof event === 'object' ? event : {};
  if (row.display_in_thinking_bubble === false || row.thinking_bubble_visible === false) return false;
  const provider = cleanText(providerEventType || row.provider_event_type || row.event_type || row.type, 160).toLowerCase();
  const kind = cleanText(row.activity_kind || row.kind || row.type, 80).toLowerCase();
  const text = cleanDisplayText(displayText || row.display_text || row.text || row.summary || '', 1000).toLowerCase();
  const joined = `${provider} ${kind} ${text}`;
  if (
    kind === 'assistant_delta' ||
    kind === 'completed' ||
    provider === 'end' ||
    provider.includes('turn.completed') ||
    /completed the turn|final answer is shown in the message|assistant draft streamed/.test(text)
  ) {
    return false;
  }
  if (
    joined.includes('/.claude/plugins/cache/') ||
    joined.includes('\\.claude\\plugins\\cache\\') ||
    joined.includes('claude-plugins-official') ||
    joined.includes('rust-analyzer-lsp@claude-plugins-official')
  ) {
    return false;
  }
  if (/decision|reasoning|thought|plan|permission|approval|error|failed/.test(joined)) return true;
  if (/command|exec|shell|bash|tool|mcp|function|file|edit|patch|diff|write|search|grep|find/.test(joined)) return true;
  if (
    provider === 'external_cli.launch' ||
    provider.includes('context.') ||
    provider.includes('availability') ||
    provider.includes('health') ||
    provider.includes('session.') ||
    provider.includes('prepare') ||
    provider.includes('launch') ||
    provider.includes('thread.started') ||
    provider.includes('turn.started') ||
    provider.includes('turn.completed')
  ) {
    return false;
  }
  if (
    /^preparing\b/.test(text) ||
    /^loaded \d+ prior context row/.test(text) ||
    /^checking .* availability/.test(text) ||
    /^starting .* session/.test(text) ||
    /^launching .* turn with bounded context pack/.test(text) ||
    /^launching .* cli\b/.test(text) ||
    /^runtime thread started/.test(text) ||
    /^runtime turn started/.test(text) ||
    /^runtime completed the turn/.test(text)
  ) {
    return false;
  }
  return true;
}

function isProgressOnlyActivity(event) {
  const row = event && typeof event === 'object' ? event : {};
  const provider = cleanText(row.provider_event_type || row.event_type || row.type, 160).toLowerCase();
  const kind = cleanText(row.activity_kind || row.kind || row.type, 80).toLowerCase();
  return row.progress_only === true ||
    row.persist_in_activity_trace === false ||
    provider === 'external_cli.process_alive' ||
    kind === 'runtime_progress';
}

function sanitizeAgentRuntimeActivityEvent(row, index, defaults = {}) {
  const event = row && typeof row === 'object' ? row : {};
  const rawActivity = parseRawProviderActivityText(event.display_text || event.text || event.summary || '');
  const compactText = rawActivity ? compactRawProviderActivityText(rawActivity, event) : '';
  const providerEventType = cleanText(
    event.provider_event_type || event.provider_type || event.event_type || (rawActivity && rawActivity.type),
    160,
  );
  const displayText = cleanDisplayText(
    compactText || event.display_text || event.text || event.summary || '',
    4000,
  );
  return {
    type: 'agent_activity_event',
    activity_kind: cleanText(event.activity_kind || event.kind || event.type, 80) || 'activity',
    provider_event_type: providerEventType,
    source: cleanText(event.source || 'external_cli_stream', 120),
    sequence_no: Number(event.sequence_no || index + 1) || index + 1,
    item_id: cleanText(event.item_id || event.itemId || '', 200),
    status: cleanText(event.status || '', 80),
    text: displayText,
    display_text: displayText,
    display_in_thinking_bubble: shouldDisplayActivityInThinkingBubble(event, displayText, providerEventType),
    persist_in_activity_trace: event.persist_in_activity_trace !== false,
    progress_only: event.progress_only === true,
    role: cleanText(event.role || '', 40),
    timeline_role: cleanText(event.timeline_role || '', 80),
    steering_id: cleanText(event.steering_id || '', 200),
    user_text: cleanDisplayText(event.user_text || '', 4000),
    user_text_preview: cleanDisplayText(event.user_text_preview || '', 1000),
    receipt_ref: cleanText(event.receipt_ref || '', 240),
    result_ref: cleanText(event.result_ref || '', 240),
    engine_id: cleanEngineId(event.engine_id || defaults.engineId),
    trace_id: cleanText(event.trace_id || defaults.traceId, 200),
    session_id: cleanText(event.session_id || defaults.sessionId, 200),
    turn_id: cleanText(event.turn_id || defaults.turnId, 200),
  };
}

function traceReplacementInfo(title) {
  const text = cleanDisplayText(title, 1000);
  const checks = [
    { kind: 'run', active: /^running\s+(.+)$/i, final: /^(ran|failed running)\s+(.+)$/i },
    { kind: 'write', active: /^writing\s+(.+)$/i, final: /^(wrote|failed writing)\s+(.+)$/i },
    { kind: 'read', active: /^reading\s+(.+)$/i, final: /^(read|failed reading)\s+(.+)$/i },
    { kind: 'search', active: /^searching\s+(.+)$/i, final: /^(searched|failed searching)\s+(.+)$/i },
  ];
  for (const check of checks) {
    const active = text.match(check.active);
    if (active) return { phase: 'active', key: `${check.kind}:${cleanText(active[1], 900).toLowerCase()}` };
    const final = text.match(check.final);
    if (final) return { phase: 'final', key: `${check.kind}:${cleanText(final[2], 900).toLowerCase()}` };
  }
  return null;
}

function traceFileActionInfo(row) {
  const source = row && typeof row === 'object' ? row : {};
  const title = cleanDisplayText(source.title || source.display_text || source.text, 1000);
  if (!title) return null;
  const checks = [
    { kind: 'write', match: /^(?:writing|wrote|failed writing)\s+(.+)$/i },
    { kind: 'read', match: /^(?:reading|read|failed reading)\s+(.+)$/i },
  ];
  for (const check of checks) {
    const matched = title.match(check.match);
    if (!matched || !matched[1]) continue;
    const target = cleanDisplayText(matched[1], 900);
    if (!target) continue;
    return { kind: check.kind, target };
  }
  return null;
}

function traceFileActionVerb(kind) {
  return cleanText(kind, 40) === 'write' ? 'Edited' : 'Read';
}

function traceFileActionState(row) {
  const source = row && typeof row === 'object' ? row : {};
  const title = cleanDisplayText(source.title || source.display_text || source.text, 1000).toLowerCase();
  const status = cleanText(source.status, 80);
  if (/^failed\s+/.test(title) || /fail|error/.test(status.toLowerCase())) return 'failed';
  if (/^(?:writing|reading)\s+/.test(title) || /start|run|progress/.test(status.toLowerCase())) return 'running';
  return 'completed';
}

function compactActivityTraceRows(rows) {
  const sourceRows = Array.isArray(rows) ? rows.filter(Boolean) : [];
  const finalKeys = new Set();
  let hasConcreteReadRow = false;
  for (const row of sourceRows) {
    const info = traceReplacementInfo(row && row.title);
    if (info && info.phase === 'final') finalKeys.add(info.key);
    if (info && info.key.startsWith('read:') && !/^read:(read|ls|glob|grep)$/i.test(info.key)) hasConcreteReadRow = true;
  }
  const filteredRows = sourceRows
    .filter((row) => {
      const title = cleanDisplayText(row && row.title, 1000);
      const lowerTitle = title.toLowerCase();
      if (
        /^preparing\b/.test(lowerTitle) ||
        /^loaded \d+ prior context row/.test(lowerTitle) ||
        /^checking .* availability/.test(lowerTitle) ||
        /^starting .* session/.test(lowerTitle) ||
        /^launching .* turn with bounded context pack/.test(lowerTitle) ||
        /^launching .* cli\b/.test(lowerTitle) ||
        /^runtime thread started/.test(lowerTitle) ||
        /^runtime turn started/.test(lowerTitle) ||
        /^runtime completed the turn/.test(lowerTitle)
      ) return false;
      if (hasConcreteReadRow && /^reading\s+(read|ls|glob|grep)$/i.test(title)) return false;
      const info = traceReplacementInfo(row && row.title);
      return !(info && info.phase === 'active' && finalKeys.has(info.key));
    })
    .map((row, index) => ({ ...row, sequence_no: index + 1 }));
  const compacted = [];
  let group = null;
  const flush = () => {
    if (!group) return;
    const count = group.children.length;
    const verb = traceFileActionVerb(group.kind);
    compacted.push({
      type: 'agent_runtime_activity_trace_row',
      sequence_no: compacted.length + 1,
      activity_kind: group.kind,
      provider_event_type: 'gateway.compacted_file_activity',
      status: group.status,
      title: `${verb} ${count} ${count === 1 ? 'file' : 'files'}`,
      display_in_thinking_bubble: true,
      detail_ref: group.detail_ref,
      line_kind: group.kind,
      children: group.children,
    });
    group = null;
  };
  for (const row of filteredRows) {
    const action = traceFileActionInfo(row);
    if (!action) {
      flush();
      compacted.push({ ...row, sequence_no: compacted.length + 1 });
      continue;
    }
    const state = traceFileActionState(row);
    const child = {
      ...row,
      sequence_no: 0,
      activity_kind: action.kind,
      status: state,
      title: `${traceFileActionVerb(action.kind)} ${action.target}`,
      line_kind: action.kind,
      activity_target: action.target,
      display_in_thinking_bubble: true,
    };
    if (!group || group.kind !== action.kind || group.status !== state) {
      flush();
      group = {
        kind: action.kind,
        status: state,
        detail_ref: row && row.detail_ref,
        children: [],
      };
    }
    group.children.push(child);
  }
  flush();
  return compacted.map((row, index) => ({
    ...row,
    sequence_no: index + 1,
    children: Array.isArray(row.children)
      ? row.children.map((child, childIndex) => ({ ...child, sequence_no: childIndex + 1 }))
      : undefined,
  }));
}

function normalizeModelProviderContext(body, engineId) {
  const source = body && body.model_provider_context && typeof body.model_provider_context === 'object'
    ? body.model_provider_context
    : {};
  const provider = cleanText(
    source.provider ||
    source.model_provider ||
    body && (body.model_provider || body.provider || body.selected_provider),
    120,
  ).toLowerCase();
  let model = cleanText(
    source.model ||
    source.model_name ||
    source.runtime_model ||
    source.selected_model ||
    body && (body.model || body.model_name || body.runtime_model || body.selected_model),
    240,
  );
  let qualified = cleanText(source.qualified_model_ref || source.id || '', 280);
  if (!qualified && provider && model) qualified = `${provider}/${model}`;
  if (!provider && qualified.includes('/')) {
    const parts = qualified.split('/');
    const inferredProvider = cleanText(parts.shift(), 120).toLowerCase();
    const inferredModel = cleanText(parts.join('/'), 240);
    return {
      type: 'agent_runtime_model_provider_context',
      source_authority: 'gateway_agent_runtime_model_provider_projection',
      provider: inferredProvider,
      model: model || inferredModel,
      qualified_model_ref: qualified,
      selected_runtime_engine_id: cleanEngineId(engineId),
      credential_materialization: 'adapter_env_from_gateway_secret_refs_or_process_env_only',
      credential_ref: cleanText(source.credential_ref || source.provider_credential_ref || '', 240),
      secrets_included: false,
    };
  }
  return {
    type: 'agent_runtime_model_provider_context',
    source_authority: 'gateway_agent_runtime_model_provider_projection',
    provider,
    model,
    qualified_model_ref: qualified,
    selected_runtime_engine_id: cleanEngineId(engineId),
    credential_materialization: 'adapter_env_from_gateway_secret_refs_or_process_env_only',
    credential_ref: cleanText(source.credential_ref || source.provider_credential_ref || '', 240),
    secrets_included: false,
  };
}

function classifyAgentRuntimePreTurnFailureCode(engineId, source, fallback = 'agent_runtime_engine_unavailable') {
  const cleanEngine = cleanEngineId(engineId) || 'agent_runtime';
  const text = cleanDisplayText([
    source && source.error,
    source && source.error_code,
    source && source.reason,
    source && source.status,
    source && source.provider_status,
    source && source.provider_reason,
    source && source.setup_action_ref,
    source && source.version_preview,
    source && source.stderr_preview,
    source && source.message,
  ].filter(Boolean).join('\n'), 12000).toLowerCase();
  if (
    text.includes('quota') ||
    text.includes('credit') ||
    text.includes('billing') ||
    text.includes('subscription') ||
    text.includes('payment required') ||
    text.includes('insufficient balance') ||
    text.includes('insufficient_quota') ||
    text.includes('resource_exhausted') ||
    text.includes('usage limit') ||
    text.includes('usage_limit') ||
    text.includes('usage cap') ||
    text.includes('usage_cap') ||
    text.includes('spend limit') ||
    text.includes('spending limit') ||
    text.includes('monthly limit') ||
    text.includes('daily limit') ||
    text.includes('weekly limit') ||
    text.includes('try again at') ||
    text.includes('out of tokens') ||
    text.includes('tokens exhausted') ||
    text.includes('token balance') ||
    text.includes('no tokens remaining') ||
    text.includes('not enough tokens') ||
    text.includes('token quota')
  ) {
    return `${cleanEngine}_provider_quota_or_subscription_unavailable`;
  }
  if (
    text.includes('unauthorized') ||
    text.includes('not authorized') ||
    text.includes('authentication') ||
    text.includes('auth_required') ||
    text.includes('auth required') ||
    text.includes('login required') ||
    text.includes('please login') ||
    text.includes('please log in') ||
    text.includes('api key') ||
    text.includes('invalid token') ||
    text.includes('token expired') ||
    text.includes('model: (not set)') ||
    text.includes('model not set') ||
    text.includes('provider: auto') ||
    text.includes('not configured') ||
    text.includes('(not set)')
  ) {
    return `${cleanEngine}_provider_auth_required`;
  }
  if (
    text.includes('rate limit') ||
    text.includes('rate-limit') ||
    text.includes('too many requests') ||
    text.includes('429')
  ) {
    return `${cleanEngine}_provider_rate_limited`;
  }
  if (
    text.includes('network') ||
    text.includes('offline') ||
    text.includes('connection refused') ||
    text.includes('connection reset') ||
    text.includes('econnreset') ||
    text.includes('econnrefused') ||
    text.includes('enotfound') ||
    text.includes('dns') ||
    text.includes('socket hang up') ||
    text.includes('transport unavailable')
  ) {
    return `${cleanEngine}_provider_network_unavailable`;
  }
  if (
    text.includes('not found') ||
    text.includes('missing') ||
    text.includes('not installed') ||
    text.includes('command not found') ||
    text.includes('enoent')
  ) {
    return `${cleanEngine}_runtime_not_available`;
  }
  return cleanText(source && (source.error_code || source.error), 120) || fallback;
}

function agentRuntimeFailureNextActions(errorCode, engineId) {
  const code = cleanText(errorCode, 160).toLowerCase();
  const engine = cleanEngineId(engineId) || 'agent_runtime';
  if (code.includes('provider_auth_required')) {
    return [
      `Check ${engine} login/API-key configuration.`,
      'Retry the turn after credentials are available.',
    ];
  }
  if (code.includes('provider_quota_or_subscription_unavailable')) {
    return [
      `Diagnostic: check ${engine} billing, subscription, or quota status.`,
      'Recover: switch to another runtime engine or provider until quota is restored.',
    ];
  }
  if (code.includes('provider_rate_limited')) {
    return [
      'Wait for the provider rate limit window to reset.',
      'Retry with a smaller or lower-frequency turn.',
    ];
  }
  if (code.includes('provider_network_unavailable')) {
    return [
      'Check local network/provider connectivity.',
      'Retry the turn or switch to an available local/runtime engine.',
    ];
  }
  if (code.includes('runtime_not_available')) {
    return [
      `Install or repair the ${engine} runtime binary.`,
      'Refresh runtime discovery after installation.',
    ];
  }
  if (code.includes('timeout') || code.includes('timed_out')) {
    return [
      'Retry with a smaller request or longer Gateway turn budget.',
      'Check whether the selected runtime is stalled.',
    ];
  }
  if (code.includes('payload') || code.includes('budget')) {
    return [
      'Reduce the requested output/detail size or fetch raw evidence by ref.',
      'Keep default chat projection bounded and retry.',
    ];
  }
  if (code.includes('transport')) {
    return [
      'Check the runtime socket/CLI transport and retry.',
      'Switch engines if the selected transport remains unavailable.',
    ];
  }
  return [
    'Inspect the linked trace/receipt refs for failure evidence.',
    'Retry or switch runtime engines if the failure repeats.',
  ];
}

function workedLabelFromMs(workedMs) {
  const workedSeconds = Math.max(0, Math.round((Number(workedMs) || 0) / 1000));
  return workedSeconds >= 3600
    ? `Worked for ${Math.floor(workedSeconds / 3600)}h ${Math.floor((workedSeconds % 3600) / 60)}m ${workedSeconds % 60}s`
    : workedSeconds >= 60
      ? `Worked for ${Math.floor(workedSeconds / 60)}m ${workedSeconds % 60}s`
      : `Worked for ${workedSeconds}s`;
}

function buildAgentRuntimeFailureActivityTrace({ traceId, engineId, sessionId, turnId, errorCode, displayText, workedMs = 0, providerEventType = 'pre_turn.failure' }) {
  const cleanEngine = cleanEngineId(engineId) || 'agent_runtime';
  const title = cleanDisplayText(displayText || `${cleanEngine} failed with ${errorCode || 'a classified error'}.`, 1000);
  return {
    type: 'agent_runtime_activity_trace_projection',
    source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    trace_id: cleanText(traceId, 200),
    engine_id: cleanEngine,
    session_id: cleanText(sessionId, 200),
    turn_id: cleanText(turnId, 200),
    collapsed_by_default: true,
    collapse_label: workedLabelFromMs(workedMs),
    worked_ms: Math.max(0, Number(workedMs) || 0),
    row_count: 1,
    raw_activity_event_count: 1,
    rows: [
      {
        type: 'agent_runtime_activity_trace_row',
        sequence_no: 1,
        activity_kind: 'error',
        provider_event_type: cleanText(providerEventType, 160),
        status: 'failed',
        title,
        detail_ref: `agent-runtime-activity/${cleanReceiptComponent(traceId, 200)}/${cleanReceiptComponent(turnId, 200)}/1`,
      },
    ],
    summary_text: `${cleanEngine} failed with ${cleanText(errorCode || 'a classified error', 160)}.`,
  };
}

function steeringInterventionActivityEvent(intervention, index, defaults = {}) {
  const source = intervention && typeof intervention === 'object' ? intervention : {};
  const text = cleanDisplayText(source.text || source.text_preview, 4000);
  const preview = cleanDisplayText(source.text_preview || text, 1000);
  const title = preview ? `User steered: ${preview}` : 'User steered this turn.';
  return {
    type: 'agent_activity_event',
    activity_kind: 'user_steer',
    provider_event_type: 'steering.user_message',
    source: 'gateway_runtime_steering',
    sequence_no: Number(index + 1) || 1,
    item_id: cleanReceiptComponent(source.steering_id || `steer_${index + 1}`, 200),
    status: cleanText(source.status || 'applied_to_next_turn', 80),
    text: title,
    display_text: title,
    display_in_thinking_bubble: true,
    role: 'user',
    timeline_role: 'user_steer',
    steering_id: cleanText(source.steering_id, 200),
    user_text: text,
    user_text_preview: preview,
    created_at: cleanText(source.created_at, 80),
    source_authority: cleanText(source.source_authority || 'gateway_agent_runtime_steer_route', 160),
    engine_id: cleanEngineId(defaults.engineId),
    trace_id: cleanText(defaults.traceId, 200),
    session_id: cleanText(defaults.sessionId, 200),
    turn_id: cleanText(defaults.turnId, 200),
  };
}

function agentRuntimePreTurnFailureProjection(traceId, engineId, agentId, sessionId, turnId, reason, source = {}) {
  const cleanEngine = cleanEngineId(engineId) || 'agent_runtime';
  const errorCode = classifyAgentRuntimePreTurnFailureCode(cleanEngine, { ...source, reason });
  const displayText = cleanDisplayText(
    reason || `${cleanEngine} is not available for this turn.`,
    1200,
  );
  const receiptProjection = source && source.receipt_projection && typeof source.receipt_projection === 'object'
    ? source.receipt_projection
    : null;
  const receiptRefs = Array.isArray(source && source.receipt_refs)
    ? source.receipt_refs.map((item) => cleanText(item, 240)).filter(Boolean).slice(0, 8)
    : receiptProjection && Array.isArray(receiptProjection.receipt_refs)
      ? receiptProjection.receipt_refs.map((item) => cleanText(item, 240)).filter(Boolean).slice(0, 8)
      : [];
  const workedMs = Math.max(0, Number(source && source.worked_ms) || 0);
  const activityTrace = buildAgentRuntimeFailureActivityTrace({
    traceId,
    engineId: cleanEngine,
    sessionId,
    turnId,
    errorCode,
    displayText,
    workedMs,
    providerEventType: source && source.provider_event_type || 'pre_turn.failure',
  });
  return {
    ok: false,
    status_code: 200,
    type: 'agent_runtime_turn_projection',
    trace_id: traceId,
    engine_id: cleanEngine,
    agent_id: cleanText(agentId, 160),
    session_id: cleanText(sessionId, 200),
    turn_id: cleanText(turnId, 200),
    status: 'failed_with_reason',
    error_code: errorCode,
    reason: displayText,
    retryable: !/quota|subscription|auth|login|api_key|billing/i.test(errorCode),
    timed_out: false,
    timeout_ms: 0,
    next_actions: agentRuntimeFailureNextActions(errorCode, cleanEngine),
    text: displayText,
    display_text: displayText,
    output_text: displayText,
    output_preview: cleanText(displayText, 4000),
    agent_activity_events: [
      {
        type: 'agent_activity_event',
        activity_kind: 'error',
        provider_event_type: 'pre_turn.failure',
        source: 'infring_gateway_agent_runtime_socket',
        sequence_no: 1,
        status: 'failed',
        text: displayText,
        display_text: displayText,
        engine_id: cleanEngine,
        trace_id: traceId,
        session_id: cleanText(sessionId, 200),
        turn_id: cleanText(turnId, 200),
      },
    ],
    activity_event_count: 1,
    raw_activity_event_count: 1,
    structured_activity: true,
    activity_trace: activityTrace,
    result_ref: '',
    receipt_ref: '',
    receipt_refs: receiptRefs,
    receipt_count: receiptRefs.length,
    receipt_projection: receiptProjection ? {
      type: 'agent_runtime_receipt_projection',
      receipt_refs: receiptRefs,
      receipt_count: Number(receiptProjection.receipt_count) || receiptRefs.length,
    } : null,
    pending_permission_request: null,
  };
}

function defaultContextPack(input = {}) {
  return {
    source_basis: 'gateway_runtime_default_context_pack',
    source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    row_count: 0,
    fanout_target: DEFAULT_CONTEXT_FANOUT_TARGET,
    fragments: [],
    frontier: {
      hot_atom_refs: [],
      warm_span_refs: [],
      cool_span_refs: [],
      cold_span_refs: [],
    },
    universal_tool_grants: {
      tools: [],
      source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    },
    trace_id: cleanText(input.traceId, 200),
    agent_id: cleanText(input.agentId, 160),
    session_id: cleanText(input.sessionId, 200),
  };
}

function normalizeContextPack(pack, fallback) {
  const contextPack = pack && typeof pack === 'object' ? pack : fallback;
  if (!Array.isArray(contextPack.fragments)) contextPack.fragments = [];
  if (!contextPack.frontier || typeof contextPack.frontier !== 'object') contextPack.frontier = {};
  for (const key of ['hot_atom_refs', 'warm_span_refs', 'cool_span_refs', 'cold_span_refs']) {
    if (!Array.isArray(contextPack.frontier[key])) contextPack.frontier[key] = [];
  }
  if (!contextPack.universal_tool_grants || typeof contextPack.universal_tool_grants !== 'object') {
    contextPack.universal_tool_grants = { tools: [], source_authority: 'gateway.runtime.agent_runtime_turn_projection' };
  }
  if (!Array.isArray(contextPack.universal_tool_grants.tools)) contextPack.universal_tool_grants.tools = [];
  return contextPack;
}

function contextFragmentPayload(fragment) {
  return fragment && fragment.payload && typeof fragment.payload === 'object' ? fragment.payload : {};
}

function normalizeConversationRole(value, fallback = 'message') {
  const role = cleanText(value, 40).toLowerCase();
  if (role === 'human' || role === 'user_message') return 'user';
  if (role === 'agent' || role === 'ai' || role === 'assistant_message') return 'assistant';
  if (role === 'function' || role === 'tool_result_bundle' || role === 'tool_receipt') return 'tool';
  if (role === 'status_summary' || role === 'system_event') return 'system';
  if (['user', 'assistant', 'tool', 'system'].includes(role)) return role;
  return fallback;
}

function buildConversationWindow(contextPack, currentTurnId = '') {
  const fragments = Array.isArray(contextPack && contextPack.fragments) ? contextPack.fragments : [];
  const fanout = Math.max(1, Math.min(Number(contextPack && contextPack.fanout_target) || DEFAULT_CONTEXT_FANOUT_TARGET, 14));
  const currentUserRef = currentTurnId ? `turn/${cleanText(currentTurnId, 200)}/user` : '';
  const rows = [];
  const seen = new Set();
  for (const fragment of fragments) {
    if (!fragment || fragment.kind !== 'atom') continue;
    const payload = contextFragmentPayload(fragment);
    const text = cleanDisplayText(payload.text_preview || payload.summary || '', 1600);
    if (!text) continue;
    const role = normalizeConversationRole(payload.role || payload.source_kind || payload.record_type);
    const sourceKind = cleanText(payload.source_kind || payload.record_type || `${role}_message`, 80);
    const sequenceNo = Number(payload.sequence_no || payload.coverage?.end_seq || 0) || 0;
    const sourceRef = cleanText(payload.source_ref || fragment.ref_id || fragment.fragment_id, 240);
    if (currentUserRef && sourceRef === currentUserRef) continue;
    const key = [
      role,
      sourceKind,
      sourceRef || text.toLowerCase().replace(/\s+/g, ' ').slice(0, 240),
    ].join('|');
    if (seen.has(key)) continue;
    seen.add(key);
    rows.push({
      type: 'agent_runtime_conversation_record',
      role,
      source_kind: sourceKind,
      speaker_label: cleanText(payload.speaker_label || role, 120),
      text_preview: text,
      ref_id: cleanText(fragment.ref_id || fragment.fragment_id, 200),
      source_ref: sourceRef,
      sequence_no: sequenceNo,
      source_authority: cleanText(payload.source_authority || contextPack.source_authority, 160),
    });
  }
  return rows.sort((left, right) => Number(left.sequence_no || 0) - Number(right.sequence_no || 0)).slice(-fanout);
}

function buildConversationSummaryRef(contextPack) {
  const frontier = contextPack && contextPack.frontier && typeof contextPack.frontier === 'object' ? contextPack.frontier : {};
  const spanRefs = [
    ...(Array.isArray(frontier.warm_span_refs) ? frontier.warm_span_refs : []),
    ...(Array.isArray(frontier.cool_span_refs) ? frontier.cool_span_refs : []),
    ...(Array.isArray(frontier.cold_span_refs) ? frontier.cold_span_refs : []),
  ].map((item) => cleanText(item, 200)).filter(Boolean);
  if (!spanRefs.length) return null;
  return {
    type: 'agent_runtime_conversation_summary_ref',
    source_authority: cleanText(contextPack.source_authority, 160),
    ref_ids: spanRefs.slice(-12),
    detail_fetch_required: true,
  };
}

function buildRelevantMemory(contextPack) {
  const fragments = Array.isArray(contextPack && contextPack.fragments) ? contextPack.fragments : [];
  return fragments
    .filter((fragment) => fragment && (fragment.kind === 'memory_version' || fragment.kind === 'task_anchor' || fragment.kind === 'span'))
    .map((fragment) => {
      const payload = contextFragmentPayload(fragment);
      const memoryRefs = Array.isArray(payload.memory_version_refs)
        ? payload.memory_version_refs.map((item) => cleanText(item, 200)).filter(Boolean).slice(0, 8)
        : [];
      return {
        type: 'agent_runtime_relevant_memory_ref',
        kind: cleanText(fragment.kind, 40),
        ref_id: cleanText(fragment.ref_id || fragment.fragment_id, 200),
        level: Number(fragment.level) || 0,
        summary: cleanDisplayText(payload.summary || payload.text_preview || '', 900),
        memory_version_refs: memoryRefs,
        detail_fetch_required: true,
      };
    })
    .filter((row) => row.ref_id || row.summary || row.memory_version_refs.length)
    .slice(-12);
}

function buildContextBudget(contextPack) {
  const frontier = contextPack && contextPack.frontier && typeof contextPack.frontier === 'object' ? contextPack.frontier : {};
  return {
    type: 'agent_runtime_context_budget',
    source_authority: cleanText(contextPack && contextPack.source_authority, 160),
    budget_tokens: Number(frontier.budget_tokens) || 0,
    used_tokens: Number(frontier.used_tokens) || 0,
    pressure_state: cleanText(frontier.pressure_state || '', 80),
    fidelity_score: Number(frontier.fidelity_score) || 0,
    fanout_target: Number(contextPack && contextPack.fanout_target) || DEFAULT_CONTEXT_FANOUT_TARGET,
    hot_tail_count: Number(contextPack && contextPack.hot_tail_count) || 0,
    fragment_count: Array.isArray(contextPack && contextPack.fragments) ? contextPack.fragments.length : 0,
  };
}

function buildPermissionScope(contextPack, permissionPolicy) {
  const grants = contextPack && contextPack.universal_tool_grants && typeof contextPack.universal_tool_grants === 'object'
    ? contextPack.universal_tool_grants
    : {};
  const tools = Array.isArray(grants.tools) ? grants.tools : [];
  return {
    type: 'agent_runtime_permission_scope',
    source_authority: cleanText(grants.source_authority || 'gateway_universal_core_tools_policy', 160),
    tool_ids: tools.map((tool) => cleanText(tool && tool.tool_id, 120)).filter(Boolean),
    proposal_only: true,
    permission_policy_attached: !!(permissionPolicy && typeof permissionPolicy === 'object'),
    gatekeeper_kind: 'user',
    future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor', 'admin_agent'],
  };
}

function runtimeEngineLabel(engineId) {
  const clean = cleanEngineId(engineId);
  const labels = {
    infring_native: 'InfRing Native',
    codex_cli: 'Codex',
    claude_code: 'Claude Code',
    grok_code: 'Grok Code',
    opencode: 'OpenCode',
    openclaw: 'OpenClaw',
    hermes_agent: 'Hermes Agent',
  };
  return labels[clean] || cleanText(clean.replace(/[_-]+/g, ' '), 120);
}

function buildRuntimeStackDeclaration(input, permissionScope) {
  const engineId = cleanEngineId(input && input.engineId);
  return {
    type: 'agent_runtime_stack_declaration',
    schema_version: 1,
    source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    host_substrate: 'InfRing',
    active_engine_id: engineId,
    active_engine_label: runtimeEngineLabel(engineId),
    host_owned_authority: [
      'context_pack',
      'memory_projection',
      'permission_policy',
      'approval_gate',
      'durable_receipts',
      'audit_trace',
      'payload_budget',
    ],
    engine_owned_scope: [
      'native_reasoning',
      'private_framework_tool_harness',
      'provider_model_runtime',
    ],
    permission_owner: 'InfRing Gateway/Kernel policy, not the external engine and not Shell',
    durable_effect_rule: 'Durable effects require Gateway/Kernel authorization and receipt-backed execution; never invent approval or claim a durable effect without a receipt/result.',
    write_policy: 'Reads may be default-allowed by policy; mutating filesystem, memory, artifact, or external effects must use proposal/approval or an explicit Gateway-issued direct mutation grant.',
    blocked_action_rule: 'If blocked, surface a permission request or typed failure with the concrete next action instead of silently ending the turn.',
    shell_boundary_rule: 'Shell only renders projections and submits user decisions; Shell does not construct context, grant authority, or execute runtime tools.',
    universal_tools_rule: 'Universal tools are tiny and proposal-oriented; terminal execution and direct file writes are not universal tools.',
    receipt_rule: 'Visible receipts, result refs, or approval decision refs are durable facts; proposals are intentions until approved and receipted.',
    permission_scope_ref: permissionScope && permissionScope.type ? 'context_pack.permission_scope' : '',
  };
}


function attachStructuredTurnEnvelope(contextPack, input) {
  const pack = contextPack && typeof contextPack === 'object' ? contextPack : {};
  const attachmentRefs = Array.isArray(input.attachmentRefs) ? input.attachmentRefs : [];
  const userMessage = {
    type: 'agent_runtime_user_message',
    source_authority: 'gateway_agent_runtime_turn_projection',
    role: 'user',
    text: cleanDisplayText(input.text, 12000),
    attachment_refs: attachmentRefs,
    attachment_count: attachmentRefs.length,
  };
  const conversationWindow = buildConversationWindow(pack, input.turnId);
  const relevantMemory = buildRelevantMemory(pack);
  const contextBudget = buildContextBudget(pack);
  const permissionScope = buildPermissionScope(pack, input.permissionPolicy);
  const runtimeStackDeclaration = buildRuntimeStackDeclaration(input, permissionScope);
  const approvalResume = input.approvalResume && typeof input.approvalResume === 'object'
    ? input.approvalResume
    : pack.approval_resume && typeof pack.approval_resume === 'object'
      ? pack.approval_resume
      : null;
  const envelope = {
    schema_version: 1,
    type: 'AgentRuntimeTurnEnvelope',
    source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    trace_id: cleanText(input.traceId, 200),
    engine_id: cleanEngineId(input.engineId),
    agent_id: cleanText(input.agentId, 160),
    session_id: cleanText(input.sessionId, 200),
    turn_id: cleanText(input.turnId, 200),
    user_message: userMessage,
    conversation_window: conversationWindow,
    conversation_summary_ref: buildConversationSummaryRef(pack),
    relevant_memory: relevantMemory,
    agent_profile: {
      type: 'agent_runtime_agent_profile_ref',
      agent_id: cleanText(input.agentId, 160),
      detail_fetch_required: true,
    },
    artifact_refs: attachmentRefs,
    permission_scope: permissionScope,
    runtime_stack_declaration: runtimeStackDeclaration,
    approval_resume: approvalResume,
    universal_tool_grants: pack.universal_tool_grants,
    context_budget: contextBudget,
    model_provider_context: pack.model_provider_context || null,
    active_workspace: pack.active_workspace || null,
  };
  pack.user_message = userMessage;
  pack.conversation_window = conversationWindow;
  pack.conversation_summary_ref = envelope.conversation_summary_ref;
  pack.relevant_memory = relevantMemory;
  pack.agent_profile = envelope.agent_profile;
  pack.artifact_refs = attachmentRefs;
  pack.permission_scope = permissionScope;
  pack.runtime_stack_declaration = runtimeStackDeclaration;
  pack.approval_resume = approvalResume;
  pack.context_budget = contextBudget;
  pack.turn_envelope = envelope;
  return envelope;
}

function createAgentRuntimeTurnProjectionStore(deps = {}) {
  const root = deps.root || process.cwd();
  const contextFanoutTarget = Number(deps.contextFanoutTarget) || DEFAULT_CONTEXT_FANOUT_TARGET;
  const normalizeAgentRuntimeTurnInput = deps.normalizeAgentRuntimeTurnInput || defaultNormalizeAgentRuntimeTurnInput;
  const buildUniversalToolGrants = deps.buildUniversalToolGrants || defaultBuildUniversalToolGrants;
  const buildAgentRuntimeStructuredTurn = deps.buildAgentRuntimeStructuredTurn || defaultBuildAgentRuntimeStructuredTurn;
  const loadAgentRuntimeSelection = typeof deps.loadAgentRuntimeSelection === 'function'
    ? deps.loadAgentRuntimeSelection
    : null;
  const noop = () => {};

  async function agentRuntimeTurnProjection(traceId, body, options = {}) {
    const engineId = resolveAgentRuntimeEngineId(body, {
      loadSelection: loadAgentRuntimeSelection,
      defaultEngineId: 'infring_native',
    });
    if (!engineId) {
      return {
        ok: false,
        status_code: 400,
        error: 'agent_runtime_engine_id_required',
        trace_id: traceId,
        engine_id: 'infring_native',
      };
    }
    const agentId = cleanText(body && body.agent_id, 160) || 'default';
    const sessionId = cleanText(body && body.session_id, 200) || `shell_${agentId}`;
    const turnId = cleanText(body && body.turn_id, 200) || `turn_${Date.now().toString(36)}`;
    const inputNormalization = normalizeAgentRuntimeTurnInput({
      body,
      traceId,
      engineId,
      agentId,
      sessionId,
      turnId,
    });
    const text = inputNormalization.text;
    const attachmentRefs = inputNormalization.attachmentRefs;
    if (!text && !attachmentRefs.length) {
      return {
        ok: false,
        status_code: 400,
        error: 'agent_runtime_turn_missing_input',
        trace_id: traceId,
        engine_id: engineId,
      };
    }
    const registry = deps.loadAgentRuntimeEngineRegistry ? deps.loadAgentRuntimeEngineRegistry(root) : { engines: [] };
    const engine = deps.findAgentRuntimeEngine
      ? deps.findAgentRuntimeEngine(registry, engineId)
      : null;
    if (!engine) {
      const displayText = `Agent runtime '${engineId}' is not registered.`;
      return {
        ok: false,
        status_code: 200,
        status: 'failed_with_reason',
        error: 'agent_runtime_engine_unknown',
        display_text: displayText,
        output_text: displayText,
        trace_id: traceId,
        engine_id: engineId,
      };
    }
    const workspace = deps.loadAgentRuntimeWorkspace ? deps.loadAgentRuntimeWorkspace(traceId) : { workspace_dir: root, active_workspace: root };
    const requestWorkspaceDir = cleanWorkingDirectory(
      body.working_directory ||
        body.current_working_directory ||
        body.present_working_directory ||
        body.cwd,
    );
    const activeWorkspaceDir = requestWorkspaceDir || workspace.workspace_dir || workspace.active_workspace || root;
    const modelProviderContext = normalizeModelProviderContext(body, engineId);
    const router = deps.createRouter({
      liveDispatch: true,
      nativeOrchestrationClient: options.nativeOrchestrationClient,
      cwd: activeWorkspaceDir,
    });
    const streamedActivityEvents = [];
    const activityDefaults = { engineId, traceId, sessionId, turnId };
    const sanitizeActivity = deps.sanitizeAgentRuntimeActivityEvent || sanitizeAgentRuntimeActivityEvent;
    let lastProgressActivityAtMs = Date.now();
    let rescheduleRouteTimeout = null;
    const onActivity = (event) => {
      const normalized = sanitizeActivity(event, streamedActivityEvents.length, activityDefaults);
      lastProgressActivityAtMs = Date.now();
      if (typeof rescheduleRouteTimeout === 'function') rescheduleRouteTimeout();
      if (isProgressOnlyActivity(normalized)) return;
      if (!normalized.display_text && !normalized.provider_event_type) return;
      streamedActivityEvents.push(normalized);
      if (typeof options.onActivity === 'function') options.onActivity(normalized);
    };
    const emitSyntheticActivity = (activityKind, providerEventType, displayText, status = 'completed') => onActivity({
      type: 'agent_activity_event',
      activity_kind: activityKind,
      provider_event_type: providerEventType,
      source: 'infring_gateway_agent_runtime_socket',
      status,
      display_text: displayText,
      text: displayText,
      engine_id: engineId,
      trace_id: traceId,
      session_id: sessionId,
      turn_id: turnId,
    });
    if (inputNormalization.largeTextAttachment) {
      emitSyntheticActivity(
        'activity',
        'context.attachment.materialized',
        `Moved oversized user text into ${inputNormalization.largeTextAttachment.filename} before runtime dispatch.`,
      );
    }
    try {
      (deps.appendAgentRuntimeTranscriptTurn || noop)({
        sessionId,
        agentId,
        traceId,
        turnId,
        engineId,
        userText: text,
        assistantText: '',
      });
    } catch {}
    try {
      (deps.appendAgentRuntimeTurnAtoms || noop)({
        root,
        sessionId,
        agentId,
        traceId,
        turnId,
        engineId,
        userText: text,
        assistantText: '',
      });
    } catch {}
    emitSyntheticActivity('started', 'context.prepare', `Preparing ${engineId} with InfRing conversation context.`);
    try {
      (deps.ingestAgentRuntimeContextProjection || noop)({
        root,
        sessionId,
        agentId,
        traceId,
        projection: body && body.context_projection,
      });
    } catch {}
    const rawFallbackContextRows = deps.loadAgentRuntimeContextRows
      ? deps.loadAgentRuntimeContextRows({ root, sessionId, agentId })
      : [];
    const fallbackContextRows = boundAgentRuntimeFallbackContextRows(rawFallbackContextRows);
    const loadedContextText = rawFallbackContextRows.length === fallbackContextRows.length
      ? `Loaded ${fallbackContextRows.length} prior context row${fallbackContextRows.length === 1 ? '' : 's'} for ${engineId}.`
      : `Loaded ${fallbackContextRows.length} of ${rawFallbackContextRows.length} prior context rows for ${engineId}.`;
    emitSyntheticActivity(
      'activity',
      'context.loaded',
      loadedContextText,
    );
    const kernelContext = await (deps.materializeKernelAgentRuntimeContextPack
      ? deps.materializeKernelAgentRuntimeContextPack({
        root,
        sessionId,
        agentId,
        traceId,
        atoms: fallbackContextRows,
        timeoutMs: 8000,
      })
      : Promise.resolve({ ok: false, reason: 'kernel_materializer_unavailable' })
    ).catch((error) => ({
      ok: false,
      reason: cleanText(error && error.message ? error.message : error, 240),
    }));
    const fallbackContextPack = () => deps.materializeAgentRuntimeContextPack
      ? Promise.resolve(deps.materializeAgentRuntimeContextPack({
        root,
        sessionId,
        agentId,
        traceId,
      })).catch(() => (deps.buildAgentRuntimeContextPack || defaultContextPack)({ body, agentId, sessionId, traceId }))
      : Promise.resolve((deps.buildAgentRuntimeContextPack || defaultContextPack)({ body, agentId, sessionId, traceId }));
    let contextPack = kernelContext && kernelContext.ok && kernelContext.context_pack
      ? kernelContext.context_pack
      : await fallbackContextPack();
    contextPack = normalizeContextPack(contextPack, defaultContextPack({ body, agentId, sessionId, traceId }));
    contextPack.active_workspace = {
      workspace_dir: workspace.workspace_dir,
      active_workspace: workspace.active_workspace,
      display_label: workspace.display_label,
      git_root: workspace.git_root,
      git_root_label: workspace.git_root_label,
      permission_boundary: workspace.permission_boundary,
      source_authority: 'gateway_agent_runtime_workspace_selection',
    };
    contextPack.model_provider_context = modelProviderContext;
    if (attachmentRefs.length) {
      contextPack.runtime_attachment_refs = {
        type: 'agent_runtime_attachment_refs',
        source_authority: 'gateway_agent_runtime_attachment_normalization',
        attachment_count: attachmentRefs.length,
        attachments: attachmentRefs,
      };
    }
    const permissionPolicy = deps.mergeAgentRuntimeApprovalPermissionPolicy
      ? deps.mergeAgentRuntimeApprovalPermissionPolicy(body && body.permission_policy, sessionId, engineId)
      : body && body.permission_policy;
    const approvalResumeSource = body && body.approval_resume && typeof body.approval_resume === 'object'
      ? body.approval_resume
      : null;
    const approvalResume = approvalResumeSource ? {
      type: 'agent_runtime_approval_resume_projection',
      schema_version: 1,
      approval_id: cleanApprovalId(approvalResumeSource.approval_id),
      resume_token: cleanApprovalId(approvalResumeSource.resume_token),
      approved_tool_id: cleanText(approvalResumeSource.approved_tool_id, 120),
      approval_decision: cleanText(approvalResumeSource.approval_decision, 80),
      approval_resume_action: cleanText(approvalResumeSource.approval_resume_action, 160),
      decision_receipt_ref: cleanText(approvalResumeSource.decision_receipt_ref, 240),
      approved_effect_executed: approvalResumeSource.approved_effect_executed === true,
      approved_effect_path: cleanText(approvalResumeSource.approved_effect_path, 600),
      approved_effect_artifact_ref: cleanText(approvalResumeSource.approved_effect_artifact_ref, 600),
      approved_effect_result_ref: cleanText(approvalResumeSource.approved_effect_result_ref, 600),
      approved_effect_receipt_ref: cleanText(approvalResumeSource.approved_effect_receipt_ref, 600),
      approved_effect_display_text: cleanText(approvalResumeSource.approved_effect_display_text, 1000),
      source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    } : null;
    if (approvalResume && (approvalResume.approval_id || approvalResume.resume_token || approvalResume.approved_tool_id)) {
      contextPack.approval_resume = approvalResume;
    }
    contextPack.universal_tool_grants = buildUniversalToolGrants({
      traceId,
      sessionId,
      agentId,
      engineId,
      permissionPolicy,
    });
    emitSyntheticActivity('started', 'engine.health', `Checking ${engineId} availability.`);
    const health = await router.healthCheck({
      type: 'agent_runtime.health_check',
      trace_id: traceId,
      engine_id: engineId,
      session_id: sessionId,
      cwd: activeWorkspaceDir,
      workspace_dir: activeWorkspaceDir,
      active_workspace: workspace,
    });
    if (!health || (health.status !== 'available' && health.status !== 'adapter_ready')) {
      const reason = cleanDisplayText(
        health && (health.reason || health.error || health.version_preview || health.status)
          ? `${engineId} is unavailable: ${health.reason || health.error || health.version_preview || health.status}`
          : `${engineId} is unavailable for this turn.`,
        1200,
      );
      const errorCode = classifyAgentRuntimePreTurnFailureCode(engineId, { ...(health || {}), reason });
      let receiptProjection = null;
      try {
        receiptProjection = deps.recordAgentRuntimeTurnReceipts
          ? deps.recordAgentRuntimeTurnReceipts({
            traceId,
            engineId,
            agentId,
            sessionId,
            turnId,
            status: 'failed_with_reason',
            modelProviderContext,
            contextPack,
            errorCode,
            reason,
            retryable: !/quota|subscription|auth|login|api_key|billing/i.test(errorCode),
            timedOut: false,
            timeoutMs: 0,
            outputText: reason,
            outputPreview: cleanText(reason, 4000),
          })
          : null;
      } catch {}
      return agentRuntimePreTurnFailureProjection(traceId, engineId, agentId, sessionId, turnId, reason, {
        ...(health || {}),
        error_code: errorCode,
        receipt_projection: receiptProjection,
        receipt_refs: receiptProjection && receiptProjection.receipt_refs,
      });
    }
    emitSyntheticActivity('started', 'session.start', `Starting ${engineId} session ${sessionId}.`);
    await router.startSession({
      type: 'agent_runtime.session_start',
      trace_id: traceId,
      engine_id: engineId,
      agent_id: agentId,
      session_id: sessionId,
      cwd: activeWorkspaceDir,
      workspace_dir: activeWorkspaceDir,
      active_workspace: workspace,
    });
    const steeringInterventions = deps.drainAgentRuntimeSteeringInterventions
      ? deps.drainAgentRuntimeSteeringInterventions({
        agentId,
        sessionId,
        engineId,
        traceId,
        turnId,
      })
      : [];
    if (steeringInterventions.length) {
      contextPack.runtime_steering = {
        type: 'agent_runtime_steering_context',
        mode: 'next_turn_interventions',
        source_authority: 'gateway_agent_runtime_steer_route',
        intervention_count: steeringInterventions.length,
        interventions: steeringInterventions,
      };
      steeringInterventions.forEach((intervention, index) => {
        onActivity(steeringInterventionActivityEvent(intervention, index, {
          engineId,
          traceId,
          sessionId,
          turnId,
        }));
      });
    }
    const turnEnvelope = attachStructuredTurnEnvelope(contextPack, {
      traceId,
      engineId,
      agentId,
      sessionId,
      turnId,
      text,
      attachmentRefs,
      permissionPolicy,
      approvalResume,
    });
    const structuredTurn = buildAgentRuntimeStructuredTurn({
      traceId,
      engineId,
      agentId,
      sessionId,
      turnId,
      text,
      attachmentRefs,
      turnEnvelope,
      contextPack,
      modelProviderContext,
      transportMode: 'structured_json',
      transportTarget: 'structured_json',
      promptTextCompatibilityAllowed: true,
    });
    contextPack.structured_turn_ref = structuredTurn.structured_turn_ref;
    contextPack.structured_transport = {
      type: 'agent_runtime_structured_transport_projection',
      structured_turn_ref: structuredTurn.structured_turn_ref,
      transport_mode: structuredTurn.transport_mode,
      transport_target: structuredTurn.transport_target,
      source_authority: structuredTurn.source_authority,
    };
    const requestedMaxTurnSeconds = Math.max(1, Math.min(
      Number(body && body.capability_budget && body.capability_budget.max_turn_seconds) ||
        Number(body && body.max_turn_seconds) ||
        180,
      300,
    ));
    const requestedAbsoluteMaxTurnSeconds = Math.max(requestedMaxTurnSeconds, Math.min(
      Number(body && body.capability_budget && body.capability_budget.max_absolute_turn_seconds) ||
        Number(body && body.max_absolute_turn_seconds) ||
        Number(process.env.INFRING_AGENT_RUNTIME_MAX_ABSOLUTE_TURN_SECONDS) ||
        1800,
      3600,
    ));
    const turnMessage = {
      type: 'agent_runtime.turn_submit',
      trace_id: traceId,
      engine_id: engineId,
      agent_id: agentId,
      session_id: sessionId,
      turn_id: turnId,
      working_directory: activeWorkspaceDir,
      cwd: activeWorkspaceDir,
      workspace_dir: activeWorkspaceDir,
      active_workspace: workspace,
      model_provider_context: modelProviderContext,
      approval_resume: approvalResume,
      input: { text, attachments: attachmentRefs },
      turn_envelope: turnEnvelope,
      structured_turn: structuredTurn,
      abort_signal: options.abortSignal,
      context_pack: contextPack,
      capability_budget: {
        max_default_response_bytes: 65536,
        max_turn_seconds: requestedMaxTurnSeconds,
        max_absolute_turn_seconds: requestedAbsoluteMaxTurnSeconds,
        shell_projection_only: true,
        context_pack_required: true,
        context_pack_fanout_target: contextFanoutTarget,
        universal_tool_grants_required: true,
        universal_tool_ids: contextPack.universal_tool_grants.tools.map((tool) => tool.tool_id),
      },
    };
    emitSyntheticActivity('started', 'turn.launch', `Launching ${engineId} turn with bounded context pack.`);
    const turnStartedAtMs = Date.now();
    const turnTimeoutMs = requestedMaxTurnSeconds * 1000;
    const turnAbsoluteTimeoutMs = requestedAbsoluteMaxTurnSeconds * 1000;
    let routeTimedOut = false;
    const turnPromise = router.streamTurn(turnMessage, onActivity);
    let routeTimeoutTimer = null;
    const timeoutPromise = new Promise((resolve) => {
      const resolveTimeout = (timeoutKind) => {
        if (routeTimedOut) return;
        routeTimedOut = true;
        const timedOutByAbsoluteLimit = timeoutKind === 'absolute';
        const timeoutDisplayText = timedOutByAbsoluteLimit
          ? `${engineId} reached the ${requestedAbsoluteMaxTurnSeconds}s absolute Gateway safety limit. Gateway returned a bounded timeout projection.`
          : `${engineId} stopped receiving runtime activity for ${requestedMaxTurnSeconds}s. Gateway returned a bounded no-progress timeout projection.`;
        resolve({
          type: 'turn.complete',
          trace_id: traceId,
          engine_id: engineId,
          agent_id: agentId,
          session_id: sessionId,
          turn_id: turnId,
          status: 'timed_out',
          error_code: 'agent_runtime_gateway_turn_timeout',
          reason: timeoutDisplayText,
          retryable: true,
          timed_out: true,
          timeout_ms: timedOutByAbsoluteLimit ? turnAbsoluteTimeoutMs : turnTimeoutMs,
          output_text: timeoutDisplayText,
          output_preview: timeoutDisplayText,
          activity_events: [{
            type: 'agent_activity_event',
            trace_id: traceId,
            engine_id: engineId,
            session_id: sessionId,
            turn_id: turnId,
            activity_kind: 'error',
            provider_event_type: 'turn.timeout',
            source: 'gateway_runtime_turn_projection_timeout',
            sequence_no: 1,
            item_id: 'gateway-turn-timeout',
            status: 'timed_out',
            text: timeoutDisplayText,
            display_text: timeoutDisplayText,
          }],
        });
      };
      rescheduleRouteTimeout = () => {
        if (routeTimedOut) return;
        if (routeTimeoutTimer) clearTimeout(routeTimeoutTimer);
        const now = Date.now();
        const idleRemainingMs = turnTimeoutMs - Math.max(0, now - lastProgressActivityAtMs);
        const absoluteRemainingMs = turnAbsoluteTimeoutMs - Math.max(0, now - turnStartedAtMs);
        if (absoluteRemainingMs <= 0) {
          resolveTimeout('absolute');
          return;
        }
        if (idleRemainingMs <= 0) {
          resolveTimeout('idle');
          return;
        }
        const nextWaitMs = Math.max(1, Math.min(idleRemainingMs, absoluteRemainingMs));
        routeTimeoutTimer = setTimeout(() => {
          const latestNow = Date.now();
          const latestIdleRemainingMs = turnTimeoutMs - Math.max(0, latestNow - lastProgressActivityAtMs);
          const latestAbsoluteRemainingMs = turnAbsoluteTimeoutMs - Math.max(0, latestNow - turnStartedAtMs);
          resolveTimeout(latestAbsoluteRemainingMs <= 0 && latestAbsoluteRemainingMs <= latestIdleRemainingMs ? 'absolute' : 'idle');
        }, nextWaitMs);
        if (routeTimeoutTimer && typeof routeTimeoutTimer.unref === 'function') routeTimeoutTimer.unref();
      };
      rescheduleRouteTimeout();
    });
    let turn = await Promise.race([turnPromise, timeoutPromise]);
    if (!routeTimedOut && routeTimeoutTimer) clearTimeout(routeTimeoutTimer);
    if (routeTimedOut && router && typeof router.cancelTurn === 'function') {
      router.cancelTurn({
        type: 'agent_runtime.cancel_turn',
        trace_id: traceId,
        engine_id: engineId,
        session_id: sessionId,
        turn_id: turnId,
        reason: 'gateway_turn_timeout',
      }).catch(() => {});
    }
    const workedMs = Math.max(0, Date.now() - turnStartedAtMs);
    if (!(turn && Array.isArray(turn.activity_events) && turn.activity_events.length)) {
      emitSyntheticActivity('completed', 'turn.completed', `${engineId} returned ${cleanText(turn && turn.status, 80) || 'a result'}.`);
    }
    const autoApprovalCandidate = turn && turn.permission_request && typeof turn.permission_request === 'object'
      ? turn.permission_request
      : null;
    const autoApprovalToolId = cleanText(autoApprovalCandidate && autoApprovalCandidate.tool_id, 120);
    const autoAllowedToolCalls = Array.isArray(permissionPolicy && permissionPolicy.always_allowed_tool_calls)
      ? permissionPolicy.always_allowed_tool_calls.map((toolId) => cleanText(toolId, 120)).filter(Boolean)
      : [];
    const autoApprovalAllowed = !!(
      autoApprovalCandidate &&
      autoApprovalToolId &&
      autoAllowedToolCalls.indexOf(autoApprovalToolId) >= 0 &&
      cleanText(autoApprovalCandidate.resume_strategy || 'gateway_apply_approved_effect', 120) === 'gateway_apply_approved_effect' &&
      deps.executeAgentRuntimeApprovedProposal
    );
    if (autoApprovalAllowed) {
      const autoApprovalId = cleanApprovalId(
        autoApprovalCandidate.approval_id ||
        `auto_${autoApprovalToolId}_${traceId}_${turnId}`,
      );
      const autoExecutionBody = {
        ...autoApprovalCandidate,
        tool_id: autoApprovalToolId,
        proposal_arguments: autoApprovalCandidate.proposal_arguments,
        working_directory:
          autoApprovalCandidate.working_directory ||
          turnMessage.working_directory ||
          turnMessage.cwd ||
          turnMessage.workspace_dir,
        engine_id: engineId,
        agent_id: agentId,
        session_id: sessionId,
        turn_id: turnId,
        tool_call_ref: autoApprovalCandidate.tool_call_ref,
      };
      let autoExecutionResult = null;
      try {
        autoExecutionResult = deps.executeAgentRuntimeApprovedProposal(traceId, autoApprovalId, autoExecutionBody);
      } catch (error) {
        autoExecutionResult = {
          ok: false,
          type: 'agent_runtime_auto_approved_effect_error',
          approval_id: autoApprovalId,
          trace_id: traceId,
          tool_id: autoApprovalToolId,
          error: cleanText(error && error.message ? error.message : error, 240),
        };
      }
      const autoDisplayText = cleanDisplayText(
        autoExecutionResult && autoExecutionResult.ok
          ? autoExecutionResult.display_text || `Executed always-allowed ${autoApprovalToolId}.`
          : `Always-allowed ${autoApprovalToolId} failed${autoExecutionResult && autoExecutionResult.error ? `: ${autoExecutionResult.error}` : '.'}`,
        1200,
      );
      const autoActivityEvents = Array.isArray(turn && turn.activity_events) ? turn.activity_events.slice(0, 80) : [];
      autoActivityEvents.push({
        type: autoExecutionResult && autoExecutionResult.ok ? 'approval.auto_applied' : 'approval.auto_apply_failed',
        activity_kind: autoExecutionResult && autoExecutionResult.ok ? 'receipt' : 'error',
        provider_event_type: 'approval.always_allow',
        status: autoExecutionResult && autoExecutionResult.ok ? 'done' : 'error',
        display_text: autoDisplayText,
        text: autoDisplayText,
        engine_id: engineId,
        trace_id: traceId,
        session_id: sessionId,
        turn_id: turnId,
      });
      turn = {
        ...(turn || {}),
        status: autoExecutionResult && autoExecutionResult.ok ? 'completed' : 'failed',
        output_text: autoDisplayText,
        display_text: autoDisplayText,
        text: autoDisplayText,
        output_preview: autoDisplayText,
        error_code: autoExecutionResult && autoExecutionResult.ok ? '' : 'agent_runtime_auto_approved_effect_failed',
        reason: autoExecutionResult && autoExecutionResult.ok ? '' : autoDisplayText,
        permission_request: null,
        approval_pause: null,
        result_ref: cleanText(autoExecutionResult && autoExecutionResult.result_ref, 240),
        receipt_ref: cleanText(autoExecutionResult && autoExecutionResult.receipt_ref, 240),
        activity_events: autoActivityEvents,
        activity_event_count: autoActivityEvents.length,
        approved_effect_executed: !!(autoExecutionResult && autoExecutionResult.ok),
        approved_effect_path: cleanText(autoExecutionResult && autoExecutionResult.path, 600),
        approved_effect_result_ref: cleanText(autoExecutionResult && autoExecutionResult.result_ref, 600),
        approved_effect_receipt_ref: cleanText(autoExecutionResult && autoExecutionResult.receipt_ref, 600),
      };
    }
    const output = cleanDisplayText(
      turn && (turn.output_text || turn.display_text || turn.text || turn.response || turn.output_preview || turn.delta || turn.reason),
      24000,
    );
    const outputPreview = cleanText(turn && (turn.output_preview || output), 4000);
    const pendingPermissionRequest = turn && turn.permission_request && typeof turn.permission_request === 'object'
      ? turn.permission_request
      : null;
    const approvalPauseFromTurn = turn && turn.approval_pause && typeof turn.approval_pause === 'object' && cleanText(turn.status, 80) === 'permission_required'
      ? turn.approval_pause
      : null;
    const permissionRequestFromPause = approvalPauseFromTurn ? {
      type: 'permission.requested',
      approval_id: cleanApprovalId(
        approvalPauseFromTurn.approval_id
        || (approvalPauseFromTurn.approvalId)
        || approvalPauseFromTurn.resume_token
        || approvalPauseFromTurn.result_ref
        || approvalPauseFromTurn.receipt_ref
      ) || `pending_${Math.floor(Date.now() / 1000)}_${Math.floor(Math.random() * 100000)}`,
      trace_id: cleanText(approvalPauseFromTurn.trace_id || traceId, 200),
      request_id: cleanText(approvalPauseFromTurn.request_id || turnMessage.request_id || '', 200),
      engine_id: cleanEngineId(approvalPauseFromTurn.engine_id || engineId),
      session_id: cleanText(approvalPauseFromTurn.session_id || sessionId, 200),
      turn_id: cleanText(approvalPauseFromTurn.turn_id || turnId, 200),
      working_directory: cleanText(
        approvalPauseFromTurn.working_directory ||
        approvalPauseFromTurn.current_working_directory ||
        approvalPauseFromTurn.present_working_directory ||
        approvalPauseFromTurn.cwd ||
        turnMessage.working_directory ||
        turnMessage.cwd ||
        turnMessage.workspace_dir ||
        '',
        1000,
      ),
      tool_call_ref: cleanText(approvalPauseFromTurn.tool_call_ref, 240),
      tool_id: cleanText(approvalPauseFromTurn.tool_id, 120),
      capability: cleanText(approvalPauseFromTurn.capability, 160),
      reason: cleanText(approvalPauseFromTurn.reason || approvalPauseFromTurn.pause_reason, 1000),
      argument_keys: Array.isArray(approvalPauseFromTurn.argument_keys)
        ? approvalPauseFromTurn.argument_keys.map((key) => cleanText(key, 80)).filter(Boolean).slice(0, 24)
        : [],
      proposal_arguments: deps.sanitizeAgentRuntimeProposalArguments
        ? deps.sanitizeAgentRuntimeProposalArguments(approvalPauseFromTurn.proposal_arguments)
        : {},
      gatekeeper_kind: cleanText(approvalPauseFromTurn.gatekeeper_kind || 'user', 80) || 'user',
      status: 'paused_pending_approval',
      turn_status: 'permission_required',
      pause_reason: cleanText(
        approvalPauseFromTurn.pause_reason || approvalPauseFromTurn.reason || 'agent_runtime_tool_call_requires_approval',
        1000,
      ),
      resume_strategy: cleanText(approvalPauseFromTurn.resume_strategy || 'gateway_apply_approved_effect', 120),
      source: cleanText(approvalPauseFromTurn.source || 'gateway_runtime_turn_projection_pause', 160),
      resume_token: cleanApprovalId(approvalPauseFromTurn.resume_token),
      future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor', 'admin_agent'],
      decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
      decision_scope: 'tool_call',
      approval_route: cleanText(
        approvalPauseFromTurn.approval_route
          || approvalPauseFromTurn.decision_route
          || (approvalPauseFromTurn.approval_id
            ? `/api/shell-socket/approvals/${encodeURIComponent(cleanApprovalId(approvalPauseFromTurn.approval_id))}/decision`
            : ''),
        260,
      ),
    } : null;
    const pendingPermissionProjection = (pendingPermissionRequest || permissionRequestFromPause) ? {
      type: 'permission.requested',
      approval_id: cleanApprovalId((pendingPermissionRequest || {}).approval_id || (permissionRequestFromPause || {}).approval_id),
      trace_id: cleanText(
        (pendingPermissionRequest || {}).trace_id || (permissionRequestFromPause || {}).trace_id || traceId,
        200,
      ),
      request_id: cleanText((pendingPermissionRequest || {}).request_id || (permissionRequestFromPause || {}).request_id, 200),
      engine_id: cleanEngineId(
        (pendingPermissionRequest || {}).engine_id
        || (permissionRequestFromPause || {}).engine_id
        || engineId,
      ),
      session_id: cleanText(
        (pendingPermissionRequest || {}).session_id || (permissionRequestFromPause || {}).session_id || sessionId,
        200,
      ),
      turn_id: cleanText(
        (pendingPermissionRequest || {}).turn_id || (permissionRequestFromPause || {}).turn_id || turnId,
        200,
      ),
      working_directory: cleanText(
        (pendingPermissionRequest || {}).working_directory ||
          (pendingPermissionRequest || {}).current_working_directory ||
          (pendingPermissionRequest || {}).present_working_directory ||
          (pendingPermissionRequest || {}).cwd ||
          (permissionRequestFromPause || {}).working_directory ||
          (permissionRequestFromPause || {}).current_working_directory ||
          (permissionRequestFromPause || {}).present_working_directory ||
          (permissionRequestFromPause || {}).cwd ||
          turnMessage.working_directory ||
          turnMessage.cwd ||
          turnMessage.workspace_dir ||
          '',
        1000,
      ),
      tool_call_ref: cleanText((pendingPermissionRequest || {}).tool_call_ref || (permissionRequestFromPause || {}).tool_call_ref, 240),
      tool_id: cleanText((pendingPermissionRequest || {}).tool_id || (permissionRequestFromPause || {}).tool_id, 120),
      capability: cleanText((pendingPermissionRequest || {}).capability || (permissionRequestFromPause || {}).capability, 160),
      reason: cleanText((pendingPermissionRequest || {}).reason || (permissionRequestFromPause || {}).reason, 1000),
      argument_keys: Array.isArray(pendingPermissionRequest.argument_keys)
        ? pendingPermissionRequest.argument_keys.map((key) => cleanText(key, 80)).filter(Boolean).slice(0, 24)
        : [],
      proposal_arguments: deps.sanitizeAgentRuntimeProposalArguments
        ? deps.sanitizeAgentRuntimeProposalArguments(
          (pendingPermissionRequest || {}).proposal_arguments || (permissionRequestFromPause || {}).proposal_arguments
        )
        : {},
      gatekeeper_kind: cleanText(
        (pendingPermissionRequest || {}).gatekeeper_kind || (permissionRequestFromPause || {}).gatekeeper_kind || 'user',
        80,
      ) || 'user',
      status: 'paused_pending_approval',
      turn_status: 'permission_required',
      pause_reason: cleanText(
        (pendingPermissionRequest || {}).pause_reason
          || (pendingPermissionRequest || {}).reason
          || (permissionRequestFromPause || {}).pause_reason
          || (permissionRequestFromPause || {}).reason
          || 'agent_runtime_tool_call_requires_approval',
        1000,
      ),
      resume_strategy: cleanText((pendingPermissionRequest || {}).resume_strategy || (permissionRequestFromPause || {}).resume_strategy || 'grant_then_retry_next_turn', 120),
      source: cleanText((pendingPermissionRequest || {}).source || (permissionRequestFromPause || {}).source || 'gateway_runtime_turn_projection', 160),
      resume_token: cleanApprovalId((pendingPermissionRequest || {}).resume_token || (permissionRequestFromPause || {}).resume_token),
      future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor', 'admin_agent'],
      decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
      decision_scope: 'tool_call',
      approval_route: cleanText(
        (pendingPermissionRequest || {}).approval_route
          || (permissionRequestFromPause || {}).approval_route
          || (permissionRequestFromPause || {}).decision_route
          || (pendingPermissionRequest || {}).approval_id
          || (permissionRequestFromPause || {}).approval_id
          ? `/api/shell-socket/approvals/${encodeURIComponent(
            cleanApprovalId((pendingPermissionRequest || {}).approval_id || (permissionRequestFromPause || {}).approval_id),
          )}/decision`
          : '',
        260,
      ),
    } : null;
    const recordedPendingPermission = pendingPermissionProjection && deps.recordAgentRuntimePendingApproval
      ? deps.recordAgentRuntimePendingApproval(pendingPermissionProjection)
      : pendingPermissionProjection;
    const projectedPendingPermission = recordedPendingPermission || pendingPermissionProjection;
    const permissionDisplayText = projectedPendingPermission
      ? `Permission required: ${cleanText(projectedPendingPermission.reason || projectedPendingPermission.tool_id || 'agent runtime requested approval', 1000)}`
      : '';
    const rawTurnStatus = cleanText(turn && turn.status, 80);
    const terminalOutcomeStatus = projectedPendingPermission
      ? 'permission_required'
      : rawTurnStatus === 'completed'
        ? 'completed'
        : turn && turn.timed_out
          ? 'timed_out_with_reason'
          : rawTurnStatus === 'failed' || (turn && turn.error_code)
            ? 'failed_with_reason'
            : (rawTurnStatus || 'unknown');
    const terminalOutcomeOk = terminalOutcomeStatus === 'completed' || terminalOutcomeStatus === 'permission_required';
    const finalActivityEvents = Array.isArray(turn && turn.activity_events)
      ? turn.activity_events.map((event, index) => sanitizeActivity(event, index, activityDefaults))
      : [];
    const permissionActivityEvent = projectedPendingPermission
      ? sanitizeActivity({
        type: 'permission.requested',
        activity_kind: 'permission_request',
        provider_event_type: 'permission.requested',
        source: 'gateway_runtime_permission_pause',
        sequence_no: streamedActivityEvents.length + finalActivityEvents.length + 1,
        item_id: cleanApprovalId(projectedPendingPermission.approval_id),
        status: 'paused_pending_approval',
        text: permissionDisplayText,
        display_text: permissionDisplayText,
        engine_id: engineId,
        trace_id: traceId,
        session_id: sessionId,
        turn_id: turnId,
      }, streamedActivityEvents.length + finalActivityEvents.length, activityDefaults)
      : null;
    const activityDedupe = new Set();
    const activityEvents = [...streamedActivityEvents, ...finalActivityEvents, permissionActivityEvent]
      .filter((event) => event && (event.display_text || event.provider_event_type))
      .filter((event) => {
        const key = cleanDisplayText(event.display_text, 1000)
          ? [
              event.activity_kind,
              event.provider_event_type,
              cleanDisplayText(event.display_text, 1000),
            ].join('|')
          : [
              event.sequence_no,
              event.activity_kind,
              event.provider_event_type,
            ].join('|');
        if (activityDedupe.has(key)) return false;
        activityDedupe.add(key);
        return true;
      })
      .slice(-80);
    const activityTraceRows = compactActivityTraceRows(activityEvents
      .map((event, index) => {
        const title = cleanDisplayText(
          event.display_text || event.text || event.summary || event.provider_event_type || event.activity_kind,
          1000,
        );
        if (!title) return null;
        return {
          type: 'agent_runtime_activity_trace_row',
          sequence_no: index + 1,
          provider_sequence_no: Number(event.sequence_no) || 0,
          activity_kind: cleanText(event.activity_kind || 'activity', 80),
          provider_event_type: cleanText(event.provider_event_type || '', 160),
          status: cleanText(event.status || '', 80),
          title,
          display_in_thinking_bubble: shouldDisplayActivityInThinkingBubble(event, title, event.provider_event_type),
          role: cleanText(event.role || '', 40),
          timeline_role: cleanText(event.timeline_role || '', 80),
          steering_id: cleanText(event.steering_id || '', 200),
          user_text: cleanDisplayText(event.user_text || '', 4000),
          user_text_preview: cleanDisplayText(event.user_text_preview || '', 1000),
          detail_ref: `agent-runtime-activity/${traceId}/${turnId}/${index + 1}`,
        };
      })
      .filter(Boolean)
      .slice(-48));
    const workedLabel = workedLabelFromMs(workedMs);
    const activityTraceProjection = {
      type: 'agent_runtime_activity_trace_projection',
      source_authority: 'gateway.runtime.agent_runtime_turn_projection',
      trace_id: traceId,
      engine_id: engineId,
      session_id: sessionId,
      turn_id: turnId,
      collapsed_by_default: true,
      collapse_label: workedLabel,
      worked_ms: workedMs,
      row_count: activityTraceRows.length,
      raw_activity_event_count: Number(turn && turn.activity_event_count) || activityEvents.length,
      rows: activityTraceRows,
      summary_text: projectedPendingPermission
        ? permissionDisplayText
        : terminalOutcomeStatus === 'completed'
          ? `${engineId} completed the turn.`
          : terminalOutcomeStatus === 'failed_with_reason'
            ? `${engineId} failed with ${cleanText(turn && turn.error_code, 120) || 'a classified error'}.`
            : terminalOutcomeStatus === 'timed_out_with_reason'
              ? `${engineId} timed out.`
              : `${engineId} ended with status ${terminalOutcomeStatus}.`,
    };
    const persistedAssistantOutput = projectedPendingPermission ? permissionDisplayText : output;
    try {
      (deps.appendAgentRuntimeTurnAtoms || noop)({
        root,
        sessionId,
        agentId,
        traceId,
        turnId,
        engineId,
        userText: text,
        assistantText: persistedAssistantOutput,
        resultRef: turn && turn.result_ref,
        receiptRef: turn && turn.receipt_ref,
      });
    } catch {}
    try {
      (deps.appendAgentRuntimeTranscriptTurn || noop)({
        sessionId,
        agentId,
        traceId,
        turnId,
        engineId,
        userText: '',
        assistantText: persistedAssistantOutput,
        status: terminalOutcomeStatus,
        pendingPermissionRequest: projectedPendingPermission,
        activityEvents,
        activityTrace: activityTraceProjection,
        workedMs,
        workedLabel,
      });
    } catch {}
    let receiptProjection = null;
    try {
      receiptProjection = deps.recordAgentRuntimeTurnReceipts
        ? deps.recordAgentRuntimeTurnReceipts({
          traceId,
          engineId,
          agentId,
          sessionId,
          turnId,
          status: terminalOutcomeStatus,
          modelProviderContext,
          contextPack,
          pendingPermissionRequest: projectedPendingPermission,
          resultRef: turn && turn.result_ref,
          adapterReceiptRef: turn && turn.receipt_ref,
          errorCode: turn && turn.error_code,
          reason: turn && turn.reason,
          retryable: turn && turn.retryable,
          timedOut: turn && turn.timed_out,
          timeoutMs: turn && turn.timeout_ms,
          outputText: output,
          outputPreview,
        })
        : null;
    } catch {}
    return {
      ok: terminalOutcomeOk,
      status_code: 200,
      type: 'agent_runtime_turn_projection',
      trace_id: traceId,
      engine_id: engineId,
      agent_id: agentId,
      session_id: sessionId,
      turn_id: turnId,
      status: terminalOutcomeStatus,
      error_code: cleanText(turn && turn.error_code, 120),
      reason: cleanDisplayText(turn && turn.reason, 1200),
      retryable: turn && turn.retryable === true,
      timed_out: turn && turn.timed_out === true,
      timeout_ms: Number(turn && turn.timeout_ms) || 0,
      next_actions: (terminalOutcomeStatus === 'failed_with_reason' || terminalOutcomeStatus === 'timed_out_with_reason')
        ? agentRuntimeFailureNextActions(cleanText(turn && turn.error_code, 120), engineId)
        : [],
      text: projectedPendingPermission ? permissionDisplayText : output,
      display_text: projectedPendingPermission ? permissionDisplayText : output,
      output_text: projectedPendingPermission ? permissionDisplayText : output,
      output_preview: projectedPendingPermission ? cleanText(permissionDisplayText, 4000) : outputPreview,
      agent_activity_events: activityEvents,
      activity_event_count: activityEvents.length,
      raw_activity_event_count: Number(turn && turn.activity_event_count) || activityEvents.length,
      structured_activity: turn && turn.structured_activity === true,
      activity_trace: activityTraceProjection,
      result_ref: cleanText(turn && turn.result_ref, 240),
      receipt_ref: cleanText(turn && turn.receipt_ref, 240),
      receipt_refs: receiptProjection && Array.isArray(receiptProjection.receipt_refs)
        ? receiptProjection.receipt_refs
        : [cleanText(turn && turn.receipt_ref, 240)].filter(Boolean),
      receipt_count: Number(receiptProjection && receiptProjection.receipt_count) || 0,
      receipt_projection: receiptProjection ? {
        type: 'agent_runtime_receipt_projection',
        receipt_refs: Array.isArray(receiptProjection.receipt_refs) ? receiptProjection.receipt_refs : [],
        receipt_count: Number(receiptProjection.receipt_count) || 0,
      } : null,
      pending_permission: !!projectedPendingPermission,
      pending_permission_request: projectedPendingPermission,
      permission_request: projectedPendingPermission,
      approval_pause_active: !!projectedPendingPermission,
      approval_pause: projectedPendingPermission ? {
        type: 'agent_runtime_approval_pause',
        status: 'paused_pending_approval',
        turn_status: 'permission_required',
        approval_id: cleanApprovalId(projectedPendingPermission.approval_id),
        resume_token: cleanApprovalId(projectedPendingPermission.resume_token),
        resume_strategy: cleanText(projectedPendingPermission.resume_strategy || 'gateway_apply_approved_effect_or_retry_next_turn', 120),
        decision_route: cleanText(projectedPendingPermission.approval_route, 300),
        gatekeeper_kind: cleanText(projectedPendingPermission.gatekeeper_kind || 'user', 80) || 'user',
        future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor', 'admin_agent'],
      } : null,
      context_pack: {
        type: 'agent_runtime_context_pack_projection',
        source_basis: contextPack.source_basis,
        source_authority: contextPack.source_authority,
        row_count: contextPack.row_count,
        fanout_target: contextPack.fanout_target,
        fragment_count: contextPack.fragments.length,
        hot_atom_count: contextPack.frontier.hot_atom_refs.length,
        warm_span_count: contextPack.frontier.warm_span_refs.length,
        cool_span_count: contextPack.frontier.cool_span_refs.length,
        cold_span_count: contextPack.frontier.cold_span_refs.length,
        universal_tool_count: contextPack.universal_tool_grants.tools.length,
        steering_intervention_count: contextPack.runtime_steering ? contextPack.runtime_steering.intervention_count : 0,
        universal_tool_source_authority: contextPack.universal_tool_grants.source_authority,
        runtime_stack_declared: !!contextPack.runtime_stack_declaration,
        runtime_stack_host_substrate: contextPack.runtime_stack_declaration ? cleanText(contextPack.runtime_stack_declaration.host_substrate, 80) : '',
        runtime_stack_active_engine_id: contextPack.runtime_stack_declaration ? cleanEngineId(contextPack.runtime_stack_declaration.active_engine_id) : '',
        kernel_materializer_used: !!(kernelContext && kernelContext.ok),
        kernel_materializer_mode: cleanText(kernelContext && kernelContext.command_mode, 40),
        turn_envelope_attached: true,
        conversation_window_count: Array.isArray(turnEnvelope.conversation_window) ? turnEnvelope.conversation_window.length : 0,
        relevant_memory_count: Array.isArray(turnEnvelope.relevant_memory) ? turnEnvelope.relevant_memory.length : 0,
        context_budget_used_tokens: Number(turnEnvelope.context_budget && turnEnvelope.context_budget.used_tokens) || 0,
        context_budget_tokens: Number(turnEnvelope.context_budget && turnEnvelope.context_budget.budget_tokens) || 0,
      },
      health: {
        status: cleanText(health && health.status, 80),
        discovery_source: cleanText(health && health.discovery_source, 120),
        version_preview: cleanText(health && health.version_preview, 200),
      },
    };
  }

  return {
    agentRuntimeTurnProjection,
    agentRuntimePreTurnFailureProjection,
  };
}

module.exports = {
  createAgentRuntimeTurnProjectionStore,
  sanitizeAgentRuntimeActivityEvent,
  classifyAgentRuntimePreTurnFailureCode,
  agentRuntimePreTurnFailureProjection,
};
