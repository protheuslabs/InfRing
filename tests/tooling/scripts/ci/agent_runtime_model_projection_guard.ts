#!/usr/bin/env node
/* eslint-disable no-console */

// Verifies Gateway-owned model menu projection for runtime engines. The Shell
// may display these rows, but model discovery/default interpretation belongs to
// Gateway and adapter health metadata.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_model_projection_guard_current.json');

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function modelRow(provider, model, displayName) {
  return {
    id: `${provider}/${model}`,
    provider,
    model,
    model_name: model,
    display_name: displayName || model,
    adapter_model_arg: model,
    available: true,
    source: 'validation_fixture_live_model_discovery',
  };
}

function engineRegistryRows() {
  return [
    {
      engine_id: 'infring_native',
      display_name: 'InfRing Native',
      status: 'adapter_seam_ready',
      model_menu: {
        source: 'infring_active_model_catalog',
        framework_native_models: false,
        inherit_active_llm_when_unconfigured: true,
        credential_inheritance_allowed: true,
        provider_allowlist: ['openai', 'anthropic', 'xai'],
      },
    },
    {
      engine_id: 'codex_cli',
      display_name: 'Codex',
      status: 'adapter_seam_ready',
      model_menu: {
        source: 'codex_registry_seed',
        framework_native_models: true,
      },
    },
    {
      engine_id: 'claude_code',
      display_name: 'Claude Code',
      status: 'adapter_seam_ready',
      model_menu: {
        source: 'claude_registry_seed',
        framework_native_models: true,
      },
    },
    {
      engine_id: 'grok_code',
      display_name: 'Grok Code',
      status: 'adapter_seam_ready',
      model_menu: {
        source: 'grok_registry_seed',
        framework_native_models: true,
      },
    },
    {
      engine_id: 'openclaw',
      display_name: 'OpenClaw',
      status: 'planned_adapter',
      model_menu: {
        source: 'infring_model_provider_inheritance',
        framework_native_models: false,
        inherit_active_llm_when_unconfigured: true,
        credential_inheritance_allowed: true,
        provider_allowlist: ['openai', 'anthropic', 'xai'],
      },
    },
    {
      engine_id: 'hermes_agent',
      display_name: 'Hermes Agent',
      status: 'planned_adapter',
      model_menu: {
        source: 'infring_model_provider_inheritance',
        framework_native_models: false,
        inherit_active_llm_when_unconfigured: true,
        credential_inheritance_allowed: true,
        provider_allowlist: ['openai', 'anthropic', 'xai'],
      },
    },
  ];
}

function healthForEngine(engineId) {
  const common = {
    type: 'engine.health.result',
    status: 'available',
    discovery_source: 'validation_fixture',
  };
  if (engineId === 'codex_cli') {
    return {
      ...common,
      model_menu: {
        source: 'codex_debug_models',
        framework_native_models: true,
        model_rows: [
          modelRow('openai', 'gpt-5.5', 'GPT-5.5'),
          modelRow('openai', 'gpt-5.4', 'GPT-5.4'),
          modelRow('openai', 'gpt-5.5-spark', 'GPT-5.5 Spark'),
          modelRow('openai', 'default', 'Default'),
        ],
        default_selection_policy: {
          type: 'framework_configured_default',
          menu_row: false,
          current_model: 'gpt-5.5',
          rule: 'Default is metadata, not a selectable model row.',
        },
      },
    };
  }
  if (engineId === 'claude_code') {
    return {
      ...common,
      model_menu: {
        source: 'claude_code_help_model_aliases',
        framework_native_models: true,
        model_rows: [
          modelRow('anthropic', 'claude-opus-4-8', 'Claude Opus 4.8'),
          modelRow('anthropic', 'claude-sonnet-4-6', 'Claude Sonnet 4.6'),
          modelRow('anthropic', 'claude-haiku-4-5-20251001', 'Claude Haiku 4.5'),
          modelRow('anthropic', 'framework-default', 'Framework Default'),
        ],
        default_selection_policy: {
          type: 'framework_configured_default',
          menu_row: false,
          current_model: '',
          rule: 'Claude Code default is framework policy, not a fake row.',
        },
      },
    };
  }
  if (engineId === 'grok_code') {
    return {
      ...common,
      model_menu: {
        source: 'grok_models_command',
        framework_native_models: true,
        model_rows: [
          modelRow('grok_code', 'grok-4', 'Grok 4'),
          modelRow('grok_code', 'grok-code-fast-1', 'Grok Code Fast 1'),
          modelRow('grok_code', 'default', 'Default'),
        ],
        default_selection_policy: {
          type: 'runtime_discovered_default',
          menu_row: false,
          current_model: 'grok-code-fast-1',
          rule: 'Runtime default is metadata, not a selectable row.',
        },
      },
    };
  }
  return common;
}

async function main() {
  const { createAgentRuntimeEngineProjectionStore } = require(path.join(ROOT, 'gateway/runtime/agent_runtime/agent_runtime_engine_projections.ts'));
  const registry = { engines: engineRegistryRows() };
  const store = createAgentRuntimeEngineProjectionStore({
    root: ROOT,
    loadRegistry: () => registry,
    loadSelection: () => ({ engine_id: 'codex_cli', updated_at: '2026-06-07T00:00:00.000Z' }),
    saveSelection: (engineId) => ({ engine_id: engineId, updated_at: '2026-06-07T00:00:00.000Z' }),
    createAdapterMap: () => Object.fromEntries(registry.engines.map((engine) => [
      engine.engine_id,
      {
        health_check: async () => healthForEngine(engine.engine_id),
      },
    ])),
  });
  const projection = await store.agentRuntimeEnginesProjection(`validation:agent-runtime-models:${Date.now()}`);
  const rows = Array.isArray(projection.engines) ? projection.engines : [];
  const violations = [];

  for (const row of rows) {
    const engineId = clean(row.engine_id, 120);
    const available = row.available_models && typeof row.available_models === 'object' ? row.available_models : {};
    const modelRows = Array.isArray(available.rows) ? available.rows : [];
    if (!available || available.type !== 'agent_runtime_available_models_projection') {
      violations.push({ kind: 'available_models_projection_missing', engine_id: engineId });
      continue;
    }
    if (available.secrets_included !== false) violations.push({ kind: 'model_projection_secrets_flag_wrong', engine_id: engineId });
    if (available.show_in_llm_menu !== true) violations.push({ kind: 'model_projection_not_menu_visible', engine_id: engineId });
    if (Number(available.row_count) !== modelRows.length) violations.push({ kind: 'model_row_count_mismatch', engine_id: engineId, row_count: available.row_count, actual: modelRows.length });
    for (const model of modelRows) {
      const id = clean(model.id || model.model || model.model_name || model.adapter_model_arg, 240).toLowerCase();
      const display = clean(model.display_name, 240).toLowerCase();
      if (!id) violations.push({ kind: 'model_row_missing_id', engine_id: engineId, model });
      if (!clean(model.capability_kind || model.action_kind, 120)) violations.push({ kind: 'model_row_capability_missing', engine_id: engineId, model_id: id });
      if (!clean(model.deployment_kind, 120)) violations.push({ kind: 'model_row_deployment_missing', engine_id: engineId, model_id: id });
      if (id === 'default' || id.endsWith('/default') || id === 'framework-default' || id.endsWith('/framework-default') || display === 'default' || display.endsWith(' default')) {
        violations.push({ kind: 'fake_default_model_row_leaked', engine_id: engineId, model });
      }
      if (model.secrets_included === true) violations.push({ kind: 'model_row_secrets_leaked', engine_id: engineId, model_id: id });
      if (model.cloud === true || model.api_backed === true) {
        if (model.deployment_kind !== 'cloud') violations.push({ kind: 'cloud_model_deployment_not_cloud', engine_id: engineId, model_id: id, deployment_kind: model.deployment_kind });
        if (model.action_kind !== 'cloud' && model.capability_kind !== 'cloud') violations.push({ kind: 'cloud_model_action_not_cloud', engine_id: engineId, model_id: id, action_kind: model.action_kind, capability_kind: model.capability_kind });
        if (model.downloadable === true || model.download_available === true) violations.push({ kind: 'cloud_model_marked_downloadable', engine_id: engineId, model_id: id });
      }
      if (model.downloadable === true && model.cloud === true) violations.push({ kind: 'downloadable_model_marked_cloud', engine_id: engineId, model_id: id });
    }
    if (['codex_cli', 'claude_code', 'grok_code'].includes(engineId)) {
      if (available.source !== 'framework_native') violations.push({ kind: 'native_framework_model_source_wrong', engine_id: engineId, source: available.source });
      if (!available.framework_native_models) violations.push({ kind: 'native_framework_models_flag_missing', engine_id: engineId });
      if (!modelRows.length) violations.push({ kind: 'native_framework_model_rows_missing', engine_id: engineId });
      if (!available.default_selection_policy || available.default_selection_policy.menu_row !== false) {
        violations.push({ kind: 'native_framework_default_policy_missing_or_rowed', engine_id: engineId });
      }
      for (const model of modelRows) {
        const id = clean(model.id || model.model || model.model_name || model.adapter_model_arg, 240).toLowerCase();
        if (model.cloud !== true || model.api_backed !== true) violations.push({ kind: 'native_framework_model_not_cloud_api', engine_id: engineId, model_id: id });
      }
    }
    if (['infring_native', 'openclaw', 'hermes_agent'].includes(engineId)) {
      if (available.source !== 'inherited_infring') violations.push({ kind: 'inherited_framework_model_source_wrong', engine_id: engineId, source: available.source });
      if (available.inherit_active_llm_when_unconfigured !== true) violations.push({ kind: 'inherit_active_llm_flag_missing', engine_id: engineId });
      if (modelRows.length && available.empty_catalog_reason) violations.push({ kind: 'non_empty_model_catalog_has_empty_reason', engine_id: engineId });
      if (!modelRows.length && available.empty_catalog_reason !== 'inherits_active_infring_provider_model') {
        violations.push({ kind: 'inherited_framework_empty_reason_wrong', engine_id: engineId, reason: available.empty_catalog_reason });
      }
    }
  }

  for (const expectedEngine of ['infring_native', 'codex_cli', 'claude_code', 'grok_code', 'openclaw', 'hermes_agent']) {
    if (!rows.some((row) => row.engine_id === expectedEngine)) violations.push({ kind: 'expected_engine_missing', engine_id: expectedEngine });
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_model_projection_guard',
    generated_at: new Date().toISOString(),
    mode: 'deterministic_gateway_engine_projection',
    engine_count: rows.length,
    engines: rows.map((row) => ({
      engine_id: row.engine_id,
      status: row.status,
      model_source: row.available_models && row.available_models.source,
      catalog_source: row.available_models && row.available_models.catalog_source,
      row_count: row.available_models && row.available_models.row_count,
      empty_catalog_reason: row.available_models && row.available_models.empty_catalog_reason,
      models: row.available_models && Array.isArray(row.available_models.rows)
        ? row.available_models.rows.map((model) => model.id)
        : [],
      capabilities: row.available_models && Array.isArray(row.available_models.rows)
        ? row.available_models.rows.map((model) => ({
          id: model.id,
          capability_kind: model.capability_kind,
          deployment_kind: model.deployment_kind,
          downloadable: model.downloadable,
          cloud: model.cloud,
        }))
        : [],
    })),
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_model_projection_guard',
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
