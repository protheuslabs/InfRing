#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::shell-socket-chat-projection.
//
// Thin Gateway bridge for Shell-independent chat/session projections. This file
// does not own chat/session truth; Rust shell-socket routes do. The bridge makes
// the Gateway seam explicit, caps caller-controlled query shape, and leaves the
// fragile legacy Shell untouched.

'use strict';

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function parsePositiveInt(value, fallback, min, max) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}

function routeForPath(pathname) {
  const path = String(pathname || '');
  if (path === '/api/shell-socket/agents') {
    return {
      route_id: 'shell_socket.agent_roster',
      capability_id: 'list_agents',
      max_limit: 100,
      default_limit: 50,
      timeout_ms: 8000,
    };
  }
  if (/^\/api\/shell-socket\/agents\/[^/]+\/sessions$/.test(path)) {
    return {
      route_id: 'shell_socket.session_list',
      capability_id: 'list_sessions',
      max_limit: 100,
      default_limit: 40,
      timeout_ms: 8000,
    };
  }
  if (/^\/api\/shell-socket\/sessions\/[^/]+\/messages$/.test(path)) {
    return {
      route_id: 'shell_socket.message_window',
      capability_id: 'get_message_window',
      max_limit: 80,
      default_limit: 80,
      timeout_ms: 10000,
    };
  }
  return null;
}

function isShellSocketChatProjectionPath(pathname) {
  return !!routeForPath(pathname);
}

function boundedProjectionPath(requestUrl, route) {
  const search = new URLSearchParams();
  const limit = parsePositiveInt(
    requestUrl.searchParams.get('limit'),
    route.default_limit,
    1,
    route.max_limit,
  );
  const cursor = cleanString(requestUrl.searchParams.get('cursor') || '', 120);
  search.set('limit', String(limit));
  if (cursor) search.set('cursor', cursor);
  const query = search.toString();
  return `${requestUrl.pathname}${query ? `?${query}` : ''}`;
}

function projectionMetadata(route, traceId) {
  return {
    trace_id: cleanString(traceId, 180),
    route_id: route.route_id,
    capability_id: route.capability_id,
    projection_owner: 'core.shell_socket',
    gateway_owner: 'adapters.runtime',
    bounded: true,
    legacy_shell_mutation: false,
  };
}

async function shellSocketChatProjection(options) {
  const requestUrl = options && options.requestUrl;
  const route = requestUrl ? routeForPath(requestUrl.pathname) : null;
  const traceId = cleanString(options && options.traceId, 180);
  if (!route) {
    return {
      status: 404,
      payload: {
        ok: false,
        type: 'shell_socket_chat_projection_route_not_found',
        trace_id: traceId,
      },
    };
  }
  try {
    const path = boundedProjectionPath(requestUrl, route);
    const payload = await options.fetchBackendJson(options.flags, path, route.timeout_ms);
    const base = payload && typeof payload === 'object' ? payload : {};
    return {
      status: base.ok === false ? 502 : 200,
      payload: {
        ...base,
        ok: base.ok !== false,
        trace_id: traceId,
        gateway_projection: projectionMetadata(route, traceId),
      },
    };
  } catch (error) {
    return {
      status: 503,
      payload: {
        ok: false,
        type: 'shell_socket_chat_projection_unavailable',
        trace_id: traceId,
        route_id: route.route_id,
        capability_id: route.capability_id,
        error: cleanString(error && error.message ? error.message : error, 240),
        retry_after_ms: 1000,
      },
    };
  }
}

module.exports = {
  isShellSocketChatProjectionPath,
  shellSocketChatProjection,
};
