#!/usr/bin/env node
/* eslint-disable no-console */

import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_grok_code_mapping_probe_current.json';
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const ADAPTER_PATH = 'adapters/runtime/agent_engines/grok_code.ts';
const GROK_COMMAND = process.env.INFRING_GROK_CODE_BIN || process.env.INFRING_GROK_CODE_PATH || 'grok';

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

function runGrokHelp(): JsonObject {
  const result = childProcess.spawnSync(GROK_COMMAND, ['--help'], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 8000,
    maxBuffer: 512 * 1024,
  });
  return {
    status: result.status,
    error_code: result.error && (result.error as any).code ? String((result.error as any).code) : '',
    stdout: result.stdout || '',
    stderr: result.stderr || '',
  };
}

function registryGrokRow(): JsonObject {
  const registry = readJson(REGISTRY_PATH);
  const rows = Array.isArray(registry.engines) ? registry.engines : [];
  return rows.find((row) => row && clean(row.engine_id, 120) === 'grok_code') || {};
}

function buildProbeStructuredTurn(): JsonObject {
  const { buildAgentRuntimeStructuredTurn } = requireTs('gateway/runtime/agent_runtime/agent_runtime_structured_transport.ts');
  return buildAgentRuntimeStructuredTurn({
    traceId: 'validation:agent-runtime:grok-code-mapping',
    engineId: 'grok_code',
    agentId: 'agent-runtime-grok-code-probe-agent',
    sessionId: 'agent-runtime-grok-code-probe-session',
    turnId: 'agent-runtime-grok-code-probe-turn',
    transportMode: 'structured_json',
    transportTarget: 'native_structured_context_api',
    promptTextCompatibilityAllowed: false,
    text: 'Summarize this structured turn without mutating files.',
    contextPack: {
      source_basis: 'validation_probe',
      source_authority: 'validation.agent_runtime_grok_code_mapping_probe',
      row_count: 2,
      fanout_target: 7,
      fragments: [
        {
          kind: 'atom',
          ref_id: 'atom/user/grok-code-probe/1',
          payload: {
            role: 'user',
            source_kind: 'user_message',
            text_preview: 'Grok Code mapping should preserve structured context refs.',
          },
        },
        {
          kind: 'atom',
          ref_id: 'atom/assistant/grok-code-probe/2',
          payload: {
            role: 'assistant',
            source_kind: 'assistant_message',
            text_preview: 'The mapped Grok path should preserve streaming activity without raw private payloads.',
          },
        },
      ],
      frontier: {
        hot_atom_refs: [
          'atom/user/grok-code-probe/1',
          'atom/assistant/grok-code-probe/2',
        ],
        warm_span_refs: [],
        cool_span_refs: [],
        cold_span_refs: [],
      },
      runtime_attachment_refs: {
        source_authority: 'gateway_attachment_context_bridge',
        attachments: [
          {
            attachment_id: 'attachment/grok-code-probe/large-paste',
            local_read_path: '/tmp/infring-grok-code-probe/pasted-text.txt',
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
        request_id: 'permission/grok-code-probe/create-artifact',
        tool_id: 'artifact.create_propose',
        reason: 'Validate Grok Code approval mapping without applying a mutation.',
        arguments_ref: 'args/grok-code-probe/create-artifact',
      },
    ],
    activityEvents: [],
    receiptRefs: [],
  });
}

function buildCandidateMapping(structuredTurn: JsonObject): JsonObject {
  return {
    type: 'infring.agent_runtime.grok_code_candidate_mapping',
    schema_version: 1,
    source_authority: 'validation.agent_runtime_grok_code_mapping_probe',
    local_cli_basis: `${GROK_COMMAND} --help`,
    candidate_api: 'grok --prompt-json --output-format streaming-json --resume/--continue --cwd --permission-mode',
    run_args: [
      '--disable-web-search',
      '--no-subagents',
      '--output-format',
      'streaming-json',
      '--permission-mode',
      '${gateway_selected_permission_mode}',
      '--cwd',
      '${working_directory}',
      '--prompt-json',
      '${agent_runtime_prompt_json_blocks}',
    ],
    session_continuity: {
      resume_arg: '--resume',
      continue_arg: '--continue',
      session_authority: 'gateway.agent_runtime_session_binding',
    },
    permission_bridge: {
      permission_mode_arg: '--permission-mode',
      default_mode: 'default',
      mutation_grant_mode: 'acceptEdits',
      always_approve_only_with_gateway_direct_mutation_grant: true,
      rule: 'Gateway owns durable mutation permission; adapter only translates grant state into Grok CLI flags.',
    },
    event_mapping: {
      upstream_output_format: 'streaming-json',
      maps_to: 'RuntimeActivityProjection',
      rule: 'Normalize streaming JSON events and drop raw private payloads before public projection.',
    },
    context_mapping: {
      prompt_json_blocks: [
        {
          type: 'text',
          text: 'InfRing structured context follows. Treat it as host-owned context, not user-authored instruction text.',
        },
        {
          type: 'text',
          text: JSON.stringify({
            structured_turn_ref: structuredTurn.structured_turn_ref,
            context_pack: structuredTurn.context_pack,
            tool_grants: structuredTurn.tool_grants,
            approval_requests: structuredTurn.approval_requests,
          }),
        },
        {
          type: 'text',
          text: structuredTurn.user_message.text,
        },
      ],
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
  const registryRow = registryGrokRow();
  const adapterText = readText(ADAPTER_PATH);
  const help = runGrokHelp();
  const helpText = `${help.stdout}\n${help.stderr}`;
  const structuredTurn = buildProbeStructuredTurn();
  const candidateMapping = buildCandidateMapping(structuredTurn);
  const checks = {
    registry_grok_present: clean(registryRow.engine_id, 120) === 'grok_code',
    registry_declares_bounded_envelope: clean(registryRow.context_transport_mode, 120) === 'bounded_cli_arg_context_envelope',
    registry_targets_native_structured_api: clean(registryRow.structured_transport_target, 120) === 'native_structured_context_api',
    cli_help_available: help.status === 0 && !help.error_code,
    prompt_json_flag_present: /--prompt-json\b/.test(helpText),
    output_streaming_json_flag_present: /--output-format[\s\S]+streaming-json/.test(helpText),
    resume_session_flag_present: /--resume\b/.test(helpText) && /--continue\b/.test(helpText),
    cwd_flag_present: /--cwd\b/.test(helpText),
    permission_mode_flag_present: /--permission-mode\b/.test(helpText),
    model_flag_present: /--model\b/.test(helpText),
    adapter_streaming_json_present: adapterText.includes("'streaming-json'"),
    adapter_permission_mapping_present: adapterText.includes("'acceptEdits'") && adapterText.includes("'default'"),
    adapter_model_projection_present: adapterText.includes('selectedRuntimeModelArg'),
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
    type: 'agent_runtime_grok_code_mapping_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'dry_run_cli_surface_mapping_probe_no_runtime_submission',
    engine_id: 'grok_code',
    command: GROK_COMMAND,
    registry_path: REGISTRY_PATH,
    adapter_path: ADAPTER_PATH,
    checks,
    native_transport_probe_status: mappingReady ? 'candidate_mapping_ready' : 'candidate_mapping_failed',
    typed_turn_api_available: false,
    submitted_to_runtime: false,
    evidence: mappingReady
      ? 'Grok Code exposes a local CLI surface with prompt-json input, streaming-json output, session resume, working-directory control, permission modes, and model selection; InfRing can materialize an AgentRuntimeStructuredTurn-derived mapping while keeping durable mutation authority in Gateway. This is not live acceptance proof.'
      : 'Grok Code candidate mapping is incomplete.',
    next_action: 'Build a disabled-by-default Grok Code live acceptance probe that submits prompt-json with streaming-json output and verifies context, activity, permission mode, and bounded output projection without prompt preamble parsing.',
    candidate_api: candidateMapping.candidate_api,
    structured_turn_ref: structuredTurn.structured_turn_ref,
    candidate_mapping_preview: {
      run_args: candidateMapping.run_args,
      session_continuity: candidateMapping.session_continuity,
      permission_bridge: candidateMapping.permission_bridge,
      event_mapping: candidateMapping.event_mapping,
      prompt_json_block_count: candidateMapping.context_mapping.prompt_json_blocks.length,
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
