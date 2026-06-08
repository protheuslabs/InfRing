#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::opencode.
//
// OpenCode is a CLI-backed external runtime seam. InfRing discovers and runs it
// through Gateway adapter projection only; Shell must not probe or execute it.

'use strict';

const { createCliRuntimeEngineAdapter, selectedRuntimeModelArg } = require('./cli_runtime_adapter.ts');

const DIRECT_NATIVE_MUTATION_GRANTS = new Set(['direct_file_write', 'native.direct_file_write', 'filesystem.direct_write']);

function mutationGrantActive(ctx) {
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  return always.some((toolId) => DIRECT_NATIVE_MUTATION_GRANTS.has(String(toolId || '').trim()));
}

function createOpenCodeEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'opencode',
    contextTransportMode: 'prompt_text_compat',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'transitional_bootstrap',
    command: options.command || process.env.INFRING_OPENCODE_BIN || process.env.INFRING_OPENCODE_PATH,
    commandFallback: 'opencode',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_OPENCODE_LIVE',
    downloadActionRef: 'agent_runtime_download/opencode',
    artifactKind: 'opencode_result_projection',
    receiptKind: 'opencode_adapter_receipt',
    versionArgs: ['--version'],
    runArgs: (prompt, ctx) => {
      const modelArg = selectedRuntimeModelArg(ctx, ['opencode', 'openrouter', 'openai', 'anthropic', 'xai', 'google', 'gemini']);
      const mutationGrant = mutationGrantActive(ctx);
      return [
        'run',
        ...(modelArg ? ['--model', modelArg] : []),
        ...(mutationGrant ? ['--dangerously-skip-permissions'] : []),
        prompt,
      ];
    },
    ...options,
  });
}

module.exports = {
  createOpenCodeEngineAdapter,
};
