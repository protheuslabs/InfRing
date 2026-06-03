#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::universal-core-tools.
// Gateway-owned proposal contract for the tiny tool surface shared by native and
// external runtime engines. Engines may propose these calls; they may not execute
// them directly or treat tool proposals as Kernel authority.

'use strict';

const UNIVERSAL_CORE_TOOL_IDS = Object.freeze([
  'conversation.read',
  'memory.read',
  'memory.write_propose',
  'artifact.read',
  'artifact.create_propose',
  'permission.request',
]);

const TOOL_DEFINITIONS = Object.freeze({
  'conversation.read': {
    capability: 'read_conversation_projection',
    mode: 'gateway_read_projection',
    durable_effect: 'none',
    output_policy: 'bounded_rows_and_refs_only',
  },
  'memory.read': {
    capability: 'read_memory_projection',
    mode: 'gateway_to_kernel_memory_read',
    durable_effect: 'none',
    output_policy: 'bounded_summaries_and_memory_refs_only',
  },
  'memory.write_propose': {
    capability: 'propose_memory_write',
    mode: 'proposal_only',
    durable_effect: 'kernel_authorized_after_gateway_policy',
    output_policy: 'proposal_ref_and_receipt_ref_only',
  },
  'artifact.read': {
    capability: 'read_artifact_projection',
    mode: 'gateway_read_projection',
    durable_effect: 'none',
    output_policy: 'bounded_artifact_projection_only',
  },
  'artifact.create_propose': {
    capability: 'propose_artifact_create',
    mode: 'proposal_only',
    durable_effect: 'kernel_or_gateway_authorized_after_policy',
    output_policy: 'proposal_ref_and_receipt_ref_only',
  },
  'permission.request': {
    capability: 'request_permission',
    mode: 'proposal_only',
    durable_effect: 'permission_gate_only',
    output_policy: 'permission_request_ref_only',
  },
});

const FORBIDDEN_DEFAULT_FIELDS = new Set([
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

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function hasForbiddenDefaultField(value, seen = new Set()) {
  if (!value || typeof value !== 'object') return false;
  if (seen.has(value)) return false;
  seen.add(value);
  if (Array.isArray(value)) return value.some((item) => hasForbiddenDefaultField(item, seen));
  for (const key of Object.keys(value)) {
    if (FORBIDDEN_DEFAULT_FIELDS.has(key)) return true;
    if (hasForbiddenDefaultField(value[key], seen)) return true;
  }
  return false;
}

function grantRowsFor(toolIds) {
  return (Array.isArray(toolIds) && toolIds.length ? toolIds : UNIVERSAL_CORE_TOOL_IDS)
    .map((toolId) => cleanString(toolId, 120))
    .filter((toolId) => UNIVERSAL_CORE_TOOL_IDS.includes(toolId))
    .map((toolId) => ({
      tool_id: toolId,
      ...TOOL_DEFINITIONS[toolId],
      engine_may_execute_directly: false,
      gateway_validation_required: true,
      receipt_required_for_durable_effect: TOOL_DEFINITIONS[toolId].durable_effect !== 'none',
    }));
}

function buildUniversalToolGrants(options = {}) {
  const rows = grantRowsFor(options.toolIds);
  return {
    schema_version: 1,
    type: 'agent_runtime_universal_tool_grants',
    source_authority: 'gateway_universal_core_tools_policy',
    trace_id: cleanString(options.traceId, 200),
    session_id: cleanString(options.sessionId, 200),
    agent_id: cleanString(options.agentId, 160),
    engine_id: cleanString(options.engineId, 120),
    proposal_shape: 'infring_universal_tool_proposal',
    proposal_only: true,
    native_workflow_tools_exposed: false,
    tools: rows,
  };
}

function renderUniversalToolGrantPromptSection(grants) {
  const pack = grants && typeof grants === 'object' ? grants : null;
  const tools = Array.isArray(pack && pack.tools) ? pack.tools : [];
  if (!tools.length) return '';
  const lines = [
    'Universal InfRing core tools (proposal-only):',
    '- You may propose these tools when needed. Do not claim they executed.',
    '- Gateway validates proposals; Kernel/Memory/Artifact authority performs any durable effect and emits receipts.',
    '- Proposal JSON shape: {"type":"infring_universal_tool_proposal","tool_id":"memory.read","reason":"why needed","arguments":{}}',
  ];
  for (const tool of tools.slice(0, 12)) {
    lines.push(`- ${cleanString(tool.tool_id, 80)} mode=${cleanString(tool.mode, 80)} output=${cleanString(tool.output_policy, 120)}`);
  }
  return lines.join('\n');
}

function normalizeUniversalToolProposal(value, grants) {
  const proposal = value && typeof value === 'object' ? value : {};
  const toolId = cleanString(proposal.tool_id || proposal.capability, 120);
  const grantedIds = new Set((Array.isArray(grants && grants.tools) ? grants.tools : grantRowsFor()).map((row) => row.tool_id));
  if (proposal.type !== 'infring_universal_tool_proposal') {
    return { ok: false, error_code: 'universal_tool_proposal_type_invalid' };
  }
  if (!grantedIds.has(toolId)) {
    return { ok: false, error_code: 'universal_tool_not_granted', tool_id: toolId };
  }
  if (hasForbiddenDefaultField(proposal)) {
    return { ok: false, error_code: 'universal_tool_forbidden_default_field', tool_id: toolId };
  }
  return {
    ok: true,
    type: 'tool.proposed',
    tool_id: toolId,
    capability: TOOL_DEFINITIONS[toolId].capability,
    mode: TOOL_DEFINITIONS[toolId].mode,
    durable_effect: TOOL_DEFINITIONS[toolId].durable_effect,
    reason: cleanString(proposal.reason, 1000),
    arguments: proposal.arguments && typeof proposal.arguments === 'object' ? proposal.arguments : {},
    gateway_validation_required: true,
    engine_may_execute_directly: false,
  };
}

module.exports = {
  UNIVERSAL_CORE_TOOL_IDS,
  TOOL_DEFINITIONS,
  FORBIDDEN_DEFAULT_FIELDS,
  buildUniversalToolGrants,
  renderUniversalToolGrantPromptSection,
  normalizeUniversalToolProposal,
  hasForbiddenDefaultField,
};
