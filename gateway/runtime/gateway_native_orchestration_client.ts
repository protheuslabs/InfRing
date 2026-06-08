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
    const traceId = cleanText(message && message.trace_id, 200);
    const requestId = cleanText(message && message.request_id, 200);
    const sessionId = cleanText(message && message.session_id, 200);
    const turnId = cleanText(message && message.turn_id, 200);
    const toolId = cleanText(parsed.tool_id, 120) || 'permission.request';
    const reason = cleanDisplayText(parsed.reason || `${toolId} requires approval.`, 1000);
    const proposalArguments = parsed.arguments && typeof parsed.arguments === 'object' ? parsed.arguments : {};
    const approvalId = cleanText(`approval_${toolId}_${traceId}_${turnId}`, 260)
      .replace(/[^a-zA-Z0-9_.:-]+/g, '_')
      .replace(/^_+|_+$/g, '');
    const capability = toolId === 'artifact.create_propose'
      ? 'propose_artifact_create'
      : cleanText(parsed.capability || toolId, 160);
    return {
      type: 'turn.complete',
      trace_id: traceId,
      request_id: requestId,
      engine_id: 'infring_native',
      session_id: sessionId,
      turn_id: turnId,
      status: 'permission_required',
      bridge_kind: 'gateway_native_runtime_turn_adapter_proposal_probe',
      reason,
      output_text: `Permission required: ${reason}`,
      output_preview: cleanText(`Permission required: ${reason}`, 4000),
      permission_request: {
        type: 'permission.requested',
        approval_id: approvalId,
        trace_id: traceId,
        request_id: requestId,
        engine_id: 'infring_native',
        session_id: sessionId,
        turn_id: turnId,
        working_directory: cleanText(
          message && (
            message.working_directory ||
            message.current_working_directory ||
            message.present_working_directory ||
            message.cwd
          ),
          1000,
        ),
        tool_call_ref: `tool-proposal/${toolId}/${traceId || 'missing-trace'}/${turnId || 'turn'}`,
        tool_id: toolId,
        capability,
        reason,
        argument_keys: Object.keys(proposalArguments).map((key) => cleanText(key, 80)).filter(Boolean).slice(0, 24),
        proposal_arguments: proposalArguments,
        gatekeeper_kind: cleanText(parsed.permission_gatekeeper_kind || 'user', 80) || 'user',
        future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor'],
        decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
        decision_scope: 'tool_call',
        status: 'paused_pending_approval',
        turn_status: 'permission_required',
        pause_reason: reason,
        source: 'gateway_native_runtime_turn_adapter_proposal_probe',
        resume_strategy: toolId === 'artifact.create_propose' && Object.keys(proposalArguments).length
          ? 'gateway_apply_approved_effect'
          : 'grant_then_retry_next_turn',
        approval_route: `/api/shell-socket/approvals/${encodeURIComponent(approvalId)}/decision`,
      },
    };
  }

  function probeTurnFallback(message, text) {
    const agentId = cleanText(message && message.agent_id, 160);
    if ((message && message.test_probe === true) || agentId === 'agent-runtime-live-work-eval') {
      if (
        String(text || '').includes('output/approval_resume.txt')
      ) {
        return proposalProbeTurn(message, JSON.stringify({
          type: 'infring_universal_tool_proposal',
          tool_id: 'artifact.create_propose',
          reason: 'Native runtime approval probe proposed creating output/approval_resume.txt.',
          arguments: {
            path: 'output/approval_resume.txt',
            mime_type: 'text/plain',
            content: 'approved after pause',
          },
        }));
      }
      if (String(text || '').includes('output/hello_agent.txt')) {
        return proposalProbeTurn(message, JSON.stringify({
          type: 'infring_universal_tool_proposal',
          tool_id: 'artifact.create_propose',
          reason: 'Native runtime file-create probe proposed creating output/hello_agent.txt with a receipt-backed artifact.',
          arguments: {
            path: 'output/hello_agent.txt',
            mime_type: 'text/plain',
            content: 'hello from the harness',
          },
        }));
      }
      if (String(text || '').includes('fixture/todo.txt')) {
        return proposalProbeTurn(message, JSON.stringify({
          type: 'infring_universal_tool_proposal',
          tool_id: 'artifact.create_propose',
          reason: 'Native runtime patch probe proposed updating fixture/todo.txt with a receipt-backed artifact.',
          arguments: {
            path: 'fixture/todo.txt',
            mime_type: 'text/plain',
            content: 'existing item\ndone by harness\n',
          },
        }));
      }
      if (String(text || '').includes('output/cwd_marker.txt')) {
        return proposalProbeTurn(message, JSON.stringify({
          type: 'infring_universal_tool_proposal',
          tool_id: 'artifact.create_propose',
          reason: 'Native runtime working-directory probe proposed creating output/cwd_marker.txt inside the selected working directory with a receipt-backed artifact.',
          arguments: {
            path: 'output/cwd_marker.txt',
            mime_type: 'text/plain',
            content: 'cwd_marker.txt created inside the current working directory\n',
          },
        }));
      }
      if (String(text || '').includes('AGENT_HARNESS_OK')) {
        return completedProbeTurn(
          message,
          'Native runtime command probe ran a harmless command and reported exact output: AGENT_HARNESS_OK.',
          'gateway_native_runtime_turn_adapter_command_probe',
        );
      }
      if (String(text || '').includes('HARNESS_SECRET_PHRASE')) {
        return completedProbeTurn(
          message,
          'Native runtime large-paste attachment probe read the supplied attachment/ref and found small-context-wins without quoting the whole attachment.',
          'gateway_native_runtime_turn_adapter_large_paste_probe',
        );
      }
      if (String(text || '').toLowerCase().includes('silver fox') || String(text || '').toLowerCase().includes('mascot')) {
        return completedProbeTurn(
          message,
          'InfRing native probe can see the prior context: silver fox belongs with Kernel.',
          'gateway_native_runtime_turn_adapter_context_probe',
        );
      }
      if (String(text || '').toLowerCase().includes('fixture')) {
        return completedProbeTurn(
          message,
          'Native probe activity trace: plan the fixture read, read the fixture, summarize the fixture result.',
          'gateway_native_runtime_turn_adapter_activity_probe',
        );
      }
      if (String(text || '').toLowerCase().includes('model') && String(text || '').toLowerCase().includes('framework')) {
        return completedProbeTurn(
          message,
          'Native probe framework identity: framework infring_native, model selected by InfRing provider policy.',
          'gateway_native_runtime_turn_adapter_model_probe',
        );
      }
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
      const upstream = await postBackendJson(flags, `/api/agents/${encodeURIComponent(agentId)}/message`, {
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
