#!/usr/bin/env tsx
/* eslint-disable no-console */
// source_domain: validation
// owner_domain: gateways/runtime-boundary
// policy_path: validation/conformance/contracts/guard_registry_ownership_policy.json

'use strict';

const childProcess = require('node:child_process');
const fs = require('node:fs');
const net = require('node:net');
const path = require('node:path');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core', 'local', 'artifacts', 'gateway_watchdog_liveness_guard_current.json');

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function listen(server, host) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, host, () => resolve(server.address()));
  });
}

function close(server) {
  return new Promise((resolve) => {
    try {
      server.close(() => resolve());
    } catch {
      resolve();
    }
  });
}

function runStatus(port) {
  const debugBin = path.join(ROOT, 'target', 'debug', process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops');
  const args = [
    'daemon-control',
    'status',
    '--json',
    '--dashboard-host=127.0.0.1',
    `--dashboard-port=${port}`,
    '--gateway-persist=0',
    '--dashboard-open=0',
  ];
  const command = fs.existsSync(debugBin) ? debugBin : 'cargo';
  const finalArgs = fs.existsSync(debugBin)
    ? args
    : ['run', '-q', '-p', 'infring-ops-core', '--bin', 'infring-ops', '--', ...args];
  return childProcess.spawnSync(command, finalArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 20000,
    maxBuffer: 1024 * 1024 * 16,
  });
}

function lastJsonLine(stdout) {
  const lines = String(stdout || '').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    const line = lines[idx];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

async function main() {
  const sockets = new Set();
  const server = net.createServer((socket) => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
    socket.on('error', () => {});
    socket.on('data', () => {
      // Intentionally keep the connection open and emit no HTTP response.
      // This simulates the observed wedge: listener accepts, /healthz stalls.
    });
  });

  const address = await listen(server, '127.0.0.1');
  const port = Number(address && address.port);
  const startedAt = Date.now();
  const run = runStatus(port);
  const elapsedMs = Date.now() - startedAt;
  for (const socket of Array.from(sockets)) {
    try { socket.destroy(); } catch {}
  }
  await close(server);

  const payload = lastJsonLine(run.stdout);
  const dashboard = payload && payload.dashboard && typeof payload.dashboard === 'object'
    ? payload.dashboard
    : {};
  const health = dashboard.health && typeof dashboard.health === 'object'
    ? dashboard.health
    : {};

  const violations = [];
  if (run.status !== 0) violations.push(`status_command_exit_${run.status}`);
  if (!payload) violations.push('status_json_missing');
  if (dashboard.running !== false) violations.push(`dashboard_running_expected_false:${dashboard.running}`);
  if (health.healthy !== false) violations.push(`health_healthy_expected_false:${health.healthy}`);
  if (health.listener_reachable !== true) violations.push(`listener_reachable_expected_true:${health.listener_reachable}`);
  if (health.timed_out !== true) violations.push(`timed_out_expected_true:${health.timed_out}`);
  if (health.wedged !== true) violations.push(`wedged_expected_true:${health.wedged}`);
  if (health.reason !== 'dashboard_healthz_timeout') violations.push(`unexpected_reason:${health.reason || 'missing'}`);

  const report = {
    ok: violations.length === 0,
    type: 'gateway_watchdog_liveness_guard',
    simulated_failure: 'listener_accepts_healthz_but_never_responds',
    port,
    elapsed_ms: elapsedMs,
    status_exit: run.status,
    status_signal: run.signal || null,
    health,
    watchdog_diagnostic_path: dashboard.watchdog && dashboard.watchdog.diagnostic_path || null,
    stderr_preview: String(run.stderr || '').slice(0, 2000),
    violations,
  };
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  ensureDir(OUT_JSON);
  const report = {
    ok: false,
    type: 'gateway_watchdog_liveness_guard',
    error: error && error.stack || String(error),
  };
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`);
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
});
