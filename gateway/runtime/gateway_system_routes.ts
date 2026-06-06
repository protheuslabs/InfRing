#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::system-routes.
//
// Gateway owns bounded system/update route projections. Adapter hosts may serve
// HTTP, but they must delegate route admission and fallback shape here.

'use strict';

function isGatewaySystemRoute(pathname) {
  const path = String(pathname || '');
  return path === '/api/system/release-check' ||
    path === '/api/config' ||
    path === '/api/config/schema' ||
    path === '/api/auth/check';
}

function systemProjection(traceId, routeId, routeClass = 'health_status') {
  return {
    route_id: routeId,
    route_class: routeClass,
    gateway_owner: 'gateway.runtime',
    bounded: true,
    trace_id: traceId,
  };
}

function createGatewaySystemRouteHandler(options = {}) {
  const fetchBackendJson = options.fetchBackendJson;
  const sendJson = options.sendJson;
  if (typeof fetchBackendJson !== 'function') {
    throw new Error('gateway_system_route_fetch_backend_json_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('gateway_system_route_send_json_missing');
  }

  async function handleGatewaySystemRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const requestUrl = args.requestUrl;
    const traceId = String(args.traceId || '');
    const flags = args.flags || {};
    if (!req || !res || !isGatewaySystemRoute(pathname)) return false;

    if (req.method === 'GET' && pathname === '/api/system/release-check') {
      const qs = requestUrl && requestUrl.search ? requestUrl.search : '';
      const payload = await fetchBackendJson(flags, `/api/update/check${qs}`, 5000, traceId).catch(() => ({
        ok: true,
        update_available: false,
        gateway_projection: { ...systemProjection(traceId, 'gateway.system.release_check'), fallback: true },
      }));
      sendJson(res, 200, {
        ...(payload && typeof payload === 'object' ? payload : {}),
        trace_id: traceId,
        gateway_projection: {
          ...systemProjection(traceId, 'gateway.system.release_check'),
          ...((payload && payload.gateway_projection && typeof payload.gateway_projection === 'object') ? payload.gateway_projection : {}),
        },
      });
      return true;
    }

    if (req.method === 'GET' && pathname === '/api/config') {
      const payload = await fetchBackendJson(flags, '/api/config', 8000, traceId).catch(() => ({
        ok: false,
        error: 'config_unavailable',
      }));
      sendJson(res, 200, {
        ...(payload && typeof payload === 'object' ? payload : {}),
        trace_id: traceId,
        gateway_projection: {
          ...systemProjection(traceId, 'gateway.system.config', 'bounded_search_query'),
          ...((payload && payload.gateway_projection && typeof payload.gateway_projection === 'object') ? payload.gateway_projection : {}),
        },
      });
      return true;
    }

    if (req.method === 'GET' && pathname === '/api/config/schema') {
      const payload = await fetchBackendJson(flags, '/api/config/schema', 8000, traceId).catch(() => ({
        ok: true,
        sections: {},
      }));
      sendJson(res, 200, {
        ...(payload && typeof payload === 'object' ? payload : {}),
        trace_id: traceId,
        gateway_projection: {
          ...systemProjection(traceId, 'gateway.system.config_schema', 'bounded_search_query'),
          ...((payload && payload.gateway_projection && typeof payload.gateway_projection === 'object') ? payload.gateway_projection : {}),
        },
      });
      return true;
    }

    if (req.method === 'GET' && pathname === '/api/auth/check') {
      const payload = await fetchBackendJson(flags, '/api/auth/check', 8000, traceId).catch(() => ({
        ok: true,
        mode: 'none',
        authenticated: true,
        user: 'operator',
      }));
      sendJson(res, 200, {
        ...(payload && typeof payload === 'object' ? payload : {}),
        trace_id: traceId,
        gateway_projection: {
          ...systemProjection(traceId, 'gateway.system.auth_check'),
          ...((payload && payload.gateway_projection && typeof payload.gateway_projection === 'object') ? payload.gateway_projection : {}),
        },
      });
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'gateway_system_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: String(req.method || ''),
      pathname,
    });
    return true;
  }

  return {
    isGatewaySystemRoute,
    handleGatewaySystemRoute,
  };
}

module.exports = {
  isGatewaySystemRoute,
  createGatewaySystemRouteHandler,
};
