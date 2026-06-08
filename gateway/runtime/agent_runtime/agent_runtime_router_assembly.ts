#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime-router-assembly.
//
// Gateway owns Agent Runtime router assembly. Provider-specific adapters remain
// injected translators; this module maps those injected factories into the
// canonical Gateway router without importing adapter implementations.

'use strict';

const { createAgentRuntimeRouter } = require('./agent_runtime_router.ts');

const DEFAULT_ENGINE_ORDER = Object.freeze([
  'infring_native',
  'codex_cli',
  'claude_code',
  'grok_code',
  'opencode',
  'openclaw',
  'hermes_agent',
]);

function createGatewayAgentRuntimeRouterAssembly(options = {}) {
  const root = options.root || process.cwd();
  const adapterFactories = options.adapterFactories && typeof options.adapterFactories === 'object'
    ? options.adapterFactories
    : {};
  const engineOrder = Array.isArray(options.engineOrder) && options.engineOrder.length
    ? options.engineOrder.map((engineId) => String(engineId || '').trim()).filter(Boolean)
    : DEFAULT_ENGINE_ORDER.slice();
  const normalizeWorkspacePath = typeof options.normalizeWorkspacePath === 'function'
    ? options.normalizeWorkspacePath
    : (value) => value || root;
  const createRouterBase = typeof options.createRouter === 'function'
    ? options.createRouter
    : createAgentRuntimeRouter;

  function createAdapterMap(turnOptions = {}) {
    const liveDispatch = turnOptions.liveDispatch === true;
    const cwd = normalizeWorkspacePath(turnOptions.cwd || root);
    const adapters = {};
    for (const engineId of engineOrder) {
      const factory = adapterFactories[engineId];
      if (typeof factory !== 'function') continue;
      if (engineId === 'infring_native') {
        adapters[engineId] = factory({
          liveDispatch,
          cwd,
          nativeOrchestrationClient: turnOptions.nativeOrchestrationClient,
          orchestrationClient: turnOptions.nativeOrchestrationClient || turnOptions.orchestrationClient,
        });
      } else {
        adapters[engineId] = factory({
          liveDispatch,
          cwd,
        });
      }
    }
    return adapters;
  }

  function createRouter(turnOptions = {}) {
    const router = createRouterBase({
      root,
      disableTraceWriter: turnOptions.disableTraceWriter === true,
      registryInfo: turnOptions.registryInfo,
      registryPath: turnOptions.registryPath,
      tracePath: turnOptions.tracePath,
      traceWriter: turnOptions.traceWriter,
    });
    const adapters = createAdapterMap(turnOptions);
    for (const [engineId, adapter] of Object.entries(adapters)) {
      router.registerAdapter(engineId, adapter);
    }
    return router;
  }

  return {
    createAdapterMap,
    createRouter,
  };
}

module.exports = {
  createGatewayAgentRuntimeRouterAssembly,
};
