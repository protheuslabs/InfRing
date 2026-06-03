#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::claude-code.

'use strict';

const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

function createClaudeCodeEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'claude_code',
    commandFallback: 'claude',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_CLAUDE_CODE_LIVE',
    downloadActionRef: 'agent_runtime_download/claude_code',
    artifactKind: 'claude_code_result_projection',
    receiptKind: 'claude_code_adapter_receipt',
    versionArgs: ['--version'],
    // Keep the one-shot print adapter stateless at the native Claude layer.
    // Reusing --session-id in print mode can leave the session locked; InfRing
    // continuity is supplied through the Gateway-owned bounded context pack.
    runArgs: (prompt) => [
      '--print',
      '--output-format',
      'stream-json',
      '--verbose',
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
