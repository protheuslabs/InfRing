#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::turn-routes.
//
// Gateway owns Agent Runtime turn route admission, streaming envelope shape,
// steering ingress, context-pack preview admission, and route-level error
// projection. Adapter hosts may serve HTTP, but they must delegate this route
// behavior here.

'use strict';

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function isAgentRuntimeTurnRoute(pathname) {
  return pathname === '/api/shell-socket/agent-runtime/turn/stream' ||
    pathname === '/api/agent-runtime/turn/stream' ||
    pathname === '/api/shell-socket/agent-runtime/turn' ||
    pathname === '/api/agent-runtime/turn' ||
    pathname === '/api/shell-socket/agent-runtime/steer' ||
    pathname === '/api/agent-runtime/steer' ||
    pathname === '/api/shell-socket/agent-runtime/context-pack/preview' ||
    pathname === '/api/agent-runtime/context-pack/preview';
}

function createAgentRuntimeTurnRouteHandler(options = {}) {
  const turnProjectionStore = options.turnProjectionStore;
  const contextPreviewProjectionStore = options.contextPreviewProjectionStore;
  const steer = options.steer;
  const createNativeOrchestrationClient = options.createNativeOrchestrationClient;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  if (!turnProjectionStore || typeof turnProjectionStore.agentRuntimeTurnProjection !== 'function') {
    throw new Error('agent_runtime_turn_route_projection_store_missing');
  }
  if (!contextPreviewProjectionStore || typeof contextPreviewProjectionStore.agentRuntimeContextPackPreviewProjection !== 'function') {
    throw new Error('agent_runtime_turn_route_context_preview_store_missing');
  }
  if (typeof steer !== 'function') {
    throw new Error('agent_runtime_turn_route_steer_missing');
  }
  if (typeof createNativeOrchestrationClient !== 'function') {
    throw new Error('agent_runtime_turn_route_native_orchestration_client_missing');
  }
  if (typeof readJsonBody !== 'function') {
    throw new Error('agent_runtime_turn_route_read_json_body_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('agent_runtime_turn_route_send_json_missing');
  }

  async function handleAgentRuntimeTurnRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const traceId = String(args.traceId || '');
    const flags = args.flags || {};
    if (!req || !res || !isAgentRuntimeTurnRoute(pathname)) return false;

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
      const payload = await turnProjectionStore.agentRuntimeTurnProjection(traceId, body, {
        stream: true,
        nativeOrchestrationClient: createNativeOrchestrationClient(flags),
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
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/turn' || pathname === '/api/agent-runtime/turn')) {
      const body = await readJsonBody(req, 65536);
      const payload = await turnProjectionStore.agentRuntimeTurnProjection(traceId, body, {
        nativeOrchestrationClient: createNativeOrchestrationClient(flags),
      }).catch((error) => ({
        ok: false,
        status_code: 502,
        type: 'agent_runtime_turn_projection_error',
        trace_id: traceId,
        error: cleanText(error && error.message ? error.message : error, 240),
      }));
      sendJson(res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/steer' || pathname === '/api/agent-runtime/steer')) {
      const body = await readJsonBody(req, 65536).catch(() => ({}));
      const payload = steer(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/context-pack/preview' || pathname === '/api/agent-runtime/context-pack/preview')) {
      const body = await readJsonBody(req, 65536).catch(() => ({}));
      const payload = await contextPreviewProjectionStore.agentRuntimeContextPackPreviewProjection(traceId, body).catch((error) => ({
        ok: false,
        status_code: 502,
        type: 'agent_runtime_context_pack_preview_error',
        trace_id: traceId,
        error: cleanText(error && error.message ? error.message : error, 240),
      }));
      sendJson(res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'agent_runtime_turn_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: String(req.method || ''),
      pathname,
    });
    return true;
  }

  return {
    isAgentRuntimeTurnRoute,
    handleAgentRuntimeTurnRoute,
  };
}

module.exports = {
  isAgentRuntimeTurnRoute,
  createAgentRuntimeTurnRouteHandler,
};
