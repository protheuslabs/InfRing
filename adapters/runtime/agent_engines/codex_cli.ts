#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::codex-cli.
//
// Bounded Codex CLI adapter. The Gateway owns live dispatch and context
// injection; this adapter must not force ephemeral/no-memory execution because
// external runtime engines are expected to behave like durable conversation
// engines when selected from the Shell.

'use strict';

const { createCliRuntimeEngineAdapter, selectedRuntimeModelArg } = require('./cli_runtime_adapter.ts');

const DIRECT_NATIVE_MUTATION_GRANTS = new Set(['direct_file_write', 'native.direct_file_write', 'filesystem.direct_write']);

function nativeMutationGrantActive(ctx) {
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  return always.some((toolId) => DIRECT_NATIVE_MUTATION_GRANTS.has(String(toolId || '').trim()));
}

function codexSandboxMode(ctx) {
  return nativeMutationGrantActive(ctx) ? 'workspace-write' : 'read-only';
}

function createCodexCliEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    contextTransportMode: 'bounded_stdin_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'bounded_stdin_active_upstream_structured_input_pending',
    command: options.command || process.env.INFRING_CODEX_CLI_BIN || process.env.INFRING_CODEX_CLI_PATH,
    commandFallback: 'codex',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_CODEX_LIVE',
    downloadActionRef: 'agent_runtime_download/codex_cli',
    artifactKind: 'codex_cli_result_projection',
    receiptKind: 'external_cli_adapter_receipt',
    versionArgs: ['--version'],
    modelDiscovery: {
      kind: 'codex_debug_models',
      args: ['debug', 'models'],
      source: 'codex_debug_models',
      freshnessAuthority: 'codex_cli_debug_models',
      timeoutMs: 8000,
      maxOutputBytes: 1048576,
    },
    runArgs: (_prompt, ctx) => {
      const modelArg = selectedRuntimeModelArg(ctx, ['codex_cli', 'codex', 'openai']);
      return [
        'exec',
        '--json',
        '--skip-git-repo-check',
        '--sandbox',
        codexSandboxMode(ctx),
        '--ignore-rules',
        '--color',
        'never',
        ...(modelArg ? ['--model', modelArg] : []),
        '-',
      ];
    },
    runStdin: (prompt) => prompt,
    ...options,
  });
}

module.exports = {
  createCodexCliEngineAdapter,
};
