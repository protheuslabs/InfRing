#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::approval-routes.
//
// Gateway owns external-runtime approval decision admission and response shape.
// Adapter hosts may expose HTTP, but approval policy and effect receipts are
// Gateway behavior and must not live in adapter route blocks.

'use strict';

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function isAgentRuntimeApprovalRoute(pathname) {
  return String(pathname || '') === '/api/shell-socket/approvals/pending' ||
    String(pathname || '') === '/api/agent-runtime/approvals/pending' ||
    /^\/api\/shell-socket\/approvals\/[^/]+\/decision$/.test(String(pathname || ''));
}

function createAgentRuntimeApprovalRouteHandler(options = {}) {
  const approvalStore = options.approvalStore;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  if (!approvalStore || typeof approvalStore.agentRuntimeApprovalDecisionProjection !== 'function') {
    throw new Error('agent_runtime_approval_route_store_missing');
  }
  if (typeof approvalStore.agentRuntimePendingApprovalsProjection !== 'function') {
    throw new Error('agent_runtime_approval_route_pending_projection_missing');
  }
  if (typeof readJsonBody !== 'function') {
    throw new Error('agent_runtime_approval_route_read_json_body_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('agent_runtime_approval_route_send_json_missing');
  }

  async function handleAgentRuntimeApprovalRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const traceId = String(args.traceId || '');
    if (!req || !res || !isAgentRuntimeApprovalRoute(pathname)) return false;

    if (req.method === 'GET' && (pathname === '/api/shell-socket/approvals/pending' || pathname === '/api/agent-runtime/approvals/pending')) {
      const payload = approvalStore.agentRuntimePendingApprovalsProjection(traceId);
      sendJson(res, payload.status_code || 200, payload);
      return true;
    }

    if (req.method === 'POST') {
      const match = pathname.match(/^\/api\/shell-socket\/approvals\/([^/]+)\/decision$/);
      const body = await readJsonBody(req, 327680).catch(() => ({}));
      const approvalId = decodeURIComponent((match && match[1]) || '');
      const payload = approvalStore.agentRuntimeApprovalDecisionProjection(traceId, approvalId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'agent_runtime_approval_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: cleanText(req.method || '', 32),
      pathname,
    });
    return true;
  }

  return {
    isAgentRuntimeApprovalRoute,
    handleAgentRuntimeApprovalRoute,
  };
}

module.exports = {
  isAgentRuntimeApprovalRoute,
  createAgentRuntimeApprovalRouteHandler,
};
