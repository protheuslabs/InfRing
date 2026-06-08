#!/usr/bin/env tsx

'use strict';

const { createHttpSocketRuntimeEngineAdapter } = require('./http_socket_runtime_adapter.ts');

function createOpenClawEngineAdapter(options = {}) {
  return createHttpSocketRuntimeEngineAdapter({
    ...options,
    engineId: 'openclaw',
    engineKind: 'external_framework_adapter',
    downloadActionRef: 'agent_runtime_download/openclaw',
    contextTransportMode: 'structured_json',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'planned_structured_target',
    healthProbes: [
      { method: 'GET', path: '/health' },
      { method: 'GET', path: '/api/health' },
      { method: 'GET', path: '/status' },
      { method: 'GET', path: '/api/status' },
    ],
  });
}

module.exports = {
  createOpenClawEngineAdapter,
};

