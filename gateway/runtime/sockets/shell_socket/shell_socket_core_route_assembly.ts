#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::sockets::shell-socket::core-route-assembly.
//
// Gateway owns Shell Socket core route assembly. Adapter hosts may provide HTTP
// body/response/backend I/O, but they must not construct status, command, eval,
// or lifecycle route policy directly.

'use strict';

const {
  createShellSocketCoreRouteHandler,
} = require('./shell_socket_core_routes.ts');
const {
  gatewayStatusPayloadWithBootStage,
} = require('../../gateway_status_projection.ts');

function createGatewayShellSocketCoreRouteAssembly(options = {}) {
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  const fetchBackend = options.fetchBackend;
  const fetchBackendJson = options.fetchBackendJson;
  const statusPayloadWithBootStage = typeof options.statusPayloadWithBootStage === 'function'
    ? options.statusPayloadWithBootStage
    : gatewayStatusPayloadWithBootStage;

  return createShellSocketCoreRouteHandler({
    readJsonBody,
    sendJson,
    fetchBackend,
    fetchBackendJson,
    statusPayloadWithBootStage,
  });
}

module.exports = {
  createGatewayShellSocketCoreRouteAssembly,
};
