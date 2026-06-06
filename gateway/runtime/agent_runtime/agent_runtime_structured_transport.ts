#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::structured-transport.
//
// Gateway owns the canonical AgentRuntimeStructuredTurn payload. Adapters may
// translate this private Gateway payload into provider-specific transport, but
// Shell does not construct it and Kernel does not depend on engine schemas.

'use strict';

function cleanText(value, maxLen = 200) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function cleanDisplayText(value, maxLen = 12000) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .trim()
    .slice(0, maxLen);
}

function cleanEngineId(value) {
  return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, '');
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function boundedObject(value) {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {};
}

function buildStructuredContextPackProjection(contextPack) {
  const pack = boundedObject(contextPack);
  const frontier = boundedObject(pack.frontier);
  const universalToolGrants = boundedObject(pack.universal_tool_grants);
  return {
    type: 'agent_runtime_structured_context_pack_projection',
    source_basis: cleanText(pack.source_basis || 'gateway_runtime_context_pack', 160),
    source_authority: cleanText(pack.source_authority || 'gateway.runtime.agent_runtime_turn_projection', 200),
    row_count: Number(pack.row_count || 0) || 0,
    fanout_target: Number(pack.fanout_target || 0) || 0,
    fragment_count: asArray(pack.fragments).length,
    frontier: {
      hot_atom_refs: asArray(frontier.hot_atom_refs).map((item) => cleanText(item, 200)).filter(Boolean).slice(-14),
      warm_span_refs: asArray(frontier.warm_span_refs).map((item) => cleanText(item, 200)).filter(Boolean).slice(-14),
      cool_span_refs: asArray(frontier.cool_span_refs).map((item) => cleanText(item, 200)).filter(Boolean).slice(-14),
      cold_span_refs: asArray(frontier.cold_span_refs).map((item) => cleanText(item, 200)).filter(Boolean).slice(-14),
    },
    conversation_window: asArray(pack.conversation_window).slice(-14),
    conversation_summary_ref: pack.conversation_summary_ref || null,
    relevant_memory: asArray(pack.relevant_memory).slice(-12),
    artifact_refs: asArray(pack.artifact_refs).slice(0, 12),
    permission_scope: pack.permission_scope || null,
    context_budget: pack.context_budget || null,
    universal_tool_grants: {
      source_authority: cleanText(universalToolGrants.source_authority || 'gateway_universal_core_tools_policy', 200),
      tools: asArray(universalToolGrants.tools).slice(0, 8),
    },
    runtime_attachment_refs: pack.runtime_attachment_refs || null,
    runtime_steering: pack.runtime_steering || null,
    model_provider_context: pack.model_provider_context || null,
    active_workspace: pack.active_workspace || null,
  };
}

function buildAgentRuntimeStructuredTurn(input = {}) {
  const turnEnvelope = boundedObject(input.turnEnvelope);
  const contextPack = boundedObject(input.contextPack);
  const userMessage = boundedObject(turnEnvelope.user_message);
  const modelProviderContext = input.modelProviderContext && typeof input.modelProviderContext === 'object'
    ? input.modelProviderContext
    : (contextPack.model_provider_context && typeof contextPack.model_provider_context === 'object' ? contextPack.model_provider_context : null);
  const engineId = cleanEngineId(input.engineId || turnEnvelope.engine_id);
  const structured = {
    schema_version: 1,
    type: 'AgentRuntimeStructuredTurn',
    source_authority: 'gateway.runtime.agent_runtime_structured_transport',
    transport_mode: cleanText(input.transportMode || 'structured_json', 80),
    transport_target: cleanText(input.transportTarget || 'structured_json', 80),
    prompt_text_compatibility: {
      allowed: input.promptTextCompatibilityAllowed === true,
      source_authority: 'gateway.runtime.agent_runtime_structured_transport',
      rule: 'Prompt text may render from this structured turn only as a transitional compatibility layer.',
    },
    trace_id: cleanText(input.traceId || turnEnvelope.trace_id, 200),
    engine_id: engineId,
    agent_id: cleanText(input.agentId || turnEnvelope.agent_id, 160),
    session_id: cleanText(input.sessionId || turnEnvelope.session_id, 200),
    turn_id: cleanText(input.turnId || turnEnvelope.turn_id, 200),
    user_message: {
      type: 'agent_runtime_user_message',
      role: 'user',
      text: cleanDisplayText(userMessage.text || input.text || '', 12000),
      attachment_refs: asArray(userMessage.attachment_refs || input.attachmentRefs).slice(0, 12),
      attachment_count: asArray(userMessage.attachment_refs || input.attachmentRefs).length,
    },
    context_pack: buildStructuredContextPackProjection(contextPack),
    tool_grants: buildStructuredContextPackProjection(contextPack).universal_tool_grants,
    approval_requests: asArray(input.approvalRequests).slice(0, 8),
    activity_events: asArray(input.activityEvents).slice(-48),
    assistant_output: {
      type: 'agent_runtime_assistant_output_projection',
      status: cleanText(input.status || 'pending', 80),
      text: cleanDisplayText(input.assistantText || '', 12000),
      result_ref: cleanText(input.resultRef || '', 240),
    },
    receipts: {
      type: 'agent_runtime_receipt_refs_projection',
      receipt_refs: asArray(input.receiptRefs).map((item) => cleanText(item, 240)).filter(Boolean).slice(0, 12),
    },
    model_provider_context: modelProviderContext ? {
      ...modelProviderContext,
      secrets_included: false,
    } : null,
  };
  structured.structured_turn_ref = `agent-runtime-structured-turn/${structured.trace_id || 'trace'}/${structured.turn_id || 'turn'}`;
  return structured;
}

module.exports = {
  buildAgentRuntimeStructuredTurn,
};
