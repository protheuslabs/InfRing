#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::dashboard-surface-lock.
//
// Gateway owns the compatibility host rule that exactly one primary dashboard
// surface may be served. Adapter hosts may provide filesystem locations and UI
// root detection predicates, but must not carry alternate-dashboard policy.

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const GATEWAY_ALT_DASHBOARD_PATTERN = /(legacy|reference_runtime|control_runtime|deprecated)/i;
const GATEWAY_FORBIDDEN_ALT_DASHBOARD_NAMES = [
  'legacy_dashboard',
  'reference_runtime_dashboard',
  'control_runtime_dashboard',
  'dashboard_legacy',
  'deprecated_dashboard',
];

function createGatewayDashboardSurfaceLock(options = {}) {
  const dashboardDir = path.resolve(String(options.dashboardDir || ''));
  const staticDir = path.resolve(String(options.staticDir || ''));
  const hasPrimaryDashboardUi = typeof options.hasPrimaryDashboardUi === 'function'
    ? options.hasPrimaryDashboardUi
    : () => false;

  function forbiddenAltDashboardDirs() {
    return GATEWAY_FORBIDDEN_ALT_DASHBOARD_NAMES.map((name) => path.resolve(dashboardDir, name));
  }

  function discoverSiblingAltDashboardSurfaces() {
    const out = [];
    let rows = [];
    try { rows = fs.readdirSync(dashboardDir, { withFileTypes: true }); } catch { return out; }
    for (const entry of rows) {
      if (!entry || typeof entry.isDirectory !== 'function' || !entry.isDirectory()) continue;
      const dirPath = path.resolve(dashboardDir, String(entry.name || ''));
      if (!dirPath || dirPath === staticDir) continue;
      const dirName = path.basename(dirPath);
      const hasInlineDashboardRoot = hasPrimaryDashboardUi(dirPath);
      const hasBuildIndex = fs.existsSync(path.resolve(dirPath, 'build', 'index.html'));
      const hasIndexHtml = fs.existsSync(path.resolve(dirPath, 'index.html'));
      if (GATEWAY_ALT_DASHBOARD_PATTERN.test(dirName) || hasInlineDashboardRoot || hasBuildIndex || hasIndexHtml) out.push(dirPath);
    }
    return out;
  }

  function assertNoAlternateDashboardSurfaces() {
    const found = new Set();
    forbiddenAltDashboardDirs().filter((dirPath) => fs.existsSync(dirPath)).forEach((dirPath) => found.add(dirPath));
    discoverSiblingAltDashboardSurfaces().forEach((dirPath) => found.add(dirPath));
    if (found.size === 0) return;
    const labels = Array.from(found).map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
    throw new Error(`forbidden_dashboard_surface_present:${labels}`);
  }

  function assertSingleDashboardRoot() {
    if (!hasPrimaryDashboardUi(staticDir)) throw new Error('primary_dashboard_ui_missing');
    let rows = [];
    try { rows = fs.readdirSync(dashboardDir, { withFileTypes: true }); } catch { return; }
    const duplicateRoots = rows
      .filter((entry) => entry && typeof entry.isDirectory === 'function' && entry.isDirectory())
      .map((entry) => path.resolve(dashboardDir, String(entry.name || '')))
      .filter((dirPath) => dirPath !== staticDir && hasPrimaryDashboardUi(dirPath));
    if (!duplicateRoots.length) return;
    const labels = duplicateRoots.map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
    throw new Error(`multiple_dashboard_roots_detected:${labels}`);
  }

  function assertDashboardSurfaceLocked() {
    assertNoAlternateDashboardSurfaces();
    assertSingleDashboardRoot();
  }

  return {
    discoverSiblingAltDashboardSurfaces,
    assertNoAlternateDashboardSurfaces,
    assertSingleDashboardRoot,
    assertDashboardSurfaceLocked,
  };
}

module.exports = {
  GATEWAY_ALT_DASHBOARD_PATTERN,
  GATEWAY_FORBIDDEN_ALT_DASHBOARD_NAMES,
  createGatewayDashboardSurfaceLock,
};
