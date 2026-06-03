#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::codex-cli.
//
// Bounded Codex CLI adapter. The Gateway owns live dispatch and context
// injection; this adapter must not force ephemeral/no-memory execution because
// external runtime engines are expected to behave like durable conversation
// engines when selected from the Shell.

'use strict';

const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

function createCodexCliEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'codex_cli',
    command: options.command || process.env.INFRING_CODEX_CLI_BIN || process.env.INFRING_CODEX_CLI_PATH,
    commandFallback: 'codex',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_CODEX_LIVE',
    downloadActionRef: 'agent_runtime_download/codex_cli',
    artifactKind: 'codex_cli_result_projection',
    receiptKind: 'external_cli_adapter_receipt',
    versionArgs: ['--version'],
    runArgs: (prompt) => [
      'exec',
      '--json',
      '--sandbox',
      'read-only',
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
