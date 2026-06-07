#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::host-lifecycle.
//
// Gateway owns host lifecycle timing/projection shape. Concrete process-exit
// authority is injected by the compatibility host so Gateway policy modules do
// not directly terminate host processes.

'use strict';

const {
  normalizeGatewayShutdownExitDelayMs,
} = require('./gateway_host_config.ts');

function createGatewayHostLifecycleController(options = {}) {
  const normalizeExitDelayMs = typeof options.normalizeExitDelayMs === 'function'
    ? options.normalizeExitDelayMs
    : normalizeGatewayShutdownExitDelayMs;
  const setTimer = typeof options.setTimeout === 'function' ? options.setTimeout : setTimeout;
  const exitProcess = typeof options.exitProcess === 'function' ? options.exitProcess : null;

  function scheduleGatewayHostExit(cleanup, delayMs) {
    if (!exitProcess) throw new Error('gateway_host_exit_process_unavailable');
    const waitMs = normalizeExitDelayMs(delayMs);
    setTimer(() => {
      try {
        if (typeof cleanup === 'function') cleanup();
      } catch {}
      setTimer(() => {
        try { exitProcess(0); } catch {}
      }, 0);
    }, waitMs);
    return {
      ok: true,
      type: 'gateway_host_exit_scheduled',
      wait_ms: waitMs,
    };
  }

  return {
    scheduleGatewayHostExit,
  };
}

module.exports = {
  createGatewayHostLifecycleController,
};
