#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::system-actions.
//
// Gateway owns bounded system action argument construction, environment
// defaults, and result projection. Compatibility hosts may provide the concrete
// process/bridge capabilities while migration continues.

'use strict';

const path = require('node:path');
const { cleanGatewayText: cleanText } = require('./gateway_text_boundary.ts');

function parseGatewayLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function isDashboardDaemonExecutable(executablePath) {
  const name = path.basename(cleanText(executablePath, 260)).toLowerCase();
  return name.includes('infringd');
}

function dashboardSystemActionArgs(action, payload = {}, executablePath = '') {
  const normalized = cleanText(action, 40).toLowerCase();
  const body = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  if (normalized === 'restart') {
    if (isDashboardDaemonExecutable(executablePath)) return ['restart', '--json'];
    return ['daemon-control', 'heal', '--json', '--dashboard-open=0'];
  }
  if (normalized === 'shutdown') {
    if (isDashboardDaemonExecutable(executablePath)) return ['stop', '--json'];
    return ['daemon-control', 'stop', '--json'];
  }
  if (normalized === 'update') {
    const args = ['update', '--json'];
    if (body.force === true) args.push('--force');
    if (body.apply !== false) args.push('--apply');
    return args;
  }
  throw new Error(`unknown_dashboard_system_action:${normalized}`);
}

function dashboardSystemActionEnv(root, env = process.env) {
  return {
    ...env,
    INFRING_ROOT: root,
    INFRING_OPS_ALLOW_STALE: env.INFRING_OPS_ALLOW_STALE || '1',
    INFRING_NPM_ALLOW_STALE: env.INFRING_NPM_ALLOW_STALE || '1',
  };
}

function createGatewayDashboardSystemActionDispatcher(options = {}) {
  const root = options.root || process.cwd();
  const envSource = typeof options.env === 'function' ? options.env : () => (options.env || process.env);
  const invokeBridge = typeof options.invokeInfringOpsViaBridge === 'function' ? options.invokeInfringOpsViaBridge : null;
  const resolveBinary = typeof options.resolveBinary === 'function' ? options.resolveBinary : null;
  const spawnProcess = typeof options.spawnProcess === 'function' ? options.spawnProcess : null;

  function actionEnv() {
    return dashboardSystemActionEnv(root, envSource());
  }

  function runDashboardSystemAction(action, payload = {}) {
    const args = dashboardSystemActionArgs(action, payload, 'infring-ops');
    const run = invokeBridge
      ? invokeBridge(args, {
          allowProcessFallback: false,
          unknownDomainFallback: false,
        })
      : null;
    const projectedRun = run || {
      status: 1,
      stdout: '',
      stderr: 'resident_ipc_bridge_unavailable',
      payload: null,
    };
    const status = Number.isFinite(Number(projectedRun.status)) ? Number(projectedRun.status) : 1;
    const receipt = (projectedRun && projectedRun.payload && typeof projectedRun.payload === 'object')
      ? projectedRun.payload
      : parseGatewayLastJson(projectedRun.stdout);
    const ok = status === 0 && (!receipt || receipt.ok !== false);
    const error = ok
      ? ''
      : cleanText(
          (receipt && receipt.error) || projectedRun.stderr || projectedRun.stdout || `${cleanText(action, 40).toLowerCase()}_failed`,
          260,
        );
    return {
      ok,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: args[0],
      args: args.slice(1),
      exit_code: status,
      payload: receipt || null,
      error,
    };
  }

  function dispatchDashboardSystemAction(action, payload = {}) {
    const env = actionEnv();
    const bin = resolveBinary ? resolveBinary({ env }) : '';
    const args = dashboardSystemActionArgs(action, payload, bin || 'infring-ops');
    if (!bin) {
      return {
        ok: false,
        type: 'dashboard_system_action',
        action: cleanText(action, 40).toLowerCase(),
        command: '',
        args: args.slice(1),
        error: 'dashboard_backend_binary_missing',
      };
    }
    if (!spawnProcess) {
      return {
        ok: false,
        type: 'dashboard_system_action',
        action: cleanText(action, 40).toLowerCase(),
        command: path.basename(String(bin || '')),
        args: args.slice(1),
        error: 'dashboard_system_action_spawn_unavailable',
      };
    }
    try {
      const child = spawnProcess(bin, args, {
        cwd: root,
        env,
        detached: true,
        stdio: 'ignore',
      });
      if (child && typeof child.unref === 'function') child.unref();
      const receiptRef = `receipt/gateway-system-action-dispatch/${cleanText(action, 40).toLowerCase()}/${Number(child && child.pid) || 0}`;
      return {
        ok: true,
        type: 'dashboard_system_action',
        action: cleanText(action, 40).toLowerCase(),
        command: path.basename(bin),
        args: args.slice(1),
        dispatch_mode: 'detached_subprocess',
        pid: Number(child && child.pid) || 0,
        accepted_async: true,
        receipt_ref: receiptRef,
        receipt: {
          id: receiptRef,
          ref: receiptRef,
          receipt_ref: receiptRef,
          kind: 'gateway_system_action_dispatch',
        },
        payload: null,
        error: '',
      };
    } catch (error) {
      return {
        ok: false,
        type: 'dashboard_system_action',
        action: cleanText(action, 40).toLowerCase(),
        command: path.basename(String(bin || '')),
        args: args.slice(1),
        error: cleanText(error && error.message ? error.message : String(error), 260),
      };
    }
  }

  return {
    dashboardSystemActionArgs,
    dashboardSystemActionEnv: actionEnv,
    runDashboardSystemAction,
    dispatchDashboardSystemAction,
  };
}

module.exports = {
  parseGatewayLastJson,
  dashboardSystemActionArgs,
  dashboardSystemActionEnv,
  createGatewayDashboardSystemActionDispatcher,
};
