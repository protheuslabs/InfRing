#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::command-routes.
//
// Gateway owns runtime command route admission. Shell/CLI clients may request
// projections and submit canonical command intents, but must not execute raw
// provider slash/CLI commands directly.

'use strict';

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function readQueryEngineId(requestUrl) {
  if (!requestUrl || !requestUrl.searchParams) return '';
  return cleanText(requestUrl.searchParams.get('engine_id') || requestUrl.searchParams.get('engineId') || '', 120);
}

function isAgentRuntimeCommandRoute(pathname) {
  return pathname === '/api/shell-socket/agent-runtime/commands' ||
    pathname === '/api/agent-runtime/commands' ||
    pathname === '/api/shell-socket/agent-runtime/commands/execute' ||
    pathname === '/api/agent-runtime/commands/execute';
}

function createAgentRuntimeCommandRouteHandler(options = {}) {
  const commandCatalogStore = options.commandCatalogStore;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  if (!commandCatalogStore || typeof commandCatalogStore.agentRuntimeCommandCatalogProjection !== 'function') {
    throw new Error('agent_runtime_command_route_catalog_store_missing');
  }
  if (typeof commandCatalogStore.agentRuntimeCommandActionProjection !== 'function') {
    throw new Error('agent_runtime_command_route_action_store_missing');
  }
  if (typeof readJsonBody !== 'function') {
    throw new Error('agent_runtime_command_route_read_json_body_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('agent_runtime_command_route_send_json_missing');
  }

  async function handleAgentRuntimeCommandRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const traceId = String(args.traceId || '');
    const requestUrl = args.requestUrl;
    if (!req || !res || !isAgentRuntimeCommandRoute(pathname)) return false;

    if (req.method === 'GET' && (pathname === '/api/shell-socket/agent-runtime/commands' || pathname === '/api/agent-runtime/commands')) {
      const payload = commandCatalogStore.agentRuntimeCommandCatalogProjection(traceId, {
        engine_id: readQueryEngineId(requestUrl),
      });
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/commands' || pathname === '/api/agent-runtime/commands')) {
      const body = await readJsonBody(req, 8192).catch(() => ({}));
      const payload = commandCatalogStore.agentRuntimeCommandCatalogProjection(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/commands/execute' || pathname === '/api/agent-runtime/commands/execute')) {
      const body = await readJsonBody(req, 16384).catch(() => ({}));
      const payload = await commandCatalogStore.agentRuntimeCommandActionProjection(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 202), payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'agent_runtime_command_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: cleanText(req.method || '', 32),
      pathname,
    });
    return true;
  }

  return {
    isAgentRuntimeCommandRoute,
    handleAgentRuntimeCommandRoute,
  };
}

module.exports = {
  isAgentRuntimeCommandRoute,
  createAgentRuntimeCommandRouteHandler,
};
