#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::claude-code.

'use strict';

const {
  createCliRuntimeEngineAdapter,
  nativeDirectMutationGrantActive,
  selectedRuntimeModelArg,
} = require('./cli_runtime_adapter.ts');
const { runClaudeStreamJsonStructuredTurn } = require('./claude_stream_json_transport.ts');

function createClaudeCodeEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'claude_code',
    contextTransportMode: 'bounded_cli_arg_context_envelope',
    structuredTransportTarget: 'native_structured_context_api',
    transportMigrationStatus: 'bounded_cli_arg_active_upstream_native_transport_available_mapping_pending',
    nativeTransport: {
      available: true,
      mode: 'claude_stream_json',
      envVar: 'INFRING_AGENT_RUNTIME_CLAUDE_CODE_NATIVE_TRANSPORT',
      mappingStatus: 'native_transport_adapter_path_available_disabled_by_default',
      evidenceRef: 'core/local/artifacts/agent_runtime_claude_stream_json_live_acceptance_probe_current.json',
    },
    runNativeStructuredTurn: runClaudeStreamJsonStructuredTurn,
    commandFallback: 'claude',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_CLAUDE_CODE_LIVE',
    downloadActionRef: 'agent_runtime_download/claude_code',
    artifactKind: 'claude_code_result_projection',
    receiptKind: 'claude_code_adapter_receipt',
    versionArgs: ['--version'],
    modelDiscovery: {
      kind: 'claude_code_help_model_aliases',
      args: ['--help'],
      source: 'claude_code_help_model_aliases',
      freshnessAuthority: 'claude_code_cli_model_aliases_and_environment',
      timeoutMs: 5000,
      maxOutputBytes: 65536,
    },
    // Keep the one-shot print adapter stateless at the native Claude layer.
    // Reusing --session-id in print mode can leave the session locked; InfRing
    // continuity is supplied through the Gateway-owned bounded context pack.
    runArgs: (prompt, ctx) => {
      const modelArg = selectedRuntimeModelArg(ctx, ['claude_code', 'claude', 'anthropic']);
      return [
        '--print',
        '--output-format',
        'stream-json',
        '--verbose',
        ...claudePermissionArgs(ctx),
        ...(modelArg ? ['--model', modelArg] : []),
        '--include-partial-messages',
        '--include-hook-events',
        prompt,
      ];
    },
    ...options,
  });
}

function claudePermissionArgs(ctx) {
  const mutationGrant = nativeDirectMutationGrantActive(ctx);
  const args = [
    '--permission-mode',
    mutationGrant ? 'acceptEdits' : 'default',
  ];
  if (mutationGrant) {
    args.push('--allowedTools', 'Read,Write,Edit,Bash');
  }
  return args;
}

module.exports = {
  createClaudeCodeEngineAdapter,
  claudePermissionArgs,
};
