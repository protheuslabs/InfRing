#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::engine-routes.
//
// Gateway owns Agent Runtime engine route admission, menu/install projections,
// and selection status/error shape. Adapter hosts may serve HTTP, but they must
// delegate runtime engine boundary behavior here.

'use strict';

function cleanEngineId(value) {
  return String(value == null ? '' : value)
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_.-]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 120);
}

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function isAgentRuntimeEngineRoute(pathname) {
  return pathname === '/api/shell-socket/agent-runtime/engines' ||
    pathname === '/api/agent-runtime/engines' ||
    pathname === '/api/shell-socket/agent-runtime/selection' ||
    pathname === '/api/agent-runtime/selection' ||
    /^\/api\/(?:shell-socket\/)?agent-runtime\/engines\/[^/]+\/install$/.test(String(pathname || ''));
}

function createAgentRuntimeEngineRouteHandler(options = {}) {
  const engineProjectionStore = options.engineProjectionStore;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  const selectEngine = options.selectEngine;
  if (!engineProjectionStore || typeof engineProjectionStore.agentRuntimeEnginesProjection !== 'function') {
    throw new Error('agent_runtime_engine_route_projection_store_missing');
  }
  if (typeof engineProjectionStore.agentRuntimeEngineInstallProjection !== 'function') {
    throw new Error('agent_runtime_engine_route_install_projection_missing');
  }
  if (typeof selectEngine !== 'function') {
    throw new Error('agent_runtime_engine_route_select_engine_missing');
  }
  if (typeof readJsonBody !== 'function') {
    throw new Error('agent_runtime_engine_route_read_json_body_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('agent_runtime_engine_route_send_json_missing');
  }

  async function handleAgentRuntimeEngineRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const traceId = String(args.traceId || '');
    if (!req || !res || !isAgentRuntimeEngineRoute(pathname)) return false;

    const installMatch = pathname.match(/^\/api\/(?:shell-socket\/)?agent-runtime\/engines\/([^/]+)\/install$/);
    if (req.method === 'POST' && installMatch) {
      await readJsonBody(req, 8192).catch(() => ({}));
      const engineId = decodeURIComponent(installMatch[1] || '');
      const payload = await engineProjectionStore.agentRuntimeEngineInstallProjection(traceId, engineId).catch((error) => ({
        ok: false,
        status_code: 502,
        type: 'agent_runtime_engine_install_projection_error',
        trace_id: traceId,
        engine_id: cleanEngineId(engineId),
        error: cleanText(error && error.message ? error.message : error, 240),
      }));
      sendJson(res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      return true;
    }

    if (req.method === 'GET' && (pathname === '/api/shell-socket/agent-runtime/engines' || pathname === '/api/agent-runtime/engines')) {
      const payload = await engineProjectionStore.agentRuntimeEnginesProjection(traceId).catch((error) => ({
        ok: false,
        type: 'agent_runtime_engines_projection_error',
        trace_id: traceId,
        error: cleanText(error && error.message ? error.message : error, 240),
        engines: [],
      }));
      sendJson(res, payload.ok === false ? 503 : 200, payload);
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/selection' || pathname === '/api/agent-runtime/selection')) {
      const body = await readJsonBody(req, 8192).catch(() => ({}));
      const payload = selectEngine(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'agent_runtime_engine_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: String(req.method || ''),
      pathname,
    });
    return true;
  }

  return {
    isAgentRuntimeEngineRoute,
    handleAgentRuntimeEngineRoute,
  };
}

module.exports = {
  isAgentRuntimeEngineRoute,
  createAgentRuntimeEngineRouteHandler,
};
