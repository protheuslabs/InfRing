#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::shell-socket-lifecycle-ingress.
//
// Gateway bridge for Shell-originated agent/session lifecycle requests. Rust
// shell-socket remains the lifecycle authority; this module makes the Gateway
// ingress seam explicit and bounded for Shell-independent operation.

'use strict';

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function routeForLifecyclePath(pathname) {
  const path = String(pathname || '');
  const rows = [
    [/^\/api\/shell-socket\/agents\/create$/, 'shell_socket.create_agent', 'create_agent', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/archive-all$/, 'shell_socket.archive_all_agents', 'archive_all_agents', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/archived\/delete-all$/, 'shell_socket.delete_all_archived_agents', 'delete_all_archived_agents', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/archive$/, 'shell_socket.archive_agent', 'archive_agent', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/revive$/, 'shell_socket.revive_agent', 'revive_agent', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/clone$/, 'shell_socket.clone_agent', 'clone_agent', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/stop$/, 'shell_socket.stop_agent', 'stop_agent', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/history\/clear$/, 'shell_socket.clear_agent_history', 'clear_agent_history', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/archived\/delete$/, 'shell_socket.delete_archived_agent', 'delete_archived_agent', 'agent_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/sessions$/, 'shell_socket.create_session', 'create_session', 'session_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/sessions\/[^/]+\/switch$/, 'shell_socket.switch_session', 'switch_session', 'session_lifecycle'],
    [/^\/api\/shell-socket\/agents\/[^/]+\/sessions\/[^/]+\/delete$/, 'shell_socket.delete_session', 'delete_session', 'session_lifecycle'],
  ];
  for (const [pattern, routeId, capabilityId, lifecycleFamily] of rows) {
    if (pattern.test(path)) {
      return {
        route_id: routeId,
        capability_id: capabilityId,
        lifecycle_family: lifecycleFamily,
        timeout_ms: 30000,
      };
    }
  }
  return null;
}

function isShellSocketLifecycleIngressPath(pathname) {
  return !!routeForLifecyclePath(pathname);
}

function boundedLifecycleBody(route, body, traceId) {
  const source = body && typeof body === 'object' && !Array.isArray(body) ? body : {};
  const out = {};
  for (const key of ['name', 'label', 'mode', 'contract_id', 'session_id', 'source_agent_id']) {
    if (key in source) out[key] = cleanString(source[key], 180);
  }
  for (const key of ['defaults', 'config', 'metadata']) {
    if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      out[key] = source[key];
    }
  }
  out.gateway_ingress = {
    trace_id: cleanString(traceId, 180),
    route_id: route.route_id,
    capability_id: route.capability_id,
    route_class: 'request_ingress',
    lifecycle_family: route.lifecycle_family,
    gateway_owner: 'gateway.runtime',
    authority_owner: 'core.shell_socket',
    legacy_shell_mutation: false,
  };
  return out;
}

function byteSize(value) {
  try { return Buffer.byteLength(JSON.stringify(value || {}), 'utf8'); } catch { return 65537; }
}

async function shellSocketLifecycleIngress(options) {
  const requestUrl = options && options.requestUrl;
  const route = requestUrl ? routeForLifecyclePath(requestUrl.pathname) : null;
  const traceId = cleanString(options && options.traceId, 180);
  if (!route) {
    return {
      status: 404,
      payload: {
        ok: false,
        type: 'shell_socket_lifecycle_ingress_route_not_found',
        trace_id: traceId,
      },
    };
  }
  const body = boundedLifecycleBody(route, options && options.body, traceId);
  if (byteSize(body) > 65536) {
    return {
      status: 413,
      payload: {
        ok: false,
        type: 'shell_socket_lifecycle_ingress_too_large',
        trace_id: traceId,
        route_id: route.route_id,
        capability_id: route.capability_id,
        max_request_bytes: 65536,
      },
    };
  }
  try {
    const upstream = await options.fetchBackend(options.flags, requestUrl.pathname, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-infring-trace-id': traceId,
      },
      body: JSON.stringify(body),
      cache: 'no-store',
    }, route.timeout_ms);
    const text = await upstream.text();
    let payload = {};
    try { payload = text ? JSON.parse(text) : {}; } catch { payload = {}; }
    const base = payload && typeof payload === 'object' ? payload : {};
    return {
      status: upstream.status || (base.ok === false ? 400 : 202),
      payload: {
        ...base,
        trace_id: traceId,
        gateway_ingress: body.gateway_ingress,
      },
    };
  } catch (error) {
    return {
      status: 503,
      payload: {
        ok: false,
        type: 'shell_socket_lifecycle_ingress_unavailable',
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
  isShellSocketLifecycleIngressPath,
  shellSocketLifecycleIngress,
};
