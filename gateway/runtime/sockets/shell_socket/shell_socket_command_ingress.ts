#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::shell-socket-command-ingress.
//
// Thin Gateway bridge for command-bearing Shell socket ingress. Rust
// shell-socket routes remain the authority owner; this module only bounds the
// HTTP shape, preserves trace metadata, and prevents the legacy Shell from
// being the long-term command authority surface.

'use strict';

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function routeForCommandPath(pathname) {
  const path = String(pathname || '');
  if (path === '/api/shell-socket/input') {
    return {
      route_id: 'shell_socket.submit_input',
      capability_id: 'submit_input',
      command_family: 'message_or_slash_input',
      timeout_ms: 120000,
    };
  }
  if (/^\/api\/shell-socket\/agents\/[^/]+\/message$/.test(path)) {
    return {
      route_id: 'shell_socket.submit_message_result',
      capability_id: 'submit_message_result',
      command_family: 'agent_message',
      timeout_ms: 120000,
    };
  }
  if (/^\/api\/shell-socket\/agents\/[^/]+\/model$/.test(path)) {
    return {
      route_id: 'shell_socket.set_model',
      capability_id: 'set_model',
      command_family: 'model_selection',
      timeout_ms: 30000,
    };
  }
  if (path === '/api/shell-socket/terminal/commands') {
    return {
      route_id: 'shell_socket.submit_terminal_command',
      capability_id: 'submit_terminal_command',
      command_family: 'terminal_command',
      timeout_ms: 120000,
    };
  }
  return null;
}

function isShellSocketCommandIngressPath(pathname) {
  return !!routeForCommandPath(pathname);
}

function boundedIngressBody(route, body, traceId) {
  const source = body && typeof body === 'object' && !Array.isArray(body) ? body : {};
  const out = { ...source };
  for (const key of ['agent_id', 'target_agent_id', 'agent_runtime_engine_id', 'model', 'model_id', 'provider']) {
    if (key in out) out[key] = cleanString(out[key], 180);
  }
  for (const key of ['message', 'text', 'input', 'command']) {
    if (key in out) out[key] = cleanString(out[key], 24000);
  }
  out.gateway_ingress = {
    trace_id: cleanString(traceId, 180),
    route_id: route.route_id,
    capability_id: route.capability_id,
    route_class: 'request_ingress',
    command_family: route.command_family,
    gateway_owner: 'gateway.runtime',
    authority_owner: 'core.shell_socket',
    legacy_shell_mutation: false,
  };
  return out;
}

function byteSize(value) {
  try { return Buffer.byteLength(JSON.stringify(value || {}), 'utf8'); } catch { return 65537; }
}

async function shellSocketCommandIngress(options) {
  const requestUrl = options && options.requestUrl;
  const route = requestUrl ? routeForCommandPath(requestUrl.pathname) : null;
  const traceId = cleanString(options && options.traceId, 180);
  if (!route) {
    return {
      status: 404,
      payload: {
        ok: false,
        type: 'shell_socket_command_ingress_route_not_found',
        trace_id: traceId,
      },
    };
  }
  const body = boundedIngressBody(route, options && options.body, traceId);
  if (byteSize(body) > 65536) {
    return {
      status: 413,
      payload: {
        ok: false,
        type: 'shell_socket_command_ingress_too_large',
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
        type: 'shell_socket_command_ingress_unavailable',
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
  isShellSocketCommandIngressPath,
  shellSocketCommandIngress,
};
