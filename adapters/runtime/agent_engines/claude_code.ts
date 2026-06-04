#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::claude-code.

'use strict';

const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

function mutationGrantActive(ctx) {
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  return always.includes('artifact.create_propose') || always.includes('permission.request');
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
    // Keep the one-shot print adapter stateless at the native Claude layer.
    // Reusing --session-id in print mode can leave the session locked; InfRing
    // continuity is supplied through the Gateway-owned bounded context pack.
    runArgs: (prompt, ctx) => [
      '--print',
      '--output-format',
      'stream-json',
      '--verbose',
      '--permission-mode',
      mutationGrantActive(ctx) ? 'acceptEdits' : 'default',
      '--include-partial-messages',
      '--include-hook-events',
      prompt,
    ],
    ...options,
  });
}

module.exports = {
  createClaudeCodeEngineAdapter,
};
