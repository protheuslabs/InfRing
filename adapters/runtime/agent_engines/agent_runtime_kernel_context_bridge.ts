#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::kernel-context-bridge.
//
// Narrow Gateway bridge to the Layer-2 memory Kernel context materializer. This
// module does not implement context semantics itself; it invokes the Rust
// `agent_runtime_context_materializer` binary when available and returns the
// canonical AgentRuntimeContextPack projection.

'use strict';

const childProcess = require('child_process');
const fs = require('fs');
const path = require('path');
const { loadAgentRuntimeContextRows } = require('./agent_runtime_context_store.ts');

const BIN_NAME = process.platform === 'win32' ? 'agent_runtime_context_materializer.exe' : 'agent_runtime_context_materializer';

function cleanString(value, max = 2000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function candidateBinaryPaths(root) {
  const workspaceRoot = root || process.cwd();
  const fromEnv = cleanString(process.env.INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_BIN, 1000);
  return [
    fromEnv,
    path.join(workspaceRoot, 'target', 'release', BIN_NAME),
    path.join(workspaceRoot, 'target', 'debug', BIN_NAME),
  ].filter(Boolean);
}

function resolveKernelMaterializerCommand(root) {
  const workspaceRoot = root || process.cwd();
  for (const candidate of candidateBinaryPaths(root)) {
    if (candidate && fs.existsSync(candidate)) {
      return { mode: 'binary', command: candidate, args: [] };
    }
  }
  const cargoMode = cleanString(process.env.INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO || 'auto', 20).toLowerCase();
  const manifestPath = path.join(workspaceRoot, 'core', 'layer2', 'memory', 'Cargo.toml');
  const materializerSourcePath = path.join(workspaceRoot, 'core', 'layer2', 'memory', 'src', 'bin', 'agent_runtime_context_materializer.rs');
  const cargoAllowed = cargoMode !== '0' && cargoMode !== 'false' && cargoMode !== 'off';
  const cargoExplicit = cargoMode === '1' || cargoMode === 'true' || cargoMode === 'on';
  const cargoAuto = cargoMode === 'auto' && fs.existsSync(manifestPath) && fs.existsSync(materializerSourcePath);
  if (cargoAllowed && (cargoExplicit || cargoAuto)) {
    return {
      mode: 'cargo',
      command: 'cargo',
      args: [
        'run',
        '--quiet',
        '--manifest-path',
        manifestPath,
        '--bin',
        'agent_runtime_context_materializer',
      ],
      auto: cargoAuto,
    };
  }
  return null;
}

function spawnJson(commandSpec, payload, options = {}) {
  const timeoutMs = Math.max(1000, Math.min(Number(options.timeoutMs) || 8000, 60000));
  return new Promise((resolve) => {
    const child = childProcess.spawn(commandSpec.command, commandSpec.args || [], {
      cwd: options.root || process.cwd(),
      env: { ...process.env },
      shell: false,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let settled = false;
    const append = (current, chunk) => {
      const next = Buffer.concat([current, Buffer.from(chunk || '')]);
      return next.length > 256000 ? next.subarray(next.length - 256000) : next;
    };
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      try { child.kill('SIGTERM'); } catch {}
      resolve({ ok: false, error: 'kernel_context_materializer_timeout', stderr: stderr.toString('utf8') });
    }, timeoutMs);
    child.stdout.on('data', (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
    child.on('error', (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ ok: false, error: cleanString(error && error.message, 400), stderr: stderr.toString('utf8') });
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      const raw = stdout.toString('utf8').trim();
      try {
        const parsed = JSON.parse(raw);
        resolve({ ok: code === 0 && parsed && parsed.ok !== false, exit_code: code, payload: parsed, stderr: stderr.toString('utf8') });
      } catch (error) {
        resolve({ ok: false, exit_code: code, error: 'kernel_context_materializer_invalid_json', stdout: raw.slice(0, 2000), stderr: stderr.toString('utf8') });
      }
    });
    try { child.stdin.write(`${JSON.stringify(payload)}\n`); } catch {}
    try { child.stdin.end(); } catch {}
  });
}

async function materializeKernelAgentRuntimeContextPack(options = {}) {
  const root = options.root || process.cwd();
  const commandSpec = resolveKernelMaterializerCommand(root);
  if (!commandSpec) {
    return {
      ok: false,
      unavailable: true,
      reason: 'kernel_context_materializer_binary_unavailable',
    };
  }
  const sessionId = cleanString(options.sessionId, 200) || 'session';
  const agentId = cleanString(options.agentId, 160) || 'default';
  const atoms = Array.isArray(options.atoms)
    ? options.atoms
    : loadAgentRuntimeContextRows({ root, sessionId, agentId });
  const run = await spawnJson(commandSpec, {
    session_id: sessionId,
    agent_id: agentId,
    budget_tokens: Number(options.budgetTokens) || 6000,
    pinned_anchor_refs: Array.isArray(options.pinnedAnchorRefs) ? options.pinnedAnchorRefs : [],
    atoms,
  }, { root, timeoutMs: options.timeoutMs });
  if (!run.ok || !run.payload) {
    return {
      ok: false,
      unavailable: false,
      reason: cleanString(run.error || run.stderr || 'kernel_context_materializer_failed', 400),
      command_mode: commandSpec.mode,
    };
  }
  return {
    ok: true,
    command_mode: commandSpec.mode,
    context_pack: run.payload,
  };
}

module.exports = {
  resolveKernelMaterializerCommand,
  materializeKernelAgentRuntimeContextPack,
};
