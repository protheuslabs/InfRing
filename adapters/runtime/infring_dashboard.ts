#!/usr/bin/env tsx
// Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.

const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { createHash } = require('node:crypto');
const { spawn } = require('node:child_process');
const {
  ROOT,
  invokeInfringOpsViaBridge,
  resolveBinary,
  runInfringOps,
} = require('./run_infring_ops.ts');
const { buildPrimaryDashboardHtml, hasPrimaryDashboardUi, readBuildVersionInfo, readPrimaryDashboardAsset } = require('./dashboard_asset_router.ts');
const { createAgentWsBridge } = require('../../gateway/runtime/sockets/agent_ws/agent_ws_bridge.ts');
const { loadAgentRuntimeEngineRegistry, createAgentRuntimeRouter } = require('../../gateway/runtime/agent_runtime/agent_runtime_router.ts');
const {
  ingestAgentRuntimeContextProjection,
  appendAgentRuntimeTurnAtoms,
  materializeAgentRuntimeContextPack,
  loadAgentRuntimeContextRows,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_context_store.ts');
const { materializeKernelAgentRuntimeContextPack } = require('../../gateway/runtime/agent_runtime/agent_runtime_kernel_context_bridge.ts');
const { buildUniversalToolGrants } = require('../../gateway/runtime/agent_runtime/universal_core_tools.ts');
const { createInfringNativeEngineAdapter } = require('./agent_engines/infring_native.ts');
const { createCodexCliEngineAdapter } = require('./agent_engines/codex_cli.ts');
const { createClaudeCodeEngineAdapter } = require('./agent_engines/claude_code.ts');
const { createGrokCodeEngineAdapter } = require('./agent_engines/grok_code.ts');
const {
  isShellSocketChatProjectionPath,
  shellSocketChatProjection,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_chat_projection.ts');
const {
  isShellSocketCommandIngressPath,
  shellSocketCommandIngress,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_command_ingress.ts');
const {
  isShellSocketStatusProjectionPath,
  shellSocketStatusProjection,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_status_projection.ts');
const {
  isShellSocketEvalIssueIngressPath,
  shellSocketEvalIssueIngress,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_eval_issue_ingress.ts');
const {
  isShellSocketLifecycleIngressPath,
  shellSocketLifecycleIngress,
} = require('../../gateway/runtime/sockets/shell_socket/shell_socket_lifecycle_ingress.ts');
const {
  backendFreshnessSnapshot: backendFreshnessSnapshotFromProcess,
  backendSpawnEnv: backendSpawnEnvForRoot,
  shouldRestartStaleBackend,
} = require('./dashboard_backend_freshness.ts');
const {
  normalizeAgentRuntimeTurnInput,
} = require('../../gateway/runtime/agent_runtime_input_normalizer.ts');
const {
  createAgentRuntimeWorkspaceStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_workspace.ts');
const {
  createAgentRuntimeApprovalStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_approvals.ts');
const {
  createAgentRuntimeTranscriptStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_transcripts.ts');
const {
  createAgentRuntimeSessionStateStore,
} = require('../../gateway/runtime/agent_runtime/agent_runtime_session_state.ts');

const DASHBOARD_DIR = path.resolve(ROOT, 'client', 'runtime', 'systems', 'ui');
const CANONICAL_STATIC_DIR = path.resolve(DASHBOARD_DIR, 'infring_static');
const STATIC_DIR = CANONICAL_STATIC_DIR;
const FORBIDDEN_ALT_DASHBOARD_DIRS = [
  path.resolve(DASHBOARD_DIR, 'legacy_dashboard'),
  path.resolve(DASHBOARD_DIR, 'reference_runtime_dashboard'),
  path.resolve(DASHBOARD_DIR, 'control_runtime_dashboard'),
  path.resolve(DASHBOARD_DIR, 'dashboard_legacy'),
  path.resolve(DASHBOARD_DIR, 'deprecated_dashboard'),
];
const SIBLING_ALT_DASHBOARD_PATTERN = /(legacy|reference_runtime|control_runtime|deprecated)/i;
const STATUS_DIR = path.resolve(
  ROOT,
  'client',
  'runtime',
  'local',
  'state',
  'ui',
  'infring_dashboard',
);
const STATUS_PATH = path.resolve(STATUS_DIR, 'server_status.json');
const STATUS_SNAPSHOT_PATH = path.resolve(STATUS_DIR, 'latest_snapshot.json');
const TROUBLESHOOTING_DIR = path.resolve(STATUS_DIR, 'troubleshooting');
const TROUBLESHOOTING_RECENT_WORKFLOWS_PATH = path.resolve(TROUBLESHOOTING_DIR, 'recent_workflows.json');
const TROUBLESHOOTING_EVAL_QUEUE_PATH = path.resolve(TROUBLESHOOTING_DIR, 'eval_queue.json');
const TROUBLESHOOTING_ISSUE_OUTBOX_PATH = path.resolve(TROUBLESHOOTING_DIR, 'issue_outbox.json');
const TROUBLESHOOTING_LATEST_SNAPSHOT_PATH = path.resolve(TROUBLESHOOTING_DIR, 'latest_snapshot.json');
const TROUBLESHOOTING_SNAPSHOT_HISTORY_PATH = path.resolve(TROUBLESHOOTING_DIR, 'snapshot_history.jsonl');
const TROUBLESHOOTING_LATEST_EVAL_REPORT_PATH = path.resolve(TROUBLESHOOTING_DIR, 'latest_eval_report.json');
const TROUBLESHOOTING_DEFAULT_EVAL_MODEL = 'gpt-5.4';
const TROUBLESHOOTING_MAX_RECENT = 10;
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const DEFAULT_TEAM = 'ops';
const DEFAULT_REFRESH_MS = 2000;
const DEFAULT_BACKEND_READY_TIMEOUT_MS = 120000;
const BACKEND_PORT_OFFSET = 1000;
const DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS = 180;
const DASHBOARD_SHUTDOWN_EXIT_DELAY_MIN_MS = 80;
const DASHBOARD_SHUTDOWN_EXIT_DELAY_MAX_MS = 5000;
const HOP_BY_HOP = new Set(['connection', 'host', 'keep-alive', 'proxy-authenticate', 'proxy-authorization', 'te', 'trailers', 'transfer-encoding', 'upgrade']);
const agentRuntimeWorkspaceStore = createAgentRuntimeWorkspaceStore({ root: ROOT, statusDir: STATUS_DIR });
const {
  normalizeAgentRuntimeWorkspacePath,
  loadAgentRuntimeWorkspace,
  saveAgentRuntimeWorkspace,
  pickAgentRuntimeWorkspaceDirectory,
} = agentRuntimeWorkspaceStore;
const agentRuntimeApprovalStore = createAgentRuntimeApprovalStore({ root: ROOT });
const {
  sanitizeAgentRuntimeProposalArguments,
  agentRuntimeApprovalDecisionProjection,
  mergeAgentRuntimeApprovalPermissionPolicy,
} = agentRuntimeApprovalStore;
const agentRuntimeTranscriptStore = createAgentRuntimeTranscriptStore({ statusDir: STATUS_DIR });
const {
  appendAgentRuntimeTranscriptTurn,
  mergeAgentRuntimeTranscriptPayload,
  agentRuntimeTranscriptFilterFromShellSocketPath,
} = agentRuntimeTranscriptStore;
const agentRuntimeSessionStateStore = createAgentRuntimeSessionStateStore({ statusDir: STATUS_DIR });
const {
  loadAgentRuntimeSelection,
  saveAgentRuntimeSelection,
  queueAgentRuntimeSteeringIntervention,
  drainAgentRuntimeSteeringInterventions,
} = agentRuntimeSessionStateStore;

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanApprovalId(value) { return cleanText(value, 260).replace(/[^a-zA-Z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanPathText(value, maxLen = 1200) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/\n+/g, ' ').trim().slice(0, maxLen); }
const AGENT_RUNTIME_CONTEXT_FANOUT_TARGET = 7;
const AGENT_RUNTIME_CONTEXT_HOT_TAIL_COUNT = 4;
const AGENT_RUNTIME_CONTEXT_MAX_ROWS = 49;
const AGENT_RUNTIME_CONTEXT_ROW_TEXT_MAX = 1200;
function estimateContextTokens(value) {
  return Math.max(1, Math.ceil(String(value == null ? '' : value).length / 4));
}
function cleanContextRole(value) {
  const role = cleanText(value, 40).toLowerCase();
  if (role === 'user' || role === 'human') return 'user';
  if (role === 'assistant' || role === 'agent' || role === 'ai') return 'assistant';
  if (role === 'tool' || role === 'function') return 'tool';
  if (role === 'system') return 'system';
  return 'message';
}
function contextRowText(row) {
  if (!row || typeof row !== 'object') return '';
  return cleanDisplayText(
    row.text_preview || row.content_preview || row.visible_text || row.text || row.message || row.content || '',
    AGENT_RUNTIME_CONTEXT_ROW_TEXT_MAX,
  );
}
function contextRef(prefix, sessionId, level, startSeq, endSeq) {
  const cleanSession = cleanText(sessionId, 120).replace(/[^a-zA-Z0-9_.:-]+/g, '_') || 'session';
  return `${prefix}_${cleanSession}_l${level}_${startSeq}_${endSeq}`;
}
function buildSpan(sessionId, level, children, status) {
  const rows = Array.isArray(children) ? children : [];
  const first = rows[0] || {};
  const last = rows[rows.length - 1] || first;
  const startSeq = Number(first.start_seq || first.sequence_no || 0) || 0;
  const endSeq = Number(last.end_seq || last.sequence_no || startSeq) || startSeq;
  const childRefs = rows.map((row) => cleanText(row.atom_id || row.span_id || row.ref_id, 160)).filter(Boolean);
  const decisions = [];
  const constraints = [];
  const openLoops = [];
  const entities = [];
  const taskRefs = [];
  const memoryVersionRefs = [];
  for (const row of rows) {
    for (const [source, target] of [
      [row.decisions, decisions],
      [row.constraints, constraints],
      [row.open_loops, openLoops],
      [row.entities, entities],
      [row.task_refs, taskRefs],
      [row.memory_version_refs, memoryVersionRefs],
    ]) {
      if (Array.isArray(source)) {
        for (const item of source) {
          const cleaned = cleanText(item, 180);
          if (cleaned && !target.includes(cleaned)) target.push(cleaned);
        }
      }
    }
  }
  const excerpt = rows
    .map((row) => cleanText(row.summary || row.text_preview || row.payload?.text_preview || row.ref_id || row.atom_id || row.span_id, 220))
    .filter(Boolean)
    .join(' | ')
    .slice(0, 900);
  return {
    span_id: contextRef('ctx_span_projection', sessionId, level, startSeq, endSeq),
    session_id: cleanText(sessionId, 200),
    level,
    status,
    start_seq: startSeq,
    end_seq: endSeq,
    child_refs: childRefs,
    summary: excerpt || `context rollup level=${level} covers ${startSeq}-${endSeq} from ${childRefs.length} children`,
    decisions,
    constraints,
    open_loops: openLoops,
    entities,
    task_refs: taskRefs,
    memory_version_refs: memoryVersionRefs,
    token_count: rows.reduce((sum, row) => sum + (Number(row.token_count) || estimateContextTokens(row.summary || row.text_preview || '')), 0),
    fidelity_score: 1,
  };
}
function buildAgentRuntimeContextPack(options = {}) {
  const body = options.body && typeof options.body === 'object' ? options.body : {};
  const sessionId = cleanText(options.sessionId || body.session_id, 200) || 'session';
  const agentId = cleanText(options.agentId || body.agent_id, 160) || 'default';
  const projection = body.context_projection && typeof body.context_projection === 'object' ? body.context_projection : {};
  const sourceRows = Array.isArray(projection.rows) ? projection.rows : [];
  const rawRows = sourceRows.slice(-(AGENT_RUNTIME_CONTEXT_MAX_ROWS * 2));
  const rows = [];
  const seenRows = new Set();
  for (const row of rawRows.slice().reverse()) {
    const text = contextRowText(row)
      .toLowerCase()
      .replace(/\s+/g, ' ')
      .replace(/[^\p{L}\p{N}\s._:/-]/gu, '')
      .trim();
    if (!text) continue;
    const role = cleanContextRole(row.role || row.origin_kind || row.actor);
    const key = `${role}:${text}`;
    if (seenRows.has(key)) continue;
    seenRows.add(key);
    rows.push(row);
  }
  rows.reverse();
  if (rows.length > AGENT_RUNTIME_CONTEXT_MAX_ROWS) rows.splice(0, rows.length - AGENT_RUNTIME_CONTEXT_MAX_ROWS);
  const atoms = [];
  rows.forEach((row, idx) => {
    const text = contextRowText(row);
    if (!text) return;
    const sequenceNo = idx + 1;
    const role = cleanContextRole(row.role || row.origin_kind || row.actor);
    const sourceKind = row.source_kind || (role === 'user'
      ? 'user_message'
      : role === 'assistant'
        ? 'assistant_message'
        : role === 'tool'
          ? 'tool_receipt'
          : role === 'system'
            ? 'system_event'
            : 'message_event');
    atoms.push({
      atom_id: cleanText(row.atom_id || row.id, 160) || contextRef('ctx_atom_projection', sessionId, 0, sequenceNo, sequenceNo),
      session_id: sessionId,
      sequence_no: sequenceNo,
      source_kind: sourceKind,
      record_type: sourceKind,
      source_ref: cleanText(row.detail_ref || row.id || `message-${sequenceNo}`, 240),
      source_authority: cleanText(row.source_authority || projection.source || 'shell_bounded_message_projection', 160),
      speaker_label: cleanText(row.speaker_label || row.origin_display_name || row.agent_name || role, 120),
      role,
      text_preview: text,
      token_count: Math.min(Number(row.token_count) || estimateContextTokens(text), 4000),
      task_refs: Array.isArray(row.task_refs) ? row.task_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 12) : [],
      memory_version_refs: Array.isArray(row.memory_version_refs) ? row.memory_version_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 12) : [],
      lineage_refs: Array.isArray(row.lineage_refs) ? row.lineage_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 12) : [],
    });
  });
  const spans = [];
  const level0 = [];
  for (let start = 0; start < atoms.length; start += AGENT_RUNTIME_CONTEXT_FANOUT_TARGET) {
    const chunk = atoms.slice(start, start + AGENT_RUNTIME_CONTEXT_FANOUT_TARGET);
    const status = chunk.length >= AGENT_RUNTIME_CONTEXT_FANOUT_TARGET ? 'sealed' : 'active';
    const span = buildSpan(sessionId, 0, chunk, status);
    level0.push(span);
    spans.push(span);
  }
  let sealed = level0.filter((row) => row.status === 'sealed');
  let level = 1;
  while (sealed.length >= AGENT_RUNTIME_CONTEXT_FANOUT_TARGET && level <= 5) {
    const parents = [];
    for (let start = 0; start + AGENT_RUNTIME_CONTEXT_FANOUT_TARGET <= sealed.length; start += AGENT_RUNTIME_CONTEXT_FANOUT_TARGET) {
      const parent = buildSpan(sessionId, level, sealed.slice(start, start + AGENT_RUNTIME_CONTEXT_FANOUT_TARGET), 'sealed');
      parents.push(parent);
      spans.push(parent);
    }
    sealed = parents;
    level += 1;
  }
  const byLevel = new Map();
  for (const span of spans) {
    const arr = byLevel.get(span.level) || [];
    arr.push(span);
    byLevel.set(span.level, arr);
  }
  const selectedSpans = [];
  for (const levelKey of Array.from(byLevel.keys()).sort((a, b) => a - b)) {
    const arr = byLevel.get(levelKey).slice().sort((a, b) => Number(b.end_seq) - Number(a.end_seq));
    if (arr[0]) selectedSpans.push(arr[0]);
  }
  const hotAtoms = atoms.slice(-AGENT_RUNTIME_CONTEXT_HOT_TAIL_COUNT);
  const fragments = [
    ...hotAtoms.map((atom) => ({
      fragment_id: contextRef('ctx_fragment_atom_projection', sessionId, 0, atom.sequence_no, atom.sequence_no),
      kind: 'atom',
      ref_id: atom.atom_id,
      level: 0,
      token_count: atom.token_count,
      payload: {
        source_kind: atom.source_kind,
        record_type: atom.record_type || atom.source_kind,
        source_ref: atom.source_ref,
        source_authority: atom.source_authority,
        speaker_label: atom.speaker_label,
        role: atom.role,
        text_preview: atom.text_preview,
        sequence_no: atom.sequence_no,
      },
      lineage_refs: atom.lineage_refs,
    })),
    ...selectedSpans.map((span) => ({
      fragment_id: contextRef('ctx_fragment_span_projection', sessionId, span.level, span.start_seq, span.end_seq),
      kind: 'span',
      ref_id: span.span_id,
      level: span.level,
      token_count: span.token_count,
      payload: {
        summary: span.summary,
        decisions: span.decisions,
        constraints: span.constraints,
        open_loops: span.open_loops,
        entities: span.entities,
        task_refs: span.task_refs,
        memory_version_refs: span.memory_version_refs,
        fidelity_score: span.fidelity_score,
        status: span.status,
        coverage: { start_seq: span.start_seq, end_seq: span.end_seq },
      },
      lineage_refs: [],
    })),
  ];
  return {
    schema_version: 1,
    type: 'agent_runtime_context_pack',
    source_basis: 'core.layer2.memory.context_topology_projection',
    source_authority: 'gateway_bounded_projection_pending_kernel_materializer_route',
    canonical_model_refs: [
      'core/layer2/memory/src/context_atoms.rs',
      'core/layer2/memory/src/context_topology.rs',
      'core/layer2/memory/src/context_materializer.rs',
    ],
    session_id: sessionId,
    agent_id: agentId,
    fanout_target: AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
    hot_tail_count: AGENT_RUNTIME_CONTEXT_HOT_TAIL_COUNT,
    row_count: atoms.length,
    raw_row_count: sourceRows.length,
    dedupe_policy: {
      type: 'gateway_projection_tail_window',
      key_basis: 'source_projection_rows',
      preserves_latest_duplicate: true,
    },
    frontier: {
      session_id: sessionId,
      hot_atom_refs: hotAtoms.map((row) => row.atom_id),
      warm_span_refs: selectedSpans.filter((row) => row.level === 0).map((row) => row.span_id),
      cool_span_refs: selectedSpans.filter((row) => row.level === 1 || row.level === 2).map((row) => row.span_id),
      cold_span_refs: selectedSpans.filter((row) => row.level >= 3).map((row) => row.span_id),
      pinned_anchor_refs: Array.isArray(projection.pinned_anchor_refs) ? projection.pinned_anchor_refs.map((item) => cleanText(item, 180)).filter(Boolean).slice(0, 16) : [],
      budget_tokens: 6000,
      used_tokens: fragments.reduce((sum, row) => sum + (Number(row.token_count) || 0), 0),
      pressure_state: fragments.length > 24 ? 'medium' : 'low',
      fidelity_score: 1,
    },
    fragments,
  };
}
function createAgentRuntimeEngineAdapterMap(options = {}) {
  const liveDispatch = options.liveDispatch === true;
  const cwd = normalizeAgentRuntimeWorkspacePath(options.cwd || ROOT);
  return {
    infring_native: createInfringNativeEngineAdapter({
      liveDispatch,
      orchestrationClient: options.nativeOrchestrationClient || options.orchestrationClient,
    }),
    codex_cli: createCodexCliEngineAdapter({ liveDispatch, cwd }),
    claude_code: createClaudeCodeEngineAdapter({ liveDispatch, cwd }),
    grok_code: createGrokCodeEngineAdapter({ liveDispatch, cwd }),
  };
}
function createDashboardAgentRuntimeRouter(options = {}) {
  const router = createAgentRuntimeRouter({ root: ROOT, disableTraceWriter: options.disableTraceWriter === true });
  const adapters = createAgentRuntimeEngineAdapterMap({
    liveDispatch: options.liveDispatch === true,
    nativeOrchestrationClient: options.nativeOrchestrationClient,
    cwd: options.cwd,
  });
  for (const [engineId, adapter] of Object.entries(adapters)) router.registerAdapter(engineId, adapter);
  return router;
}
function findAgentRuntimeEngine(registryInfo, engineId) {
  const target = cleanEngineId(engineId);
  const engines = Array.isArray(registryInfo && registryInfo.engines) ? registryInfo.engines : [];
  return engines.find((engine) => cleanEngineId(engine && engine.engine_id) === target) || null;
}
function sanitizeAgentRuntimeActivityEvent(row, index, defaults = {}) {
  const event = row && typeof row === 'object' ? row : {};
  return {
    type: 'agent_activity_event',
    activity_kind: cleanText(event.activity_kind || event.kind || event.type, 80) || 'activity',
    provider_event_type: cleanText(event.provider_event_type || event.provider_type || event.event_type, 160),
    source: cleanText(event.source || 'external_cli_stream', 120),
    sequence_no: Number(event.sequence_no || index + 1) || index + 1,
    item_id: cleanText(event.item_id || event.itemId || '', 200),
    status: cleanText(event.status || '', 80),
    text: cleanDisplayText(event.text || event.display_text || event.summary || '', 4000),
    display_text: cleanDisplayText(event.display_text || event.text || event.summary || '', 4000),
    receipt_ref: cleanText(event.receipt_ref || '', 240),
    result_ref: cleanText(event.result_ref || '', 240),
    engine_id: cleanEngineId(event.engine_id || defaults.engineId),
    trace_id: cleanText(event.trace_id || defaults.traceId, 200),
    session_id: cleanText(event.session_id || defaults.sessionId, 200),
    turn_id: cleanText(event.turn_id || defaults.turnId, 200),
  };
}
function classifyAgentRuntimePreTurnFailureCode(engineId, source, fallback = 'agent_runtime_engine_unavailable') {
  const cleanEngine = cleanEngineId(engineId) || 'agent_runtime';
  const text = cleanDisplayText([
    source && source.error,
    source && source.error_code,
    source && source.reason,
    source && source.status,
    source && source.version_preview,
    source && source.stderr_preview,
    source && source.message,
  ].filter(Boolean).join('\n'), 12000).toLowerCase();
  if (
    text.includes('quota') ||
    text.includes('credit') ||
    text.includes('billing') ||
    text.includes('subscription') ||
    text.includes('payment required') ||
    text.includes('insufficient balance')
  ) {
    return `${cleanEngine}_provider_quota_or_subscription_unavailable`;
  }
  if (
    text.includes('unauthorized') ||
    text.includes('not authorized') ||
    text.includes('authentication') ||
    text.includes('auth required') ||
    text.includes('login required') ||
    text.includes('please login') ||
    text.includes('please log in') ||
    text.includes('api key') ||
    text.includes('invalid token') ||
    text.includes('token expired')
  ) {
    return `${cleanEngine}_provider_auth_required`;
  }
  if (
    text.includes('rate limit') ||
    text.includes('rate-limit') ||
    text.includes('too many requests') ||
    text.includes('429')
  ) {
    return `${cleanEngine}_provider_rate_limited`;
  }
  if (
    text.includes('not found') ||
    text.includes('missing') ||
    text.includes('not installed') ||
    text.includes('command not found') ||
    text.includes('enoent')
  ) {
    return `${cleanEngine}_runtime_not_available`;
  }
  return cleanText(source && (source.error_code || source.error), 120) || fallback;
}
function agentRuntimePreTurnFailureProjection(traceId, engineId, agentId, sessionId, turnId, reason, source = {}) {
  const cleanEngine = cleanEngineId(engineId) || 'agent_runtime';
  const errorCode = classifyAgentRuntimePreTurnFailureCode(cleanEngine, { ...source, reason });
  const displayText = cleanDisplayText(
    reason || `${cleanEngine} is not available for this turn.`,
    1200,
  );
  return {
    ok: false,
    status_code: 200,
    type: 'agent_runtime_turn_projection',
    trace_id: traceId,
    engine_id: cleanEngine,
    agent_id: cleanText(agentId, 160),
    session_id: cleanText(sessionId, 200),
    turn_id: cleanText(turnId, 200),
    status: 'failed_with_reason',
    error_code: errorCode,
    reason: displayText,
    retryable: !/quota|subscription|auth|login|api_key|billing/i.test(errorCode),
    timed_out: false,
    timeout_ms: 0,
    text: displayText,
    display_text: displayText,
    output_text: displayText,
    output_preview: cleanText(displayText, 4000),
    agent_activity_events: [
      {
        type: 'agent_activity_event',
        activity_kind: 'error',
        provider_event_type: 'pre_turn.failure',
        source: 'infring_gateway_agent_runtime_socket',
        sequence_no: 1,
        status: 'failed',
        text: displayText,
        display_text: displayText,
        engine_id: cleanEngine,
        trace_id: traceId,
        session_id: cleanText(sessionId, 200),
        turn_id: cleanText(turnId, 200),
      },
    ],
    activity_event_count: 1,
    raw_activity_event_count: 1,
    structured_activity: true,
    result_ref: '',
    receipt_ref: '',
    pending_permission_request: null,
  };
}
async function agentRuntimeTurnProjection(traceId, body, options = {}) {
  const rawEngineId = body && (body.engine_id || body.agent_runtime_engine_id || body.runtime_engine_id);
  const engineId = cleanEngineId(rawEngineId || 'infring_native');
  if (!engineId) {
    return {
      ok: false,
      status_code: 400,
      error: 'agent_runtime_engine_id_required',
      trace_id: traceId,
      engine_id: 'infring_native',
    };
  }
  const agentId = cleanText(body && body.agent_id, 160) || 'default';
  const sessionId = cleanText(body && body.session_id, 200) || `shell_${agentId}`;
  const turnId = cleanText(body && body.turn_id, 200) || `turn_${Date.now().toString(36)}`;
  const inputNormalization = normalizeAgentRuntimeTurnInput({
    body,
    traceId,
    engineId,
    agentId,
    sessionId,
    turnId,
  });
  const text = inputNormalization.text;
  const attachmentRefs = inputNormalization.attachmentRefs;
  if (!text && !attachmentRefs.length) {
    return {
      ok: false,
      status_code: 400,
      error: 'agent_runtime_turn_missing_input',
      trace_id: traceId,
      engine_id: engineId,
    };
  }
  const registry = loadAgentRuntimeEngineRegistry(ROOT);
  const engine = findAgentRuntimeEngine(registry, engineId);
  if (!engine) {
    return {
      ok: false,
      status_code: 404,
      error: 'agent_runtime_engine_unknown',
      trace_id: traceId,
      engine_id: engineId,
    };
  }
  const workspace = loadAgentRuntimeWorkspace(traceId);
  const activeWorkspaceDir = workspace.workspace_dir || workspace.active_workspace || ROOT;
  const router = createDashboardAgentRuntimeRouter({
    liveDispatch: true,
    nativeOrchestrationClient: options.nativeOrchestrationClient,
    cwd: activeWorkspaceDir,
  });
  const streamedActivityEvents = [];
  const activityDefaults = { engineId, traceId, sessionId, turnId };
  const onActivity = (event) => {
    const normalized = sanitizeAgentRuntimeActivityEvent(event, streamedActivityEvents.length, activityDefaults);
    if (!normalized.display_text && !normalized.provider_event_type) return;
    streamedActivityEvents.push(normalized);
    if (typeof options.onActivity === 'function') options.onActivity(normalized);
  };
  const emitSyntheticActivity = (activityKind, providerEventType, displayText, status = 'completed') => onActivity({
    type: 'agent_activity_event',
    activity_kind: activityKind,
    provider_event_type: providerEventType,
    source: 'infring_gateway_agent_runtime_socket',
    status,
    display_text: displayText,
    text: displayText,
    engine_id: engineId,
    trace_id: traceId,
    session_id: sessionId,
    turn_id: turnId,
  });
  if (inputNormalization.largeTextAttachment) {
    emitSyntheticActivity(
      'activity',
      'context.attachment.materialized',
      `Moved oversized user text into ${inputNormalization.largeTextAttachment.filename} before runtime dispatch.`,
    );
  }
  try {
    appendAgentRuntimeTranscriptTurn({
      sessionId,
      agentId,
      traceId,
      turnId,
      engineId,
      userText: text,
      assistantText: '',
    });
  } catch {}
  try {
    appendAgentRuntimeTurnAtoms({
      root: ROOT,
      sessionId,
      agentId,
      traceId,
      turnId,
      engineId,
      userText: text,
      assistantText: '',
    });
  } catch {}
  emitSyntheticActivity('started', 'context.prepare', `Preparing ${engineId} with InfRing conversation context.`);
  try {
    ingestAgentRuntimeContextProjection({
      root: ROOT,
      sessionId,
      agentId,
      traceId,
      projection: body && body.context_projection,
    });
  } catch {}
  const fallbackContextRows = loadAgentRuntimeContextRows({ root: ROOT, sessionId, agentId });
  emitSyntheticActivity(
    'activity',
    'context.loaded',
    `Loaded ${fallbackContextRows.length} prior context row${fallbackContextRows.length === 1 ? '' : 's'} for ${engineId}.`,
  );
  const kernelContext = await materializeKernelAgentRuntimeContextPack({
    root: ROOT,
    sessionId,
    agentId,
    traceId,
    atoms: fallbackContextRows,
    timeoutMs: 8000,
  }).catch((error) => ({
    ok: false,
    reason: cleanText(error && error.message ? error.message : error, 240),
  }));
  const contextPack = kernelContext && kernelContext.ok && kernelContext.context_pack
    ? kernelContext.context_pack
    : await Promise.resolve(materializeAgentRuntimeContextPack({
      root: ROOT,
      sessionId,
      agentId,
      traceId,
    })).catch(() => buildAgentRuntimeContextPack({ body, agentId, sessionId, traceId }));
  contextPack.active_workspace = {
    workspace_dir: workspace.workspace_dir,
    active_workspace: workspace.active_workspace,
    display_label: workspace.display_label,
    git_root: workspace.git_root,
    git_root_label: workspace.git_root_label,
    permission_boundary: workspace.permission_boundary,
    source_authority: 'gateway_agent_runtime_workspace_selection',
  };
  if (attachmentRefs.length) {
    contextPack.runtime_attachment_refs = {
      type: 'agent_runtime_attachment_refs',
      source_authority: 'gateway_agent_runtime_attachment_normalization',
      attachment_count: attachmentRefs.length,
      attachments: attachmentRefs,
    };
  }
  const permissionPolicy = mergeAgentRuntimeApprovalPermissionPolicy(body && body.permission_policy, sessionId, engineId);
  contextPack.universal_tool_grants = buildUniversalToolGrants({
    traceId,
    sessionId,
    agentId,
    engineId,
    permissionPolicy,
  });
  emitSyntheticActivity('started', 'engine.health', `Checking ${engineId} availability.`);
  const health = await router.healthCheck({
    type: 'agent_runtime.health_check',
    trace_id: traceId,
    engine_id: engineId,
    session_id: sessionId,
    cwd: activeWorkspaceDir,
    workspace_dir: activeWorkspaceDir,
    active_workspace: workspace,
  });
  if (!health || (health.status !== 'available' && health.status !== 'adapter_ready')) {
    const reason = cleanDisplayText(
      health && (health.reason || health.error || health.version_preview || health.status)
        ? `${engineId} is unavailable: ${health.reason || health.error || health.version_preview || health.status}`
        : `${engineId} is unavailable for this turn.`,
      1200,
    );
    return agentRuntimePreTurnFailureProjection(traceId, engineId, agentId, sessionId, turnId, reason, health || {});
  }
  emitSyntheticActivity('started', 'session.start', `Starting ${engineId} session ${sessionId}.`);
  await router.startSession({
    type: 'agent_runtime.session_start',
    trace_id: traceId,
    engine_id: engineId,
    agent_id: agentId,
    session_id: sessionId,
    cwd: activeWorkspaceDir,
    workspace_dir: activeWorkspaceDir,
    active_workspace: workspace,
  });
  const steeringInterventions = drainAgentRuntimeSteeringInterventions({
    agentId,
    sessionId,
    engineId,
    traceId,
    turnId,
  });
  if (steeringInterventions.length) {
    contextPack.runtime_steering = {
      type: 'agent_runtime_steering_context',
      mode: 'next_turn_interventions',
      source_authority: 'gateway_agent_runtime_steer_route',
      intervention_count: steeringInterventions.length,
      interventions: steeringInterventions,
    };
    emitSyntheticActivity(
      'activity',
      'steering.loaded',
      `Loaded ${steeringInterventions.length} queued steering intervention${steeringInterventions.length === 1 ? '' : 's'} for this turn.`,
    );
  }
  const turnMessage = {
    type: 'agent_runtime.turn_submit',
    trace_id: traceId,
    engine_id: engineId,
    agent_id: agentId,
    session_id: sessionId,
	    turn_id: turnId,
	    cwd: activeWorkspaceDir,
	    workspace_dir: activeWorkspaceDir,
	    active_workspace: workspace,
	    input: { text, attachments: attachmentRefs },
	    context_pack: contextPack,
	    capability_budget: {
	      max_default_response_bytes: 65536,
	      max_turn_seconds: 180,
	      shell_projection_only: true,
	      context_pack_required: true,
	      context_pack_fanout_target: AGENT_RUNTIME_CONTEXT_FANOUT_TARGET,
	      universal_tool_grants_required: true,
	      universal_tool_ids: contextPack.universal_tool_grants.tools.map((tool) => tool.tool_id),
	    },
	  };
  emitSyntheticActivity('started', 'turn.launch', `Launching ${engineId} turn with bounded context pack.`);
  const turn = options && options.stream === true
    ? await router.streamTurn(turnMessage, onActivity)
    : await router.submitTurn(turnMessage);
  if (!(turn && Array.isArray(turn.activity_events) && turn.activity_events.length)) {
    emitSyntheticActivity('completed', 'turn.completed', `${engineId} returned ${cleanText(turn && turn.status, 80) || 'a result'}.`);
  }
  const output = cleanDisplayText(
    turn && (turn.output_text || turn.display_text || turn.text || turn.response || turn.output_preview || turn.delta || turn.reason),
    24000,
  );
  const outputPreview = cleanText(turn && (turn.output_preview || output), 4000);
  const pendingPermissionRequest = turn && turn.permission_request && typeof turn.permission_request === 'object'
    ? turn.permission_request
    : null;
  const rawTurnStatus = cleanText(turn && turn.status, 80);
  const terminalOutcomeStatus = pendingPermissionRequest
    ? 'permission_required'
    : rawTurnStatus === 'completed'
      ? 'completed'
      : turn && turn.timed_out
        ? 'timed_out_with_reason'
        : rawTurnStatus === 'failed' || (turn && turn.error_code)
          ? 'failed_with_reason'
          : (rawTurnStatus || 'unknown');
  const terminalOutcomeOk = terminalOutcomeStatus === 'completed' || terminalOutcomeStatus === 'permission_required';
  const finalActivityEvents = Array.isArray(turn && turn.activity_events)
    ? turn.activity_events.map((event, index) => sanitizeAgentRuntimeActivityEvent(event, index, activityDefaults))
    : [];
  const activityDedupe = new Set();
  const activityEvents = [...streamedActivityEvents, ...finalActivityEvents]
    .filter((event) => event && (event.display_text || event.provider_event_type))
    .filter((event) => {
      const key = cleanDisplayText(event.display_text, 1000)
        ? [
            event.activity_kind,
            event.provider_event_type,
            cleanDisplayText(event.display_text, 1000),
          ].join('|')
        : [
            event.sequence_no,
            event.activity_kind,
            event.provider_event_type,
          ].join('|');
      if (activityDedupe.has(key)) return false;
      activityDedupe.add(key);
      return true;
    })
    .slice(-80);
  const persistedAssistantOutput = pendingPermissionRequest ? '' : output;
  try {
    appendAgentRuntimeTurnAtoms({
      root: ROOT,
      sessionId,
      agentId,
      traceId,
      turnId,
      engineId,
      userText: text,
      assistantText: persistedAssistantOutput,
      resultRef: turn && turn.result_ref,
      receiptRef: turn && turn.receipt_ref,
    });
  } catch {}
  try {
    appendAgentRuntimeTranscriptTurn({
      sessionId,
      agentId,
      traceId,
      turnId,
      engineId,
      userText: text,
      assistantText: persistedAssistantOutput,
      pendingPermissionRequest,
    });
  } catch {}
  return {
    ok: terminalOutcomeOk,
    status_code: 200,
    type: 'agent_runtime_turn_projection',
    trace_id: traceId,
    engine_id: engineId,
    agent_id: agentId,
    session_id: sessionId,
    turn_id: turnId,
    status: terminalOutcomeStatus,
    error_code: cleanText(turn && turn.error_code, 120),
    reason: cleanDisplayText(turn && turn.reason, 1200),
    retryable: turn && turn.retryable === true,
    timed_out: turn && turn.timed_out === true,
    timeout_ms: Number(turn && turn.timeout_ms) || 0,
    text: pendingPermissionRequest ? '' : output,
    display_text: pendingPermissionRequest ? '' : output,
    output_text: pendingPermissionRequest ? '' : output,
    output_preview: pendingPermissionRequest ? '' : outputPreview,
	    agent_activity_events: activityEvents,
	    activity_event_count: activityEvents.length,
	    raw_activity_event_count: Number(turn && turn.activity_event_count) || activityEvents.length,
	    structured_activity: turn && turn.structured_activity === true,
	    result_ref: cleanText(turn && turn.result_ref, 240),
	    receipt_ref: cleanText(turn && turn.receipt_ref, 240),
	    pending_permission_request: pendingPermissionRequest ? {
	      type: 'permission.requested',
	      approval_id: cleanApprovalId(pendingPermissionRequest.approval_id),
	      trace_id: cleanText(pendingPermissionRequest.trace_id || traceId, 200),
	      request_id: cleanText(pendingPermissionRequest.request_id, 200),
	      engine_id: cleanEngineId(pendingPermissionRequest.engine_id || engineId),
	      session_id: cleanText(pendingPermissionRequest.session_id || sessionId, 200),
	      turn_id: cleanText(pendingPermissionRequest.turn_id || turnId, 200),
	      tool_call_ref: cleanText(pendingPermissionRequest.tool_call_ref, 240),
	      tool_id: cleanText(pendingPermissionRequest.tool_id, 120),
	      capability: cleanText(pendingPermissionRequest.capability, 160),
	      reason: cleanText(pendingPermissionRequest.reason, 1000),
	      argument_keys: Array.isArray(pendingPermissionRequest.argument_keys)
	        ? pendingPermissionRequest.argument_keys.map((key) => cleanText(key, 80)).filter(Boolean).slice(0, 24)
	        : [],
	      proposal_arguments: sanitizeAgentRuntimeProposalArguments(pendingPermissionRequest.proposal_arguments),
	      gatekeeper_kind: cleanText(pendingPermissionRequest.gatekeeper_kind || 'user', 80) || 'user',
	      status: 'paused_pending_approval',
	      future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor'],
	      decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
	      decision_scope: 'tool_call',
	      approval_route: `/api/shell-socket/approvals/${encodeURIComponent(cleanApprovalId(pendingPermissionRequest.approval_id))}/decision`,
	    } : null,
	    context_pack: {
	      type: 'agent_runtime_context_pack_projection',
	      source_basis: contextPack.source_basis,
	      source_authority: contextPack.source_authority,
	      row_count: contextPack.row_count,
	      fanout_target: contextPack.fanout_target,
	      fragment_count: contextPack.fragments.length,
	      hot_atom_count: contextPack.frontier.hot_atom_refs.length,
	      warm_span_count: contextPack.frontier.warm_span_refs.length,
	      cool_span_count: contextPack.frontier.cool_span_refs.length,
	      cold_span_count: contextPack.frontier.cold_span_refs.length,
	      universal_tool_count: contextPack.universal_tool_grants.tools.length,
	      steering_intervention_count: contextPack.runtime_steering ? contextPack.runtime_steering.intervention_count : 0,
	      universal_tool_source_authority: contextPack.universal_tool_grants.source_authority,
	      kernel_materializer_used: !!(kernelContext && kernelContext.ok),
	      kernel_materializer_mode: cleanText(kernelContext && kernelContext.command_mode, 40),
	    },
	    health: {
	      status: cleanText(health && health.status, 80),
      discovery_source: cleanText(health && health.discovery_source, 120),
      version_preview: cleanText(health && health.version_preview, 200),
    },
  };
}
async function agentRuntimeContextPackPreviewProjection(traceId, body) {
  const rawEngineId = body && (body.engine_id || body.agent_runtime_engine_id || body.runtime_engine_id);
  const engineId = cleanEngineId(rawEngineId || 'infring_native');
  const agentId = cleanText(body && body.agent_id, 160) || 'default';
  const sessionId = cleanText(body && body.session_id, 200) || `shell_${agentId}`;
  const fallbackContextRows = loadAgentRuntimeContextRows({ root: ROOT, sessionId, agentId });
  const kernelContext = await materializeKernelAgentRuntimeContextPack({
    root: ROOT,
    sessionId,
    agentId,
    traceId,
    atoms: fallbackContextRows,
    timeoutMs: 8000,
  }).catch((error) => ({
    ok: false,
    reason: cleanText(error && error.message ? error.message : error, 240),
  }));
  const contextPack = kernelContext && kernelContext.ok && kernelContext.context_pack
    ? kernelContext.context_pack
    : await Promise.resolve(materializeAgentRuntimeContextPack({
      root: ROOT,
      sessionId,
      agentId,
      traceId,
    })).catch(() => buildAgentRuntimeContextPack({ body, agentId, sessionId, traceId }));
  contextPack.universal_tool_grants = buildUniversalToolGrants({
    traceId,
    sessionId,
    agentId,
    engineId,
    permissionPolicy: body && body.permission_policy,
  });
  const fragments = Array.isArray(contextPack.fragments) ? contextPack.fragments : [];
  return {
    ok: true,
    type: 'agent_runtime_context_pack_preview',
    trace_id: traceId,
    engine_id: engineId,
    agent_id: agentId,
    session_id: sessionId,
    source_basis: cleanText(contextPack.source_basis, 160),
    source_authority: cleanText(contextPack.source_authority, 200),
    row_count: Number(contextPack.row_count) || 0,
    raw_row_count: Number(contextPack.raw_row_count) || Number(contextPack.row_count) || 0,
    dedupe_policy: contextPack.dedupe_policy || null,
    kernel_materializer_used: !!(kernelContext && kernelContext.ok),
    kernel_materializer_mode: cleanText(kernelContext && kernelContext.command_mode, 40),
    frontier: {
      hot_atom_count: Array.isArray(contextPack.frontier && contextPack.frontier.hot_atom_refs) ? contextPack.frontier.hot_atom_refs.length : 0,
      warm_span_count: Array.isArray(contextPack.frontier && contextPack.frontier.warm_span_refs) ? contextPack.frontier.warm_span_refs.length : 0,
      cool_span_count: Array.isArray(contextPack.frontier && contextPack.frontier.cool_span_refs) ? contextPack.frontier.cool_span_refs.length : 0,
      cold_span_count: Array.isArray(contextPack.frontier && contextPack.frontier.cold_span_refs) ? contextPack.frontier.cold_span_refs.length : 0,
      pressure_state: cleanText(contextPack.frontier && contextPack.frontier.pressure_state, 80),
    },
    fragments: fragments.slice(-24).map((fragment) => {
      const payload = fragment && fragment.payload && typeof fragment.payload === 'object' ? fragment.payload : {};
      return {
        fragment_id: cleanText(fragment && fragment.fragment_id, 200),
        kind: cleanText(fragment && fragment.kind, 40),
        ref_id: cleanText(fragment && fragment.ref_id, 200),
        level: Number(fragment && fragment.level) || 0,
        source_kind: cleanText(payload.source_kind || payload.record_type, 120),
        speaker_label: cleanText(payload.speaker_label || payload.role, 120),
        role: cleanText(payload.role, 40),
        source_ref: cleanText(payload.source_ref, 240),
        summary: cleanDisplayText(payload.text_preview || payload.summary || '', 800),
      };
    }),
    universal_tool_count: Array.isArray(contextPack.universal_tool_grants && contextPack.universal_tool_grants.tools)
      ? contextPack.universal_tool_grants.tools.length
      : 0,
  };
}
function isTransientSocketError(error) {
  const code = cleanText(error && error.code ? error.code : '', 40);
  return code === 'ECONNRESET' || code === 'EPIPE' || code === 'ERR_STREAM_PREMATURE_CLOSE';
}
function ignoreStreamErrors(stream) {
  if (!stream || typeof stream.on !== 'function') return;
  if (stream.__infringIgnoreErrorsInstalled) return;
  stream.__infringIgnoreErrorsInstalled = true;
  stream.on('error', () => {});
}
function parsePositiveInt(value, fallback, min = 1, max = 65535) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}
function agentRuntimeInstallPlatformAliases() {
  const platform = process.platform;
  const aliases = new Set(['all', platform]);
  if (platform === 'darwin') aliases.add('macos');
  if (platform === 'win32') aliases.add('windows');
  return aliases;
}
function agentRuntimeInstallAllowed() {
  const raw = cleanText(process.env.INFRING_AGENT_RUNTIME_INSTALL_ALLOWED, 40).toLowerCase();
  return raw !== '0' && raw !== 'false' && raw !== 'no' && raw !== 'deny';
}
function selectAgentRuntimeInstallCommand(install) {
  const spec = install && install.command_line_install && typeof install.command_line_install === 'object'
    ? install.command_line_install
    : {};
  const commands = Array.isArray(spec.commands) ? spec.commands : [];
  const aliases = agentRuntimeInstallPlatformAliases();
  for (const candidate of commands) {
    const row = candidate && typeof candidate === 'object' ? candidate : {};
    const platforms = Array.isArray(row.platforms) && row.platforms.length ? row.platforms : ['all'];
    if (platforms.some((item) => aliases.has(cleanText(item, 40).toLowerCase()))) return row;
  }
  return null;
}
function captureAgentRuntimeInstall(commandSpec) {
  const row = commandSpec && typeof commandSpec === 'object' ? commandSpec : {};
  const shellCommand = cleanDisplayText(row.shell_command || '', 4000);
  const command = shellCommand
    ? (process.platform === 'win32' ? 'powershell.exe' : '/bin/sh')
    : cleanText(row.command || '', 500);
  const args = shellCommand
    ? (process.platform === 'win32' ? ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', shellCommand] : ['-lc', shellCommand])
    : (Array.isArray(row.args) ? row.args.map((item) => cleanText(item, 1000)) : []);
  if (!command) return Promise.resolve({ ok: false, exit_code: null, stdout: '', stderr: 'agent_runtime_install_command_missing' });
  const timeoutMs = parsePositiveInt(row.timeout_ms, 240000, 5000, 600000);
  const maxOutputBytes = parsePositiveInt(row.max_output_bytes, 24000, 1024, 65536);
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd: ROOT,
      env: { ...process.env },
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    const append = (current, chunk) => {
      const next = Buffer.concat([current, Buffer.from(chunk || '')]);
      return next.length > maxOutputBytes ? next.subarray(next.length - maxOutputBytes) : next;
    };
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      try { child.kill('SIGTERM'); } catch {}
      resolve({ ok: false, timed_out: true, exit_code: null, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    }, timeoutMs);
    child.stdout.on('data', (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
    child.on('error', (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: false, timed_out: false, exit_code: null, stdout: '', stderr: cleanText(error && error.message ? error.message : error, 2000) });
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: code === 0, timed_out: false, exit_code: code, stdout: stdout.toString('utf8'), stderr: stderr.toString('utf8') });
    });
  });
}

function projectAgentRuntimeEngineRow(engine, health) {
  const row = engine && typeof engine === 'object' ? engine : {};
  const install = row.install && typeof row.install === 'object' ? row.install : {};
  const engineId = cleanEngineId(row.engine_id);
  const healthStatus = cleanText(health && health.status ? health.status : '', 80);
  const registryStatus = cleanText(row.status || '', 80);
  const nativeReady = engineId === 'infring_native' && registryStatus === 'adapter_seam_ready';
  const status = healthStatus || (nativeReady ? 'available' : (registryStatus || 'unknown'));
  const selectable = status === 'available' || status === 'adapter_ready' || nativeReady;
  const downloadAvailable = install.download_available === true || (health && health.download_available === true);
  const commandLineInstall = selectAgentRuntimeInstallCommand(install);
  const installActionAvailable = cleanText(install.preferred_install_method || '', 80) === 'command_line' && !!commandLineInstall;
  const registryCapabilities = Array.isArray(row.capabilities) ? row.capabilities.map((item) => cleanText(item, 120)).filter(Boolean).slice(0, 12) : [];
  const supportsLiveSteering = (health && health.supports_live_steering === true) || row.supports_live_steering === true;
  const supportsNextTurnSteering = (health && health.supports_next_turn_steering === true) || row.supports_next_turn_steering === true || engineId !== 'infring_native';
  const steeringMode = supportsLiveSteering ? 'live' : supportsNextTurnSteering ? 'next_turn' : 'unsupported';
  const capabilities = registryCapabilities.slice();
  if (supportsLiveSteering && !capabilities.includes('live_steering')) capabilities.push('live_steering');
  if (supportsNextTurnSteering && !capabilities.includes('next_turn_steering')) capabilities.push('next_turn_steering');
  return {
    engine_id: engineId,
    display_name: cleanText(row.display_name || engineId, 120),
    engine_kind: cleanText(row.engine_kind || '', 120),
    transport_kind: cleanText(row.transport_kind || '', 120),
    status,
    selectable,
    capabilities: capabilities.slice(0, 14),
    supports_live_steering: supportsLiveSteering,
    supports_next_turn_steering: supportsNextTurnSteering,
    steering_mode: steeringMode,
    steering_transport: cleanText((health && health.steering_transport) || (steeringMode === 'next_turn' ? 'gateway_next_turn_intervention' : steeringMode), 120),
    download_available: !!downloadAvailable,
    install_action_available: !!installActionAvailable,
    command_line_install_available: !!commandLineInstall,
    install_permission_state: agentRuntimeInstallAllowed() ? 'allowed' : 'permission_required',
    download_action_ref: cleanText(install.download_action_ref || (health && health.download_action_ref) || '', 240),
    preferred_install_method: cleanText(install.preferred_install_method || '', 80),
    command_line_hint: cleanText(install.command_line_hint || '', 500),
    browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
    display_when_missing: cleanText(install.display_when_missing || (downloadAvailable ? 'download_icon' : ''), 80),
    version_preview: cleanText(health && health.version_preview ? health.version_preview : '', 240),
  };
}

async function agentRuntimeEnginesProjection(traceId) {
  const info = loadAgentRuntimeEngineRegistry(ROOT);
  const engines = Array.isArray(info.engines) ? info.engines : [];
  const engineAdapters = createAgentRuntimeEngineAdapterMap({ liveDispatch: false });
  const selection = loadAgentRuntimeSelection();
  const rows = [];
  for (const engine of engines) {
    const engineId = cleanEngineId(engine && engine.engine_id);
    let health = null;
    if (engineAdapters[engineId] && typeof engineAdapters[engineId].health_check === 'function') {
      health = await engineAdapters[engineId].health_check({
        message: {
          trace_id: traceId,
          request_id: `agent-runtime-menu:${Date.now()}`,
          engine_id: engineId,
          session_id: 'dashboard-menu',
        },
        engine,
      }).catch((error) => ({
        status: 'not_downloaded',
        download_available: true,
        reason: cleanText(error && error.message ? error.message : error, 200),
      }));
    }
    rows.push(projectAgentRuntimeEngineRow(engine, health));
  }
  return {
    ok: true,
    type: 'agent_runtime_engines_projection',
    trace_id: traceId,
    socket_route: '/ws/agent-runtime',
    selected_default_engine_id: selection.engine_id || 'infring_native',
    active_engine_id: selection.engine_id || 'infring_native',
    active_engine_updated_at: selection.updated_at || '',
    engines: rows,
  };
}
function agentRuntimeSelectionProjection(traceId, body) {
  const engineId = cleanEngineId(body && (body.engine_id || body.agent_runtime_engine_id || body.runtime_engine_id));
  if (!engineId) return { ok: false, status_code: 400, type: 'agent_runtime_selection_projection', trace_id: traceId, error: 'engine_id_required' };
  const info = loadAgentRuntimeEngineRegistry(ROOT);
  const engine = findAgentRuntimeEngine(info, engineId);
  if (!engine) return { ok: false, status_code: 404, type: 'agent_runtime_selection_projection', trace_id: traceId, engine_id: engineId, error: 'engine_not_registered' };
  const saved = saveAgentRuntimeSelection(engineId, traceId);
  return {
    ok: true,
    type: 'agent_runtime_selection_projection',
    trace_id: traceId,
    engine_id: saved.engine_id,
    updated_at: saved.updated_at,
    source: saved.source,
  };
}
function readAgentRuntimeSteeringRecords() {
  let raw = '';
  try { raw = fs.readFileSync(AGENT_RUNTIME_STEERING_PATH, 'utf8'); } catch { return []; }
  return raw.split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(-AGENT_RUNTIME_STEERING_MAX_RECORDS)
    .map((line) => {
      try { return JSON.parse(line); } catch { return null; }
    })
    .filter((row) => row && typeof row === 'object');
}
function writeAgentRuntimeSteeringRecords(rows) {
  ensureDir(path.dirname(AGENT_RUNTIME_STEERING_PATH));
  const serialized = (Array.isArray(rows) ? rows : [])
    .slice(-AGENT_RUNTIME_STEERING_MAX_RECORDS)
    .map((row) => JSON.stringify(row))
    .join('\n');
  fs.writeFileSync(AGENT_RUNTIME_STEERING_PATH, serialized ? `${serialized}\n` : '', 'utf8');
}
function agentRuntimeSteerProjection(traceId, body) {
  const engineId = cleanEngineId(body && (body.engine_id || body.agent_runtime_engine_id || body.runtime_engine_id)) || 'infring_native';
  const registry = loadAgentRuntimeEngineRegistry(ROOT);
  const engine = findAgentRuntimeEngine(registry, engineId);
  if (!engine) return { ok: false, status_code: 404, type: 'agent_runtime_steer_projection', trace_id: traceId, engine_id: engineId, error: 'agent_runtime_engine_unknown' };
  return queueAgentRuntimeSteeringIntervention(traceId, body);
}
function agentRuntimeWorkspaceProjection(traceId, body) {
  const requested = body && (body.workspace_dir || body.active_workspace || body.cwd || body.path);
  if (requested) {
    const saved = saveAgentRuntimeWorkspace(requested, traceId, body && body.scope);
    return saved;
  }
  return loadAgentRuntimeWorkspace(traceId);
}
function agentRuntimeWorkspacePickerProjection(traceId, body) {
  const picked = pickAgentRuntimeWorkspaceDirectory();
  if (!picked.ok) {
    const current = loadAgentRuntimeWorkspace(traceId);
    return {
      ...current,
      ok: false,
      status_code: picked.cancelled ? 409 : 502,
      type: 'agent_runtime_workspace_picker_projection',
      error: picked.cancelled ? 'workspace_picker_cancelled' : 'workspace_picker_failed',
      reason: cleanText(picked.reason, 240),
    };
  }
  const saved = saveAgentRuntimeWorkspace(picked.path, traceId, body && body.scope);
  return {
    ...saved,
    type: 'agent_runtime_workspace_picker_projection',
    picker: 'native_os_folder_dialog',
  };
}
async function agentRuntimeEngineInstallProjection(traceId, requestedEngineId) {
  const engineId = cleanEngineId(requestedEngineId);
  if (!engineId) return { ok: false, status_code: 400, type: 'agent_runtime_engine_install_projection', trace_id: traceId, status: 'engine_id_required' };
  const info = loadAgentRuntimeEngineRegistry(ROOT);
  const engine = findAgentRuntimeEngine(info, engineId);
  if (!engine) return { ok: false, status_code: 404, type: 'agent_runtime_engine_install_projection', trace_id: traceId, engine_id: engineId, status: 'engine_not_registered' };
  const install = engine.install && typeof engine.install === 'object' ? engine.install : {};
  const adapter = createAgentRuntimeEngineAdapterMap({ liveDispatch: false })[engineId];
  const healthMessage = { trace_id: traceId, request_id: `agent-runtime-install:${Date.now()}`, engine_id: engineId, session_id: 'dashboard-install' };
  const beforeHealth = adapter && typeof adapter.health_check === 'function'
    ? await adapter.health_check({ message: healthMessage, engine }).catch((error) => ({ status: 'health_check_failed', reason: cleanText(error && error.message ? error.message : error, 240) }))
    : null;
  if (beforeHealth && beforeHealth.status === 'available') {
    return {
      ok: true,
      type: 'agent_runtime_engine_install_projection',
      trace_id: traceId,
      engine_id: engineId,
      status: 'already_available',
      health: {
        status: 'available',
        discovery_source: cleanText(beforeHealth.discovery_source, 120),
        version_preview: cleanText(beforeHealth.version_preview, 240),
      },
    };
  }
  if (cleanText(install.preferred_install_method || '', 80) !== 'command_line') {
    return {
      ok: false,
      status_code: 409,
      type: 'agent_runtime_engine_install_projection',
      trace_id: traceId,
      engine_id: engineId,
      status: 'no_command_line_installer',
      browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
      command_line_hint: cleanText(install.command_line_hint || '', 500),
    };
  }
  const commandSpec = selectAgentRuntimeInstallCommand(install);
  if (!commandSpec) {
    return {
      ok: false,
      status_code: 409,
      type: 'agent_runtime_engine_install_projection',
      trace_id: traceId,
      engine_id: engineId,
      status: 'command_line_installer_unavailable_for_platform',
      browser_fallback_url: cleanText(install.browser_fallback_url || '', 500),
      command_line_hint: cleanText(install.command_line_hint || '', 500),
    };
  }
  if (!agentRuntimeInstallAllowed()) {
    return {
      ok: false,
      status_code: 403,
      type: 'agent_runtime_engine_install_projection',
      trace_id: traceId,
      engine_id: engineId,
      status: 'permission_required',
      command_line_hint: cleanText(install.command_line_hint || '', 500),
    };
  }
  const run = await captureAgentRuntimeInstall(commandSpec);
  const afterHealth = adapter && typeof adapter.health_check === 'function'
    ? await adapter.health_check({ message: healthMessage, engine }).catch((error) => ({ status: 'health_check_failed', reason: cleanText(error && error.message ? error.message : error, 240) }))
    : null;
  const available = afterHealth && afterHealth.status === 'available';
  return {
    ok: !!(run.ok && available),
    status_code: run.ok && available ? 200 : 502,
    type: 'agent_runtime_engine_install_projection',
    trace_id: traceId,
    engine_id: engineId,
    status: available ? 'installed_available' : (run.ok ? 'install_completed_but_not_available' : 'install_failed'),
    exit_code: run.exit_code,
    timed_out: !!run.timed_out,
    stdout_preview: cleanDisplayText(run.stdout, 2000),
    stderr_preview: cleanDisplayText(run.stderr, 2000),
    health: afterHealth ? {
      status: cleanText(afterHealth.status, 80),
      discovery_source: cleanText(afterHealth.discovery_source, 120),
      version_preview: cleanText(afterHealth.version_preview, 240),
      reason: cleanText(afterHealth.reason, 240),
    } : null,
  };
}
function normalizeShutdownExitDelayMs(value) {
  const num = Number(value);
  if (!Number.isFinite(num)) return DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS;
  if (!Number.isSafeInteger(Math.floor(num))) return DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS;
  return Math.max(
    DASHBOARD_SHUTDOWN_EXIT_DELAY_MIN_MS,
    Math.min(DASHBOARD_SHUTDOWN_EXIT_DELAY_MAX_MS, Math.floor(num)),
  );
}
function normalizeArgs(argv = process.argv.slice(2)) { return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : []; }
function defaultApiPort(port) {
  if (port + BACKEND_PORT_OFFSET <= 65535) return port + BACKEND_PORT_OFFSET;
  if (port - BACKEND_PORT_OFFSET >= 1) return port - BACKEND_PORT_OFFSET;
  return port === 65535 ? 65534 : port + 1;
}
function parseFlags(argv = []) {
  const out = {
    mode: 'serve',
    host: DEFAULT_HOST,
    port: DEFAULT_PORT,
    team: DEFAULT_TEAM,
    refreshMs: DEFAULT_REFRESH_MS,
    pretty: true,
    apiHost: '',
    apiPort: 0,
    apiReadyTimeoutMs: DEFAULT_BACKEND_READY_TIMEOUT_MS,
    uiMode: cleanText(process.env.INFRING_DASHBOARD_UI || 'primary', 24).toLowerCase(),
  };
  let modeSet = false;
  for (const token of argv) {
    const value = String(token || '').trim();
    if (!value) continue;
    if (!modeSet && !value.startsWith('--')) { out.mode = value.toLowerCase(); modeSet = true; continue; }
    if (value.startsWith('--host=')) out.host = cleanText(value.slice(7), 100) || DEFAULT_HOST;
    else if (value.startsWith('--port=')) out.port = parsePositiveInt(value.slice(7), DEFAULT_PORT);
    else if (value.startsWith('--team=')) out.team = cleanText(value.slice(7), 80) || DEFAULT_TEAM;
    else if (value.startsWith('--refresh-ms=')) out.refreshMs = parsePositiveInt(value.slice(13), DEFAULT_REFRESH_MS, 800, 60000);
    else if (value.startsWith('--api-host=')) out.apiHost = cleanText(value.slice(11), 100);
    else if (value.startsWith('--backend-host=')) out.apiHost = cleanText(value.slice(15), 100);
    else if (value.startsWith('--api-port=')) out.apiPort = parsePositiveInt(value.slice(11), 0);
    else if (value.startsWith('--backend-port=')) out.apiPort = parsePositiveInt(value.slice(15), 0);
    else if (value.startsWith('--api-ready-timeout-ms=')) out.apiReadyTimeoutMs = parsePositiveInt(value.slice(23), DEFAULT_BACKEND_READY_TIMEOUT_MS, 1500, 300000);
    else if (value.startsWith('--ui=')) out.uiMode = cleanText(value.slice(5), 24).toLowerCase();
    else if (value === '--pretty=0' || value === '--pretty=false') out.pretty = false;
  }
  out.uiMode = 'primary';
  out.apiHost = out.apiHost || out.host;
  out.apiPort = out.apiPort || defaultApiPort(out.port);
  if (out.apiPort === out.port) out.apiPort = defaultApiPort(out.port + 1);
  return out;
}
function ensureDir(dirPath) { fs.mkdirSync(dirPath, { recursive: true }); }
function writeJson(filePath, value) { ensureDir(path.dirname(filePath)); fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8'); }
function writeJsonIfMissing(filePath, value) {
  if (fs.existsSync(filePath)) return false;
  writeJson(filePath, value);
  return true;
}
function appendJsonl(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}
function appendBoundedJsonl(filePath, value, maxRows) {
  ensureDir(path.dirname(filePath));
  let rows = [];
  try {
    rows = fs.readFileSync(filePath, 'utf8').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  } catch {}
  rows.push(JSON.stringify(value));
  fs.writeFileSync(filePath, `${rows.slice(-Math.max(1, maxRows || 1)).join('\n')}\n`, 'utf8');
}
function deterministicReceiptHash(value) {
  try {
    return createHash('sha256').update(JSON.stringify(value)).digest('hex');
  } catch {
    return '';
  }
}
function readRecentActionRows(limit = TROUBLESHOOTING_MAX_RECENT) {
  const historyPath = path.resolve(STATUS_DIR, 'actions', 'history.jsonl');
  let raw = '';
  try {
    raw = fs.readFileSync(historyPath, 'utf8');
  } catch {
    return [];
  }
  const lines = raw
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (!lines.length) return [];
  const out = [];
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    let parsed = null;
    try {
      parsed = JSON.parse(lines[idx]);
    } catch {
      parsed = null;
    }
    if (!parsed || cleanText(parsed.action || '', 80) !== 'app.chat') continue;
    out.push(parsed);
    if (out.length >= limit) break;
  }
  return out.reverse();
}
function summarizeBootstrapActionRow(actionRow, previousSummary) {
  const laneOk = actionRow?.ok === true;
  const laneStatus = Number.isFinite(Number(actionRow?.lane_status))
    ? Number(actionRow.lane_status)
    : laneOk
      ? 0
      : 1;
  const payload = actionRow && typeof actionRow.payload === 'object' && actionRow.payload
    ? actionRow.payload
    : {};
  const input = cleanText(payload.input || payload.message || payload.prompt || '', 240);
  const tools = Array.isArray(actionRow?.tool_receipts)
    ? actionRow.tool_receipts
    : Array.isArray(payload?.tool_receipts)
      ? payload.tool_receipts
      : [];
  const toolSummary = tools
    .slice(0, 3)
    .map((row) => cleanText(row?.name || row?.tool || '', 40))
    .filter(Boolean)
    .join(',');
  const laneLabel = laneOk ? 'lane_ok' : `lane_fail(${laneStatus})`;
  const inputLabel = input ? `input:${cleanText(input, 64)}` : 'input:empty';
  const toolLabel = toolSummary ? `tools:${toolSummary}` : 'tools:none';
  const summary = `${laneLabel};${inputLabel};${toolLabel}`;
  if (!previousSummary) return summary;
  return `${summary};prev:${cleanText(previousSummary, 120)}`;
}
function bootstrapRecentWorkflowEntries() {
  const actionRows = readRecentActionRows(TROUBLESHOOTING_MAX_RECENT);
  const entries = [];
  let previousSummary = '';
  for (let index = 0; index < actionRows.length; index += 1) {
    const row = actionRows[index] || {};
    const payload = row && typeof row.payload === 'object' && row.payload ? row.payload : {};
    const laneOk = row?.ok === true;
    const laneStatus = Number.isFinite(Number(row?.lane_status))
      ? Number(row.lane_status)
      : laneOk
        ? 0
        : 1;
    const summary = summarizeBootstrapActionRow(row, previousSummary);
    previousSummary = summary;
    const entry = {
      workflow_id: cleanText(row?.id || `wf_${index + 1}`, 120) || `wf_${index + 1}`,
      source_sequence: index + 1,
      ts: cleanText(row?.ts || nowIso(), 80),
      lane_ok: laneOk,
      lane_status: laneStatus,
      error_code: cleanText(row?.error_code || row?.error || '', 120).toLowerCase(),
      exchange: {
        user: cleanText(payload.input || payload.message || payload.prompt || '', 1600),
        assistant: cleanText(row?.response || payload.response || '', 2000),
        tool_receipts: Array.isArray(row?.tool_receipts)
          ? row.tool_receipts.slice(0, 12)
          : [],
      },
      process_summary: {
        previous: cleanText(index === 0 ? '' : entries[index - 1]?.process_summary?.current || '', 360),
        current: cleanText(summary, 360),
        source: 'snapshot_compat_bootstrap',
      },
      metadata: {
        source: 'snapshot_compat_bootstrap',
      },
    };
    entry.receipt_hash = deterministicReceiptHash(entry);
    entries.push(entry);
  }
  return entries;
}
function writeBridgeOutput(out) {
  if (!out || typeof out !== 'object') return 1;
  if (out.stdout) process.stdout.write(String(out.stdout));
  if (out.stderr) process.stderr.write(String(out.stderr));
  if (out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  const status = Number(out.status);
  return Number.isFinite(status) ? status : 1;
}
function bootstrapTroubleshootingFromSnapshot(snapshotPayload) {
  const payload = snapshotPayload && typeof snapshotPayload === 'object' ? snapshotPayload : {};
  const seededEntries = bootstrapRecentWorkflowEntries();
  writeJsonIfMissing(TROUBLESHOOTING_RECENT_WORKFLOWS_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_recent_workflows',
    ts: nowIso(),
    entries: seededEntries,
    receipt_hash: deterministicReceiptHash({
      entries: seededEntries,
      type: 'dashboard_troubleshooting_recent_workflows',
    }),
  });
  writeJsonIfMissing(TROUBLESHOOTING_EVAL_QUEUE_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_eval_queue',
    ts: nowIso(),
    items: [],
    receipt_hash: deterministicReceiptHash({
      items: [],
      type: 'dashboard_troubleshooting_eval_queue',
    }),
  });
  writeJsonIfMissing(TROUBLESHOOTING_ISSUE_OUTBOX_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_issue_outbox',
    ts: nowIso(),
    items: [],
    receipt_hash: deterministicReceiptHash({
      items: [],
      type: 'dashboard_troubleshooting_issue_outbox',
    }),
  });
  if (!fs.existsSync(TROUBLESHOOTING_LATEST_SNAPSHOT_PATH)) {
    const failureCount = seededEntries.filter((row) => row?.lane_ok !== true).length;
    const snapshot = {
      ok: true,
      type: 'dashboard_troubleshooting_snapshot',
      snapshot_id: `snap_${Date.now().toString(36)}`,
      trigger: 'runtime_bootstrap_compat',
      ts: nowIso(),
      failure_count: failureCount,
      entry_count: seededEntries.length,
      entries: seededEntries,
      metadata: {
        source: 'dashboard_snapshot_compat_bootstrap',
        snapshot_receipt_hash: cleanText(payload.receipt_hash || '', 160),
      },
    };
    snapshot.receipt_hash = deterministicReceiptHash(snapshot);
    writeJson(TROUBLESHOOTING_LATEST_SNAPSHOT_PATH, snapshot);
    appendJsonl(TROUBLESHOOTING_SNAPSHOT_HISTORY_PATH, snapshot);
  }
  writeJsonIfMissing(TROUBLESHOOTING_LATEST_EVAL_REPORT_PATH, {
    ok: true,
    type: 'dashboard_troubleshooting_eval_report',
    ts: nowIso(),
    status: 'idle',
    reason: 'runtime_bootstrap_compat',
    model: TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
    model_source: 'strong_default_bootstrap',
    strong_default_model: TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
    entry_count: seededEntries.length,
    issues: [],
    summary: 'Eval runtime is initialized and waiting for failure snapshots.',
    receipt_hash: deterministicReceiptHash({
      status: 'idle',
      model: TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
      entry_count: seededEntries.length,
      type: 'dashboard_troubleshooting_eval_report',
    }),
  });
}
function runSnapshotWithCompatBootstrap(args, options) {
  const out = invokeInfringOpsViaBridge(['dashboard-ui', ...args], options);
  if (!out) {
    const status = runInfringOps(['dashboard-ui', ...args], options);
    if (Number(status) === 0 && fs.existsSync(STATUS_SNAPSHOT_PATH)) {
      try {
        const fallbackPayload = JSON.parse(fs.readFileSync(STATUS_SNAPSHOT_PATH, 'utf8'));
        if (fallbackPayload && typeof fallbackPayload === 'object') {
          bootstrapTroubleshootingFromSnapshot(fallbackPayload);
        }
      } catch {}
    }
    return status;
  }
  const parsedPayload = out.payload && typeof out.payload === 'object'
    ? out.payload
    : parseLastJson(out.stdout || '');
  if (parsedPayload && typeof parsedPayload === 'object') {
    bootstrapTroubleshootingFromSnapshot(parsedPayload);
    if (!out.payload) out.payload = parsedPayload;
  }
  return writeBridgeOutput(out);
}
function discoverSiblingAltDashboardSurfaces() {
  const out = [];
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return out; }
  for (const entry of rows) {
    if (!entry || typeof entry.isDirectory !== 'function' || !entry.isDirectory()) continue;
    const dirPath = path.resolve(DASHBOARD_DIR, String(entry.name || ''));
    if (!dirPath || dirPath === STATIC_DIR) continue;
    const dirName = path.basename(dirPath);
    const hasInlineDashboardRoot = hasPrimaryDashboardUi(dirPath);
    const hasBuildIndex = fs.existsSync(path.resolve(dirPath, 'build', 'index.html'));
    const hasIndexHtml = fs.existsSync(path.resolve(dirPath, 'index.html'));
    if (SIBLING_ALT_DASHBOARD_PATTERN.test(dirName) || hasInlineDashboardRoot || hasBuildIndex || hasIndexHtml) out.push(dirPath);
  }
  return out;
}
function assertNoAlternateDashboardSurfaces() {
  const found = new Set();
  FORBIDDEN_ALT_DASHBOARD_DIRS.filter((dirPath) => fs.existsSync(dirPath)).forEach((dirPath) => found.add(dirPath));
  discoverSiblingAltDashboardSurfaces().forEach((dirPath) => found.add(dirPath));
  if (found.size === 0) return;
  const labels = Array.from(found).map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
  throw new Error(`forbidden_dashboard_surface_present:${labels}`);
}
function assertSingleDashboardRoot() {
  if (!hasPrimaryDashboardUi(STATIC_DIR)) throw new Error('primary_dashboard_ui_missing');
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return; }
  const duplicateRoots = rows
    .filter((entry) => entry && typeof entry.isDirectory === 'function' && entry.isDirectory())
    .map((entry) => path.resolve(DASHBOARD_DIR, String(entry.name || '')))
    .filter((dirPath) => dirPath !== STATIC_DIR && hasPrimaryDashboardUi(dirPath));
  if (!duplicateRoots.length) return;
  const labels = duplicateRoots.map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
  throw new Error(`multiple_dashboard_roots_detected:${labels}`);
}
function assertDashboardSurfaceLocked() {
  assertNoAlternateDashboardSurfaces();
  assertSingleDashboardRoot();
}
function backendBase(flags) { return `http://${flags.apiHost}:${flags.apiPort}`; }
async function sleep(ms) { await new Promise((resolve) => setTimeout(resolve, ms)); }
async function fetchBackend(flags, pathname, init = {}, timeoutMs = 15000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try { return await fetch(`${backendBase(flags)}${pathname}`, { ...init, signal: controller.signal }); }
  finally { clearTimeout(timer); }
}
async function fetchBackendJson(flags, pathname, timeoutMs = 15000, traceId = '') {
  const cleanTraceId = sanitizeTraceId(traceId);
  const init = cleanTraceId
    ? { cache: 'no-store', headers: { 'x-infring-trace-id': cleanTraceId } }
    : { cache: 'no-store' };
  const res = await fetchBackend(flags, pathname, init, timeoutMs);
  if (!res.ok) throw new Error(`backend_http_${pathname}_${res.status}`);
  return res.json();
}
async function postBackendJson(flags, pathname, body, timeoutMs = 15000, traceId = '') {
  const cleanTraceId = sanitizeTraceId(traceId);
  const headers = { 'content-type': 'application/json' };
  if (cleanTraceId) headers['x-infring-trace-id'] = cleanTraceId;
  const res = await fetchBackend(flags, pathname, {
    method: 'POST',
    cache: 'no-store',
    headers,
    body: JSON.stringify(body || {}),
  }, timeoutMs);
  if (!res.ok) throw new Error(`backend_http_${pathname}_${res.status}`);
  return res.json();
}
function createGatewayNativeOrchestrationClient(flags) {
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
async function backendHealth(flags, timeoutMs = 5000) {
  try { return (await fetchBackend(flags, '/healthz', {}, timeoutMs)).ok; } catch { return false; }
}
function backendSpawnEnv() { return backendSpawnEnvForRoot(ROOT, process.env); }
function backendFreshnessSnapshot(flags) {
  return backendFreshnessSnapshotFromProcess(flags, { root: ROOT, resolveBinary, env: backendSpawnEnv() });
}
async function waitForBackendDown(flags, timeoutMs = 6000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!(await backendHealth(flags, 800))) return true;
    await sleep(150);
  }
  return !(await backendHealth(flags, 800));
}
async function stopStaleBackend(flags, freshness) {
  const rows = freshness && Array.isArray(freshness.listener_pids) ? freshness.listener_pids : [];
  const pids = rows.map((row) => Number(row && row.pid)).filter((pid) => Number.isInteger(pid) && pid > 0);
  if (!pids.length) return false;
  for (const pid of pids) {
    try { process.kill(pid, 'SIGTERM'); } catch {}
  }
  if (await waitForBackendDown(flags)) return true;
  for (const pid of pids) {
    try { process.kill(pid, 'SIGKILL'); } catch {}
  }
  return waitForBackendDown(flags);
}
async function statusPayloadWithBootStage(flags) {
  const startedAt = Date.now();
  const healthOk = await backendHealth(flags, 1200);
  if (!healthOk) {
    return {
      ok: false,
      error: 'backend_unreachable',
      connected: false,
      connection_state: 'disconnected',
      boot_stage: 'backend_unreachable',
      backend_health_ok: false,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
  try {
    const status = await fetchBackendJson(flags, '/api/status', 1800);
    const base = (status && typeof status === 'object') ? status : {};
    const connected = base.connected !== false;
    const degraded = !!base.degraded || base.ok === false;
    const out = {
      ...base,
      ok: connected,
      connected,
      degraded,
      connection_state: connected ? 'connected' : 'disconnected',
      boot_stage: cleanText(base.boot_stage || base.last_stage || (degraded ? 'status_degraded' : 'ready'), 60),
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
    };
    if (!out.error && degraded) out.error = 'status_degraded';
    return out;
  } catch {
    return {
      ok: true,
      degraded: true,
      warning: 'status_unavailable',
      connected: true,
      connection_state: 'connected',
      boot_stage: 'backend_ready_status_probe_timeout',
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
}
function spawnBackend(flags) {
  const laneArgs = ['dashboard-ui', 'serve', `--host=${flags.apiHost}`, `--port=${flags.apiPort}`, `--team=${flags.team}`, `--refresh-ms=${flags.refreshMs}`];
  const env = backendSpawnEnv();
  const bin = resolveBinary({ env });
  if (!bin) throw new Error('dashboard_backend_binary_missing');
  const child = spawn(bin, laneArgs, { cwd: ROOT, env, stdio: ['ignore', 'pipe', 'pipe'] });
  if (child.stdout) child.stdout.on('data', (chunk) => process.stdout.write(chunk));
  if (child.stderr) child.stderr.on('data', (chunk) => process.stderr.write(chunk));
  return child;
}
async function ensureBackend(flags) {
  if (await backendHealth(flags, 1500)) {
    const freshness = backendFreshnessSnapshot(flags);
    if (!freshness.stale) return { child: null, reused: true, freshness };
    if (!shouldRestartStaleBackend()) return { child: null, reused: true, freshness };
    const stopped = await stopStaleBackend(flags, freshness);
    if (!stopped) return { child: null, reused: true, freshness: { ...freshness, restart_failed: true } };
  }
  const child = spawnBackend(flags);
  const deadline = Date.now() + flags.apiReadyTimeoutMs;
  while (Date.now() < deadline) {
    if (await backendHealth(flags, 1500)) return { child, reused: false, freshness: backendFreshnessSnapshot(flags) };
    if (child.exitCode != null) throw new Error(`dashboard_backend_exit:${child.exitCode}`);
    await sleep(250);
  }
  try { child.kill('SIGTERM'); } catch {}
  throw new Error('dashboard_backend_timeout');
}
function sanitizeTraceId(value) {
  const raw = Array.isArray(value) ? value[0] : value;
  const clean = String(raw || '').trim();
  return /^[A-Za-z0-9_.:-]{8,160}$/.test(clean) ? clean : '';
}
function requestTraceId(req) {
  if (req.__infringTraceId) return req.__infringTraceId;
  const rawHeader = req.headers && (req.headers['x-infring-trace-id'] || req.headers['traceparent']);
  const existing = sanitizeTraceId(rawHeader);
  const minted = `trace_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 14)}`;
  req.__infringTraceId = existing || minted;
  req.__infringTraceBoundary = {
    trace_id: req.__infringTraceId,
    source: existing ? 'incoming_header' : 'gateway_minted',
    gateway_boundary: 'adapters.runtime.infring_dashboard',
  };
  return req.__infringTraceId;
}
function requestTraceBoundary(req) {
  requestTraceId(req);
  return req.__infringTraceBoundary || { trace_id: req.__infringTraceId || '', source: 'unknown' };
}
function sendJson(res, statusCode, value) {
  res.writeHead(statusCode, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
  res.end(`${JSON.stringify(value, null, 2)}\n`);
}
function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}
function readJsonBody(req, maxBytes = 65536) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0;
    ignoreStreamErrors(req);
    req.on('data', (chunk) => {
      const next = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      total += next.length;
      if (total > maxBytes) {
        reject(new Error('request_body_too_large'));
        return;
      }
      chunks.push(next);
    });
    req.on('end', () => {
      if (!chunks.length) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'));
      } catch {
        reject(new Error('request_body_invalid_json'));
      }
    });
    req.on('error', reject);
  });
}
function currentDashboardBuildInfo() {
  return readBuildVersionInfo(STATIC_DIR);
}
function mergeDashboardVersionPayload(payload) {
  const base = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  const build = currentDashboardBuildInfo();
  const version = cleanText(build && build.version, 120) || '0.0.0';
  const tag = cleanText(build && build.tag, 120) || `v${version}`;
  const source = cleanText(build && build.source, 80) || 'fallback_default';
  return {
    ...base,
    ok: base.ok !== false,
    version,
    tag,
    version_tag: tag,
    source,
    version_source: source,
    platform: base.platform || process.platform,
    arch: base.arch || process.arch,
  };
}
function filteredHeaders(headers, host, traceId = '') {
  const out = {};
  for (const [key, value] of Object.entries(headers || {})) {
    if (!value || HOP_BY_HOP.has(String(key).toLowerCase())) continue;
    out[key] = value;
  }
  out.host = host;
  if (traceId) out['x-infring-trace-id'] = traceId;
  return out;
}
function dashboardSystemActionArgs(action, payload = {}) {
  const normalized = cleanText(action, 40).toLowerCase();
  const body = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  if (normalized === 'restart') return ['restart', '--json'];
  if (normalized === 'shutdown') return ['stop', '--json'];
  if (normalized === 'update') {
    const args = ['update', '--json'];
    if (body.force === true) args.push('--force');
    if (body.apply !== false) args.push('--apply');
    return args;
  }
  throw new Error(`unknown_dashboard_system_action:${normalized}`);
}
function dashboardSystemActionEnv() {
  return {
    ...process.env,
    INFRING_ROOT: ROOT,
    INFRING_OPS_ALLOW_STALE: process.env.INFRING_OPS_ALLOW_STALE || '1',
    INFRING_NPM_ALLOW_STALE: process.env.INFRING_NPM_ALLOW_STALE || '1',
  };
}
function runDashboardSystemAction(action, payload = {}) {
  const args = dashboardSystemActionArgs(action, payload);
  const run =
    invokeInfringOpsViaBridge(args, {
      allowProcessFallback: false,
      unknownDomainFallback: false,
    }) || {
      status: 1,
      stdout: '',
      stderr: 'resident_ipc_bridge_unavailable',
      payload: null,
    };
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  const receipt = (run && run.payload && typeof run.payload === 'object') ? run.payload : parseLastJson(run.stdout);
  const ok = status === 0 && (!receipt || receipt.ok !== false);
  const error = ok
    ? ''
    : cleanText(
        (receipt && receipt.error) || run.stderr || run.stdout || `${cleanText(action, 40).toLowerCase()}_failed`,
        260,
      );
  return {
    ok,
    type: 'dashboard_system_action',
    action: cleanText(action, 40).toLowerCase(),
    command: args[0],
    args: args.slice(1),
    exit_code: status,
    payload: receipt || null,
    error,
  };
}
function dispatchDashboardSystemAction(action, payload = {}) {
  const args = dashboardSystemActionArgs(action, payload);
  const env = dashboardSystemActionEnv();
  const bin = resolveBinary({ env });
  if (!bin) {
    return {
      ok: false,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: '',
      args: args.slice(1),
      error: 'dashboard_backend_binary_missing',
    };
  }
  try {
    const child = spawn(bin, args, {
      cwd: ROOT,
      env,
      detached: true,
      stdio: 'ignore',
    });
    if (child && typeof child.unref === 'function') child.unref();
    return {
      ok: true,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: path.basename(bin),
      args: args.slice(1),
      dispatch_mode: 'detached_subprocess',
      pid: Number(child && child.pid) || 0,
      payload: null,
      error: '',
    };
  } catch (error) {
    return {
      ok: false,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: path.basename(String(bin || '')),
      args: args.slice(1),
      error: cleanText(error && error.message ? error.message : String(error), 260),
    };
  }
}
function scheduleDashboardHostExit(cleanup, normalizedDelayMs = DASHBOARD_SHUTDOWN_EXIT_DELAY_DEFAULT_MS) {
  const waitMs = normalizeShutdownExitDelayMs(normalizedDelayMs);
  setTimeout(() => {
    try { cleanup(); } catch {}
    setTimeout(() => {
      try { process.exit(0); } catch {}
    }, 0);
  }, waitMs);
}
function proxyToBackend(req, res, flags) {
  return new Promise((resolve, reject) => {
    ignoreStreamErrors(req);
    ignoreStreamErrors(res);
    ignoreStreamErrors(req.socket);
    ignoreStreamErrors(res.socket);
    const upstream = http.request({ host: flags.apiHost, port: flags.apiPort, method: req.method || 'GET', path: req.url || '/', headers: filteredHeaders(req.headers, `${flags.apiHost}:${flags.apiPort}`, requestTraceId(req)) }, (upstreamRes) => {
      ignoreStreamErrors(upstreamRes);
      ignoreStreamErrors(upstreamRes.socket);
      res.writeHead(upstreamRes.statusCode || 502, upstreamRes.headers);
      upstreamRes.pipe(res);
      upstreamRes.on('end', resolve);
      upstreamRes.on('error', reject);
    });
    ignoreStreamErrors(upstream);
    upstream.on('error', reject);
    req.pipe(upstream);
  });
}
function proxyUpgrade(req, socket, head, flags) {
  ignoreStreamErrors(req);
  ignoreStreamErrors(req.socket);
  ignoreStreamErrors(socket);
  const upstream = http.request({
    host: flags.apiHost,
    port: flags.apiPort,
    path: req.url || '/',
    headers: { ...filteredHeaders(req.headers, `${flags.apiHost}:${flags.apiPort}`, requestTraceId(req)), connection: 'Upgrade', upgrade: req.headers.upgrade || 'websocket' },
  });
  upstream.on('upgrade', (upstreamRes, upstreamSocket, upstreamHead) => {
    ignoreStreamErrors(upstreamRes);
    ignoreStreamErrors(upstreamSocket);
    const headerLines = [`HTTP/1.1 ${upstreamRes.statusCode || 101} ${upstreamRes.statusMessage || 'Switching Protocols'}`];
    for (const [key, value] of Object.entries(upstreamRes.headers || {})) {
      if (Array.isArray(value)) value.forEach((entry) => headerLines.push(`${key}: ${entry}`));
      else if (value != null) headerLines.push(`${key}: ${value}`);
    }
    socket.write(`${headerLines.join('\r\n')}\r\n\r\n`);
    if (head && head.length) upstreamSocket.write(head);
    if (upstreamHead && upstreamHead.length) socket.write(upstreamHead);
    upstreamSocket.pipe(socket).pipe(upstreamSocket);
  });
  upstream.on('response', (upstreamRes) => {
    ignoreStreamErrors(upstreamRes);
    socket.write(`HTTP/1.1 ${upstreamRes.statusCode || 502} ${upstreamRes.statusMessage || 'Bad Gateway'}\r\nConnection: close\r\n\r\n`);
    upstreamRes.pipe(socket);
  });
  upstream.on('error', () => { try { socket.destroy(); } catch {} });
  upstream.end();
}
async function runServe(flags) {
  assertDashboardSurfaceLocked();
  let dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR);
  if (!dashboardHtml.trim()) throw new Error('primary_dashboard_html_empty');
  const backend = {
    child: null,
    reused: false,
    ready: await backendHealth(flags, 1500),
    freshness: null,
    startup_error: '',
  };
  let backendStartPromise = null;
  if (!backend.ready) {
    backendStartPromise = ensureBackend(flags)
      .then((result) => {
        backend.child = result && result.child ? result.child : null;
        backend.reused = !!(result && result.reused);
        backend.ready = true;
        backend.freshness = result && result.freshness ? result.freshness : null;
        backend.startup_error = '';
        return result;
      })
      .catch((error) => {
        backend.ready = false;
        backend.startup_error = cleanText(error && error.message ? error.message : String(error), 200);
        return null;
      });
  } else {
    try {
      const result = await ensureBackend(flags);
      backend.child = result && result.child ? result.child : null;
      backend.reused = !!(result && result.reused);
      backend.ready = true;
      backend.freshness = result && result.freshness ? result.freshness : null;
      backend.startup_error = '';
    } catch (error) {
      backend.reused = true;
      backend.freshness = backendFreshnessSnapshot(flags);
      backend.startup_error = cleanText(error && error.message ? error.message : String(error), 200);
    }
  }
  const wsBridge = createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson });
  const status = {
    ok: true,
    type: 'infring_dashboard_server',
    ts: nowIso(),
    url: `http://${flags.host}:${flags.port}/dashboard`,
    host: flags.host,
    port: flags.port,
    refresh_ms: flags.refreshMs,
    team: flags.team,
    authority: 'primary_dashboard_ui_over_rust_core_api',
    dashboard_ui_mode_requested: flags.uiMode,
    dashboard_ui_mode_active: 'primary',
    backend_url: backendBase(flags),
    backend_reused: backend.reused,
    backend_ready: backend.ready,
    backend_freshness: backend.freshness,
    backend_start_pending: !!backendStartPromise,
    backend_start_error: '',
    ws_bridge_enabled: !!wsBridge.ws_enabled,
    ws_bridge_error: cleanText(wsBridge.ws_error || '', 120),
    dashboard_static_dir: path.basename(STATIC_DIR),
    status_path: path.relative(ROOT, STATUS_PATH),
  };
  function persistStatus() {
    status.backend_reused = backend.reused;
    status.backend_ready = backend.ready;
    status.backend_freshness = backend.freshness;
    status.backend_start_pending = !!backendStartPromise && !backend.ready && !backend.startup_error;
    status.backend_start_error = backend.startup_error;
    ensureDir(STATUS_DIR);
    writeJson(STATUS_PATH, status);
  }
  if (backendStartPromise) {
    backendStartPromise.finally(() => {
      try { persistStatus(); } catch {}
    });
  }
  const server = http.createServer(async (req, res) => {
    const requestUrl = new URL(req.url || '/', `http://${flags.host}:${flags.port}`);
    const pathname = requestUrl.pathname;
    const traceId = requestTraceId(req);
    try { res.setHeader('x-infring-trace-id', traceId); } catch {}
    try { res.setHeader('x-infring-trace-source', requestTraceBoundary(req).source || 'unknown'); } catch {}
    try {
      if ((req.method === 'GET' || req.method === 'HEAD') && (pathname === '/dashboard-classic' || pathname === '/dashboard-shell')) {
        res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' });
        res.end(req.method === 'HEAD' ? '' : 'dashboard_surface_retired');
        return;
      }
      if ((req.method === 'GET' || req.method === 'HEAD') && pathname === '/') {
        res.writeHead(302, { location: '/dashboard', 'cache-control': 'no-store' });
        res.end();
        return;
      }
      if (
        (req.method === 'GET' || req.method === 'HEAD') &&
        (pathname === '/dashboard' || pathname === '/dashboard/' || (pathname.startsWith('/dashboard/') && !path.extname(pathname)))
      ) {
        dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR) || dashboardHtml;
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(req.method === 'HEAD' ? '' : dashboardHtml);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/status') {
        const status = mergeDashboardVersionPayload(await statusPayloadWithBootStage(flags));
        return void sendJson(res, 200, status);
      }
      if (req.method === 'GET' && pathname === '/api/version') {
        const versionPayload = await fetchBackendJson(flags, '/api/version', 4000).catch(() => ({ ok: true }));
        return void sendJson(res, 200, mergeDashboardVersionPayload(versionPayload));
      }
      if (req.method === 'GET' && pathname === '/api/system/release-check') {
        const qs = requestUrl.search || '';
        const payload = await fetchBackendJson(flags, `/api/update/check${qs}`, 5000).catch(() => ({
          ok: true,
          update_available: false,
        }));
        return void sendJson(res, 200, payload);
      }
      const legacyAgentSessionMatch = pathname.match(/^\/api\/agents\/([^/]+)\/session$/);
      if (req.method === 'GET' && legacyAgentSessionMatch) {
        const agentId = decodeURIComponent(legacyAgentSessionMatch[1] || '');
        const upstreamPath = `${pathname}${requestUrl.search || ''}`;
        const payload = await fetchBackendJson(flags, upstreamPath, 10000, traceId).catch((error) => ({
          ok: false,
          type: 'agent_session_projection_unavailable',
          trace_id: traceId,
          error: cleanText(error && error.message ? error.message : error, 240),
        }));
        const sessionId = payload && (payload.session_id || payload.current_session_id || (payload.session && payload.session.id));
        const merged = mergeAgentRuntimeTranscriptPayload(payload, {
          agentId,
          sessionId,
          limit: requestUrl.searchParams.get('limit'),
        });
        return void sendJson(res, merged.ok === false ? 502 : 200, merged);
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/turn/stream' || pathname === '/api/agent-runtime/turn/stream')) {
        const body = await readJsonBody(req, 65536);
        res.writeHead(200, {
          'content-type': 'application/x-ndjson; charset=utf-8',
          'cache-control': 'no-store',
          'x-accel-buffering': 'no',
        });
        const writeEvent = (event) => {
          if (res.writableEnded || res.destroyed) return;
          try { res.write(`${JSON.stringify(event)}\n`); } catch {}
        };
        writeEvent({ type: 'start', trace_id: traceId, route: 'agent_runtime.turn.stream' });
        const payload = await agentRuntimeTurnProjection(traceId, body, {
          stream: true,
          nativeOrchestrationClient: createGatewayNativeOrchestrationClient(flags),
          onActivity: (event) => writeEvent({ type: 'activity', trace_id: traceId, event }),
        }).catch((error) => ({
          ok: false,
          status_code: 502,
          type: 'agent_runtime_turn_stream_error',
          trace_id: traceId,
          error: cleanText(error && error.message ? error.message : error, 240),
        }));
        writeEvent({ type: 'final', trace_id: traceId, payload });
        if (!res.writableEnded && !res.destroyed) res.end();
        return;
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/turn' || pathname === '/api/agent-runtime/turn')) {
        const body = await readJsonBody(req, 65536);
        const payload = await agentRuntimeTurnProjection(traceId, body, {
          nativeOrchestrationClient: createGatewayNativeOrchestrationClient(flags),
        }).catch((error) => ({
          ok: false,
          status_code: 502,
          type: 'agent_runtime_turn_projection_error',
          trace_id: traceId,
          error: cleanText(error && error.message ? error.message : error, 240),
        }));
        return void sendJson(res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/steer' || pathname === '/api/agent-runtime/steer')) {
        const body = await readJsonBody(req, 65536).catch(() => ({}));
        const payload = agentRuntimeSteerProjection(traceId, body);
        return void sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/context-pack/preview' || pathname === '/api/agent-runtime/context-pack/preview')) {
        const body = await readJsonBody(req, 65536).catch(() => ({}));
        const payload = await agentRuntimeContextPackPreviewProjection(traceId, body).catch((error) => ({
          ok: false,
          status_code: 502,
          type: 'agent_runtime_context_pack_preview_error',
          trace_id: traceId,
          error: cleanText(error && error.message ? error.message : error, 240),
        }));
        return void sendJson(res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      }
      const approvalDecisionMatch = pathname.match(/^\/api\/shell-socket\/approvals\/([^/]+)\/decision$/);
      if (req.method === 'POST' && approvalDecisionMatch) {
        const body = await readJsonBody(req, 327680).catch(() => ({}));
        const approvalId = decodeURIComponent(approvalDecisionMatch[1] || '');
        const payload = agentRuntimeApprovalDecisionProjection(traceId, approvalId, body);
        return void sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      }
      const agentRuntimeInstallMatch = pathname.match(/^\/api\/(?:shell-socket\/)?agent-runtime\/engines\/([^/]+)\/install$/);
      if (req.method === 'POST' && agentRuntimeInstallMatch) {
        await readJsonBody(req, 8192).catch(() => ({}));
        const engineId = decodeURIComponent(agentRuntimeInstallMatch[1] || '');
        const payload = await agentRuntimeEngineInstallProjection(traceId, engineId).catch((error) => ({
          ok: false,
          status_code: 502,
          type: 'agent_runtime_engine_install_projection_error',
          trace_id: traceId,
          engine_id: cleanEngineId(engineId),
          error: cleanText(error && error.message ? error.message : error, 240),
        }));
        return void sendJson(res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      }
      if (req.method === 'GET' && (pathname === '/api/shell-socket/agent-runtime/engines' || pathname === '/api/agent-runtime/engines')) {
        const payload = await agentRuntimeEnginesProjection(traceId).catch((error) => ({
          ok: false,
          type: 'agent_runtime_engines_projection_error',
          trace_id: traceId,
          error: cleanText(error && error.message ? error.message : error, 240),
          engines: [],
        }));
        return void sendJson(res, payload.ok === false ? 503 : 200, payload);
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/selection' || pathname === '/api/agent-runtime/selection')) {
        const body = await readJsonBody(req, 8192).catch(() => ({}));
        const payload = agentRuntimeSelectionProjection(traceId, body);
        return void sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      }
      if (req.method === 'GET' && (pathname === '/api/shell-socket/agent-runtime/workspace' || pathname === '/api/agent-runtime/workspace')) {
        const payload = agentRuntimeWorkspaceProjection(traceId, {});
        return void sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/workspace' || pathname === '/api/agent-runtime/workspace')) {
        const body = await readJsonBody(req, 8192).catch(() => ({}));
        const payload = agentRuntimeWorkspaceProjection(traceId, body);
        return void sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      }
      if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/workspace/pick' || pathname === '/api/agent-runtime/workspace/pick')) {
        const body = await readJsonBody(req, 8192).catch(() => ({}));
        const payload = agentRuntimeWorkspacePickerProjection(traceId, body);
        return void sendJson(res, payload.status_code || (payload.ok === false ? 409 : 200), payload);
      }
      if (req.method === 'GET' && isShellSocketChatProjectionPath(pathname)) {
        const result = await shellSocketChatProjection({ flags, requestUrl, traceId, fetchBackendJson });
        const filter = agentRuntimeTranscriptFilterFromShellSocketPath(pathname);
        const payload = filter
          ? mergeAgentRuntimeTranscriptPayload(result.payload, {
            agentId: filter.agentId,
            sessionId: filter.sessionId,
            limit: requestUrl.searchParams.get('limit'),
          })
          : result.payload;
        return void sendJson(res, result.status, payload);
      }
      if (req.method === 'GET' && isShellSocketStatusProjectionPath(pathname)) {
        const result = await shellSocketStatusProjection({ flags, traceId, fetchBackendJson, statusPayloadWithBootStage });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'POST' && isShellSocketCommandIngressPath(pathname)) {
        const body = await readJsonBody(req, 65536);
        const result = await shellSocketCommandIngress({ flags, requestUrl, traceId, body, fetchBackend });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'POST' && isShellSocketEvalIssueIngressPath(pathname)) {
        const body = await readJsonBody(req, 65536);
        const result = await shellSocketEvalIssueIngress({ flags, traceId, body, fetchBackend });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'POST' && isShellSocketLifecycleIngressPath(pathname)) {
        const body = await readJsonBody(req, 65536);
        const result = await shellSocketLifecycleIngress({ flags, requestUrl, traceId, body, fetchBackend });
        return void sendJson(res, result.status, result.payload);
      }
      if (req.method === 'GET') {
        const agentSessionsMatch = pathname.match(/^\/api\/agents\/([^/]+)\/sessions$/);
        if (agentSessionsMatch) {
          const rawAgentId = String(agentSessionsMatch[1] || '').trim();
          let decodedAgentId = rawAgentId;
          try { decodedAgentId = decodeURIComponent(rawAgentId); } catch {}
          const normalizedAgentId = cleanText(decodedAgentId, 120).toLowerCase();
          if (normalizedAgentId === 'system') {
            return void sendJson(res, 200, {
              ok: true,
              agent_id: 'system',
              sessions: [],
              system_thread: true,
            });
          }
        }
      }
      if (req.method === 'GET' && pathname === '/api/config') {
        const config = await fetchBackendJson(flags, '/api/config', 8000).catch(() => ({ ok: false, error: 'config_unavailable' }));
        return void sendJson(res, 200, config);
      }
      if (req.method === 'GET' && pathname === '/api/config/schema') {
        const schema = await fetchBackendJson(flags, '/api/config/schema', 8000).catch(() => ({ ok: true, sections: {} }));
        return void sendJson(res, 200, schema);
      }
      if (req.method === 'GET' && pathname === '/api/auth/check') {
        const auth = await fetchBackendJson(flags, '/api/auth/check', 8000).catch(() => ({ ok: true, mode: 'none', authenticated: true, user: 'operator' }));
        return void sendJson(res, 200, auth);
      }
      if (req.method === 'POST' && pathname === '/api/system/restart') {
        const body = await readJsonBody(req);
        const result = dispatchDashboardSystemAction('restart', body);
        return void sendJson(res, result.ok ? 200 : 500, result);
      }
      if (req.method === 'POST' && pathname === '/api/system/update') {
        const body = await readJsonBody(req);
        try {
          const upstream = await fetchBackend(flags, '/api/system/update', {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(body || {})
          }, body && body.apply === false ? 8000 : 3500);
          const text = await upstream.text();
          let payload = {};
          try {
            payload = text ? JSON.parse(text) : {};
          } catch {
            payload = {};
          }
          return void sendJson(
            res,
            upstream.status || ((payload && payload.ok === false) ? 400 : 200),
            payload && typeof payload === 'object' ? payload : { ok: upstream.ok }
          );
        } catch (_) {
          const result = runDashboardSystemAction('update', body);
          return void sendJson(res, result.ok ? 200 : 500, result);
        }
      }
      if (req.method === 'POST' && pathname === '/api/system/shutdown') {
        const body = await readJsonBody(req);
        const result = dispatchDashboardSystemAction('shutdown', body);
        sendJson(res, result.ok ? 200 : 500, result);
        if (result.ok) {
          const exitDelayMs = normalizeShutdownExitDelayMs(body && body.exit_delay_ms);
          scheduleDashboardHostExit(cleanup, exitDelayMs);
        }
        return;
      }
      if (req.method === 'GET') {
        const asset = readPrimaryDashboardAsset(STATIC_DIR, pathname);
        if (asset) {
          res.writeHead(200, { 'content-type': asset.contentType, 'cache-control': 'no-store' });
          res.end(asset.body);
          return;
        }
      }
      if (pathname === '/healthz' || pathname.startsWith('/api/')) return void await proxyToBackend(req, res, flags);
      sendJson(res, 404, { ok: false, type: 'infring_dashboard_not_found', path: pathname });
    } catch (error) {
      const message = cleanText(error && error.message ? error.message : String(error), 260);
      const statusCode = message === 'request_body_invalid_json' || message === 'request_body_too_large' ? 400 : 500;
      sendJson(res, statusCode, { ok: false, type: 'infring_dashboard_request_error', trace_id: traceId, error: message });
    }
  });
  server.on('upgrade', (req, socket, head) => {
    if (wsBridge.tryHandle(req, socket, head)) return;
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    if (!pathname.startsWith('/api/')) { socket.destroy(); return; }
    proxyUpgrade(req, socket, head, flags);
  });
  server.on('clientError', (_error, socket) => {
    try { socket.destroy(); } catch {}
  });
  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    try { server.close(); } catch {}
    if (backend.child && backend.child.exitCode == null) { try { backend.child.kill('SIGTERM'); } catch {} }
  };
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);
  process.on('exit', cleanup);
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(flags.port, flags.host, () => {
      server.off('error', reject);
      persistStatus();
      console.log(JSON.stringify(status, null, 2));
      console.log(`Dashboard listening at ${status.url}`);
      resolve(null);
    });
  });
}
async function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const flags = parseFlags(args);
  if (flags.mode === 'serve' || flags.mode === 'web') { await runServe(flags); return null; }
  const opsOptions = {
    unknownDomainFallback: true,
    env: {
      INFRING_OPS_USE_PREBUILT: process.env.INFRING_OPS_USE_PREBUILT || '0',
      INFRING_OPS_LOCAL_TIMEOUT_MS: process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000',
    },
  };
  if (flags.mode === 'snapshot') return runSnapshotWithCompatBootstrap(args, opsOptions);
  return runInfringOps(['dashboard-ui', ...args], opsOptions);
}
module.exports = {
  cleanText,
  backendFreshnessSnapshot,
  currentDashboardBuildInfo,
  dashboardSystemActionArgs,
  isTransientSocketError,
  mergeDashboardVersionPayload,
  normalizeArgs,
  parseFlags,
  dispatchDashboardSystemAction,
  run,
  runDashboardSystemAction,
  scheduleDashboardHostExit,
};
if (require.main === module) {
  process.on('uncaughtException', (error) => {
    if (isTransientSocketError(error)) {
      console.error(cleanText(`dashboard_host_socket:${error.code || 'unknown'}`, 280));
      return;
    }
    console.error(cleanText(error && error.message ? error.message : String(error), 280));
    process.exitCode = 1;
  });
  Promise.resolve(run(process.argv.slice(2)))
    .then((exitCode) => { if (typeof exitCode === 'number') process.exitCode = exitCode; })
    .catch((error) => { console.error(cleanText(error && error.message ? error.message : String(error), 280)); process.exitCode = 1; });
}
