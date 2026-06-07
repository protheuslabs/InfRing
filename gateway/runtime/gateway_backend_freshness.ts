#!/usr/bin/env tsx
// Layer ownership: gateway/runtime::backend-freshness.
//
// Gateway owns backend freshness detection for compatibility hosts.

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const BACKEND_FRESHNESS_SKEW_MS = 1500;

function cleanText(value, maxLen = 200) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function statMtimeMs(filePath) {
  try {
    const stat = fs.statSync(filePath);
    return Number.isFinite(stat.mtimeMs) ? stat.mtimeMs : 0;
  } catch {
    return 0;
  }
}

function backendSpawnEnv(root, env = process.env) {
  return {
    ...env,
    INFRING_ROOT: root,
    INFRING_OPS_ALLOW_STALE: env.INFRING_OPS_ALLOW_STALE || '1',
    INFRING_NPM_ALLOW_STALE: env.INFRING_NPM_ALLOW_STALE || '1',
  };
}

function normalizePidRows(stdout) {
  return Array.from(new Set(
    String(stdout || '')
      .split(/\r?\n/)
      .map((line) => Number(line.trim()))
      .filter((pid) => Number.isInteger(pid) && pid > 0)
  ));
}

function backendListenerPids(flags, options = {}) {
  const root = options.root || process.cwd();
  const port = Number(flags && flags.apiPort);
  if (!Number.isInteger(port) || port < 1 || port > 65535) return [];
  if (process.platform === 'win32') return [];
  try {
    const proc = spawnSync('lsof', ['-nP', '-t', `-iTCP:${port}`, '-sTCP:LISTEN'], {
      cwd: root,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 2500,
    });
    return normalizePidRows(proc && proc.stdout);
  } catch {
    return [];
  }
}

function backendProcessStartMs(pid, options = {}) {
  const root = options.root || process.cwd();
  if (!Number.isInteger(Number(pid)) || Number(pid) <= 0 || process.platform === 'win32') return 0;
  try {
    const proc = spawnSync('ps', ['-p', String(pid), '-o', 'lstart='], {
      cwd: root,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 2500,
    });
    const parsed = Date.parse(cleanText(proc && proc.stdout ? proc.stdout : '', 80));
    return Number.isFinite(parsed) ? parsed : 0;
  } catch {
    return 0;
  }
}

function shouldRestartStaleBackend(env = process.env) {
  const raw = String((env && env.INFRING_DASHBOARD_RESTART_STALE_BACKEND) || '').trim().toLowerCase();
  return raw !== '0' && raw !== 'false' && raw !== 'no' && raw !== 'off';
}

function backendFreshnessSnapshot(flags, options = {}) {
  const root = options.root || process.cwd();
  const resolveBinary = options.resolveBinary;
  const env = options.env || backendSpawnEnv(root);
  const bin = typeof resolveBinary === 'function' ? resolveBinary({ env }) : '';
  const binaryMtimeMs = bin ? statMtimeMs(bin) : 0;
  const pids = backendListenerPids(flags, { root }).map((pid) => {
    const startedAtMs = backendProcessStartMs(pid, { root });
    const stale = !!(binaryMtimeMs && startedAtMs && startedAtMs + BACKEND_FRESHNESS_SKEW_MS < binaryMtimeMs);
    return {
      pid,
      started_at_ms: startedAtMs,
      stale,
    };
  });
  const checked = !!(bin && binaryMtimeMs && pids.length && pids.every((row) => row.started_at_ms));
  const stale = checked && pids.some((row) => row.stale);
  return {
    checked,
    stale,
    reason: stale ? 'backend_process_older_than_resolved_binary' : (checked ? 'backend_process_fresh_enough' : 'backend_freshness_unavailable'),
    binary_path: bin ? path.relative(root, bin) : '',
    binary_mtime_ms: Math.round(binaryMtimeMs || 0),
    listener_pids: pids,
    restart_enabled: shouldRestartStaleBackend(env),
  };
}

function createGatewayBackendFreshnessSnapshot(options = {}) {
  return (flags) => backendFreshnessSnapshot(flags, {
    ...options,
    env: typeof options.env === 'function' ? options.env() : options.env,
  });
}

module.exports = {
  backendFreshnessSnapshot,
  createGatewayBackendFreshnessSnapshot,
  backendListenerPids,
  backendProcessStartMs,
  backendSpawnEnv,
  shouldRestartStaleBackend,
};
