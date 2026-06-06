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
function cleanEngineId(value) { return cleanText(value, 120).toLowerCase().replace(/[^a-z0-9_.-]+/g, '_').replace(/^_+|_+$/g, ''); }
function cleanApprovalId(value) { return cleanText(value, 260).replace(/[^a-zA-Z0-9_.:-]+/g, '_').replace(/^_+|_+$/g, ''); }

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function createAgentRuntimeApprovalStore(options = {}) {
  const root = path.resolve(options.root || process.cwd());
  const approvalDecisions = new Map();
  const pendingApprovals = new Map();

  function sanitizeAgentRuntimeProposalArguments(value) {
    const source = value && typeof value === 'object' ? value : {};
    const out = {};
    const rawPath = cleanText(source.path || source.file || source.filename || source.relative_path, 500);
    if (rawPath) out.path = rawPath;
    const mimeType = cleanText(source.mime_type || source.content_type || 'text/plain', 120);
    if (mimeType) out.mime_type = mimeType;
    if (source.content != null) out.content = cleanDisplayText(source.content, 262144);
    else if (source.text != null) out.content = cleanDisplayText(source.text, 262144);
    else if (source.body != null) out.content = cleanDisplayText(source.body, 262144);
    return out;
  }

  function resolveAgentRuntimeArtifactPath(rawPath) {
    const value = String(rawPath == null ? '' : rawPath).replace(/\\/g, '/').trim();
    if (!value) throw new Error('artifact_path_required');
    if (path.isAbsolute(value) || value.startsWith('~') || value.includes('\0')) throw new Error('artifact_path_must_be_repo_relative');
    const normalized = path.posix.normalize(value).replace(/^\/+/, '');
    if (!normalized || normalized === '.' || normalized.startsWith('../') || normalized.includes('/../')) throw new Error('artifact_path_escapes_repo');
    const target = path.resolve(root, normalized);
    const rootWithSep = root.endsWith(path.sep) ? root : root + path.sep;
    if (target !== root && !target.startsWith(rootWithSep)) throw new Error('artifact_path_escapes_repo');
    if (target === root) throw new Error('artifact_path_must_name_file');
    return { target, relativePath: path.relative(root, target).replace(/\\/g, '/') };
  }

  function executeAgentRuntimeApprovedProposal(traceId, approvalId, body) {
    const toolId = cleanText(body && body.tool_id, 120);
    if (toolId !== 'artifact.create_propose') return null;
    const args = sanitizeAgentRuntimeProposalArguments(body && (body.proposal_arguments || body.arguments));
    const resolved = resolveAgentRuntimeArtifactPath(args.path);
    const content = cleanDisplayText(args.content || '', 262144);
    ensureDir(path.dirname(resolved.target));
    fs.writeFileSync(resolved.target, content, 'utf8');
    const digest = createHash('sha256').update(content).digest('hex');
    const bytes = Buffer.byteLength(content, 'utf8');
    return {
      ok: true,
      type: 'agent_runtime_approval_effect_receipt',
      approval_id: cleanApprovalId(approvalId),
      trace_id: cleanText(traceId, 200),
      tool_id: toolId,
      effect: 'artifact_written',
      path: resolved.relativePath,
      bytes,
      sha256: digest,
      mime_type: cleanText(args.mime_type || 'text/plain', 120),
      result_ref: `artifact/${resolved.relativePath}`,
      receipt_ref: `receipt/agent-runtime-approval/${cleanApprovalId(approvalId)}`,
      display_text: `Created ${resolved.relativePath} (${bytes} bytes).`,
    };
  }

  function normalizePendingApprovalRequest(request) {
    const source = request && typeof request === 'object' ? request : {};
    const approvalId = cleanApprovalId(source.approval_id);
    const toolId = cleanText(source.tool_id, 120);
    const traceId = cleanText(source.trace_id, 200);
    const proposalArguments = sanitizeAgentRuntimeProposalArguments(source.proposal_arguments || source.arguments);
    return {
      type: 'agent_runtime_pending_approval',
      schema_version: 1,
      approval_id: approvalId,
      trace_id: traceId,
      request_id: cleanText(source.request_id, 200),
      engine_id: cleanEngineId(source.engine_id),
      session_id: cleanText(source.session_id, 200),
      turn_id: cleanText(source.turn_id, 200),
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
      resume_strategy: Object.keys(proposalArguments || {}).length
        ? 'gateway_apply_approved_effect'
        : 'grant_then_retry_next_turn',
      created_at: nowIso(),
      source_authority: 'gateway.runtime.agent_runtime_approvals',
    };
  }

  function recordAgentRuntimePendingApproval(request) {
    const row = normalizePendingApprovalRequest(request);
    if (!row.approval_id) return null;
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
      approval_route: `/api/shell-socket/approvals/${encodeURIComponent(row.approval_id)}/decision`,
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
      engine_id: cleanEngineId((body && body.engine_id) || (pending && pending.engine_id)),
      session_id: cleanText((body && body.session_id) || (pending && pending.session_id), 200),
      tool_call_ref: cleanText((body && body.tool_call_ref) || (pending && pending.tool_call_ref), 240),
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
    const row = {
      type: 'approval_decision_ack',
      ok: true,
      trace_id: traceId,
      approval_id: id,
      decision,
      tool_id: cleanText(decisionBody && decisionBody.tool_id, 120),
      tool_call_ref: cleanText(decisionBody && decisionBody.tool_call_ref, 240),
      engine_id: cleanEngineId(decisionBody && decisionBody.engine_id),
      session_id: cleanText(decisionBody && decisionBody.session_id, 200),
      gatekeeper_kind: cleanText((body && body.gatekeeper_kind) || (pending && pending.gatekeeper_kind) || 'user', 80) || 'user',
      decided_at: nowIso(),
      pending_request_found: !!pending,
      paused_turn_id: cleanText(pending && pending.turn_id, 200),
      resume_strategy: pending && pending.resume_strategy
        ? pending.resume_strategy
        : executionResult && executionResult.ok
          ? 'gateway_apply_approved_effect'
          : 'grant_then_retry_next_turn',
      resumed: decision !== 'deny' && !!(executionResult && executionResult.ok),
      durable_effect_executed: !!(executionResult && executionResult.ok),
      execution_result: executionResult,
      next_action: decision === 'deny'
        ? 'tool_call_denied'
        : executionResult && executionResult.ok
          ? 'tool_call_executed'
          : 'tool_call_permission_recorded_for_next_agent_runtime_turn',
    };
    pendingApprovals.delete(id);
    approvalDecisions.set(id, row);
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
      if (decision !== 'allow_once' && decision !== 'always_allow_tool_call') continue;
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

  return {
    sanitizeAgentRuntimeProposalArguments,
    recordAgentRuntimePendingApproval,
    agentRuntimeApprovalDecisionProjection,
    mergeAgentRuntimeApprovalPermissionPolicy,
  };
}

module.exports = {
  createAgentRuntimeApprovalStore,
};
