#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::sockets::shell-socket::core-routes.
//
// Gateway owns Shell Socket status projection and bounded command/eval/lifecycle
// ingress routing. Adapter hosts may expose HTTP, but they must delegate route
// admission and response shaping here.

'use strict';

const {
  isShellSocketCommandIngressPath,
  shellSocketCommandIngress,
} = require('./shell_socket_command_ingress.ts');
const {
  isShellSocketStatusProjectionPath,
  shellSocketStatusProjection,
} = require('./shell_socket_status_projection.ts');
const {
  isShellSocketEvalIssueIngressPath,
  shellSocketEvalIssueIngress,
} = require('./shell_socket_eval_issue_ingress.ts');
const {
  isShellSocketLifecycleIngressPath,
  shellSocketLifecycleIngress,
} = require('./shell_socket_lifecycle_ingress.ts');

function isShellSocketCoreRoute(pathname) {
  return isShellSocketStatusProjectionPath(pathname) ||
    isShellSocketCommandIngressPath(pathname) ||
    isShellSocketEvalIssueIngressPath(pathname) ||
    isShellSocketLifecycleIngressPath(pathname);
}

function createShellSocketCoreRouteHandler(options = {}) {
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  const fetchBackend = options.fetchBackend;
  const fetchBackendJson = options.fetchBackendJson;
  const statusPayloadWithBootStage = options.statusPayloadWithBootStage;
  if (typeof readJsonBody !== 'function') {
    throw new Error('shell_socket_core_route_read_json_body_missing');
  }
  if (typeof sendJson !== 'function') {
    throw new Error('shell_socket_core_route_send_json_missing');
  }
  if (typeof fetchBackend !== 'function') {
    throw new Error('shell_socket_core_route_fetch_backend_missing');
  }
  if (typeof fetchBackendJson !== 'function') {
    throw new Error('shell_socket_core_route_fetch_backend_json_missing');
  }
  if (typeof statusPayloadWithBootStage !== 'function') {
    throw new Error('shell_socket_core_route_status_payload_missing');
  }

  async function handleShellSocketCoreRoute(args = {}) {
    const req = args.req;
    const res = args.res;
    const pathname = String(args.pathname || '');
    const requestUrl = args.requestUrl;
    const traceId = String(args.traceId || '');
    const flags = args.flags || {};
    if (!req || !res || !isShellSocketCoreRoute(pathname)) return false;

    if (req.method === 'GET' && isShellSocketStatusProjectionPath(pathname)) {
      const result = await shellSocketStatusProjection({ flags, traceId, fetchBackendJson, statusPayloadWithBootStage });
      sendJson(res, result.status, result.payload);
      return true;
    }

    if (req.method === 'POST' && isShellSocketCommandIngressPath(pathname)) {
      const body = await readJsonBody(req, 65536);
      const result = await shellSocketCommandIngress({ flags, requestUrl, traceId, body, fetchBackend });
      sendJson(res, result.status, result.payload);
      return true;
    }

    if (req.method === 'POST' && isShellSocketEvalIssueIngressPath(pathname)) {
      const body = await readJsonBody(req, 65536);
      const result = await shellSocketEvalIssueIngress({ flags, traceId, body, fetchBackend });
      sendJson(res, result.status, result.payload);
      return true;
    }

    if (req.method === 'POST' && isShellSocketLifecycleIngressPath(pathname)) {
      const body = await readJsonBody(req, 65536);
      const result = await shellSocketLifecycleIngress({ flags, requestUrl, traceId, body, fetchBackend });
      sendJson(res, result.status, result.payload);
      return true;
    }

    sendJson(res, 405, {
      ok: false,
      type: 'shell_socket_core_route_error',
      trace_id: traceId,
      error: 'method_not_allowed',
      method: String(req.method || ''),
      pathname,
    });
    return true;
  }

  return {
    isShellSocketCoreRoute,
    handleShellSocketCoreRoute,
  };
}

module.exports = {
  isShellSocketCoreRoute,
  createShellSocketCoreRouteHandler,
};
