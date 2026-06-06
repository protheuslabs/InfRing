#!/usr/bin/env node
/* eslint-disable no-console */

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const gatewaySystemRoutesPath = path.join(ROOT, 'gateway/runtime/gateway_system_routes.ts');
const forbiddenGatewayAuthorityTokens = [
  /\bspawn\s*\(/,
  /\bresolveBinary\s*\(/,
  /\binvokeInfringOpsViaBridge\s*\(/,
  /\brunInfringOps\s*\(/,
  /\bprocess\.exit\s*\(/,
  /\bdispatchDashboardSystemAction\s*\(/,
  /\brunDashboardSystemAction\s*\(/,
];
const defaultOutJson = 'core/local/artifacts/gateway_system_action_authority_guard_current.json';
const defaultOutMarkdown = 'local/workspace/reports/GATEWAY_SYSTEM_ACTION_AUTHORITY_GUARD_CURRENT.md';

function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const index = process.argv.indexOf(`--${name}`);
  if (index >= 0 && index + 1 < process.argv.length) return process.argv[index + 1];
  return fallback;
}

const outJson = flag('out-json', defaultOutJson);
const outMarkdown = flag('out-markdown', defaultOutMarkdown);

type CapturedResponse = {
  status: number;
  payload: any;
};

function writeArtifacts(payload: any): void {
  const artifactPaths: string[] = [];
  if (outJson) {
    const jsonPath = path.join(ROOT, outJson);
    fs.mkdirSync(path.dirname(jsonPath), { recursive: true });
    fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);
    artifactPaths.push(outJson);
  }
  if (outMarkdown) {
    const markdownPath = path.join(ROOT, outMarkdown);
    fs.mkdirSync(path.dirname(markdownPath), { recursive: true });
    const failures = Array.isArray(payload.failures) ? payload.failures : [];
    const lines = [
      '# Gateway System Action Authority Guard',
      '',
      `Generated: ${payload.generated_at || ''}`,
      `Status: ${payload.ok ? 'PASS' : 'FAIL'}`,
      '',
      '## Summary',
      '',
      `- Checked actions: ${(payload.checked_actions || []).join(', ')}`,
      `- Core forwarding: ${payload.core_forwarding === true}`,
      `- Legacy fallback labeled: ${payload.legacy_fallback_labeled === true}`,
      `- Gateway OS authority forbidden: ${payload.gateway_os_authority_forbidden === true}`,
      '',
      '## Failures',
      '',
      failures.length ? failures.map((row: any) => `- ${row.id}: ${row.detail}`).join('\n') : '- None',
      '',
    ];
    fs.writeFileSync(markdownPath, `${lines.join('\n')}\n`);
    artifactPaths.push(outMarkdown);
  }
  payload.artifact_paths = artifactPaths;
}

function fail(id: string, detail: string): never {
  const payload = {
    ok: false,
    type: 'gateway_system_action_authority_guard',
    generated_at: new Date().toISOString(),
    failures: [{ id, detail }],
    checked_actions: ['restart', 'update', 'shutdown'],
    core_forwarding: false,
    legacy_fallback_labeled: false,
    gateway_os_authority_forbidden: false,
  };
  writeArtifacts(payload);
  console.log(JSON.stringify(payload, null, 2));
  process.exit(1);
}

function assert(condition: unknown, id: string, detail: string): void {
  if (!condition) fail(id, detail);
}

async function main(): Promise<void> {
  assert(fs.existsSync(gatewaySystemRoutesPath), 'gateway_system_routes_missing', 'gateway/runtime/gateway_system_routes.ts must exist.');
  const gatewaySystemRoutesSource = fs.readFileSync(gatewaySystemRoutesPath, 'utf8');
  const forbidden = forbiddenGatewayAuthorityTokens
    .map((pattern) => String(pattern))
    .filter((pattern, index) => forbiddenGatewayAuthorityTokens[index].test(gatewaySystemRoutesSource));
  assert(forbidden.length === 0, 'gateway_system_routes_own_os_authority', `Forbidden OS-authority token(s) found in Gateway system routes: ${forbidden.join(', ')}`);

  const {
    isGatewaySystemRoute,
    createGatewaySystemRouteHandler,
  } = require(gatewaySystemRoutesPath);

  for (const route of ['/api/system/restart', '/api/system/update', '/api/system/shutdown']) {
    assert(isGatewaySystemRoute(route), 'system_action_route_not_gateway_wrapped', `${route} must be recognized by Gateway system route wrapper.`);
  }

  for (const action of ['restart', 'update', 'shutdown']) {
    let captured: CapturedResponse | null = null;
    let cleanupCalls = 0;
    let fallbackCalls = 0;
    const handler = createGatewaySystemRouteHandler({
      fetchBackendJson: async () => ({ ok: true }),
      fetchBackend: async (_flags: any, route: string) => ({
        ok: true,
        status: 200,
        text: async () => JSON.stringify({
          ok: true,
          route,
          receipt_ref: `receipt/mock-${action}`,
        }),
      }),
      readJsonBody: async () => ({ apply: false, exit_delay_ms: 80 }),
      sendJson: (_res: any, status: number, payload: any) => {
        captured = { status, payload };
      },
      legacyHostFallback: () => {
        fallbackCalls += 1;
        return { ok: true };
      },
      onHostShutdownAccepted: () => {
        cleanupCalls += 1;
      },
    });
    const handled = await handler.handleGatewaySystemRoute({
      req: { method: 'POST' },
      res: {},
      pathname: `/api/system/${action}`,
      traceId: `guard-${action}`,
      flags: {},
    });
    assert(handled, 'system_action_not_handled', `${action} was not handled by Gateway route wrapper.`);
    assert(captured && captured.status === 200, 'system_action_bad_status', `${action} did not return 200 in mock Core-forward path.`);
    assert(captured?.payload?.gateway_projection?.authority_owner === 'core.ops', 'system_action_not_core_authoritative', `${action} did not project authority_owner=core.ops.`);
    assert(captured?.payload?.gateway_projection?.forwarded_to_core === true, 'system_action_not_forwarded_to_core', `${action} did not project forwarded_to_core=true.`);
    assert(fallbackCalls === 0, 'system_action_used_legacy_fallback_despite_core', `${action} called legacy fallback even though Core route was available.`);
    assert(action === 'shutdown' ? cleanupCalls === 1 : cleanupCalls === 0, 'system_action_host_cleanup_mismatch', `${action} host cleanup callback count was ${cleanupCalls}.`);
  }

  for (const action of ['restart', 'update', 'shutdown']) {
    let captured: CapturedResponse | null = null;
    let fallbackCalls = 0;
    const handler = createGatewaySystemRouteHandler({
      fetchBackendJson: async () => ({ ok: true }),
      fetchBackend: async () => {
        throw new Error('mock_core_unavailable');
      },
      readJsonBody: async () => ({ apply: false, exit_delay_ms: 80 }),
      sendJson: (_res: any, status: number, payload: any) => {
        captured = { status, payload };
      },
      legacyHostFallback: (fallbackAction: string) => {
        fallbackCalls += 1;
        return {
          ok: true,
          action: fallbackAction,
          receipt_ref: `receipt/legacy-host-${fallbackAction}`,
        };
      },
      onHostShutdownAccepted: () => {},
    });
    const handled = await handler.handleGatewaySystemRoute({
      req: { method: 'POST' },
      res: {},
      pathname: `/api/system/${action}`,
      traceId: `guard-fallback-${action}`,
      flags: {},
    });
    assert(handled, 'system_action_fallback_not_handled', `${action} fallback was not handled.`);
    assert(fallbackCalls === 1, 'system_action_fallback_not_called_once', `${action} fallback call count was ${fallbackCalls}.`);
    assert(captured && captured.status === 200, 'system_action_fallback_bad_status', `${action} fallback did not return 200.`);
    assert(captured?.payload?.gateway_projection?.authority_owner === 'legacy_dashboard_host_shim', 'system_action_fallback_not_labeled_legacy', `${action} fallback was not labeled legacy_dashboard_host_shim.`);
    assert(captured?.payload?.gateway_projection?.target_authority_owner === 'core.ops', 'system_action_fallback_missing_core_target', `${action} fallback did not preserve target_authority_owner=core.ops.`);
    assert(captured?.payload?.gateway_projection?.legacy_host_fallback === true, 'system_action_fallback_flag_missing', `${action} fallback did not project legacy_host_fallback=true.`);
  }

  const payload = {
    ok: true,
    type: 'gateway_system_action_authority_guard',
    generated_at: new Date().toISOString(),
    checked_actions: ['restart', 'update', 'shutdown'],
    core_forwarding: true,
    legacy_fallback_labeled: true,
    gateway_os_authority_forbidden: true,
    failures: [],
  };
  writeArtifacts(payload);
  console.log(JSON.stringify(payload, null, 2));
}

main().catch((error) => {
  fail('unexpected_error', error && error.stack ? error.stack : String(error));
});
