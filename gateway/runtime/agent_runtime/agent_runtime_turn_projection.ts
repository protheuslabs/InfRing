#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::turn-projection.
//
// Gateway owns the Shell-facing Agent Runtime turn projection. The legacy
// dashboard host may still serve HTTP while it is being retired, but turn
// normalization, context materialization, activity projection, permission pause
// shaping, and bounded output packaging belong to the Gateway membrane.

'use strict';

const { normalizeAgentRuntimeTurnInput: defaultNormalizeAgentRuntimeTurnInput } = require('../agent_runtime_input_normalizer.ts');
const { buildUniversalToolGrants: defaultBuildUniversalToolGrants } = require('./universal_core_tools.ts');

const DEFAULT_CONTEXT_FANOUT_TARGET = 7;

function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanApprovalId(value) { return cleanText(value, 260).replace(/[^a-zA-Z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, ''); }

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

function defaultContextPack(input = {}) {
  return {
    source_basis: 'gateway_runtime_default_context_pack',
    source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    row_count: 0,
    fanout_target: DEFAULT_CONTEXT_FANOUT_TARGET,
    fragments: [],
    frontier: {
      hot_atom_refs: [],
      warm_span_refs: [],
      cool_span_refs: [],
      cold_span_refs: [],
    },
    universal_tool_grants: {
      tools: [],
      source_authority: 'gateway.runtime.agent_runtime_turn_projection',
    },
    trace_id: cleanText(input.traceId, 200),
    agent_id: cleanText(input.agentId, 160),
    session_id: cleanText(input.sessionId, 200),
  };
}

function normalizeContextPack(pack, fallback) {
  const contextPack = pack && typeof pack === 'object' ? pack : fallback;
  if (!Array.isArray(contextPack.fragments)) contextPack.fragments = [];
  if (!contextPack.frontier || typeof contextPack.frontier !== 'object') contextPack.frontier = {};
  for (const key of ['hot_atom_refs', 'warm_span_refs', 'cool_span_refs', 'cold_span_refs']) {
    if (!Array.isArray(contextPack.frontier[key])) contextPack.frontier[key] = [];
  }
  if (!contextPack.universal_tool_grants || typeof contextPack.universal_tool_grants !== 'object') {
    contextPack.universal_tool_grants = { tools: [], source_authority: 'gateway.runtime.agent_runtime_turn_projection' };
  }
  if (!Array.isArray(contextPack.universal_tool_grants.tools)) contextPack.universal_tool_grants.tools = [];
  return contextPack;
}

function createAgentRuntimeTurnProjectionStore(deps = {}) {
  const root = deps.root || process.cwd();
  const contextFanoutTarget = Number(deps.contextFanoutTarget) || DEFAULT_CONTEXT_FANOUT_TARGET;
  const normalizeAgentRuntimeTurnInput = deps.normalizeAgentRuntimeTurnInput || defaultNormalizeAgentRuntimeTurnInput;
  const buildUniversalToolGrants = deps.buildUniversalToolGrants || defaultBuildUniversalToolGrants;
  const noop = () => {};

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
    const registry = deps.loadAgentRuntimeEngineRegistry ? deps.loadAgentRuntimeEngineRegistry(root) : { engines: [] };
    const engine = deps.findAgentRuntimeEngine
      ? deps.findAgentRuntimeEngine(registry, engineId)
      : null;
    if (!engine) {
      return {
        ok: false,
        status_code: 404,
        error: 'agent_runtime_engine_unknown',
        trace_id: traceId,
        engine_id: engineId,
      };
    }
    const workspace = deps.loadAgentRuntimeWorkspace ? deps.loadAgentRuntimeWorkspace(traceId) : { workspace_dir: root, active_workspace: root };
    const activeWorkspaceDir = workspace.workspace_dir || workspace.active_workspace || root;
    const router = deps.createRouter({
      liveDispatch: true,
      nativeOrchestrationClient: options.nativeOrchestrationClient,
      cwd: activeWorkspaceDir,
    });
    const streamedActivityEvents = [];
    const activityDefaults = { engineId, traceId, sessionId, turnId };
    const sanitizeActivity = deps.sanitizeAgentRuntimeActivityEvent || ((event) => event);
    const onActivity = (event) => {
      const normalized = sanitizeActivity(event, streamedActivityEvents.length, activityDefaults);
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
      (deps.appendAgentRuntimeTranscriptTurn || noop)({
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
      (deps.appendAgentRuntimeTurnAtoms || noop)({
        root,
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
      (deps.ingestAgentRuntimeContextProjection || noop)({
        root,
        sessionId,
        agentId,
        traceId,
        projection: body && body.context_projection,
      });
    } catch {}
    const fallbackContextRows = deps.loadAgentRuntimeContextRows
      ? deps.loadAgentRuntimeContextRows({ root, sessionId, agentId })
      : [];
    emitSyntheticActivity(
      'activity',
      'context.loaded',
      `Loaded ${fallbackContextRows.length} prior context row${fallbackContextRows.length === 1 ? '' : 's'} for ${engineId}.`,
    );
    const kernelContext = await (deps.materializeKernelAgentRuntimeContextPack
      ? deps.materializeKernelAgentRuntimeContextPack({
        root,
        sessionId,
        agentId,
        traceId,
        atoms: fallbackContextRows,
        timeoutMs: 8000,
      })
      : Promise.resolve({ ok: false, reason: 'kernel_materializer_unavailable' })
    ).catch((error) => ({
      ok: false,
      reason: cleanText(error && error.message ? error.message : error, 240),
    }));
    const fallbackContextPack = () => deps.materializeAgentRuntimeContextPack
      ? Promise.resolve(deps.materializeAgentRuntimeContextPack({
        root,
        sessionId,
        agentId,
        traceId,
      })).catch(() => (deps.buildAgentRuntimeContextPack || defaultContextPack)({ body, agentId, sessionId, traceId }))
      : Promise.resolve((deps.buildAgentRuntimeContextPack || defaultContextPack)({ body, agentId, sessionId, traceId }));
    let contextPack = kernelContext && kernelContext.ok && kernelContext.context_pack
      ? kernelContext.context_pack
      : await fallbackContextPack();
    contextPack = normalizeContextPack(contextPack, defaultContextPack({ body, agentId, sessionId, traceId }));
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
    const permissionPolicy = deps.mergeAgentRuntimeApprovalPermissionPolicy
      ? deps.mergeAgentRuntimeApprovalPermissionPolicy(body && body.permission_policy, sessionId, engineId)
      : body && body.permission_policy;
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
    const steeringInterventions = deps.drainAgentRuntimeSteeringInterventions
      ? deps.drainAgentRuntimeSteeringInterventions({
        agentId,
        sessionId,
        engineId,
        traceId,
        turnId,
      })
      : [];
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
        context_pack_fanout_target: contextFanoutTarget,
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
      ? turn.activity_events.map((event, index) => sanitizeActivity(event, index, activityDefaults))
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
      (deps.appendAgentRuntimeTurnAtoms || noop)({
        root,
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
      (deps.appendAgentRuntimeTranscriptTurn || noop)({
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
        proposal_arguments: deps.sanitizeAgentRuntimeProposalArguments
          ? deps.sanitizeAgentRuntimeProposalArguments(pendingPermissionRequest.proposal_arguments)
          : {},
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

  return {
    agentRuntimeTurnProjection,
    agentRuntimePreTurnFailureProjection,
  };
}

module.exports = {
  createAgentRuntimeTurnProjectionStore,
  classifyAgentRuntimePreTurnFailureCode,
  agentRuntimePreTurnFailureProjection,
};
