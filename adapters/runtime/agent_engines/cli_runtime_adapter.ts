#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::cli-runtime-adapter.
//
// Shared bounded adapter for CLI-based external agent runtimes. It keeps live
// dispatch opt-in and uses Gateway-owned discovery so Shell never probes local
// runtime installs directly.

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
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 15000, 120000));
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
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 15000, 120000));
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
      if (onActivity) {
        try { onActivity(normalizeCliActivityEvent(parsed, activityIndex, ctx, engineId)); } catch {}
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
    const text = cleanDisplayString(payload.text_preview || payload.summary || payload.source_ref || '', 900);
    if (!text) return '';
    return `- atom ${cleanString(row.ref_id, 80)} [${role}] ${text}`;
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
  if (role === 'human') return 'user';
  if (role === 'agent' || role === 'ai' || role === 'assistant') return 'assistant';
  if (role === 'tool' || role === 'tool_result_bundle') return 'tool';
  if (role === 'system' || role === 'status_summary') return 'system';
  return role || 'message';
}

function formatConversationTranscriptLine(fragment) {
  const row = fragment && typeof fragment === 'object' ? fragment : {};
  const payload = row.payload && typeof row.payload === 'object' ? row.payload : {};
  const role = contextTranscriptRole(row);
  const text = cleanDisplayString(payload.text_preview || payload.summary || row.text_preview || row.summary || '', 1600);
  if (!text) return '';
  return `${role}: ${text}`;
}

function buildPromptWithContext(contextPack, currentPrompt) {
  const current = cleanDisplayString(currentPrompt || '', 12000);
  if (!current) return '';
  const pack = contextPack && typeof contextPack === 'object' ? contextPack : null;
  const fragments = pack && Array.isArray(pack.fragments) ? pack.fragments.slice() : [];
  const toolGrantSection = renderUniversalToolGrantPromptSection(pack && pack.universal_tool_grants);
  if (!pack) return current;
  if (fragments.length === 0 && !toolGrantSection) return current;
  const hot = fragments
    .filter((row) => row && row.kind === 'atom')
    .sort((a, b) => fragmentSortValue(a) - fragmentSortValue(b))
    .slice(-12);
  const hotContextFragments = hot
    .map(formatContextFragment)
    .filter(Boolean)
    .slice(-6);
  const conversationTranscript = hot
    .map(formatConversationTranscriptLine)
    .filter(Boolean)
    .slice(-12);
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
    `- session_id: ${cleanString(pack.session_id, 120)}`,
    `- fanout_target: ${cleanString(pack.fanout_target || 7, 20)}`,
    '- policy: The Recent conversation transcript below is visible prior chat context from this InfRing session. Treat it as the conversation history you can see. Do not claim you have no prior messages when transcript rows are present. If the user says "try again", infer the retry target from that transcript when possible.',
  ];
  if (conversationTranscript.length) lines.push('', 'Recent conversation transcript:', ...conversationTranscript);
  if (spans.length) lines.push('', 'Selected context spans:', ...spans);
  if (hotContextFragments.length) lines.push('', 'Recent context atoms:', ...hotContextFragments);
  if (toolGrantSection) lines.push('', toolGrantSection);
  lines.push('', 'Current user turn:', current);
  return cleanDisplayString(lines.join('\n'), 24000);
}

function parseJsonlRows(raw) {
  return String(raw == null ? '' : raw)
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      try { return JSON.parse(line); } catch { return null; }
    })
    .filter((row) => row && typeof row === 'object');
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

function extractTextFromContent(value, maxLen = 4000) {
  if (value == null) return '';
  if (typeof value === 'string') return cleanDisplayString(value, maxLen);
  if (Array.isArray(value)) {
    return cleanDisplayString(value.map((item) => extractTextFromContent(item, maxLen)).filter(Boolean).join(''), maxLen);
  }
  if (typeof value !== 'object') return cleanDisplayString(value, maxLen);
  if (value.content != null) {
    const contentText = extractTextFromContent(value.content, maxLen);
    if (contentText) return contentText;
  }
  return cleanDisplayString(
    value.text ||
      value.delta ||
      value.message ||
      value.summary ||
      value.output_text ||
      value.output ||
      value.result ||
      value.data ||
      value.transcript ||
      '',
    maxLen,
  );
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

function outputTextFromActivityRows(rows, fallbackText) {
  let finalText = '';
  const assistantParts = [];
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
  }
  return cleanDisplayString(finalText || assistantParts.join('') || fallbackText || '', 64000);
}

function parseCliActivityOutput(stdout, stderr, ctx, defaultEngineId) {
  const rows = parseJsonlRows(stdout);
  if (!rows.length) {
    const output = cleanDisplayString(stdout || stderr, 64000);
    return {
      output_text: output,
      output_preview: cleanString(output, 4000),
      activity_events: [],
      activity_event_count: 0,
      structured_activity: false,
    };
  }
  const events = rows
    .map((row, index) => normalizeCliActivityEvent(row, index, ctx, defaultEngineId))
    .filter((event) => event && (event.text || event.provider_event_type))
    .slice(-120);
  const output = outputTextFromActivityRows(rows, stderr);
  return {
    output_text: output,
    output_preview: cleanString(output, 4000),
    activity_events: events,
    activity_event_count: rows.length,
    structured_activity: true,
  };
}

function createCliRuntimeEngineAdapter(options = {}) {
  const engineId = cleanString(options.engineId || 'external_cli', 120);
  const engineKind = cleanString(options.engineKind || 'external_cli_adapter', 120);
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
    const run = await runner(command, runArgs(prompt, ctx), {
      timeoutMs,
      maxOutputBytes: 64000,
      ctx,
      engineId,
      onActivity: ctx.onActivity,
    });
    const parsed = parseCliActivityOutput(run.stdout, run.stderr, ctx, engineId);
    const outputText = parsed.output_text;
    return {
      ...baseEvent(ctx, 'turn.complete', engineId),
      status: run.ok ? 'completed' : 'failed',
      result_ref: stableRef(`artifact/${engineId}/result`, ctx, engineId),
      receipt_ref: stableRef(`receipt/${engineId}/turn`, ctx, engineId),
      output_text: outputText,
      output_preview: parsed.output_preview,
      activity_events: parsed.activity_events,
      activity_event_count: parsed.activity_event_count,
      structured_activity: parsed.structured_activity,
      exit_code: run.exit_code,
    };
  }

  return {
    async health_check(ctx) {
      const discovery = discover(ctx);
      const command = cleanString(discovery.command || selectedCommand, 500);
      const probe = discovery.status === 'available'
        ? await spawnCapture(command, versionArgs, { timeoutMs: 5000, maxOutputBytes: 4096 })
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
