#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::backend-lifecycle.
//
// Gateway owns backend lifecycle wait/stop decisions. Compatibility hosts may
// still spawn the process during migration, but stale-listener shutdown and
// health wait loops are Gateway boundary behavior.

'use strict';

const { gatewayBackendHealth } = require('./gateway_http_boundary.ts');
const { sleepGatewayMs } = require('./gateway_timing.ts');

async function waitForGatewayBackendDown(flags, timeoutMs = 6000, options = {}) {
  const health = typeof options.backendHealth === 'function' ? options.backendHealth : gatewayBackendHealth;
  const sleep = typeof options.sleep === 'function' ? options.sleep : sleepGatewayMs;
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!(await health(flags, 800))) return true;
    await sleep(150);
  }
  return !(await health(flags, 800));
}

async function stopStaleGatewayBackend(flags, freshness, options = {}) {
  const rows = freshness && Array.isArray(freshness.listener_pids) ? freshness.listener_pids : [];
  const pids = rows.map((row) => Number(row && row.pid)).filter((pid) => Number.isInteger(pid) && pid > 0);
  if (!pids.length) return false;
  const killProcess = typeof options.killProcess === 'function' ? options.killProcess : (pid, signal) => process.kill(pid, signal);
  const waitForDown = typeof options.waitForDown === 'function'
    ? options.waitForDown
    : (targetFlags) => waitForGatewayBackendDown(targetFlags, options.timeoutMs || 6000, options);
  for (const pid of pids) {
    try { killProcess(pid, 'SIGTERM'); } catch {}
  }
  if (await waitForDown(flags)) return true;
  for (const pid of pids) {
    try { killProcess(pid, 'SIGKILL'); } catch {}
  }
  return waitForDown(flags);
}

module.exports = {
  waitForGatewayBackendDown,
  stopStaleGatewayBackend,
};
