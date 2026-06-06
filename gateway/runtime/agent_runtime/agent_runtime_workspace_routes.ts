#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::workspace-routes.
//
// Gateway owns Agent Runtime workspace route admission because workspace
// selection defines the external engine home base, git-root derivation, and
// permission boundary. Adapter hosts may serve HTTP, but they must delegate
// route policy and projection shape here.

'use strict';

function isAgentRuntimeWorkspaceRoute(pathname) {
  return pathname === '/api/shell-socket/agent-runtime/workspace' ||
    pathname === '/api/agent-runtime/workspace' ||
    pathname === '/api/shell-socket/agent-runtime/workspace/pick' ||
    pathname === '/api/agent-runtime/workspace/pick';
}

function createAgentRuntimeWorkspaceRouteHandler(options = {}) {
  const store = options.workspaceStore;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  if (!store || typeof store.agentRuntimeWorkspaceProjection !== 'function') {
    throw new Error('agent_runtime_workspace_route_store_missing');
  }
  if (typeof readJsonBody !== 'function') {
    throw new Error('agent_runtime_workspace_route_read_json_body_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('agent_runtime_workspace_route_send_json_missing');
  }

  async function handleAgentRuntimeWorkspaceRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const traceId = String(args.traceId || '');
    if (!req || !res || !isAgentRuntimeWorkspaceRoute(pathname)) return false;

    if (req.method === 'GET' && (pathname === '/api/shell-socket/agent-runtime/workspace' || pathname === '/api/agent-runtime/workspace')) {
      const payload = store.agentRuntimeWorkspaceProjection(traceId, {});
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }
    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/workspace' || pathname === '/api/agent-runtime/workspace')) {
      const body = await readJsonBody(req, 8192).catch(() => ({}));
      const payload = store.agentRuntimeWorkspaceProjection(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }
    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/workspace/pick' || pathname === '/api/agent-runtime/workspace/pick')) {
      const body = await readJsonBody(req, 8192).catch(() => ({}));
      const payload = store.agentRuntimeWorkspacePickerProjection(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 409 : 200), payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'agent_runtime_workspace_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: String(req.method || ''),
      pathname,
    });
    return true;
  }

  return {
    isAgentRuntimeWorkspaceRoute,
    handleAgentRuntimeWorkspaceRoute,
  };
}

module.exports = {
  isAgentRuntimeWorkspaceRoute,
  createAgentRuntimeWorkspaceRouteHandler,
};
