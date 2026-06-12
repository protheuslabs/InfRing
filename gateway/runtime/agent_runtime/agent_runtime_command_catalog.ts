#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::command-catalog.
//
// Gateway owns the public command projection for Agent Runtime controls. Runtime
// adapters may declare private slash/CLI command hints, but Shell/CLI clients
// consume canonical InfRing intents and submit those intents back to Gateway.

'use strict';

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function cleanCommand(value, maxLen = 120) {
  return cleanText(value, maxLen).replace(/[^\S\r\n]+/g, ' ');
}

function cleanEngineId(value) {
  return cleanText(value, 120)
    .toLowerCase()
    .replace(/[^a-z0-9_.-]+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function cleanIntentId(value) {
  return cleanText(value, 120)
    .toLowerCase()
    .replace(/[^a-z0-9_.:-]+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function operationalProjectionForRow(row) {
  const intentId = cleanIntentId(row && row.intent_id);
  const executionKind = cleanText(row && row.execution_kind, 120);
  const explicitState = cleanText(row && row.operational_state, 120);
  let operationalState = explicitState;
  let operationalLabel = cleanText(row && row.operational_label, 120);
  let operationalDetail = cleanText(row && row.operational_detail, 500);
  let connected = row && row.connected === false ? false : true;
  let fullyOperational = row && row.fully_operational === true;

  if (!operationalState && executionKind === 'interactive_runtime_control') {
    operationalState = 'manual_action_required';
    operationalLabel = operationalLabel || 'Manual login';
    operationalDetail = operationalDetail || 'Gateway can expose this runtime control, but interactive auth still requires the runtime native login surface.';
    fullyOperational = false;
  } else if (!operationalState && executionKind === 'gateway_health_projection') {
    operationalState = 'connected';
    operationalLabel = operationalLabel || 'Operational';
    operationalDetail = operationalDetail || 'Gateway executes this read-only status refresh as a bounded runtime health projection.';
    fullyOperational = true;
  } else if (!operationalState && intentId === 'runtime.set_model') {
    operationalState = 'connected';
    operationalLabel = operationalLabel || 'Operational';
    operationalDetail = operationalDetail || 'InfRing model switching has a connected client/Gateway path.';
    fullyOperational = true;
  } else if (!operationalState && (intentId === 'runtime.context_preview' || intentId === 'runtime.help')) {
    operationalState = 'projection_connected';
    operationalLabel = operationalLabel || 'Projection connected';
    operationalDetail = operationalDetail || 'This command resolves to an InfRing-owned bounded projection.';
    fullyOperational = true;
  } else if (!operationalState && intentId === 'runtime.approvals') {
    operationalState = 'projection_connected';
    operationalLabel = operationalLabel || 'Approval projection';
    operationalDetail = operationalDetail || 'This command resolves to pending approval projections; approval decisions remain Gateway-owned.';
    fullyOperational = true;
  } else if (!operationalState) {
    operationalState = 'intent_route_only';
    operationalLabel = operationalLabel || 'Route only';
    operationalDetail = operationalDetail || 'The canonical intent is cataloged, but the full action path is not yet wired.';
    fullyOperational = false;
  }

  if (
    operationalState === 'unsupported' ||
    operationalState === 'stubbed_or_unwired' ||
    operationalState === 'not_available'
  ) {
    connected = false;
    fullyOperational = false;
  }

  return {
    operational_state: operationalState,
    operational_label: operationalLabel || operationalState,
    operational_detail: operationalDetail,
    connected,
    fully_operational: fullyOperational,
  };
}

const INFRING_NATIVE_COMMANDS = Object.freeze([
  {
    intent_id: 'runtime.select',
    display_command: '/runtime',
    title: 'Switch active runtime',
    description: 'Select InfRing Native, Codex, Claude Code, or another registered runtime.',
    execution_kind: 'gateway_state_intent',
    safety_class: 'control',
  },
  {
    intent_id: 'runtime.set_model',
    display_command: '/model',
    title: 'Switch model',
    description: 'Change the active model for the current runtime through InfRing model state.',
    execution_kind: 'gateway_state_intent',
    safety_class: 'control',
  },
  {
    intent_id: 'runtime.set_workspace',
    display_command: '/cwd',
    title: 'Set working directory',
    description: 'Set the runtime home base and workspace permission boundary.',
    execution_kind: 'gateway_state_intent',
    safety_class: 'control',
  },
  {
    intent_id: 'runtime.context_preview',
    display_command: '/context',
    title: 'Preview context pack',
    description: 'Inspect the bounded context projection that will be sent to the selected runtime.',
    execution_kind: 'gateway_projection',
    safety_class: 'read_only',
  },
  {
    intent_id: 'runtime.approvals',
    display_command: '/approve',
    title: 'Review approvals',
    description: 'Open pending runtime approval requests without entering a chat turn.',
    execution_kind: 'gateway_projection',
    safety_class: 'approval',
  },
  {
    intent_id: 'runtime.help',
    display_command: '/help',
    title: 'Show InfRing commands',
    description: 'Show runtime controls and InfRing command help for this client.',
    execution_kind: 'gateway_projection',
    safety_class: 'read_only',
  },
]);

const RUNTIME_COMMAND_MAPPINGS = Object.freeze({
  claude_code: [
    {
      intent_id: 'runtime.authenticate',
      display_command: '/login',
      native_command: '/login',
      native_command_kind: 'slash_command',
      execution_kind: 'interactive_runtime_control',
      title: 'Authenticate Claude Code',
      description: 'Start Claude Code login/authentication for this machine.',
      manual_action_hint: 'Open Claude Code and run /login, or use the Claude CLI login flow if available.',
      safety_class: 'auth',
    },
    {
      intent_id: 'runtime.refresh_status',
      display_command: '/status',
      native_command: '/status',
      native_command_kind: 'slash_command',
      execution_kind: 'gateway_health_projection',
      title: 'Refresh Claude status',
      description: 'Refresh Claude Code availability and authentication status through Gateway.',
      safety_class: 'read_only',
    },
  ],
  codex_cli: [
    {
      intent_id: 'runtime.authenticate',
      display_command: '/login',
      native_command: 'codex login',
      native_command_kind: 'cli_command',
      execution_kind: 'interactive_runtime_control',
      title: 'Authenticate Codex',
      description: 'Start Codex authentication for this machine.',
      manual_action_hint: 'Run codex login in a terminal if interactive authentication is required.',
      safety_class: 'auth',
    },
    {
      intent_id: 'runtime.refresh_status',
      display_command: '/status',
      native_command: 'codex --version',
      native_command_kind: 'cli_command',
      execution_kind: 'gateway_health_projection',
      title: 'Refresh Codex status',
      description: 'Refresh Codex availability and provider status through Gateway.',
      safety_class: 'read_only',
    },
  ],
  opencode: [
    {
      intent_id: 'runtime.authenticate',
      display_command: '/login',
      native_command: 'opencode auth login',
      native_command_kind: 'cli_command',
      execution_kind: 'interactive_runtime_control',
      title: 'Authenticate OpenCode',
      description: 'Start OpenCode authentication if the runtime reports an auth requirement.',
      manual_action_hint: 'Run opencode auth login in a terminal if interactive authentication is required.',
      safety_class: 'auth',
    },
    {
      intent_id: 'runtime.refresh_status',
      display_command: '/status',
      native_command: 'opencode --version',
      native_command_kind: 'cli_command',
      execution_kind: 'gateway_health_projection',
      title: 'Refresh OpenCode status',
      description: 'Refresh OpenCode availability through Gateway.',
      safety_class: 'read_only',
    },
  ],
  grok_code: [
    {
      intent_id: 'runtime.refresh_status',
      display_command: '/status',
      native_command: 'grok --version',
      native_command_kind: 'cli_command',
      execution_kind: 'gateway_health_projection',
      title: 'Refresh Grok Code status',
      description: 'Refresh Grok Code availability through Gateway.',
      safety_class: 'read_only',
    },
  ],
});

function findEngineDisplayName(registryInfo, engineId) {
  const target = cleanEngineId(engineId);
  const engines = Array.isArray(registryInfo && registryInfo.engines) ? registryInfo.engines : [];
  const row = engines.find((engine) => cleanEngineId(engine && engine.engine_id) === target);
  return cleanText(row && row.display_name, 120) || target || 'Agent Runtime';
}

function projectCommandRow(row, engineId, groupId) {
  const intentId = cleanIntentId(row && row.intent_id);
  const command = cleanCommand(row && row.display_command, 120);
  const operational = operationalProjectionForRow(row);
  return {
    type: 'agent_runtime_command_row',
    command_id: `${cleanEngineId(engineId) || 'infring'}:${intentId || command.replace(/^\//, '')}`,
    intent_id: intentId,
    display_command: command,
    canonical_command: cleanCommand(row && row.canonical_command, 120) || `/${intentId.replace(/\./g, ' ')}`,
    title: cleanText(row && row.title, 160),
    description: cleanText(row && row.description, 500),
    group_id: cleanText(groupId, 120),
    engine_id: cleanEngineId(engineId),
    native_command: cleanCommand(row && row.native_command, 500),
    native_command_kind: cleanText(row && row.native_command_kind, 80),
    execution_kind: cleanText(row && row.execution_kind, 120),
    safety_class: cleanText(row && row.safety_class, 80),
    manual_action_hint: cleanText(row && row.manual_action_hint, 500),
    operational_state: operational.operational_state,
    operational_label: operational.operational_label,
    operational_detail: operational.operational_detail,
    connected: operational.connected,
    fully_operational: operational.fully_operational,
    action_route: '/api/shell-socket/agent-runtime/commands/execute',
    default_passthrough_allowed: false,
    chat_memory_eligible: false,
    secrets_included: false,
  };
}

function commandGroupsForEngine(registryInfo, engineId) {
  const cleanEngine = cleanEngineId(engineId || 'infring_native') || 'infring_native';
  const displayName = findEngineDisplayName(registryInfo, cleanEngine);
  const runtimeRows = Array.isArray(RUNTIME_COMMAND_MAPPINGS[cleanEngine])
    ? RUNTIME_COMMAND_MAPPINGS[cleanEngine]
    : [];
  const groups = [];
  if (runtimeRows.length) {
    groups.push({
      type: 'agent_runtime_command_group',
      group_id: 'runtime_native_commands',
      title: `${displayName} / commands`,
      description: `Commands translated from ${displayName} slash/CLI controls into InfRing runtime intents.`,
      engine_id: cleanEngine,
      commands: runtimeRows.map((row) => projectCommandRow(row, cleanEngine, 'runtime_native_commands')),
    });
  }
  groups.push({
    type: 'agent_runtime_command_group',
    group_id: 'infring_native_commands',
    title: 'InfRing native / commands',
    description: 'InfRing-owned runtime controls available across Shell, CLI, and future clients.',
    engine_id: cleanEngine,
    commands: INFRING_NATIVE_COMMANDS.map((row) => projectCommandRow(row, 'infring', 'infring_native_commands')),
  });
  return groups;
}

function flattenCommandGroups(groups) {
  const out = [];
  for (const group of Array.isArray(groups) ? groups : []) {
    const rows = Array.isArray(group && group.commands) ? group.commands : [];
    for (const row of rows) out.push(row);
  }
  return out;
}

function findProjectedCommand(groups, intentId, displayCommand) {
  const cleanIntent = cleanIntentId(intentId);
  const cleanDisplay = cleanCommand(displayCommand, 120).toLowerCase();
  return flattenCommandGroups(groups).find((row) => {
    if (!row) return false;
    if (cleanIntent && cleanIntentId(row.intent_id) === cleanIntent) return true;
    if (cleanDisplay && cleanCommand(row.display_command, 120).toLowerCase() === cleanDisplay) return true;
    return false;
  }) || null;
}

function boundedHealthProjection(engineId, health, error) {
  const source = health && typeof health === 'object' ? health : {};
  const rawStatus = cleanText(source.status || source.provider_readiness || source.state || '', 120);
  const status = rawStatus || (error ? 'health_check_failed' : 'unknown');
  const providerReadiness = cleanText(source.provider_readiness || source.readiness || '', 120);
  const reason = cleanText(
    source.reason ||
      source.error ||
      source.message ||
      source.provider_unavailable_reason ||
      (error && (error.message || error)) ||
      '',
    500,
  );
  const displayText = reason
    ? `${engineId} status refreshed: ${status}. ${reason}`
    : `${engineId} status refreshed: ${status}.`;
  return {
    type: 'agent_runtime_command_status_projection',
    source_authority: 'gateway.agent_runtime_command_catalog',
    engine_id: engineId,
    status,
    provider_readiness: providerReadiness,
    model: cleanText(source.model || source.current_model || source.model_id || '', 160),
    display_text: displayText,
    action_executed: true,
    raw_runtime_payload_included: false,
    secrets_included: false,
  };
}

function healthProjectionActionOutcome(traceId, row, body, options = {}) {
  const intentId = cleanIntentId(row.intent_id);
  const engineId = cleanEngineId(row.engine_id || body && body.engine_id);
  const base = {
    ok: true,
    status_code: 200,
    type: 'agent_runtime_command_action_projection',
    trace_id: traceId,
    status: 'completed',
    terminal_outcome: 'completed',
    intent_id: intentId,
    engine_id: engineId,
    display_command: row.display_command,
    native_command: row.native_command,
    native_command_kind: row.native_command_kind,
    execution_kind: cleanText(row.execution_kind, 120),
    operational_state: row.operational_state,
    operational_label: row.operational_label,
    operational_detail: row.operational_detail,
    connected: row.connected !== false,
    fully_operational: row.fully_operational === true,
    action_executed: true,
    receipt_required_for_durable_effect: false,
    secrets_included: false,
  };
  const projector = typeof options.projectRuntimeStatus === 'function' ? options.projectRuntimeStatus : null;
  if (!projector) {
    const resultProjection = boundedHealthProjection(engineId, {
      status: 'projection_unavailable',
      reason: 'No runtime health projector was configured for this command execution context.',
    });
    return {
      ...base,
      status: 'completed_with_projection_unavailable',
      terminal_outcome: 'completed_with_projection_unavailable',
      display_text: resultProjection.display_text,
      result_projection: resultProjection,
    };
  }
  const projected = projector({
    trace_id: traceId,
    engine_id: engineId,
    intent_id: intentId,
    display_command: row.display_command,
    native_command: row.native_command,
    command: row,
    body,
  });
  const finalize = (health) => {
    const resultProjection = health && health.type === 'agent_runtime_command_status_projection'
      ? health
      : boundedHealthProjection(engineId, health);
    return {
      ...base,
      display_text: resultProjection.display_text,
      result_projection: resultProjection,
    };
  };
  const fail = (error) => {
    const resultProjection = boundedHealthProjection(engineId, { status: 'health_check_failed' }, error);
    return {
      ...base,
      status: 'completed_with_health_failure',
      terminal_outcome: 'completed_with_health_failure',
      display_text: resultProjection.display_text,
      result_projection: resultProjection,
    };
  };
  if (projected && typeof projected.then === 'function') return projected.then(finalize, fail);
  return finalize(projected);
}

function actionOutcomeForCommand(traceId, command, body, options = {}) {
  const row = command && typeof command === 'object' ? command : {};
  const intentId = cleanIntentId(row.intent_id);
  const engineId = cleanEngineId(row.engine_id || body && body.engine_id);
  const executionKind = cleanText(row.execution_kind, 120);
  if (intentId === 'runtime.authenticate') {
    return {
      ok: true,
      status_code: 202,
      type: 'agent_runtime_command_action_projection',
      trace_id: traceId,
      status: 'manual_action_required',
      terminal_outcome: 'manual_action_required',
      intent_id: intentId,
      engine_id: engineId,
      display_command: row.display_command,
      native_command: row.native_command,
      native_command_kind: row.native_command_kind,
      execution_kind: executionKind,
      operational_state: row.operational_state,
      operational_label: row.operational_label,
      operational_detail: row.operational_detail,
      connected: row.connected !== false,
      fully_operational: row.fully_operational === true,
      display_text: row.manual_action_hint || `Authenticate ${engineId} using its native login flow.`,
      action_executed: false,
      receipt_required_for_durable_effect: true,
      secrets_included: false,
    };
  }
  if (intentId === 'runtime.refresh_status') {
    return healthProjectionActionOutcome(traceId, row, body, options);
  }
  return {
    ok: true,
    status_code: 202,
    type: 'agent_runtime_command_action_projection',
    trace_id: traceId,
    status: 'accepted_for_gateway_handling',
    terminal_outcome: 'accepted_for_gateway_handling',
    intent_id: intentId,
    engine_id: engineId,
    display_command: row.display_command,
    native_command: row.native_command,
    native_command_kind: row.native_command_kind,
    execution_kind: executionKind,
    operational_state: row.operational_state,
    operational_label: row.operational_label,
    operational_detail: row.operational_detail,
    connected: row.connected !== false,
    fully_operational: row.fully_operational === true,
    display_text: `Gateway accepted ${row.display_command || intentId} as ${intentId}.`,
    action_executed: false,
    secrets_included: false,
  };
}

function createAgentRuntimeCommandCatalogStore(options = {}) {
  const loadRegistry = typeof options.loadRegistry === 'function'
    ? options.loadRegistry
    : () => ({ engines: [] });
  const loadSelection = typeof options.loadSelection === 'function'
    ? options.loadSelection
    : () => ({ engine_id: 'infring_native' });
  const createAdapterMap = typeof options.createAdapterMap === 'function'
    ? options.createAdapterMap
    : null;
  const projectRuntimeStatus = typeof options.projectRuntimeStatus === 'function'
    ? options.projectRuntimeStatus
    : async ({ trace_id: traceId, engine_id: engineId }) => {
      if (!createAdapterMap) return { status: 'projection_unavailable' };
      const adapters = createAdapterMap() || {};
      const adapter = adapters[engineId];
      if (!adapter || typeof adapter.health_check !== 'function') {
        return {
          status: 'not_available',
          reason: 'Runtime adapter does not expose a health_check method.',
        };
      }
      const registryInfo = loadRegistry();
      const engines = Array.isArray(registryInfo && registryInfo.engines) ? registryInfo.engines : [];
      const engine = engines.find((item) => cleanEngineId(item && item.engine_id) === engineId) || { engine_id: engineId };
      return adapter.health_check({
        message: {
          type: 'engine.health.request',
          trace_id: traceId,
          engine_id: engineId,
          source_authority: 'gateway.agent_runtime_command_catalog',
        },
        engine,
      });
    };

  function selectedEngineId(body) {
    return cleanEngineId(body && (body.engine_id || body.engineId)) ||
      cleanEngineId(loadSelection() && loadSelection().engine_id) ||
      'infring_native';
  }

  function agentRuntimeCommandCatalogProjection(traceId, body = {}) {
    const engineId = selectedEngineId(body);
    const registryInfo = loadRegistry();
    const groups = commandGroupsForEngine(registryInfo, engineId);
    const commands = flattenCommandGroups(groups);
    return {
      ok: true,
      type: 'agent_runtime_command_catalog_projection',
      source_authority: 'gateway.agent_runtime_command_catalog',
      trace_id: String(traceId || ''),
      engine_id: engineId,
      status: 'projected',
      groups,
      commands,
      command_count: commands.length,
      canonical_intent_authority: true,
      runtime_private_command_passthrough_default_allowed: false,
      shell_may_execute_raw_runtime_command: false,
      secrets_included: false,
    };
  }

  function agentRuntimeCommandActionProjection(traceId, body = {}) {
    const engineId = selectedEngineId(body);
    const registryInfo = loadRegistry();
    const groups = commandGroupsForEngine(registryInfo, engineId);
    const command = findProjectedCommand(groups, body.intent_id || body.intentId, body.display_command || body.command);
    if (!command) {
      return {
        ok: false,
        status_code: 404,
        type: 'agent_runtime_command_action_error',
        trace_id: String(traceId || ''),
        engine_id: engineId,
        error: 'agent_runtime_command_not_found',
        intent_id: cleanIntentId(body.intent_id || body.intentId),
        display_command: cleanCommand(body.display_command || body.command, 120),
      };
    }
    return actionOutcomeForCommand(String(traceId || ''), command, body, { projectRuntimeStatus });
  }

  return {
    agentRuntimeCommandCatalogProjection,
    agentRuntimeCommandActionProjection,
  };
}

module.exports = {
  createAgentRuntimeCommandCatalogStore,
  commandGroupsForEngine,
  INFRING_NATIVE_COMMANDS,
  RUNTIME_COMMAND_MAPPINGS,
};
