#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::system-routes.
//
// Gateway owns bounded system/update route projections. Adapter hosts may serve
// HTTP, but they must delegate route admission and fallback shape here.

'use strict';

function isGatewaySystemRoute(pathname) {
  return String(pathname || '') === '/api/system/release-check';
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
        gateway_projection: {
          route_id: 'gateway.system.release_check',
          route_class: 'health_status',
          gateway_owner: 'gateway.runtime',
          fallback: true,
        },
      }));
      sendJson(res, 200, {
        ...(payload && typeof payload === 'object' ? payload : {}),
        trace_id: traceId,
        gateway_projection: {
          route_id: 'gateway.system.release_check',
          route_class: 'health_status',
          gateway_owner: 'gateway.runtime',
          bounded: true,
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
