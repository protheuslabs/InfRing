#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::codex-cli.
//
// Bounded first external engine adapter. Live process execution is opt-in through
// options.liveDispatch so the Gateway can prove adapter shape and health without
// accidentally launching a coding agent from a passive registry load.

'use strict';

const childProcess = require('child_process');
const { resolveEngineDiscovery } = require('./discovery.ts');
const { buildPromptWithContext } = require('./cli_runtime_adapter.ts');

function cleanString(value, max = 2000) {
  return stripTerminalControls(value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}

function cleanDisplayString(value, max = 24000) {
  return stripTerminalControls(value)
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{5,}/g, '\n\n\n\n')
    .trim()
    .slice(0, max);
}

function baseEvent(ctx, type) {
  const message = (ctx && ctx.message) || {};
  const engine = (ctx && ctx.engine) || {};
  return {
    type,
    trace_id: cleanString(message.trace_id, 200),
    request_id: cleanString(message.request_id, 200),
    engine_id: cleanString(message.engine_id || engine.engine_id || 'codex_cli', 120),
    session_id: cleanString(message.session_id, 200),
    turn_id: cleanString(message.turn_id, 200),
  };
}

function stableRef(prefix, ctx) {
  const event = baseEvent(ctx, 'ref');
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
  let current = '';
  if (typeof input === 'string') current = cleanString(input, 12000);
  if (input && typeof input === 'object') current = cleanString(input.text || input.message || input.prompt || '', 12000);
  return buildPromptWithContext(ctx && ctx.message && ctx.message.context_pack, current);
}

function createCodexCliEngineAdapter(options = {}) {
  const liveDispatch = options.liveDispatch === true || process.env.INFRING_AGENT_RUNTIME_CODEX_LIVE === '1';
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 60000, 300000));
  let selectedCommand = cleanString(options.command || process.env.INFRING_CODEX_CLI_BIN || process.env.INFRING_CODEX_CLI_PATH || 'codex', 500);

  function discover(ctx) {
    const engine = (ctx && ctx.engine) || { engine_id: 'codex_cli' };
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
      const command = cleanString(discovery.command || selectedCommand || 'codex', 500);
      const probe = discovery.status === 'available'
        ? await spawnCapture(command, ['--version'], { timeoutMs: 5000, maxOutputBytes: 4096 })
        : { ok: false, stdout: '', stderr: discovery.reason || discovery.status };
      return {
        ...baseEvent(ctx, 'engine.health.result'),
        status: probe.ok ? 'available' : discovery.status || 'not_downloaded',
        engine_kind: 'external_cli_adapter',
        command,
        discovery_source: discovery.discovery_source,
        custom_location_allowed: discovery.custom_location_allowed,
        resolved_path: discovery.resolved_path || null,
        download_available: true,
        download_action_ref: 'agent_runtime_download/codex_cli',
        version_preview: cleanString(probe.stdout || probe.stderr, 500),
      };
    },

    async start_session(ctx) {
      return {
        ...baseEvent(ctx, 'session.started'),
        status: 'started',
        receipt_ref: stableRef('receipt/codex-cli/session', ctx),
      };
    },

    async submit_turn(ctx) {
      const prompt = extractPrompt(ctx);
      if (!prompt) {
        return {
          ...baseEvent(ctx, 'error'),
          error_code: 'codex_cli_prompt_missing',
          reason: 'Codex CLI adapter requires input.text, input.message, input.prompt, or string input.',
          retryable: false,
        };
      }
      if (!liveDispatch) {
        return {
          ...baseEvent(ctx, 'error'),
          error_code: 'codex_cli_live_dispatch_disabled',
          reason: 'Codex CLI adapter is installed as a bounded external runtime seam; live dispatch requires INFRING_AGENT_RUNTIME_CODEX_LIVE=1 or adapter option liveDispatch=true.',
          retryable: false,
        };
      }
      const discovery = discover(ctx);
      const command = cleanString(discovery.command || selectedCommand || 'codex', 500);
      const run = await spawnCapture(command, ['exec', '--sandbox', 'read-only', '--ignore-rules', '--ephemeral', '--color', 'never', prompt], { timeoutMs, maxOutputBytes: 64000 });
      const outputText = cleanDisplayString(run.stdout || run.stderr, 64000);
      return {
        ...baseEvent(ctx, 'turn.complete'),
        status: run.ok ? 'completed' : 'failed',
        result_ref: stableRef('artifact/codex-cli/result', ctx),
        receipt_ref: stableRef('receipt/codex-cli/turn', ctx),
        output_text: outputText,
        output_preview: cleanString(outputText, 4000),
        exit_code: run.exit_code,
      };
    },

    async stream_events(ctx) {
      return {
        ...baseEvent(ctx, 'heartbeat'),
        status: 'adapter_ready',
      };
    },

    async cancel_turn(ctx) {
      return {
        ...baseEvent(ctx, 'turn.cancelled'),
        status: 'cancel_requested',
        receipt_ref: stableRef('receipt/codex-cli/cancel', ctx),
      };
    },

    async collect_artifacts(ctx) {
      return {
        ...baseEvent(ctx, 'artifact.created'),
        artifact_ref: stableRef('artifact/codex-cli/result', ctx),
        artifact_kind: 'codex_cli_result_projection',
      };
    },

    async emit_receipts(ctx) {
      return {
        ...baseEvent(ctx, 'receipt.created'),
        receipt_ref: stableRef('receipt/codex-cli/turn', ctx),
        receipt_kind: 'external_cli_adapter_receipt',
      };
    },
  };
}

module.exports = {
  createCodexCliEngineAdapter,
  spawnCapture,
  stripTerminalControls,
  extractPrompt,
};
