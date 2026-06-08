#!/usr/bin/env tsx

'use strict';

const childProcess = require('child_process');
const fs = require('fs');
const path = require('path');
const { createHttpSocketRuntimeEngineAdapter } = require('./http_socket_runtime_adapter.ts');
const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

const FORBIDDEN_SAFE_BRIDGE_TOOLSETS = new Set([
  '*',
  'all',
  'terminal',
  'file',
  'debugging',
  'hermes-cli',
  'hermes-acp',
  'hermes-telegram',
  'hermes-discord',
  'hermes-whatsapp',
  'hermes-slack',
]);

function cleanToolsetList(value) {
  const tokens = String(value == null ? '' : value)
    .split(',')
    .map((token) => token.trim().toLowerCase())
    .filter(Boolean);
  if (!tokens.length) return 'safe';
  if (tokens.some((token) => FORBIDDEN_SAFE_BRIDGE_TOOLSETS.has(token))) return 'safe';
  return tokens.join(',');
}

function cleanText(value, max = 4000) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
    .replace(/\r\n/g, '\n')
    .trim()
    .slice(0, max);
}

function spawnHermesStatus(command, cwd) {
  return new Promise((resolve) => {
    const child = childProcess.spawn(command, ['status'], {
      cwd: cwd || process.cwd(),
      env: process.env,
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    const append = (current, chunk) => {
      const next = Buffer.concat([current, Buffer.from(chunk || '')]);
      return next.length > 24000 ? next.subarray(next.length - 24000) : next;
    };
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      try { child.kill('SIGTERM'); } catch {}
      resolve({ ok: false, timed_out: true, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    }, 8000);
    child.stdout.on('data', (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
    child.on('error', (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: false, error: cleanText(error && error.message, 500), stdout: '', stderr: '' });
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: code === 0, exit_code: code, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    });
  });
}

function classifyHermesProviderReadiness(statusProbe) {
  const text = cleanText(`${statusProbe && statusProbe.stdout || ''}\n${statusProbe && statusProbe.stderr || ''}`, 24000);
  const lower = text.toLowerCase();
  if (!statusProbe || statusProbe.timed_out) {
    return {
      provider_status: 'unknown',
      provider_reason: 'hermes_status_probe_timeout',
      status_preview: text,
    };
  }
  if (!statusProbe.ok) {
    return {
      provider_status: 'unknown',
      provider_reason: statusProbe.error || 'hermes_status_probe_failed',
      status_preview: text,
    };
  }
  if (
    lower.includes('model: (not set)') ||
    lower.includes('.env file: ✗ not found') ||
    lower.includes('openrouter ✗ (not set)') ||
    lower.includes('openai ✗ (not set)') ||
    lower.includes('anthropic ✗ (not set)') ||
    lower.includes('not configured') ||
    lower.includes('not logged in')
  ) {
    return {
      provider_status: 'auth_required',
      provider_reason: 'No Hermes inference provider/model/API credential is configured. Run `hermes model`, `hermes setup`, or configure ~/.hermes/.env.',
      status_preview: text,
    };
  }
  return {
    provider_status: 'configured',
    provider_reason: '',
    status_preview: text,
  };
}

function referenceCheckoutHermesCommand() {
  const candidate = path.resolve(process.cwd(), 'references/coding-agent-systems/hermes-agent/hermes');
  try {
    return fs.existsSync(candidate) ? candidate : '';
  } catch {
    return '';
  }
}

function hermesCommandOverride(options = {}) {
  return options.command ||
    process.env.INFRING_HERMES_AGENT_COMMAND ||
    referenceCheckoutHermesCommand();
}

function createHermesAgentEngineAdapter(options = {}) {
  const httpAdapter = createHttpSocketRuntimeEngineAdapter({
    ...options,
    engineId: 'hermes_agent',
    engineKind: 'external_framework_adapter',
    downloadActionRef: 'agent_runtime_download/hermes_agent',
    contextTransportMode: 'structured_json',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'planned_structured_target',
    healthProbes: [
      { method: 'GET', path: '/api/status' },
      { method: 'GET', path: '/api/auth/me', allow_unauthorized: true },
      { method: 'GET', path: '/api/sessions?limit=1&offset=0' },
      { method: 'GET', path: '/api/config/schema' },
    ],
  });
  const safeCliAdapter = createCliRuntimeEngineAdapter({
    ...options,
    engineId: 'hermes_agent',
    engineKind: 'external_framework_cli_safe_adapter',
    command: hermesCommandOverride(options),
    commandFallback: 'hermes',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_HERMES_AGENT_LIVE',
    downloadActionRef: 'agent_runtime_download/hermes_agent',
    artifactKind: 'hermes_agent_result_projection',
    receiptKind: 'hermes_agent_adapter_receipt',
    contextTransportMode: 'prompt_text_compat',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'transitional_safe_oneshot_bridge',
    versionArgs: ['--version'],
    runArgs: (prompt, ctx) => [
      '--toolsets',
      cleanToolsetList(options.safeToolsets || process.env.INFRING_HERMES_AGENT_SAFE_TOOLSETS || 'safe'),
      '-z',
      prompt,
    ],
  });

  return {
    async health_check(ctx) {
      const [httpHealth, cliHealth] = await Promise.all([
        httpAdapter.health_check(ctx),
        safeCliAdapter.health_check(ctx),
      ]);
      if (cliHealth && cliHealth.status === 'available') {
        const statusProbe = await spawnHermesStatus(cliHealth.command || options.command || 'hermes', options.cwd);
        const providerReadiness = classifyHermesProviderReadiness(statusProbe);
        return {
          ...cliHealth,
          status: providerReadiness.provider_status === 'auth_required' ? 'auth_required' : cliHealth.status,
          bridge_mode: 'cli_safe_oneshot',
          safe_toolsets: cleanToolsetList(options.safeToolsets || process.env.INFRING_HERMES_AGENT_SAFE_TOOLSETS || 'safe'),
          provider_readiness: providerReadiness.provider_status === 'auth_required' ? 'blocked' : providerReadiness.provider_status,
          provider_status: providerReadiness.provider_status,
          provider_reason: providerReadiness.provider_reason,
          status_reason: providerReadiness.provider_reason,
          reason: providerReadiness.provider_reason,
          error_code: providerReadiness.provider_status === 'auth_required' ? 'hermes_agent_provider_auth_required' : '',
          setup_action_ref: providerReadiness.provider_status === 'auth_required' ? 'agent_runtime_setup/hermes_agent_provider' : '',
          mutating_tool_bridge_ready: false,
          mutating_tool_bridge_reason: 'Hermes oneshot auto-bypasses native approvals, so InfRing only enables non-mutating safe toolsets until a mediated approval bridge exists.',
          dashboard_health_status: httpHealth && httpHealth.status,
          dashboard_url: httpHealth && httpHealth.url,
          dashboard_health_path: httpHealth && httpHealth.health_path,
        };
      }
      if (httpHealth && httpHealth.status === 'available') {
        return {
          ...httpHealth,
          status: 'health_only',
          bridge_mode: 'dashboard_health_only',
          reason: 'Hermes dashboard/API is reachable, but the safe CLI turn bridge is unavailable; live turn submission is not selectable yet.',
          cli_health_status: cliHealth && cliHealth.status,
          mutating_tool_bridge_ready: false,
        };
      }
      return {
        ...(httpHealth || cliHealth),
        cli_health_status: cliHealth && cliHealth.status,
        dashboard_health_status: httpHealth && httpHealth.status,
        bridge_mode: 'unavailable',
        mutating_tool_bridge_ready: false,
      };
    },
    start_session: safeCliAdapter.start_session,
    submit_turn: safeCliAdapter.submit_turn,
    stream_turn: safeCliAdapter.stream_turn,
    stream_events: safeCliAdapter.stream_events,
    cancel_turn: safeCliAdapter.cancel_turn,
    collect_artifacts: safeCliAdapter.collect_artifacts,
    emit_receipts: safeCliAdapter.emit_receipts,
  };
}

module.exports = {
  createHermesAgentEngineAdapter,
};
