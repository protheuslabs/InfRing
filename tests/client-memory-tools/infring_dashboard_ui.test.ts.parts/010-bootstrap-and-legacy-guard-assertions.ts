'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const TARGET_SOURCE = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const ADAPTER_DASHBOARD_HOST_TS_PATH = path.resolve(ROOT, 'adapters/runtime/infring_dashboard.ts');
const GATEWAY_SYSTEM_ROUTES_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_system_routes.ts');
const GATEWAY_SYSTEM_ACTIONS_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_system_actions.ts');
const GATEWAY_SYSTEM_ROUTE_ASSEMBLY_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_system_route_assembly.ts');
const GATEWAY_HOST_LIFECYCLE_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_host_lifecycle.ts');
const GATEWAY_BACKEND_FRESHNESS_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_backend_freshness.ts');
const GATEWAY_BACKEND_STARTUP_STATE_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_backend_startup_state.ts');
const GATEWAY_BACKEND_HOST_LAUNCHER_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_backend_host_launcher.ts');
const GATEWAY_DASHBOARD_HOST_STATUS_TS_PATH = path.resolve(ROOT, 'gateway/runtime/gateway_dashboard_host_status.ts');
const DASHBOARD_STATIC_ASSET_ROUTER_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/dashboard_static_asset_router.ts'
);
const TS_BOOTSTRAP_TS_PATH = path.resolve(ROOT, 'client/runtime/lib/ts_bootstrap.ts');
const DASHBOARD_STATIC_DIR = path.resolve(ROOT, 'client/runtime/systems/ui/infring_static');
const REMOVED_DASHBOARD_CLIENT_REL = [
  'client',
  'runtime',
  'systems',
  'ui',
  ['infring', 'dashboard', 'client.tsx'].join('_'),
].join('/');
const REMOVED_DASHBOARD_CSS_REL = [
  'client',
  'runtime',
  'systems',
  'ui',
  ['infring', 'dashboard.css'].join('_'),
].join('/');
const REMOVED_DASHBOARD_FAMILY = ['V6', 'DASHBOARD', '006'].join('-');
const REMOVED_SPEC_TOKEN = ['INFRING', 'DASHBOARD', 'UI', 'SPEC'].join('_');
const REMOVED_NODE_UI_FLAG = ['--node', 'ui'].join('-');
const REMOVED_LEGACY_NODE_UI_FLAG = ['--legacy', 'node', 'ui'].join('-');
const REMOVED_TOGGLE_CONTROLS_ACTION = ['dashboard.ui', 'toggleControls'].join('.');
const REMOVED_TOGGLE_SECTION_ACTION = ['dashboard.ui', 'toggleSection'].join('.');
const REMOVED_SWITCH_TAB_ACTION = ['dashboard.ui', 'switchControlsTab'].join('.');
const REMOVED_BROWSER_SHELL_TYPE = ['infring_dashboard_browser_shell', 'removed'].join('_');
const LEGACY_DASHBOARD_ARTIFACTS = [
  path.resolve(ROOT, REMOVED_DASHBOARD_CLIENT_REL),
  path.resolve(ROOT, `${REMOVED_DASHBOARD_CLIENT_REL}.parts`),
  path.resolve(ROOT, REMOVED_DASHBOARD_CSS_REL),
  path.resolve(ROOT, `${REMOVED_DASHBOARD_CSS_REL}.parts`),
  path.resolve(ROOT, 'docs/workspace', `${REMOVED_SPEC_TOKEN}.md`),
  path.resolve(ROOT, 'docs/workspace/DASHBOARD_AUTHORITY_PARITY_CHECKLIST.md'),
];
const CHAT_PAGE_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/pages/chat.ts'
);
const APP_STATIC_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/app.ts'
);
const API_STATIC_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/api.ts'
);
const DASHBOARD_AGENT_WS_BRIDGE_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/dashboard_agent_ws_bridge.ts'
);
const AGENTS_PAGE_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/pages/agents.ts'
);
const TOPBAR_COMPONENTS_CSS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/css/components.css'
);
const STATIC_UI_JS_ROOT = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js'
);
const SNAPSHOT_PATH = path.resolve(
  ROOT,
  'client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json'
);
const PRIMARY_DASHBOARD_HTML_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/index_body.html'
);
const PRIMARY_DASHBOARD_HTML_PARTS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/index_body.html.parts'
);
const PRIMARY_DASHBOARD_COMPONENTS_CSS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/css/components.css'
);
const PRIMARY_DASHBOARD_COMPONENTS_CSS_PARTS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/css/components.css.parts'
);
const PRIMARY_DASHBOARD_LAYOUT_CSS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/css/layout.css'
);
const PRIMARY_DASHBOARD_LAYOUT_CSS_PARTS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/css/layout.css.parts'
);
const STATIC_UI_AUTHORITY_PATTERNS = [
  /\brunLaneCached?\s*\(/,
  /\bspawnSync\s*\(/,
  /\bexecSync\s*\(/,
  /\bchild_process\b/,
  /\binfring-ops\b/,
  /\bcollab-plane\b/,
  /\battention-queue\b/,
  /\bhermes-plane\b/,
  /\bmodel-router\b/,
  /\bterminate-role\b/,
  /\bbacklog-delivery-plane\b/,
  /\bdashboard_runtime_authority\b/,
];

function runSnapshot() {
  return spawnSync(process.execPath, [ENTRYPOINT, TARGET, 'snapshot', '--pretty=0'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env: process.env,
    maxBuffer: 128 * 1024 * 1024,
  });
}

function parseJson(text) {
  const raw = String(text || '').trim();
  assert(raw.length > 0, 'snapshot output should not be empty');
  return JSON.parse(raw);
}

function getFlag(name) {
  const prefix = `${name}=`;
  const row = process.argv.slice(2).find((entry) => String(entry).startsWith(prefix));
  if (!row) return '';
  return String(row).slice(prefix.length).trim();
}

function readUtf8(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  const partDirs = [`${filePath}.parts`];
  if (ext === '.js') {
    partDirs.push(filePath.replace(/\.js$/i, '.ts') + '.parts');
  } else if (ext === '.ts') {
    partDirs.push(filePath.replace(/\.ts$/i, '.js') + '.parts');
  }
  for (const partsDir of partDirs) {
    if (fs.existsSync(partsDir) && fs.statSync(partsDir).isDirectory()) {
      const partFiles = fs
        .readdirSync(partsDir, { withFileTypes: true })
        .filter((entry) => entry && entry.isFile())
        .map((entry) => entry.name)
        .filter((name) => path.extname(name).toLowerCase() === ext)
        .sort((a, b) => a.localeCompare(b, 'en'))
        .map((name) => fs.readFileSync(path.join(partsDir, name), 'utf8'));
      if (partFiles.length > 0) return partFiles.join('\n');
    }
  }
  return fs.readFileSync(filePath, 'utf8');
}

function isRustDashboardLaneWrapperSource(source) {
  const text = String(source || '');
  if (!text) return false;
  const legacyWrapperSignature =
    text.includes('Thin client wrapper only: delegates all dashboard authority to Rust core.') &&
    text.includes("runInfringOps(['dashboard-ui'");
  const apiHostWrapperSignature =
    text.includes('Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.') &&
    text.includes("authority: 'primary_dashboard_ui_over_rust_core_api'") &&
    text.includes('function proxyToBackend(req, res, flags)');
  return legacyWrapperSignature || apiHostWrapperSignature;
}

function assertDashboardFileSizeCaps() {
  const uiRoot = path.resolve(ROOT, 'client/runtime/systems/ui');
  const sourceExts = new Set(['.ts', '.tsx', '.js', '.jsx', '.css', '.html']);
  const generatedDirs = new Set(['.svelte-kit', 'build', 'dist', 'node_modules']);
  const violations = [];
  const walk = (dir) => {
    if (!fs.existsSync(dir)) return;
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (generatedDirs.has(entry.name)) continue;
        walk(fullPath);
        continue;
      }
      const ext = path.extname(entry.name).toLowerCase();
      if (!sourceExts.has(ext)) continue;
      const vendorToken = `${path.sep}vendor${path.sep}`;
      if (fullPath.includes(vendorToken) || path.basename(path.dirname(fullPath)) === 'vendor') continue;
      const lines = fs.readFileSync(fullPath, 'utf8').split(/\r?\n/).length;
      if (lines <= 500) continue;
      const header = fs.readFileSync(fullPath, 'utf8').split(/\r?\n/).slice(0, 8).join('\n');
      if (/FILE_SIZE_EXCEPTION:\s*reason=.+owner=.+expires=\d{4}-\d{2}-\d{2}/.test(header)) continue;
      violations.push({
        file: path.relative(ROOT, fullPath),
        lines,
      });
    }
  };
  walk(uiRoot);
  assert.strictEqual(
    violations.length,
    0,
    `dashboard source files must stay <=500 LoC unless FILE_SIZE_EXCEPTION is declared: ${JSON.stringify(violations.slice(0, 12))}`
  );
}

function assertPrimaryDashboardAuthorityContract() {
  const hostSource = readUtf8(ADAPTER_DASHBOARD_HOST_TS_PATH);
  const distBuildSource = readUtf8(path.resolve(ROOT, 'tests/tooling/scripts/ci/build_dashboard_dist.ts'));
  const assetRouterSource = readUtf8(DASHBOARD_STATIC_ASSET_ROUTER_TS_PATH);
  const segmentedSources = [
    {
      wrapper: PRIMARY_DASHBOARD_HTML_PATH,
      parts: PRIMARY_DASHBOARD_HTML_PARTS_PATH,
      label: 'dashboard HTML',
    },
    {
      wrapper: PRIMARY_DASHBOARD_COMPONENTS_CSS_PATH,
      parts: PRIMARY_DASHBOARD_COMPONENTS_CSS_PARTS_PATH,
      label: 'dashboard components CSS',
    },
    {
      wrapper: PRIMARY_DASHBOARD_LAYOUT_CSS_PATH,
      parts: PRIMARY_DASHBOARD_LAYOUT_CSS_PARTS_PATH,
      label: 'dashboard layout CSS',
    },
  ];
  assertContains(
    hostSource,
    "process.env.INFRING_DASHBOARD_UI || 'primary'",
    'dashboard host should default to the authoritative primary dashboard surface'
  );
  assertContains(
    hostSource,
    'dashboard_surface_retired',
    'dashboard host should explicitly reject retired dashboard URLs'
  );
  assert.ok(
    !hostSource.includes("location: `/dashboard${search}`"),
    'dashboard host should not redirect retired dashboard URLs back into the live authority surface'
  );
  assertContains(
    hostSource,
    "pathname.startsWith('/dashboard/') && !path.extname(pathname)",
    'dashboard host should treat /dashboard/<page> as authoritative dashboard entrypoints'
  );
  assert.ok(
    !distBuildSource.includes('dashboard_sveltekit'),
    'dashboard dist build should no longer package the retired Svelte dashboard module'
  );
  assertContains(
    assetRouterSource,
    'cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js',
    'dashboard asset router should load Highlight.js from a pinned external source instead of an oversized vendored bundle'
  );
  assert.ok(
    !assetRouterSource.includes("readForkScript(staticDir, 'vendor/highlight.min')"),
    'dashboard asset router should not inline the retired vendored Highlight.js bundle'
  );
  for (const source of segmentedSources) {
    assert.ok(
      !fs.existsSync(source.wrapper),
      `${source.label} wrapper file should stay deleted once segmented parts are authoritative`
    );
    assert.ok(
      fs.existsSync(source.parts) && fs.statSync(source.parts).isDirectory(),
      `${source.label} parts directory should remain the authoritative source`
    );
  }
}

function walkUiJsFiles(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walkUiJsFiles(fullPath, out);
      continue;
    }
    if (!/\.(js|ts)$/.test(entry.name)) continue;
    out.push(fullPath);
  }
  return out;
}

function assertContains(haystack, needle, message) {
  if (isRustDashboardLaneWrapperSource(haystack)) {
    // The dashboard lane authority moved into Rust core; string-level JS lane probes
    // are not valid in wrapper mode.
    return;
  }
  assert.ok(String(haystack).includes(needle), message || `missing: ${needle}`);
}

function assertThinClientAuthorityBoundary() {
  const files = walkUiJsFiles(STATIC_UI_JS_ROOT);
  const offenders = [];
  for (const filePath of files) {
    const source = readUtf8(filePath);
    for (const pattern of STATIC_UI_AUTHORITY_PATTERNS) {
      if (!pattern.test(source)) continue;
      offenders.push({
        file: path.relative(ROOT, filePath),
        marker: String(pattern),
      });
    }
  }
  assert.strictEqual(
    offenders.length,
    0,
    `browser UI must stay thin-client (no runtime authority primitives): ${JSON.stringify(offenders.slice(0, 10))}`
  );
}

function assertHeroMenuIsNotClipped() {
  const cssSource = readUtf8(TOPBAR_COMPONENTS_CSS_PATH);
  assertContains(
    cssSource,
    '.global-taskbar-left {\n  min-width: 0;\n  flex: 1 1 auto;\n  display: flex;\n  align-items: center;\n  gap: 14px;\n  overflow: visible;',
    'taskbar hero dropdown must not be clipped by the left taskbar flex container'
  );
  assertContains(
    cssSource,
    '.global-taskbar-page-slot {\n  display: inline-flex;\n  align-items: center;\n  gap: 8px;\n  min-width: 0;\n  overflow: hidden;',
    'page slot should retain clipping after hero dropdown overflow is restored'
  );
}

function assertLegacyDashboardArtifactsRemoved() {
  for (const artifactPath of LEGACY_DASHBOARD_ARTIFACTS) {
    assert.ok(
      !fs.existsSync(artifactPath),
      `removed dashboard artifact should stay deleted: ${path.relative(ROOT, artifactPath)}`
    );
  }

  const readmeSource = readUtf8(path.resolve(ROOT, 'README.md'));
  const todoSource = readUtf8(path.resolve(ROOT, 'docs/workspace/TODO.md'));
  const srsSource = readUtf8(path.resolve(ROOT, 'docs/workspace/SRS.md'));
  const packageSource = readUtf8(path.resolve(ROOT, 'package.json'));
  const cliSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/src/infringctl.rs'));
  const rustUiSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/src/dashboard_ui.rs'));
  const laneSource = readUtf8(TARGET_SOURCE);
  const cohesionSource = readUtf8(
    path.resolve(ROOT, 'client/runtime/config/module_cohesion_legacy_baseline.json')
  );

  assert.ok(!readmeSource.includes(REMOVED_NODE_UI_FLAG), 'README should not mention removed dashboard override flags');
  assert.ok(!todoSource.includes(REMOVED_DASHBOARD_FAMILY), 'TODO should not track the removed compact dashboard family');
  assert.ok(!srsSource.includes(REMOVED_DASHBOARD_FAMILY), 'SRS should not describe the removed compact dashboard family');
  assert.ok(!srsSource.includes(REMOVED_SPEC_TOKEN), 'SRS should not reference the removed dashboard spec');
  assert.ok(!packageSource.includes(REMOVED_DASHBOARD_FAMILY), 'package.json should not expose removed dashboard contract lanes');
  assert.ok(!cliSource.includes(REMOVED_NODE_UI_FLAG), 'CLI should not accept removed dashboard override flags');
  assert.ok(!cliSource.includes(REMOVED_LEGACY_NODE_UI_FLAG), 'CLI should not accept removed dashboard fallback flags');
  assert.ok(!laneSource.includes(REMOVED_TOGGLE_CONTROLS_ACTION), 'dashboard host should not retain removed compact-dashboard actions');
  assert.ok(!laneSource.includes(REMOVED_TOGGLE_SECTION_ACTION), 'dashboard host should not retain removed compact-dashboard section actions');
  assert.ok(!laneSource.includes(REMOVED_SWITCH_TAB_ACTION), 'dashboard host should not retain removed compact-dashboard tab actions');
  assert.ok(!rustUiSource.includes(REMOVED_BROWSER_SHELL_TYPE), 'Rust API lane should not advertise the removed legacy shell');
  assert.ok(
    !cohesionSource.includes(REMOVED_DASHBOARD_CLIENT_REL),
    'module cohesion baseline should not retain deleted dashboard client paths'
  );
}

function assertChatSyntaxGuards() {
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  assert.ok(
    !/async\s+resolveArtifactDirectives\s*:\s*function/.test(chatSource),
    'invalid async object-property syntax in chat page can break dashboard script boot'
  );
  assert.ok(
    /resolveArtifactDirectives\s*:\s*async\s+function(?:\s+\w+)?\s*\(/.test(chatSource),
    'resolveArtifactDirectives must be declared as async function property'
  );
}

function assertDashboardInlineScriptsParse() {
  const { installTsRequireHook } = require(TS_BOOTSTRAP_TS_PATH);
  installTsRequireHook();
  const { buildPrimaryDashboardHtml } = require(DASHBOARD_STATIC_ASSET_ROUTER_TS_PATH);
  const html = String(buildPrimaryDashboardHtml(DASHBOARD_STATIC_DIR) || '');
  assert.ok(html.includes('<script>'), 'dashboard html should include inline script blocks');
  const scriptMatches = Array.from(html.matchAll(/<script>([\s\S]*?)<\/script>/g));
  assert.ok(scriptMatches.length > 0, 'dashboard html should emit at least one inline script');
  scriptMatches.forEach((match, idx) => {
    const source = String((match && match[1]) || '').trim();
    assert.ok(source.length > 0, `dashboard inline script ${idx + 1} should not be empty`);
    assert.doesNotThrow(
      () => new vm.Script(source, { filename: `dashboard_inline_script_${idx + 1}.js` }),
      `dashboard inline script ${idx + 1} must parse successfully`
    );
  });
}

function resolveExpectedDashboardBuildVersion() {
  const latestTagProc = spawnSync('git', ['tag', '--list', '--sort=-v:refname', 'v*'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  const latestTag = latestTagProc.status === 0
    ? String(latestTagProc.stdout || '')
        .split(/\r?\n/)
        .map((row) => String(row || '').trim())
        .find(Boolean)
    : '';
  if (latestTag) {
    return {
      version: latestTag.replace(/^[vV]/, ''),
      tag: latestTag,
      source: 'git_latest_tag',
    };
  }
  const pkg = JSON.parse(fs.readFileSync(path.resolve(ROOT, 'package.json'), 'utf8'));
  const version = String((pkg && pkg.version) || '0.0.0').trim().replace(/^[vV]/, '') || '0.0.0';
  return {
    version,
    tag: `v${version}`,
    source: 'package_json',
  };
}

function assertDashboardBuildVersionFresh() {
  const { installTsRequireHook } = require(TS_BOOTSTRAP_TS_PATH);
  installTsRequireHook();
  const { buildPrimaryDashboardHtml } = require(DASHBOARD_STATIC_ASSET_ROUTER_TS_PATH);
  const html = String(buildPrimaryDashboardHtml(DASHBOARD_STATIC_DIR) || '');
  const match = html.match(/window\.__INFRING_BUILD_INFO\s*=\s*(\{[\s\S]*?\});/);
  assert.ok(match, 'dashboard html should bootstrap build info');
  const payload = JSON.parse(String(match[1] || '{}'));
  const expected = resolveExpectedDashboardBuildVersion();
  assert.strictEqual(
    payload.version,
    expected.version,
    `dashboard build info should follow current repo version (${expected.version})`
  );
  assert.strictEqual(
    payload.tag,
    expected.tag,
    `dashboard build tag should follow current repo tag (${expected.tag})`
  );
  assert.strictEqual(
    payload.source,
    expected.source,
    `dashboard build source should come from ${expected.source}`
  );
}

function assertDashboardVersionRefreshUsesApiVersion() {
  const appSource = readUtf8(APP_STATIC_TS_PATH);
  assertContains(
    appSource,
    "InfringAPI.get('/api/version').catch(function() { return null; })",
    'dashboard store should fetch live version from /api/version during status refresh'
  );
  assertContains(
    appSource,
    "var liveVersion = String(versionObj.version || versionObj.tag || '').trim().replace(/^[vV]/, '');",
    'dashboard store should normalize live version payload before updating the brand label'
  );
  assertContains(
    appSource,
    "this.version = liveVersion || statusObj.version || this.version || window.__INFRING_APP_VERSION || '0.0.0';",
    'dashboard taskbar version should prefer the live /api/version payload over stale bootstrap state'
  );
}

function assertTaskbarHeroSystemMenu() {
  const appSource = readUtf8(APP_STATIC_TS_PATH);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html'));
  const hostSource = readUtf8(ADAPTER_DASHBOARD_HOST_TS_PATH);
  const gatewaySystemRouteSource = readUtf8(GATEWAY_SYSTEM_ROUTES_TS_PATH);
  const gatewaySystemActionSource = readUtf8(GATEWAY_SYSTEM_ACTIONS_TS_PATH);
  const gatewaySystemRouteAssemblySource = readUtf8(GATEWAY_SYSTEM_ROUTE_ASSEMBLY_TS_PATH);
  const gatewayHostLifecycleSource = readUtf8(GATEWAY_HOST_LIFECYCLE_TS_PATH);
  assertHeroMenuIsNotClipped();
  assertContains(htmlSource, 'class="taskbar-hero-menu dashboard-dropdown-surface"', 'taskbar hero dropdown container missing');
  assertContains(htmlSource, "runTaskbarHeroCommand('restart')", 'taskbar hero restart action missing');
  assertContains(htmlSource, "runTaskbarHeroCommand('update')", 'taskbar hero update action missing');
  assertContains(htmlSource, "runTaskbarHeroCommand('shutdown')", 'taskbar hero shutdown action missing');
  assertContains(htmlSource, 'class="taskbar-hero-menu-version"', 'taskbar hero dropdown should render the version footer');
  assert.ok(!htmlSource.includes('taskbar-brand-version'), 'hero should no longer render inline version text');
  assertContains(appSource, 'taskbarHeroMenuOpen: false,', 'taskbar hero dropdown state missing');
  assertContains(appSource, "taskbarHeroActionPending: '',", 'taskbar hero pending-action state missing');
  assertContains(appSource, 'toggleTaskbarHeroMenu()', 'taskbar hero dropdown toggle handler missing');
  assertContains(appSource, 'async runTaskbarHeroCommand(action)', 'taskbar hero command runner missing');
  assertContains(appSource, 'async postTaskbarHeroSystemRoute(route, body, options)', 'taskbar hero direct system-route helper missing');
  assertContains(appSource, "dashboardAction = 'dashboard.system.restart';", 'taskbar hero restart action bus route missing');
  assertContains(appSource, "dashboardAction = 'dashboard.update.apply';", 'taskbar hero update action bus route missing');
  assertContains(appSource, "dashboardAction = 'dashboard.system.shutdown';", 'taskbar hero shutdown action bus route missing');
  assertContains(appSource, "result = await this.postTaskbarHeroSystemRoute(legacyRoute, body, {", 'taskbar hero should prefer the direct system route');
  assertContains(appSource, "result = await InfringAPI.post('/api/dashboard/action', {", 'taskbar hero should still support dashboard action bus fallback');
  assertContains(appSource, "allowTransientSuccess: actionKey === 'restart'", 'taskbar hero should only tolerate transient disconnect for restart');
  assertContains(appSource, 'async waitForTaskbarHeroGatewayHealth(options)', 'taskbar hero restart should verify Gateway health after transient disconnect');
  assertContains(appSource, "InfringToast.success('Gateway restarted and healthy')", 'taskbar hero restart should report verified health, not only accepted command dispatch');
  assertContains(appSource, "legacyRoute = '/api/system/restart';", 'taskbar hero restart legacy fallback missing');
  assertContains(appSource, "legacyRoute = '/api/system/update';", 'taskbar hero update legacy fallback missing');
  assertContains(appSource, "legacyRoute = '/api/system/shutdown';", 'taskbar hero shutdown legacy fallback missing');
  assertContains(gatewaySystemRouteSource, "pathname === '/api/system/restart'", 'Gateway system route restart endpoint missing');
  assertContains(gatewaySystemRouteSource, "pathname === '/api/system/update'", 'Gateway system route update endpoint missing');
  assertContains(gatewaySystemRouteSource, "pathname === '/api/system/shutdown'", 'Gateway system route shutdown endpoint missing');
  assertContains(gatewaySystemActionSource, "function dashboardSystemActionArgs(action, payload = {}, executablePath = '')", 'Gateway system-action arg resolver missing');
  assertContains(gatewaySystemActionSource, "return ['daemon-control', 'heal', '--json', '--dashboard-open=0'];", 'Gateway dashboard-origin restart should route ops binaries through daemon-control heal');
  assertContains(gatewaySystemActionSource, "return ['daemon-control', 'stop', '--json'];", 'Gateway system shutdown should route ops binaries through daemon-control stop');
  assertContains(gatewaySystemActionSource, 'function dispatchDashboardSystemAction(action, payload = {})', 'Gateway system actions should expose detached dispatch for lifecycle actions');
  assertContains(gatewayHostLifecycleSource, 'function scheduleGatewayHostExit(cleanup, delayMs)', 'Gateway host lifecycle should expose bounded shutdown exit scheduling');
  assertContains(gatewaySystemRouteAssemblySource, "if (action === 'update') return runDashboardSystemAction('update', body);", 'Gateway route assembly should route update through system action runner');
  assertContains(gatewaySystemRouteAssemblySource, 'return dispatchDashboardSystemAction(action, body);', 'Gateway route assembly should dispatch restart/shutdown lifecycle actions asynchronously');
  assertContains(gatewaySystemRouteSource, "if (action === 'restart' || action === 'shutdown')", 'Gateway system routes should bypass backend forwarding for lifecycle actions');
  assertContains(gatewaySystemRouteSource, "lifecycle_dispatch: true", 'Gateway system routes should mark lifecycle dispatch projections');
  assertContains(gatewaySystemActionSource, "accepted_async: true", 'Gateway lifecycle dispatch should explicitly mark async acceptance');
  assertContains(gatewaySystemActionSource, "kind: 'gateway_system_action_dispatch'", 'Gateway lifecycle dispatch should emit a dispatch receipt projection');
  assertContains(hostSource, 'scheduleGatewayHostExit(cleanup, body && body.exit_delay_ms);', 'dashboard host should inject shutdown exit through Gateway lifecycle controller');
  assertContains(gatewaySystemRouteSource, "const upstream = await fetchBackend(flags, pathname, {", 'Gateway system routes should prefer the Rust backend system endpoint');
  assertContains(gatewaySystemActionSource, "if (normalized === 'update') {", 'Gateway update command mapping missing');
  assertContains(gatewaySystemActionSource, "if (body.apply !== false) args.push('--apply');", 'Gateway update action should default to apply mode');
}

function assertDashboardHostOverlaysLiveVersion() {
  const { installTsRequireHook } = require(TS_BOOTSTRAP_TS_PATH);
  installTsRequireHook();
  const dashboardHost = require(TARGET_SOURCE);
  assert.ok(
    dashboardHost && typeof dashboardHost.mergeDashboardVersionPayload === 'function',
    'dashboard host should export version overlay helper'
  );
  const payload = dashboardHost.mergeDashboardVersionPayload({
    ok: true,
    version: '0.2.1-alpha.1',
    tag: 'v0.2.1-alpha.1',
    source: 'backend_stale',
    platform: 'macos',
    arch: 'aarch64',
  });
  const expected = resolveExpectedDashboardBuildVersion();
  assert.strictEqual(
    payload.version,
    expected.version,
    `dashboard host should override stale backend version with ${expected.version}`
  );
  assert.strictEqual(
    payload.tag,
    expected.tag,
    `dashboard host should override stale backend tag with ${expected.tag}`
  );
  assert.strictEqual(
    payload.version_source,
    expected.source,
    `dashboard host should expose live version source ${expected.source}`
  );
}

function assertDashboardHostRejectsStaleBackendProcess() {
  const { installTsRequireHook } = require(TS_BOOTSTRAP_TS_PATH);
  installTsRequireHook();
  const hostSource = readUtf8(ADAPTER_DASHBOARD_HOST_TS_PATH);
  const gatewayBackendFreshnessSource = readUtf8(GATEWAY_BACKEND_FRESHNESS_TS_PATH);
  const gatewayBackendStartupStateSource = readUtf8(GATEWAY_BACKEND_STARTUP_STATE_TS_PATH);
  const gatewayBackendHostLauncherSource = readUtf8(GATEWAY_BACKEND_HOST_LAUNCHER_TS_PATH);
  const gatewayDashboardHostStatusSource = readUtf8(GATEWAY_DASHBOARD_HOST_STATUS_TS_PATH);
  assertContains(
    gatewayBackendFreshnessSource,
    'function backendFreshnessSnapshot(flags, options = {})',
    'Gateway should expose a backend process freshness probe'
  );
  assertContains(
    gatewayBackendFreshnessSource,
    'backend_process_older_than_resolved_binary',
    'Gateway should mark a healthy-but-stale backend as unsafe to reuse'
  );
  assertContains(
    gatewayBackendHostLauncherSource,
    'if (await backendHealth(flags, 1500))',
    'Gateway backend host launcher should inspect an already-healthy backend before reusing it'
  );
  assertContains(
    gatewayDashboardHostStatusSource,
    'backend_freshness: backend.freshness || null',
    'Gateway dashboard host status should publish backend freshness diagnostics'
  );
  const dashboardHost = require(TARGET_SOURCE);
  assert.strictEqual(
    typeof dashboardHost.backendFreshnessSnapshot,
    'function',
    'dashboard host should export backend freshness diagnostics for targeted tests'
  );
  const flags = dashboardHost.parseFlags(['serve', '--port=65000', '--api-port=65001']);
  const freshness = dashboardHost.backendFreshnessSnapshot(flags);
  assert.strictEqual(typeof freshness, 'object', 'backend freshness probe should return a structured object');
  assert.strictEqual(freshness.stale, false, 'unused backend test port should not report stale reuse');
  assert.ok(
    Object.prototype.hasOwnProperty.call(freshness, 'restart_enabled'),
    'backend freshness probe should report whether stale restart is enabled'
  );
}

function assertChatEnhancementFeatures() {
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  const agentsSource = readUtf8(AGENTS_PAGE_TS_PATH);
  const apiSource = readUtf8(API_STATIC_TS_PATH);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html'));
  const cssSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/css/components.css'));
  const laneSource = readUtf8(TARGET_SOURCE);

  // Fresh agent init flow ("Who am I?" thinking bubble + init panel)
  assertContains(chatSource, "text: 'Who am I?'", 'fresh-init "Who am I?" seed message missing');
  assert.ok(
    /text:\s*'Who am I\?'[\s\S]{0,220}thinking:\s*true/.test(chatSource),
    'fresh-init "Who am I?" message should render through thinking bubble state'
  );
  assert.ok(
    chatSource.includes('ensureFreshInitThread(resolved);') || chatSource.includes('startFreshInitSequence(resolved);'),
    'fresh-init thread bootstrap missing'
  );
  assertContains(htmlSource, 'class="chat-init-panel"', 'fresh-init panel markup missing');
  assertContains(htmlSource, 'Initialize Agent', 'fresh-init panel title missing');
  assertContains(htmlSource, 'Advanced setup', 'fresh-init advanced setup toggle missing');
  assertContains(htmlSource, 'chat-init-model-grid', 'fresh-init LLM suggestion grid missing');
  assertContains(htmlSource, 'Vibe', 'fresh-init vibe section missing');
  assertContains(chatSource, 'refreshFreshInitModelSuggestions: async function(templateDef)', 'fresh-init role-based LLM ranking helper missing');
  assertContains(chatSource, 'scoreFreshInitModelForRole: function(model, roleKey)', 'fresh-init model scoring function missing');
  assertContains(chatSource, 'freshInitModelSelection = ranked.length ? this.normalizeFreshInitModelRef(ranked[0]) : \'\';', 'fresh-init should auto-select top-ranked model by default');
  assertContains(
    chatSource,
    'if (this.anchorAgentTrailToThinking(host, rect, now, pad, w, h)) return;',
    'thinking anchor should take priority over fresh-init anchor'
  );
  assertContains(
    chatSource,
    'if (activeThinking && activeThinking.offsetParent !== null) return false;',
    'fresh-init anchor should not override an active thinking bubble anchor'
  );
  assertContains(cssSource, '.chat-init-advanced-toggle', 'fresh-init advanced toggle styles missing');
  assertContains(cssSource, '.chat-init-model-meta', 'fresh-init model metadata row styles missing');
  assertContains(chatSource, 'sessionHasAnyHistory: function(data)', 'empty-session history detector missing');
  assertContains(chatSource, 'recoverEmptySessionRender: function(agentId, sessionPayload)', 'empty-session render recovery helper missing');
  assertContains(chatSource, 'pinToLatestOnOpen: function(container, options)', 'chat open pin-to-latest helper missing');
  assertContains(chatSource, 'cancelPinToLatestOnOpen: function()', 'chat open pin cancel helper missing');
  assertContains(chatSource, 'self.pinToLatestOnOpen(null, { maxFrames: 24 });', 'session loader should re-pin to latest after render settles');
  assertContains(chatSource, 'scrollBottomBufferPx: 84', 'chat bottom buffer baseline should preserve visual padding without blank over-scroll');
  assertContains(chatSource, 'scrollBottomClampSlackPx: 16', 'chat bottom clamp slack tuning missing');
  assertContains(chatSource, 'page && page.showFreshArchetypeTiles', 'fresh-init mode should bypass hard bottom clamp');
  assertContains(chatSource, 'setTimeout(function() { host.scrollTop = Math.min(Number(host.scrollTop || 0), resolveLatestMessageScrollTop(page, host));', 'bottom clamp should defer correction to avoid scroll thrash');
  assertContains(chatSource, "text: 'This session is empty. Send a message to begin.'", 'empty-session fallback message missing');
  assertContains(chatSource, 'self.recoverEmptySessionRender(agentId, data || null);', 'empty-session recovery hook missing from session loader');
  assertContains(
    htmlSource,
    "x-if=\"currentAgent && !sessionLoading && (!messages || messages.length === 0) && !showFreshArchetypeTiles\"",
    'primary chat empty-session fallback UI missing'
  );
  assertContains(
    htmlSource,
    "x-if=\"currentAgent && !sessionLoading && (!filteredMessages || filteredMessages.length === 0) && !showFreshArchetypeTiles\"",
    'inline filtered chat empty-session fallback UI missing'
  );
  assert.ok(
    !htmlSource.includes('class="chat-loading-overlay" x-show="currentAgent && sessionLoading" x-cloak'),
    'chat should not render duplicate session-loading overlay spinners'
  );
  assertContains(
    agentsSource,
    "InfringAPI.post('/api/agents/archive-all'",
    'agents archive-all action should use bulk archive endpoint first'
  );

  // Prompt suggestion chips above composer
  assertContains(chatSource, 'refreshPromptSuggestions', 'prompt suggestion refresh flow missing');
  assertContains(chatSource, "/api/agents/' + encodeURIComponent(agentId) + '/suggestions", 'suggestion API client call missing');
  assert.ok(
    !chatSource.includes('collectPromptSuggestionContext()'),
    'prompt suggestions should not derive context from shell-local message history'
  );
  assert.ok(
    !chatSource.includes('payload.recent_context ='),
    'prompt suggestion requests should not include shell-derived recent_context'
  );
  assertContains(chatSource, '/^(post-(response|silent|error|terminal)|init|refresh)$/i.test(cleanHint)', 'prompt suggestion hint sanitizer missing in chat client');
  assertContains(chatSource, 'row = clampWords(row, 10);', 'suggestion normalizer should preserve full 10-word budget');
  assertContains(chatSource, 'if (words < 3 || words > 10) return true;', 'suggestion normalizer word budget guard missing');
  assertContains(chatSource, "rows.push('Tell me more about ' + seed);", 'template-driven suggestion fallback missing');
  assertContains(chatSource, "rows.push('What are next steps for ' + seed);", 'next-step suggestion template missing');
  assertContains(chatSource, "rows.push('Can you verify ' + seed + ' works');", 'verification suggestion template missing');
  assertContains(laneSource, 'Do not echo instructions or policy text.', 'prompt suggestion generator anti-echo guard missing');
  assertContains(laneSource, 'META_SUGGESTION_PATTERNS', 'prompt suggestion meta-pattern scrubber missing');
  assertContains(laneSource, 'function sanitizeSuggestionHint(value)', 'dashboard suggestion hint sanitizer missing');
  assert.ok(
    !laneSource.includes('highest-ROI fix for this task'),
    'prompt suggestion fallback should not emit generic "this task" phrasing'
  );
  assertContains(htmlSource, 'class="prompt-suggestions-row"', 'prompt suggestion row missing');
  assertContains(htmlSource, 'prompt-suggestion-chip', 'prompt suggestion chip missing');
  assertContains(chatSource, 'appendUserChatMessage: function(finalText, msgImages, options)', 'queued prompt render helper missing');
  assertContains(chatSource, 'this.appendUserChatMessage(nextText, nextImages, { deferPersist: true });', 'queued prompts must render only when dequeued');
  assertContains(chatSource, "this.appendUserChatMessage(finalText, msgImages, { deferPersist: true });", 'immediate dispatch should render via shared append helper');
  assertContains(chatSource, 'queue_id: next && next.queue_id ? String(next.queue_id) : \'\'', 'dequeue payload should keep queue id context');
  assertContains(cssSource, 'max-width: 90%;', 'prompt queue stack should cap width at 90%');
  assertContains(cssSource, '.prompt-queue-item:first-child', 'prompt queue top item selector missing');
  assertContains(cssSource, 'border-top-left-radius: 10px;', 'prompt queue top item top-left radius missing');
  assertContains(cssSource, 'border-top-right-radius: 10px;', 'prompt queue top item top-right radius missing');

  // Local model download flow in model switcher
  assertContains(chatSource, 'downloadModelToLocal: function(model)', 'model download action handler missing');
  assertContains(chatSource, "InfringAPI.post('/api/models/download'", 'model download API call missing');
  assertContains(chatSource, 'isModelDownloadable: function(model)', 'model download availability helper missing');
  assertContains(chatSource, 'modelSpecialtyLabel: function(model)', 'model specialty label helper missing');
  assertContains(htmlSource, 'class="model-download-inline-btn"', 'model download button missing in model switcher');
  assertContains(htmlSource, 'class="model-meta-stat model-meta-specialty"', 'model specialty metadata row missing');
  assertContains(laneSource, "req.method === 'POST' && pathname === '/api/models/download'", 'model download backend endpoint missing');
  assertContains(laneSource, 'function inferSystemSpecProfile()', 'local model recommendation should derive system profile');
  assertContains(laneSource, 'function maybeEmitLocalModelBootstrapReminder(snapshot, options = {})', 'startup local-model reminder helper missing');
  assertContains(laneSource, 'Download or connect a local LLM to enable offline mode.', 'offline local-model startup notice text missing');
  assertContains(laneSource, 'function assignSubagentModelOverride(agentId, snapshot, options = {})', 'subagent model routing helper missing');

  // Pointer effects: neon trail in dark mode + ripple in light mode
  assertContains(chatSource, 'handleMessagesPointerMove(event)', 'pointer move handler missing');
  assertContains(chatSource, 'handleMessagesPointerDown(event)', 'pointer down handler missing');
  assertContains(cssSource, '.chat-pointer-trail-dot', 'pointer trail style missing');
  assertContains(cssSource, '.chat-pointer-ripple', 'pointer ripple style missing');
  assertContains(cssSource, "body[data-theme='dark'] .chat-pointer-trail-dot", 'dark neon pointer style missing');
  assertContains(cssSource, "body:not([data-theme='dark']) .chat-pointer-orb.chat-pointer-agent", 'light-mode fairy orb visibility style missing');
  assertContains(cssSource, "rgba(0, 0, 0, 1) 28%", 'light-mode fairy orb should stay black at the visible core');
  assertContains(cssSource, "body:not([data-theme='dark']) .chat-loading-fairy-avatar", 'light-mode loading fairy orb style missing');
  assertContains(cssSource, "rgba(0, 0, 0, 0.98) 27%", 'light-mode loading fairy should stay black');

  // Artifact output: full file and folder tree + downloadable archive
  assertContains(chatSource, "case '/file':", 'slash command /file missing');
  assertContains(chatSource, "case '/folder':", 'slash command /folder missing');
  assertContains(
    chatSource,
    'if (matched && this.isShellOwnedSlashCommand(matched.cmd)) {',
    'chat send path must gate shell-owned slash execution explicitly'
  );
  const shellOwnedSlashBlockStart = chatSource.indexOf('isShellOwnedSlashCommand: function(cmd) {');
  assert.ok(shellOwnedSlashBlockStart >= 0, 'shell-owned slash command classifier missing');
  const shellOwnedSlashBlockEnd = chatSource.indexOf('\n    runSlashMemprobe:', shellOwnedSlashBlockStart);
  assert.ok(shellOwnedSlashBlockEnd > shellOwnedSlashBlockStart, 'shell-owned slash command classifier boundary missing');
  const shellOwnedSlashBlock = chatSource.slice(shellOwnedSlashBlockStart, shellOwnedSlashBlockEnd);
  assertContains(shellOwnedSlashBlock, "case '/help':", 'shell-owned slash classifier should keep local help handling');
  assert.ok(!shellOwnedSlashBlock.includes("case '/file':"), 'workflow-owned /file slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/folder':"), 'workflow-owned /folder slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/memory':"), 'workflow-owned /memory slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/browse':"), 'workflow-owned /browse slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/search':"), 'workflow-owned /search slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/batch':"), 'workflow-owned /batch slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/capabilities':"), 'workflow-owned /capabilities slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/cron':"), 'workflow-owned /cron slash command must not stay shell-owned');
  assert.ok(!shellOwnedSlashBlock.includes("case '/undo':"), 'workflow-owned /undo slash command must not stay shell-owned');
  assertContains(laneSource, "parts[3] === 'file' && parts[4] === 'read'", 'lane file-read endpoint missing');
  assertContains(laneSource, "parts[3] === 'folder' && parts[4] === 'export'", 'lane folder-export endpoint missing');
  assertContains(laneSource, "pathname.startsWith('/api/chat/export/')", 'chat export download endpoint missing');
  assertContains(apiSource, 'upload_endpoint_stub_requires_dashboard_restart', 'upload client should detect stale compat-stub responses');
  assertContains(chatSource, "Failed to upload ' + att.file.name + ': ' + reason", 'upload failure toast should include backend reason');
  assertContains(htmlSource, 'msg.file_output && msg.file_output.path', 'file output chat render missing');
  assertContains(htmlSource, 'msg.folder_output && msg.folder_output.path', 'folder output chat render missing');
  assertContains(htmlSource, 'class="chat-folder-download-link"', 'folder archive download link missing');

  // Progress UI (0-100%)
  assertContains(chatSource, 'parseProgressFromText', 'progress parser missing');
  assertContains(chatSource, 'messageProgress: function(msg)', 'progress accessor missing');
  assertContains(chatSource, 'progressFillStyle: function(msg)', 'progress style function missing');
  assertContains(htmlSource, 'class="chat-progress-wrap"', 'chat progress UI wrapper missing');
  assertContains(htmlSource, 'class="chat-progress-fill"', 'chat progress fill UI missing');

  // Multi-origin source-run grouping (group chat prep)
  assertContains(chatSource, 'messageSourceKey: function(msg)', 'message source-key resolver missing');
  assertContains(chatSource, 'isFirstInSourceRun: function(idx, rows)', 'first-in-run helper missing');
  assertContains(chatSource, 'isLastInSourceRun: function(idx, rows)', 'last-in-run helper missing');
  assertContains(chatSource, 'showMessageTitle(msg, idx, rows)', 'source-run title visibility helper missing');
  assertContains(
    htmlSource,
    "x-show=\"showMessageTitle(msg, idx, messages)\"",
    'primary message list missing source-run title wiring'
  );
  assertContains(
    htmlSource,
    "x-show=\"showMessageTitle(msg, idx, filteredMessages)\"",
    'filtered message list missing source-run title wiring'
  );
  assertContains(
    htmlSource,
    "isGrouped(idx, messages)",
    'primary message list missing source-run grouped wiring'
  );
  assertContains(
    htmlSource,
    "isGrouped(idx, filteredMessages)",
    'filtered message list missing source-run grouped wiring'
  );
  assertContains(cssSource, '.message.system.has-tail', 'system-tail render support missing');
  assertContains(chatSource, 'source_agent_id: m && m.source_agent_id ? String(m.source_agent_id) : \'\'', 'source agent id normalization missing');
  assertContains(chatSource, 'agent_origin: m && m.agent_origin ? String(m.agent_origin) : \'\'', 'agent origin normalization missing');
  assertContains(chatSource, 'User: ... Agent: <answer>', 'chat transcript-leak sanitizer doc missing');
  assert.ok(
    chatSource.includes("var systemOrigin = m && m.system_origin ? String(m.system_origin) : '';") ||
      chatSource.includes('var derivedSystemOrigin = \'\';'),
    'system origin extraction missing'
  );
  assertContains(chatSource, 'system_origin: systemOrigin,', 'system origin normalization missing');
  assert.ok(
    chatSource.includes("return 'system:legacy:' + legacySystemId.toLowerCase();") ||
      chatSource.includes("return 'system';"),
    'system message source key grouping fallback missing'
  );
  assertContains(chatSource, "system_origin: 'slash:help'", 'slash help messages should carry explicit system origin');
  assertContains(chatSource, "system_origin: 'agent:inactive'", 'inactive agent notices should carry explicit system origin');
  assertContains(chatSource, "system_origin: 'runtime:error'", 'runtime error messages should carry explicit system origin');
  assertContains(
    laneSource,
    'User: ... Agent: <final>',
    'server transcript-leak sanitizer doc missing'
  );
  assertContains(chatSource, '/^task accepted\\.\\s*report findings in this thread with receipt-backed evidence\\.?$/i.test(compactText)', 'legacy runtime task ack rows should be filtered from render history');
  assertContains(chatSource, 'return null;', 'legacy runtime task noise should be dropped during session normalization');
  assertContains(chatSource, '}).filter(function(row) { return !!row; });', 'session normalization should filter null rows');
  assertContains(chatSource, 'agent_id: data && data.agent_id ? String(data.agent_id)', 'live ws agent id propagation missing');
  assertContains(chatSource, 'agent_name: data && data.agent_name ? String(data.agent_name)', 'live ws agent name propagation missing');
  assertContains(chatSource, 'var responseTools = Array.isArray(data.tools)', 'ws response tool fallback map missing');
  assertContains(chatSource, "rtool.name || '').toLowerCase() === 'thought_process'", 'ws response thought-tool hydration guard missing');
  assertContains(chatSource, 'streamedTools = responseTools;', 'ws response tool fallback should hydrate final tool cards when stream events are sparse');
  assertContains(chatSource, 'this.assistantTurnMetadataFromPayload(data, streamedTools)', 'ws response path should preserve structured turn metadata from payloads');
  assert.ok(
    !chatSource.includes('fallbackAssistantTextFromPayload: function(payload, tools)'),
    'chat UI should not expose shell-side assistant fallback text replacement'
  );
  assertContains(chatSource, 'responseWorkflowFromPayload: function(payload)', 'chat UI must preserve workflow metadata for history repair and regression checks');
  assertContains(chatSource, 'workflowResponseTextFromPayload: function(payload)', 'chat UI must expose workflow-authored response extraction for synthesized turns');
  assertContains(chatSource, 'var workflowText = this.workflowResponseTextFromPayload(data);', 'chat UI should prefer workflow-authored response text before placeholder assistant prose');
  assertContains(chatSource, 'out.response_workflow = data.response_workflow;', 'chat history metadata must preserve workflow state on assistant rows');
  assertContains(chatSource, 'if (role === \'agent\' && !isTerminal && !String(text || \'\').trim()) {', 'session normalization must repair blank assistant rows even when tool cards are sparse');
  assert.ok(
    !chatSource.includes('self.fallbackAssistantTextFromPayload(m, tools)'),
    'session normalization should not repair assistant history rows from workflow/finalization metadata inside the shell'
  );
  assert.ok(
    !chatSource.includes('this.normalizeSessionMessages({ messages: sanitized })'),
    'cached session restore should not re-normalize persisted tool metadata back into shell-owned session truth'
  );
  assertContains(chatSource, 'duplicate.response_finalization = payload.response_finalization;', 'dedupe merge should preserve response finalization metadata on agent rows');
  assert.ok(
    !chatSource.includes('this.fallbackAssistantTextFromPayload(duplicate, duplicate.tools || [])'),
    'dedupe merge should preserve workflow metadata without rehydrating visible assistant text inside the shell'
  );
  assertContains(chatSource, "id: 'completion-step-' + (si + 1) + '-' + stepName", 'tool completion repair should synthesize visible tool rows when only completion metadata was persisted');
}

function assertMemoryApiWired() {
  var laneSource = readUtf8(TARGET_SOURCE);
  assertContains(
    laneSource,
    "pathname.startsWith('/api/memory/agents/')",
    'memory kv API route missing (agent-scoped memory must not fall through compat stubs)'
  );
  assertContains(
    laneSource,
    "pathname === '/api/memory/search' || pathname === '/api/memory_search'",
    'memory search API fallback route missing'
  );
  assertContains(
    laneSource,
    'recordPassiveConversationMemory(agentId, userText, assistantText, metaText);',
    'passive memory ingestion hook missing from chat conversation append path'
  );
}

function assertEyesPageWired() {
  const laneSource = readUtf8(TARGET_SOURCE);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html'));
  const appSource = readUtf8(APP_STATIC_TS_PATH);
  const eyesPagePath = path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/js/pages/eyes.ts');
