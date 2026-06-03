#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-engines::grok-code.

'use strict';

const { createCliRuntimeEngineAdapter } = require('./cli_runtime_adapter.ts');

function createGrokCodeEngineAdapter(options = {}) {
  return createCliRuntimeEngineAdapter({
    engineId: 'grok_code',
    commandFallback: 'grok',
    liveEnvVar: 'INFRING_AGENT_RUNTIME_GROK_CODE_LIVE',
    downloadActionRef: 'agent_runtime_download/grok_code',
    artifactKind: 'grok_code_result_projection',
    receiptKind: 'grok_code_adapter_receipt',
    versionArgs: ['--version'],
    runArgs: (prompt) => ['--disable-web-search', '--no-memory', '--no-subagents', '--no-plan', '--output-format', 'plain', '--verbatim', '--single', prompt],
    ...options,
  });
}

module.exports = {
  createGrokCodeEngineAdapter,
};
