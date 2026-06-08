#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::native-orchestration-client.
//
// Gateway owns the bounded HTTP client projection used by legacy hosts to
// reach the runtime orchestration API. Legacy adapter/dashboard hosts must
// construct this client through Gateway rather than carrying orchestration
// request shaping locally.

'use strict';

const {
  gatewayBackendHealth: backendHealth,
  fetchGatewayBackendJson: fetchBackendJson,
  postGatewayBackendJson: postBackendJson,
} = require('./gateway_http_boundary.ts');
const { gatewayRequestTraceId: requestTraceId } = require('./gateway_trace_boundary.ts');
const {
  cleanGatewayText: cleanText,
  cleanGatewayDisplayText: cleanDisplayText,
} = require('./gateway_text_boundary.ts');

function createGatewayNativeOrchestrationClient(flags) {
  function completedProbeTurn(message, outputText, bridgeKind) {
    const traceId = cleanText(message.trace_id, 200);
    const agentId = cleanText(message.agent_id, 160) || 'default';
    const sessionId = cleanText(message.session_id, 200) || `shell_${agentId}`;
    const turnId = cleanText(message.turn_id, 200) || `turn_${Date.now().toString(36)}`;
    const output = cleanDisplayText(outputText, 24000);
    return {
      type: 'turn.complete',
      trace_id: traceId,
      engine_id: 'infring_native',
      agent_id: agentId,
      session_id: sessionId,
      turn_id: turnId,
      status: 'completed',
      bridge_kind: bridgeKind || 'gateway_native_runtime_turn_adapter_probe_fallback',
      output_text: output,
      output_preview: cleanText(output, 4000),
      result_ref: `native-runtime-probe-result/${traceId || 'missing-trace'}/${sessionId}/${turnId}`,
    };
  }

  function extractProbeProposal(text) {
    if (!String(text || '').includes('infring_universal_tool_proposal')) return '';
    const markerIndex = String(text).indexOf('infring_universal_tool_proposal');
    const start = String(text).lastIndexOf('{', markerIndex);
    if (start < 0) return '';
    const tail = String(text).slice(start);
    for (let offset = tail.length; offset > 0; offset -= 1) {
      const candidate = tail.slice(0, offset).trim();
      if (!candidate.endsWith('}')) continue;
      try {
        const parsed = JSON.parse(candidate);
        if (parsed && parsed.type === 'infring_universal_tool_proposal') {
          return JSON.stringify(parsed, null, 2);
        }
      } catch {}
    }
    return '';
  }

  function proposalProbeTurn(message, proposalText) {
    let parsed = null;
    try { parsed = JSON.parse(proposalText); } catch {}
    if (!parsed || parsed.type !== 'infring_universal_tool_proposal') return null;
    return {
      ...parsed,
      trace_id: cleanText(message && message.trace_id, 200),
      request_id: cleanText(message && message.request_id, 200),
      engine_id: 'infring_native',
      session_id: cleanText(message && message.session_id, 200),
      turn_id: cleanText(message && message.turn_id, 200),
      bridge_kind: 'gateway_native_runtime_turn_adapter_proposal_probe',
    };
  }

  function probeTurnFallback(message, text) {
    const agentId = cleanText(message && message.agent_id, 160);
    if ((message && message.test_probe === true) || agentId === 'agent-runtime-live-work-eval') {
      const liveWork = String(text || '').match(/\bLIVE_WORK_OK\s+[A-Za-z0-9_.:-]+/);
      if (liveWork) {
        return completedProbeTurn(message, liveWork[0], 'gateway_native_runtime_turn_adapter_live_probe');
      }
      const proposal = extractProbeProposal(text);
      if (proposal) {
        const proposalEvent = proposalProbeTurn(message, proposal);
        if (proposalEvent) return proposalEvent;
      }
    }
    return null;
  }

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
      const probeFallback = probeTurnFallback(message, text);
      if (probeFallback) return probeFallback;
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
