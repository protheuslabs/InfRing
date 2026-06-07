#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::dashboard-static-responses.
//
// Gateway owns dashboard static/compat response routing shape. Compatibility
// hosts inject asset readers and HTTP response primitives; this module decides
// bounded route responses for dashboard HTML, version/status, retired surfaces,
// and static assets.

'use strict';

const path = require('node:path');
const {
  gatewayStatusPayloadWithBootStage,
} = require('./gateway_status_projection.ts');
const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

function createGatewayDashboardStaticResponseController(options = {}) {
  const staticDir = options.staticDir || '';
  const buildPrimaryDashboardHtml = typeof options.buildPrimaryDashboardHtml === 'function'
    ? options.buildPrimaryDashboardHtml
    : () => '';
  const readPrimaryDashboardAsset = typeof options.readPrimaryDashboardAsset === 'function'
    ? options.readPrimaryDashboardAsset
    : () => null;
  const fetchBackendJson = typeof options.fetchBackendJson === 'function' ? options.fetchBackendJson : null;
  const sendJson = typeof options.sendJson === 'function' ? options.sendJson : null;
  const mergeDashboardVersionPayload = typeof options.mergeDashboardVersionPayload === 'function'
    ? options.mergeDashboardVersionPayload
    : (payload) => payload;
  const statusPayloadWithBootStage = typeof options.statusPayloadWithBootStage === 'function'
    ? options.statusPayloadWithBootStage
    : gatewayStatusPayloadWithBootStage;

  if (!sendJson) throw new Error('gateway_dashboard_static_send_json_missing');

  let dashboardHtmlCache = '';

  function loadDashboardHtml() {
    dashboardHtmlCache = buildPrimaryDashboardHtml(staticDir) || dashboardHtmlCache || '';
    return dashboardHtmlCache;
  }

  function isDashboardHtmlRoute(req, pathname) {
    return (req.method === 'GET' || req.method === 'HEAD') &&
      (pathname === '/dashboard' || pathname === '/dashboard/' || (pathname.startsWith('/dashboard/') && !path.extname(pathname)));
  }

  async function handleGatewayDashboardStaticRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const flags = args.flags || {};
    if (!req || !res) return false;

    if ((req.method === 'GET' || req.method === 'HEAD') && (pathname === '/dashboard-classic' || pathname === '/dashboard-shell')) {
      res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' });
      res.end(req.method === 'HEAD' ? '' : 'dashboard_surface_retired');
      return true;
    }

    if ((req.method === 'GET' || req.method === 'HEAD') && pathname === '/') {
      res.writeHead(302, { location: '/dashboard', 'cache-control': 'no-store' });
      res.end();
      return true;
    }

    if (isDashboardHtmlRoute(req, pathname)) {
      const html = loadDashboardHtml();
      res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
      res.end(req.method === 'HEAD' ? '' : html);
      return true;
    }

    if (req.method === 'GET' && pathname === '/api/status') {
      const status = mergeDashboardVersionPayload(await statusPayloadWithBootStage(flags));
      sendJson(res, 200, status);
      return true;
    }

    if (req.method === 'GET' && pathname === '/api/version') {
      const versionPayload = fetchBackendJson
        ? await fetchBackendJson(flags, '/api/version', 4000).catch(() => ({ ok: true }))
        : { ok: true };
      sendJson(res, 200, mergeDashboardVersionPayload(versionPayload));
      return true;
    }

    if (req.method === 'GET') {
      const agentSessionsMatch = pathname.match(/^\/api\/agents\/([^/]+)\/sessions$/);
      if (agentSessionsMatch) {
        const rawAgentId = String(agentSessionsMatch[1] || '').trim();
        let decodedAgentId = rawAgentId;
        try { decodedAgentId = decodeURIComponent(rawAgentId); } catch {}
        const normalizedAgentId = cleanText(decodedAgentId, 120).toLowerCase();
        if (normalizedAgentId === 'system') {
          sendJson(res, 200, {
            ok: true,
            agent_id: 'system',
            sessions: [],
            system_thread: true,
          });
          return true;
        }
      }
    }

    if (req.method === 'GET') {
      const asset = readPrimaryDashboardAsset(staticDir, pathname);
      if (asset) {
        res.writeHead(200, { 'content-type': asset.contentType, 'cache-control': 'no-store' });
        res.end(asset.body);
        return true;
      }
    }

    return false;
  }

  return {
    handleGatewayDashboardStaticRoute,
    loadDashboardHtml,
  };
}

module.exports = {
  createGatewayDashboardStaticResponseController,
};
