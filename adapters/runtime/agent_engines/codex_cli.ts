#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::codex-cli.
//
// Bounded Codex CLI adapter. The Gateway owns live dispatch and context
// injection; this adapter must not force ephemeral/no-memory execution because
// external runtime engines are expected to behave like durable conversation
// engines when selected from the Shell.

'use strict';

const {
  createCliRuntimeEngineAdapter,
  nativeDirectMutationGrantActive,
  selectedRuntimeModelArg,
} = require('./cli_runtime_adapter.ts');

function codexSandboxMode(ctx) {
  return nativeDirectMutationGrantActive(ctx) ? 'workspace-write' : 'read-only';
}

function createCodexCliEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    contextTransportMode: 'bounded_stdin_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'bounded_stdin_active_upstream_native_transport_available_mapping_pending',
    nativeTransport: {
      available: true,
      mode: 'codex_app_server_json_rpc',
      envVar: 'INFRING_AGENT_RUNTIME_CODEX_NATIVE_TRANSPORT',
      mappingStatus: 'live_acceptance_proven_adapter_mapping_pending',
      evidenceRef: 'core/local/artifacts/agent_runtime_codex_app_server_live_acceptance_probe_current.json',
    },
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
  codexSandboxMode,
};
