#!/usr/bin/env node
/* eslint-disable no-console */

// Proves primary external runtime model menus are Gateway-owned, runtime-scoped,
// and truthful enough for Shell rendering: no fake "default" model row, no
// stale cross-runtime provider leakage, and capability metadata is projected
// from the active runtime model source.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_model_projection_truth_guard_current.json');
const ENGINES = ['codex_cli', 'claude_code'];

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function modelRow(provider, model, displayName, extra = {}) {
  return {
    id: `${provider}/${model}`,
    provider,
    model,
    model_name: model,
    display_name: displayName || model,
    adapter_model_arg: model,
    available: true,
    source: 'model_projection_truth_fixture',
    ...extra,
  };
}

function registryEngines() {
  return [
    {
      engine_id: 'codex_cli',
      display_name: 'Codex',
      status: 'adapter_seam_ready',
      model_menu: {
        source: 'codex_registry_seed',
        framework_native_models: true,
        inherit_active_llm_when_unconfigured: false,
        model_rows: [
          modelRow('openai', 'gpt-5.5', 'GPT-5.5'),
          modelRow('openai', 'gpt-5.4', 'GPT-5.4'),
        ],
        default_selection_policy: {
          type: 'framework_configured_default',
          menu_row: false,
          current_model: 'gpt-5.5',
          rule: 'Default is metadata, not a selectable model row.',
        },
      },
    },
    {
      engine_id: 'claude_code',
      display_name: 'Claude Code',
      status: 'adapter_seam_ready',
      model_menu: {
        source: 'claude_registry_seed',
        framework_native_models: true,
        inherit_active_llm_when_unconfigured: false,
        model_rows: [
          modelRow('anthropic', 'claude-opus-4-8', 'Claude Opus 4.8'),
          modelRow('anthropic', 'claude-sonnet-4-6', 'Claude Sonnet 4.6'),
        ],
        default_selection_policy: {
          type: 'framework_configured_default',
          menu_row: false,
          current_model: '',
          rule: 'Claude Code default is framework policy, not a fake row.',
        },
      },
    },
  ];
}

function healthFor(engineId) {
  if (engineId === 'codex_cli') {
    return {
      type: 'engine.health.result',
      status: 'available',
      engine_id: engineId,
      discovery_source: 'model_projection_truth_codex_fixture',
      provider_readiness: 'ready',
      model_menu: {
        source: 'codex_debug_models',
        framework_native_models: true,
        discovery_ok: true,
        model_rows: [
          modelRow('openai', 'gpt-5.5', 'GPT-5.5', { cloud: true, requires_auth: true }),
          modelRow('openai', 'gpt-5.4', 'GPT-5.4', { cloud: true, requires_auth: true }),
          modelRow('openai', 'default', 'Default', { cloud: true, requires_auth: true }),
        ],
        default_selection_policy: {
          type: 'runtime_discovered_default',
          menu_row: false,
          current_model: 'gpt-5.5',
          rule: 'Runtime default is metadata, not a selectable model row.',
        },
      },
    };
  }
  return {
    type: 'engine.health.result',
    status: 'available',
    engine_id: engineId,
    discovery_source: 'model_projection_truth_claude_fixture',
    provider_readiness: 'ready',
    model_menu: {
      source: 'claude_code_help_model_aliases',
      framework_native_models: true,
      discovery_ok: true,
      model_rows: [
        modelRow('anthropic', 'claude-opus-4-8', 'Claude Opus 4.8', { cloud: true, requires_auth: true }),
        modelRow('anthropic', 'claude-sonnet-4-6', 'Claude Sonnet 4.6', { cloud: true, requires_auth: true }),
        modelRow('anthropic', 'framework-default', 'Framework Default', { cloud: true, requires_auth: true }),
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

function createAdapterMap() {
  const out = {};
  for (const engineId of ENGINES) {
    out[engineId] = {
      health_check: async ({ message }) => ({
        ...healthFor(engineId),
        trace_id: message && message.trace_id,
      }),
      start_session: async () => ({ status: 'started' }),
      submit_turn: async () => ({ status: 'completed' }),
      stream_events: async () => ({ status: 'ok' }),
      cancel_turn: async () => ({ status: 'cancelled' }),
      collect_artifacts: async () => ({ artifacts: [] }),
      emit_receipts: async () => ({ receipt_refs: [] }),
    };
  }
  return out;
}

function modelRows(engineRow) {
  const available = engineRow && engineRow.available_models && typeof engineRow.available_models === 'object'
    ? engineRow.available_models
    : {};
  if (Array.isArray(available.rows)) return available.rows;
  if (Array.isArray(available.model_rows)) return available.model_rows;
  const menu = engineRow && engineRow.model_menu && typeof engineRow.model_menu === 'object'
    ? engineRow.model_menu
    : {};
  return Array.isArray(menu.model_rows) ? menu.model_rows : [];
}

function rowModelId(row) {
  return clean(row && (row.model || row.model_name || row.id || row.display_name), 240);
}

function rowProvider(row) {
  return clean(row && (row.provider || row.model_provider || row.provider_id || row.source_provider), 120).toLowerCase();
}

function defaultLike(row) {
  const values = [
    row && row.id,
    row && row.model,
    row && row.model_name,
    row && row.display_name,
    row && row.adapter_model_arg,
  ].map((item) => clean(item, 240).toLowerCase());
  return values.some((value) =>
    value === 'default' ||
    value === 'framework-default' ||
    value.endsWith('/default') ||
    value.endsWith('/framework-default') ||
    value === 'framework default'
  );
}

function summarizeEngine(row) {
  const rows = modelRows(row);
  const ids = rows.map(rowModelId).filter(Boolean);
  const providers = rows.map(rowProvider).filter(Boolean);
  const defaultRows = rows.filter(defaultLike).map((item) => ({
    id: clean(item && item.id, 240),
    model: clean(item && item.model, 240),
    display_name: clean(item && item.display_name, 240),
  }));
  const activeEngine = clean(row && row.engine_id, 120);
  const expectedProvider = activeEngine === 'codex_cli' ? 'openai' : 'anthropic';
  const forbiddenProvider = activeEngine === 'codex_cli' ? 'anthropic' : 'openai';
  const expectedModels = activeEngine === 'codex_cli'
    ? ['gpt-5.5', 'gpt-5.4']
    : ['claude-opus-4-8', 'claude-sonnet-4-6'];
  const available = row && row.available_models && typeof row.available_models === 'object' ? row.available_models : {};
  const defaultPolicy = available.default_selection_policy && typeof available.default_selection_policy === 'object'
    ? available.default_selection_policy
    : row && row.model_menu && row.model_menu.default_selection_policy || {};
  const rowsHaveCapabilityState = rows.every((item) =>
    item &&
    typeof item === 'object' &&
    item.cloud === true &&
    item.downloadable !== true &&
    item.requires_auth === true
  );
  return {
    engine_id: activeEngine,
    row_count: rows.length,
    ids,
    providers,
    catalog_source: clean(available.catalog_source || row && row.model_menu && row.model_menu.source, 160),
    framework_native_models: available.framework_native_models === true || !!(row && row.model_menu && row.model_menu.framework_native_models),
    inherit_active_llm_when_unconfigured: available.inherit_active_llm_when_unconfigured === true,
    default_policy_menu_row: defaultPolicy && defaultPolicy.menu_row,
    default_policy_current_model: clean(defaultPolicy && defaultPolicy.current_model, 240),
    default_rows: defaultRows,
    expected_models_present: expectedModels.every((model) => ids.includes(model) || ids.some((id) => id.endsWith(`/${model}`))),
    expected_provider_only: providers.length > 0 && providers.every((provider) => provider === expectedProvider),
    forbidden_provider_absent: !providers.includes(forbiddenProvider),
    rows_have_capability_state: rowsHaveCapabilityState,
    ok: !!(
      activeEngine &&
      rows.length >= expectedModels.length &&
      expectedModels.every((model) => ids.includes(model) || ids.some((id) => id.endsWith(`/${model}`))) &&
      providers.length > 0 &&
      providers.every((provider) => provider === expectedProvider) &&
      !providers.includes(forbiddenProvider) &&
      defaultRows.length === 0 &&
      defaultPolicy &&
      defaultPolicy.menu_row === false &&
      (available.framework_native_models === true || !!(row && row.model_menu && row.model_menu.framework_native_models)) &&
      available.inherit_active_llm_when_unconfigured !== true &&
      rowsHaveCapabilityState
    ),
  };
}

async function main() {
  const { createAgentRuntimeEngineProjectionStore } = require(path.join(
    ROOT,
    'gateway/runtime/agent_runtime/agent_runtime_engine_projections.ts',
  ));
  const store = createAgentRuntimeEngineProjectionStore({
    root: ROOT,
    loadRegistry: () => ({ engines: registryEngines() }),
    createAdapterMap,
    loadSelection: () => ({ engine_id: 'codex_cli', updated_at: '2026-06-12T00:00:00.000Z' }),
    saveSelection: (engineId, traceId) => ({
      engine_id: engineId,
      trace_id: traceId,
      updated_at: '2026-06-12T00:00:00.000Z',
      source: 'model_projection_truth_guard',
    }),
  });
  const projection = await store.agentRuntimeEnginesProjection(`validation:agent-runtime-model-projection-truth:${Date.now()}`);
  const rows = Array.isArray(projection && projection.engines) ? projection.engines : [];
  const summaries = ENGINES.map((engineId) => summarizeEngine(rows.find((row) => clean(row && row.engine_id, 120) === engineId)));
  const violations = [];
  if (!projection || projection.ok !== true) {
    violations.push({ kind: 'model_projection_missing_or_not_ok' });
  }
  for (const summary of summaries) {
    if (!summary.ok) {
      violations.push({
        kind: 'primary_engine_model_projection_truth_failed',
        engine_id: summary.engine_id,
        row_count: summary.row_count,
        ids: summary.ids,
        providers: summary.providers,
        default_rows: summary.default_rows,
        expected_models_present: summary.expected_models_present,
        expected_provider_only: summary.expected_provider_only,
        forbidden_provider_absent: summary.forbidden_provider_absent,
        rows_have_capability_state: summary.rows_have_capability_state,
        default_policy_menu_row: summary.default_policy_menu_row,
        inherit_active_llm_when_unconfigured: summary.inherit_active_llm_when_unconfigured,
      });
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_model_projection_truth_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    mode: 'deterministic_primary_external_gateway_model_projection_truth',
    engines_tested: ENGINES,
    active_engine_id: projection && projection.active_engine_id,
    summaries,
    policy: {
      shell_replaceability_policy: 'Shell receives model rows only; Gateway owns model truth and runtime scoping.',
      default_policy: 'Default/current model is metadata, not a selectable model row.',
      runtime_scope_policy: 'Codex and Claude rows must not leak across each other or fall back to the generic InfRing provider menu.',
      capability_policy: 'Cloud/API-backed rows are projected as non-downloadable and auth-backed.',
    },
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
    type: 'agent_runtime_model_projection_truth_guard',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    policy_path: 'validation/conformance/contracts/agent_runtime_framework_feature_parity_contract.json',
    error: clean(error && error.stack ? error.stack : error, 6000),
    violations: ['model_projection_truth_guard_crashed'],
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
