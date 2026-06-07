#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime (Gateway runtime-engine router seam).
//
// This module is intentionally a seam, not live dashboard wiring. It gives the
// Gateway one normalized place to select an agent runtime engine and dispatch
// through the contract in validation/conformance/contracts.
//
// ROUTER_SCOPE_CONTRACT:
// - The router routes, normalizes, compacts, traces, and dispatches.
// - It must not plan workflows, execute workflow logic, own memory authority,
//   own Shell state, decide approvals, run terminal/file mutations, or contain
//   provider-specific business logic.
// - Provider-specific behavior belongs in adapters; coordination belongs outside
//   this router seam.

'use strict';

const fs = require('fs');
const path = require('path');
const { createAgentRuntimeTraceWriter } = require('./agent_runtime_trace_writer.ts');
const { normalizeUniversalToolProposal } = require('./universal_core_tools.ts');

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

const ROUTER_SCOPE_CONTRACT = Object.freeze({
  router_routes_only: true,
  provider_specific_logic_belongs_in_adapter: true,
  coordination_logic_belongs_outside_router: true,
  approval_decisions_belong_to_gateway_policy_or_gatekeeper: true,
});

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

function cleanDisplayString(value, max = 2000) {
  return String(value == null ? '' : value)
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .trim()
    .slice(0, max);
}

function sanitizeUniversalProposalArguments(args, toolId) {
  const source = args && typeof args === 'object' ? args : {};
  const cleanToolId = cleanString(toolId, 120);
  const out = {};
  if (cleanToolId === 'artifact.create_propose') {
    const rawPath = cleanString(source.path || source.file || source.filename || source.relative_path || '', 500);
    if (rawPath) out.path = rawPath;
    const mimeType = cleanString(source.mime_type || source.content_type || 'text/plain', 120);
    if (mimeType) out.mime_type = mimeType;
    if (source.content != null) out.content = cleanDisplayString(source.content, 48000);
    else if (source.text != null) out.content = cleanDisplayString(source.text, 48000);
    else if (source.body != null) out.content = cleanDisplayString(source.body, 48000);
    return out;
  }
  for (const key of Object.keys(source).slice(0, 24)) {
    const cleanKey = cleanString(key, 80);
    if (!cleanKey) continue;
    const value = source[key];
    if (value == null) continue;
    if (typeof value === 'string') out[cleanKey] = cleanDisplayString(value, 12000);
    else if (typeof value === 'number' || typeof value === 'boolean') out[cleanKey] = value;
    else {
      try {
        out[cleanKey] = cleanDisplayString(JSON.stringify(value), 12000);
      } catch {
        out[cleanKey] = cleanString(value, 1000);
      }
    }
  }
  return out;
}

function compactGatewayActivityEvents(events, maxEvents, maxTextBytes) {
  const rows = Array.isArray(events) ? events : [];
  const limit = Math.max(0, Math.min(Number(maxEvents) || 0, rows.length));
  return rows.slice(Math.max(0, rows.length - limit)).map((event, index) => {
    const row = event && typeof event === 'object' ? event : {};
    const display = cleanDisplayString(row.display_text || row.text || row.summary || row.output || row.result || '', maxTextBytes);
    return {
      type: cleanString(row.type || 'agent_activity_event', 120),
      activity_kind: cleanString(row.activity_kind || row.kind || row.type || 'activity', 80),
      provider_event_type: cleanString(row.provider_event_type || row.provider_type || row.event_type || '', 160),
      source: cleanString(row.source || 'external_runtime', 120),
      sequence_no: Number(row.sequence_no || index + 1) || index + 1,
      item_id: cleanString(row.item_id || row.itemId || row.id || '', 200),
      status: cleanString(row.status || row.state || '', 80),
      text: display,
      display_text: display,
      engine_id: cleanString(row.engine_id, 120),
      trace_id: cleanString(row.trace_id, 200),
      session_id: cleanString(row.session_id, 200),
      turn_id: cleanString(row.turn_id, 200),
    };
  });
}

function compactOversizedGatewayEvent(normalized) {
  const source = normalized && typeof normalized === 'object' ? normalized : {};
  const activityRows = Array.isArray(source.activity_events) ? source.activity_events : [];
  const attempts = [
    { output: 24000, preview: 3000, events: 24, eventText: 700 },
    { output: 16000, preview: 2000, events: 16, eventText: 500 },
    { output: 8000, preview: 1200, events: 8, eventText: 320 },
  ];
  for (const attempt of attempts) {
    const compact = {
      ...source,
      output_text: cleanDisplayString(source.output_text || source.display_text || source.text || source.response || '', attempt.output),
      display_text: cleanDisplayString(source.display_text || source.output_text || source.text || source.response || '', attempt.output),
      text: cleanDisplayString(source.text || source.display_text || source.output_text || source.response || '', attempt.output),
      output_preview: cleanDisplayString(source.output_preview || source.output_text || source.display_text || source.text || '', attempt.preview),
      response: undefined,
      delta: undefined,
      activity_events: compactGatewayActivityEvents(activityRows, attempt.events, attempt.eventText),
      permission_denials: Array.isArray(source.permission_denials)
        ? source.permission_denials.map((item) => cleanDisplayString(item, 700)).filter(Boolean).slice(0, 4)
        : undefined,
      payload_budget_compacted: true,
      original_activity_event_count: Number(source.activity_event_count || activityRows.length) || activityRows.length,
      activity_event_count: Number(source.activity_event_count || activityRows.length) || activityRows.length,
    };
    if (!compact.output_text) delete compact.output_text;
    if (!compact.display_text) delete compact.display_text;
    if (!compact.text) delete compact.text;
    if (!compact.output_preview) delete compact.output_preview;
    if (!compact.activity_events.length) delete compact.activity_events;
    if (!compact.permission_denials || !compact.permission_denials.length) delete compact.permission_denials;
    if (payloadByteSize(compact) <= MAX_DEFAULT_EVENT_BYTES) return compact;
  }
  return null;
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
  if (source.type === 'infring_universal_tool_proposal') {
    const proposal = normalizeUniversalToolProposal(source, message && message.context_pack && message.context_pack.universal_tool_grants);
    if (!proposal.ok) {
      return makeErrorEvent(message, proposal.error_code || 'universal_tool_proposal_invalid', 'Universal tool proposal was rejected by Gateway policy.');
    }
    const toolId = cleanString(proposal.tool_id, 120);
    const normalizedProposal = {
      type: 'tool.proposed',
      trace_id: messageTraceId,
      request_id: cleanString(source.request_id || (message && message.request_id), 200),
      engine_id: cleanString(source.engine_id || (message && message.engine_id), 120),
      session_id: cleanString(source.session_id || (message && message.session_id), 200),
      turn_id: cleanString(source.turn_id || (message && message.turn_id), 200),
      tool_call_ref: `tool-proposal/${toolId}/${messageTraceId}/${cleanString((message && message.turn_id) || source.turn_id || 'turn', 120)}`,
      capability: cleanString(proposal.capability, 160),
      tool_id: toolId,
      proposal_only: true,
      gateway_validation_required: true,
      engine_may_execute_directly: false,
      reason: cleanString(proposal.reason, 1000),
      argument_keys: Object.keys(proposal.arguments || {}).map((key) => cleanString(key, 80)).filter(Boolean).slice(0, 24),
      permission_status: cleanString(proposal.permission_status, 120),
      permission_requires_user_approval: proposal.permission_requires_user_approval === true,
      permission_decision_key: cleanString(proposal.permission_decision_key, 200),
      permission_gatekeeper_kind: cleanString(proposal.permission_gatekeeper_kind || 'user', 80) || 'user',
    };
    if (normalizedProposal.permission_requires_user_approval) {
      const approvalId = cleanString(`approval_${toolId}_${messageTraceId}_${normalizedProposal.turn_id}`, 260)
        .replace(/[^a-zA-Z0-9_.:-]+/g, '_')
        .replace(/^_+|_+$/g, '');
      const proposalArguments = sanitizeUniversalProposalArguments(proposal.arguments, toolId);
      normalizedProposal.permission_request = {
        type: 'permission.requested',
        approval_id: approvalId,
        trace_id: normalizedProposal.trace_id,
        request_id: normalizedProposal.request_id,
        engine_id: normalizedProposal.engine_id,
        session_id: normalizedProposal.session_id,
        turn_id: normalizedProposal.turn_id,
        working_directory: cleanString(
          (message && (
            message.working_directory ||
            message.current_working_directory ||
            message.present_working_directory ||
            message.cwd
          )) || '',
          1000,
        ),
        tool_call_ref: normalizedProposal.tool_call_ref,
        tool_id: normalizedProposal.tool_id,
        capability: normalizedProposal.capability,
        reason: normalizedProposal.reason,
        argument_keys: normalizedProposal.argument_keys,
        proposal_arguments: proposalArguments,
        gatekeeper_kind: normalizedProposal.permission_gatekeeper_kind,
        future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor'],
        decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
        decision_scope: 'tool_call',
        status: 'paused_pending_approval',
        turn_status: 'permission_required',
        pause_reason: normalizedProposal.reason || 'agent_runtime_tool_call_requires_approval',
        source: 'gateway_universal_tool_proposal_normalizer',
        resume_strategy: Object.keys(proposalArguments).length
          ? 'gateway_apply_approved_effect'
          : 'grant_then_retry_next_turn',
        approval_route: `/api/shell-socket/approvals/${encodeURIComponent(approvalId)}/decision`,
      };
    }
    if (hasForbiddenDefaultField(normalizedProposal) || payloadByteSize(normalizedProposal) > MAX_DEFAULT_EVENT_BYTES) {
      return makeErrorEvent(message, 'universal_tool_proposal_projection_invalid', 'Universal tool proposal projection failed Gateway payload policy.');
    }
    return normalizedProposal;
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
    const compact = compactOversizedGatewayEvent(normalized);
    if (compact && !hasForbiddenDefaultField(compact) && payloadByteSize(compact) <= MAX_DEFAULT_EVENT_BYTES) return compact;
    return makeErrorEvent(message, 'agent_runtime_default_payload_budget_exceeded', 'Agent runtime event exceeded default Gateway payload budget after compaction.');
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

  async function streamAdapter(message, onEvent) {
    const dispatch = requireDispatch(message, 'submit_turn');
    if (dispatch.error) return dispatch.error;
    const method = dispatch.adapter && typeof dispatch.adapter.stream_turn === 'function' ? 'stream_turn' : 'submit_turn';
    const emit = (event) => {
      const normalized = normalizeGatewayEvent(event, dispatch.msg, 'agent_activity_event');
      if (traceWriter && typeof traceWriter.write === 'function') traceWriter.write(normalized);
      if (typeof onEvent === 'function') onEvent(normalized);
      return normalized;
    };
    try {
      const result = await dispatch.adapter[method]({
        message: dispatch.msg,
        engine: dispatch.engine,
        registry: registryInfo.registry,
        onActivity: emit,
      });
      const normalized = normalizeGatewayEvent(result, dispatch.msg, 'turn.complete');
      if (traceWriter && typeof traceWriter.write === 'function') traceWriter.write(normalized);
      return normalized;
    } catch (err) {
      const error = makeErrorEvent(dispatch.msg, 'agent_runtime_adapter_error', err && err.message ? err.message : String(err || 'adapter_error'));
      if (traceWriter && typeof traceWriter.write === 'function') traceWriter.write(error);
      if (typeof onEvent === 'function') onEvent(error);
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
    streamTurn(message, onEvent) {
      return streamAdapter(message, onEvent);
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
  ROUTER_SCOPE_CONTRACT,
  loadAgentRuntimeEngineRegistry,
  createAgentRuntimeRouter,
  normalizeGatewayEvent,
  hasForbiddenDefaultField,
};
