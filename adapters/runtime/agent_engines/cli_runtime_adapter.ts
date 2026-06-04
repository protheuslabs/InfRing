#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::cli-runtime-adapter.
//
// Shared bounded adapter for CLI-based external agent runtimes. It keeps live
// dispatch opt-in and uses Gateway-owned discovery so Shell never probes local
// runtime installs directly.
//
// PROMPT_TEXT_COMPATIBILITY_LAYER: this file renders the canonical Gateway
// context pack into bounded prompt text only because Codex/Claude/Grok CLI
// transports currently expose text-first turn APIs.
// STRUCTURED_JSON_TRANSPORT_TARGET: migrate provider adapters to structured
// JSON/native session bridges as soon as each engine exposes a stable typed
// context/tool/approval/output channel. Prompt text must not become authority.

'use strict';

const childProcess = require('child_process');
const crypto = require('crypto');
const { resolveEngineDiscovery } = require('./discovery.ts');
const { renderUniversalToolGrantPromptSection } = require('./universal_core_tools.ts');

function cleanString(value, max = 2000) {
  return stripTerminalControls(value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function cleanDisplayString(value, max = 24000) {
  return stripTerminalControls(value)
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{5,}/g, '\n\n\n\n')
    .trim()
    .slice(0, max);
}

function cleanTextChunk(value, max = 24000) {
  return stripTerminalControls(value)
    .replace(/\r\n/g, '\n')
    .slice(0, max);
}

function decodeEmbeddedTextJsonFragments(value) {
  const text = String(value == null ? '' : value);
  if (!text.includes('{"type":"text"')) return text;
  return text.replace(/\{"type":"text","data":"((?:\\.|[^"\\])*)"\}/g, (match) => {
    try {
      const parsed = JSON.parse(match);
      return cleanTextChunk(parsed && parsed.data, 12000);
    } catch {
      return '';
    }
  });
}

function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}

function baseEvent(ctx, type, defaultEngineId) {
  const message = (ctx && ctx.message) || {};
  const engine = (ctx && ctx.engine) || {};
  return {
    type,
    trace_id: cleanString(message.trace_id, 200),
    request_id: cleanString(message.request_id, 200),
    engine_id: cleanString(message.engine_id || engine.engine_id || defaultEngineId, 120),
    session_id: cleanString(message.session_id, 200),
    turn_id: cleanString(message.turn_id, 200),
  };
}

function stableRef(prefix, ctx, defaultEngineId) {
  const event = baseEvent(ctx, 'ref', defaultEngineId);
  const trace = event.trace_id || 'missing-trace';
  const session = event.session_id || 'no-session';
  const turn = event.turn_id || event.request_id || 'no-turn';
  return `${prefix}/${event.engine_id}/${trace}/${session}/${turn}`;
}

function stableExternalSessionUuid(ctx, defaultEngineId) {
  const message = (ctx && ctx.message) || {};
  const engine = cleanString(message.engine_id || defaultEngineId || 'external_cli', 120);
  const session = cleanString(message.session_id || message.agent_id || 'default', 200);
  const seed = `${engine}:${session}`;
  const hex = crypto.createHash('sha256').update(seed).digest('hex');
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    `4${hex.slice(13, 16)}`,
    `${(parseInt(hex.slice(16, 18), 16) & 0x3f | 0x80).toString(16).padStart(2, '0')}${hex.slice(18, 20)}`,
    hex.slice(20, 32),
  ].join('-');
}

function spawnCapture(command, args, options = {}) {
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 15000, 300000));
  const maxOutputBytes = Math.max(1024, Math.min(Number(options.maxOutputBytes) || 24000, 65536));
  return new Promise((resolve) => {
    const child = childProcess.spawn(command, Array.isArray(args) ? args : [], {
      cwd: options.cwd || process.cwd(),
      env: { ...process.env, ...(options.env || {}) },
      shell: false,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      try { child.kill('SIGTERM'); } catch {}
      resolve({ ok: false, timed_out: true, exit_code: null, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    }, timeoutMs);
    const append = (current, chunk) => {
      const next = Buffer.concat([current, Buffer.from(chunk || '')]);
      return next.length > maxOutputBytes ? next.subarray(next.length - maxOutputBytes) : next;
    };
    child.stdout.on('data', (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
    child.on('error', (err) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: false, timed_out: false, exit_code: null, stdout: '', stderr: cleanString(err && err.message, 2000) });
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: code === 0, timed_out: false, exit_code: code, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    });
    if (options.stdin) {
      try { child.stdin.write(String(options.stdin)); } catch {}
    }
    try { child.stdin.end(); } catch {}
  });
}

function spawnActivityCapture(command, args, options = {}) {
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 15000, 300000));
  const maxOutputBytes = Math.max(1024, Math.min(Number(options.maxOutputBytes) || 24000, 65536));
  const ctx = options.ctx || null;
  const engineId = cleanString(options.engineId || 'external_cli', 120);
  const onActivity = typeof options.onActivity === 'function' ? options.onActivity : null;
  return new Promise((resolve) => {
    const child = childProcess.spawn(command, Array.isArray(args) ? args : [], {
      cwd: options.cwd || process.cwd(),
      env: { ...process.env, ...(options.env || {}) },
      shell: false,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let stdoutLineBuffer = '';
    let activityIndex = 0;
    let settled = false;
    const append = (current, chunk) => {
      const next = Buffer.concat([current, Buffer.from(chunk || '')]);
      return next.length > maxOutputBytes ? next.subarray(next.length - maxOutputBytes) : next;
    };
    const emitJsonLine = (line) => {
      const text = String(line || '').trim();
      if (!text) return;
      let parsed = null;
      try { parsed = JSON.parse(text); } catch { return; }
      if (!parsed || typeof parsed !== 'object') return;
      const semantic = semanticCliActivityEvents([parsed], ctx, engineId)[0] || null;
      const normalized = semantic || normalizeCliActivityEvent(parsed, activityIndex, ctx, engineId);
      if (onActivity && semantic) {
        try { onActivity({ ...semantic, sequence_no: activityIndex + 1 }); } catch {}
      } else if (onActivity && shouldStreamCliRawFallbackEvent(normalized)) {
        try { onActivity(normalized); } catch {}
      }
      activityIndex += 1;
    };
    const drainStdoutLines = (chunkText, flush) => {
      stdoutLineBuffer += String(chunkText || '');
      const lines = stdoutLineBuffer.split(/\r?\n/);
      stdoutLineBuffer = flush ? '' : (lines.pop() || '');
      const ready = flush ? lines : lines;
      for (const line of ready) emitJsonLine(line);
    };
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      try { child.kill('SIGTERM'); } catch {}
      drainStdoutLines('', true);
      resolve({ ok: false, timed_out: true, exit_code: null, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    }, timeoutMs);
    child.stdout.on('data', (chunk) => {
      stdout = append(stdout, chunk);
      drainStdoutLines(Buffer.from(chunk || '').toString('utf8'), false);
    });
    child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
    child.on('error', (err) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      drainStdoutLines('', true);
      resolve({ ok: false, timed_out: false, exit_code: null, stdout: '', stderr: cleanString(err && err.message, 2000) });
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      drainStdoutLines('', true);
      resolve({ ok: code === 0, timed_out: false, exit_code: code, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    });
    if (options.stdin) {
      try { child.stdin.write(String(options.stdin)); } catch {}
    }
    try { child.stdin.end(); } catch {}
  });
}

function extractPrompt(ctx) {
  const input = ctx && ctx.message && ctx.message.input;
  const current = typeof input === 'string'
    ? cleanDisplayString(input, 12000)
    : (input && typeof input === 'object' ? cleanDisplayString(input.text || input.message || input.prompt || '', 12000) : '');
  return buildPromptWithContext(ctx && ctx.message && ctx.message.context_pack, current);
}

function fragmentSortValue(fragment) {
  const payload = fragment && fragment.payload && typeof fragment.payload === 'object' ? fragment.payload : {};
  const coverage = payload.coverage && typeof payload.coverage === 'object' ? payload.coverage : {};
  return Number(payload.sequence_no || coverage.end_seq || 0) || 0;
}

function formatContextFragment(fragment) {
  const row = fragment && typeof fragment === 'object' ? fragment : {};
  const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
  const kind = cleanString(row.kind || 'fragment', 40);
  if (kind === 'atom') {
    const role = cleanString(payload.role || payload.source_kind || 'message', 40);
    const sourceKind = cleanString(payload.source_kind || payload.record_type || '', 80);
    const speaker = cleanString(payload.speaker_label || role, 80);
    const text = cleanDisplayString(payload.text_preview || payload.summary || payload.source_ref || '', 900);
    if (!text) return '';
    return `- atom ${cleanString(row.ref_id, 80)} [${speaker}/${role}${sourceKind ? `/${sourceKind}` : ''}] ${text}`;
  }
  if (kind === 'span') {
    const coverage = payload.coverage && typeof payload.coverage === 'object' ? payload.coverage : {};
    const level = row.level == null ? '?' : String(row.level);
    const summary = cleanDisplayString(payload.summary || '', 900);
    const openLoops = Array.isArray(payload.open_loops) && payload.open_loops.length
      ? ` open_loops=${payload.open_loops.map((item) => cleanString(item, 80)).filter(Boolean).slice(0, 4).join('; ')}`
      : '';
    const constraints = Array.isArray(payload.constraints) && payload.constraints.length
      ? ` constraints=${payload.constraints.map((item) => cleanString(item, 80)).filter(Boolean).slice(0, 4).join('; ')}`
      : '';
    return `- span ${cleanString(row.ref_id, 80)} [level=${level} seq=${cleanString(coverage.start_seq, 20)}-${cleanString(coverage.end_seq, 20)}] ${summary}${openLoops}${constraints}`.trim();
  }
  return '';
}

function contextTranscriptRole(fragment) {
  const row = fragment && typeof fragment === 'object' ? fragment : {};
  const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
  const role = cleanString(payload.role || payload.source_kind || row.role || '', 40).toLowerCase();
  if (role === 'human' || role === 'user_message') return 'user';
  if (role === 'agent' || role === 'ai' || role === 'assistant' || role === 'assistant_message') return 'assistant';
  if (role === 'tool' || role === 'tool_result_bundle' || role === 'tool_receipt') return 'tool';
  if (role === 'system' || role === 'status_summary' || role === 'system_event') return 'system';
  return role || 'message';
}

function formatConversationTranscriptLine(fragment) {
  const row = fragment && typeof fragment === 'object' ? fragment : {};
  const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
  const role = contextTranscriptRole(row);
  const sourceKind = cleanString(payload.source_kind || payload.record_type || '', 80);
  const speaker = cleanString(payload.speaker_label || role, 80);
  const text = cleanDisplayString(payload.text_preview || payload.summary || row.text_preview || row.summary || '', 1600);
  if (!text) return '';
  return `${speaker} (${role}${sourceKind ? `/${sourceKind}` : ''}): ${text}`;
}

function dedupePromptLines(lines) {
  const out = [];
  const seen = new Set();
  for (const line of Array.isArray(lines) ? lines : []) {
    const text = cleanDisplayString(line, 1800);
    if (!text) continue;
    const key = text.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(text);
  }
  return out;
}

function renderRuntimeSteeringPromptSection(runtimeSteering) {
  const source = runtimeSteering && typeof runtimeSteering === 'object' ? runtimeSteering : null;
  const interventions = source && Array.isArray(source.interventions) ? source.interventions : [];
  const lines = [];
  for (const row of interventions.slice(-7)) {
    const item = row && typeof row === 'object' ? row : {};
    const text = cleanDisplayString(item.text || item.text_preview || '', 1200);
    if (!text) continue;
    const stamp = cleanString(item.created_at, 80);
    const priority = cleanString(item.priority || 'steer', 40);
    lines.push(`- ${stamp ? `${stamp} ` : ''}[${priority}] ${text}`);
  }
  const deduped = dedupePromptLines(lines).slice(-7);
  if (!deduped.length) return '';
  return [
    'Queued steering interventions:',
    '- policy: These are user steering instructions submitted while a prior runtime turn was active. Treat them as high-priority direction for this turn, but do not claim they were live-injected into the previous completed turn.',
    ...deduped,
  ].join('\n');
}

function renderRuntimeAttachmentPromptSection(runtimeAttachmentRefs) {
  const refs = runtimeAttachmentRefs && typeof runtimeAttachmentRefs === 'object' ? runtimeAttachmentRefs : null;
  const rows = refs && Array.isArray(refs.attachments) ? refs.attachments : [];
  const lines = [];
  for (const row of rows.slice(0, 12)) {
    const item = row && typeof row === 'object' ? row : {};
    const filename = cleanString(item.filename || item.name || item.file_id || 'attachment', 240);
    const fileId = cleanString(item.file_id || item.attachment_id || '', 200);
    const sourceKind = cleanString(item.source_kind || 'file_attachment', 80);
    const contentType = cleanString(item.content_type || item.mime_type || 'application/octet-stream', 120);
    const sizeBytes = Number(item.size_bytes || item.size || 0) || 0;
    const localReadPath = cleanString(item.local_read_path || item.read_path || '', 1000);
    const preview = cleanDisplayString(item.content_preview || item.text_preview || '', 3000);
    if (!filename && !preview) continue;
    lines.push(`- ${filename}${fileId ? ` [ref=${fileId}]` : ''} kind=${sourceKind} type=${contentType}${sizeBytes ? ` bytes=${sizeBytes}` : ''}`);
    if (localReadPath) lines.push(`  read_path: ${localReadPath}`);
    if (preview) lines.push(`  preview: ${preview}`);
  }
  if (!lines.length) return '';
  return [
    'Runtime attachment refs:',
    '- policy: Large pasted text and uploaded files are represented as attachment refs, not raw chat transcript. Treat pasted text attachments as supplemental user-provided context and do not ask the user to paste them again. If a read_path is present, read that file to access the full attachment body.',
    ...lines,
  ].join('\n');
}

function buildPromptWithContext(contextPack, currentPrompt) {
  const current = cleanDisplayString(currentPrompt || '', 12000);
  if (!current) return '';
  const pack = contextPack && typeof contextPack === 'object' ? contextPack : null;
  const fragments = pack && Array.isArray(pack.fragments) ? pack.fragments.slice() : [];
  const toolGrantSection = renderUniversalToolGrantPromptSection(pack && pack.universal_tool_grants);
  const steeringSection = renderRuntimeSteeringPromptSection(pack && pack.runtime_steering);
  const attachmentSection = renderRuntimeAttachmentPromptSection(pack && pack.runtime_attachment_refs);
  if (!pack) return current;
  if (fragments.length === 0 && !toolGrantSection && !steeringSection && !attachmentSection) return current;
  const hot = fragments
    .filter((row) => row && row.kind === 'atom')
    .sort((a, b) => fragmentSortValue(a) - fragmentSortValue(b))
    .slice(-12);
  const hotContextFragments = hot
    .map(formatContextFragment)
    .filter(Boolean)
    .slice(-6);
  const conversationTranscript = dedupePromptLines(hot.map(formatConversationTranscriptLine)).slice(-12);
  const spans = fragments
    .filter((row) => row && row.kind === 'span')
    .sort((a, b) => {
      const levelDelta = Number(a.level || 0) - Number(b.level || 0);
      return levelDelta || fragmentSortValue(a) - fragmentSortValue(b);
    })
    .map(formatContextFragment)
    .filter(Boolean)
    .slice(-8);
  const lines = [
    'InfRing bounded context pack:',
    `- source_basis: ${cleanString(pack.source_basis || 'core.layer2.memory.context_topology_projection', 120)}`,
    `- source_authority: ${cleanString(pack.source_authority || 'gateway_bounded_projection', 160)}`,
    `- gateway_generated_at: ${new Date().toISOString()}`,
    `- session_id: ${cleanString(pack.session_id, 120)}`,
    `- fanout_target: ${cleanString(pack.fanout_target || 7, 20)}`,
    '- trust_model: visible transcript rows are context; receipts are facts; tool proposals are intentions only until an approval decision and durable receipt exist.',
    '- temporal_precedence: if dates conflict, prefer explicit Gateway/runtime timestamps in this context pack, then the controlling runtime environment, then transcript text as historical evidence.',
    '- policy: The Recent conversation transcript below is visible prior chat context from this InfRing session. Treat it as the conversation history you can see. Do not claim you have no prior messages when transcript rows are present. If the user says "try again", infer the retry target from that transcript when possible.',
    '- permission bridge: If the current user turn requires creating, editing, deleting, running commands, or writing memory and your runtime/tooling cannot proceed because approval is missing, do not stop with a generic refusal. Attempt the native action/proposal path if available. If it is blocked, report the exact blocked action and permission reason so InfRing Gateway can convert it into a user approval request.',
  ];
  if (conversationTranscript.length) lines.push('', 'Recent conversation transcript:', ...conversationTranscript);
  if (spans.length) lines.push('', 'Selected context spans:', ...spans);
  if (hotContextFragments.length) lines.push('', 'Recent context atoms:', ...hotContextFragments);
  if (attachmentSection) lines.push('', attachmentSection);
  if (steeringSection) lines.push('', steeringSection);
  if (toolGrantSection) lines.push('', toolGrantSection);
  lines.push('', 'Current user turn:', current);
  return cleanDisplayString(lines.join('\n'), 24000);
}

function resolveTurnTimeoutMs(ctx, fallbackTimeoutMs) {
  const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
  const budget = message.capability_budget && typeof message.capability_budget === 'object'
    ? message.capability_budget
    : {};
  const budgetSeconds = Number(budget.max_turn_seconds || 0);
  const budgetTimeoutMs = Number.isFinite(budgetSeconds) && budgetSeconds > 0
    ? budgetSeconds * 1000
    : 0;
  const fallback = Number(fallbackTimeoutMs || 0);
  const selected = budgetTimeoutMs || (Number.isFinite(fallback) && fallback > 0 ? fallback : 60000);
  return Math.max(1000, Math.min(selected, 300000));
}

function cliRuntimeFailureText(engineId, run, timeoutMs) {
  const cleanEngine = cleanString(engineId || 'external_cli', 120);
  const timeoutSeconds = Math.max(1, Math.round(Number(timeoutMs || 0) / 1000));
  if (run && run.timed_out) {
    return `${cleanEngine} did not finish within ${timeoutSeconds}s. The external runtime process was stopped by the InfRing Gateway turn timeout.`;
  }
  const stderr = cleanDisplayString(run && run.stderr, 4000);
  if (stderr) return stderr;
  const stdout = cleanDisplayString(run && run.stdout, 4000);
  if (stdout) return stdout;
  const exitCode = run && run.exit_code != null ? String(run.exit_code) : 'unknown';
  return `${cleanEngine} exited without a usable assistant response (exit_code=${exitCode}).`;
}

function classifyCliRuntimeFailureCode(engineId, run, failureText) {
  const cleanEngine = cleanString(engineId || 'external_cli', 120);
  if (run && run.timed_out) return `${cleanEngine}_turn_timeout`;
  const text = cleanDisplayString([
    failureText,
    run && run.stderr,
    run && run.stdout,
  ].filter(Boolean).join('\n'), 12000).toLowerCase();
  if (
    text.includes('quota') ||
    text.includes('credit') ||
    text.includes('billing') ||
    text.includes('subscription') ||
    text.includes('payment required') ||
    text.includes('insufficient balance')
  ) {
    return `${cleanEngine}_provider_quota_or_subscription_unavailable`;
  }
  if (
    text.includes('unauthorized') ||
    text.includes('not authorized') ||
    text.includes('authentication') ||
    text.includes('auth required') ||
    text.includes('login required') ||
    text.includes('please login') ||
    text.includes('please log in') ||
    text.includes('api key') ||
    text.includes('invalid token') ||
    text.includes('token expired')
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
    text.includes('econnreset') ||
    text.includes('etimedout') ||
    text.includes('enotfound') ||
    text.includes('connection refused') ||
    text.includes('connection reset')
  ) {
    return `${cleanEngine}_provider_network_unavailable`;
  }
  return `${cleanEngine}_turn_failed`;
}

function appendCliRuntimeFailureEvent(events, ctx, engineId, run, timeoutMs) {
  const rows = Array.isArray(events) ? events.slice() : [];
  if (run && run.ok) return rows;
  const text = cliRuntimeFailureText(engineId, run, timeoutMs);
  rows.push({
    ...baseEvent(ctx, 'agent_activity_event', engineId),
    type: 'agent_activity_event',
    activity_kind: 'error',
    provider_event_type: run && run.timed_out ? 'turn.timeout' : 'turn.failed',
    source: 'external_cli_process_lifecycle',
    sequence_no: rows.length + 1,
    item_id: run && run.timed_out ? 'external-cli-timeout' : 'external-cli-failure',
    status: 'failed',
    text,
    display_text: text,
  });
  return rows.slice(-80);
}

function parseJsonlRows(raw) {
  return String(raw == null ? '' : raw)
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .flatMap(parseJsonObjectsFromLine)
    .filter((row) => row && typeof row === 'object');
}

function parseJsonObjectsFromLine(line) {
  const text = String(line || '').trim();
  if (!text) return [];
  try {
    const parsed = JSON.parse(text);
    return parsed && typeof parsed === 'object' ? [parsed] : [];
  } catch {}
  const out = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (ch === '\\') {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === '{') {
      if (depth === 0) start = i;
      depth += 1;
      continue;
    }
    if (ch === '}') {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        const candidate = text.slice(start, i + 1);
        try {
          const parsed = JSON.parse(candidate);
          if (parsed && typeof parsed === 'object') out.push(parsed);
        } catch {}
        start = -1;
      }
    }
  }
  return out;
}

function compactEventType(row) {
  const direct = cleanString(row && (row.type || row.event || row.method || row.name || row.kind), 160);
  if (direct) return direct;
  const item = row && row.item && typeof row.item === 'object' ? row.item : null;
  return cleanString(item && (item.type || item.kind || item.name), 160) || 'activity';
}

function classifyActivityKind(eventType, row) {
  const type = cleanString(eventType, 160).toLowerCase();
  const item = row && row.item && typeof row.item === 'object' ? row.item : {};
  const itemType = cleanString(item.type || item.kind || item.name, 160).toLowerCase();
  const joined = `${type} ${itemType}`;
  if (joined.includes('reasoning')) return 'reasoning_summary';
  if (joined.includes('thought')) return 'reasoning_summary';
  if (joined.includes('plan')) return 'plan_update';
  if (joined.includes('permission') || joined.includes('approval')) return 'permission_event';
  if (joined.includes('tool') || joined.includes('mcp')) return 'tool_call_event';
  if (joined.includes('command') || joined.includes('exec') || joined.includes('shell') || joined.includes('bash')) return 'command_event';
  if (joined.includes('file') || joined.includes('patch') || joined.includes('diff')) return 'file_change_event';
  if (joined.includes('message') || joined.includes('delta') || joined.includes('response') || joined.includes('result')) return 'assistant_delta';
  if (type === 'text') return 'assistant_delta';
  if (joined.includes('error') || joined.includes('failed')) return 'error';
  if (joined.includes('start')) return 'started';
  if (type === 'end') return 'completed';
  if (joined.includes('complete') || joined.includes('finish')) return 'completed';
  return 'activity';
}

function extractTextFromContent(value, maxLen = 4000, preserveWhitespace = false) {
  if (value == null) return '';
  if (typeof value === 'string') {
    const decoded = decodeEmbeddedTextJsonFragments(value);
    return preserveWhitespace ? cleanTextChunk(decoded, maxLen) : cleanDisplayString(decoded, maxLen);
  }
  if (Array.isArray(value)) {
    const joined = value.map((item) => extractTextFromContent(item, maxLen, preserveWhitespace)).filter(Boolean).join('');
    return preserveWhitespace ? cleanTextChunk(joined, maxLen) : cleanDisplayString(joined, maxLen);
  }
  if (typeof value !== 'object') return cleanDisplayString(value, maxLen);
  if (cleanString(value.type || value.kind, 80).toLowerCase() === 'text' && value.data != null) {
    return extractTextFromContent(value.data, maxLen, true);
  }
  if (value.content != null) {
    const contentText = extractTextFromContent(value.content, maxLen, preserveWhitespace);
    if (contentText) return contentText;
  }
  const text =
    value.text ||
      value.delta ||
      value.message ||
      value.summary ||
      value.output_text ||
      value.output ||
      value.result ||
      value.data ||
      value.transcript ||
      '';
  return preserveWhitespace ? cleanTextChunk(decodeEmbeddedTextJsonFragments(text), maxLen) : cleanDisplayString(decodeEmbeddedTextJsonFragments(text), maxLen);
}

function extractActivityText(row, kind) {
  if (!row || typeof row !== 'object') return '';
  const item = row.item && typeof row.item === 'object' ? row.item : {};
  const direct = extractTextFromContent(
    row.delta ||
      row.text ||
      row.message ||
      row.summary ||
      row.content ||
      row.result ||
      row.data ||
      row.output_text ||
      row.output ||
      item.delta ||
      item.text ||
      item.message ||
      item.summary ||
      item.content ||
      item.data ||
      item.output_text ||
      item.output,
    kind === 'assistant_delta' ? 12000 : 4000,
    kind === 'assistant_delta',
  );
  if (direct) return direct;
  const command = cleanDisplayString(row.command || item.command || row.cmd || item.cmd || '', 2000);
  if (command) return command;
  const tool = cleanDisplayString(row.tool || item.tool || row.tool_name || item.tool_name || row.name || item.name || '', 1000);
  if (tool) return tool;
  return cleanDisplayString(JSON.stringify(row), 1200);
}

function normalizeCliActivityEvent(row, index, ctx, defaultEngineId) {
  const eventType = compactEventType(row);
  const kind = classifyActivityKind(eventType, row);
  const text = extractActivityText(row, kind);
  const base = baseEvent(ctx, kind, defaultEngineId);
  const item = row && row.item && typeof row.item === 'object' ? row.item : {};
  return {
    ...base,
    type: 'agent_activity_event',
    activity_kind: kind,
    provider_event_type: eventType,
    source: 'external_cli_stream',
    sequence_no: index + 1,
    item_id: cleanString(row && (row.item_id || row.itemId || row.id || item.id || item.item_id || item.itemId), 200),
    status: cleanString(row && (row.status || row.state || item.status || item.state), 80),
    text: cleanDisplayString(text, kind === 'assistant_delta' ? 12000 : 4000),
    display_text: cleanDisplayString(text, kind === 'assistant_delta' ? 12000 : 4000),
  };
}

function shouldStreamCliActivityEvent(event) {
  const row = event && typeof event === 'object' ? event : {};
  const kind = cleanString(row.activity_kind || row.kind || '', 80);
  if (kind === 'assistant_delta') return false;
  if (kind === 'reasoning_summary') return false;
  return true;
}

function shouldStreamCliRawFallbackEvent(event) {
  const row = event && typeof event === 'object' ? event : {};
  const kind = cleanString(row.activity_kind || row.kind || '', 80).toLowerCase();
  const providerType = cleanString(row.provider_event_type || row.event_type || '', 160).toLowerCase();
  const text = cleanDisplayString(row.display_text || row.text || row.summary || '', 500);
  if (!shouldStreamCliActivityEvent(row)) return false;
  if (kind === 'error' || providerType.includes('error') || providerType.includes('fail')) return true;
  if (kind === 'permission_event' || providerType.includes('permission') || providerType.includes('approval')) return true;
  if (text.startsWith('{') || text.startsWith('[')) return false;
  if (kind === 'started' || kind === 'completed') return false;
  return false;
}

function compactCliActivityEvents(rows, ctx, defaultEngineId) {
  const normalized = (Array.isArray(rows) ? rows : [])
    .map((row, index) => normalizeCliActivityEvent(row, index, ctx, defaultEngineId))
    .filter((event) => event && (event.text || event.provider_event_type));
  const out = [];
  let assistantDeltaCount = 0;
  let assistantPreview = '';
  let reasoningCount = 0;
  for (const event of normalized) {
    if (event.activity_kind === 'assistant_delta') {
      assistantDeltaCount += 1;
      if (event.display_text) assistantPreview = cleanDisplayString(`${assistantPreview}${event.display_text}`, 1200);
      continue;
    }
    if (event.activity_kind === 'reasoning_summary') {
      reasoningCount += 1;
      continue;
    }
    if (event.activity_kind === 'completed' && cleanString(event.provider_event_type, 80).toLowerCase() === 'end') {
      out.push({
        ...event,
        text: 'Runtime reported end of turn.',
        display_text: 'Runtime reported end of turn.',
      });
      continue;
    }
    out.push(event);
  }
  if (reasoningCount > 0) {
    out.push({
      ...baseEvent(ctx, 'agent_activity_event', defaultEngineId),
      type: 'agent_activity_event',
      activity_kind: 'reasoning_summary',
      provider_event_type: 'reasoning_summary.compacted',
      source: 'external_cli_stream',
      sequence_no: normalized.length,
      item_id: `reasoning-compact-${reasoningCount}`,
      status: 'completed',
      text: `Runtime emitted ${reasoningCount} reasoning/status chunk${reasoningCount === 1 ? '' : 's'}; details were compacted by Gateway.`,
      display_text: `Runtime emitted ${reasoningCount} reasoning/status chunk${reasoningCount === 1 ? '' : 's'}; details were compacted by Gateway.`,
    });
  }
  if (assistantDeltaCount > 0) {
    out.push({
      ...baseEvent(ctx, 'agent_activity_event', defaultEngineId),
      type: 'agent_activity_event',
      activity_kind: 'assistant_delta',
      provider_event_type: 'assistant_delta.compacted',
      source: 'external_cli_stream',
      sequence_no: normalized.length + 1,
      item_id: `assistant-delta-compact-${assistantDeltaCount}`,
      status: 'completed',
      text: assistantPreview
        ? `Assistant draft streamed ${assistantDeltaCount} text chunk${assistantDeltaCount === 1 ? '' : 's'}; final answer is shown in the message.`
        : `Assistant draft streamed ${assistantDeltaCount} text chunk${assistantDeltaCount === 1 ? '' : 's'}.`,
      display_text: assistantPreview
        ? `Assistant draft streamed ${assistantDeltaCount} text chunk${assistantDeltaCount === 1 ? '' : 's'}; final answer is shown in the message.`
        : `Assistant draft streamed ${assistantDeltaCount} text chunk${assistantDeltaCount === 1 ? '' : 's'}.`,
    });
  }
  return out.slice(-40);
}

function outputTextFromActivityRows(rows, fallbackText) {
  let finalText = '';
  const assistantParts = [];
  const reasoningParts = [];
  for (const row of rows) {
    const eventType = compactEventType(row).toLowerCase();
    const kind = classifyActivityKind(eventType, row);
    const text = extractActivityText(row, kind);
    if (!text) continue;
    if (
      row.is_final === true ||
      row.final === true ||
      eventType === 'result' ||
      eventType === 'turn.complete' ||
      eventType === 'turn.completed' ||
      eventType.includes('turn/completed') ||
      eventType.includes('completed')
    ) {
      if (kind === 'assistant_delta' || row.result || row.output || row.output_text || row.message) finalText = text;
    }
    if (kind === 'assistant_delta') assistantParts.push(text);
    if (kind === 'reasoning_summary') reasoningParts.push(text);
  }
  return cleanDisplayString(
    decodeEmbeddedTextJsonFragments(
      finalText ||
        assistantParts.join('') ||
        extractArtifactLikeAnswerFromReasoning(reasoningParts.join('')) ||
        fallbackText ||
        '',
    ),
    64000,
  );
}

function extractArtifactLikeAnswerFromReasoning(reasoningText) {
  const text = decodeEmbeddedTextJsonFragments(reasoningText || '');
  if (!text.trim()) return '';
  const fenced = Array.from(text.matchAll(/```[\s\S]*?```/g)).map((match) => match[0]).filter(Boolean);
  if (fenced.length) return fenced[fenced.length - 1];
  const lines = text.split('\n');
  const codeStart = lines.findIndex((line) => /^\s*(def|class|import|from|while|for|if|print\(|[a-zA-Z_][a-zA-Z0-9_]*\s*=)/.test(line));
  if (codeStart >= 0) return lines.slice(codeStart).join('\n');
  return '';
}

function permissionDeniedText(value) {
  const text = cleanDisplayString(value, 4000);
  if (!text) return '';
  const lower = text.toLowerCase();
  if (
    lower.includes('blocked action:') ||
    lower.includes('permission reason:') ||
    lower.includes("couldn't create") ||
    lower.includes('couldn’t create') ||
    lower.includes('could not create') ||
    lower.includes('could not write') ||
    lower.includes('filesystem is read-only') ||
    lower.includes('filesystem is read only') ||
    lower.includes('runtime filesystem is read-only') ||
    lower.includes('runtime filesystem is read only') ||
    lower.includes('approval policy is never') ||
    lower.includes('native filesystem write is blocked') ||
    lower.includes('filesystem write is blocked') ||
    lower.includes('write is blocked') ||
    lower.includes('permission denied') ||
    lower.includes('permission was denied') ||
    lower.includes('permissions denied') ||
    lower.includes('permission is denied') ||
    lower.includes("don't have permission") ||
    lower.includes('dont have permission') ||
    lower.includes('do not have permission') ||
    lower.includes("didn't have permission") ||
    lower.includes('did not have permission') ||
    lower.includes('no permission') ||
    lower.includes('lacks permission') ||
    lower.includes('insufficient permission') ||
    lower.includes('insufficient permissions') ||
    lower.includes('write permission') ||
    lower.includes('edit permission') ||
    lower.includes('permission to write') ||
    lower.includes('permission to edit') ||
    lower.includes('permission to create') ||
    lower.includes('permission to save') ||
    lower.includes('not allowed to write') ||
    lower.includes('not allowed to edit') ||
    lower.includes('not allowed to create') ||
    lower.includes('cannot write') ||
    lower.includes("can't write") ||
    lower.includes('unable to write') ||
    lower.includes('cannot create') ||
    lower.includes("can't create") ||
    lower.includes('unable to create') ||
    lower.includes('approve the write') ||
    lower.includes('approve this write') ||
    lower.includes('approve file') ||
    lower.includes('approval to write') ||
    lower.includes('approval to create') ||
    lower.includes('approval to edit') ||
    lower.includes('requires permission') ||
    lower.includes('requires approval') ||
    lower.includes('approval required') ||
    lower.includes('needs approval') ||
    lower.includes('need approval') ||
    lower.includes('needs permission') ||
    lower.includes('need permission') ||
    lower.includes('blocked by permission') ||
    lower.includes('blocked by sandbox') ||
    lower.includes('read-only sandbox') ||
    lower.includes('read only sandbox') ||
    lower.includes('sandbox is read-only') ||
    lower.includes('sandbox is read only') ||
    lower.includes('readonly sandbox') ||
    lower.includes('restricted to read-only') ||
    lower.includes('restricted to read only')
  ) {
    return text;
  }
  return '';
}

function extractUniversalToolProposals(value) {
  const text = cleanDisplayString(value, 64000);
  if (!text || !text.includes('infring_universal_tool_proposal')) return [];
  return parseJsonObjectsFromLine(text)
    .filter((row) => row && row.type === 'infring_universal_tool_proposal')
    .slice(0, 8);
}

function sanitizeProposalArguments(args, toolId) {
  const source = args && typeof args === 'object' ? args : {};
  const out = {};
  if (toolId === 'artifact.create_propose') {
    const rawPath = cleanString(source.path || source.file || source.filename || source.relative_path || '', 500);
    if (rawPath) out.path = rawPath;
    const mimeType = cleanString(source.mime_type || source.content_type || 'text/plain', 120);
    if (mimeType) out.mime_type = mimeType;
    if (source.content != null) out.content = cleanDisplayString(source.content, 262144);
    else if (source.text != null) out.content = cleanDisplayString(source.text, 262144);
    else if (source.body != null) out.content = cleanDisplayString(source.body, 262144);
    return out;
  }
  for (const key of Object.keys(source).slice(0, 24)) {
    const cleanKey = cleanString(key, 80);
    if (!cleanKey) continue;
    const value = source[key];
    if (value == null) continue;
    if (typeof value === 'string') out[cleanKey] = cleanDisplayString(value, 12000);
    else if (typeof value === 'number' || typeof value === 'boolean') out[cleanKey] = value;
    else {
      try {
        out[cleanKey] = cleanDisplayString(JSON.stringify(value), 12000);
      } catch {
        out[cleanKey] = cleanString(value, 1000);
      }
    }
  }
  return out;
}

function collectPermissionDenials(rows, fallbackText) {
  const out = [];
  const add = (value) => {
    const text = permissionDeniedText(value);
    if (text) out.push(text);
  };
  for (const row of Array.isArray(rows) ? rows : []) {
    if (!row || typeof row !== 'object') continue;
    const denials = Array.isArray(row.permission_denials) ? row.permission_denials : [];
    for (const denial of denials) {
      const denialText = cleanDisplayString(extractTextFromContent(denial, 4000) || JSON.stringify(denial), 4000);
      if (denialText) out.push(denialText);
    }
    add(row.permission_denial || row.denial || row.error || row.reason || row.message || row.result || row.output || row.stderr || row.data);
    const eventType = compactEventType(row).toLowerCase();
    if (eventType.includes('permission') || eventType.includes('approval')) {
      const eventText = cleanDisplayString(JSON.stringify(row), 4000);
      if (eventText) out.push(eventText);
    }
  }
  add(fallbackText);
  return Array.from(new Set(out)).slice(0, 8);
}

function inferPermissionToolId(text) {
  const lower = cleanDisplayString(text, 4000).toLowerCase();
  if (lower.includes('memory') || lower.includes('remember')) return 'memory.write_propose';
  if (
    lower.includes('write') ||
    lower.includes('edit') ||
    lower.includes('file') ||
    lower.includes('create') ||
    lower.includes('patch') ||
    lower.includes('save')
  ) {
    return 'artifact.create_propose';
  }
  return 'permission.request';
}

function buildPermissionRequestFromDenials(denials, ctx, defaultEngineId) {
  if (!Array.isArray(denials) || !denials.length) return null;
  const text = cleanDisplayString(denials[0], 1000);
  const toolId = inferPermissionToolId(text);
  const base = baseEvent(ctx, 'permission.requested', defaultEngineId);
  const turnId = base.turn_id || base.request_id || 'turn';
  const approvalId = cleanString(`approval_${toolId}_${base.trace_id || 'trace'}_${turnId}`, 260)
    .replace(/[^a-zA-Z0-9_.:-]+/g, '_')
    .replace(/^_+|_+$/g, '');
  return {
    type: 'permission.requested',
    approval_id: approvalId,
    trace_id: base.trace_id,
    request_id: base.request_id,
    engine_id: base.engine_id,
    session_id: base.session_id,
    turn_id: turnId,
    tool_call_ref: `external-permission-denial/${toolId}/${base.trace_id || 'trace'}/${turnId}`,
    tool_id: toolId,
    capability: toolId === 'artifact.create_propose'
      ? 'propose_artifact_create'
      : toolId === 'memory.write_propose'
        ? 'propose_memory_write'
        : 'request_permission',
    reason: cleanDisplayString(text || 'External runtime requested permission to continue.', 1000),
    argument_keys: [],
    gatekeeper_kind: 'user',
    future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor'],
    decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
    decision_scope: 'tool_call',
    approval_route: `/api/shell-socket/approvals/${encodeURIComponent(approvalId)}/decision`,
    source: 'external_cli_permission_denial_normalizer',
    future_gatekeeper_contract: {
      current_gatekeeper_kind: 'user',
      supported_future_gatekeepers: ['system_policy', 'agent_supervisor', 'admin_agent'],
      durable_effect_executed_before_approval: false,
    },
  };
}

function buildPermissionRequestFromProposal(proposal, ctx, defaultEngineId) {
  const row = proposal && typeof proposal === 'object' ? proposal : null;
  if (!row || row.type !== 'infring_universal_tool_proposal') return null;
  const toolId = cleanString(row.tool_id || row.capability || 'permission.request', 120);
  const args = row.arguments && typeof row.arguments === 'object' ? row.arguments : {};
  const base = baseEvent(ctx, 'permission.requested', defaultEngineId);
  const turnId = base.turn_id || base.request_id || 'turn';
  const approvalId = cleanString(`approval_${toolId}_${base.trace_id || 'trace'}_${turnId}`, 260)
    .replace(/[^a-zA-Z0-9_.:-]+/g, '_')
    .replace(/^_+|_+$/g, '');
  const reason = cleanDisplayString(
    row.reason ||
      (toolId === 'artifact.create_propose'
        ? `External runtime proposed creating or changing ${cleanString(args.path || args.file || 'an artifact', 240)}.`
        : 'External runtime proposed a gated InfRing tool call.'),
    1000,
  );
  return {
    type: 'permission.requested',
    approval_id: approvalId,
    trace_id: base.trace_id,
    request_id: base.request_id,
    engine_id: base.engine_id,
    session_id: base.session_id,
    turn_id: turnId,
    tool_call_ref: `external-tool-proposal/${toolId}/${base.trace_id || 'trace'}/${turnId}`,
    tool_id: toolId,
    capability: toolId === 'artifact.create_propose'
      ? 'propose_artifact_create'
      : toolId === 'memory.write_propose'
        ? 'propose_memory_write'
        : toolId === 'permission.request'
          ? 'request_permission'
          : cleanString(row.capability || toolId, 160),
    reason,
    argument_keys: Object.keys(args).map((key) => cleanString(key, 80)).filter(Boolean).slice(0, 24),
    proposal_arguments: sanitizeProposalArguments(args, toolId),
    gatekeeper_kind: 'user',
    future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor'],
    decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
    decision_scope: 'tool_call',
    approval_route: `/api/shell-socket/approvals/${encodeURIComponent(approvalId)}/decision`,
    source: 'external_cli_universal_tool_proposal_normalizer',
    proposal_ref: `external-tool-proposal/${toolId}/${base.trace_id || 'trace'}/${turnId}`,
    future_gatekeeper_contract: {
      current_gatekeeper_kind: 'user',
      supported_future_gatekeepers: ['system_policy', 'agent_supervisor', 'admin_agent'],
      durable_effect_executed_before_approval: false,
    },
  };
}

function buildPermissionRequestFromProposals(proposals, ctx, defaultEngineId) {
  if (!Array.isArray(proposals) || !proposals.length) return null;
  return buildPermissionRequestFromProposal(proposals[0], ctx, defaultEngineId);
}

function semanticCompactText(value, max = 220) {
  return cleanDisplayString(value, max).replace(/\n+/g, ' ').replace(/\s{2,}/g, ' ').trim();
}

function firstSemanticString(value, keys, depth = 0) {
  if (!value || depth > 5) return '';
  if (typeof value === 'string') return '';
  if (Array.isArray(value)) {
    for (const item of value.slice(0, 16)) {
      const found = firstSemanticString(item, keys, depth + 1);
      if (found) return found;
    }
    return '';
  }
  if (typeof value !== 'object') return '';
  for (const key of keys) {
    const raw = value[key];
    if (typeof raw === 'string' || typeof raw === 'number') {
      const text = semanticCompactText(raw, 500);
      if (text) return text;
    }
    if (Array.isArray(raw) && raw.every((item) => typeof item === 'string' || typeof item === 'number')) {
      const text = semanticCompactText(raw.join(' '), 500);
      if (text) return text;
    }
  }
  for (const child of Object.values(value).slice(0, 24)) {
    const found = firstSemanticString(child, keys, depth + 1);
    if (found) return found;
  }
  return '';
}

function semanticProviderStatus(row) {
  const eventType = compactEventType(row).toLowerCase();
  if (eventType.includes('started') || eventType.includes('running') || eventType.includes('progress')) return 'running';
  if (eventType.includes('completed') || eventType.includes('finished') || eventType.includes('done')) return 'completed';
  if (eventType.includes('failed') || eventType.includes('error')) return 'failed';
  const status = firstSemanticString(row, ['status', 'state', 'phase', 'outcome'], 0).toLowerCase();
  if (status.includes('fail') || status.includes('error')) return 'failed';
  if (status.includes('complete') || status.includes('done') || status.includes('success')) return 'completed';
  if (status.includes('start') || status.includes('running') || status.includes('progress')) return 'running';
  return '';
}

function semanticProviderTarget(row) {
  return firstSemanticString(row, [
    'path',
    'file',
    'file_path',
    'filename',
    'target_path',
    'relative_path',
    'uri',
    'artifact_path',
  ], 0);
}

function semanticProviderCommand(row) {
  return firstSemanticString(row, [
    'command',
    'cmd',
    'shell_command',
    'bash_command',
    'exec_command',
    'argv',
  ], 0);
}

function semanticProviderTool(row) {
  return firstSemanticString(row, [
    'tool_name',
    'tool',
    'name',
    'function_name',
    'call_name',
  ], 0);
}

function semanticProviderQuery(row) {
  return firstSemanticString(row, [
    'query',
    'pattern',
    'search',
    'grep',
    'regex',
  ], 0);
}

function semanticActivityKindFromRow(row) {
  const type = compactEventType(row).toLowerCase();
  if (type.includes('command') || type.includes('bash') || type.includes('shell') || type.includes('exec')) return 'command';
  if (type.includes('file') || type.includes('edit') || type.includes('patch') || type.includes('write')) return 'file_change';
  if (type.includes('search') || type.includes('grep') || type.includes('find')) return 'search';
  if (type.includes('tool')) return 'tool_call';
  if (type.includes('complete') || type.includes('result')) return 'completed';
  return 'activity';
}

function semanticActivityTextFromRow(row, defaultEngineId) {
  const type = compactEventType(row).toLowerCase();
  if (!type || type === 'thread.started' || type === 'turn.started') return '';
  if (type === 'assistant_delta' || type === 'assistant_delta.compacted' || type.includes('partial')) return '';
  const status = semanticProviderStatus(row);
  const statusPrefix = status === 'completed' ? 'Completed' : status === 'failed' ? 'Failed' : 'Working on';
  const command = semanticProviderCommand(row);
  if (command || type.includes('command') || type.includes('bash') || type.includes('shell') || type.includes('exec')) {
    return command
      ? `${statusPrefix} command: ${command}`
      : `${statusPrefix} a shell command.`;
  }
  const query = semanticProviderQuery(row);
  if (query || type.includes('search') || type.includes('grep') || type.includes('find')) {
    return query
      ? `${statusPrefix} search: ${query}`
      : `${statusPrefix} a workspace search.`;
  }
  const target = semanticProviderTarget(row);
  if (target || type.includes('file') || type.includes('edit') || type.includes('patch') || type.includes('write')) {
    if (type.includes('read') || type.includes('open')) {
      return target ? `Reading file: ${target}` : 'Reading a file.';
    }
    return target
      ? `${statusPrefix} file change: ${target}`
      : `${statusPrefix} a file change.`;
  }
  const tool = semanticProviderTool(row);
  if (tool || type.includes('tool')) {
    return tool
      ? `${statusPrefix} tool: ${tool}`
      : `${statusPrefix} a tool call.`;
  }
  if (type.includes('complete') || type.includes('result')) {
    return `${cleanString(defaultEngineId || 'agent_runtime', 80)} completed the turn; final answer is shown in the message.`;
  }
  const message = firstSemanticString(row, ['message', 'summary', 'title', 'description', 'reason'], 0);
  if (message && !message.startsWith('{')) return message;
  return '';
}

function semanticCliActivityEvents(rows, ctx, defaultEngineId) {
  const out = [];
  const seen = new Set();
  for (const row of Array.isArray(rows) ? rows : []) {
    const text = semanticActivityTextFromRow(row, defaultEngineId);
    if (!text) continue;
    const providerType = compactEventType(row);
    const key = `${providerType}|${text}`.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({
      ...baseEvent(ctx, providerType || 'activity', defaultEngineId),
      activity_kind: semanticActivityKindFromRow(row),
      provider_event_type: providerType,
      status: semanticProviderStatus(row) || 'done',
      display_text: cleanDisplayString(text, 1000),
    });
  }
  return out.slice(-48);
}

function parseCliActivityOutput(stdout, stderr, ctx, defaultEngineId) {
  const rows = parseJsonlRows(stdout);
  if (!rows.length) {
    const output = cleanDisplayString(stdout || stderr, 64000);
    const permissionDenials = collectPermissionDenials([], output);
    const permissionProposals = extractUniversalToolProposals(output);
    return {
      output_text: output,
      output_preview: cleanString(output, 4000),
      activity_events: [],
      activity_event_count: 0,
      structured_activity: false,
      permission_denials: permissionDenials,
      permission_request: buildPermissionRequestFromProposals(permissionProposals, ctx, defaultEngineId) ||
        buildPermissionRequestFromDenials(permissionDenials, ctx, defaultEngineId),
    };
  }
  const compactEvents = compactCliActivityEvents(rows, ctx, defaultEngineId);
  const semanticEvents = semanticCliActivityEvents(rows, ctx, defaultEngineId);
  const assistantSummaryEvents = compactEvents.filter((event) =>
    event && (event.activity_kind === 'assistant_delta' || event.provider_event_type === 'assistant_delta.compacted')
  );
  const events = semanticEvents.length
    ? [...semanticEvents, ...assistantSummaryEvents].slice(-80)
    : compactEvents;
  const output = outputTextFromActivityRows(rows, stderr);
  const permissionDenials = collectPermissionDenials(rows, [output, stderr].filter(Boolean).join('\n'));
  const permissionProposals = extractUniversalToolProposals([output, stderr].filter(Boolean).join('\n'));
  return {
    output_text: output,
    output_preview: cleanString(output, 4000),
    activity_events: events,
    activity_event_count: rows.length,
    structured_activity: true,
    permission_denials: permissionDenials,
    permission_request: buildPermissionRequestFromProposals(permissionProposals, ctx, defaultEngineId) ||
      buildPermissionRequestFromDenials(permissionDenials, ctx, defaultEngineId),
  };
}

function createCliRuntimeEngineAdapter(options = {}) {
  const engineId = cleanString(options.engineId || 'external_cli', 120);
  const engineKind = cleanString(options.engineKind || 'external_cli_adapter', 120);
  const contextTransportMode = cleanString(options.contextTransportMode || 'prompt_text_compat', 80);
  const structuredTransportTarget = cleanString(options.structuredTransportTarget || 'structured_json', 80);
  const transportMigrationStatus = cleanString(options.transportMigrationStatus || 'transitional_bootstrap', 120);
  const downloadActionRef = cleanString(options.downloadActionRef || `agent_runtime_download/${engineId}`, 500);
  const artifactKind = cleanString(options.artifactKind || `${engineId}_result_projection`, 120);
  const receiptKind = cleanString(options.receiptKind || 'external_cli_adapter_receipt', 120);
  const liveEnvVar = cleanString(options.liveEnvVar || `INFRING_AGENT_RUNTIME_${engineId.toUpperCase()}_LIVE`, 120);
  const liveDispatch = options.liveDispatch === true || process.env[liveEnvVar] === '1';
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 60000, 300000));
  const versionArgs = Array.isArray(options.versionArgs) ? options.versionArgs : ['--version'];
  const runArgs = typeof options.runArgs === 'function' ? options.runArgs : (prompt) => [prompt];
  let selectedCommand = cleanString(options.command || options.commandFallback || engineId, 500);

  function discover(ctx) {
    const engine = (ctx && ctx.engine) || { engine_id: engineId };
    const discovery = resolveEngineDiscovery(engine, {
      command: options.command,
      config: options.config,
      env: options.env || process.env,
    });
    if (discovery.command) selectedCommand = cleanString(discovery.command, 500);
    return discovery;
  }

  async function submitTurnWithOptionalStream(ctx) {
    const prompt = extractPrompt(ctx);
    if (!prompt) {
      return {
        ...baseEvent(ctx, 'error', engineId),
        error_code: `${engineId}_prompt_missing`,
        reason: `${engineId} adapter requires input.text, input.message, input.prompt, or string input.`,
        retryable: false,
      };
    }
    if (!liveDispatch) {
      return {
        ...baseEvent(ctx, 'error', engineId),
        error_code: `${engineId}_live_dispatch_disabled`,
        reason: `${engineId} adapter is installed as a bounded external runtime seam; live dispatch requires ${liveEnvVar}=1 or adapter option liveDispatch=true.`,
        retryable: false,
      };
    }
    const discovery = discover(ctx);
    const command = cleanString(discovery.command || selectedCommand || options.commandFallback || engineId, 500);
    const runner = typeof ctx.onActivity === 'function' ? spawnActivityCapture : spawnCapture;
    const turnTimeoutMs = resolveTurnTimeoutMs(ctx, timeoutMs);
    const run = await runner(command, runArgs(prompt, ctx), {
      timeoutMs: turnTimeoutMs,
      maxOutputBytes: 64000,
      cwd: options.cwd || (ctx && ctx.message && (ctx.message.cwd || ctx.message.workspace_dir)) || process.cwd(),
      ctx,
      engineId,
      onActivity: ctx.onActivity,
    });
    const parsed = parseCliActivityOutput(run.stdout, run.stderr, ctx, engineId);
    const failureText = run.ok ? '' : cliRuntimeFailureText(engineId, run, turnTimeoutMs);
    const outputText = parsed.output_text || failureText;
    const errorCode = run.ok ? '' : classifyCliRuntimeFailureCode(engineId, run, failureText);
    return {
      ...baseEvent(ctx, 'turn.complete', engineId),
      status: run.ok ? 'completed' : run.timed_out ? 'timed_out' : 'failed',
      error_code: errorCode,
      reason: failureText,
      retryable: run.timed_out === true,
      result_ref: stableRef(`artifact/${engineId}/result`, ctx, engineId),
      receipt_ref: stableRef(`receipt/${engineId}/turn`, ctx, engineId),
      output_text: outputText,
      output_preview: cleanString(outputText || parsed.output_preview, 4000),
      activity_events: appendCliRuntimeFailureEvent(parsed.activity_events, ctx, engineId, run, turnTimeoutMs),
      activity_event_count: parsed.activity_event_count,
      structured_activity: parsed.structured_activity,
      permission_denials: parsed.permission_denials,
      permission_request: parsed.permission_request,
      exit_code: run.exit_code,
      timed_out: run.timed_out === true,
      timeout_ms: turnTimeoutMs,
      stderr_preview: cleanString(run.stderr, 2000),
    };
  }

  return {
    async health_check(ctx) {
      const discovery = discover(ctx);
      const command = cleanString(discovery.command || selectedCommand, 500);
      const probe = discovery.status === 'available'
      ? await spawnCapture(command, versionArgs, { timeoutMs: 5000, maxOutputBytes: 4096, cwd: options.cwd || process.cwd() })
        : { ok: false, stdout: '', stderr: discovery.reason || discovery.status };
      return {
        ...baseEvent(ctx, 'engine.health.result', engineId),
        status: probe.ok ? 'available' : discovery.status || 'not_downloaded',
        engine_kind: engineKind,
        command,
        discovery_source: discovery.discovery_source,
        custom_location_allowed: discovery.custom_location_allowed,
        resolved_path: discovery.resolved_path || null,
        download_available: Boolean(discovery.download_available || downloadActionRef),
        download_action_ref: downloadActionRef,
        context_transport_mode: contextTransportMode,
        structured_transport_target: structuredTransportTarget,
        transport_migration_status: transportMigrationStatus,
        supports_live_steering: false,
        supports_next_turn_steering: true,
        steering_transport: 'gateway_next_turn_intervention',
        version_preview: cleanString(probe.stdout || probe.stderr, 500),
      };
    },

    async start_session(ctx) {
      return {
        ...baseEvent(ctx, 'session.started', engineId),
        status: 'started',
        receipt_ref: stableRef(`receipt/${engineId}/session`, ctx, engineId),
      };
    },

    async submit_turn(ctx) {
      return submitTurnWithOptionalStream(ctx);
    },

    async stream_turn(ctx) {
      return submitTurnWithOptionalStream(ctx);
    },

    async stream_events(ctx) {
      return { ...baseEvent(ctx, 'heartbeat', engineId), status: 'adapter_ready' };
    },

    async cancel_turn(ctx) {
      return {
        ...baseEvent(ctx, 'turn.cancelled', engineId),
        status: 'cancel_requested',
        receipt_ref: stableRef(`receipt/${engineId}/cancel`, ctx, engineId),
      };
    },

    async collect_artifacts(ctx) {
      return {
        ...baseEvent(ctx, 'artifact.created', engineId),
        artifact_ref: stableRef(`artifact/${engineId}/result`, ctx, engineId),
        artifact_kind: artifactKind,
      };
    },

    async emit_receipts(ctx) {
      return {
        ...baseEvent(ctx, 'receipt.created', engineId),
        receipt_ref: stableRef(`receipt/${engineId}/turn`, ctx, engineId),
        receipt_kind: receiptKind,
      };
    },
  };
}

module.exports = {
  createCliRuntimeEngineAdapter,
  spawnCapture,
  spawnActivityCapture,
  stripTerminalControls,
  buildPromptWithContext,
  parseCliActivityOutput,
  stableExternalSessionUuid,
};
