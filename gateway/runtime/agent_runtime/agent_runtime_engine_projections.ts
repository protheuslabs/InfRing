#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::engine-projections.
//
// Gateway owns external-runtime engine menu/install projections. Provider
// adapters remain translators and are injected here for health checks; they do
// not own the Shell-facing engine selection/install route shape.

'use strict';

const { spawn } = require('node:child_process');

function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function parsePositiveInt(value, fallback, min = 1, max = 65535) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}

function agentRuntimeInstallPlatformAliases() {
  const platform = process.platform;
  const aliases = new Set(['all', platform]);
  if (platform === 'darwin') aliases.add('macos');
  if (platform === 'win32') aliases.add('windows');
  return aliases;
}

function agentRuntimeInstallAllowed() {
  const raw = cleanText(process.env.INFRING_AGENT_RUNTIME_INSTALL_ALLOWED, 40).toLowerCase();
  return raw !== '0' && raw !== 'false' && raw !== 'no' && raw !== 'deny';
}

function selectAgentRuntimeInstallCommand(install) {
  const spec = install && install.command_line_install && typeof install.command_line_install === 'object'
    ? install.command_line_install
    : {};
  const commands = Array.isArray(spec.commands) ? spec.commands : [];
  const aliases = agentRuntimeInstallPlatformAliases();
  for (const candidate of commands) {
    const row = candidate && typeof candidate === 'object' ? candidate : {};
    const platforms = Array.isArray(row.platforms) && row.platforms.length ? row.platforms : ['all'];
    if (platforms.some((item) => aliases.has(cleanText(item, 40).toLowerCase()))) return row;
  }
  return null;
}

function findAgentRuntimeEngine(registryInfo, engineId) {
  const target = cleanEngineId(engineId);
  const engines = Array.isArray(registryInfo && registryInfo.engines) ? registryInfo.engines : [];
  return engines.find((engine) => cleanEngineId(engine && engine.engine_id) === target) || null;
}

function createAgentRuntimeEngineProjectionStore(options = {}) {
  const root = options.root || process.cwd();
  const loadRegistry = typeof options.loadRegistry === 'function'
    ? options.loadRegistry
    : () => ({ engines: [] });
  const createAdapterMap = typeof options.createAdapterMap === 'function'
    ? options.createAdapterMap
    : () => ({});
  const loadSelection = typeof options.loadSelection === 'function'
    ? options.loadSelection
    : () => ({ engine_id: 'infring_native', updated_at: '' });

  function captureAgentRuntimeInstall(commandSpec) {
    const row = commandSpec && typeof commandSpec === 'object' ? commandSpec : {};
    const shellCommand = cleanDisplayText(row.shell_command || '', 4000);
    const command = shellCommand
      ? (process.platform === 'win32' ? 'powershell.exe' : '/bin/sh')
      : cleanText(row.command || '', 500);
    const args = shellCommand
      ? (process.platform === 'win32' ? ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', shellCommand] : ['-lc', shellCommand])
      : (Array.isArray(row.args) ? row.args.map((item) => cleanText(item, 1000)) : []);
    if (!command) return Promise.resolve({ ok: false, exit_code: null, stdout: '', stderr: 'agent_runtime_install_command_missing' });
    const timeoutMs = parsePositiveInt(row.timeout_ms, 240000, 5000, 600000);
    const maxOutputBytes = parsePositiveInt(row.max_output_bytes, 24000, 1024, 65536);
    return new Promise((resolve) => {
      const child = spawn(command, args, {
        cwd: root,
        env: { ...process.env },
        shell: false,
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      let stdout = Buffer.alloc(0);
      let stderr = Buffer.alloc(0);
      let settled = false;
      const append = (current, chunk) => {
        const next = Buffer.concat([current, Buffer.from(chunk || '')]);
        return next.length > maxOutputBytes ? next.subarray(next.length - maxOutputBytes) : next;
      };
      const timer = setTimeout(() => {
        if (settled) return;
        settled = true;
        try { child.kill('SIGTERM'); } catch {}
        resolve({ ok: false, timed_out: true, exit_code: null, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
      }, timeoutMs);
      child.stdout.on('data', (chunk) => { stdout = append(stdout, chunk); });
      child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
      child.on('error', (error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve({ ok: false, timed_out: false, exit_code: null, stdout: '', stderr: cleanText(error && error.message ? error.message : error, 2000) });
      });
      child.on('close', (code) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve({ ok: code === 0, timed_out: false, exit_code: code, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
      });
    });
  }

  function projectAgentRuntimeEngineRow(engine, health) {
    const row = engine && typeof engine === 'object' ? engine : {};
    const install = row.install && typeof row.install === 'object' ? row.install : {};
    const engineId = cleanEngineId(row.engine_id);
    const healthStatus = cleanText(health && health.status ? health.status : '', 80);
    const registryStatus = cleanText(row.status || '', 80);
    const nativeReady = engineId === 'infring_native' && registryStatus === 'adapter_seam_ready';
    const status = healthStatus || (nativeReady ? 'available' : (registryStatus || 'unknown'));
    const selectable = status === 'available' || status === 'adapter_ready' || nativeReady;
    const downloadAvailable = install.download_available === true || (health && health.download_available === true);
    const commandLineInstall = selectAgentRuntimeInstallCommand(install);
    const installActionAvailable = cleanText(install.preferred_install_method || '', 80) === 'command_line' && !!commandLineInstall;
    const registryCapabilities = Array.isArray(row.capabilities) ? row.capabilities.map((item) => cleanText(item, 120)).filter(Boolean).slice(0, 12) : [];
    const supportsLiveSteering = (health && health.supports_live_steering === true) || row.supports_live_steering === true;
    const supportsNextTurnSteering = (health && health.supports_next_turn_steering === true) || row.supports_next_turn_steering === true || engineId !== 'infring_native';
    const steeringMode = supportsLiveSteering ? 'live' : supportsNextTurnSteering ? 'next_turn' : 'unsupported';
    const capabilities = registryCapabilities.slice();
    if (supportsLiveSteering && !capabilities.includes('live_steering')) capabilities.push('live_steering');
    if (supportsNextTurnSteering && !capabilities.includes('next_turn_steering')) capabilities.push('next_turn_steering');
    return {
      engine_id: engineId,
      display_name: cleanText(row.display_name || engineId, 120),
      engine_kind: cleanText(row.engine_kind || '', 120),
      transport_kind: cleanText(row.transport_kind || '', 120),
      status,
      selectable,
      capabilities: capabilities.slice(0, 14),
      supports_live_steering: supportsLiveSteering,
      supports_next_turn_steering: supportsNextTurnSteering,
      steering_mode: steeringMode,
      steering_transport: cleanText((health && health.steering_transport) || (steeringMode === 'next_turn' ? 'gateway_next_turn_intervention' : steeringMode), 120),
      download_available: !!downloadAvailable,
      install_action_available: !!installActionAvailable,
      command_line_install_available: !!commandLineInstall,
      install_permission_state: agentRuntimeInstallAllowed() ? 'allowed' : 'permission_required',
      download_action_ref: cleanText(install.download_action_ref || (health && health.download_action_ref) || '', 240),
      preferred_install_method: cleanText(install.preferred_install_method || '', 80),
      command_line_hint: cleanText(install.command_line_hint || '', 500),
      browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
      display_when_missing: cleanText(install.display_when_missing || (downloadAvailable ? 'download_icon' : ''), 80),
      version_preview: cleanText(health && health.version_preview ? health.version_preview : '', 240),
    };
  }

  async function agentRuntimeEnginesProjection(traceId) {
    const info = loadRegistry(root);
    const engines = Array.isArray(info.engines) ? info.engines : [];
    const engineAdapters = createAdapterMap({ liveDispatch: false });
    const selection = loadSelection();
    const rows = [];
    for (const engine of engines) {
      const engineId = cleanEngineId(engine && engine.engine_id);
      let health = null;
      if (engineAdapters[engineId] && typeof engineAdapters[engineId].health_check === 'function') {
        health = await engineAdapters[engineId].health_check({
          message: {
            trace_id: traceId,
            request_id: `agent-runtime-menu:${Date.now()}`,
            engine_id: engineId,
            session_id: 'dashboard-menu',
          },
          engine,
        }).catch((error) => ({
          status: 'not_downloaded',
          download_available: true,
          reason: cleanText(error && error.message ? error.message : error, 200),
        }));
      }
      rows.push(projectAgentRuntimeEngineRow(engine, health));
    }
    return {
      ok: true,
      type: 'agent_runtime_engines_projection',
      trace_id: traceId,
      socket_route: '/ws/agent-runtime',
      selected_default_engine_id: selection.engine_id || 'infring_native',
      active_engine_id: selection.engine_id || 'infring_native',
      active_engine_updated_at: selection.updated_at || '',
      engines: rows,
    };
  }

  async function agentRuntimeEngineInstallProjection(traceId, requestedEngineId) {
    const engineId = cleanEngineId(requestedEngineId);
    if (!engineId) return { ok: false, status_code: 400, type: 'agent_runtime_engine_install_projection', trace_id: traceId, status: 'engine_id_required' };
    const info = loadRegistry(root);
    const engine = findAgentRuntimeEngine(info, engineId);
    if (!engine) return { ok: false, status_code: 404, type: 'agent_runtime_engine_install_projection', trace_id: traceId, engine_id: engineId, status: 'engine_not_registered' };
    const install = engine.install && typeof engine.install === 'object' ? engine.install : {};
    const adapter = createAdapterMap({ liveDispatch: false })[engineId];
    const healthMessage = { trace_id: traceId, request_id: `agent-runtime-install:${Date.now()}`, engine_id: engineId, session_id: 'dashboard-install' };
    const beforeHealth = adapter && typeof adapter.health_check === 'function'
      ? await adapter.health_check({ message: healthMessage, engine }).catch((error) => ({ status: 'health_check_failed', reason: cleanText(error && error.message ? error.message : error, 240) }))
      : null;
    if (beforeHealth && beforeHealth.status === 'available') {
      return {
        ok: true,
        type: 'agent_runtime_engine_install_projection',
        trace_id: traceId,
        engine_id: engineId,
        status: 'already_available',
        health: {
          status: 'available',
          discovery_source: cleanText(beforeHealth.discovery_source, 120),
          version_preview: cleanText(beforeHealth.version_preview, 240),
        },
      };
    }
    if (cleanText(install.preferred_install_method || '', 80) !== 'command_line') {
      return {
        ok: false,
        status_code: 409,
        type: 'agent_runtime_engine_install_projection',
        trace_id: traceId,
        engine_id: engineId,
        status: 'no_command_line_installer',
        browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
        command_line_hint: cleanText(install.command_line_hint || '', 500),
      };
    }
    const commandSpec = selectAgentRuntimeInstallCommand(install);
    if (!commandSpec) {
      return {
        ok: false,
        status_code: 409,
        type: 'agent_runtime_engine_install_projection',
        trace_id: traceId,
        engine_id: engineId,
        status: 'command_line_installer_unavailable_for_platform',
        browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
        command_line_hint: cleanText(install.command_line_hint || '', 500),
      };
    }
    if (!agentRuntimeInstallAllowed()) {
      return {
        ok: false,
        status_code: 403,
        type: 'agent_runtime_engine_install_projection',
        trace_id: traceId,
        engine_id: engineId,
        status: 'permission_required',
        command_line_hint: cleanText(install.command_line_hint || '', 500),
      };
    }
    const run = await captureAgentRuntimeInstall(commandSpec);
    const afterHealth = adapter && typeof adapter.health_check === 'function'
      ? await adapter.health_check({ message: healthMessage, engine }).catch((error) => ({ status: 'health_check_failed', reason: cleanText(error && error.message ? error.message : error, 240) }))
      : null;
    const available = afterHealth && afterHealth.status === 'available';
    return {
      ok: !!(run.ok && available),
      status_code: run.ok && available ? 200 : 502,
      type: 'agent_runtime_engine_install_projection',
      trace_id: traceId,
      engine_id: engineId,
      status: available ? 'installed_available' : (run.ok ? 'install_completed_but_not_available' : 'install_failed'),
      exit_code: run.exit_code,
      timed_out: !!run.timed_out,
      stdout_preview: cleanDisplayText(run.stdout, 2000),
      stderr_preview: cleanDisplayText(run.stderr, 2000),
      health: afterHealth ? {
        status: cleanText(afterHealth.status, 80),
        discovery_source: cleanText(afterHealth.discovery_source, 120),
        version_preview: cleanText(afterHealth.version_preview, 240),
        reason: cleanText(afterHealth.reason, 240),
      } : null,
    };
  }

  return {
    projectAgentRuntimeEngineRow,
    agentRuntimeEnginesProjection,
    agentRuntimeEngineInstallProjection,
  };
}

module.exports = {
  createAgentRuntimeEngineProjectionStore,
};
