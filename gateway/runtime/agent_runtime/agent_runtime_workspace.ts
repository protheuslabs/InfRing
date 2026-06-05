#!/usr/bin/env tsx

// Layer ownership: gateway/runtime::agent-runtime::workspace-selection.
//
// Gateway owns external-runtime workspace selection because it is part of the
// boundary membrane between Shell input and engine execution. Shells may request
// a working directory, but they must not own the permission boundary or git-root
// derivation policy.

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function stripTerminalControls(value) {
  return String(value == null ? '' : value)
    .replace(/\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1B\\))/g, '')
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '');
}
function cleanPathText(value, maxLen = 1200) { return stripTerminalControls(value).replace(/\r\n/g, '\n').replace(/\n+/g, ' ').trim().slice(0, maxLen); }

function writeJson(file, payload) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(payload, null, 2)}\n`);
}

function createAgentRuntimeWorkspaceStore(options = {}) {
  const root = path.resolve(options.root || process.cwd());
  const statusDir = path.resolve(options.statusDir || path.join(root, 'client', 'runtime', 'local', 'state', 'ui', 'infring_dashboard'));
  const workspacePath = path.resolve(statusDir, 'agent_runtime_workspace.json');

  function normalizeAgentRuntimeWorkspacePath(value) {
    const raw = cleanPathText(value, 1200);
    if (!raw) return root;
    const expanded = raw === '~' || raw.startsWith('~/')
      ? path.join(process.env.HOME || root, raw.slice(2))
      : raw;
    const resolved = path.resolve(expanded);
    try {
      const stat = fs.statSync(resolved);
      if (!stat.isDirectory()) return root;
      return fs.realpathSync(resolved);
    } catch {
      return root;
    }
  }

  function deriveGitRootForWorkspace(workspaceDir) {
    let cursor = normalizeAgentRuntimeWorkspacePath(workspaceDir);
    const seen = new Set();
    while (cursor && !seen.has(cursor)) {
      seen.add(cursor);
      try {
        if (fs.existsSync(path.join(cursor, '.git'))) return cursor;
      } catch {}
      const next = path.dirname(cursor);
      if (!next || next === cursor) break;
      cursor = next;
    }
    return '';
  }

  function agentRuntimeWorkspaceLabel(workspaceDir) {
    const dir = normalizeAgentRuntimeWorkspacePath(workspaceDir);
    const base = path.basename(dir) || dir;
    return `.../${base}`;
  }

  function projectAgentRuntimeWorkspace(traceId, row) {
    const workspaceDir = normalizeAgentRuntimeWorkspacePath(row && (row.workspace_dir || row.active_workspace));
    const gitRoot = cleanPathText((row && row.git_root) || deriveGitRootForWorkspace(workspaceDir), 1200);
    return {
      ok: true,
      type: 'agent_runtime_workspace_projection',
      schema_version: 1,
      trace_id: cleanText(traceId, 200),
      active_workspace: workspaceDir,
      workspace_dir: workspaceDir,
      display_label: agentRuntimeWorkspaceLabel(workspaceDir),
      basename: path.basename(workspaceDir) || workspaceDir,
      git_root: gitRoot,
      git_root_label: gitRoot ? agentRuntimeWorkspaceLabel(gitRoot) : '',
      scope: cleanText(row && row.scope, 80) || 'global_default',
      source: cleanText(row && row.source, 120) || 'dashboard_gateway',
      updated_at: cleanText(row && row.updated_at, 80),
      permission_boundary: {
        home_base: workspaceDir,
        derived_git_root: gitRoot,
        write_outside_workspace_requires_approval: true,
        write_outside_git_root_requires_approval: true,
      },
    };
  }

  function loadAgentRuntimeWorkspace(traceId = '') {
    try {
      const parsed = JSON.parse(fs.readFileSync(workspacePath, 'utf8'));
      return projectAgentRuntimeWorkspace(traceId, parsed);
    } catch {
      return projectAgentRuntimeWorkspace(traceId, {
        workspace_dir: root,
        git_root: deriveGitRootForWorkspace(root),
        scope: 'global_default',
        source: 'default_repo_root',
        updated_at: '',
      });
    }
  }

  function saveAgentRuntimeWorkspace(workspaceDir, traceId, scope = 'global_default') {
    const activeWorkspace = normalizeAgentRuntimeWorkspacePath(workspaceDir);
    const row = {
      type: 'agent_runtime_workspace_selection',
      schema_version: 1,
      workspace_dir: activeWorkspace,
      active_workspace: activeWorkspace,
      display_label: agentRuntimeWorkspaceLabel(activeWorkspace),
      git_root: deriveGitRootForWorkspace(activeWorkspace),
      scope: cleanText(scope, 80) || 'global_default',
      updated_at: nowIso(),
      trace_id: cleanText(traceId, 200),
      source: 'dashboard_gateway',
    };
    writeJson(workspacePath, row);
    return projectAgentRuntimeWorkspace(traceId, row);
  }

  function pickAgentRuntimeWorkspaceDirectory() {
    if (process.platform === 'darwin') {
      const result = spawnSync('osascript', ['-e', 'POSIX path of (choose folder with prompt "Select InfRing working directory")'], {
        encoding: 'utf8',
        timeout: 120000,
      });
      const out = cleanPathText(result.stdout, 1200);
      if (result.status === 0 && out) return { ok: true, path: out };
      return { ok: false, cancelled: true, reason: cleanText(result.stderr || 'folder_picker_cancelled', 240) };
    }
    if (process.platform === 'win32') {
      const script = '[void][System.Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms");$d=New-Object System.Windows.Forms.FolderBrowserDialog;$d.Description="Select InfRing working directory";if($d.ShowDialog() -eq "OK"){[Console]::WriteLine($d.SelectedPath)}';
      const result = spawnSync('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
        encoding: 'utf8',
        timeout: 120000,
        windowsHide: true,
      });
      const out = cleanPathText(result.stdout, 1200);
      if (result.status === 0 && out) return { ok: true, path: out };
      return { ok: false, cancelled: true, reason: cleanText(result.stderr || 'folder_picker_cancelled', 240) };
    }
    const result = spawnSync('zenity', ['--file-selection', '--directory', '--title=Select InfRing working directory'], {
      encoding: 'utf8',
      timeout: 120000,
    });
    const out = cleanPathText(result.stdout, 1200);
    if (result.status === 0 && out) return { ok: true, path: out };
    return { ok: false, cancelled: true, reason: cleanText(result.stderr || 'folder_picker_unavailable', 240) };
  }

  return {
    workspacePath,
    normalizeAgentRuntimeWorkspacePath,
    deriveGitRootForWorkspace,
    agentRuntimeWorkspaceLabel,
    projectAgentRuntimeWorkspace,
    loadAgentRuntimeWorkspace,
    saveAgentRuntimeWorkspace,
    pickAgentRuntimeWorkspaceDirectory,
  };
}

module.exports = {
  createAgentRuntimeWorkspaceStore,
};
