#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines (Gateway runtime-engine router seam).
//
// This module is intentionally a seam, not live dashboard wiring. It gives the
// Gateway one normalized place to select an agent runtime engine and dispatch
// through the contract in validation/conformance/contracts.

'use strict';

const fs = require('fs');
const path = require('path');
const { createAgentRuntimeTraceWriter } = require('./agent_runtime_trace_writer.ts');

const DEFAULT_REGISTRY_PATH = path.join(
  'validation',
  'conformance',
  'contracts',
  'agent_runtime_engine_registry.json',
);

const MAX_DEFAULT_EVENT_BYTES = 65536;

const FORBIDDEN_DEFAULT_PAYLOAD_FIELDS = new Set([
  'raw',
  'root',
  'all_messages',
  'conversation_tree',
  'raw_tool_input',
  'raw_tool_result',
  'trace_body',
  'plan_graph',
  'workflow_graph',
  'execution_observation',
  'authorization_state',
  'policy_decision',
  'external_framework_transcript',
  'adapter_process_dump',
]);

const REQUIRED_ADAPTER_METHODS = [
  'health_check',
  'start_session',
  'submit_turn',
  'stream_events',
  'cancel_turn',
  'collect_artifacts',
  'emit_receipts',
];

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function loadAgentRuntimeEngineRegistry(root, registryPath) {
  const workspaceRoot = root || process.cwd();
  const rel = registryPath || DEFAULT_REGISTRY_PATH;
  const fullPath = path.isAbsolute(rel) ? rel : path.join(workspaceRoot, rel);
  const registry = readJson(fullPath);
  const engines = Array.isArray(registry.engines) ? registry.engines : [];
  const byId = new Map();
  for (const engine of engines) {
    if (!engine || typeof engine !== 'object') continue;
    const engineId = cleanString(engine.engine_id, 120);
    if (!engineId) continue;
    byId.set(engineId, engine);
  }
  return {
    path: fullPath,
    registry,
    engines,
    byId,
  };
}

function hasForbiddenDefaultField(value, seen = new Set()) {
  if (!value || typeof value !== 'object') return false;
  if (seen.has(value)) return false;
  seen.add(value);
  if (Array.isArray(value)) {
    for (const item of value) {
      if (hasForbiddenDefaultField(item, seen)) return true;
    }
    return false;
  }
  for (const key of Object.keys(value)) {
    if (FORBIDDEN_DEFAULT_PAYLOAD_FIELDS.has(key)) return true;
    if (hasForbiddenDefaultField(value[key], seen)) return true;
  }
  return false;
}

function payloadByteSize(value) {
  try {
    return Buffer.byteLength(JSON.stringify(value || {}), 'utf8');
  } catch {
    return MAX_DEFAULT_EVENT_BYTES + 1;
  }
}

function makeErrorEvent(message, code, reason) {
  return {
    type: 'error',
    trace_id: cleanString(message && message.trace_id, 200),
    request_id: cleanString(message && message.request_id, 200),
    engine_id: cleanString(message && message.engine_id, 120),
    session_id: cleanString(message && message.session_id, 200),
    turn_id: cleanString(message && message.turn_id, 200),
    error_code: cleanString(code, 120),
    reason: cleanString(reason, 1000),
    retryable: false,
  };
}

function normalizeGatewayEvent(event, message, fallbackType) {
  const source = event && typeof event === 'object' ? event : {};
  const messageTraceId = cleanString(message && message.trace_id, 200);
  const sourceTraceId = cleanString(source.trace_id, 200);
  if (!messageTraceId) return makeErrorEvent(message, 'agent_runtime_trace_id_missing', 'Agent runtime message missing trace_id.');
  if (sourceTraceId && sourceTraceId !== messageTraceId) {
    return makeErrorEvent(
      message,
      'agent_runtime_trace_id_replaced',
      'Agent runtime adapter attempted to replace the canonical trace_id.',
    );
  }
  const normalized = {
    ...source,
    type: cleanString(source.type || fallbackType || 'agent_runtime.event', 120),
    trace_id: messageTraceId,
    request_id: cleanString(source.request_id || (message && message.request_id), 200),
    engine_id: cleanString(source.engine_id || (message && message.engine_id), 120),
    session_id: cleanString(source.session_id || (message && message.session_id), 200),
  };
  if (hasForbiddenDefaultField(normalized)) {
    return makeErrorEvent(message, 'agent_runtime_forbidden_default_payload_field', 'Agent runtime event contains a forbidden default payload field.');
  }
  if (payloadByteSize(normalized) > MAX_DEFAULT_EVENT_BYTES) {
    return makeErrorEvent(message, 'agent_runtime_default_payload_budget_exceeded', 'Agent runtime event exceeded default Gateway payload budget.');
  }
  return normalized;
}

function validateAdapterShape(adapter) {
  const missing = [];
  for (const method of REQUIRED_ADAPTER_METHODS) {
    if (!adapter || typeof adapter[method] !== 'function') missing.push(method);
  }
  return missing;
}

function createAgentRuntimeRouter(options = {}) {
  const root = options.root || process.cwd();
  const registryInfo = options.registryInfo || loadAgentRuntimeEngineRegistry(root, options.registryPath);
  const adapters = new Map();
  const traceWriter = options.traceWriter || (options.disableTraceWriter ? null : createAgentRuntimeTraceWriter({ root, tracePath: options.tracePath }));
  const initialAdapters = options.adapters || {};
  for (const [engineId, adapter] of Object.entries(initialAdapters)) {
    adapters.set(engineId, adapter);
  }

  function listEngines() {
    return registryInfo.engines.map((engine) => ({
      engine_id: engine.engine_id,
      display_name: engine.display_name,
      engine_kind: engine.engine_kind,
      transport_kind: engine.transport_kind,
      status: engine.status,
      capabilities: Array.isArray(engine.capabilities) ? engine.capabilities.slice() : [],
    }));
  }

  function getEngine(engineId) {
    return registryInfo.byId.get(cleanString(engineId, 120)) || null;
  }

  function registerAdapter(engineId, adapter) {
    const id = cleanString(engineId, 120);
    if (!id) throw new Error('agent_runtime_engine_id_required');
    if (!getEngine(id)) throw new Error(`agent_runtime_engine_not_registered:${id}`);
    const missing = validateAdapterShape(adapter);
    if (missing.length) throw new Error(`agent_runtime_adapter_missing_methods:${id}:${missing.join(',')}`);
    adapters.set(id, adapter);
    return { ok: true, engine_id: id };
  }

  function requireDispatch(message, method) {
    const msg = message && typeof message === 'object' ? message : {};
    if (!cleanString(msg.trace_id, 200)) return { error: makeErrorEvent(msg, 'agent_runtime_trace_id_missing', 'Every agent runtime message must carry trace_id.') };
    const engine = getEngine(msg.engine_id);
    if (!engine) return { error: makeErrorEvent(msg, 'agent_runtime_engine_unknown', 'Requested engine_id is not registered.') };
    const adapter = adapters.get(engine.engine_id);
    if (!adapter) return { error: makeErrorEvent(msg, 'agent_runtime_adapter_unavailable', 'Requested engine is registered but no live adapter is attached.') };
    if (typeof adapter[method] !== 'function') return { error: makeErrorEvent(msg, 'agent_runtime_adapter_method_missing', `Adapter missing ${method}.`) };
    return { msg, engine, adapter };
  }

  async function callAdapter(message, method, fallbackType) {
    const dispatch = requireDispatch(message, method);
    if (dispatch.error) return dispatch.error;
    try {
      const result = await dispatch.adapter[method]({
        message: dispatch.msg,
        engine: dispatch.engine,
        registry: registryInfo.registry,
      });
      const normalized = normalizeGatewayEvent(result, dispatch.msg, fallbackType);
      if (traceWriter && typeof traceWriter.write === 'function') traceWriter.write(normalized);
      return normalized;
    } catch (err) {
      const error = makeErrorEvent(dispatch.msg, 'agent_runtime_adapter_error', err && err.message ? err.message : String(err || 'adapter_error'));
      if (traceWriter && typeof traceWriter.write === 'function') traceWriter.write(error);
      return error;
    }
  }

  return {
    registry_path: registryInfo.path,
    listEngines,
    getEngine,
    registerAdapter,
    healthCheck(message) {
      return callAdapter(message, 'health_check', 'engine.health.result');
    },
    startSession(message) {
      return callAdapter(message, 'start_session', 'session.started');
    },
    submitTurn(message) {
      return callAdapter(message, 'submit_turn', 'turn.complete');
    },
    streamEvents(message) {
      return callAdapter(message, 'stream_events', 'heartbeat');
    },
    cancelTurn(message) {
      return callAdapter(message, 'cancel_turn', 'turn.cancelled');
    },
    collectArtifacts(message) {
      return callAdapter(message, 'collect_artifacts', 'artifact.created');
    },
    emitReceipts(message) {
      return callAdapter(message, 'emit_receipts', 'receipt.created');
    },
  };
}

module.exports = {
  DEFAULT_REGISTRY_PATH,
  FORBIDDEN_DEFAULT_PAYLOAD_FIELDS,
  REQUIRED_ADAPTER_METHODS,
  loadAgentRuntimeEngineRegistry,
  createAgentRuntimeRouter,
  normalizeGatewayEvent,
  hasForbiddenDefaultField,
};
