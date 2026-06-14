#!/usr/bin/env tsx

const childProcess = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const HIGHLIGHT_JS_CDN_URL = 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js';

const PAGE_SCRIPTS = ['overview', 'chat', 'agents', 'workflows', 'workflow-builder', 'channels', 'eyes', 'skills', 'hands', 'scheduler', 'settings', 'usage', 'sessions', 'logs', 'wizard', 'approvals', 'comms', 'runtime'];
const MIME = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.ico': 'image/x-icon',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
  '.md': 'text/plain; charset=utf-8',
  '.mp3': 'audio/mpeg',
  '.ogg': 'audio/ogg',
  '.pdf': 'application/pdf',
  '.png': 'image/png',
  '.svg': 'image/svg+xml; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.wav': 'audio/wav',
  '.webm': 'audio/webm',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

function fileExists(filePath) {
  try { return fs.existsSync(filePath); } catch { return false; }
}
function readText(filePath, fallback = '') {
  try { return fs.readFileSync(filePath, 'utf8'); } catch { return fallback; }
}
function cleanText(value, maxLen = 200) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}
function normalizeVersionText(value) {
  return cleanText(value, 120).replace(/^[vV]/, '');
}
function parseVersionText(value) {
  const normalized = normalizeVersionText(value);
  const match = normalized.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/);
  if (!match) return null;
  return {
    raw: normalized,
    major: Number(match[1] || 0),
    minor: Number(match[2] || 0),
    patch: Number(match[3] || 0),
    prerelease: String(match[4] || '').split('.').filter(Boolean),
  };
}
function comparePrereleaseIdentifiers(left, right) {
  var leftText = String(left || '');
  var rightText = String(right || '');
  var leftNum = /^\d+$/.test(leftText);
  var rightNum = /^\d+$/.test(rightText);
  if (leftNum && rightNum) {
    var leftValue = Number(leftText);
    var rightValue = Number(rightText);
    if (leftValue > rightValue) return 1;
    if (leftValue < rightValue) return -1;
    return 0;
  }
  if (leftNum && !rightNum) return -1;
  if (!leftNum && rightNum) return 1;
  if (leftText > rightText) return 1;
  if (leftText < rightText) return -1;
  return 0;
}
function compareVersionText(left, right) {
  var leftParsed = parseVersionText(left);
  var rightParsed = parseVersionText(right);
  if (leftParsed && rightParsed) {
    if (leftParsed.major !== rightParsed.major) return leftParsed.major > rightParsed.major ? 1 : -1;
    if (leftParsed.minor !== rightParsed.minor) return leftParsed.minor > rightParsed.minor ? 1 : -1;
    if (leftParsed.patch !== rightParsed.patch) return leftParsed.patch > rightParsed.patch ? 1 : -1;
    if (!leftParsed.prerelease.length && !rightParsed.prerelease.length) return 0;
    if (!leftParsed.prerelease.length) return 1;
    if (!rightParsed.prerelease.length) return -1;
    var len = Math.max(leftParsed.prerelease.length, rightParsed.prerelease.length);
    for (var i = 0; i < len; i += 1) {
      var leftPart = leftParsed.prerelease[i];
      var rightPart = rightParsed.prerelease[i];
      if (leftPart == null) return -1;
      if (rightPart == null) return 1;
      var cmp = comparePrereleaseIdentifiers(leftPart, rightPart);
      if (cmp !== 0) return cmp;
    }
    return 0;
  }
  var leftNormalized = normalizeVersionText(left);
  var rightNormalized = normalizeVersionText(right);
  if (!leftNormalized && !rightNormalized) return 0;
  if (!leftNormalized) return -1;
  if (!rightNormalized) return 1;
  if (leftNormalized > rightNormalized) return 1;
  if (leftNormalized < rightNormalized) return -1;
  return 0;
}
function readJsonFile(filePath) {
  try {
    return JSON.parse(readText(filePath, '{}') || '{}');
  } catch {
    return null;
  }
}
function versionSourcePriority(source) {
  var key = cleanText(source, 80);
  if (key === 'git_latest_tag') return 40;
  if (key === 'install_release_meta') return 30;
  if (key === 'install_release_tag') return 28;
  if (key === 'runtime_version_contract') return 20;
  if (key === 'package_json') return 10;
  return 0;
}
function buildVersionCandidate(version, tag, source) {
  var normalizedVersion = normalizeVersionText(version);
  if (!normalizedVersion) return null;
  var normalizedTag = cleanText(tag || ('v' + normalizedVersion), 120) || ('v' + normalizedVersion);
  return {
    version: normalizedVersion,
    tag: normalizedTag,
    source: cleanText(source || 'unknown', 80) || 'unknown',
  };
}
function pickHigherVersionCandidate(best, candidate) {
  if (!candidate) return best || null;
  if (!best) return candidate;
  var cmp = compareVersionText(candidate.version, best.version);
  if (cmp > 0) return candidate;
  if (cmp < 0) return best;
  return versionSourcePriority(candidate.source) >= versionSourcePriority(best.source) ? candidate : best;
}
function readGitLatestTagCandidate(workspaceRoot) {
  try {
    var result = childProcess.spawnSync('git', ['tag', '--list', '--sort=-v:refname', 'v*'], {
      cwd: workspaceRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    if (!result || result.status !== 0) return null;
    var tag = String(result.stdout || '').split(/\r?\n/).map(function(row) {
      return cleanText(row, 120);
    }).find(Boolean);
    return buildVersionCandidate(tag, tag, 'git_latest_tag');
  } catch {
    return null;
  }
}
function readInstalledReleaseCandidate(workspaceRoot) {
  var metaPath = path.resolve(workspaceRoot, 'local/state/ops/install_release_meta.json');
  var meta = readJsonFile(metaPath);
  if (meta && typeof meta === 'object') {
    var metaValue = cleanText(
      (meta && (meta.release_version_normalized || meta.release_tag)) || '',
      120
    );
    var metaTag = cleanText(meta && meta.release_tag, 120);
    var metaCandidate = buildVersionCandidate(metaValue, metaTag || ('v' + normalizeVersionText(metaValue)), 'install_release_meta');
    if (metaCandidate) return metaCandidate;
  }
  var tagPath = path.resolve(workspaceRoot, 'local/state/ops/install_release_tag.txt');
  var rawTag = cleanText(readText(tagPath, '').split(/\r?\n/)[0] || '', 120);
  return buildVersionCandidate(rawTag, rawTag, 'install_release_tag');
}
function findWorkspaceRoot(startDir) {
  let cursor = path.resolve(startDir || '.');
  for (let hop = 0; hop < 12; hop += 1) {
    const packageJsonPath = path.resolve(cursor, 'package.json');
    if (fileExists(packageJsonPath)) return cursor;
    const next = path.dirname(cursor);
    if (!next || next === cursor) break;
    cursor = next;
  }
  return path.resolve(startDir || '.');
}
function readBuildVersionInfo(staticDir) {
  const workspaceRoot = findWorkspaceRoot(staticDir);
  const runtimeVersionPath = path.resolve(
    workspaceRoot,
    'client',
    'runtime',
    'config',
    'runtime_version.json'
  );
  const packagePath = path.resolve(workspaceRoot, 'package.json');
  let best = null;
  const runtimeVersion = readJsonFile(runtimeVersionPath);
  if (runtimeVersion && typeof runtimeVersion === 'object') {
    best = pickHigherVersionCandidate(
      best,
      buildVersionCandidate(
        runtimeVersion && runtimeVersion.version,
        runtimeVersion && runtimeVersion.tag,
        cleanText(runtimeVersion && runtimeVersion.source, 80) || 'runtime_version_contract'
      )
    );
  }
  const pkg = readJsonFile(packagePath);
  if (pkg && typeof pkg === 'object') {
    best = pickHigherVersionCandidate(
      best,
      buildVersionCandidate(pkg && pkg.version, '', 'package_json')
    );
  }
  best = pickHigherVersionCandidate(best, readInstalledReleaseCandidate(workspaceRoot));
  best = pickHigherVersionCandidate(best, readGitLatestTagCandidate(workspaceRoot));
  if (!best) {
    return {
      version: '0.0.0',
      tag: 'v0.0.0',
      source: 'fallback_default',
    };
  }
  return best;
}
function contentTypeForFile(filePath) {
  return MIME[path.extname(filePath).toLowerCase()] || 'application/octet-stream';
}
const SEGMENT_GHOST_STEM_PATTERN = /(?:^|[._-])(zz|bak|backup|orig|rej|tmp|temp|old)(?:[._-]|$)/i;
function isGhostSegmentPartFileName(fileName) {
  const normalized = String(fileName || '').trim().toLowerCase();
  if (!normalized) return false;
  if (normalized.endsWith('~')) return true;
  const ext = path.extname(normalized);
  const stem = ext ? normalized.slice(0, -ext.length) : normalized;
  return SEGMENT_GHOST_STEM_PATTERN.test(stem);
}
function listSegmentPartFiles(basePath) {
  const ext = path.extname(basePath).toLowerCase();
  const partDirs = [`${basePath}.parts`];
  if (ext === '.js') partDirs.push(basePath.replace(/\.js$/i, '.ts') + '.parts');
  if (ext === '.ts') partDirs.push(basePath.replace(/\.ts$/i, '.js') + '.parts');
  const segmentFileSortComparator = (a, b) => {
    const parseSortKey = (absPath) => {
      const fileName = path.basename(absPath);
      const stem = fileName.replace(/\.[^.]+$/, '');
      // Preserve deterministic order across renamed segment files by using any
      // leading numeric shard prefix (e.g. 005-, 020-, 0001-), not only 4 digits.
      const shardMatch = stem.match(/^(\d+)/);
      const shard = shardMatch ? Number.parseInt(shardMatch[1], 10) : Number.MAX_SAFE_INTEGER;
      const partMatch = stem.match(/(?:^|[._-])part(\d+)([a-z]*)/i);
      const partNumber = partMatch ? Number.parseInt(partMatch[1], 10) : -1;
      const partSuffix = partMatch ? String(partMatch[2] || '').toLowerCase() : '';
      return { shard, partNumber, partSuffix, stem };
    };
    const ka = parseSortKey(a);
    const kb = parseSortKey(b);
    if (ka.shard !== kb.shard) return ka.shard - kb.shard;
    if (ka.partNumber !== kb.partNumber) return ka.partNumber - kb.partNumber;
    if (ka.partSuffix !== kb.partSuffix) return ka.partSuffix.localeCompare(kb.partSuffix, 'en');
    return ka.stem.localeCompare(kb.stem, 'en');
  };
  for (const partsDir of partDirs) {
    try {
      if (!fs.statSync(partsDir).isDirectory()) continue;
      const rows = fs.readdirSync(partsDir, { withFileTypes: true })
        .filter(
          (entry) =>
            entry.isFile() &&
            path.extname(entry.name).toLowerCase() === ext &&
            !isGhostSegmentPartFileName(entry.name),
        )
        .map((entry) => path.resolve(partsDir, entry.name))
        .sort(segmentFileSortComparator);
      if (rows.length) return rows;
    } catch {}
  }
  return [];
}
function readSegmentedText(basePath, fallback = '') {
  const partFiles = listSegmentPartFiles(basePath);
  if (partFiles.length) {
    const joined = partFiles.map((filePath) => readText(filePath, '')).filter(Boolean).join('\n');
    if (joined.trim()) return joined;
  }
  return readText(basePath, fallback);
}
function rebrandDashboardText(text) {
  return String(text || '')
    .replace(/\bReference Runtime\b/g, 'Infring')
    .replace(/\bREFERENCE_RUNTIME\b/g, 'INFRING')
    .replace(/\breference_runtime\b/g, 'infring')
    .replace(/\bControl Runtime\b/g, 'Infring')
    .replace(/\bCONTROL_RUNTIME\b/g, 'INFRING')
    .replace(/\bcontrol_runtime\b/g, 'infring');
}
function injectBeforeHeadClose(head, snippet) {
  if (!snippet) return head;
  if (head.includes('</head>')) return head.replace('</head>', `${snippet}\n</head>`);
  return `${head}\n${snippet}`;
}
function readForkScript(staticDir, basePathNoExt) {
  const jsPath = path.resolve(staticDir, `${basePathNoExt}.js`);
  if (fileExists(jsPath) || listSegmentPartFiles(jsPath).length > 0) return readSegmentedText(jsPath, '');
  const tsPath = path.resolve(staticDir, `${basePathNoExt}.ts`);
  return fileExists(tsPath) || listSegmentPartFiles(tsPath).length > 0 ? readSegmentedText(tsPath, '') : '';
}
function agentMutationSyncPatchScript() {
  return [
    '(function(){',
    '  if (window.__infringAgentMutationSyncPatchInstalled) return;',
    '  window.__infringAgentMutationSyncPatchInstalled = true;',
    '  function parseUrl(rawPath) {',
    '    try { return new URL(String(rawPath || \'\'), window.location.origin); } catch(_) { return null; }',
    '  }',
    '  function readStore() {',
    '    try {',
    '      if (window.Alpine && typeof window.Alpine.store === \'function\') return window.Alpine.store(\'app\');',
    '    } catch(_) {}',
    '    return null;',
    '  }',
    '  function triggerForcedRefreshBurst() {',
    '    var delays = [0, 260, 920];',
    '    delays.forEach(function(delay) {',
    '      window.setTimeout(function() {',
    '        var store = readStore();',
    '        if (!store || typeof store.refreshAgents !== \'function\') return;',
    '        Promise.resolve(store.refreshAgents({ force: true })).catch(function() {});',
    '      }, delay);',
    '    });',
    '  }',
    '  function isCreatePath(pathname) {',
    '    if (pathname === \'/api/agents\') return true;',
    '    return /^\\/api\\/agents\\/[^\\/]+\\/(clone|revive)$/.test(pathname);',
    '  }',
    '  function isArchivePath(pathname) {',
    '    return /^\\/api\\/agents\\/[^\\/]+$/.test(pathname);',
    '  }',
    '  function installApiPatch() {',
    '    var api = window.InfringAPI;',
    '    if (!api) return false;',
    '    if (api.__infringAgentMutationSyncPatched) return true;',
    '    api.__infringAgentMutationSyncPatched = true;',
    '    var basePost = typeof api.post === \'function\' ? api.post.bind(api) : null;',
    '    var baseDel = typeof api.del === \'function\' ? api.del.bind(api) : null;',
    '    if (basePost) {',
    '      api.post = function(path, body) {',
    '        var parsed = parseUrl(path);',
    '        return Promise.resolve(basePost(path, body)).then(function(result) {',
    '          var pathname = parsed ? parsed.pathname : String(path || \'\');',
    '          if (isCreatePath(pathname)) triggerForcedRefreshBurst();',
    '          return result;',
    '        });',
    '      };',
    '    }',
    '    if (baseDel) {',
    '      api.del = function(path) {',
    '        var parsed = parseUrl(path);',
    '        return Promise.resolve(baseDel(path)).then(function(result) {',
    '          var pathname = parsed ? parsed.pathname : String(path || \'\');',
    '          if (isArchivePath(pathname)) triggerForcedRefreshBurst();',
    '          return result;',
    '        });',
    '      };',
    '      api.delete = api.del;',
    '    }',
    '    return true;',
    '  }',
    '  if (installApiPatch()) return;',
    '  var attempts = 0;',
    '  var timer = window.setInterval(function() {',
    '    attempts += 1;',
    '    if (installApiPatch() || attempts >= 80) window.clearInterval(timer);',
    '  }, 100);',
    '})();',
  ].join('\n');
}

function hasPrimaryDashboardUi(staticDir) {
  const headPath = path.resolve(staticDir, 'index_head.html');
  const bodyPath = path.resolve(staticDir, 'index_body.html');
  return (fileExists(headPath) || listSegmentPartFiles(headPath).length > 0) && (fileExists(bodyPath) || listSegmentPartFiles(bodyPath).length > 0);
}
function buildPrimaryDashboardHtml(staticDir) {
  const buildVersion = readBuildVersionInfo(staticDir);
  const head = readSegmentedText(path.resolve(staticDir, 'index_head.html'), '');
  const body = readSegmentedText(path.resolve(staticDir, 'index_body.html'), '');
  if (!head || !body) return '';
  const headWithExternalAssets = injectBeforeHeadClose(
    head,
    `<script defer src="${HIGHLIGHT_JS_CDN_URL}"></script>`
  );
  const css = [
    readSegmentedText(path.resolve(staticDir, 'css/theme.css'), ''),
    readSegmentedText(path.resolve(staticDir, 'css/layout.css'), ''),
    readSegmentedText(path.resolve(staticDir, 'css/components.css'), ''),
    readText(path.resolve(staticDir, 'vendor/github-dark.min.css'), ''),
  ].join('\n');
  const scripts = [
    readForkScript(staticDir, 'vendor/marked.min'),
    readForkScript(staticDir, 'vendor/highlight.min'),
    readForkScript(staticDir, 'vendor/chart.umd.min'),
    readForkScript(staticDir, 'js/shell/app_store_shell_services'),
    readForkScript(staticDir, 'js/chat_store'),
    readForkScript(staticDir, 'js/svelte/chat_bubble.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_stream_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/sidebar_rail_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/sidebar_agent_list_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/popup_window_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/taskbar_menu_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_map_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/agent_details_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/tool_card_stack_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/composer_lane_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/taskbar_dropdown_cluster_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/taskbar_system_items_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/bottom_dock_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/workspace_panel_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/prompt_queue_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/prompt_suggestions_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/context_ring_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_archived_banner_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_header_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_search_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_input_footer_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/session_switcher_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_thread_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_divider_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_meta_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_context_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_progress_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_artifact_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_media_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_terminal_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/message_placeholder_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/messages_surface_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_empty_state_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/dropzone_overlay_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_loading_overlay_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_map_rail_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/system_thread_placeholder_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_map_viewport_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_loading_content_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/taskbar_hero_menu_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/taskbar_nav_cluster_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/slash_command_menu_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/model_picker_menu_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/git_tree_picker_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/model_switcher_panel_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/approvals_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/chat_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/agents_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/scheduler_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/scheduler_jobs_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/scheduler_triggers_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/scheduler_history_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/eyes_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/overview_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/workflows_list_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/workflows_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/workflows_builder_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/channels_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/skills_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/skills_installed_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/skills_clawhub_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/skills_mcp_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/skills_create_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_providers_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_models_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_tools_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_info_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_config_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_security_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_network_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_budget_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/settings_migration_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/analytics_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/analytics_summary_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/analytics_by_model_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/analytics_by_agent_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/analytics_costs_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/logs_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/logs_live_controls_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/logs_audit_controls_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/logs_live_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/logs_audit_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/wizard_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/sessions_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/sessions_filter_controls_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/sessions_conversation_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/sessions_memory_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/comms_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/runtime_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/hands_page_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/hands_available_tab_shell.bundle'),
    readForkScript(staticDir, 'js/svelte/hands_active_tab_shell.bundle'),
    readForkScript(staticDir, 'js/shell/shared_shell_services'),
    readForkScript(staticDir, 'js/shell/dragbar_shell_services'),
    readForkScript(staticDir, 'js/shell/taskbar_dock_shell_services'),
    readForkScript(staticDir, 'js/shell/simple_page_panel_shell_services'),
    readForkScript(staticDir, 'js/shell/message_metadata_shell_services'),
    readForkScript(staticDir, 'js/api'),
    readForkScript(staticDir, 'js/app_bottom_dock_ghost_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_geometry_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_snap_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_bounds_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_wall_cap_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_container_drag_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_tile_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_order_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_axis_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_reorder_animation_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_hover_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_pointer_drag_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_legacy_drag_helpers'),
    readForkScript(staticDir, 'js/app_bottom_dock_drop_helpers'),
    readForkScript(staticDir, 'js/app_dashboard_popup_helpers'),
    readForkScript(staticDir, 'js/app_dashboard_popup_origin_helpers'),
    readForkScript(staticDir, 'js/app_rendering_helpers'),
    readForkScript(staticDir, 'js/app_boot_progress_helpers'),
    readForkScript(staticDir, 'js/app_bootstrap_state_helpers'),
    readForkScript(staticDir, 'js/app_chat_sidebar_animation_helpers'),
    readForkScript(staticDir, 'js/app_chat_sidebar_topology_helpers'),
    readForkScript(staticDir, 'js/app_chat_sidebar_search_helpers'),
    readForkScript(staticDir, 'js/app_navigation_history_helpers'),
    readForkScript(staticDir, 'js/app_sidebar_toggle_helpers'),
    readForkScript(staticDir, 'js/app_auth_helpers'),
    readForkScript(staticDir, 'js/app_layout_measurement_helpers'),
    readForkScript(staticDir, 'js/app_taskbar_dock_projection_helpers'),
    readForkScript(staticDir, 'js/app_taskbar_dock_drag_helpers'),
    readForkScript(staticDir, 'js/app_drag_surface_bounds_helpers'),
    readForkScript(staticDir, 'js/app_drag_surface_lock_helpers'),
    readForkScript(staticDir, 'js/app_chat_map_basic_helpers'),
    readForkScript(staticDir, 'js/app_taskbar_reorder_helpers'),
    readForkScript(staticDir, 'js/app_chat_sidebar_snap_helpers'),
    readForkScript(staticDir, 'js/app_chat_sidebar_drag_helpers'),
    readForkScript(staticDir, 'js/app_chat_map_snap_helpers'),
    readForkScript(staticDir, 'js/app_popup_window_projection_helpers'),
    readForkScript(staticDir, 'js/app_popup_window_drag_helpers'),
    readForkScript(staticDir, 'js/app_agent_refresh_helpers'),
    readForkScript(staticDir, 'js/app_agent_preview_helpers'),
    readForkScript(staticDir, 'js/app_notification_helpers'),
    readForkScript(staticDir, 'js/app_session_activity_helpers'),
    readForkScript(staticDir, 'js/app_status_helpers'),
    readForkScript(staticDir, 'js/app_ui_state_helpers'),
    readForkScript(staticDir, 'js/app_markdown_helpers'),
    readForkScript(staticDir, 'js/app_chat_store_bridge'),
    readForkScript(staticDir, 'js/app'),
    readForkScript(staticDir, 'js/pages/chat_conversation_cache_helpers'),
    readForkScript(staticDir, 'js/pages/chat_input_history_helpers'),
    readForkScript(staticDir, 'js/pages/chat_message_display_helpers'),
    readForkScript(staticDir, 'js/pages/chat_model_catalog_helpers'),
    readForkScript(staticDir, 'js/pages/chat_paste_helpers'),
    readForkScript(staticDir, 'js/pages/chat_session_notice_helpers'),
    readForkScript(staticDir, 'js/pages/chat_scroll_helpers'),
    PAGE_SCRIPTS.map((name) => readForkScript(staticDir, `js/pages/${name}`)).filter(Boolean).join('\n'),
  ].filter(Boolean).join('\n');
  const alpine = readForkScript(staticDir, 'vendor/alpine.min');
  const versionBootstrap = [
    'window.__INFRING_BUILD_INFO = ' + JSON.stringify(buildVersion) + ';',
    'window.__INFRING_APP_VERSION = window.__INFRING_BUILD_INFO.version || "0.0.0";',
    'window.__INFRING_APP_TAG = window.__INFRING_BUILD_INFO.tag || ("v" + window.__INFRING_APP_VERSION);',
  ].join('\n');
  return rebrandDashboardText([headWithExternalAssets, '<style>', css, '</style>', body, '<script>', versionBootstrap, '</script>', '<script>', scripts, '</script>', '<script>', alpine, '</script>', '<script>', agentMutationSyncPatchScript(), '</script>', '</body></html>'].join('\n'));
}
function readPrimaryDashboardAsset(staticDir, pathname) {
  const requestPath = pathname === '/' || pathname === '/dashboard' || pathname === '/dashboard/' ? '/index_body.html' : pathname;
  const resolved = path.resolve(staticDir, String(requestPath || '/').replace(/^\/+/, ''));
  const ext = path.extname(resolved).toLowerCase();
  if (!resolved.startsWith(staticDir)) return null;
  if (!fileExists(resolved) && listSegmentPartFiles(resolved).length === 0) return null;
  if (['.js', '.ts', '.css', '.html', '.json', '.map', '.md', '.txt'].includes(ext)) return { body: rebrandDashboardText(readSegmentedText(resolved, '')), contentType: contentTypeForFile(resolved) };
  return { body: fs.readFileSync(resolved), contentType: contentTypeForFile(resolved) };
}

module.exports = {
  hasPrimaryDashboardUi,
  buildPrimaryDashboardHtml,
  readBuildVersionInfo,
  readPrimaryDashboardAsset,
};
