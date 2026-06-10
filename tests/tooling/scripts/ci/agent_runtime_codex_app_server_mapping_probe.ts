#!/usr/bin/env node
/* eslint-disable no-console */

import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_codex_app_server_mapping_probe_current.json';
const SCRATCH_DIR = 'core/local/artifacts/agent-runtime-codex-app-server-schema-probe';
const CODEX_COMMAND = process.env.INFRING_CODEX_CLI_BIN || process.env.INFRING_CODEX_CLI_PATH || 'codex';

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

function runCodex(args: string[]): JsonObject {
  const result = childProcess.spawnSync(CODEX_COMMAND, args, {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 15000,
    maxBuffer: 2 * 1024 * 1024,
  });
  return {
    status: result.status,
    error_code: result.error && (result.error as any).code ? String((result.error as any).code) : '',
    stdout: result.stdout || '',
    stderr: result.stderr || '',
  };
}

function walkFiles(absDir: string): string[] {
  const out: string[] = [];
  function walk(current: string) {
    let entries: fs.Dirent[] = [];
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const abs = path.join(current, entry.name);
      if (entry.isDirectory()) walk(abs);
      else out.push(abs);
    }
  }
  walk(absDir);
  return out;
}

function extractSchemaMethods(schemaFiles: string[]): JsonObject {
  const clientMethods: string[] = [];
  const serverMethods: string[] = [];
  const titles: string[] = [];
  for (const file of schemaFiles) {
    let parsed: JsonObject = {};
    try {
      parsed = JSON.parse(fs.readFileSync(file, 'utf8'));
    } catch {
      continue;
    }
    if (typeof parsed.title === 'string') titles.push(parsed.title);
    const target =
      parsed.title === 'ClientRequest'
        ? clientMethods
        : parsed.title === 'ServerRequest'
          ? serverMethods
          : null;
    if (!target || !Array.isArray(parsed.oneOf)) continue;
    for (const variant of parsed.oneOf) {
      const methodEnum = variant?.properties?.method?.enum;
      if (Array.isArray(methodEnum)) {
        for (const method of methodEnum) {
          if (typeof method === 'string' && !target.includes(method)) target.push(method);
        }
      }
    }
  }
  const turnCandidates = clientMethods.filter((method) => /(^|\/)turn(\/|$)|message|input|send/i.test(method));
  const sessionCandidates = clientMethods.filter((method) => /thread|session|conversation/i.test(method));
  const approvalCallbacks = serverMethods.filter((method) => /approval|elicitation|requestUserInput|tool\/call/i.test(method));
  return {
    client_methods: clientMethods.sort(),
    server_methods: serverMethods.sort(),
    schema_titles: titles.sort(),
    candidate_methods: {
      turn_submission: turnCandidates.sort(),
      session_lifecycle: sessionCandidates.sort(),
      approval_callbacks: approvalCallbacks.sort(),
    },
    selected_live_acceptance_path: {
      status: turnCandidates.length > 0 && sessionCandidates.length > 0 ? 'candidate_method_surface_identified' : 'candidate_method_surface_incomplete',
      session_method: sessionCandidates.includes('thread/start') ? 'thread/start' : sessionCandidates[0] || '',
      turn_method: turnCandidates.includes('turn/start') ? 'turn/start' : turnCandidates[0] || '',
      approval_callback_methods: approvalCallbacks,
      note: 'Selection is schema-derived only; it is not live app-server acceptance proof.',
    },
  };
}

function buildProbeStructuredTurn(): JsonObject {
  const { buildAgentRuntimeStructuredTurn } = requireTs('gateway/runtime/agent_runtime/agent_runtime_structured_transport.ts');
  return buildAgentRuntimeStructuredTurn({
    traceId: 'validation:agent-runtime:codex-app-server-mapping',
    engineId: 'codex_cli',
    agentId: 'agent-runtime-codex-app-server-probe-agent',
    sessionId: 'agent-runtime-codex-app-server-probe-session',
    turnId: 'agent-runtime-codex-app-server-probe-turn',
    transportMode: 'structured_json',
    transportTarget: 'native_structured_context_api',
    promptTextCompatibilityAllowed: false,
    text: 'Summarize the structured turn without mutating files.',
    contextPack: {
      source_basis: 'validation_probe',
      source_authority: 'validation.agent_runtime_codex_app_server_mapping_probe',
      row_count: 2,
      fanout_target: 7,
      fragments: [
        {
          kind: 'atom',
          ref_id: 'atom/user/codex-app-server-probe/1',
          payload: {
            role: 'user',
            source_kind: 'user_message',
            text_preview: 'Codex app-server mapping probe should preserve structured context refs.',
          },
        },
        {
          kind: 'atom',
          ref_id: 'atom/assistant/codex-app-server-probe/2',
          payload: {
            role: 'assistant',
            source_kind: 'assistant_message',
            text_preview: 'The mapped app-server candidate should avoid prompt/stdin preamble transport.',
          },
        },
      ],
      frontier: {
        hot_atom_refs: [
          'atom/user/codex-app-server-probe/1',
          'atom/assistant/codex-app-server-probe/2',
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

function main() {
  const scratchAbs = path.join(ROOT, SCRATCH_DIR);
  try { fs.rmSync(scratchAbs, { recursive: true, force: true }); } catch {}
  fs.mkdirSync(scratchAbs, { recursive: true });

  const help = runCodex(['app-server', 'generate-json-schema', '--help']);
  const schemaGeneration = runCodex(['app-server', 'generate-json-schema', '--experimental', '--out', scratchAbs]);
  const files = walkFiles(scratchAbs);
  const schemaFiles = files.filter((file) => /\.json$/i.test(file));
  const schemaMethods = extractSchemaMethods(schemaFiles);
  const schemaText = schemaFiles.slice(0, 20).map((file) => {
    try { return fs.readFileSync(file, 'utf8').slice(0, 20000); } catch { return ''; }
  }).join('\n');
  const structuredTurn = buildProbeStructuredTurn();
  const candidateEnvelope = {
    type: 'infring.agent_runtime.codex_app_server_candidate',
    schema_version: 1,
    source_authority: 'validation.agent_runtime_codex_app_server_mapping_probe',
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
  };
  const checks = {
    cli_help_available: help.status === 0 && !help.error_code,
    schema_generation_available: schemaGeneration.status === 0 && !schemaGeneration.error_code,
    schema_files_generated: schemaFiles.length > 0,
    app_server_protocol_terms_present: /(app[-_ ]?server|session|thread|message|method|request|response|event)/i.test(schemaText),
    candidate_method_surface_identified: schemaMethods.selected_live_acceptance_path?.status === 'candidate_method_surface_identified',
    approval_callback_surface_identified: Array.isArray(schemaMethods.candidate_methods?.approval_callbacks) && schemaMethods.candidate_methods.approval_callbacks.length >= 1,
    structured_turn_ref_present: !!structuredTurn.structured_turn_ref,
    prompt_text_compatibility_disabled: structuredTurn.prompt_text_compatibility && structuredTurn.prompt_text_compatibility.allowed === false,
    candidate_envelope_preserves_context_pack: candidateEnvelope.context_pack?.type === 'agent_runtime_structured_context_pack_projection',
    candidate_envelope_preserves_tool_grants: Array.isArray(candidateEnvelope.tool_grants?.tools) && candidateEnvelope.tool_grants.tools.length >= 1,
    no_secret_material: hasNoSecretMaterial({ candidateEnvelope, schemaText }),
  };
  const mappingReady = Object.values(checks).every(Boolean);
  const report = {
    ok: mappingReady,
    type: 'agent_runtime_codex_app_server_mapping_probe',
    generated_at: new Date().toISOString(),
    source_domain: 'validation',
    owner_domain: 'validation.agent_runtime',
    layer: 'gateway',
    mode: 'dry_run_schema_mapping_probe_no_runtime_submission',
    engine_id: 'codex_cli',
    command: CODEX_COMMAND,
    checks,
    native_transport_probe_status: mappingReady ? 'candidate_mapping_ready' : 'candidate_mapping_failed',
    typed_turn_api_available: false,
    submitted_to_runtime: false,
    evidence: mappingReady
      ? 'Codex app-server can generate a local protocol schema bundle, and InfRing can materialize an AgentRuntimeStructuredTurn-derived candidate envelope without prompt compatibility enabled. This is not yet live acceptance proof.'
      : 'Codex app-server candidate mapping is incomplete.',
    next_action: 'Inspect generated Codex app-server schema methods and build a disabled-by-default live acceptance probe for direct AgentRuntimeStructuredTurn submission without prompt/stdin rendering.',
    schema_file_count: schemaFiles.length,
    schema_file_sample: schemaFiles.slice(0, 12).map((file) => path.relative(ROOT, file).replace(/\\/g, '/')),
    schema_methods: schemaMethods,
    structured_turn_ref: structuredTurn.structured_turn_ref,
    violations: Object.entries(checks)
      .filter(([, value]) => value !== true)
      .map(([kind]) => ({ kind })),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  try { fs.rmSync(scratchAbs, { recursive: true, force: true }); } catch {}
  if (!report.ok) process.exit(1);
}

main();
