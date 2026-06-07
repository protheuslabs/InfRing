#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::status-projection.
//
// Gateway owns bounded status/version projections exposed to Shell Socket and
// dashboard compatibility hosts.

'use strict';

const { gatewayBackendHealth, fetchGatewayBackendJson } = require('./gateway_http_boundary.ts');
const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

async function gatewayStatusPayloadWithBootStage(flags) {
  const startedAt = Date.now();
  const healthOk = await gatewayBackendHealth(flags, 1200);
  if (!healthOk) {
    return {
      ok: false,
      error: 'backend_unreachable',
      connected: false,
      connection_state: 'disconnected',
      boot_stage: 'backend_unreachable',
      backend_health_ok: false,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
  try {
    const status = await fetchGatewayBackendJson(flags, '/api/status', 1800);
    const base = (status && typeof status === 'object') ? status : {};
    const connected = base.connected !== false;
    const degraded = !!base.degraded || base.ok === false;
    const out = {
      ...base,
      ok: connected,
      connected,
      degraded,
      connection_state: connected ? 'connected' : 'disconnected',
      boot_stage: cleanText(base.boot_stage || base.last_stage || (degraded ? 'status_degraded' : 'ready'), 60),
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
    };
    if (!out.error && degraded) out.error = 'status_degraded';
    return out;
  } catch {
    return {
      ok: true,
      degraded: true,
      warning: 'status_unavailable',
      connected: true,
      connection_state: 'connected',
      boot_stage: 'backend_ready_status_probe_timeout',
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
}

function createGatewayDashboardVersionProjection(options = {}) {
  const staticDir = options.staticDir || '';
  const readBuildVersionInfo = typeof options.readBuildVersionInfo === 'function'
    ? options.readBuildVersionInfo
    : () => ({});
  const platform = cleanText(options.platform || process.platform, 80) || process.platform;
  const arch = cleanText(options.arch || process.arch, 80) || process.arch;
  function currentDashboardBuildInfo() {
    return readBuildVersionInfo(staticDir);
  }
  function mergeDashboardVersionPayload(payload) {
    const base = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
    const build = currentDashboardBuildInfo();
    const version = cleanText(build && build.version, 120) || '0.0.0';
    const tag = cleanText(build && build.tag, 120) || `v${version}`;
    const source = cleanText(build && build.source, 80) || 'fallback_default';
    return {
      ...base,
      ok: base.ok !== false,
      version,
      tag,
      version_tag: tag,
      source,
      version_source: source,
      platform: base.platform || platform,
      arch: base.arch || arch,
    };
  }
  return {
    currentDashboardBuildInfo,
    mergeDashboardVersionPayload,
  };
}

module.exports = {
  gatewayStatusPayloadWithBootStage,
  createGatewayDashboardVersionProjection,
};
