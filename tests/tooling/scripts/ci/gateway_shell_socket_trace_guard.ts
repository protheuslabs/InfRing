#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'validation/conformance/contracts/gateway_shell_socket_trace_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/gateway_shell_socket_trace_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/GATEWAY_SHELL_SOCKET_TRACE_GUARD_CURRENT.md';

function argValue(name, fallback) {
  const prefix = `--${name}=`;
  const found = process.argv.find((arg) => arg.startsWith(prefix));
  return found ? found.slice(prefix.length) : fallback;
}

function boolArg(name, fallback = false) {
  const value = argValue(name, null);
  if (value == null) return fallback;
  return value === '1' || value === 'true' || value === 'yes';
}

function readJson(rel) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, rel), 'utf8'));
}

function readText(rel) {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function exists(rel) {
  return fs.existsSync(path.join(ROOT, rel));
}

function writeJson(rel, payload) {
  const abs = path.join(ROOT, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

function writeMarkdown(rel, payload) {
  const abs = path.join(ROOT, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  const lines = [
    '# Gateway Shell Socket Trace Guard',
    '',
    `- Generated: ${payload.generated_at}`,
    `- Trace ID: ${payload.trace_id}`,
    `- Contract: ${payload.contract_path}`,
    `- Checked modules: ${payload.checked_modules.length}`,
    `- Violations: ${payload.violations.length}`,
    '',
  ];
  if (payload.violations.length) {
    lines.push('| Kind | Module | Detail |', '|---|---|---|');
    for (const violation of payload.violations) {
      lines.push(`| ${escapeCell(violation.kind)} | ${escapeCell(violation.path || violation.id || '')} | ${escapeCell(violation.detail || violation.token || '')} |`);
    }
  } else {
    lines.push('No trace bridge violations found.');
  }
  fs.writeFileSync(abs, `${lines.join('\n')}\n`);
}

function escapeCell(value) {
  return String(value == null ? '' : value).replace(/\\/g, '\\\\').replace(/\|/g, '\\|').replace(/\n/g, ' ');
}

function includesAll(source, tokens, module, violations, kind) {
  for (const token of tokens) {
    if (!source.includes(token)) {
      violations.push({ kind, id: module.id, path: module.path, token });
    }
  }
}

function forbiddenPresent(source, tokens, module, violations) {
  for (const token of tokens) {
    if (source.includes(`${token}:`) || source.includes(`'${token}'`) || source.includes(`"${token}"`)) {
      violations.push({
        kind: 'forbidden_default_payload_field_present',
        id: module.id,
        path: module.path,
        token,
        detail: 'Bridge module must not expose raw runtime/default Shell payload fields.',
      });
    }
  }
}

const contractPath = argValue('contract', DEFAULT_CONTRACT);
const outJson = argValue('out-json', DEFAULT_OUT_JSON);
const outMarkdown = argValue('out-markdown', DEFAULT_OUT_MARKDOWN);
const strict = boolArg('strict', true);
const violations = [];

if (!exists(contractPath)) {
  violations.push({ kind: 'contract_missing', path: contractPath });
}

const contract = exists(contractPath) ? readJson(contractPath) : {};
const modules = Array.isArray(contract.bridge_modules) ? contract.bridge_modules : [];
if (!modules.length) {
  violations.push({ kind: 'bridge_modules_missing', path: contractPath });
}

const commonFields = Array.isArray(contract.required_common_fields) ? contract.required_common_fields : [];
const projectionFields = Array.isArray(contract.projection_bridge_requirements?.required_metadata_fields)
  ? contract.projection_bridge_requirements.required_metadata_fields
  : [];
const ingressFields = Array.isArray(contract.ingress_bridge_requirements?.required_metadata_fields)
  ? contract.ingress_bridge_requirements.required_metadata_fields
  : [];
const forbiddenProjection = Array.isArray(contract.projection_bridge_requirements?.forbidden_authority_shape)
  ? contract.projection_bridge_requirements.forbidden_authority_shape
  : [];
const forbiddenIngress = Array.isArray(contract.ingress_bridge_requirements?.forbidden_raw_context_fields)
  ? contract.ingress_bridge_requirements.forbidden_raw_context_fields
  : [];

const checked = [];
for (const module of modules) {
  if (!module.path || !module.id || !module.kind) {
    violations.push({ kind: 'bridge_module_contract_row_incomplete', id: module.id || '', path: module.path || '' });
    continue;
  }
  const row = { id: module.id, path: module.path, kind: module.kind, exists: exists(module.path) };
  checked.push(row);
  if (!row.exists) {
    violations.push({ kind: 'bridge_module_missing', id: module.id, path: module.path });
    continue;
  }
  const source = readText(module.path);
  includesAll(source, commonFields, module, violations, 'common_trace_field_missing');
  if (module.kind === 'projection') {
    includesAll(source, [module.expected_metadata_envelope || 'gateway_projection', ...projectionFields], module, violations, 'projection_trace_field_missing');
    if (!source.includes('bounded: true')) {
      violations.push({ kind: 'projection_not_marked_bounded', id: module.id, path: module.path });
    }
    forbiddenPresent(source, forbiddenProjection, module, violations);
  } else if (module.kind === 'ingress') {
    includesAll(source, [module.expected_metadata_envelope || 'gateway_ingress', ...ingressFields], module, violations, 'ingress_trace_field_missing');
    if (!source.includes('byteSize(') || !source.includes('65536')) {
      violations.push({ kind: 'ingress_byte_budget_missing', id: module.id, path: module.path });
    }
    if (!source.includes("'x-infring-trace-id': traceId")) {
      violations.push({ kind: 'ingress_upstream_trace_header_missing', id: module.id, path: module.path });
    }
    forbiddenPresent(source, forbiddenIngress.filter((field) => field.startsWith('raw_') || field === 'conversation_tree'), module, violations);
  } else {
    violations.push({ kind: 'bridge_module_kind_unknown', id: module.id, path: module.path, detail: module.kind });
  }
}

const generatedAt = new Date().toISOString();
const payload = {
  ok: violations.length === 0,
  type: 'gateway_shell_socket_trace_guard',
  generated_at: generatedAt,
  trace_id: `validation:${generatedAt}:gateway-shell-socket-trace-guard`,
  span_id: `span:gateway-shell-socket-trace-guard:${Date.now()}`,
  parent_span_id: null,
  source_domain: 'validation',
  contract_path: contractPath,
  checked_modules: checked,
  violations,
};

writeJson(outJson, payload);
writeMarkdown(outMarkdown, payload);
console.log(JSON.stringify(payload, null, 2));
if (strict && !payload.ok) process.exit(1);
