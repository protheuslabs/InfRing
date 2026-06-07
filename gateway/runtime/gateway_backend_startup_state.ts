#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::backend-startup-state.
//
// Gateway owns dashboard backend startup state/projection shape. Compatibility
// hosts inject concrete health and launch capabilities; this module only
// assembles observable startup state and pending promise lifecycle.

'use strict';

const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

function createGatewayBackendStartupStateController(options = {}) {
  const backendHealth = typeof options.backendHealth === 'function' ? options.backendHealth : null;
  const ensureBackend = typeof options.ensureBackend === 'function' ? options.ensureBackend : null;
  const backendFreshnessSnapshot = typeof options.backendFreshnessSnapshot === 'function'
    ? options.backendFreshnessSnapshot
    : () => null;

  if (!backendHealth) throw new Error('gateway_backend_startup_health_missing');
  if (!ensureBackend) throw new Error('gateway_backend_startup_ensure_missing');

  function applyBackendResult(backend, result) {
    backend.child = result && result.child ? result.child : null;
    backend.reused = !!(result && result.reused);
    backend.ready = true;
    backend.freshness = result && result.freshness ? result.freshness : null;
    backend.startup_error = '';
    return result;
  }

  async function createBackendStartupState(flags) {
    const backend = {
      child: null,
      reused: false,
      ready: await backendHealth(flags, 1500),
      freshness: null,
      startup_error: '',
    };
    let backendStartPromise = null;

    if (!backend.ready) {
      backendStartPromise = ensureBackend(flags)
        .then((result) => applyBackendResult(backend, result))
        .catch((error) => {
          backend.ready = false;
          backend.startup_error = cleanText(error && error.message ? error.message : String(error), 200);
          return null;
        });
    } else {
      try {
        applyBackendResult(backend, await ensureBackend(flags));
      } catch (error) {
        backend.reused = true;
        backend.freshness = backendFreshnessSnapshot(flags);
        backend.startup_error = cleanText(error && error.message ? error.message : String(error), 200);
      }
    }

    return {
      backend,
      backendStartPromise,
    };
  }

  return {
    createBackendStartupState,
  };
}

module.exports = {
  createGatewayBackendStartupStateController,
};
