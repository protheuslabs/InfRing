#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::system-route-assembly.
//
// Gateway owns system route/action fallback assembly. Compatibility hosts may
// inject bridge/process capabilities and shutdown callbacks, but they must not
// wire system route fallback policy locally.

'use strict';

const {
  createGatewaySystemRouteHandler,
} = require('./gateway_system_routes.ts');
const {
  createGatewayDashboardSystemActionDispatcher,
} = require('./gateway_system_actions.ts');

function createGatewaySystemRouteAssembly(options = {}) {
  const root = options.root || process.cwd();
  const fetchBackendJson = options.fetchBackendJson;
  const fetchBackend = options.fetchBackend;
  const readJsonBody = options.readJsonBody;
  const sendJson = options.sendJson;
  const onHostShutdownAccepted = options.onHostShutdownAccepted;

  const {
    runDashboardSystemAction,
    dispatchDashboardSystemAction,
  } = createGatewayDashboardSystemActionDispatcher({
    root,
    env: options.env,
    invokeInfringOpsViaBridge: options.invokeInfringOpsViaBridge,
    resolveBinary: options.resolveBinary,
    spawnProcess: options.spawnProcess,
  });

  const {
    handleGatewaySystemRoute,
  } = createGatewaySystemRouteHandler({
    fetchBackendJson,
    fetchBackend,
    readJsonBody,
    sendJson,
    legacyHostFallback: (action, body) => {
      if (action === 'update') return runDashboardSystemAction('update', body);
      return dispatchDashboardSystemAction(action, body);
    },
    onHostShutdownAccepted,
  });

  return {
    handleGatewaySystemRoute,
    runDashboardSystemAction,
    dispatchDashboardSystemAction,
  };
}

module.exports = {
  createGatewaySystemRouteAssembly,
};
