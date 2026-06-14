#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_hermes_agent_mapping_probe_current.json';
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const PRIVATE_SCHEMA_PATH = 'validation/conformance/contracts/agent_runtime_private_schemas/hermes_agent.json';
const ADAPTER_README_PATH = 'adapters/runtime/agent_engines/hermes_agent/README.md';
const REFERENCE_README_PATH = 'references/coding-agent-systems/hermes-agent/README.md';

function clean(value: any, max = 1000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function readText(rel: string): string {
  try {
    return fs.readFileSync(path.join(ROOT, rel), 'utf8');
  } catch {
    return '';
  }
}

function readJson(rel: string): JsonObject {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
  } catch {
    return {};
  }
}

function requireTs(rel: string): any {
  return require(path.join(ROOT, rel));
}

function hasNoSecretMaterial(value: any): boolean {
  return !/(api[_-]?key|secret|token|authorization)["']?\s*[:=]\s*["'][A-Za-z0-9_\-.]{12,}/i.test(JSON.stringify(value || {}));
}

function registryHermesRow(): JsonObject {
  const registry = readJson(REGISTRY_PATH);
  const rows = Array.isArray(registry.engines) ? registry.engines : [];
  return rows.find((row) => row && clean(row.engine_id, 120) === 'hermes_agent') || {};
}

function turnSubmissionCandidates(schema: JsonObject): JsonObject[] {
  const rows = schema?.structured_turn_mapping?.turn_submission_candidates;
  return Array.isArray(rows) ? rows : [];
}

function candidateByKind(schema: JsonObject, kind: string): JsonObject {
  return turnSubmissionCandidates(schema).find((row) => clean(row?.kind, 160) === kind) || {};
}

function buildProbeStructuredTurn(): JsonObject {
  const { buildAgentRuntimeStructuredTurn } = requireTs('gateway/runtime/agent_runtime/agent_runtime_structured_transport.ts');
  return buildAgentRuntimeStructuredTurn({
    traceId: 'validation:agent-runtime:hermes-agent-mapping',
    engineId: 'hermes_agent',
    agentId: 'agent-runtime-hermes-agent-probe-agent',
    sessionId: 'agent-runtime-hermes-agent-probe-session',
    turnId: 'agent-runtime-hermes-agent-probe-turn',
    transportMode: 'structured_json',
    transportTarget: 'native_structured_context_api',
    promptTextCompatibilityAllowed: false,
    text: 'Summarize this structured turn without mutating files.',
    contextPack: {
      source_basis: 'validation_probe',
      source_authority: 'validation.agent_runtime_hermes_agent_mapping_probe',
      row_count: 2,
      fanout_target: 7,
      fragments: [
        {
          kind: 'atom',
          ref_id: 'atom/user/hermes-agent-probe/1',
          payload: {
            role: 'user',
            source_kind: 'user_message',
            text_preview: 'Hermes mapping should preserve structured context refs.',
          },
        },
        {
          kind: 'atom',
          ref_id: 'atom/assistant/hermes-agent-probe/2',
          payload: {
            role: 'assistant',
            source_kind: 'assistant_message',
            text_preview: 'The mapped Hermes bridge must not claim mutating work without Gateway receipts.',
          },
        },
      ],
      frontier: {
        hot_atom_refs: [
          'atom/user/hermes-agent-probe/1',
          'atom/assistant/hermes-agent-probe/2',
        ],
        warm_span_refs: [],
        cool_span_refs: [],
        cold_span_refs: [],
      },
      universal_tool_grants: {
        source_authority: 'gateway_universal_core_tools_policy',
        tools: [
          {
            tool_id: 'conversation.read',
            default_policy: 'allowed_read_only',
          },
          {
            tool_id: 'artifact.create_propose',
            default_policy: 'requires_approval',
          },
        ],
      },
    },
    approvalRequests: [
      {
        request_id: 'permission/hermes-agent-probe/create-artifact',
        tool_id: 'artifact.create_propose',
        reason: 'Validate Hermes Agent mediated approval mapping without applying a mutation.',
        arguments_ref: 'args/hermes-agent-probe/create-artifact',
      },
    ],
    activityEvents: [],
    receiptRefs: [],
  });
}

function buildCandidateMapping(schema: JsonObject, structuredTurn: JsonObject): JsonObject {
  const safeCli = candidateByKind(schema, 'safe_cli_oneshot_bridge');
  const dashboard = candidateByKind(schema, 'dashboard_api_session_bridge');
  return {
    type: 'infring.agent_runtime.hermes_agent_candidate_mapping',
    schema_version: 1,
    source_authority: 'validation.agent_runtime_hermes_agent_mapping_probe',
    private_schema_ref: PRIVATE_SCHEMA_PATH,
    reference_repo_ref: 'references/coding-agent-systems/hermes-agent',
    current_safe_bridge: {
      kind: 'safe_cli_oneshot_bridge',
      command_shape: clean(safeCli.command_shape || 'hermes --toolsets safe -z <bounded InfRing context prompt>', 500),
      status: clean(safeCli.status || 'active_safe_non_mutating_bridge', 200),
      constraints: Array.isArray(safeCli.constraints) ? safeCli.constraints.map((item: any) => clean(item, 300)) : [],
      mutation_policy: 'non_mutating_only_until_mediated_approval_bridge',
    },
    target_session_bridge: {
      kind: 'dashboard_api_session_bridge',
      status: clean(dashboard.status || 'future_mediated_bridge_required_for_mutating_tools', 240),
      source_hint: clean(dashboard.source_hint || 'Hermes dashboard API session/message bridge', 600),
      health_candidates: schema?.discovery_schema?.health_probe_candidates || [],
      websocket_auth: 'ticket_or_token_mediated_private_adapter_state_only',
    },
    approval_bridge: {
      status: clean(schema?.approval_and_tool_policy?.mutating_tool_bridge_status || 'blocked_until_mediated_approval_bridge', 240),
      gateway_pause_required: true,
      durable_effects_require_receipts: schema?.approval_and_tool_policy?.durable_effects_require_gateway_or_kernel_receipts === true,
      forbidden_safe_cli_toolsets: schema?.approval_and_tool_policy?.safe_cli_bridge_forbidden_toolsets || [],
      rule: 'Hermes proposals may become InfRing approval requests; Hermes must not directly execute mutating universal tools through the safe oneshot bridge.',
    },
    event_mapping: {
      normalized_outputs: schema?.structured_turn_mapping?.required_normalized_outputs || [],
      forbidden_default_outputs: schema?.structured_turn_mapping?.forbidden_default_outputs || [],
      public_projection: 'RuntimeActivityProjection',
    },
    context_mapping: {
      structured_turn_ref: structuredTurn.structured_turn_ref,
      trace_id: structuredTurn.trace_id,
      engine_id: structuredTurn.engine_id,
      session_id: structuredTurn.session_id,
      turn_id: structuredTurn.turn_id,
      context_pack: structuredTurn.context_pack,
      tool_grants: structuredTurn.tool_grants,
      approval_requests: structuredTurn.approval_requests,
      model_provider_context: structuredTurn.model_provider_context,
    },
  };
}

function main() {
  const registryRow = registryHermesRow();
  const schema = readJson(PRIVATE_SCHEMA_PATH);
  const adapterReadme = readText(ADAPTER_README_PATH);
  const referenceReadme = readText(REFERENCE_README_PATH);
  const structuredTurn = buildProbeStructuredTurn();
  const candidateMapping = buildCandidateMapping(schema, structuredTurn);
  const safeCli = candidateByKind(schema, 'safe_cli_oneshot_bridge');
  const dashboard = candidateByKind(schema, 'dashboard_api_session_bridge');
  const forbiddenToolsets = schema?.approval_and_tool_policy?.safe_cli_bridge_forbidden_toolsets || [];
  const normalizedOutputs = schema?.structured_turn_mapping?.required_normalized_outputs || [];
  const forbiddenOutputs = schema?.structured_turn_mapping?.forbidden_default_outputs || [];
  const checks = {
    registry_hermes_present: clean(registryRow.engine_id, 120) === 'hermes_agent',
    registry_declares_bounded_envelope: clean(registryRow.context_transport_mode, 120) === 'bounded_cli_arg_context_envelope',
    registry_targets_native_structured_api: clean(registryRow.structured_transport_target, 120) === 'native_structured_context_api',
    private_schema_present: clean(schema.type, 200) === 'agent_runtime_private_schema_contract',
    private_schema_safe_bridge_status: clean(schema.status, 200) === 'safe_cli_bridge_schema',
    safe_cli_candidate_declared: clean(safeCli.kind, 200) === 'safe_cli_oneshot_bridge',
    dashboard_session_candidate_declared: clean(dashboard.kind, 200) === 'dashboard_api_session_bridge',
    adapter_readme_declares_gateway_only: /canonical Gateway Agent Runtime Socket/i.test(adapterReadme),
    adapter_readme_declares_non_mutating_safe_bridge: /safe CLI bridge/i.test(adapterReadme) && /mutating work must wait/i.test(adapterReadme),
    reference_readme_documents_dashboard_or_gateway: /hermes dashboard|hermes gateway|python -m hermes_cli\.main web --no-open/i.test(referenceReadme),
    approval_bridge_blocks_mutation_until_mediated: clean(schema?.approval_and_tool_policy?.mutating_tool_bridge_status, 240) === 'blocked_until_mediated_approval_bridge',
    safe_cli_forbids_terminal_and_file_toolsets: forbiddenToolsets.includes('terminal') && forbiddenToolsets.includes('file'),
    required_normalized_outputs_declared: Array.isArray(normalizedOutputs) && normalizedOutputs.includes('permission.requested') && normalizedOutputs.includes('turn.completed'),
    forbidden_raw_outputs_declared: Array.isArray(forbiddenOutputs) && forbiddenOutputs.includes('raw_dashboard_payload') && forbiddenOutputs.includes('raw_oauth_cookie'),
    structured_turn_ref_present: !!structuredTurn.structured_turn_ref,
    prompt_text_compatibility_disabled: structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.allowed === false,
    candidate_mapping_preserves_context_pack: candidateMapping.context_mapping?.context_pack?.type === 'agent_runtime_structured_context_pack_projection',
    candidate_mapping_preserves_tool_grants: Array.isArray(candidateMapping.context_mapping?.tool_grants?.tools) && candidateMapping.context_mapping.tool_grants.tools.length >= 1,
    candidate_mapping_preserves_approval_requests: Array.isArray(candidateMapping.context_mapping?.approval_requests) && candidateMapping.context_mapping.approval_requests.length >= 1,
    no_secret_material: hasNoSecretMaterial(candidateMapping),
  };
  const mappingReady = Object.values(checks).every(Boolean);
  const report = {
    ok: mappingReady,
    type: 'agent_runtime_hermes_agent_mapping_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'dry_run_private_schema_mapping_probe_no_runtime_submission',
    engine_id: 'hermes_agent',
    registry_path: REGISTRY_PATH,
    private_schema_path: PRIVATE_SCHEMA_PATH,
    adapter_readme_path: ADAPTER_README_PATH,
    reference_readme_path: REFERENCE_README_PATH,
    checks,
    native_transport_probe_status: mappingReady ? 'candidate_mapping_ready' : 'candidate_mapping_failed',
    typed_turn_api_available: false,
    submitted_to_runtime: false,
    evidence: mappingReady
      ? 'Hermes Agent has a documented safe CLI bridge and private schema for a future mediated dashboard/session bridge; InfRing can materialize an AgentRuntimeStructuredTurn-derived mapping while keeping mutating tools blocked until Gateway approval mediation exists. This is not live acceptance proof.'
      : 'Hermes Agent candidate mapping is incomplete.',
    next_action: 'Build a disabled-by-default Hermes live acceptance probe that starts or connects to the dashboard/session bridge and proves context, activity, permission pause, and output projection without using the unsafe mutating oneshot toolsets.',
    candidate_api: 'safe CLI oneshot now; mediated Hermes dashboard/session bridge target before mutating work',
    structured_turn_ref: structuredTurn.structured_turn_ref,
    candidate_mapping_preview: {
      current_safe_bridge: candidateMapping.current_safe_bridge,
      target_session_bridge: candidateMapping.target_session_bridge,
      approval_bridge_status: candidateMapping.approval_bridge.status,
      normalized_output_count: candidateMapping.event_mapping.normalized_outputs.length,
      forbidden_default_output_count: candidateMapping.event_mapping.forbidden_default_outputs.length,
    },
    violations: Object.entries(checks)
      .filter(([, value]) => value !== true)
      .map(([kind]) => ({ kind })),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main();
