#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::engine-identity.
//
// Gateway owns public Agent Runtime engine identity normalization. Shells,
// CLIs, SDKs, and external clients may use older or UI-shaped field names, but
// all route policy must collapse those names to one canonical engine_id before
// dispatching adapters or materializing context.

'use strict';

const AGENT_RUNTIME_ENGINE_ID_KEYS = Object.freeze([
  'engine_id',
  'runtime_engine_id',
  'agent_runtime_engine_id',
  'selected_runtime_engine_id',
  'active_runtime_engine_id',
  'selected_default_engine_id',
  'selected_engine_id',
  'active_engine_id',
  'engineId',
  'runtimeEngineId',
  'agentRuntimeEngineId',
  'selectedRuntimeEngineId',
  'activeRuntimeEngineId',
  'selectedDefaultEngineId',
  'selectedEngineId',
  'activeEngineId',
]);

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function cleanEngineId(value) {
  return cleanText(value, 120)
    .toLowerCase()
    .replace(/[^a-z0-9_.-]+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function explicitAgentRuntimeEngineId(source) {
  const row = source && typeof source === 'object' ? source : {};
  for (const key of AGENT_RUNTIME_ENGINE_ID_KEYS) {
    const id = cleanEngineId(row[key]);
    if (id) return id;
  }
  return '';
}

function selectionEngineId(selection) {
  const row = selection && typeof selection === 'object' ? selection : {};
  return cleanEngineId(
    row.engine_id ||
    row.runtime_engine_id ||
    row.agent_runtime_engine_id ||
    row.selected_runtime_engine_id ||
    row.active_runtime_engine_id ||
    row.selected_default_engine_id ||
    row.active_engine_id
  );
}

function resolveAgentRuntimeEngineId(source, options = {}) {
  const explicit = explicitAgentRuntimeEngineId(source);
  if (explicit) return explicit;
  const selection = typeof options.loadSelection === 'function'
    ? options.loadSelection()
    : options.selection;
  const selected = selectionEngineId(selection);
  if (selected) return selected;
  return cleanEngineId(options.defaultEngineId || 'infring_native') || 'infring_native';
}

function withCanonicalAgentRuntimeEngineId(source, options = {}) {
  const body = source && typeof source === 'object' ? source : {};
  const engineId = resolveAgentRuntimeEngineId(body, options);
  return {
    ...body,
    engine_id: engineId,
    runtime_engine_id: engineId,
    agent_runtime_engine_id: engineId,
    selected_runtime_engine_id: engineId,
  };
}

module.exports = {
  AGENT_RUNTIME_ENGINE_ID_KEYS,
  cleanEngineId,
  explicitAgentRuntimeEngineId,
  resolveAgentRuntimeEngineId,
  selectionEngineId,
  withCanonicalAgentRuntimeEngineId,
};
