#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::socket-handler.
//
// Gateway owns the public Agent Runtime socket semantics. Browser Shells, CLI
// shells, SDKs, and external clients may put bytes on a socket, but engine
// selection, context, approval, turn projection, receipts, and event shaping
// remain here so the dashboard cannot become the framework-router brain.

'use strict';

const {
  resolveAgentRuntimeEngineId,
  withCanonicalAgentRuntimeEngineId,
} = require('./agent_runtime_engine_identity.ts');

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function cleanDisplayText(value, maxLen = 4000) {
  return String(value == null ? '' : value)
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .trim()
    .slice(0, maxLen);
}

function cleanEngineId(value) {
  return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, '');
}

function cleanRef(value, maxLen = 260) {
  return cleanText(value, maxLen).replace(/[^A-Za-z0-9_.:/-]+/g, '_').replace(/^_+|_+$/g, '');
}

function socketError(message, errorCode, reason) {
  const source = message && typeof message === 'object' ? message : {};
  return {
    type: 'error',
    trace_id: cleanText(source.trace_id, 200),
    request_id: cleanText(source.request_id, 200),
    engine_id: cleanEngineId(source.engine_id),
    session_id: cleanText(source.session_id, 200),
    turn_id: cleanText(source.turn_id, 200),
    error_code: cleanText(errorCode, 120),
    reason: cleanText(reason, 1000),
    retryable: false,
    source_authority: 'gateway.runtime.agent_runtime_socket_handler',
  };
}

function emitEvent(emit, event, emitted) {
  const row = event && typeof event === 'object' ? event : {};
  emitted.push(row);
  if (typeof emit === 'function') emit(row);
  return row;
}

function normalizeMessage(message) {
  return message && typeof message === 'object' ? message : {};
}

function traceIdFor(message) {
  return cleanText(message && message.trace_id, 200);
}

function requestIdFor(message) {
  return cleanText(message && message.request_id, 200) || `agent-runtime-socket:${Date.now().toString(36)}`;
}

function socketActivityToolEvent(message, event, status) {
  const source = event && typeof event === 'object' ? event : {};
  const text = cleanDisplayText(source.display_text || source.text || source.title || source.summary || 'Runtime activity.', 1000);
  const traceId = traceIdFor(message);
  const turnId = cleanText(message && message.turn_id, 200);
  const sequence = Number(source.sequence_no) || 0;
  return {
    type: status === 'completed' ? 'tool.completed' : 'tool.started',
    trace_id: traceId,
    request_id: requestIdFor(message),
    engine_id: cleanEngineId(source.engine_id || (message && message.engine_id)),
    session_id: cleanText(source.session_id || (message && message.session_id), 200),
    turn_id: cleanText(source.turn_id || turnId, 200),
    tool_call_ref: cleanRef(source.detail_ref || source.result_ref || `agent-runtime-activity/${traceId}/${turnId || 'turn'}/${sequence || Date.now()}`, 320),
    capability: cleanText(source.activity_kind || 'runtime_activity', 160),
    status,
    display_text: text,
    detail_ref: cleanText(source.detail_ref || '', 260),
    source_authority: 'gateway.runtime.agent_runtime_socket_handler',
  };
}

function permissionRequestedEvent(message, request) {
  const row = request && typeof request === 'object' ? request : {};
  return {
    type: 'permission.requested',
    trace_id: cleanText(row.trace_id || (message && message.trace_id), 200),
    request_id: requestIdFor(message),
    engine_id: cleanEngineId(row.engine_id || (message && message.engine_id)),
    session_id: cleanText(row.session_id || (message && message.session_id), 200),
    turn_id: cleanText(row.turn_id || (message && message.turn_id), 200),
    approval_id: cleanText(row.approval_id, 260),
    tool_call_ref: cleanText(row.tool_call_ref, 260),
    capability: cleanText(row.capability, 160),
    reason: cleanText(row.reason || row.pause_reason, 1000),
    status: cleanText(row.status || 'paused_pending_approval', 120),
    turn_status: cleanText(row.turn_status || 'permission_required', 120),
    approval_route: cleanText(row.approval_route, 500),
    resume_token: cleanText(row.resume_token, 260),
    gatekeeper_kind: cleanText(row.gatekeeper_kind || 'user', 80) || 'user',
    decisions: Array.isArray(row.decisions) ? row.decisions.slice(0, 8) : ['allow_once', 'deny', 'always_allow_tool_call'],
    source_authority: 'gateway.runtime.agent_runtime_socket_handler',
  };
}

function toolProposedEvent(message, request) {
  const row = request && typeof request === 'object' ? request : {};
  return {
    type: 'tool.proposed',
    trace_id: cleanText(row.trace_id || (message && message.trace_id), 200),
    request_id: requestIdFor(message),
    engine_id: cleanEngineId(row.engine_id || (message && message.engine_id)),
    session_id: cleanText(row.session_id || (message && message.session_id), 200),
    turn_id: cleanText(row.turn_id || (message && message.turn_id), 200),
    tool_call_ref: cleanText(row.tool_call_ref, 260),
    capability: cleanText(row.capability, 160),
    tool_id: cleanText(row.tool_id, 120),
    proposal_only: true,
    gateway_validation_required: true,
    engine_may_execute_directly: false,
    reason: cleanText(row.reason || row.pause_reason, 1000),
    source_authority: 'gateway.runtime.agent_runtime_socket_handler',
  };
}

function createAgentRuntimeSocketHandler(options = {}) {
  const engineProjectionStore = options.engineProjectionStore;
  const turnProjectionStore = options.turnProjectionStore;
  const contextPreviewProjectionStore = options.contextPreviewProjectionStore;
  const approvalStore = options.approvalStore;
  const selectEngine = options.selectEngine;
  const steer = options.steer;
  const appendAgentRuntimeTranscriptTurn = typeof options.appendAgentRuntimeTranscriptTurn === 'function'
    ? options.appendAgentRuntimeTranscriptTurn
    : () => {};
  const createNativeOrchestrationClient = typeof options.createNativeOrchestrationClient === 'function'
    ? options.createNativeOrchestrationClient
    : () => ({});
  const loadAgentRuntimeSelection = typeof options.loadAgentRuntimeSelection === 'function'
    ? options.loadAgentRuntimeSelection
    : null;

  if (!engineProjectionStore || typeof engineProjectionStore.agentRuntimeEnginesProjection !== 'function') {
    throw new Error('agent_runtime_socket_engine_projection_store_missing');
  }
  if (!turnProjectionStore || typeof turnProjectionStore.agentRuntimeTurnProjection !== 'function') {
    throw new Error('agent_runtime_socket_turn_projection_store_missing');
  }
  if (!contextPreviewProjectionStore || typeof contextPreviewProjectionStore.agentRuntimeContextPackPreviewProjection !== 'function') {
    throw new Error('agent_runtime_socket_context_preview_store_missing');
  }
  if (!approvalStore || typeof approvalStore.agentRuntimeApprovalDecisionProjection !== 'function') {
    throw new Error('agent_runtime_socket_approval_store_missing');
  }
  if (typeof selectEngine !== 'function') throw new Error('agent_runtime_socket_select_engine_missing');
  if (typeof steer !== 'function') throw new Error('agent_runtime_socket_steer_missing');

  async function emitEngineList(message, emit, emitted) {
    const traceId = traceIdFor(message);
    const projection = await engineProjectionStore.agentRuntimeEnginesProjection(traceId);
    return emitEvent(emit, {
      ok: projection.ok !== false,
      type: 'engine.list.result',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engines: Array.isArray(projection.engines) ? projection.engines.slice(0, 64) : [],
      active_engine_id: cleanEngineId(projection.active_engine_id || projection.selected_default_engine_id),
      selected_default_engine_id: cleanEngineId(projection.selected_default_engine_id),
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function emitEngineHealth(message, emit, emitted) {
    const traceId = traceIdFor(message);
    const projection = await engineProjectionStore.agentRuntimeEnginesProjection(traceId);
    const engineId = cleanEngineId(message.engine_id);
    const row = (Array.isArray(projection.engines) ? projection.engines : []).find((item) => cleanEngineId(item && item.engine_id) === engineId);
    if (!row) return emitEvent(emit, socketError(message, 'agent_runtime_engine_unknown', 'Requested engine_id is not registered.'), emitted);
    return emitEvent(emit, {
      type: 'engine.health.result',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: engineId,
      status: cleanText(row.status || 'unknown', 120),
      selectable: row.selectable === true,
      capabilities: Array.isArray(row.capabilities) ? row.capabilities.slice(0, 16) : [],
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function emitSessionStarted(message, emit, emitted) {
    const traceId = traceIdFor(message);
    const engineId = resolveAgentRuntimeEngineId(message, {
      loadSelection: loadAgentRuntimeSelection,
      defaultEngineId: 'infring_native',
    });
    const sessionId = cleanText(message.session_id, 200) || `agent-runtime-session-${Date.now().toString(36)}`;
    const selection = selectEngine(traceId, { engine_id: engineId });
    if (selection && selection.ok === false) return emitEvent(emit, socketError(message, selection.error || 'agent_runtime_engine_unavailable', 'Session start failed engine selection.'), emitted);
    return emitEvent(emit, {
      type: 'session.started',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: engineId,
      session_id: sessionId,
      receipt_ref: `receipt/agent-runtime-session/${cleanRef(traceId, 120)}/${cleanRef(sessionId, 120)}`,
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function emitContextPreview(message, emit, emitted) {
    const traceId = traceIdFor(message);
    const payload = await contextPreviewProjectionStore.agentRuntimeContextPackPreviewProjection(traceId, message);
    return emitEvent(emit, {
      type: 'context.preview.result',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(payload.engine_id || message.engine_id),
      session_id: cleanText(payload.session_id || message.session_id, 200),
      ok: payload.ok !== false,
      row_count: Number(payload.row_count) || 0,
      fragment_count: Array.isArray(payload.fragments) ? payload.fragments.length : 0,
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function emitSteerAck(message, emit, emitted) {
    const traceId = traceIdFor(message);
    const payload = steer(traceId, message);
    return emitEvent(emit, {
      type: payload.ok === false ? 'error' : 'agent_runtime.steer.ack',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(payload.engine_id || message.engine_id),
      session_id: cleanText(payload.session_id || message.session_id, 200),
      ok: payload.ok !== false,
      status: cleanText(payload.status || '', 120),
      error_code: payload.ok === false ? cleanText(payload.error || 'agent_runtime_steer_failed', 120) : '',
      display_text: cleanDisplayText(payload.display_text || '', 1000),
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function emitTurnSubmit(message, emit, emitted, context) {
    const traceId = traceIdFor(message);
    const canonicalMessage = withCanonicalAgentRuntimeEngineId(message, {
      loadSelection: loadAgentRuntimeSelection,
      defaultEngineId: 'infring_native',
    });
    emitEvent(emit, {
      type: 'tool.started',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(canonicalMessage.engine_id || 'infring_native'),
      session_id: cleanText(message.session_id, 200),
      turn_id: cleanText(message.turn_id, 200),
      tool_call_ref: `agent-runtime-turn/${cleanRef(traceId, 120)}/${cleanRef(message.turn_id || 'turn', 120)}`,
      capability: 'agent_runtime.turn',
      status: 'started',
      display_text: 'Started agent runtime turn through Gateway socket.',
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
    const payload = await turnProjectionStore.agentRuntimeTurnProjection(traceId, canonicalMessage, {
      nativeOrchestrationClient: createNativeOrchestrationClient((context && context.flags) || {}),
      onActivity: (event) => emitEvent(emit, socketActivityToolEvent(message, event, 'started'), emitted),
    });
    const permission = payload && (payload.pending_permission_request || payload.permission_request);
    if (permission) {
      emitEvent(emit, toolProposedEvent(message, permission), emitted);
      emitEvent(emit, permissionRequestedEvent(message, permission), emitted);
    }
    const text = cleanDisplayText(payload && (payload.display_text || payload.output_text || payload.text || payload.output_preview), 4000);
    if (text) {
      emitEvent(emit, {
        type: 'assistant.delta',
        trace_id: traceId,
        request_id: requestIdFor(message),
        engine_id: cleanEngineId(payload.engine_id || message.engine_id),
        session_id: cleanText(payload.session_id || message.session_id, 200),
        turn_id: cleanText(payload.turn_id || message.turn_id, 200),
        delta: text,
        detail_ref: cleanText(payload.result_ref || '', 260),
        source_authority: 'gateway.runtime.agent_runtime_socket_handler',
      }, emitted);
    }
    if (payload && Array.isArray(payload.receipt_refs) && payload.receipt_refs.length) {
      emitEvent(emit, {
        type: 'receipt.created',
        trace_id: traceId,
        request_id: requestIdFor(message),
        engine_id: cleanEngineId(payload.engine_id || message.engine_id),
        session_id: cleanText(payload.session_id || message.session_id, 200),
        turn_id: cleanText(payload.turn_id || message.turn_id, 200),
        receipt_refs: payload.receipt_refs.slice(0, 12),
        receipt_count: Number(payload.receipt_count) || payload.receipt_refs.length,
        source_authority: 'gateway.runtime.agent_runtime_socket_handler',
      }, emitted);
    }
    emitEvent(emit, {
      type: 'tool.completed',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(payload && (payload.engine_id || message.engine_id)),
      session_id: cleanText(payload && (payload.session_id || message.session_id), 200),
      turn_id: cleanText(payload && (payload.turn_id || message.turn_id), 200),
      tool_call_ref: `agent-runtime-turn/${cleanRef(traceId, 120)}/${cleanRef((payload && payload.turn_id) || message.turn_id || 'turn', 120)}`,
      capability: 'agent_runtime.turn',
      status: cleanText(payload && payload.status, 120) || 'completed',
      display_text: 'Agent runtime turn reached a Gateway terminal outcome.',
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
    return emitEvent(emit, {
      type: 'turn.complete',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(payload && (payload.engine_id || message.engine_id)),
      session_id: cleanText(payload && (payload.session_id || message.session_id), 200),
      turn_id: cleanText(payload && (payload.turn_id || message.turn_id), 200),
      status: cleanText(payload && payload.status, 120) || 'completed',
      ok: !payload || payload.ok !== false,
      receipt_refs: payload && Array.isArray(payload.receipt_refs) ? payload.receipt_refs.slice(0, 12) : [],
      pending_permission: !!permission,
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function emitPermissionDecision(message, emit, emitted) {
    const traceId = traceIdFor(message);
    const approvalId = cleanText(message.approval_id || message.permission_id, 260);
    const payload = approvalStore.agentRuntimeApprovalDecisionProjection(traceId, approvalId, message);
    if (payload && payload.ok === false) return emitEvent(emit, socketError(message, payload.error || 'agent_runtime_permission_decision_failed', 'Permission decision failed.'), emitted);
    if (payload && payload.execution_result && payload.execution_result.ok) {
      emitEvent(emit, {
        type: 'artifact.created',
        trace_id: traceId,
        request_id: requestIdFor(message),
        engine_id: cleanEngineId(payload.engine_id || message.engine_id),
        session_id: cleanText(payload.session_id || message.session_id, 200),
        turn_id: cleanText(payload.paused_turn_id || message.turn_id, 200),
        artifact_ref: cleanText(payload.execution_result.artifact_ref || payload.execution_result.path || '', 400),
        receipt_ref: cleanText(payload.decision_receipt_ref, 400),
        source_authority: 'gateway.runtime.agent_runtime_socket_handler',
      }, emitted);
    }
    emitEvent(emit, {
      type: 'receipt.created',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(payload.engine_id || message.engine_id),
      session_id: cleanText(payload.session_id || message.session_id, 200),
      turn_id: cleanText(payload.paused_turn_id || message.turn_id, 200),
      receipt_refs: [cleanText(payload.decision_receipt_ref, 400)].filter(Boolean),
      receipt_count: payload.decision_receipt_ref ? 1 : 0,
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
    return emitEvent(emit, {
      type: 'turn.complete',
      trace_id: traceId,
      request_id: requestIdFor(message),
      engine_id: cleanEngineId(payload.engine_id || message.engine_id),
      session_id: cleanText(payload.session_id || message.session_id, 200),
      turn_id: cleanText(payload.paused_turn_id || message.turn_id, 200),
      status: cleanText(payload.next_action || 'permission_decision_recorded', 120),
      ok: true,
      receipt_refs: [cleanText(payload.decision_receipt_ref, 400)].filter(Boolean),
      source_authority: 'gateway.runtime.agent_runtime_socket_handler',
    }, emitted);
  }

  async function handleAgentRuntimeSocketMessage(message, emit, context = {}) {
    const normalized = normalizeMessage(message);
    const emitted = [];
    const traceId = traceIdFor(normalized);
    if (!traceId) {
      emitEvent(emit, socketError(normalized, 'agent_runtime_trace_id_missing', 'Every Agent Runtime socket message requires trace_id.'), emitted);
      return { ok: false, type: 'agent_runtime_socket_message_result', event_count: emitted.length, events: emitted };
    }
    const type = cleanText(normalized.type, 120);
    if (type === 'engine.list') await emitEngineList(normalized, emit, emitted);
    else if (type === 'engine.health') await emitEngineHealth(normalized, emit, emitted);
    else if (type === 'session.start') await emitSessionStarted(normalized, emit, emitted);
    else if (type === 'turn.submit') await emitTurnSubmit(normalized, emit, emitted, context);
    else if (type === 'turn.cancel') {
      const engineId = cleanEngineId(normalized.engine_id);
      const sessionId = cleanText(normalized.session_id, 200);
      const turnId = cleanText(normalized.turn_id, 200);
      const agentId = cleanText(normalized.agent_id, 160) || 'default';
      const reason = cleanDisplayText(
        normalized.reason || normalized.cancel_reason || 'User stopped the external runtime turn.',
        1000,
      );
      const displayText = `${engineId || 'agent_runtime'} stopped this turn: ${reason}`;
      try {
        appendAgentRuntimeTranscriptTurn({
          sessionId,
          agentId,
          traceId,
          turnId,
          engineId,
          userText: '',
          assistantText: displayText,
          status: 'cancelled',
          activityEvents: [{
            type: 'agent_activity_event',
            activity_kind: 'error',
            provider_event_type: 'turn.cancelled',
            source: 'gateway_runtime_socket_cancel',
            sequence_no: 1,
            item_id: 'gateway-socket-cancel',
            status: 'cancelled',
            text: displayText,
            display_text: displayText,
            engine_id: engineId,
            trace_id: traceId,
            session_id: sessionId,
            turn_id: turnId,
          }],
          activityTrace: {
            type: 'agent_runtime_activity_trace_projection',
            source_authority: 'gateway.runtime.agent_runtime_socket_handler',
            trace_id: traceId,
            engine_id: engineId,
            session_id: sessionId,
            turn_id: turnId,
            collapsed_by_default: true,
            collapse_label: 'Stopped',
            worked_ms: 0,
            row_count: 1,
            raw_activity_event_count: 1,
            rows: [{
              type: 'agent_runtime_activity_trace_row',
              sequence_no: 1,
              activity_kind: 'error',
              provider_event_type: 'turn.cancelled',
              status: 'cancelled',
              title: displayText,
              display_in_thinking_bubble: true,
              detail_ref: `agent-runtime-activity/${traceId}/${turnId || 'turn'}/cancel`,
            }],
            summary_text: displayText,
          },
          workedMs: 0,
          workedLabel: 'Stopped',
        });
      } catch {}
      emitEvent(emit, {
        type: 'turn.complete',
        trace_id: traceId,
        request_id: requestIdFor(normalized),
        engine_id: engineId,
        session_id: sessionId,
        turn_id: turnId,
        status: 'cancelled',
        ok: true,
        display_text: displayText,
        output_text: displayText,
        terminal_outcome: 'cancelled',
        source_authority: 'gateway.runtime.agent_runtime_socket_handler',
      }, emitted);
    } else if (type === 'heartbeat' || type === 'ping') {
      emitEvent(emit, {
        type: 'heartbeat',
        trace_id: traceId,
        request_id: requestIdFor(normalized),
        status: 'ok',
        source_authority: 'gateway.runtime.agent_runtime_socket_handler',
      }, emitted);
    } else if (type === 'context.preview') await emitContextPreview(normalized, emit, emitted);
    else if (type === 'turn.steer') await emitSteerAck(normalized, emit, emitted);
    else if (type === 'permission.decision' || type === 'approval.decision') await emitPermissionDecision(normalized, emit, emitted);
    else emitEvent(emit, socketError(normalized, 'agent_runtime_socket_message_type_unknown', 'Unknown Agent Runtime socket message type.'), emitted);
    return {
      ok: !emitted.some((event) => event && event.type === 'error'),
      type: 'agent_runtime_socket_message_result',
      trace_id: traceId,
      request_id: requestIdFor(normalized),
      event_count: emitted.length,
      events: emitted,
    };
  }

  return {
    handleAgentRuntimeSocketMessage,
  };
}

module.exports = {
  createAgentRuntimeSocketHandler,
};
