#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const fs = require('node:fs');
const path = require('node:path');
const { invokeKernelPayload, kernelFailClosedResult } = require('../../lib/infring_kernel_bridge.ts');

const SESSION_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const DEFAULT_STATE_PATH = 'client/runtime/local/state/memory/session_isolation.json';

function invoke(command, payload = {}, opts = {}) {
  return invokeKernelPayload(
    'memory-session-isolation-kernel',
    command,
    payload,
    {
      throwOnError: opts.throwOnError,
      fallbackError: `memory_session_isolation_kernel_${command}_bridge_failed`,
    }
  );
}

function cleanText(raw, maxLen = 160) {
  return String(raw || '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function normalizeArgs(args = []) {
  return Array.isArray(args) ? args.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function parseFlag(args = [], key) {
  const list = normalizeArgs(args);
  const inline = list.find((token) => token.startsWith(`${key}=`));
  if (inline) return inline.slice(key.length + 1).trim();
  const idx = list.findIndex((token) => token === key);
  if (idx >= 0 && idx + 1 < list.length) return list[idx + 1].trim();
  return '';
}

function parseJson(raw, fallback) {
  const value = String(raw || '').trim();
  if (!value) return fallback;
  try {
    return JSON.parse(value);
  } catch {
    return fallback;
  }
}

function statusCodeForPayload(payload, fallback = 1) {
  if (payload && Number.isFinite(Number(payload.status))) {
    return Number(payload.status);
  }
  if (payload && typeof payload.ok === 'boolean') {
    return payload.ok ? 0 : fallback;
  }
  return fallback;
}

function printPayload(payload) {
  if (payload && typeof payload.stdout === 'string' && payload.stdout.length > 0) {
    process.stdout.write(payload.stdout.endsWith('\n') ? payload.stdout : `${payload.stdout}\n`);
  }
  if (payload && typeof payload.stderr === 'string' && payload.stderr.length > 0) {
    process.stderr.write(payload.stderr.endsWith('\n') ? payload.stderr : `${payload.stderr}\n`);
  }
  if (
    payload
    && payload.payload
    && typeof payload.payload === 'object'
    && !(typeof payload.stdout === 'string' && payload.stdout.length > 0)
  ) {
    process.stdout.write(`${JSON.stringify(payload.payload)}\n`);
    return;
  }
  if (
    payload
    && typeof payload === 'object'
    && !Array.isArray(payload)
    && typeof payload.stdout !== 'string'
    && typeof payload.stderr !== 'string'
  ) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
  }
}

function loadState(filePath = DEFAULT_STATE_PATH) {
  const out = invoke(
    'load-state',
    {
      state_path: String(filePath || DEFAULT_STATE_PATH),
      statePath: String(filePath || DEFAULT_STATE_PATH)
    },
    { throwOnError: false }
  );
  return out.state && typeof out.state === 'object'
    ? out.state
    : {
        schema_version: '1.0',
        resources: {}
      };
}

function saveState(state, filePath = DEFAULT_STATE_PATH) {
  const out = invoke(
    'save-state',
    {
      state: state && typeof state === 'object' ? state : {},
      state_path: String(filePath || DEFAULT_STATE_PATH),
      statePath: String(filePath || DEFAULT_STATE_PATH)
    },
    { throwOnError: false }
  );
  return out.state && typeof out.state === 'object'
    ? out.state
    : {
        schema_version: '1.0',
        resources: {}
      };
}

function localLoadState(filePath = DEFAULT_STATE_PATH) {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(filePath), 'utf8'));
  } catch {
    return { schema_version: '1.0', resources: {} };
  }
}

function localSaveState(state, filePath = DEFAULT_STATE_PATH) {
  const abs = path.resolve(filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(state, null, 2)}\n`, 'utf8');
  return state;
}

function preflightValidateSessionIsolation(args = [], options = {}) {
  const list = normalizeArgs(args);
  const sessionId = parseFlag(list, '--session-id');
  if (!sessionId) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'missing_session_id',
    };
  }
  if (!SESSION_ID_PATTERN.test(sessionId)) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'invalid_session_id',
    };
  }

  const resourceId = parseFlag(list, '--resource-id');
  if (!resourceId) {
    return null;
  }

  const statePath = String(options.statePath || options.state_path || DEFAULT_STATE_PATH);
  const state = localLoadState(statePath);
  if (!state.resources || typeof state.resources !== 'object') state.resources = {};
  const owner = state.resources[resourceId];
  if (typeof owner === 'string' && owner !== sessionId) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'cross_session_leak_blocked',
      resource_id: resourceId,
      owner_session_id: owner,
      session_id: sessionId,
    };
  }
  if (!owner) {
    state.resources[resourceId] = sessionId;
    localSaveState(state, statePath);
  }
  return {
    ok: true,
    type: 'memory_session_isolation',
    reason_code: 'session_isolation_ok',
    session_id: sessionId,
    resource_id: resourceId,
  };
}

function validateSessionIsolation(args = [], options = {}) {
  const normalizedOptions = options && typeof options === 'object' ? { ...options } : {};
  const statePath = String(
    normalizedOptions.statePath || normalizedOptions.state_path || DEFAULT_STATE_PATH
  );
  if (!normalizedOptions.statePath) normalizedOptions.statePath = statePath;
  if (!normalizedOptions.state_path) normalizedOptions.state_path = statePath;

  const preflight = preflightValidateSessionIsolation(args, normalizedOptions);
  if (preflight) return preflight;

  const out = invoke(
    'validate',
    {
      args: Array.isArray(args) ? args.map((row) => String(row)) : [],
      options: normalizedOptions
    },
    { throwOnError: false }
  );
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : {
        ok: false,
        type: 'memory_session_isolation',
        reason_code: 'session_isolation_failed'
      };
}

function sessionFailureResult(validation, context = {}) {
  if (validation && typeof validation.reason_code === 'string' && validation.reason_code.trim()) {
    return kernelFailClosedResult(
      'memory_session_isolation_reject',
      validation.reason_code,
      {
        stderrPrefix: 'memory_session_isolation_reject',
        payload: context && typeof context === 'object' ? context : {},
      }
    );
  }

  const out = invoke(
    'failure-result',
    {
      validation: validation && typeof validation === 'object' ? validation : {},
      context: context && typeof context === 'object' ? context : {}
    },
    { throwOnError: false }
  );
  return out.result && typeof out.result === 'object'
    ? out.result
    : kernelFailClosedResult(
        'memory_session_isolation_reject',
        validation && typeof validation.reason_code === 'string'
          ? validation.reason_code
          : 'session_isolation_failed',
        { stderrPrefix: 'memory_session_isolation_reject' }
      );
}

function usagePayload(command) {
  return {
    ok: false,
    type: 'memory_session_isolation_usage',
    error: 'unknown_command',
    command: cleanText(command || 'status', 80),
    usage: [
      'session_isolation.ts status [--state-path=<path>]',
      'session_isolation.ts load-state [--state-path=<path>]',
      'session_isolation.ts save-state --state-json=<json> [--state-path=<path>]',
      'session_isolation.ts validate <runtime-args> [--state-path=<path>]',
      'session_isolation.ts failure-result <runtime-args> [--validation-json=<json>] [--context-json=<json>] [--state-path=<path>]',
    ]
  };
}

function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const command = (args[0] && !args[0].startsWith('-') ? args[0] : 'status').toLowerCase();
  const rest = args[0] && !args[0].startsWith('-') ? args.slice(1) : args;
  const statePath =
    parseFlag(rest, '--state-path')
    || parseFlag(rest, '--statePath')
    || DEFAULT_STATE_PATH;

  let payload;
  switch (command) {
    case 'status':
    case 'load-state':
      payload = {
        ok: true,
        type: 'memory_session_isolation_state',
        state_path: statePath,
        state: loadState(statePath),
      };
      break;
    case 'save-state': {
      const state = parseJson(parseFlag(rest, '--state-json'), {});
      payload = {
        ok: true,
        type: 'memory_session_isolation_state_saved',
        state_path: statePath,
        state: saveState(state, statePath),
      };
      break;
    }
    case 'validate':
      payload = validateSessionIsolation(rest, { statePath });
      break;
    case 'failure-result': {
      const rawValidation = parseJson(parseFlag(rest, '--validation-json'), null);
      const rawContext = parseJson(parseFlag(rest, '--context-json'), {});
      const validation =
        rawValidation && typeof rawValidation === 'object'
          ? rawValidation
          : validateSessionIsolation(rest, { statePath });
      payload = sessionFailureResult(validation, rawContext && typeof rawContext === 'object' ? rawContext : {});
      break;
    }
    default:
      payload = usagePayload(command);
      break;
  }

  printPayload(payload);
  if (command === 'validate') return statusCodeForPayload(payload, 2);
  return statusCodeForPayload(payload, 1);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  SESSION_ID_PATTERN,
  DEFAULT_STATE_PATH,
  run,
  loadState,
  saveState,
  validateSessionIsolation,
  sessionFailureResult
};
