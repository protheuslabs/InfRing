#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::cli-runtime-adapter.
//
// Shared bounded adapter for CLI-based external agent runtimes. It keeps live
// dispatch opt-in and uses Gateway-owned discovery so Shell never probes local
// runtime installs directly.

'use strict';

const childProcess = require('child_process');
const { resolveEngineDiscovery } = require('./discovery.ts');

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

function extractPrompt(ctx) {
  const input = ctx && ctx.message && ctx.message.input;
  if (typeof input === 'string') return cleanString(input, 12000);
  if (input && typeof input === 'object') return cleanString(input.text || input.message || input.prompt || '', 12000);
  return '';
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
      const run = await spawnCapture(command, runArgs(prompt), { timeoutMs, maxOutputBytes: 64000 });
      const outputText = cleanDisplayString(run.stdout || run.stderr, 64000);
      return {
        ...baseEvent(ctx, 'turn.complete', engineId),
        status: run.ok ? 'completed' : 'failed',
        result_ref: stableRef(`artifact/${engineId}/result`, ctx, engineId),
        receipt_ref: stableRef(`receipt/${engineId}/turn`, ctx, engineId),
        output_text: outputText,
        output_preview: cleanString(outputText, 4000),
        exit_code: run.exit_code,
      };
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
  stripTerminalControls,
};
