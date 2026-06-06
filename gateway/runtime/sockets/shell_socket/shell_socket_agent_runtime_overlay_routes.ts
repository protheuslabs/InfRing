#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::sockets::shell-socket::agent-runtime-overlay-routes.
//
// Gateway owns Shell Socket chat/session projection overlays. The legacy
// dashboard process may host HTTP, but it must not decide how Agent Runtime
// transcript rows are merged into Shell-facing session/chat responses.

'use strict';

const {
  isShellSocketChatProjectionPath,
  shellSocketChatProjection,
} = require('./shell_socket_chat_projection.ts');

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function isShellSocketAgentRuntimeOverlayRoute(pathname) {
  return isShellSocketChatProjectionPath(pathname) ||
    /^\/api\/agents\/[^/]+\/session$/.test(String(pathname || ''));
}

function createShellSocketAgentRuntimeOverlayRouteHandler(options = {}) {
  const transcriptStore = options.transcriptStore;
  const fetchBackendJson = options.fetchBackendJson;
  const sendJson = options.sendJson;
  if (!transcriptStore || typeof transcriptStore.mergeAgentRuntimeTranscriptPayload !== 'function') {
    throw new Error('shell_socket_agent_runtime_overlay_transcript_store_missing');
  }
  if (typeof transcriptStore.agentRuntimeTranscriptFilterFromShellSocketPath !== 'function') {
    throw new Error('shell_socket_agent_runtime_overlay_filter_missing');
  }
  if (typeof fetchBackendJson !== 'function') {
    throw new Error('shell_socket_agent_runtime_overlay_fetch_backend_json_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('shell_socket_agent_runtime_overlay_send_json_missing');
  }

  async function handleShellSocketAgentRuntimeOverlayRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const requestUrl = args.requestUrl;
    const traceId = String(args.traceId || '');
    const flags = args.flags || {};
    if (!req || !res || !isShellSocketAgentRuntimeOverlayRoute(pathname)) return false;

    const legacyAgentSessionMatch = pathname.match(/^\/api\/agents\/([^/]+)\/session$/);
    if (req.method === 'GET' && legacyAgentSessionMatch) {
      const agentId = decodeURIComponent(legacyAgentSessionMatch[1] || '');
      const upstreamPath = `${pathname}${requestUrl && requestUrl.search ? requestUrl.search : ''}`;
      const payload = await fetchBackendJson(flags, upstreamPath, 10000, traceId).catch((error) => ({
        ok: false,
        type: 'agent_session_projection_unavailable',
        trace_id: traceId,
        error: cleanText(error && error.message ? error.message : error, 240),
      }));
      const sessionId = payload && (payload.session_id || payload.current_session_id || (payload.session && payload.session.id));
      const merged = transcriptStore.mergeAgentRuntimeTranscriptPayload(payload, {
        agentId,
        sessionId,
        limit: requestUrl && requestUrl.searchParams ? requestUrl.searchParams.get('limit') : undefined,
      });
      sendJson(res, merged.ok === false ? 502 : 200, merged);
      return true;
    }

    if (req.method === 'GET' && isShellSocketChatProjectionPath(pathname)) {
      const result = await shellSocketChatProjection({ flags, requestUrl, traceId, fetchBackendJson });
      const filter = transcriptStore.agentRuntimeTranscriptFilterFromShellSocketPath(pathname);
      const payload = filter
        ? transcriptStore.mergeAgentRuntimeTranscriptPayload(result.payload, {
          agentId: filter.agentId,
          sessionId: filter.sessionId,
          limit: requestUrl && requestUrl.searchParams ? requestUrl.searchParams.get('limit') : undefined,
        })
        : result.payload;
      sendJson(res, result.status, payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'shell_socket_agent_runtime_overlay_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: String(req.method || ''),
      pathname,
    });
    return true;
  }

  return {
    isShellSocketAgentRuntimeOverlayRoute,
    handleShellSocketAgentRuntimeOverlayRoute,
  };
}

module.exports = {
  isShellSocketAgentRuntimeOverlayRoute,
  createShellSocketAgentRuntimeOverlayRouteHandler,
};
