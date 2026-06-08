#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::grok-code.

'use strict';

const { createCliRuntimeEngineAdapter, selectedRuntimeModelArg } = require('./cli_runtime_adapter.ts');

function mutationGrantActive(ctx) {
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  return always.includes('artifact.create_propose') || always.includes('permission.request');
}

function createGrokCodeEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'grok_code',
    contextTransportMode: 'prompt_text_compat',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'transitional_bootstrap',
    commandFallback: 'grok',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_GROK_CODE_LIVE',
    downloadActionRef: 'agent_runtime_download/grok_code',
    artifactKind: 'grok_code_result_projection',
    receiptKind: 'grok_code_adapter_receipt',
    versionArgs: ['--version'],
    modelDiscovery: {
      kind: 'grok_models_command',
      args: ['models'],
      source: 'grok_models_command',
      freshnessAuthority: 'grok_code_models_command',
      timeoutMs: 8000,
      maxOutputBytes: 65536,
    },
    providerReadinessProbe: {
      source: 'grok_code_minimal_turn_probe',
      timeout_ms: 12000,
      max_output_bytes: 12000,
      cache_ttl_ms: 300000,
      args: [
        '--disable-web-search',
        '--no-subagents',
        '--output-format',
        'streaming-json',
        '--permission-mode',
        'default',
        '--verbatim',
        '--single',
        'Reply exactly INFRING_GROK_READY.',
      ],
    },
    runArgs: (prompt, ctx) => {
      const modelArg = selectedRuntimeModelArg(ctx, ['grok_code', 'grok', 'xai']);
      const mutationGrant = mutationGrantActive(ctx);
      return [
        '--disable-web-search',
        '--experimental-memory',
        '--no-subagents',
        '--output-format',
        'streaming-json',
        '--permission-mode',
        mutationGrant ? 'acceptEdits' : 'default',
        ...(mutationGrant ? ['--always-approve'] : []),
        ...(modelArg ? ['--model', modelArg] : []),
        '--verbatim',
        '--single',
        prompt,
      ];
    },
    ...options,
  });
}

module.exports = {
  createGrokCodeEngineAdapter,
};
