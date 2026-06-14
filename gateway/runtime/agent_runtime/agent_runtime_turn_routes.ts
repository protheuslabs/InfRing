#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::turn-routes.
//
// Gateway owns Agent Runtime turn route admission, streaming envelope shape,
// steering ingress, context-pack preview admission, and route-level error
// projection. Adapter hosts may serve HTTP, but they must delegate this route
// behavior here.

'use strict';

const {
  resolveAgentRuntimeEngineId,
  withCanonicalAgentRuntimeEngineId,
} = require('./agent_runtime_engine_identity.ts');

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function requestedTurnRouteTimeoutMs(body) {
  const budget = body && body.capability_budget && typeof body.capability_budget === 'object'
    ? body.capability_budget
    : {};
  const seconds = Number(
    budget.max_absolute_turn_seconds ||
    body && body.max_absolute_turn_seconds ||
    budget.max_turn_seconds ||
    body && body.max_turn_seconds ||
    0
  );
  const boundedSeconds = Number.isFinite(seconds) && seconds > 0
    ? Math.max(1, Math.min(seconds, 3600))
    : 1800;
  return Math.round(boundedSeconds * 1000);
}

function shouldStreamLiveActivityEvent(event) {
  if (!event || typeof event !== 'object') return false;
  if (event.display_in_thinking_bubble === false || event.thinking_bubble_visible === false) return false;
  const kind = cleanText(event.activity_kind || event.kind || event.type, 80).toLowerCase();
  const provider = cleanText(event.provider_event_type || event.event_type || '', 160).toLowerCase();
  const text = cleanText(event.display_text || event.text || event.summary || '', 500).toLowerCase();
  if (/^(context\\.prepare|context\\.loaded|engine\\.health|session\\.start|turn\\.launch|turn\\.complete|assistant\\.draft)$/.test(provider)) return false;
  if (/^(started|completed|activity)$/.test(kind) && (
    /preparing .* conversation context|loaded \\d+ prior context rows|checking .* availability|starting .* session|launching .* bounded context pack|completed the turn|final answer is shown/.test(text)
  )) return false;
  return true;
}

function turnRouteTimeoutPayload(traceId, body, activityEvents = []) {
  const engineId = resolveAgentRuntimeEngineId(body, { defaultEngineId: 'agent_runtime' });
  const timeoutMs = requestedTurnRouteTimeoutMs(body);
  const timeoutSeconds = Math.max(1, Math.round(timeoutMs / 1000));
  const text = `${engineId} did not finish within ${timeoutSeconds}s. Gateway returned a bounded route timeout before the client disconnected.`;
  const priorActivity = Array.isArray(activityEvents)
    ? activityEvents.slice(-24)
    : [];
  return {
    ok: false,
    status_code: 504,
    type: 'agent_runtime_turn_route_timeout',
    trace_id: traceId,
    engine_id: engineId,
    status: 'timed_out_with_reason',
    terminal_outcome: 'timed_out_with_reason',
    error_code: 'agent_runtime_turn_route_timeout',
    reason: text,
    output_text: text,
    output_preview: text,
    retryable: true,
    timed_out: true,
    timeout_ms: timeoutMs,
    activity_events: [...priorActivity, {
      type: 'agent_activity_event',
      trace_id: traceId,
      engine_id: engineId,
      activity_kind: 'error',
      provider_event_type: 'turn.timeout',
      source: 'gateway_runtime_turn_route_timeout',
      sequence_no: 1,
      item_id: 'gateway-route-timeout',
      status: 'timed_out',
      text,
      display_text: text,
    }].slice(-32),
  };
}

function withTurnRouteTimeout(promise, traceId, body, activityEvents = []) {
  let timeout = null;
  const timeoutMs = requestedTurnRouteTimeoutMs(body);
  const timeoutPromise = new Promise((resolve) => {
    timeout = setTimeout(() => resolve(turnRouteTimeoutPayload(traceId, body, activityEvents)), timeoutMs);
    if (timeout && typeof timeout.unref === 'function') timeout.unref();
  });
  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timeout) clearTimeout(timeout);
  });
}

function sendTurnRouteJson(sendJson, res, statusCode, payload) {
  if (!res || res.writableEnded || res.destroyed) return false;
  try {
    sendJson(res, statusCode, payload);
    return true;
  } catch {}
  try {
    if (!res.headersSent) {
      res.writeHead(statusCode, {
        'content-type': 'application/json; charset=utf-8',
        'cache-control': 'no-store',
      });
    }
    res.end(`${JSON.stringify(payload, null, 2)}\n`);
    return true;
  } catch {}
  return false;
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
  const loadAgentRuntimeSelection = typeof options.loadAgentRuntimeSelection === 'function'
    ? options.loadAgentRuntimeSelection
    : null;
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
      const body = withCanonicalAgentRuntimeEngineId(await readJsonBody(req, 65536), {
        loadSelection: loadAgentRuntimeSelection,
        defaultEngineId: 'infring_native',
      });
      const routeActivityEvents = [];
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
      const payload = await withTurnRouteTimeout(turnProjectionStore.agentRuntimeTurnProjection(traceId, body, {
        stream: true,
        nativeOrchestrationClient: createNativeOrchestrationClient(flags),
        onActivity: (event) => {
          routeActivityEvents.push(event);
          if (shouldStreamLiveActivityEvent(event)) {
            writeEvent({ type: 'activity', trace_id: traceId, event });
          }
        },
      }), traceId, body, routeActivityEvents).catch((error) => ({
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
      const body = withCanonicalAgentRuntimeEngineId(await readJsonBody(req, 65536), {
        loadSelection: loadAgentRuntimeSelection,
        defaultEngineId: 'infring_native',
      });
      const routeActivityEvents = [];
      const abortController = typeof AbortController !== 'undefined' ? new AbortController() : null;
      let responded = false;
      const routeTimeoutMs = requestedTurnRouteTimeoutMs(body);
      const routeTimer = setTimeout(() => {
        if (responded || res.writableEnded || res.destroyed) return;
        responded = true;
        if (abortController) {
          try { abortController.abort(); } catch {}
        }
        const timeoutPayload = turnRouteTimeoutPayload(traceId, body, routeActivityEvents);
        sendTurnRouteJson(sendJson, res, timeoutPayload.status_code || 504, timeoutPayload);
      }, routeTimeoutMs);
      const payload = await turnProjectionStore.agentRuntimeTurnProjection(traceId, body, {
        nativeOrchestrationClient: createNativeOrchestrationClient(flags),
        abortSignal: abortController && abortController.signal,
        onActivity: (event) => {
          routeActivityEvents.push(event);
        },
      }).catch((error) => ({
        ok: false,
        status_code: 502,
        type: 'agent_runtime_turn_projection_error',
        trace_id: traceId,
        error: cleanText(error && error.message ? error.message : error, 240),
      }));
      clearTimeout(routeTimer);
      if (!responded && !res.writableEnded && !res.destroyed) {
        responded = true;
        sendTurnRouteJson(sendJson, res, payload.status_code || (payload.ok === false ? 502 : 200), payload);
      }
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/steer' || pathname === '/api/agent-runtime/steer')) {
      const body = withCanonicalAgentRuntimeEngineId(await readJsonBody(req, 65536).catch(() => ({})), {
        loadSelection: loadAgentRuntimeSelection,
        defaultEngineId: 'infring_native',
      });
      const payload = steer(traceId, body);
      sendJson(res, payload.status_code || (payload.ok === false ? 400 : 200), payload);
      return true;
    }

    if (req.method === 'POST' && (pathname === '/api/shell-socket/agent-runtime/context-pack/preview' || pathname === '/api/agent-runtime/context-pack/preview')) {
      const body = withCanonicalAgentRuntimeEngineId(await readJsonBody(req, 65536).catch(() => ({})), {
        loadSelection: loadAgentRuntimeSelection,
        defaultEngineId: 'infring_native',
      });
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
