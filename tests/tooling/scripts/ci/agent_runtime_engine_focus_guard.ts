#!/usr/bin/env node
/* eslint-disable no-console */
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = process.cwd();
const REGISTRY_PATH = 'validation/conformance/contracts/agent_runtime_engine_registry.json';
const STRATEGY_DOC_PATH = 'docs/workspace/agent_runtime_engine_strategy.md';
const LIVE_WORK_EVAL_PATH = 'tests/tooling/scripts/ci/agent_runtime_live_work_eval.ts';
const COORDINATION_GUARD_PATH = 'tests/tooling/scripts/ci/agent_runtime_framework_coordination_guard.ts';
const OUT_JSON = 'core/local/artifacts/agent_runtime_engine_focus_guard_current.json';

function read(rel) {
  return fs.readFileSync(path.join(ROOT, rel), 'utf8');
}

function exists(rel) {
  return fs.existsSync(path.join(ROOT, rel));
}

function json(rel) {
  return JSON.parse(read(rel));
}

function clean(value, max = 1000) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, max);
}

function push(list, kind, file, detail) {
  list.push({ kind, file, detail: clean(detail, 2000) });
}

function unique(values) {
  const out = [];
  const seen = new Set();
  for (const value of Array.isArray(values) ? values : []) {
    const item = clean(value, 160);
    if (!item || seen.has(item)) continue;
    seen.add(item);
    out.push(item);
  }
  return out;
}

function difference(left, right) {
  const rightSet = new Set(right);
  return left.filter((item) => !rightSet.has(item));
}

const violations = [];
const registry = exists(REGISTRY_PATH) ? json(REGISTRY_PATH) : {};
const engines = Array.isArray(registry.engines) ? registry.engines : [];
const engineIds = unique(engines.map((row) => row && row.engine_id));
const focus = registry.validation_focus_policy && typeof registry.validation_focus_policy === 'object'
  ? registry.validation_focus_policy
  : {};
const active = unique(focus.active_promotion_engines);
const primaryExternal = unique(
  (
    Array.isArray(focus.primary_external_engines)
      ? focus.primary_external_engines
      : []
  ).concat(
    focus.primary_external_engine ? [focus.primary_external_engine] : []
  )
);
const catalogOnly = unique(focus.catalog_only_engines);
const secondary = unique(focus.secondary_comparison_engines);
const goldenPair = registry.golden_pair_baseline && typeof registry.golden_pair_baseline === 'object'
  ? registry.golden_pair_baseline
  : {};
const goldenPairEngines = unique(goldenPair.engines);
const liveSmoke = registry.golden_pair_live_smoke && typeof registry.golden_pair_live_smoke === 'object'
  ? registry.golden_pair_live_smoke
  : {};
const liveSmokeEngines = unique(liveSmoke.engines);
const promotionGate = registry.external_runtime_promotion_gate && typeof registry.external_runtime_promotion_gate === 'object'
  ? registry.external_runtime_promotion_gate
  : {};

if (registry.type !== 'agent_runtime_engine_registry') {
  push(violations, 'registry_type_invalid', REGISTRY_PATH, registry.type);
}
if (focus.broad_registry_allowed !== true) {
  push(violations, 'broad_registry_policy_missing', REGISTRY_PATH, 'validation_focus_policy.broad_registry_allowed must be true.');
}
if (focus.ui_selectable_is_not_promotion_scope !== true) {
  push(violations, 'ui_selectable_scope_boundary_missing', REGISTRY_PATH, 'UI selectable engines must not imply promotion scope.');
}
if (active.length < 2 || active.length > 3) {
  push(violations, 'active_promotion_engine_count_invalid', REGISTRY_PATH, `active_promotion_engines must contain 2-3 engines; got ${active.length}.`);
}
if (focus.native_engine !== 'infring_native' || !active.includes('infring_native')) {
  push(violations, 'native_focus_engine_missing', REGISTRY_PATH, 'Active focus must include native_engine=infring_native.');
}
if (focus.primary_external_engine && focus.primary_external_engine !== 'codex_cli') {
  push(violations, 'primary_external_engine_default_mismatch', REGISTRY_PATH, 'primary_external_engine should remain codex_cli for the current default checkpoint path.');
}
if (!active.includes('codex_cli') || !active.includes('claude_code') || !primaryExternal.includes('codex_cli') || !primaryExternal.includes('claude_code') || primaryExternal.length < 2) {
  push(violations, 'primary_external_engines_missing', REGISTRY_PATH, 'Active focus must include both codex_cli and claude_code as primary external engines.');
}
if (secondary.length !== 0) {
  push(violations, 'secondary_comparison_lane_retired', REGISTRY_PATH, 'Secondary comparison engines are retired while both external primaries are codex_cli and claude_code. Move remaining entries into active catalog strategy instead.');
}
for (const engineId of secondary) {
  if (!active.includes(engineId)) {
    push(violations, 'secondary_engine_not_active', REGISTRY_PATH, `${engineId} must also be listed in active_promotion_engines.`);
  }
}
for (const engineId of active) {
  if (!engineIds.includes(engineId)) {
    push(violations, 'active_engine_not_registered', REGISTRY_PATH, engineId);
  }
}
for (const engineId of catalogOnly) {
  if (!engineIds.includes(engineId)) {
    push(violations, 'catalog_only_engine_not_registered', REGISTRY_PATH, engineId);
  }
}
const overlap = active.filter((engineId) => catalogOnly.includes(engineId));
if (overlap.length) {
  push(violations, 'active_catalog_overlap', REGISTRY_PATH, overlap.join(','));
}
const unaccounted = difference(engineIds, unique([...active, ...catalogOnly]));
if (unaccounted.length) {
  push(violations, 'engine_focus_unaccounted', REGISTRY_PATH, unaccounted.join(','));
}
if (focus.default_live_work_scope !== 'active_promotion_engines') {
  push(violations, 'default_live_work_scope_invalid', REGISTRY_PATH, focus.default_live_work_scope);
}
if (focus.broad_sampling_requires_explicit_alias !== true) {
  push(violations, 'broad_sampling_not_explicit', REGISTRY_PATH, 'Broad sampling must require an explicit alias.');
}
const aliases = unique(focus.allowed_broad_sampling_aliases);
for (const alias of ['adapter-ready', 'registry', 'all']) {
  if (!aliases.includes(alias)) {
    push(violations, 'broad_sampling_alias_missing', REGISTRY_PATH, alias);
  }
}
if (!/Keep the engine registry broad/.test(clean(focus.rule, 2000))) {
  push(violations, 'focus_rule_too_weak', REGISTRY_PATH, focus.rule);
}
if (goldenPair.status !== 'covered') {
  push(violations, 'golden_pair_baseline_not_covered', REGISTRY_PATH, goldenPair.status || 'missing');
}
for (const engineId of ['codex_cli', 'claude_code']) {
  if (!goldenPairEngines.includes(engineId)) {
    push(violations, 'golden_pair_engine_missing', REGISTRY_PATH, engineId);
  }
}
for (const outcome of ['completed_response', 'permission_required', 'approval_resume_completed', 'typed_failure']) {
  if (!unique(goldenPair.required_terminal_outcomes).includes(outcome)) {
    push(violations, 'golden_pair_terminal_outcome_missing', REGISTRY_PATH, outcome);
  }
}
for (const primitive of ['native_direct_mutation_grant', 'text_proposal_pause', 'approval_resume_context', 'visible_final_response', 'typed_failure_projection']) {
  if (!unique(goldenPair.required_shared_primitives).includes(primitive)) {
    push(violations, 'golden_pair_shared_primitive_missing', REGISTRY_PATH, primitive);
  }
}
if (goldenPair.proof_artifact !== 'core/local/artifacts/agent_runtime_framework_capability_guard_current.json') {
  push(violations, 'golden_pair_proof_artifact_invalid', REGISTRY_PATH, goldenPair.proof_artifact || 'missing');
}
if (liveSmoke.status !== 'passing') {
  push(violations, 'golden_pair_live_smoke_not_passing', REGISTRY_PATH, liveSmoke.status || 'missing');
}
for (const engineId of ['codex_cli', 'claude_code']) {
  if (!liveSmokeEngines.includes(engineId)) {
    push(violations, 'golden_pair_live_smoke_engine_missing', REGISTRY_PATH, engineId);
  }
}
if (promotionGate.status !== 'active' || promotionGate.required_before_reliable_promotion !== true) {
  push(violations, 'external_runtime_promotion_gate_inactive', REGISTRY_PATH, 'external_runtime_promotion_gate must be active and required before reliable promotion.');
}
for (const blocked of ['reliable', 'promoted', 'production_ready', 'baseline_covered', 'golden_pair_equivalent']) {
  if (!unique(promotionGate.promotion_blocked_status_values).includes(blocked)) {
    push(violations, 'promotion_blocked_status_missing', REGISTRY_PATH, blocked);
  }
}

if (!exists(STRATEGY_DOC_PATH)) {
  push(violations, 'engine_strategy_doc_missing', STRATEGY_DOC_PATH, 'Agent Runtime engine strategy doc is missing.');
} else {
  const strategy = read(STRATEGY_DOC_PATH);
  for (const marker of [
    'Prove Codex and Claude Code as the golden external runtime pair',
    'cataloged/selectable does not mean promoted',
    'external engine mediated work = substrate/platform proof',
    'InfRing-native useful work = native intelligence/runtime proof',
  ]) {
    if (!strategy.includes(marker)) {
      push(violations, 'engine_strategy_marker_missing', STRATEGY_DOC_PATH, marker);
    }
  }
}

if (!exists(LIVE_WORK_EVAL_PATH)) {
  push(violations, 'live_work_eval_missing', LIVE_WORK_EVAL_PATH, 'Agent Runtime live-work eval script is missing.');
} else {
  const liveWorkEval = read(LIVE_WORK_EVAL_PATH);
  if (!liveWorkEval.includes("process.env.INFRING_AGENT_RUNTIME_EVAL_ENGINE || 'codex_cli'")) {
    push(violations, 'live_work_default_primary_not_codex', LIVE_WORK_EVAL_PATH, 'Default primary live-work eval engine must remain codex_cli.');
  }
  if (!liveWorkEval.includes('This may contact every registered engine, including planned adapters.')) {
    push(violations, 'registry_all_warning_missing', LIVE_WORK_EVAL_PATH, 'registry/all broad sampling must carry an explicit warning.');
  }
  if (!liveWorkEval.includes('adapter_ready_or_safe_bridge')) {
    push(violations, 'adapter_ready_alias_not_explicit', LIVE_WORK_EVAL_PATH, 'adapter-ready/selectable broad sampling must remain explicitly named.');
  }
}

if (!exists(COORDINATION_GUARD_PATH)) {
  push(violations, 'coordination_guard_missing', COORDINATION_GUARD_PATH, 'Agent Runtime coordination guard is missing.');
} else if (!read(COORDINATION_GUARD_PATH).includes('agent_runtime_engine_focus_guard.ts')) {
  push(violations, 'coordination_guard_not_registered', COORDINATION_GUARD_PATH, 'Framework coordination guard must include engine focus guard.');
}

const pkg = exists('package.json') ? json('package.json') : {};
if (!String(pkg.scripts && pkg.scripts['ops:agent-runtime:engine-focus:guard'] || '').includes('agent_runtime_engine_focus_guard.ts')) {
  push(violations, 'package_script_missing', 'package.json', 'ops:agent-runtime:engine-focus:guard must execute agent_runtime_engine_focus_guard.ts.');
}

const payload = {
  trace_id: `validation:${new Date().toISOString()}:${process.pid}`,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'agent_runtime_engine_focus_guard',
  generated_at: new Date().toISOString(),
  summary: {
    registry_engine_count: engineIds.length,
    active_promotion_engines: active,
    catalog_only_engines: catalogOnly,
    golden_pair_engines: goldenPairEngines,
    default_live_work_scope: focus.default_live_work_scope || '',
    violations: violations.length,
  },
  violations,
};

fs.mkdirSync(path.dirname(path.join(ROOT, OUT_JSON)), { recursive: true });
fs.writeFileSync(path.join(ROOT, OUT_JSON), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
