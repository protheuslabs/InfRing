#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::claude-code.

'use strict';

const { createCliRuntimeEngineAdapter, selectedRuntimeModelArg } = require('./cli_runtime_adapter.ts');

const DIRECT_NATIVE_MUTATION_GRANTS = new Set(['direct_file_write', 'native.direct_file_write', 'filesystem.direct_write']);

function mutationGrantActive(ctx) {
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  return always.some((toolId) => DIRECT_NATIVE_MUTATION_GRANTS.has(String(toolId || '').trim()));
}

function createClaudeCodeEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'claude_code',
    contextTransportMode: 'prompt_text_compat',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'transitional_bootstrap',
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
      const mutationGrant = mutationGrantActive(ctx);
      return [
        '--print',
        '--output-format',
        'stream-json',
        '--verbose',
        '--permission-mode',
        mutationGrant ? 'acceptEdits' : 'default',
        ...(mutationGrant ? ['--allowedTools', 'Read,Write,Edit,Bash'] : []),
        ...(modelArg ? ['--model', modelArg] : []),
        '--include-partial-messages',
        '--include-hook-events',
        prompt,
      ];
    },
    ...options,
  });
}

module.exports = {
  createClaudeCodeEngineAdapter,
};
