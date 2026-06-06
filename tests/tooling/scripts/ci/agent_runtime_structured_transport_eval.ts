#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

type JsonObject = Record<string, any>;

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/agent_runtime_structured_transport_eval_current.json';
const REQUIRED_SECTIONS = [
  'schema_version',
  'type',
  'source_authority',
  'transport_mode',
  'transport_target',
  'structured_turn_ref',
  'trace_id',
  'engine_id',
  'agent_id',
  'session_id',
  'turn_id',
  'user_message',
  'context_pack',
  'tool_grants',
  'approval_requests',
  'activity_events',
  'assistant_output',
  'receipts',
];

function clean(value: any, max = 500): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(rel: string) {
  fs.mkdirSync(path.dirname(path.join(ROOT, rel)), { recursive: true });
}

function requireTs(rel: string): any {
  return require(path.join(ROOT, rel));
}

function contextPack(traceId: string, sessionId: string, agentId: string): JsonObject {
  return {
    source_basis: 'structured_transport_eval',
    source_authority: 'assurance.validation.agent_runtime_structured_transport_eval',
    trace_id: traceId,
    session_id: sessionId,
    agent_id: agentId,
    row_count: 2,
    fanout_target: 7,
    fragments: [
      {
        kind: 'atom',
        ref_id: 'atom/user/1',
        payload: {
          role: 'user',
          speaker_label: 'user',
          source_kind: 'user_message',
          sequence_no: 1,
          text_preview: 'Remember that the structured transport token is brass otter.',
        },
      },
      {
        kind: 'atom',
        ref_id: 'atom/assistant/2',
        payload: {
          role: 'assistant',
          speaker_label: 'assistant',
          source_kind: 'assistant_message',
          sequence_no: 2,
          text_preview: 'I will preserve that token across runtime engines.',
        },
      },
    ],
    frontier: {
      hot_atom_refs: ['atom/user/1', 'atom/assistant/2'],
      warm_span_refs: [],
      cool_span_refs: [],
      cold_span_refs: [],
    },
    universal_tool_grants: {
      source_authority: 'assurance.validation.agent_runtime_structured_transport_eval',
      tools: [
        {
          tool_id: 'conversation.read',
          default_policy: 'allowed_read_only',
        },
      ],
    },
  };
}

function hasNoSecrets(value: any): boolean {
  const text = JSON.stringify(value || {});
  return !/(api[_-]?key|secret|token)["']?\s*[:=]\s*["'][A-Za-z0-9_\-.]{12,}/i.test(text);
}

async function main() {
  let capturedMessage: JsonObject | null = null;
  const traceId = `trace-structured-transport-${Date.now()}`;
  const sessionId = 'structured-transport-eval-session';
  const agentId = 'structured-transport-eval-agent';
  const engineId = 'codex_cli';
  const { createAgentRuntimeTurnProjectionStore } = requireTs('gateway/runtime/agent_runtime/agent_runtime_turn_projection.ts');
  const store = createAgentRuntimeTurnProjectionStore({
    root: ROOT,
    contextFanoutTarget: 7,
    loadAgentRuntimeEngineRegistry: () => ({
      engines: [
        {
          engine_id: engineId,
          display_name: 'Codex',
          engine_kind: 'external_cli_adapter',
          status: 'adapter_seam_ready',
          implementation_path: 'adapters/runtime/agent_engines/codex_cli.ts',
        },
      ],
    }),
    findAgentRuntimeEngine: (registry: JsonObject, id: string) => (registry.engines || []).find((row: JsonObject) => row.engine_id === id),
    loadAgentRuntimeWorkspace: () => ({ workspace_dir: ROOT, active_workspace: ROOT, display_label: 'workspace' }),
    createRouter: () => ({
      healthCheck: async () => ({ status: 'available', version_preview: 'structured eval' }),
      startSession: async () => ({ ok: true }),
      submitTurn: async (message: JsonObject) => {
        capturedMessage = message;
        return {
          type: 'turn.complete',
          status: 'completed',
          output_text: 'STRUCTURED_TRANSPORT_OK',
          output_preview: 'STRUCTURED_TRANSPORT_OK',
          activity_events: [
            {
              type: 'agent_activity_event',
              activity_kind: 'activity',
              provider_event_type: 'structured_transport.eval',
              display_text: 'Structured transport payload was available to the adapter.',
              status: 'completed',
            },
          ],
          structured_activity: true,
          activity_event_count: 1,
        };
      },
      streamTurn: async (message: JsonObject) => {
        capturedMessage = message;
        return { type: 'turn.complete', status: 'completed', output_text: 'STRUCTURED_TRANSPORT_OK' };
      },
    }),
    sanitizeAgentRuntimeActivityEvent: (event: JsonObject, index: number, defaults: JsonObject) => ({
      ...event,
      sequence_no: Number(event.sequence_no || index + 1) || index + 1,
      engine_id: event.engine_id || defaults.engineId,
      trace_id: event.trace_id || defaults.traceId,
      session_id: event.session_id || defaults.sessionId,
      turn_id: event.turn_id || defaults.turnId,
    }),
    appendAgentRuntimeTranscriptTurn: () => {},
    appendAgentRuntimeTurnAtoms: () => {},
    ingestAgentRuntimeContextProjection: () => {},
    loadAgentRuntimeContextRows: () => [],
    materializeKernelAgentRuntimeContextPack: async () => ({ ok: false, reason: 'eval_uses_gateway_context_fallback' }),
    materializeAgentRuntimeContextPack: () => contextPack(traceId, sessionId, agentId),
    buildAgentRuntimeContextPack: () => contextPack(traceId, sessionId, agentId),
    mergeAgentRuntimeApprovalPermissionPolicy: (policy: JsonObject) => policy || {},
    buildUniversalToolGrants: () => ({
      source_authority: 'assurance.validation.agent_runtime_structured_transport_eval',
      tools: [{ tool_id: 'conversation.read', default_policy: 'allowed_read_only' }],
    }),
    recordAgentRuntimeTurnReceipts: () => ({
      receipt_refs: [
        'receipt/agent-runtime/engine_selection/structured/turn',
        'receipt/agent-runtime/context_pack/structured/turn',
        'receipt/agent-runtime/final_response/structured/turn',
      ],
      receipt_count: 3,
    }),
  });
  const projection = await store.agentRuntimeTurnProjection(traceId, {
    engine_id: engineId,
    agent_id: agentId,
    session_id: sessionId,
    turn_id: 'structured-transport-turn',
    message: 'Reply with STRUCTURED_TRANSPORT_OK.',
    input_text: 'Reply with STRUCTURED_TRANSPORT_OK.',
    model_provider_context: {
      provider: 'openai',
      model: 'gpt-5.5',
      qualified_model_ref: 'openai/gpt-5.5',
      secrets_included: false,
    },
  });
  const structuredTurn = capturedMessage && capturedMessage.structured_turn ? capturedMessage.structured_turn : null;
  const missing = REQUIRED_SECTIONS.filter((field) => !structuredTurn || structuredTurn[field] == null);
  const promptCompatSource = !!(
    capturedMessage &&
    capturedMessage.context_pack &&
    capturedMessage.context_pack.structured_transport &&
    capturedMessage.context_pack.structured_transport.structured_turn_ref === structuredTurn?.structured_turn_ref
  );
  const report = {
    ok: !!(
      projection &&
      projection.status === 'completed' &&
      capturedMessage &&
      structuredTurn &&
      structuredTurn.type === 'AgentRuntimeStructuredTurn' &&
      structuredTurn.source_authority === 'gateway.runtime.agent_runtime_structured_transport' &&
      structuredTurn.transport_mode === 'structured_json' &&
      structuredTurn.transport_target === 'structured_json' &&
      !missing.length &&
      promptCompatSource &&
      hasNoSecrets(structuredTurn)
    ),
    type: 'agent_runtime_structured_transport_eval',
    generated_at: new Date().toISOString(),
    turn_outcome_status: projection && projection.status,
    structured_turn_ref: structuredTurn && structuredTurn.structured_turn_ref || '',
    missing_required_sections: missing,
    prompt_compat_derives_from_structured_turn: promptCompatSource,
    no_secret_material: hasNoSecrets(structuredTurn),
    context_window_count: Array.isArray(structuredTurn?.context_pack?.conversation_window) ? structuredTurn.context_pack.conversation_window.length : 0,
    tool_grant_count: Array.isArray(structuredTurn?.tool_grants?.tools) ? structuredTurn.tool_grants.tools.length : 0,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_structured_transport_eval',
    error: clean(error && error.stack ? error.stack : error, 4000),
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
