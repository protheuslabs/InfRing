#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::native-orchestration-client.
//
// Gateway owns the bounded HTTP client projection used by legacy hosts to
// reach the runtime orchestration API. Legacy adapter/dashboard hosts must
// construct this client through Gateway rather than carrying orchestration
// request shaping locally.

'use strict';

const {
  fetchGatewayBackend,
  fetchGatewayBackendJson: fetchBackendJson,
  postGatewayBackendJson: postBackendJson,
} = require('./gateway_http_boundary.ts');
const { gatewayRequestTraceId: requestTraceId } = require('./gateway_trace_boundary.ts');
const {
  cleanGatewayText: cleanText,
  cleanGatewayDisplayText: cleanDisplayText,
} = require('./gateway_text_boundary.ts');

async function backendHealth(flags, timeoutMs = 5000) {
  try {
    const res = await fetchGatewayBackend(flags, '/api/healthz', {}, timeoutMs, requestTraceId());
    return !!(res && res.ok);
  } catch {
    return false;
  }
}

function createGatewayNativeOrchestrationClient(flags) {
  return {
    async healthCheck(ctx) {
      const traceId = cleanText(ctx && ctx.trace_id, 200);
      const ready = await backendHealth(flags, 1500);
      return {
        status: ready ? 'available' : 'not_connected',
        readiness: ready ? 'backend_message_path_ready' : 'backend_unreachable',
        engine_kind: 'native_orchestration',
        implementation_path: 'orchestration/**',
        bridge_kind: 'gateway_native_runtime_turn_adapter',
        trace_id: traceId,
      };
    },
    async startSession(ctx) {
      return {
        status: 'completed',
        session_id: cleanText(ctx && ctx.session_id, 200),
        bridge_kind: 'gateway_native_runtime_turn_adapter',
      };
    },
    async submitTurn(ctx) {
      const message = ctx && ctx.message && typeof ctx.message === 'object' ? ctx.message : {};
      const traceId = cleanText(message.trace_id, 200);
      const agentId = cleanText(message.agent_id, 160) || 'default';
      const sessionId = cleanText(message.session_id, 200) || `shell_${agentId}`;
      const turnId = cleanText(message.turn_id, 200) || `turn_${Date.now().toString(36)}`;
      const text = cleanDisplayText(message.input && message.input.text, 24000);
      if (!text) {
        return {
          type: 'turn.complete',
          trace_id: traceId,
          engine_id: 'infring_native',
          agent_id: agentId,
          session_id: sessionId,
          turn_id: turnId,
          status: 'failed',
          reason: 'native_runtime_turn_missing_input',
        };
      }
      const upstream = await postBackendJson(flags, `/api/shell-socket/agents/${encodeURIComponent(agentId)}/message`, {
        message: text,
        agent_runtime_engine_id: 'infring_native',
        runtime_turn_envelope: {
          trace_id: traceId,
          session_id: sessionId,
          turn_id: turnId,
          source: 'gateway_agent_runtime_turn',
          context_pack_ref: `agent-runtime-context/${traceId}/${sessionId}/${turnId}`,
        },
      }, 180000, traceId);
      const output = cleanDisplayText(
        upstream && (upstream.response || upstream.display_text || upstream.output_text || upstream.text || upstream.message),
        24000,
      );
      return {
        type: 'turn.complete',
        trace_id: traceId,
        engine_id: 'infring_native',
        agent_id: agentId,
        session_id: sessionId,
        turn_id: turnId,
        status: output ? 'completed' : 'failed',
        output_text: output,
        output_preview: cleanText(output, 4000),
        result_ref: cleanText(upstream && (upstream.result_ref || upstream.trace_ref), 240) || `native-runtime-result/${traceId}/${sessionId}/${turnId}`,
        receipt_ref: cleanText(upstream && (upstream.receipt_ref || upstream.receipt), 240),
      };
    },
    async streamEvents(ctx) {
      return {
        status: 'completed',
        heartbeat: true,
        session_id: cleanText(ctx && ctx.session_id, 200),
      };
    },
    async cancelTurn(ctx) {
      return {
        status: 'cancelled',
        turn_id: cleanText(ctx && ctx.turn_id, 200),
      };
    },
    async collectArtifacts(ctx) {
      return {
        status: 'completed',
        artifact_ref: `artifact/native/${cleanText(ctx && ctx.trace_id, 200) || 'missing-trace'}`,
        artifact_kind: 'native_runtime_projection',
      };
    },
    async emitReceipts(ctx) {
      return {
        status: 'completed',
        receipt_ref: `receipt/native/${cleanText(ctx && ctx.trace_id, 200) || 'missing-trace'}`,
        receipt_kind: 'gateway_native_runtime_turn_adapter',
      };
    },
  };
}

module.exports = {
  createGatewayNativeOrchestrationClient,
};
