#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::shell-socket-status-projection.
//
// Bounded Gateway status projection for Shell sockets. The Shell should render
// health/connectivity labels from this projection instead of inferring runtime
// truth locally.

'use strict';

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function isShellSocketStatusProjectionPath(pathname) {
  return String(pathname || '') === '/api/shell-socket/runtime-status';
}

function statusMetadata(traceId) {
  return {
    trace_id: cleanString(traceId, 180),
    route_id: 'shell_socket.runtime_status',
    capability_id: 'get_runtime_status',
    route_class: 'health_status',
    projection_owner: 'core.shell_socket',
    gateway_owner: 'adapters.runtime',
    bounded: true,
    shell_inference_allowed: false,
  };
}

async function shellSocketStatusProjection(options) {
  const traceId = cleanString(options && options.traceId, 180);
  try {
    const payload = await options.fetchBackendJson(options.flags, '/api/shell-socket/runtime-status', 5000);
    const base = payload && typeof payload === 'object' ? payload : {};
    return {
      status: base.ok === false ? 503 : 200,
      payload: {
        ...base,
        ok: base.ok !== false,
        trace_id: traceId,
        gateway_projection: statusMetadata(traceId),
      },
    };
  } catch (error) {
    const fallback = await options.statusPayloadWithBootStage(options.flags);
    return {
      status: fallback && fallback.ok === false ? 503 : 200,
      payload: {
        ...(fallback && typeof fallback === 'object' ? fallback : {}),
        trace_id: traceId,
        warning: cleanString(error && error.message ? error.message : 'shell_socket_runtime_status_unavailable', 240),
        gateway_projection: {
          ...statusMetadata(traceId),
          projection_owner: 'adapters.runtime.status_fallback',
        },
      },
    };
  }
}

module.exports = {
  isShellSocketStatusProjectionPath,
  shellSocketStatusProjection,
};
