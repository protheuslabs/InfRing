#!/usr/bin/env node
/* eslint-disable no-console */

import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_claude_stream_json_mapping_probe_current.json';
const CLAUDE_COMMAND = process.env.INFRING_CLAUDE_CODE_BIN || process.env.INFRING_CLAUDE_CODE_PATH || 'claude';

function clean(value: any, max = 1000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function requireTs(rel: string): any {
  return require(path.join(ROOT, rel));
}

function hasNoSecretMaterial(value: any): boolean {
  return !/(api[_-]?key|secret|token|authorization)["']?\s*[:=]\s*["'][A-Za-z0-9_\-.]{12,}/i.test(JSON.stringify(value || {}));
}

function runClaudeHelp(): JsonObject {
  const result = childProcess.spawnSync(CLAUDE_COMMAND, ['--help'], {
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

function buildProbeStructuredTurn(): JsonObject {
  const { buildAgentRuntimeStructuredTurn } = requireTs('gateway/runtime/agent_runtime/agent_runtime_structured_transport.ts');
  return buildAgentRuntimeStructuredTurn({
    traceId: 'validation:agent-runtime:claude-stream-json-mapping',
    engineId: 'claude_code',
    agentId: 'agent-runtime-claude-stream-json-probe-agent',
    sessionId: 'agent-runtime-claude-stream-json-probe-session',
    turnId: 'agent-runtime-claude-stream-json-probe-turn',
    transportMode: 'structured_json',
    transportTarget: 'native_structured_context_api',
    promptTextCompatibilityAllowed: false,
    text: 'Summarize the structured turn without mutating files.',
    contextPack: {
      source_basis: 'validation_probe',
      source_authority: 'validation.agent_runtime_claude_stream_json_mapping_probe',
      row_count: 2,
      fanout_target: 7,
      fragments: [
        {
          kind: 'atom',
          ref_id: 'atom/user/claude-stream-json-probe/1',
          payload: {
            role: 'user',
            source_kind: 'user_message',
            text_preview: 'Claude stream-json mapping probe should preserve structured context refs.',
          },
        },
        {
          kind: 'atom',
          ref_id: 'atom/assistant/claude-stream-json-probe/2',
          payload: {
            role: 'assistant',
            source_kind: 'assistant_message',
            text_preview: 'The mapped stream should avoid raw prompt preamble transport.',
          },
        },
      ],
      frontier: {
        hot_atom_refs: [
          'atom/user/claude-stream-json-probe/1',
          'atom/assistant/claude-stream-json-probe/2',
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
    approvalRequests: [],
    activityEvents: [],
    receiptRefs: [],
  });
}

function buildCandidateStreamRecords(structuredTurn: JsonObject): JsonObject[] {
  return [
    {
      type: 'infring.agent_runtime.structured_turn',
      schema_version: 1,
      transport: 'claude_code_stream_json_candidate',
      source_authority: 'validation.agent_runtime_claude_stream_json_mapping_probe',
      structured_turn_ref: structuredTurn.structured_turn_ref,
      trace_id: structuredTurn.trace_id,
      engine_id: structuredTurn.engine_id,
      session_id: structuredTurn.session_id,
      turn_id: structuredTurn.turn_id,
      user_message: structuredTurn.user_message,
      context_pack: structuredTurn.context_pack,
      tool_grants: structuredTurn.tool_grants,
      approval_requests: structuredTurn.approval_requests,
      model_provider_context: structuredTurn.model_provider_context,
    },
    {
      type: 'user',
      message: {
        role: 'user',
        content: [
          {
            type: 'text',
            text: structuredTurn.user_message.text,
          },
        ],
      },
    },
  ];
}

function main() {
  const help = runClaudeHelp();
  const helpText = `${help.stdout}\n${help.stderr}`;
  const structuredTurn = buildProbeStructuredTurn();
  const candidateStreamRecords = buildCandidateStreamRecords(structuredTurn);
  const checks = {
    cli_help_available: help.status === 0 && !help.error_code,
    input_stream_json_flag_present: /--input-format[\s\S]+stream-json/.test(helpText),
    output_stream_json_flag_present: /--output-format[\s\S]+stream-json/.test(helpText),
    print_mode_present: /--print/.test(helpText),
    structured_turn_ref_present: !!structuredTurn.structured_turn_ref,
    prompt_text_compatibility_disabled: structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.allowed === false,
    stream_records_materialized: candidateStreamRecords.length >= 2,
    stream_records_preserve_context_pack: candidateStreamRecords[0]?.context_pack?.type === 'agent_runtime_structured_context_pack_projection',
    stream_records_preserve_tool_grants: Array.isArray(candidateStreamRecords[0]?.tool_grants?.tools) && candidateStreamRecords[0].tool_grants.tools.length >= 1,
    no_secret_material: hasNoSecretMaterial(candidateStreamRecords),
  };
  const mappingReady = Object.values(checks).every(Boolean);
  const report = {
    ok: mappingReady,
    type: 'agent_runtime_claude_stream_json_mapping_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'dry_run_mapping_probe_no_runtime_submission',
    engine_id: 'claude_code',
    command: CLAUDE_COMMAND,
    checks,
    native_transport_probe_status: mappingReady ? 'candidate_mapping_ready' : 'candidate_mapping_failed',
    typed_turn_api_available: false,
    submitted_to_runtime: false,
    evidence: mappingReady
      ? 'Claude Code advertises stream-json input/output locally, and InfRing can materialize an AgentRuntimeStructuredTurn-derived candidate stream without prompt compatibility enabled. This is not yet live acceptance proof.'
      : 'Claude Code stream-json candidate mapping is incomplete.',
    next_action: 'Run a disposable live Claude Code stream-json acceptance probe that submits the candidate records and verifies structured activity/output without prompt preamble parsing.',
    candidate_record_count: candidateStreamRecords.length,
    candidate_record_types: candidateStreamRecords.map((row) => clean(row.type, 120)),
    structured_turn_ref: structuredTurn.structured_turn_ref,
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
