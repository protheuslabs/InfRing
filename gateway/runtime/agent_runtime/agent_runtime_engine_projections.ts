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

function classifyAgentRuntimeModelCapability(model, menuSource) {
  const row = model && typeof model === 'object' ? model : {};
  const provider = cleanText(row.provider || row.model_provider || row.provider_id || row.source_provider || menuSource || '', 120);
  const providerKey = provider.toLowerCase();
  const deploymentSource = cleanText(row.deployment_kind || row.deployment || row.hosting || row.availability || row.status || '', 120);
  const deploymentText = [
    deploymentSource,
    row.source || '',
    row.availability || '',
    row.status || '',
  ].join(' ').toLowerCase();
  const localProvider = providerKey === 'ollama' ||
    providerKey === 'llama.cpp' ||
    providerKey === 'llamacpp' ||
    providerKey.includes('local');
  const cloudProvider = providerKey.includes('openai') ||
    providerKey.includes('anthropic') ||
    providerKey.includes('claude') ||
    providerKey.includes('codex') ||
    providerKey.includes('grok') ||
    providerKey.includes('xai') ||
    providerKey.includes('google') ||
    providerKey.includes('gemini') ||
    providerKey.includes('mistral') ||
    providerKey.includes('openrouter') ||
    providerKey.includes('perplexity');
  const cloud = row.cloud === true ||
    row.api_backed === true ||
    row.requires_auth === true ||
    cloudProvider ||
    deploymentText.includes('cloud') ||
    deploymentText.includes('api') ||
    deploymentText.includes('remote') ||
    deploymentText.includes('hosted');
  const local = row.local === true ||
    row.local_available === true ||
    row.local_installed === true ||
    row.installed === true ||
    row.downloaded === true ||
    localProvider ||
    deploymentText.includes('local');
  const installed = row.installed === true ||
    row.downloaded === true ||
    row.local_available === true ||
    row.local_installed === true ||
    deploymentText.includes('installed');
  const updatable = installed && (
    row.update_available === true ||
    row.local_update_available === true ||
    row.can_update === true ||
    !!cleanText(row.update_command || row.ollama_update_command || '', 1000)
  );
  const explicitDownload = row.download_available === true ||
    row.local_download_available === true ||
    row.install_available === true ||
    row.ollama_pull_available === true ||
    row.can_download === true ||
    row.can_pull === true ||
    !!cleanText(row.download_url || row.download_command || row.pull_command || row.install_command || '', 1000);
  const downloadable = !cloud && !installed && explicitDownload;
  const available = row.available !== false;
  const blocked = available === false || row.blocked === true;
  const requiresAuth = row.requires_auth === true || (cloud && row.authenticated !== true);
  const deploymentKind = cloud
    ? 'cloud'
    : local
      ? 'local'
      : cleanText(deploymentSource || 'unknown', 80).toLowerCase() || 'unknown';
  const actionKind = blocked
    ? 'blocked'
    : cloud
      ? 'cloud'
      : updatable
        ? 'update'
        : installed
          ? 'locked'
          : downloadable
            ? 'download'
            : '';
  const capabilityKind = actionKind || (available ? 'available' : 'blocked');
  return {
    capability_kind: capabilityKind,
    action_kind: actionKind,
    deployment_kind: deploymentKind,
    cloud,
    api_backed: cloud,
    local,
    installed,
    downloadable,
    updatable,
    requires_auth: requiresAuth,
    selectable: available && !blocked,
    capability_status: blocked
      ? 'blocked'
      : cloud
        ? (requiresAuth ? 'cloud_requires_auth' : 'cloud_available')
        : updatable
          ? 'local_update_available'
          : installed
            ? 'local_installed'
            : downloadable
              ? 'local_download_available'
              : 'available',
  };
}

function projectAgentRuntimeModelMenu(engine, health) {
  const row = engine && typeof engine === 'object' ? engine : {};
  const source = row.model_menu && typeof row.model_menu === 'object' ? row.model_menu : {};
  const healthMenu = health && health.model_menu && typeof health.model_menu === 'object' ? health.model_menu : {};
  const menuSource = cleanText(healthMenu.source || source.source || 'infring_model_catalog', 120);
  const modelRowsSource = Array.isArray(healthMenu.model_rows)
    ? healthMenu.model_rows
    : (Array.isArray(source.model_rows) ? source.model_rows : []);
  const modelRows = modelRowsSource.map((item) => {
    const model = item && typeof item === 'object' ? item : {};
    const capability = classifyAgentRuntimeModelCapability(model, menuSource);
    return {
      id: cleanText(model.id || model.qualified_model_ref || model.model || '', 240),
      provider: cleanText(model.provider || model.model_provider || '', 120),
      model: cleanText(model.model || model.model_name || '', 180),
      model_name: cleanText(model.model_name || model.model || '', 180),
      display_name: cleanText(model.display_name || model.model_name || model.model || model.id || '', 180),
      available: capability.selectable,
      source: cleanText(model.source || menuSource || '', 120),
      adapter_model_arg: cleanText(model.adapter_model_arg || model.model || model.model_name || '', 180),
      capability_kind: capability.capability_kind,
      capability_status: capability.capability_status,
      action_kind: capability.action_kind,
      deployment_kind: capability.deployment_kind,
      cloud: capability.cloud,
      api_backed: capability.api_backed,
      local: capability.local,
      installed: capability.installed,
      downloadable: capability.downloadable,
      download_available: capability.downloadable,
      updatable: capability.updatable,
      update_available: capability.updatable,
      requires_auth: capability.requires_auth,
    };
  }).filter((item) => {
    const modelId = cleanText(item.id || item.model || item.model_name || item.adapter_model_arg || '', 240).toLowerCase();
    const display = cleanText(item.display_name || '', 240).toLowerCase();
    if (!modelId) return false;
    if (modelId === 'default' || modelId === 'framework-default' || modelId === 'framework_default') return false;
    if (modelId.endsWith('/default') || modelId.endsWith('/framework-default') || modelId.endsWith('/framework_default')) return false;
    if (display === 'default' || display.endsWith(' default')) return false;
    return true;
  }).slice(0, 64);
  const policySource = healthMenu.default_selection_policy && typeof healthMenu.default_selection_policy === 'object'
    ? healthMenu.default_selection_policy
    : (source.default_selection_policy && typeof source.default_selection_policy === 'object' ? source.default_selection_policy : null);
  const refreshSource = healthMenu.catalog_refresh_policy && typeof healthMenu.catalog_refresh_policy === 'object'
    ? healthMenu.catalog_refresh_policy
    : (source.catalog_refresh_policy && typeof source.catalog_refresh_policy === 'object' ? source.catalog_refresh_policy : null);
  return {
    source: menuSource,
    menu_label: cleanText(healthMenu.menu_label || source.menu_label || '', 160),
    show_in_llm_menu: (healthMenu.show_in_llm_menu ?? source.show_in_llm_menu) !== false,
    framework_native_models: (healthMenu.framework_native_models ?? source.framework_native_models) === true,
    inherit_active_llm_when_unconfigured: (healthMenu.inherit_active_llm_when_unconfigured ?? source.inherit_active_llm_when_unconfigured) === true,
    credential_inheritance_allowed: (healthMenu.credential_inheritance_allowed ?? source.credential_inheritance_allowed) === true,
    provider_allowlist: Array.isArray(healthMenu.provider_allowlist)
      ? healthMenu.provider_allowlist.map((item) => cleanText(item, 80)).filter(Boolean).slice(0, 24)
      : (Array.isArray(source.provider_allowlist) ? source.provider_allowlist.map((item) => cleanText(item, 80)).filter(Boolean).slice(0, 24) : []),
    default_selection_policy: policySource
      ? {
        type: cleanText(policySource.type || 'framework_configured_default', 120),
        menu_row: policySource.menu_row === true,
        current_model: cleanText(policySource.current_model || '', 240),
        rule: cleanText(policySource.rule || 'Default is a framework/provider policy, not a model row.', 500),
      }
      : null,
    catalog_refresh_policy: refreshSource
      ? {
        mode: cleanText(refreshSource.mode || 'static_seed_until_framework_model_discovery_is_wired', 160),
        freshness_authority: cleanText(refreshSource.freshness_authority || '', 240),
        fallback_source: cleanText(refreshSource.fallback_source || '', 160),
        rule: cleanText(refreshSource.rule || 'Prefer trusted provider/framework discovery over static model seeds.', 500),
      }
      : null,
    model_rows: modelRows,
    secrets_included: false,
  };
}

function projectAgentRuntimeAvailableModels(engineId, engine, health) {
  const menu = projectAgentRuntimeModelMenu(engine, health);
  let projectionSource = 'system_default';
  if (menu.framework_native_models) projectionSource = 'framework_native';
  else if (menu.inherit_active_llm_when_unconfigured || menu.credential_inheritance_allowed) projectionSource = 'inherited_infring';
  const rows = Array.isArray(menu.model_rows) ? menu.model_rows.slice(0, 64) : [];
  const emptyCatalogReason = rows.length
    ? ''
    : menu.inherit_active_llm_when_unconfigured
      ? 'inherits_active_infring_provider_model'
      : menu.framework_native_models
        ? 'runtime_model_discovery_unavailable_or_filtered'
        : menu.default_selection_policy
          ? 'framework_default_policy_only'
          : 'model_catalog_unavailable';
  return {
    type: 'agent_runtime_available_models_projection',
    source_authority: 'gateway.agent_runtime_engine_projection',
    engine_id: cleanEngineId(engineId),
    source: projectionSource,
    catalog_source: cleanText(menu.source || '', 120),
    menu_label: cleanText(menu.menu_label || '', 160),
    show_in_llm_menu: menu.show_in_llm_menu !== false,
    framework_native_models: menu.framework_native_models === true,
    inherit_active_llm_when_unconfigured: menu.inherit_active_llm_when_unconfigured === true,
    credential_inheritance_allowed: menu.credential_inheritance_allowed === true,
    provider_allowlist: Array.isArray(menu.provider_allowlist) ? menu.provider_allowlist.slice(0, 24) : [],
    default_selection_policy: menu.default_selection_policy && typeof menu.default_selection_policy === 'object'
      ? menu.default_selection_policy
      : null,
    catalog_refresh_policy: menu.catalog_refresh_policy && typeof menu.catalog_refresh_policy === 'object'
      ? menu.catalog_refresh_policy
      : null,
    selected_model_id: '',
    rows,
    model_rows: rows,
    row_count: rows.length,
    empty_catalog_reason: emptyCatalogReason,
    secrets_included: false,
  };
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
  const saveSelection = typeof options.saveSelection === 'function'
    ? options.saveSelection
    : (engineId) => ({
      engine_id: cleanEngineId(engineId),
      updated_at: '',
      source: 'gateway_agent_runtime_engine_projection_noop',
    });

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
    const providerReadiness = cleanText(health && health.provider_readiness ? health.provider_readiness : '', 80);
    const errorCode = cleanText(health && health.error_code ? health.error_code : '', 120);
    const reason = cleanText(health && (health.reason || health.status_reason) ? (health.reason || health.status_reason) : '', 500);
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
    const availableModels = projectAgentRuntimeAvailableModels(engineId, row, health);
    if (supportsLiveSteering && !capabilities.includes('live_steering')) capabilities.push('live_steering');
    if (supportsNextTurnSteering && !capabilities.includes('next_turn_steering')) capabilities.push('next_turn_steering');
    return {
      engine_id: engineId,
      display_name: cleanText(row.display_name || engineId, 120),
      engine_kind: cleanText(row.engine_kind || '', 120),
      transport_kind: cleanText(row.transport_kind || '', 120),
      status,
      selectable,
      provider_readiness: providerReadiness,
      error_code: errorCode,
      reason,
      capabilities: capabilities.slice(0, 14),
      supports_live_steering: supportsLiveSteering,
      supports_next_turn_steering: supportsNextTurnSteering,
      steering_mode: steeringMode,
      steering_transport: cleanText((health && health.steering_transport) || (steeringMode === 'next_turn' ? 'gateway_next_turn_intervention' : steeringMode), 120),
      available_models: availableModels,
      model_menu: {
        source: availableModels.catalog_source,
        menu_label: availableModels.menu_label,
        show_in_llm_menu: availableModels.show_in_llm_menu,
        framework_native_models: availableModels.framework_native_models,
        inherit_active_llm_when_unconfigured: availableModels.inherit_active_llm_when_unconfigured,
        credential_inheritance_allowed: availableModels.credential_inheritance_allowed,
        provider_allowlist: availableModels.provider_allowlist,
        default_selection_policy: availableModels.default_selection_policy,
        catalog_refresh_policy: availableModels.catalog_refresh_policy,
        model_rows: availableModels.rows,
        secrets_included: false,
      },
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

  function agentRuntimeSelectionProjection(traceId, body) {
    const engineId = cleanEngineId(body && (body.engine_id || body.agent_runtime_engine_id || body.runtime_engine_id));
    if (!engineId) return { ok: false, status_code: 400, type: 'agent_runtime_selection_projection', trace_id: traceId, error: 'engine_id_required' };
    const info = loadRegistry(root);
    const engine = findAgentRuntimeEngine(info, engineId);
    if (!engine) return { ok: false, status_code: 404, type: 'agent_runtime_selection_projection', trace_id: traceId, engine_id: engineId, error: 'engine_not_registered' };
    const saved = saveSelection(engineId, traceId);
    return {
      ok: true,
      type: 'agent_runtime_selection_projection',
      trace_id: traceId,
      engine_id: cleanEngineId(saved && saved.engine_id) || engineId,
      updated_at: cleanText(saved && saved.updated_at, 120),
      source: cleanText(saved && saved.source, 160) || 'gateway_agent_runtime_engine_projection',
    };
  }

  return {
    projectAgentRuntimeEngineRow,
    agentRuntimeEnginesProjection,
    agentRuntimeEngineInstallProjection,
    agentRuntimeSelectionProjection,
  };
}

module.exports = {
  createAgentRuntimeEngineProjectionStore,
  findAgentRuntimeEngine,
};
