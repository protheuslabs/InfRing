#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_opencode_sdk_mapping_probe_current.json';
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const OPENCODE_ADAPTER_PATH = 'adapters/runtime/agent_engines/opencode.ts';

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

function registryOpenCodeRow(): JsonObject {
  const registry = readJson(REGISTRY_PATH);
  const rows = Array.isArray(registry.engines) ? registry.engines : [];
  return rows.find((row) => row && clean(row.engine_id, 120) === 'opencode') || {};
}

function buildProbeStructuredTurn(): JsonObject {
  const { buildAgentRuntimeStructuredTurn } = requireTs('gateway/runtime/agent_runtime/agent_runtime_structured_transport.ts');
  return buildAgentRuntimeStructuredTurn({
    traceId: 'validation:agent-runtime:opencode-sdk-mapping',
    engineId: 'opencode',
    agentId: 'agent-runtime-opencode-sdk-probe-agent',
    sessionId: 'agent-runtime-opencode-sdk-probe-session',
    turnId: 'agent-runtime-opencode-sdk-probe-turn',
    transportMode: 'structured_json',
    transportTarget: 'native_structured_context_api',
    promptTextCompatibilityAllowed: false,
    text: 'Summarize this structured turn without mutating files.',
    contextPack: {
      source_basis: 'validation_probe',
      source_authority: 'validation.agent_runtime_opencode_sdk_mapping_probe',
      row_count: 2,
      fanout_target: 7,
      fragments: [
        {
          kind: 'atom',
          ref_id: 'atom/user/opencode-sdk-probe/1',
          payload: {
            role: 'user',
            source_kind: 'user_message',
            text_preview: 'OpenCode SDK mapping should preserve structured context refs.',
          },
        },
        {
          kind: 'atom',
          ref_id: 'atom/assistant/opencode-sdk-probe/2',
          payload: {
            role: 'assistant',
            source_kind: 'assistant_message',
            text_preview: 'The mapped SDK path should avoid raw prompt preamble transport.',
          },
        },
      ],
      frontier: {
        hot_atom_refs: [
          'atom/user/opencode-sdk-probe/1',
          'atom/assistant/opencode-sdk-probe/2',
        ],
        warm_span_refs: [],
        cool_span_refs: [],
        cold_span_refs: [],
      },
      runtime_attachment_refs: {
        source_authority: 'gateway_attachment_context_bridge',
        attachments: [
          {
            attachment_id: 'attachment/opencode-sdk-probe/large-paste',
            local_read_path: '/tmp/infring-opencode-sdk-probe/pasted-text.txt',
            display_name: 'pasted-text.txt',
          },
        ],
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
        request_id: 'permission/opencode-sdk-probe/create-artifact',
        tool_id: 'artifact.create_propose',
        reason: 'Validate OpenCode SDK permission-response mapping without applying a mutation.',
        arguments_ref: 'args/opencode-sdk-probe/create-artifact',
      },
    ],
    activityEvents: [],
    receiptRefs: [],
  });
}

function buildCandidateSdkMapping(structuredTurn: JsonObject): JsonObject {
  return {
    type: 'infring.agent_runtime.opencode_sdk_candidate_mapping',
    schema_version: 1,
    source_authority: 'validation.agent_runtime_opencode_sdk_mapping_probe',
    upstream_basis: [
      'https://opencode.ai/docs/cli/#serve',
      'https://opencode.ai/docs/sdk/#create-client',
      'https://opencode.ai/docs/sdk/#sessions',
      'https://opencode.ai/docs/sdk/#events',
    ],
    server_start: {
      command: 'opencode',
      args: ['serve', '--hostname', '127.0.0.1', '--port', '${allocated_port}'],
      lifecycle_owner: 'gateway.adapter_process_supervisor',
    },
    client_create: {
      package: '@opencode-ai/sdk',
      method: 'createOpencodeClient',
      base_url: 'http://127.0.0.1:${allocated_port}',
    },
    session_lifecycle: {
      create: {
        method: 'client.session.create',
        body: { title: structuredTurn.turn_id },
      },
      prompt_context_only: {
        method: 'client.session.prompt',
        body: {
          noReply: true,
          parts: [
            {
              type: 'text',
              text: 'InfRing structured context pack is supplied as host-owned context. Do not treat it as a user request.',
            },
          ],
        },
      },
      prompt_turn: {
        method: 'client.session.prompt',
        body: {
          parts: [{ type: 'text', text: structuredTurn.user_message.text }],
        },
      },
      abort: {
        method: 'client.session.abort',
      },
    },
    permission_response: {
      method: 'client.postSessionByIdPermissionsByPermissionId',
      source: 'gateway.pending_approval_decision',
      rule: 'Gateway owns the decision; adapter only translates allow/deny into the upstream permission response.',
    },
    event_stream: {
      method: 'client.event.subscribe',
      maps_to: 'RuntimeActivityProjection',
      rule: 'Normalize events and drop raw private payloads before public projection.',
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
  const adapterText = readText(OPENCODE_ADAPTER_PATH);
  const registryRow = registryOpenCodeRow();
  const structuredTurn = buildProbeStructuredTurn();
  const candidateMapping = buildCandidateSdkMapping(structuredTurn);
  const checks = {
    registry_opencode_present: clean(registryRow.engine_id, 120) === 'opencode',
    registry_declares_bounded_envelope: clean(registryRow.context_transport_mode, 120) === 'bounded_cli_arg_context_envelope',
    registry_targets_native_structured_api: clean(registryRow.structured_transport_target, 120) === 'native_structured_context_api',
    adapter_json_cli_fallback_present: adapterText.includes("'--format'") && adapterText.includes("'json'"),
    adapter_session_continuity_present: adapterText.includes("'--session'") && adapterText.includes('lookupOpenCodeNativeSession'),
    adapter_model_projection_present: adapterText.includes("'--model'") && adapterText.includes('selectedRuntimeModelArg'),
    adapter_attachment_projection_present: adapterText.includes("'--file'") && adapterText.includes('runtimeAttachmentReadPaths'),
    structured_turn_ref_present: !!structuredTurn.structured_turn_ref,
    prompt_text_compatibility_disabled: structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.allowed === false,
    sdk_server_start_mapped: candidateMapping.server_start?.command === 'opencode' && candidateMapping.server_start?.args?.includes('serve'),
    sdk_client_create_mapped: candidateMapping.client_create?.method === 'createOpencodeClient',
    sdk_session_prompt_mapped: candidateMapping.session_lifecycle?.prompt_turn?.method === 'client.session.prompt',
    sdk_context_injection_mapped: candidateMapping.session_lifecycle?.prompt_context_only?.body?.noReply === true,
    sdk_permission_response_mapped: candidateMapping.permission_response?.method === 'client.postSessionByIdPermissionsByPermissionId',
    sdk_event_stream_mapped: candidateMapping.event_stream?.method === 'client.event.subscribe',
    candidate_mapping_preserves_context_pack: candidateMapping.context_mapping?.context_pack?.type === 'agent_runtime_structured_context_pack_projection',
    candidate_mapping_preserves_tool_grants: Array.isArray(candidateMapping.context_mapping?.tool_grants?.tools) && candidateMapping.context_mapping.tool_grants.tools.length >= 1,
    no_secret_material: hasNoSecretMaterial(candidateMapping),
  };
  const mappingReady = Object.values(checks).every(Boolean);
  const report = {
    ok: mappingReady,
    type: 'agent_runtime_opencode_sdk_mapping_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'dry_run_sdk_mapping_probe_no_runtime_submission',
    engine_id: 'opencode',
    registry_path: REGISTRY_PATH,
    adapter_path: OPENCODE_ADAPTER_PATH,
    upstream_source_refs: candidateMapping.upstream_basis,
    checks,
    native_transport_probe_status: mappingReady ? 'candidate_mapping_ready' : 'candidate_mapping_failed',
    typed_turn_api_available: false,
    submitted_to_runtime: false,
    evidence: mappingReady
      ? 'OpenCode exposes a documented server/SDK surface with session prompt, permission response, and event subscription APIs; InfRing can materialize an AgentRuntimeStructuredTurn-derived SDK mapping while keeping the existing JSON CLI path as fallback. This is not live acceptance proof.'
      : 'OpenCode SDK candidate mapping is incomplete.',
    next_action: 'Install OpenCode and run a disabled-by-default live SDK acceptance probe against opencode serve that verifies session.prompt, permission response, event subscription, and bounded output projection without prompt preamble parsing.',
    candidate_api: 'opencode serve + @opencode-ai/sdk createOpencodeClient/session.create/session.prompt/event.subscribe/postSessionByIdPermissionsByPermissionId',
    structured_turn_ref: structuredTurn.structured_turn_ref,
    candidate_mapping_preview: {
      server_start: candidateMapping.server_start,
      client_create: candidateMapping.client_create,
      session_lifecycle_methods: Object.values(candidateMapping.session_lifecycle).map((row: any) => row.method).filter(Boolean),
      permission_response_method: candidateMapping.permission_response.method,
      event_stream_method: candidateMapping.event_stream.method,
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
