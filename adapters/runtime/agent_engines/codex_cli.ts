#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::codex-cli.
//
// Bounded Codex CLI adapter. The Gateway owns live dispatch and context
// injection; this adapter must not force ephemeral/no-memory execution because
// external runtime engines are expected to behave like durable conversation
// engines when selected from the Shell.

'use strict';

const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

function codexSandboxMode(ctx) {
  const grants = ctx && ctx.message && ctx.message.context_pack && ctx.message.context_pack.universal_tool_grants;
  const policy = grants && grants.permission_policy && typeof grants.permission_policy === 'object' ? grants.permission_policy : {};
  const always = Array.isArray(policy.always_allowed_tool_calls) ? policy.always_allowed_tool_calls : [];
  if (always.includes('artifact.create_propose')) return 'workspace-write';
  return 'read-only';
}

function createCodexCliEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    contextTransportMode: 'prompt_text_compat',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'transitional_bootstrap',
    command: options.command || process.env.INFRING_CODEX_CLI_BIN || process.env.INFRING_CODEX_CLI_PATH,
    commandFallback: 'codex',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_CODEX_LIVE',
    downloadActionRef: 'agent_runtime_download/codex_cli',
    artifactKind: 'codex_cli_result_projection',
    receiptKind: 'external_cli_adapter_receipt',
    versionArgs: ['--version'],
    runArgs: (prompt, ctx) => [
      'exec',
      '--json',
      '--sandbox',
      codexSandboxMode(ctx),
      '--ignore-rules',
      '--color',
      'never',
      prompt,
    ],
    ...options,
  });
}

module.exports = {
  createCodexCliEngineAdapter,
};
