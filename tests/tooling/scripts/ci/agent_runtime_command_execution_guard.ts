#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_command_execution_guard_current.json');
const REGISTRY_PATH = path.join(ROOT, 'validation', 'conformance', 'contracts', 'agent_runtime_engine_registry.json');
const {
  createAgentRuntimeCommandCatalogStore,
} = require(path.join(ROOT, 'gateway', 'runtime', 'agent_runtime', 'agent_runtime_command_catalog.ts'));

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function cleanText(value, maxLen = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function flatten(groups) {
  const rows = [];
  for (const group of Array.isArray(groups) ? groups : []) {
    for (const row of Array.isArray(group && group.commands) ? group.commands : []) rows.push(row);
  }
  return rows;
}

function createAdapterMap() {
  return {
    codex_cli: {
      async health_check({ message }) {
        return {
          type: 'engine.health.result',
          trace_id: message && message.trace_id,
          engine_id: 'codex_cli',
          status: 'available',
          provider_readiness: 'ready',
          current_model: 'gpt-5.5',
        };
      },
    },
    claude_code: {
      async health_check({ message }) {
        return {
          type: 'engine.health.result',
          trace_id: message && message.trace_id,
          engine_id: 'claude_code',
          status: 'available',
          provider_readiness: 'ready',
          current_model: 'claude-sonnet-4-6',
        };
      },
    },
  };
}

async function action(store, traceId, engineId, intentId) {
  return await store.agentRuntimeCommandActionProjection(traceId, {
    engine_id: engineId,
    intent_id: intentId,
  });
}

async function main() {
  const registry = readJson(REGISTRY_PATH);
  const store = createAgentRuntimeCommandCatalogStore({
    loadRegistry: () => ({ engines: registry.engines || [] }),
    loadSelection: () => ({ engine_id: 'codex_cli' }),
    createAdapterMap,
  });
  const traceId = `validation:agent-runtime-command-execution:${Date.now()}`;
  const engines = ['codex_cli', 'claude_code'];
  const summaries = [];
  const violations = [];

  for (const engineId of engines) {
    const projection = store.agentRuntimeCommandCatalogProjection(traceId, { engine_id: engineId });
    const rows = flatten(projection.groups);
    const statusRow = rows.find((row) => row.intent_id === 'runtime.refresh_status' && row.display_command === '/status');
    const loginRow = rows.find((row) => row.intent_id === 'runtime.authenticate' && row.display_command === '/login');
    const statusAction = await action(store, traceId, engineId, 'runtime.refresh_status');
    const loginAction = await action(store, traceId, engineId, 'runtime.authenticate');
    const unknownAction = await action(store, traceId, engineId, 'runtime.unknown');
    const result = statusAction && statusAction.result_projection || {};
    summaries.push({
      engine_id: engineId,
      status_row_state: statusRow && statusRow.operational_state,
      status_action_status: statusAction && statusAction.status,
      status_action_executed: statusAction && statusAction.action_executed,
      status_result_status: result.status,
      login_action_status: loginAction && loginAction.status,
      login_action_executed: loginAction && loginAction.action_executed,
      unknown_action_status_code: unknownAction && unknownAction.status_code,
    });
    if (!statusRow) violations.push(`${engineId}:status_row_missing`);
    if (statusRow && statusRow.operational_state !== 'connected') violations.push(`${engineId}:status_row_not_connected:${statusRow.operational_state}`);
    if (statusRow && statusRow.fully_operational !== true) violations.push(`${engineId}:status_row_not_fully_operational`);
    if (!loginRow) violations.push(`${engineId}:login_row_missing`);
    if (!statusAction || statusAction.status !== 'completed') violations.push(`${engineId}:status_action_not_completed:${cleanText(statusAction && statusAction.status, 160)}`);
    if (statusAction && statusAction.action_executed !== true) violations.push(`${engineId}:status_action_not_executed`);
    if (!result || result.type !== 'agent_runtime_command_status_projection') violations.push(`${engineId}:status_result_projection_missing`);
    if (result && result.source_authority !== 'gateway.agent_runtime_command_catalog') violations.push(`${engineId}:status_result_source_authority_unexpected`);
    if (result && result.raw_runtime_payload_included !== false) violations.push(`${engineId}:status_result_raw_payload_flag_unexpected`);
    if (result && result.secrets_included !== false) violations.push(`${engineId}:status_result_secrets_flag_unexpected`);
    if (result && result.status !== 'available') violations.push(`${engineId}:status_result_status_unexpected:${cleanText(result.status, 160)}`);
    if (!loginAction || loginAction.status !== 'manual_action_required') violations.push(`${engineId}:login_not_manual_action:${cleanText(loginAction && loginAction.status, 160)}`);
    if (loginAction && loginAction.action_executed !== false) violations.push(`${engineId}:login_was_executed`);
    if (loginAction && loginAction.secrets_included !== false) violations.push(`${engineId}:login_included_secrets`);
    if (!unknownAction || unknownAction.status_code !== 404) violations.push(`${engineId}:unknown_command_not_404`);
  }

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_command_execution_guard',
    type: 'agent_runtime_command_execution_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    contract_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    engines_tested: engines,
    policy: {
      safe_status_commands_execute_through_gateway: true,
      interactive_auth_commands_remain_manual_action: true,
      shell_executes_raw_runtime_commands: false,
      raw_runtime_payload_included: false,
    },
    summaries,
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    guard: 'agent_runtime_command_execution_guard',
    type: 'agent_runtime_command_execution_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    error: cleanText(error && error.stack ? error.stack : error, 6000),
    violations: ['agent_runtime_command_execution_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
