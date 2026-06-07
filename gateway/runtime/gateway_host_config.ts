#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::host-config.
//
// Gateway owns compatibility-host CLI parsing, host/port defaults, and bounded
// shutdown delay normalization. Adapter hosts may consume this parsed config,
// but should not carry their own Gateway host configuration policy.

'use strict';

const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

const DEFAULT_GATEWAY_HOST = '127.0.0.1';
const DEFAULT_GATEWAY_PORT = 4173;
const DEFAULT_GATEWAY_TEAM = 'ops';
const DEFAULT_GATEWAY_REFRESH_MS = 2000;
const DEFAULT_GATEWAY_BACKEND_READY_TIMEOUT_MS = 120000;
const GATEWAY_BACKEND_PORT_OFFSET = 1000;
const GATEWAY_SHUTDOWN_EXIT_DELAY_DEFAULT_MS = 180;
const GATEWAY_SHUTDOWN_EXIT_DELAY_MIN_MS = 80;
const GATEWAY_SHUTDOWN_EXIT_DELAY_MAX_MS = 5000;

function parseGatewayPositiveInt(value, fallback, min = 1, max = 65535) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}

function normalizeGatewayShutdownExitDelayMs(value) {
  const num = Number(value);
  if (!Number.isFinite(num)) return GATEWAY_SHUTDOWN_EXIT_DELAY_DEFAULT_MS;
  if (!Number.isSafeInteger(Math.floor(num))) return GATEWAY_SHUTDOWN_EXIT_DELAY_DEFAULT_MS;
  return Math.max(
    GATEWAY_SHUTDOWN_EXIT_DELAY_MIN_MS,
    Math.min(GATEWAY_SHUTDOWN_EXIT_DELAY_MAX_MS, Math.floor(num)),
  );
}

function normalizeGatewayArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function defaultGatewayApiPort(port) {
  if (port + GATEWAY_BACKEND_PORT_OFFSET <= 65535) return port + GATEWAY_BACKEND_PORT_OFFSET;
  if (port - GATEWAY_BACKEND_PORT_OFFSET >= 1) return port - GATEWAY_BACKEND_PORT_OFFSET;
  return port === 65535 ? 65534 : port + 1;
}

function parseGatewayHostFlags(argv = []) {
  const out = {
    mode: 'serve',
    host: DEFAULT_GATEWAY_HOST,
    port: DEFAULT_GATEWAY_PORT,
    team: DEFAULT_GATEWAY_TEAM,
    refreshMs: DEFAULT_GATEWAY_REFRESH_MS,
    pretty: true,
    apiHost: '',
    apiPort: 0,
    apiReadyTimeoutMs: DEFAULT_GATEWAY_BACKEND_READY_TIMEOUT_MS,
    uiMode: cleanText(process.env.INFRING_DASHBOARD_UI || 'primary', 24).toLowerCase(),
  };
  let modeSet = false;
  for (const token of argv) {
    const value = String(token || '').trim();
    if (!value) continue;
    if (!modeSet && !value.startsWith('--')) {
      out.mode = value.toLowerCase();
      modeSet = true;
      continue;
    }
    if (value.startsWith('--host=')) out.host = cleanText(value.slice(7), 100) || DEFAULT_GATEWAY_HOST;
    else if (value.startsWith('--port=')) out.port = parseGatewayPositiveInt(value.slice(7), DEFAULT_GATEWAY_PORT);
    else if (value.startsWith('--team=')) out.team = cleanText(value.slice(7), 80) || DEFAULT_GATEWAY_TEAM;
    else if (value.startsWith('--refresh-ms=')) out.refreshMs = parseGatewayPositiveInt(value.slice(13), DEFAULT_GATEWAY_REFRESH_MS, 800, 60000);
    else if (value.startsWith('--api-host=')) out.apiHost = cleanText(value.slice(11), 100);
    else if (value.startsWith('--backend-host=')) out.apiHost = cleanText(value.slice(15), 100);
    else if (value.startsWith('--api-port=')) out.apiPort = parseGatewayPositiveInt(value.slice(11), 0);
    else if (value.startsWith('--backend-port=')) out.apiPort = parseGatewayPositiveInt(value.slice(15), 0);
    else if (value.startsWith('--api-ready-timeout-ms=')) out.apiReadyTimeoutMs = parseGatewayPositiveInt(value.slice(23), DEFAULT_GATEWAY_BACKEND_READY_TIMEOUT_MS, 1500, 300000);
    else if (value.startsWith('--ui=')) out.uiMode = cleanText(value.slice(5), 24).toLowerCase();
    else if (value === '--pretty=0' || value === '--pretty=false') out.pretty = false;
  }
  out.uiMode = 'primary';
  out.apiHost = out.apiHost || out.host;
  out.apiPort = out.apiPort || defaultGatewayApiPort(out.port);
  if (out.apiPort === out.port) out.apiPort = defaultGatewayApiPort(out.port + 1);
  return out;
}

module.exports = {
  DEFAULT_GATEWAY_HOST,
  DEFAULT_GATEWAY_PORT,
  DEFAULT_GATEWAY_TEAM,
  DEFAULT_GATEWAY_REFRESH_MS,
  DEFAULT_GATEWAY_BACKEND_READY_TIMEOUT_MS,
  GATEWAY_BACKEND_PORT_OFFSET,
  GATEWAY_SHUTDOWN_EXIT_DELAY_DEFAULT_MS,
  GATEWAY_SHUTDOWN_EXIT_DELAY_MIN_MS,
  GATEWAY_SHUTDOWN_EXIT_DELAY_MAX_MS,
  parseGatewayPositiveInt,
  normalizeGatewayShutdownExitDelayMs,
  normalizeGatewayArgs,
  defaultGatewayApiPort,
  parseGatewayHostFlags,
};
