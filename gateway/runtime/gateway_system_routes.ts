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
    path === '/api/auth/check' ||
    path === '/api/system/restart' ||
    path === '/api/system/update' ||
    path === '/api/system/shutdown';
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

function receiptRefFromPayload(payload) {
  if (!payload || typeof payload !== 'object') return null;
  if (typeof payload.receipt_ref === 'string' && payload.receipt_ref.trim()) return payload.receipt_ref.trim();
  if (payload.receipt && typeof payload.receipt === 'object') {
    if (typeof payload.receipt.ref === 'string' && payload.receipt.ref.trim()) return payload.receipt.ref.trim();
    if (typeof payload.receipt.id === 'string' && payload.receipt.id.trim()) return payload.receipt.id.trim();
    if (typeof payload.receipt.receipt_ref === 'string' && payload.receipt.receipt_ref.trim()) return payload.receipt.receipt_ref.trim();
  }
  if (Array.isArray(payload.receipts)) {
    for (const row of payload.receipts) {
      const ref = receiptRefFromPayload({ receipt: row });
      if (ref) return ref;
    }
  }
  return null;
}

function systemActionRequiresReceipt(action, body) {
  if (action === 'update' && body && body.apply === false) return false;
  return action === 'restart' || action === 'shutdown' || action === 'update';
}

function systemActionEffectMode(action, body) {
  if (action === 'update' && body && body.apply === false) return 'dry_run';
  return 'mutating';
}

function systemActionReceiptProjection(action, body, payload, source) {
  const receiptRef = receiptRefFromPayload(payload);
  const receiptRequired = systemActionRequiresReceipt(action, body);
  return {
    action,
    effect_mode: systemActionEffectMode(action, body),
    receipt_required: receiptRequired,
    receipt_present: Boolean(receiptRef),
    receipt_ref: receiptRef,
    receipt_owner: source === 'legacy' ? 'legacy_dashboard_host_shim' : 'core.ops',
    receipt_source: source === 'legacy' ? 'legacy_dashboard_host_fallback' : 'core_forwarded_response',
  };
}

function createGatewaySystemRouteHandler(options = {}) {
  const fetchBackendJson = options.fetchBackendJson;
  const fetchBackend = options.fetchBackend;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  const legacyHostFallback = options.legacyHostFallback;
  const onHostShutdownAccepted = options.onHostShutdownAccepted;
  if (typeof fetchBackendJson !== 'function') {
    throw new Error('gateway_system_route_fetch_backend_json_missing');
  }
  if (typeof fetchBackend !== 'function') {
    throw new Error('gateway_system_route_fetch_backend_missing');
  }
  if (typeof readJsonBody !== 'function') {
    throw new Error('gateway_system_route_read_json_body_missing');
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

    if (req.method === 'POST' && (pathname === '/api/system/restart' || pathname === '/api/system/update' || pathname === '/api/system/shutdown')) {
      const action = pathname.split('/').pop() || '';
      const body = await readJsonBody(req).catch(() => ({}));
      try {
        const upstream = await fetchBackend(flags, pathname, {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-infring-trace-id': traceId,
          },
          body: JSON.stringify(body || {}),
          cache: 'no-store',
        }, action === 'update' && body && body.apply === false ? 8000 : 3500);
        const text = await upstream.text();
        let payload = {};
        try { payload = text ? JSON.parse(text) : {}; } catch { payload = {}; }
        const base = payload && typeof payload === 'object' ? payload : { ok: upstream.ok };
        const receiptProjection = systemActionReceiptProjection(action, body, base, 'core');
        if (base.ok !== false && receiptProjection.receipt_required && !receiptProjection.receipt_present) {
          sendJson(res, 502, {
            ok: false,
            type: 'gateway_system_action_receipt_missing',
            trace_id: traceId,
            action,
            error: 'core_system_action_missing_receipt',
            gateway_projection: {
              ...systemProjection(traceId, `gateway.system.${action}`, 'request_ingress'),
              authority_owner: 'core.ops',
              forwarded_to_core: true,
              system_action_receipt: receiptProjection,
            },
          });
          return true;
        }
        sendJson(res, upstream.status || (base.ok === false ? 400 : 200), {
          ...base,
          trace_id: traceId,
          gateway_projection: {
            ...systemProjection(traceId, `gateway.system.${action}`, 'request_ingress'),
            authority_owner: 'core.ops',
            forwarded_to_core: true,
            system_action_receipt: receiptProjection,
          },
        });
        if (action === 'shutdown' && base.ok !== false && typeof onHostShutdownAccepted === 'function') {
          onHostShutdownAccepted(body, base);
        }
        return true;
      } catch (error) {
        if (typeof legacyHostFallback !== 'function') {
          sendJson(res, 503, {
            ok: false,
            type: 'gateway_system_action_core_unavailable',
            trace_id: traceId,
            action,
            error: String(error && error.message ? error.message : error).replace(/\s+/g, ' ').trim().slice(0, 240),
            gateway_projection: {
              ...systemProjection(traceId, `gateway.system.${action}`, 'request_ingress'),
              authority_owner: 'core.ops',
              forwarded_to_core: false,
              legacy_host_fallback_available: false,
            },
          });
          return true;
        }
        const fallback = legacyHostFallback(action, body, { traceId, coreError: error });
        const payload = fallback && typeof fallback === 'object' ? fallback : { ok: false, error: 'legacy_host_fallback_invalid' };
        const receiptProjection = systemActionReceiptProjection(action, body, payload, 'legacy');
        if (payload.ok && receiptProjection.receipt_required && !receiptProjection.receipt_present) {
          sendJson(res, 502, {
            ok: false,
            type: 'gateway_system_action_legacy_fallback_receipt_missing',
            trace_id: traceId,
            action,
            error: 'legacy_dashboard_host_fallback_missing_receipt',
            gateway_projection: {
              ...systemProjection(traceId, `gateway.system.${action}`, 'request_ingress'),
              authority_owner: 'legacy_dashboard_host_shim',
              target_authority_owner: 'core.ops',
              forwarded_to_core: false,
              legacy_host_fallback: true,
              legacy_host_fallback_reason: 'core_system_action_route_unavailable',
              system_action_receipt: receiptProjection,
            },
          });
          return true;
        }
        sendJson(res, payload.ok ? 200 : 500, {
          ...payload,
          trace_id: traceId,
          gateway_projection: {
            ...systemProjection(traceId, `gateway.system.${action}`, 'request_ingress'),
            authority_owner: 'legacy_dashboard_host_shim',
            target_authority_owner: 'core.ops',
            forwarded_to_core: false,
            legacy_host_fallback: true,
            legacy_host_fallback_reason: 'core_system_action_route_unavailable',
            system_action_receipt: receiptProjection,
          },
        });
        if (action === 'shutdown' && payload.ok && typeof onHostShutdownAccepted === 'function') {
          onHostShutdownAccepted(body, payload);
        }
        return true;
      }
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
  systemActionRequiresReceipt,
  systemActionEffectMode,
  receiptRefFromPayload,
};
