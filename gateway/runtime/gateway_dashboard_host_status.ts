#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::dashboard-host-status.
//
// Gateway owns dashboard host status projection and persistence shape.
// Compatibility hosts provide live backend/ws state, but must not assemble the
// status payload locally.

'use strict';

const path = require('node:path');
const { writeGatewayJson: writeJson } = require('./gateway_artifacts.ts');
const { gatewayBackendBase } = require('./gateway_http_boundary.ts');
const { gatewayNowIso: nowIso } = require('./gateway_timing.ts');
const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

function createGatewayDashboardHostStatusController(options = {}) {
  const root = options.root || process.cwd();
  const staticDir = options.staticDir || '';
  const statusPath = options.statusPath || path.resolve(
    root,
    'client',
    'runtime',
    'local',
    'state',
    'ui',
    'infring_dashboard',
    'server_status.json',
  );

  function backendStartPending(backend, backendStartPromise) {
    return !!backendStartPromise && !(backend && backend.ready) && !(backend && backend.startup_error);
  }

  function refreshDashboardHostStatus(status, args = {}) {
    const backend = args.backend || {};
    const backendStartPromise = args.backendStartPromise || null;
    status.backend_reused = !!backend.reused;
    status.backend_ready = !!backend.ready;
    status.backend_freshness = backend.freshness || null;
    status.backend_start_pending = backendStartPending(backend, backendStartPromise);
    status.backend_start_error = cleanText(backend.startup_error || '', 200);
    return status;
  }

  function createDashboardHostStatus(args = {}) {
    const flags = args.flags || {};
    const backend = args.backend || {};
    const wsBridge = args.wsBridge || {};
    const status = {
      ok: true,
      type: 'infring_dashboard_server',
      ts: nowIso(),
      url: `http://${flags.host}:${flags.port}/dashboard`,
      host: flags.host,
      port: flags.port,
      refresh_ms: flags.refreshMs,
      team: flags.team,
      authority: 'primary_dashboard_ui_over_rust_core_api',
      dashboard_ui_mode_requested: flags.uiMode,
      dashboard_ui_mode_active: 'primary',
      backend_url: gatewayBackendBase(flags),
      backend_reused: !!backend.reused,
      backend_ready: !!backend.ready,
      backend_freshness: backend.freshness || null,
      backend_start_pending: false,
      backend_start_error: '',
      ws_bridge_enabled: !!wsBridge.ws_enabled,
      ws_bridge_error: cleanText(wsBridge.ws_error || '', 120),
      dashboard_static_dir: path.basename(staticDir),
      status_path: path.relative(root, statusPath),
    };
    return refreshDashboardHostStatus(status, args);
  }

  function persistDashboardHostStatus(status, args = {}) {
    const payload = refreshDashboardHostStatus(status, args);
    writeJson(statusPath, payload);
    return payload;
  }

  return {
    createDashboardHostStatus,
    persistDashboardHostStatus,
    refreshDashboardHostStatus,
  };
}

module.exports = {
  createGatewayDashboardHostStatusController,
};
