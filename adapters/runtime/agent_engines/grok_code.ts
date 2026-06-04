#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::grok-code.

'use strict';

const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

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
    runArgs: (prompt, ctx) => [
      '--disable-web-search',
      '--experimental-memory',
      '--no-subagents',
      '--output-format',
      'streaming-json',
      '--permission-mode',
      mutationGrantActive(ctx) ? 'acceptEdits' : 'default',
      '--verbatim',
      '--single',
      prompt,
    ],
    ...options,
  });
}

module.exports = {
  createGrokCodeEngineAdapter,
};
