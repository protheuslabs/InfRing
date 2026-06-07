#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::backend-host-launcher.
//
// Gateway owns compatibility backend launch orchestration. The concrete process
// spawn capability is injected by the host so Gateway policy modules do not
// directly own process authority.

'use strict';

const {
  backendSpawnEnv: backendSpawnEnvForRoot,
  createGatewayBackendFreshnessSnapshot,
  shouldRestartStaleBackend,
} = require('./gateway_backend_freshness.ts');
const { stopStaleGatewayBackend } = require('./gateway_backend_lifecycle.ts');
const { gatewayBackendHealth } = require('./gateway_http_boundary.ts');
const { sleepGatewayMs } = require('./gateway_timing.ts');

function createGatewayBackendHostLauncher(options = {}) {
  const root = options.root || process.cwd();
  const resolveBinary = typeof options.resolveBinary === 'function' ? options.resolveBinary : () => '';
  const spawnProcess = typeof options.spawnProcess === 'function' ? options.spawnProcess : null;
  const envSource = typeof options.env === 'function' ? options.env : () => (options.env || process.env);
  const backendHealth = typeof options.backendHealth === 'function' ? options.backendHealth : gatewayBackendHealth;
  const sleep = typeof options.sleep === 'function' ? options.sleep : sleepGatewayMs;
  const stopStaleBackend = typeof options.stopStaleBackend === 'function' ? options.stopStaleBackend : stopStaleGatewayBackend;
  const restartStaleBackend = typeof options.shouldRestartStaleBackend === 'function'
    ? options.shouldRestartStaleBackend
    : shouldRestartStaleBackend;
  const stdout = options.stdout && typeof options.stdout.write === 'function' ? options.stdout : process.stdout;
  const stderr = options.stderr && typeof options.stderr.write === 'function' ? options.stderr : process.stderr;

  function backendSpawnEnv() {
    return backendSpawnEnvForRoot(root, envSource());
  }

  const backendFreshnessSnapshot = createGatewayBackendFreshnessSnapshot({
    root,
    resolveBinary,
    env: backendSpawnEnv,
  });

  function spawnBackend(flags) {
    if (!spawnProcess) throw new Error('dashboard_backend_spawn_unavailable');
    const laneArgs = [
      'dashboard-ui',
      'serve',
      `--host=${flags.apiHost}`,
      `--port=${flags.apiPort}`,
      `--team=${flags.team}`,
      `--refresh-ms=${flags.refreshMs}`,
    ];
    const env = backendSpawnEnv();
    const bin = resolveBinary({ env });
    if (!bin) throw new Error('dashboard_backend_binary_missing');
    const child = spawnProcess(bin, laneArgs, { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'] });
    if (child.stdout) child.stdout.on('data', (chunk) => stdout.write(chunk));
    if (child.stderr) child.stderr.on('data', (chunk) => stderr.write(chunk));
    return child;
  }

  async function ensureBackend(flags) {
    if (await backendHealth(flags, 1500)) {
      const freshness = backendFreshnessSnapshot(flags);
      if (!freshness.stale) return { child: null, reused: true, freshness };
      if (!restartStaleBackend()) return { child: null, reused: true, freshness };
      const stopped = await stopStaleBackend(flags, freshness);
      if (!stopped) return { child: null, reused: true, freshness: { ...freshness, restart_failed: true } };
    }
    const child = spawnBackend(flags);
    const deadline = Date.now() + flags.apiReadyTimeoutMs;
    while (Date.now() < deadline) {
      if (await backendHealth(flags, 1500)) return { child, reused: false, freshness: backendFreshnessSnapshot(flags) };
      if (child.exitCode != null) throw new Error(`dashboard_backend_exit:${child.exitCode}`);
      await sleep(250);
    }
    try { child.kill('SIGTERM'); } catch {}
    throw new Error('dashboard_backend_timeout');
  }

  return {
    backendSpawnEnv,
    backendFreshnessSnapshot,
    spawnBackend,
    ensureBackend,
  };
}

module.exports = {
  createGatewayBackendHostLauncher,
};
