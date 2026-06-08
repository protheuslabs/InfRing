#!/usr/bin/env tsx

'use strict';

const fs = require('fs');
const path = require('path');
const { createHttpSocketRuntimeEngineAdapter } = require('./http_socket_runtime_adapter.ts');

function cleanText(value, max = 500) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function parseVersionParts(value) {
  return String(value || '')
    .replace(/^v/i, '')
    .split('.')
    .map((part) => Number.parseInt(part, 10))
    .map((part) => (Number.isFinite(part) ? part : 0));
}

function versionAtLeast(current, required) {
  const cur = parseVersionParts(current);
  const req = parseVersionParts(required);
  const length = Math.max(cur.length, req.length, 3);
  for (let index = 0; index < length; index += 1) {
    const left = cur[index] || 0;
    const right = req[index] || 0;
    if (left > right) return true;
    if (left < right) return false;
  }
  return true;
}

function referenceCheckoutOpenClawEntrypoint() {
  const candidate = path.resolve(process.cwd(), 'references/coding-agent-systems/openclaw/openclaw.mjs');
  try {
    return fs.existsSync(candidate) ? candidate : '';
  } catch {
    return '';
  }
}

function openClawReferenceReadiness() {
  const entrypoint = referenceCheckoutOpenClawEntrypoint();
  if (!entrypoint) return null;
  const requiredNode = '22.19.0';
  const currentNode = process.versions.node;
  const nodeOk = versionAtLeast(currentNode, requiredNode);
  return {
    status: nodeOk ? 'reference_checkout_entrypoint_available' : 'runtime_requirement_missing',
    command: entrypoint,
    resolved_path: entrypoint,
    discovery_source: 'reference_checkout_probe',
    runtime_requirement: `node>=${requiredNode}`,
    current_runtime: `node=${currentNode}`,
    reason: nodeOk
      ? 'OpenClaw reference checkout exposes openclaw.mjs; configure or start its Gateway/socket before live turns.'
      : `OpenClaw reference checkout exposes openclaw.mjs, but requires Node ${requiredNode}+ while this Gateway is running Node ${currentNode}.`,
    version_preview: nodeOk
      ? 'OpenClaw reference checkout entrypoint detected.'
      : `OpenClaw requires Node ${requiredNode}+; current Node is ${currentNode}.`,
  };
}

function createOpenClawEngineAdapter(options = {}) {
  const httpAdapter = createHttpSocketRuntimeEngineAdapter({
    ...options,
    engineId: 'openclaw',
    engineKind: 'external_framework_adapter',
    downloadActionRef: 'agent_runtime_download/openclaw',
    contextTransportMode: 'structured_json',
    structuredTransportTarget: 'structured_json',
    transportMigrationStatus: 'planned_structured_target',
    healthProbes: [
      { method: 'GET', path: '/health' },
      { method: 'GET', path: '/api/health' },
      { method: 'GET', path: '/status' },
      { method: 'GET', path: '/api/status' },
    ],
  });
  return {
    ...httpAdapter,
    async health_check(ctx) {
      const httpHealth = await httpAdapter.health_check(ctx);
      if (httpHealth && httpHealth.status === 'available') return httpHealth;
      const reference = openClawReferenceReadiness();
      if (!reference) return httpHealth;
      return {
        ...(httpHealth || {}),
        status: reference.status,
        command: reference.command,
        resolved_path: reference.resolved_path,
        discovery_source: reference.discovery_source,
        runtime_requirement: reference.runtime_requirement,
        current_runtime: reference.current_runtime,
        reason: cleanText(reference.reason, 500),
        version_preview: cleanText(reference.version_preview, 240),
        download_available: false,
        provider_readiness: reference.status === 'runtime_requirement_missing' ? 'runtime_requirement_missing' : 'not_checked',
        error_code: reference.status === 'runtime_requirement_missing' ? 'openclaw_runtime_requirement_missing' : '',
        retryable: true,
      };
    },
  };
}

module.exports = {
  createOpenClawEngineAdapter,
};
