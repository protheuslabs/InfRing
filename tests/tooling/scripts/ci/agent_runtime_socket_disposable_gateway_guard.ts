#!/usr/bin/env node
/* eslint-disable no-console */

'use strict';

// Starts a disposable Gateway dashboard host on a temporary local port, proves
// /ws/agent-runtime through the live WebSocket client guard, then tears the host
// down and restores the operator's dashboard status file. This makes live socket
// proof suitable for release lanes without depending on an already-running UI.

const fs = require('node:fs');
const http = require('node:http');
const net = require('node:net');
const path = require('node:path');
const { spawn } = require('node:child_process');

const ROOT = process.cwd();
const OUT_JSON = path.join(ROOT, 'core/local/artifacts/agent_runtime_socket_disposable_gateway_guard_current.json');
const STATUS_PATH = path.join(ROOT, 'client/runtime/local/state/ui/infring_dashboard/server_status.json');
const HOST = '127.0.0.1';
const START_TIMEOUT_MS = Number(process.env.INFRING_AGENT_RUNTIME_DISPOSABLE_START_TIMEOUT_MS || 20000);
const CHILD_TIMEOUT_MS = Number(process.env.INFRING_AGENT_RUNTIME_DISPOSABLE_CHILD_TIMEOUT_MS || 45000);

function clean(value, max = 4000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function writeReport(report) {
  ensureDir(OUT_JSON);
  fs.writeFileSync(OUT_JSON, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  const out = JSON.stringify(report, null, 2);
  if (report.ok) console.log(out);
  else console.error(out);
}

function snapshotStatusFile() {
  try {
    return { existed: true, content: fs.readFileSync(STATUS_PATH, 'utf8') };
  } catch {
    return { existed: false, content: '' };
  }
}

function restoreStatusFile(snapshot) {
  try {
    if (snapshot && snapshot.existed) {
      fs.mkdirSync(path.dirname(STATUS_PATH), { recursive: true });
      fs.writeFileSync(STATUS_PATH, snapshot.content || '', 'utf8');
    } else {
      fs.rmSync(STATUS_PATH, { force: true });
    }
    return true;
  } catch {
    return false;
  }
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once('error', reject);
    server.listen(0, HOST, () => {
      const port = server.address().port;
      server.close(() => resolve(port));
    });
  });
}

function waitForHealth(port) {
  const deadline = Date.now() + START_TIMEOUT_MS;
  return new Promise((resolve, reject) => {
    const attempt = () => {
      const req = http.request({ host: HOST, port, path: '/healthz', method: 'GET', timeout: 1000 }, (res) => {
        res.resume();
        if (res.statusCode >= 200 && res.statusCode < 500) {
          resolve({ status_code: res.statusCode });
          return;
        }
        retry();
      });
      req.on('timeout', () => {
        req.destroy(new Error('health_timeout'));
      });
      req.on('error', retry);
      req.end();
    };
    const retry = () => {
      if (Date.now() > deadline) {
        reject(new Error(`disposable_gateway_health_timeout:${port}`));
        return;
      }
      setTimeout(attempt, 250);
    };
    attempt();
  });
}

function stopChild(child) {
  return new Promise((resolve) => {
    if (!child || child.exitCode != null || child.signalCode != null) {
      resolve(null);
      return;
    }
    const timer = setTimeout(() => {
      try { child.kill('SIGKILL'); } catch {}
    }, 3000);
    child.once('exit', () => {
      clearTimeout(timer);
      resolve(null);
    });
    try { child.kill('SIGTERM'); } catch { resolve(null); }
  });
}

function runLiveGuard(port) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [
      'client/runtime/lib/ts_entrypoint.ts',
      'tests/tooling/scripts/ci/agent_runtime_socket_live_gateway_guard.ts',
    ], {
      cwd: ROOT,
      env: {
        ...process.env,
        INFRING_GATEWAY_HOST: HOST,
        INFRING_GATEWAY_PORT: String(port),
        INFRING_AGENT_RUNTIME_LIVE_GATEWAY_TIMEOUT_MS: process.env.INFRING_AGENT_RUNTIME_LIVE_GATEWAY_TIMEOUT_MS || '30000',
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    const timer = setTimeout(() => {
      try { child.kill('SIGKILL'); } catch {}
    }, CHILD_TIMEOUT_MS);
    child.stdout.on('data', (chunk) => { stdout += String(chunk || ''); });
    child.stderr.on('data', (chunk) => { stderr += String(chunk || ''); });
    child.on('exit', (status, signal) => {
      clearTimeout(timer);
      let parsed = null;
      const raw = stdout.trim() || stderr.trim();
      try { parsed = JSON.parse(raw.slice(raw.indexOf('{'))); } catch {}
      resolve({ status, signal, ok: status === 0 && parsed && parsed.ok === true, stdout: clean(stdout, 6000), stderr: clean(stderr, 6000), parsed });
    });
  });
}

async function main() {
  const statusSnapshot = snapshotStatusFile();
  const port = await freePort();
  const child = spawn(process.execPath, [
    'client/runtime/lib/ts_entrypoint.ts',
    'client/runtime/systems/ui/infring_dashboard.ts',
    'serve',
    `--host=${HOST}`,
    `--port=${port}`,
    '--team=ops',
    '--refresh-ms=2000',
  ], {
    cwd: ROOT,
    env: {
      ...process.env,
      INFRING_AGENT_RUNTIME_DISPOSABLE_GATEWAY: '1',
      INFRING_OPS_LOCAL_TIMEOUT_MS: process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '30000',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let hostStdout = '';
  let hostStderr = '';
  child.stdout.on('data', (chunk) => { hostStdout += String(chunk || ''); });
  child.stderr.on('data', (chunk) => { hostStderr += String(chunk || ''); });
  const startedAt = Date.now();
  let health = null;
  let liveGuard = null;
  let restoredStatus = false;
  let cleanupError = null;
  try {
    health = await waitForHealth(port);
    liveGuard = await runLiveGuard(port);
  } catch (error) {
    liveGuard = liveGuard || { ok: false, error: clean(error && error.stack ? error.stack : error, 6000) };
  } finally {
    await stopChild(child);
    try { restoredStatus = restoreStatusFile(statusSnapshot); } catch (error) { cleanupError = clean(error && error.stack ? error.stack : error, 1000); }
  }
  const violations = [];
  if (!health) violations.push({ kind: 'disposable_gateway_health_missing' });
  if (!liveGuard || liveGuard.ok !== true) violations.push({ kind: 'live_socket_guard_failed' });
  if (!restoredStatus) violations.push({ kind: 'dashboard_status_restore_failed' });
  if (cleanupError) violations.push({ kind: 'cleanup_error', error: cleanupError });
  const report = {
    ok: violations.length === 0,
    type: 'agent_runtime_socket_disposable_gateway_guard',
    generated_at: new Date().toISOString(),
    mode: 'disposable_gateway_host_live_websocket_client',
    canonical_socket_route: '/ws/agent-runtime',
    target: { host: HOST, port, source: 'disposable_gateway_host' },
    duration_ms: Date.now() - startedAt,
    health,
    live_guard: liveGuard && liveGuard.parsed ? {
      ok: liveGuard.parsed.ok,
      event_types: liveGuard.parsed.event_types || [],
      engine_count: liveGuard.parsed.engine_count || 0,
      target: liveGuard.parsed.target || null,
      violations: liveGuard.parsed.violations || [],
    } : liveGuard,
    host_output_preview: {
      stdout: clean(hostStdout, 1200),
      stderr: clean(hostStderr, 1200),
      exit_code: child.exitCode,
      signal: child.signalCode,
    },
    restored_status_file: restoredStatus,
    violations,
  };
  writeReport(report);
  if (!report.ok) process.exit(1);
}

main().catch((error) => {
  const report = {
    ok: false,
    type: 'agent_runtime_socket_disposable_gateway_guard',
    error: clean(error && error.stack ? error.stack : error, 6000),
  };
  writeReport(report);
  process.exit(1);
});
