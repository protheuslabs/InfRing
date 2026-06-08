#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { invokeKernelPayload, kernelFailClosedResult } = require('../../lib/infring_kernel_bridge.ts');

const KNOWN_COMMANDS = new Set([
  'status',
  'parse-cli',
  'command-name',
  'validate',
  'validate-ranking',
  'validate-lensmap',
  'severity-rank',
  'guard-failure',
]);

function invoke(command, payload = {}, opts = {}) {
  return invokeKernelPayload(
    'memory-policy-kernel',
    command,
    payload,
    {
      throwOnError: opts.throwOnError,
      fallbackError: 'memory_policy_kernel_bridge_failed',
    }
  );
}

function cleanText(raw, maxLen = 160) {
  return String(raw || '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function parseFlag(args = [], key) {
  const list = Array.isArray(args) ? args.map((token) => String(token || '')) : [];
  const inline = list.find((token) => token.startsWith(`${key}=`));
  if (inline) return inline.slice(key.length + 1).trim();
  const idx = list.findIndex((token) => token === key);
  if (idx >= 0 && idx + 1 < list.length) return list[idx + 1].trim();
  return '';
}

function parseJson(value, fallback) {
  const raw = String(value || '').trim();
  if (!raw) return fallback;
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

function validationResult(ok, reasonCode, extra = {}) {
  return {
    ok,
    type: 'memory_policy_validation',
    reason_code: reasonCode,
    ...extra,
  };
}

function preflightValidateMemoryPolicy(args = []) {
  const command = commandNameFromArgs(args, 'status');
  if (command !== 'query-index') return null;

  if (parseFlag(args, '--bypass') === '1' || parseFlag(args, '--allow-full-scan') === '1') {
    return validationResult(false, 'index_first_bypass_forbidden');
  }

  const directPath = parseFlag(args, '--path') || parseFlag(args, '--file');
  if (directPath && /(?:^|\/)local\/workspace\/memory\//.test(directPath)) {
    return validationResult(false, 'direct_file_read_forbidden');
  }

  if (parseFlag(args, '--bootstrap') === '1' && parseFlag(args, '--lazy-hydration') === '0') {
    return validationResult(false, 'bootstrap_requires_lazy_hydration');
  }

  const burnThreshold = Number(parseFlag(args, '--burn-threshold'));
  if (Number.isFinite(burnThreshold) && burnThreshold > 200) {
    return validationResult(false, 'burn_slo_threshold_exceeded');
  }

  const top = Number(parseFlag(args, '--top'));
  if (Number.isFinite(top) && top > 50) {
    return validationResult(false, 'recall_budget_exceeded');
  }

  if (parseFlag(args, '--allow-stale') === '1') {
    return validationResult(false, 'stale_override_forbidden');
  }

  const scores = parseJson(parseFlag(args, '--scores-json'), []);
  if (Array.isArray(scores) && scores.length > 1) {
    for (let index = 1; index < scores.length; index += 1) {
      if (Number(scores[index]) > Number(scores[index - 1])) {
        return validationResult(false, 'ranking_not_descending');
      }
    }
  }

  const annotation = parseJson(parseFlag(args, '--lensmap-annotation-json'), null);
  if (annotation && typeof annotation === 'object') {
    const tags = Array.isArray(annotation.tags) ? annotation.tags : [];
    const jots = Array.isArray(annotation.jots) ? annotation.jots : [];
    if (tags.length === 0 || jots.length === 0) {
      return validationResult(false, 'lensmap_annotation_missing_tags_or_jots');
    }
    return validationResult(true, 'policy_ok');
  }

  return null;
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

function parseCliArgs(args = [], options = {}) {
  const out = invoke(
    'parse-cli',
    { args, options: options && typeof options === 'object' ? options : {} },
    { throwOnError: false }
  );
  return out.parsed && typeof out.parsed === 'object'
    ? out.parsed
    : { positional: [], flags: {} };
}

function commandNameFromArgs(args = [], fallback = 'status') {
  const out = invoke('command-name', { args, fallback }, { throwOnError: false });
  return String(out.command || fallback).trim().toLowerCase();
}

function validateDescendingRanking(scores = [], ids = []) {
  const out = invoke('validate-ranking', { scores, ids }, { throwOnError: false });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, reason_code: 'ranking_validation_failed' };
}

function validateLensMapAnnotation(annotation) {
  const out = invoke('validate-lensmap', { annotation }, { throwOnError: false });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, reason_code: 'lensmap_annotation_invalid' };
}

function severityRank(raw) {
  const out = invoke('severity-rank', { value: raw }, { throwOnError: false });
  return Number.isFinite(Number(out.rank)) ? Number(out.rank) : 0;
}

function validateMemoryPolicy(args = [], options = {}) {
  const preflight = preflightValidateMemoryPolicy(args);
  if (preflight) return preflight;

  const out = invoke(
    'validate',
    {
      args: Array.isArray(args) ? args : [],
      options: options && typeof options === 'object' ? options : {}
    },
    { throwOnError: false }
  );
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, type: 'memory_policy_validation', reason_code: 'policy_validation_failed' };
}

function guardFailureResult(validation, context = {}) {
  if (validation && typeof validation.reason_code === 'string' && validation.reason_code.trim()) {
    return kernelFailClosedResult(
      'memory_policy_guard_reject',
      validation.reason_code,
      {
        stderrPrefix: 'memory_policy_guard_reject',
        payload: {
          layer: 'kernel_memory_policy_guard',
          ...(context && typeof context === 'object' ? context : {}),
        },
      }
    );
  }

  const out = invoke(
    'guard-failure',
    {
      validation: validation && typeof validation === 'object' ? validation : {},
      context: context && typeof context === 'object' ? context : {}
    },
    { throwOnError: false }
  );
  return out.result && typeof out.result === 'object'
    ? out.result
    : kernelFailClosedResult(
        'memory_policy_guard_reject',
        String(
          validation && typeof validation.reason_code === 'string'
            ? validation.reason_code
            : 'policy_validation_failed'
        ),
        {
          stderrPrefix: 'memory_policy_guard_reject',
          payload: { layer: 'kernel_memory_policy_guard' }
        }
      );
}

function usagePayload(command) {
  return {
    ok: false,
    type: 'memory_policy_validator_usage',
    error: 'unknown_command',
    command: cleanText(command || 'status', 80),
    usage: [
      'policy_validator.ts status',
      'policy_validator.ts parse-cli [flags]',
      'policy_validator.ts command-name [flags]',
      'policy_validator.ts validate [flags]',
      'policy_validator.ts validate-ranking --scores-json=[...] --ids-json=[...]',
      'policy_validator.ts validate-lensmap --annotation-json={...}',
      'policy_validator.ts severity-rank --value=<critical|high|medium|low>',
      'policy_validator.ts guard-failure [validate flags] [--validation-json={...}] [--context-json={...}]'
    ]
  };
}

function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
  const command = commandNameFromArgs(args, 'status');
  const rest = args.length > 0 ? args.slice(1) : [];
  if (!KNOWN_COMMANDS.has(command)) {
    const payload = usagePayload(command);
    printPayload(payload);
    return 2;
  }

  let payload;
  switch (command) {
    case 'status':
      payload = invoke('status', {}, { throwOnError: false });
      break;
    case 'parse-cli':
      payload = {
        ok: true,
        type: 'memory_policy_parse_cli',
        parsed: parseCliArgs(rest, {})
      };
      break;
    case 'command-name':
      payload = {
        ok: true,
        type: 'memory_policy_command_name',
        command: commandNameFromArgs(rest, 'status')
      };
      break;
    case 'validate':
      payload = validateMemoryPolicy(rest, {});
      break;
    case 'validate-ranking': {
      const scores = parseJson(parseFlag(rest, '--scores-json'), []);
      const ids = parseJson(parseFlag(rest, '--ids-json'), []);
      payload = validateDescendingRanking(
        Array.isArray(scores) ? scores : [],
        Array.isArray(ids) ? ids : []
      );
      break;
    }
    case 'validate-lensmap': {
      const annotation = parseJson(parseFlag(rest, '--annotation-json'), {});
      payload = validateLensMapAnnotation(annotation);
      break;
    }
    case 'severity-rank': {
      const raw = parseFlag(rest, '--value') || (rest[0] || '');
      payload = {
        ok: true,
        type: 'memory_policy_severity_rank',
        value: cleanText(raw, 80),
        rank: severityRank(raw)
      };
      break;
    }
    case 'guard-failure': {
      const validationArg = parseJson(parseFlag(rest, '--validation-json'), null);
      const context = parseJson(parseFlag(rest, '--context-json'), {});
      const validation =
        validationArg && typeof validationArg === 'object'
          ? validationArg
          : validateMemoryPolicy(rest.filter((token) => !token.startsWith('--validation-json=')), {});
      payload = guardFailureResult(validation, context);
      break;
    }
    default:
      payload = usagePayload(command);
      break;
  }

  printPayload(payload);
  if (command === 'validate' || command === 'validate-ranking' || command === 'validate-lensmap') {
    return statusCodeForPayload(payload, 2);
  }
  return statusCodeForPayload(payload, 1);
}

const DEFAULT_POLICY = (() => {
  const status = invoke('status', {}, { throwOnError: false });
  const candidate = status.default_policy && typeof status.default_policy === 'object'
    ? status.default_policy
    : {};
  return Object.freeze({
    index_first_required: candidate.index_first_required !== false,
    max_burn_slo_tokens: Number.isFinite(Number(candidate.max_burn_slo_tokens))
      ? Number(candidate.max_burn_slo_tokens)
      : 200,
    max_recall_top: Number.isFinite(Number(candidate.max_recall_top))
      ? Number(candidate.max_recall_top)
      : 50,
    max_max_files: Number.isFinite(Number(candidate.max_max_files))
      ? Number(candidate.max_max_files)
      : 20,
    max_expand_lines: Number.isFinite(Number(candidate.max_expand_lines))
      ? Number(candidate.max_expand_lines)
      : 300,
    bootstrap_hydration_token_cap: Number.isFinite(Number(candidate.bootstrap_hydration_token_cap))
      ? Number(candidate.bootstrap_hydration_token_cap)
      : 48,
    block_stale_override: candidate.block_stale_override !== false
  });
})();

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  DEFAULT_POLICY,
  run,
  parseCliArgs,
  commandNameFromArgs,
  validateMemoryPolicy,
  guardFailureResult,
  validateDescendingRanking,
  validateLensMapAnnotation,
  severityRank
};
