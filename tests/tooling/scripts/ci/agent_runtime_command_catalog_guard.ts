#!/usr/bin/env tsx
/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'agent_runtime_command_catalog_guard_current.json');
const CONTRACT_PATH = path.join(ROOT, 'validation', 'conformance', 'contracts', 'agent_runtime_command_catalog_contract.json');
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

function rowKey(row) {
  return `${cleanText(row && row.engine_id, 120)}:${cleanText(row && row.intent_id, 120)}:${cleanText(row && row.display_command, 120)}`;
}

function flatten(groups) {
  const rows = [];
  for (const group of Array.isArray(groups) ? groups : []) {
    for (const row of Array.isArray(group && group.commands) ? group.commands : []) rows.push(row);
  }
  return rows;
}

function main() {
  const contract = readJson(CONTRACT_PATH);
  const registry = readJson(REGISTRY_PATH);
  const store = createAgentRuntimeCommandCatalogStore({
    loadRegistry: () => ({ engines: registry.engines || [] }),
    loadSelection: () => ({ engine_id: 'claude_code' }),
  });

  const traceId = `validation:agent-runtime-command-catalog:${Date.now()}`;
  const claudeProjection = store.agentRuntimeCommandCatalogProjection(traceId, { engine_id: 'claude_code' });
  const codexProjection = store.agentRuntimeCommandCatalogProjection(traceId, { engine_id: 'codex_cli' });
  const claudeRows = flatten(claudeProjection.groups);
  const codexRows = flatten(codexProjection.groups);
  const claudeLogin = claudeRows.find((row) => row.intent_id === 'runtime.authenticate' && row.display_command === '/login');
  const codexLogin = codexRows.find((row) => row.intent_id === 'runtime.authenticate' && row.display_command === '/login');
  const infringRuntime = claudeRows.find((row) => row.intent_id === 'runtime.select' && row.display_command === '/runtime');
  const authAction = store.agentRuntimeCommandActionProjection(traceId, {
    engine_id: 'claude_code',
    intent_id: 'runtime.authenticate',
  });

  const allRows = [...claudeRows, ...codexRows];
  const allowedOperationalStates = new Set(
    (((contract.projection_shape || {}).operational_states) || []).map((value) => cleanText(value, 120))
  );
  const violations = [];
  if (!contract || contract.type !== 'agent_runtime_command_catalog_contract') violations.push('contract_missing_or_wrong_type');
  if (!registry || registry.command_catalog_contract !== 'validation/conformance/contracts/agent_runtime_command_catalog_contract.json') {
    violations.push('engine_registry_command_catalog_contract_ref_missing');
  }
  if (!claudeProjection || claudeProjection.source_authority !== 'gateway.agent_runtime_command_catalog') {
    violations.push('claude_projection_source_authority_unexpected');
  }
  if (!Array.isArray(claudeProjection.groups) || claudeProjection.groups.length < 2) violations.push('claude_projection_groups_missing');
  if (!claudeLogin) violations.push('claude_login_mapping_missing');
  if (claudeLogin && claudeLogin.native_command !== '/login') violations.push(`claude_login_native_command_unexpected:${claudeLogin.native_command}`);
  if (!codexLogin) violations.push('codex_login_mapping_missing');
  if (!infringRuntime) violations.push('infring_runtime_command_missing_from_external_runtime_projection');
  for (const row of allRows) {
    if (row.default_passthrough_allowed !== false) violations.push(`raw_passthrough_not_disabled:${rowKey(row)}`);
    if (row.chat_memory_eligible !== false) violations.push(`control_command_memory_eligible:${rowKey(row)}`);
    if (row.secrets_included !== false) violations.push(`command_row_secrets_included:${rowKey(row)}`);
    if (!row.intent_id || !row.display_command || !row.title || !row.execution_kind) violations.push(`command_row_missing_required_field:${rowKey(row)}`);
    if (!row.operational_state || !allowedOperationalStates.has(cleanText(row.operational_state, 120))) {
      violations.push(`command_row_operational_state_invalid:${rowKey(row)}:${row.operational_state || 'missing'}`);
    }
    if (!row.operational_label) violations.push(`command_row_operational_label_missing:${rowKey(row)}`);
    if (typeof row.connected !== 'boolean') violations.push(`command_row_connected_flag_missing:${rowKey(row)}`);
    if (typeof row.fully_operational !== 'boolean') violations.push(`command_row_fully_operational_flag_missing:${rowKey(row)}`);
    if (row.action_route !== '/api/shell-socket/agent-runtime/commands/execute') {
      violations.push(`command_row_action_route_unexpected:${rowKey(row)}:${row.action_route || 'missing'}`);
    }
    if (
      row.operational_state !== 'connected' &&
      row.operational_state !== 'projection_connected' &&
      row.fully_operational === true
    ) {
      violations.push(`command_row_unwired_marked_fully_operational:${rowKey(row)}:${row.operational_state}`);
    }
  }
  if (!authAction || authAction.status !== 'manual_action_required') {
    violations.push(`auth_action_status_unexpected:${authAction && authAction.status || 'missing'}`);
  }
  if (authAction && authAction.action_executed !== false) violations.push('auth_action_executed_in_guard');
  if (authAction && authAction.secrets_included !== false) violations.push('auth_action_secrets_included');

  const report = {
    ok: violations.length === 0,
    guard: 'agent_runtime_command_catalog_guard',
    type: 'agent_runtime_command_catalog_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    contract_path: 'validation/conformance/contracts/agent_runtime_command_catalog_contract.json',
    registry_path: 'validation/conformance/contracts/agent_runtime_engine_registry.json',
    trace_id: traceId,
    claude_command_count: claudeRows.length,
    codex_command_count: codexRows.length,
    claude_login_mapping: claudeLogin || null,
    codex_login_mapping: codexLogin || null,
    auth_action: authAction || null,
    violations,
  };

  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
