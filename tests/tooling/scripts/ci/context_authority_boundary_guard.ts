#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const contractPath = 'validation/conformance/contracts/context_authority_boundary_contract.json';
const packContractPath = 'validation/conformance/contracts/agent_runtime_context_pack_contract.json';
const outPath = 'core/local/artifacts/context_authority_boundary_guard_current.json';
const violations = [];

function abs(rel) {
  return path.join(ROOT, rel);
}

function exists(rel) {
  return fs.existsSync(abs(rel));
}

function read(rel) {
  return fs.readFileSync(abs(rel), 'utf8');
}

function readJson(rel) {
  return JSON.parse(read(rel));
}

function push(kind, detail = {}) {
  violations.push({ kind, ...detail });
}

function includesAll(source, pathRel, needles) {
  for (const needle of needles) {
    if (!source.includes(needle)) push('required_marker_missing', { path: pathRel, marker: needle });
  }
}

const contract = readJson(contractPath);
const packContract = readJson(packContractPath);

if (contract.type !== 'context_authority_boundary_contract') push('contract_type_wrong', { path: contractPath });
if (contract.canonical_authority?.owner !== 'kernel.layer2.memory') push('canonical_context_authority_owner_wrong', { path: contractPath });
if (contract.canonical_authority?.source_basis !== 'core.layer2.memory.context_topology') push('canonical_context_source_basis_wrong', { path: contractPath });
if (!packContract.context_authority_boundary_contract || packContract.context_authority_boundary_contract !== contractPath) {
  push('context_pack_contract_not_linked_to_boundary_contract', { path: packContractPath });
}
if (packContract.authority_boundary?.kernel_owns_context_truth !== true) push('context_pack_kernel_truth_not_declared', { path: packContractPath });
if (packContract.authority_boundary?.gateway_cache_is_authoritative !== false) push('gateway_cache_authority_not_denied', { path: packContractPath });
if (packContract.live_slice_policy?.production_requires_kernel_materializer !== true) push('production_kernel_materializer_not_required', { path: packContractPath });
if (packContract.live_slice_policy?.workspace_dev_may_use_kernel_materializer_cargo_auto !== true) push('workspace_kernel_materializer_cargo_auto_not_declared', { path: packContractPath });
if (packContract.engine_switch_continuity_rule?.context_continuity_eval_required !== true) push('context_continuity_eval_not_required', { path: packContractPath });
if (!packContract.engine_switch_continuity_rule?.continuity_eval_artifact) push('context_continuity_eval_artifact_missing', { path: packContractPath });
if (packContract.live_slice_policy?.gateway_context_store_is_transitional !== true) push('gateway_context_store_not_marked_transitional', { path: packContractPath });
if (!packContract.live_slice_policy?.gateway_context_store_retirement_trigger) push('gateway_context_store_retirement_trigger_missing', { path: packContractPath });

const authorityPaths = contract.canonical_authority?.allowed_authority_paths || [];
for (const rel of authorityPaths) {
  if (!exists(rel)) push('canonical_authority_path_missing', { path: rel });
}
for (const op of ['append_context_atom', 'materialize_context_topology']) {
  if (!Array.isArray(contract.canonical_authority?.required_operations) || !contract.canonical_authority.required_operations.includes(op)) {
    push('canonical_context_operation_missing', { operation: op, path: contractPath });
  }
}

const gatewayBridge = 'adapters/runtime/agent_engines/agent_runtime_kernel_context_bridge.ts';
const gatewayCache = 'adapters/runtime/agent_engines/agent_runtime_context_store.ts';
const cliAdapter = 'adapters/runtime/agent_engines/cli_runtime_adapter.ts';
const dashboard = 'adapters/runtime/infring_dashboard.ts';
const kernelMaterializer = 'core/layer2/memory/src/bin/agent_runtime_context_materializer.rs';
const oldContextStacks = 'core/layer0/ops/src/memory/stacks';

for (const rel of [gatewayBridge, gatewayCache, cliAdapter, dashboard, kernelMaterializer]) {
  if (!exists(rel)) push('required_context_boundary_file_missing', { path: rel });
}

if (exists(gatewayCache)) {
  const source = read(gatewayCache);
  includesAll(source, gatewayCache, [
    'TRANSITIONAL FALLBACK ONLY',
    'CONTEXT_STORE_AUTHORITY_CLASSIFICATION',
    "role: 'transitional_gateway_cache'",
    'canonical_authority: false',
    "canonical_authority_owner: 'kernel.layer2.memory'",
    "fallback_only_when: 'kernel_context_materializer_binary_unavailable'",
    "retirement_trigger: 'kernel_materializer_packaged_for_runtime'",
  ]);
}

if (exists(gatewayBridge)) {
  const source = read(gatewayBridge);
  includesAll(source, gatewayBridge, [
    'does not implement context semantics itself',
    'resolveKernelMaterializerCommand',
    'materializeKernelAgentRuntimeContextPack',
    'agent_runtime_context_materializer',
    "INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO || 'auto'",
  ]);
  if (!source.includes('INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_BIN') || !source.includes('INFRING_AGENT_RUNTIME_CONTEXT_KERNEL_CARGO')) {
    push('kernel_context_bridge_discovery_controls_missing', { path: gatewayBridge });
  }
}

if (exists(dashboard)) {
  const source = read(dashboard);
  if (!source.includes('materializeKernelAgentRuntimeContextPack')) push('dashboard_kernel_context_bridge_missing', { path: dashboard });
  if (!source.includes('materializeAgentRuntimeContextPack')) push('dashboard_gateway_context_fallback_missing', { path: dashboard });
  if (!source.includes('kernelContext.ok') || !source.includes('kernelContext.context_pack')) {
    push('dashboard_kernel_context_result_not_preferred', { path: dashboard });
  }
  if (!source.includes('kernel_materializer_used')) push('dashboard_context_authority_projection_missing', { path: dashboard });
}

if (exists(kernelMaterializer)) {
  const source = read(kernelMaterializer);
  includesAll(source, kernelMaterializer, [
    'append_context_atom',
    'materialize_context_topology',
    'kernel_materialize_context_topology_cli',
    'core.layer2.memory.context_topology',
  ]);
}

const legacy = contract.legacy_context_systems?.[oldContextStacks];
if (exists(oldContextStacks)) {
  if (!legacy) push('legacy_context_stacks_not_classified', { path: oldContextStacks });
  if (legacy?.runtime_engine_context_authority_allowed !== false) push('legacy_context_stacks_runtime_engine_authority_not_denied', { path: oldContextStacks });
  if (legacy?.classification !== 'non_canonical_legacy_operator_cache') push('legacy_context_stacks_classification_wrong', { path: oldContextStacks, actual: legacy?.classification });
}

const allowedNonAuthorityPaths = new Set();
for (const role of Object.values(contract.allowed_non_authority_roles || {})) {
  for (const rel of role.allowed_paths || []) allowedNonAuthorityPaths.add(rel);
}
for (const required of [gatewayBridge, gatewayCache, cliAdapter]) {
  if (!allowedNonAuthorityPaths.has(required)) push('non_authority_context_role_path_missing', { path: required });
}

if (contract.production_invariants?.kernel_materializer_is_required_for_production !== true) push('production_kernel_materializer_invariant_missing', { path: contractPath });
if (contract.production_invariants?.workspace_dev_may_auto_run_cargo_materializer !== true) push('workspace_cargo_materializer_invariant_missing', { path: contractPath });
if (contract.production_invariants?.fallback_must_emit_transitional_reason !== true) push('fallback_reason_invariant_missing', { path: contractPath });
if (contract.production_invariants?.fallback_must_have_retirement_trigger !== true) push('fallback_retirement_invariant_missing', { path: contractPath });

const artifact = {
  ok: violations.length === 0,
  guard: 'context_authority_boundary_guard',
  generated_at: new Date().toISOString(),
  contract: contractPath,
  context_pack_contract: packContractPath,
  violations,
};
fs.mkdirSync(path.dirname(abs(outPath)), { recursive: true });
fs.writeFileSync(abs(outPath), `${JSON.stringify(artifact, null, 2)}\n`);
if (!artifact.ok) {
  console.error(JSON.stringify(artifact, null, 2));
  process.exit(1);
}
console.log(JSON.stringify(artifact, null, 2));
