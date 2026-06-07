#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::dashboard-host-request-boundary.
//
// Gateway owns dashboard host request/upgrade boundary response shape. The
// compatibility host provides concrete server sockets and route handlers, while
// Gateway decides trace headers, proxy admission, not-found/error projections,
// upgrade fallback, and client-error cleanup.

'use strict';

const {
  proxyGatewayHttpRequest,
  proxyGatewayUpgrade,
} = require('./gateway_http_boundary.ts');
const {
  gatewayRequestTraceBoundary,
} = require('./gateway_trace_boundary.ts');
const {
  cleanGatewayText: cleanText,
} = require('./gateway_text_boundary.ts');

function createGatewayDashboardHostRequestBoundary(options = {}) {
  const sendJson = options.sendJson;
  const proxyHttpRequest = typeof options.proxyHttpRequest === 'function'
    ? options.proxyHttpRequest
    : proxyGatewayHttpRequest;
  const proxyUpgrade = typeof options.proxyUpgrade === 'function'
    ? options.proxyUpgrade
    : proxyGatewayUpgrade;
  const requestTraceBoundary = typeof options.requestTraceBoundary === 'function'
    ? options.requestTraceBoundary
    : gatewayRequestTraceBoundary;
  if (typeof sendJson !== 'function') throw new Error('gateway_dashboard_host_request_send_json_missing');

  function applyTraceHeaders(req, res, traceId) {
    try { res.setHeader('x-infring-trace-id', traceId); } catch {}
    try { res.setHeader('x-infring-trace-source', requestTraceBoundary(req).source || 'unknown'); } catch {}
  }

  function isBackendProxyRoute(pathname) {
    return pathname === '/healthz' || String(pathname || '').startsWith('/api/');
  }

  async function proxyDashboardBackendRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const flags = args.flags || {};
    const requestTraceId = args.requestTraceId;
    if (!req || !res || !isBackendProxyRoute(pathname)) return false;
    await proxyHttpRequest(req, res, {
      apiHost: flags.apiHost,
      apiPort: flags.apiPort,
      requestTraceId,
    });
    return true;
  }

  function sendDashboardNotFound(res, pathname) {
    sendJson(res, 404, { ok: false, type: 'infring_dashboard_not_found', path: String(pathname || '') });
  }

  function sendDashboardRequestError(res, error, traceId) {
    const message = cleanText(error && error.message ? error.message : String(error), 260);
    const statusCode = message === 'request_body_invalid_json' || message === 'request_body_too_large' ? 400 : 500;
    sendJson(res, statusCode, {
      ok: false,
      type: 'infring_dashboard_request_error',
      trace_id: String(traceId || ''),
      error: message,
    });
  }

  function handleDashboardUpgrade(args = {}) {
    const req = args.req;
    const socket = args.socket;
    const head = args.head;
    const wsBridge = args.wsBridge || null;
    const agentRuntimeSocketTransport = args.agentRuntimeSocketTransport || null;
    const flags = args.flags || {};
    const requestTraceId = args.requestTraceId;
    if (!req || !socket) return false;
    if (
      agentRuntimeSocketTransport &&
      typeof agentRuntimeSocketTransport.handleUpgrade === 'function' &&
      agentRuntimeSocketTransport.handleUpgrade({ req, socket, head, flags })
    ) return true;
    if (wsBridge && typeof wsBridge.tryHandle === 'function' && wsBridge.tryHandle(req, socket, head)) return true;
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    if (!String(pathname || '').startsWith('/api/')) {
      try { socket.destroy(); } catch {}
      return true;
    }
    proxyUpgrade(req, socket, head, {
      apiHost: flags.apiHost,
      apiPort: flags.apiPort,
      requestTraceId,
    });
    return true;
  }

  function handleClientError(_error, socket) {
    try { socket.destroy(); } catch {}
  }

  return {
    applyTraceHeaders,
    handleClientError,
    handleDashboardUpgrade,
    proxyDashboardBackendRoute,
    sendDashboardNotFound,
    sendDashboardRequestError,
  };
}

module.exports = {
  createGatewayDashboardHostRequestBoundary,
};
