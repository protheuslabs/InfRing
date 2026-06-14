#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::approvals.
//
// Gateway owns external-runtime approval decisions and approved effect receipts.
// Engine adapters may propose intentions; durable effects only happen after this
// boundary receives an approval decision from an authorized gatekeeper.

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanDisplayText(value, maxLen = 24000) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').trim().slice(0, maxLen); }
function cleanArtifactContent(value, maxLen = 262144) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/[ \t]+\n/g, '\n').slice(0, maxLen); }
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanApprovalId(value) { return cleanText(value, 260).replace(/[^a-zA-Z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanReceiptComponent(value, maxLen = 200) { return cleanText(value, maxLen).replace(/[^A-Za-z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, '') || 'unknown'; }
function cleanWorkingDirectory(value) {
  const raw = String(value == null ? '' : value).replace(/\0/g, '').trim();
  if (!raw || raw.startsWith('~')) return '';
  return path.resolve(raw);
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function stableDigest(value) {
  return createHash('sha256').update(JSON.stringify(value || {})).digest('hex');
}

function approvalResumeToken(source) {
  const requested = cleanApprovalId(source && source.resume_token);
  if (requested) return requested;
  const approvalId = cleanApprovalId(source && source.approval_id);
  const traceId = cleanText(source && source.trace_id, 200);
  const turnId = cleanText(source && source.turn_id, 200);
  const toolId = cleanText(source && source.tool_id, 120);
  const digest = stableDigest({ approvalId, traceId, turnId, toolId }).slice(0, 32);
  return cleanApprovalId(`resume_${digest}`);
}

function pendingThreadKey(row) {
  const source = row && typeof row === 'object' ? row : {};
  const sessionId = cleanText(source.session_id, 200);
  if (sessionId) return `session:${sessionId}`;
  const agentId = cleanText(source.agent_id, 160);
  if (agentId) return `agent:${agentId}`;
  const traceId = cleanText(source.trace_id, 200);
  if (traceId) return `trace:${traceId}`;
  return '';
}

function pendingProposalKey(row) {
  const source = row && typeof row === 'object' ? row : {};
  return stableDigest({
    thread: pendingThreadKey(source),
    tool_id: cleanText(source.tool_id, 120),
    working_directory: cleanWorkingDirectory(source.working_directory),
    proposal_arguments: source.proposal_arguments && typeof source.proposal_arguments === 'object'
      ? source.proposal_arguments
      : {},
  });
}

function createAgentRuntimeApprovalStore(options = {}) {
  const root = path.resolve(options.root || process.cwd());
  const appendApprovedEffectContext = typeof options.appendApprovedEffectContext === 'function'
    ? options.appendApprovedEffectContext
    : null;
  const decisionsPath = path.resolve(
    options.decisionsPath ||
      path.join(root, 'local', 'state', 'observability', 'agent_runtime', 'agent_runtime_approval_decisions.jsonl'),
  );
  const approvalDecisions = new Map();
  const pendingApprovals = new Map();

  function loadPersistedAlwaysAllowedDecisions() {
    let raw = '';
    try { raw = fs.readFileSync(decisionsPath, 'utf8'); } catch { return; }
    for (const line of raw.split(/\r?\n/)) {
      if (!line.trim()) continue;
      let row = null;
      try { row = JSON.parse(line); } catch { continue; }
      if (!row || row.decision !== 'always_allow_tool_call') continue;
      const approvalId = cleanApprovalId(row.approval_id);
      const toolId = cleanText(row.tool_id, 120);
      if (!approvalId || !toolId) continue;
      approvalDecisions.set(approvalId, row);
    }
  }

  function persistApprovalDecision(row) {
    const source = row && typeof row === 'object' ? row : null;
    if (!source) return;
    try {
      ensureDir(path.dirname(decisionsPath));
      fs.appendFileSync(decisionsPath, `${JSON.stringify(source)}\n`, 'utf8');
    } catch {}
  }

  function sanitizeAgentRuntimeProposalArguments(value) {
    const source = value && typeof value === 'object' ? value : {};
    const out = {};
    const rawPath = cleanText(source.path || source.file || source.filename || source.relative_path, 500);
    if (rawPath) out.path = rawPath;
    const mimeType = cleanText(source.mime_type || source.content_type || 'text/plain', 120);
    if (mimeType) out.mime_type = mimeType;
    if (source.content != null) out.content = cleanArtifactContent(source.content, 262144);
    else if (source.text != null) out.content = cleanArtifactContent(source.text, 262144);
    else if (source.body != null) out.content = cleanArtifactContent(source.body, 262144);
    return out;
  }

function resolveAgentRuntimeArtifactPath(rawPath, workingDirectory) {
    let baseDir = cleanWorkingDirectory(workingDirectory) || root;
    if (baseDir === path.resolve('/workspace') && !fs.existsSync(baseDir)) {
      baseDir = root;
    }
    const value = String(rawPath == null ? '' : rawPath).replace(/\\/g, '/').trim();
    if (!value) throw new Error('artifact_path_required');
    if (path.isAbsolute(value) || value.startsWith('~') || value.includes('\0')) throw new Error('artifact_path_must_be_workspace_relative');
    const normalized = path.posix.normalize(value).replace(/^\/+/, '');
    if (!normalized || normalized === '.' || normalized.startsWith('../') || normalized.includes('/../')) throw new Error('artifact_path_escapes_workspace');
    const target = path.resolve(baseDir, normalized);
    const baseWithSep = baseDir.endsWith(path.sep) ? baseDir : baseDir + path.sep;
    if (target !== baseDir && !target.startsWith(baseWithSep)) throw new Error('artifact_path_escapes_workspace');
    if (target === baseDir) throw new Error('artifact_path_must_name_file');
    return {
      target,
      relativePath: path.relative(baseDir, target).replace(/\\/g, '/'),
      workingDirectory: baseDir,
    };
  }

  function executeAgentRuntimeApprovedProposal(traceId, approvalId, body) {
    const toolId = cleanText(body && body.tool_id, 120);
    if (toolId !== 'artifact.create_propose') return null;
    const args = sanitizeAgentRuntimeProposalArguments(body && (body.proposal_arguments || body.arguments));
    const resolved = resolveAgentRuntimeArtifactPath(args.path, body && body.working_directory);
    const content = cleanArtifactContent(args.content || '', 262144);
    ensureDir(path.dirname(resolved.target));
    fs.writeFileSync(resolved.target, content, 'utf8');
    const digest = createHash('sha256').update(content).digest('hex');
    const bytes = Buffer.byteLength(content, 'utf8');
    const result = {
      ok: true,
      type: 'agent_runtime_approval_effect_receipt',
      approval_id: cleanApprovalId(approvalId),
      trace_id: cleanText(traceId, 200),
      tool_id: toolId,
      agent_id: cleanText(body && body.agent_id, 160),
      engine_id: cleanEngineId(body && body.engine_id),
      session_id: cleanText(body && body.session_id, 200),
      turn_id: cleanText(body && body.turn_id, 200),
      effect: 'artifact_written',
      path: resolved.relativePath,
      working_directory: resolved.workingDirectory,
      bytes,
      sha256: digest,
      mime_type: cleanText(args.mime_type || 'text/plain', 120),
      content_preview: cleanDisplayText(content, 1200),
      result_ref: `artifact/${resolved.relativePath}`,
      receipt_ref: `receipt/agent-runtime-approval/${cleanApprovalId(approvalId)}`,
      display_text: `Created ${resolved.relativePath} (${bytes} bytes).`,
    };
    if (appendApprovedEffectContext) {
      try {
        result.context_append = appendApprovedEffectContext({
          approvalId,
          traceId,
          toolId,
          agentId: result.agent_id,
          engineId: result.engine_id,
          sessionId: result.session_id,
          turnId: result.turn_id,
          path: result.path,
          artifactRef: result.result_ref,
          receiptRef: result.receipt_ref,
          bytes: result.bytes,
          sha256: result.sha256,
          contentPreview: result.content_preview,
          displayText: result.display_text,
        });
      } catch (error) {
        result.context_append = {
          ok: false,
          error: cleanText(error && error.message ? error.message : error, 240),
        };
      }
    }
    return result;
  }

  function normalizePendingApprovalRequest(request) {
    const source = request && typeof request === 'object' ? request : {};
    const approvalId = cleanApprovalId(source.approval_id);
    const toolId = cleanText(source.tool_id, 120);
    const traceId = cleanText(source.trace_id, 200);
    const proposalArguments = sanitizeAgentRuntimeProposalArguments(source.proposal_arguments || source.arguments);
    const explicitResumeStrategy = cleanText(source.resume_strategy, 120);
    const row = {
      type: 'agent_runtime_pending_approval',
      schema_version: 1,
      projection_kind: 'permission_request',
      projection_schema_version: 1,
      approval_id: approvalId,
      trace_id: traceId,
      request_id: cleanText(source.request_id, 200),
      engine_id: cleanEngineId(source.engine_id),
      agent_id: cleanText(source.agent_id, 160),
      session_id: cleanText(source.session_id, 200),
      turn_id: cleanText(source.turn_id, 200),
      working_directory: cleanWorkingDirectory(
        source.working_directory ||
          source.current_working_directory ||
          source.present_working_directory ||
          source.cwd,
      ),
      tool_call_ref: cleanText(source.tool_call_ref, 240),
      tool_id: toolId,
      capability: cleanText(source.capability, 160),
      reason: cleanText(source.reason, 1000),
      argument_keys: Array.isArray(source.argument_keys)
        ? source.argument_keys.map((key) => cleanText(key, 80)).filter(Boolean).slice(0, 24)
        : Object.keys(proposalArguments || {}).map((key) => cleanText(key, 80)).filter(Boolean).slice(0, 24),
      proposal_arguments: proposalArguments,
      gatekeeper_kind: cleanText(source.gatekeeper_kind || 'user', 80) || 'user',
      future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor', 'admin_agent'],
      decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
      decision_scope: 'tool_call',
      status: 'paused_pending_approval',
      turn_status: 'permission_required',
      pause_reason: cleanText(source.pause_reason || source.reason || 'agent_runtime_tool_call_requires_approval', 1000),
      resume_token: approvalResumeToken(source),
      resume_strategy: explicitResumeStrategy || (toolId === 'permission.request'
        ? 'grant_then_retry_next_turn'
        : Object.keys(proposalArguments || {}).length
        ? 'gateway_apply_approved_effect'
        : 'grant_then_retry_next_turn'),
      created_at: nowIso(),
      source: cleanText(source.source || 'gateway.runtime.agent_runtime_approvals', 160),
      source_authority: 'gateway.runtime.agent_runtime_approvals',
    };
    row.thread_key = pendingThreadKey(row);
    row.proposal_key = pendingProposalKey(row);
    return row;
  }

  function recordAgentRuntimePendingApproval(request) {
    const row = normalizePendingApprovalRequest(request);
    if (!row.approval_id) return null;
    const threadKey = pendingThreadKey(row);
    if (threadKey) {
      for (const existing of pendingApprovals.values()) {
        if (!existing || pendingThreadKey(existing) !== threadKey) continue;
        const projected = projectPendingApprovalRow(existing);
        return {
          ...projected,
          coalesced_with_existing_pending_approval: true,
          coalesced_reason: pendingProposalKey(existing) === pendingProposalKey(row)
            ? 'duplicate_permission_request'
            : 'thread_already_has_pending_permission_request',
          requested_approval_id: row.approval_id,
          active_approval_id: cleanApprovalId(existing.approval_id),
          queue_policy: 'single_pending_permission_per_thread',
        };
      }
    }
    pendingApprovals.set(row.approval_id, row);
    if (pendingApprovals.size > 200) {
      const firstKey = pendingApprovals.keys().next().value;
      if (firstKey) pendingApprovals.delete(firstKey);
    }
    const argumentPreview = cleanDisplayText(
      row.proposal_arguments && (row.proposal_arguments.path || row.proposal_arguments.content || row.proposal_arguments.text || ''),
      1200,
    );
    return {
      ...row,
      proposal_arguments: undefined,
      proposal_arguments_ref: `approval-pending/${row.approval_id}/proposal-arguments`,
      proposal_arguments_preview: argumentPreview,
      resume_token: row.resume_token,
      turn_status: row.turn_status,
      pause_reason: row.pause_reason,
      approval_route: `/api/shell-socket/approvals/${encodeURIComponent(row.approval_id)}/decision`,
      queue_policy: 'single_pending_permission_per_thread',
    };
  }

  function projectPendingApprovalRow(row) {
    const source = row && typeof row === 'object' ? row : {};
    const argumentPreview = cleanDisplayText(
      source.proposal_arguments && (source.proposal_arguments.path || source.proposal_arguments.content || source.proposal_arguments.text || ''),
      1200,
    );
    return {
      type: 'agent_runtime_pending_approval',
      schema_version: 1,
      projection_kind: 'permission_request',
      projection_schema_version: 1,
      approval_id: cleanApprovalId(source.approval_id),
      trace_id: cleanText(source.trace_id, 200),
      request_id: cleanText(source.request_id, 200),
      engine_id: cleanEngineId(source.engine_id),
      agent_id: cleanText(source.agent_id, 160),
      session_id: cleanText(source.session_id, 200),
      turn_id: cleanText(source.turn_id, 200),
      working_directory: cleanWorkingDirectory(source.working_directory),
      tool_call_ref: cleanText(source.tool_call_ref, 240),
      tool_id: cleanText(source.tool_id, 120),
      capability: cleanText(source.capability, 160),
      reason: cleanText(source.reason, 1000),
      argument_keys: Array.isArray(source.argument_keys) ? source.argument_keys.slice(0, 24) : [],
      gatekeeper_kind: cleanText(source.gatekeeper_kind || 'user', 80) || 'user',
      future_gatekeeper_kinds: ['user', 'system_policy', 'agent_supervisor', 'admin_agent'],
      decisions: ['allow_once', 'deny', 'always_allow_tool_call'],
      decision_scope: 'tool_call',
      status: 'paused_pending_approval',
      turn_status: 'permission_required',
      pause_reason: cleanText(source.pause_reason || source.reason || 'agent_runtime_tool_call_requires_approval', 1000),
      resume_token: approvalResumeToken(source),
      resume_strategy: cleanText(source.resume_strategy || 'grant_then_retry_next_turn', 120),
      created_at: cleanText(source.created_at, 80),
      source: cleanText(source.source || 'gateway.runtime.agent_runtime_approvals', 160),
      source_authority: 'gateway.runtime.agent_runtime_approvals',
      thread_key: cleanText(source.thread_key || pendingThreadKey(source), 260),
      queue_policy: 'single_pending_permission_per_thread',
      proposal_arguments_ref: `approval-pending/${cleanApprovalId(source.approval_id)}/proposal-arguments`,
      proposal_arguments_preview: argumentPreview,
      approval_route: `/api/shell-socket/approvals/${encodeURIComponent(cleanApprovalId(source.approval_id))}/decision`,
    };
  }

  function agentRuntimePendingApprovalsProjection(traceId) {
    const rows = Array.from(pendingApprovals.values())
      .map(projectPendingApprovalRow)
      .filter((row) => row.approval_id)
      .sort((a, b) => String(b.created_at || '').localeCompare(String(a.created_at || '')))
      .slice(0, 20);
    return {
      ok: true,
      type: 'agent_runtime_pending_approvals_projection',
      schema_version: 1,
      projection_kind: 'permission_request_list',
      projection_schema_version: 1,
      trace_id: cleanText(traceId, 200),
      pending_count: rows.length,
      pending_requests: rows,
      secrets_included: false,
      source_authority: 'gateway.runtime.agent_runtime_approvals',
    };
  }

  function agentRuntimeApprovalDecisionProjection(traceId, approvalId, body) {
    const id = cleanApprovalId(approvalId);
    const decision = cleanText(body && body.decision, 80);
    const allowed = new Set(['allow_once', 'deny', 'always_allow_tool_call']);
    if (!id) return { ok: false, status_code: 400, type: 'approval_decision_ack', trace_id: traceId, error: 'approval_id_required' };
    if (!allowed.has(decision)) return { ok: false, status_code: 400, type: 'approval_decision_ack', trace_id: traceId, approval_id: id, error: 'approval_decision_invalid' };
    const pending = pendingApprovals.get(id) || null;
    const decisionBody = {
      ...(pending || {}),
      ...(body && typeof body === 'object' ? body : {}),
      tool_id: cleanText((body && body.tool_id) || (pending && pending.tool_id), 120),
      proposal_arguments:
        body && (body.proposal_arguments || body.arguments)
          ? (body.proposal_arguments || body.arguments)
          : pending && pending.proposal_arguments,
      agent_id: cleanText((body && body.agent_id) || (pending && pending.agent_id), 160),
      engine_id: cleanEngineId((body && body.engine_id) || (pending && pending.engine_id)),
      session_id: cleanText((body && body.session_id) || (pending && pending.session_id), 200),
      turn_id: cleanText((body && body.turn_id) || (pending && pending.turn_id), 200),
      tool_call_ref: cleanText((body && body.tool_call_ref) || (pending && pending.tool_call_ref), 240),
      working_directory: cleanWorkingDirectory(
        (body && (
          body.working_directory ||
          body.current_working_directory ||
          body.present_working_directory ||
          body.cwd
        )) ||
          (pending && pending.working_directory),
      ),
    };
    let executionResult = null;
    if (decision !== 'deny') {
      try {
        executionResult = executeAgentRuntimeApprovedProposal(traceId, id, decisionBody);
      } catch (error) {
        executionResult = {
          ok: false,
          type: 'agent_runtime_approval_effect_error',
          approval_id: id,
          trace_id: traceId,
          tool_id: cleanText(decisionBody && decisionBody.tool_id, 120),
          error: cleanText(error && error.message ? error.message : error, 240),
        };
      }
    }
    const resumeToken = approvalResumeToken(pending || decisionBody);
    const resumeStrategy = pending && pending.resume_strategy
      ? pending.resume_strategy
      : executionResult && executionResult.ok
        ? 'gateway_apply_approved_effect'
        : 'grant_then_retry_next_turn';
    const resumeAction = decision === 'deny'
      ? 'fail_paused_turn_with_denial'
      : executionResult && executionResult.ok
        ? 'resume_paused_turn_with_gateway_applied_effect_receipt'
        : 'resume_next_turn_with_permission_grant';
    const decisionReceiptBase = {
      type: 'agent_runtime_approval_decision_receipt',
      schema_version: 1,
      projection_kind: 'receipt',
      projection_schema_version: 1,
      receipt_ref: `receipt/agent-runtime-approval-decision/${cleanReceiptComponent(id, 240)}`,
      trace_id: cleanText(traceId, 200),
      approval_id: id,
      resume_token: resumeToken,
      paused_turn_id: cleanText(pending && pending.turn_id, 200),
      paused_turn_status: 'permission_required',
      decision,
      gatekeeper_kind: cleanText((body && body.gatekeeper_kind) || (pending && pending.gatekeeper_kind) || 'user', 80) || 'user',
      resume_strategy: resumeStrategy,
      resume_action: resumeAction,
      durable_effect_executed: !!(executionResult && executionResult.ok),
      decided_at: nowIso(),
      source_authority: 'gateway.runtime.agent_runtime_approvals',
    };
    const decisionReceipt = {
      ...decisionReceiptBase,
      receipt_hash: stableDigest(decisionReceiptBase),
    };
    const effectDisplayText = cleanDisplayText(executionResult && executionResult.display_text, 1000);
    const effectError = cleanText(executionResult && executionResult.error, 240);
    const decisionDisplayText = decision === 'deny'
      ? `Permission denied for ${cleanText(decisionBody && decisionBody.tool_id, 120) || 'tool call'}.`
      : executionResult && executionResult.ok
        ? effectDisplayText || `Permission approved and ${cleanText(decisionBody && decisionBody.tool_id, 120) || 'tool call'} executed.`
        : executionResult && executionResult.ok === false
          ? `Permission approved, but the approved effect failed${effectError ? `: ${effectError}` : '.'}`
          : `Permission approved for ${cleanText(decisionBody && decisionBody.tool_id, 120) || 'tool call'}; the agent can retry with the grant.`;
    const row = {
      type: 'approval_decision_ack',
      projection_kind: 'permission_decision',
      projection_schema_version: 1,
      ok: true,
      status: decision === 'deny' ? 'denied' : executionResult && executionResult.ok === false ? 'failed_with_reason' : 'completed',
      trace_id: traceId,
      approval_id: id,
      resume_token: resumeToken,
      decision,
      display_text: decisionDisplayText,
      output_text: decisionDisplayText,
      text: decisionDisplayText,
      tool_id: cleanText(decisionBody && decisionBody.tool_id, 120),
      tool_call_ref: cleanText(decisionBody && decisionBody.tool_call_ref, 240),
      engine_id: cleanEngineId(decisionBody && decisionBody.engine_id),
      agent_id: cleanText(decisionBody && decisionBody.agent_id, 160),
      session_id: cleanText(decisionBody && decisionBody.session_id, 200),
      working_directory: cleanWorkingDirectory(decisionBody && decisionBody.working_directory),
      gatekeeper_kind: cleanText((body && body.gatekeeper_kind) || (pending && pending.gatekeeper_kind) || 'user', 80) || 'user',
      decided_at: nowIso(),
      pending_request_found: !!pending,
      paused_turn_id: cleanText(pending && pending.turn_id, 200),
      paused_turn_status: 'permission_required',
      resume_strategy: resumeStrategy,
      resume_action: resumeAction,
      resumed: decision !== 'deny',
      durable_effect_executed: !!(executionResult && executionResult.ok),
      approved_effect_executed: !!(executionResult && executionResult.ok),
      approved_effect_path: cleanText(executionResult && executionResult.path, 600),
      approved_effect_artifact_ref: cleanText(executionResult && executionResult.result_ref, 600),
      approved_effect_result_ref: cleanText(executionResult && executionResult.result_ref, 600),
      approved_effect_receipt_ref: cleanText(executionResult && executionResult.receipt_ref, 600),
      approved_effect_display_text: effectDisplayText,
      approved_effect_bytes: executionResult && executionResult.ok ? Number(executionResult.bytes) || 0 : 0,
      approved_effect_sha256: cleanText(executionResult && executionResult.sha256, 80),
      execution_result: executionResult,
      decision_receipt_ref: decisionReceipt.receipt_ref,
      decision_receipt: decisionReceipt,
      next_action: decision === 'deny'
        ? 'tool_call_denied'
        : executionResult && executionResult.ok
          ? 'tool_call_executed'
          : 'tool_call_permission_recorded_for_next_agent_runtime_turn',
    };
    pendingApprovals.delete(id);
    approvalDecisions.set(id, row);
    persistApprovalDecision(row);
    if (approvalDecisions.size > 200) {
      const firstKey = approvalDecisions.keys().next().value;
      if (firstKey) approvalDecisions.delete(firstKey);
    }
    return row;
  }

  function mergeAgentRuntimeApprovalPermissionPolicy(source, sessionId, engineId) {
    const base = source && typeof source === 'object' ? source : {};
    const alwaysAllowed = new Set(
      Array.isArray(base.always_allowed_tool_calls)
        ? base.always_allowed_tool_calls.map((toolId) => cleanText(toolId, 120)).filter(Boolean)
        : [],
    );
    const session = cleanText(sessionId, 200);
    const engine = cleanEngineId(engineId);
    for (const [approvalId, row] of Array.from(approvalDecisions.entries())) {
      if (!row || typeof row !== 'object') continue;
      const decision = cleanText(row.decision, 80);
      const resumeStrategy = cleanText(row.resume_strategy, 120);
      const effectAlreadyExecuted = row.durable_effect_executed === true || row.approved_effect_executed === true;
      const allowOnceGrantsNextTurn = decision === 'allow_once' &&
        resumeStrategy === 'grant_then_retry_next_turn' &&
        !effectAlreadyExecuted;
      if (decision !== 'always_allow_tool_call' && !allowOnceGrantsNextTurn) continue;
      const toolId = cleanText(row.tool_id, 120);
      if (!toolId) continue;
      const rowSession = cleanText(row.session_id, 200);
      const rowEngine = cleanEngineId(row.engine_id);
      if (rowSession && session && rowSession !== session) continue;
      if (rowEngine && engine && rowEngine !== engine) continue;
      alwaysAllowed.add(toolId);
      if (decision === 'allow_once') approvalDecisions.delete(approvalId);
    }
    return {
      ...base,
      always_allowed_tool_calls: Array.from(alwaysAllowed).slice(0, 64),
    };
  }

  loadPersistedAlwaysAllowedDecisions();

  return {
    sanitizeAgentRuntimeProposalArguments,
    executeAgentRuntimeApprovedProposal,
    recordAgentRuntimePendingApproval,
    agentRuntimePendingApprovalsProjection,
    agentRuntimeApprovalDecisionProjection,
    mergeAgentRuntimeApprovalPermissionPolicy,
  };
}

module.exports = {
  createAgentRuntimeApprovalStore,
};
