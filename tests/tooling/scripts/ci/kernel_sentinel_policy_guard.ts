#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
type Violation = { kind: string; path: string; detail: string };
function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}
function boolFlag(name: string, fallback = false): boolean {
  const raw = flag(name, fallback ? '1' : '0');
  return raw === '1' || raw === 'true';
}
function abs(rel: string): string { return path.join(ROOT, rel); }
function read(rel: string): string { return fs.readFileSync(abs(rel), 'utf8'); }
function json(rel: string): any { return JSON.parse(read(rel)); }
function exists(rel: string): boolean { return fs.existsSync(abs(rel)); }
function ensureDir(rel: string): void { fs.mkdirSync(path.dirname(abs(rel)), { recursive: true }); }
function requireArrayIncludes(violations: Violation[], rel: string, kind: string, actual: unknown, expected: string[]): void {
  const values = Array.isArray(actual) ? actual.map(String) : [];
  for (const value of expected) if (!values.includes(value)) violations.push({ kind, path: rel, detail: `Missing ${value}` });
}
function validateFeedback(violations: Violation[]): void {
  const rel = 'observability/sentinel/feedback_absorption_policy.json';
  const mdRel = 'observability/sentinel/feedback_absorption_policy.md';
  const policy = json(rel);
  const md = read(mdRel);
  if (policy.type !== 'kernel_sentinel_feedback_absorption_policy') violations.push({ kind: 'feedback_policy_type_invalid', path: rel, detail: 'Wrong policy type.' });
  if (policy.owner_domain !== 'observability') violations.push({ kind: 'feedback_owner_invalid', path: rel, detail: 'Owner must be observability.' });
  if (policy.release_policy?.safe_to_auto_apply_patch !== false || policy.release_policy?.human_review_required !== true) violations.push({ kind: 'feedback_release_policy_invalid', path: rel, detail: 'Policy must require review and forbid auto-apply.' });
  requireArrayIncludes(violations, rel, 'feedback_lane_missing', policy.promotion_lane, ['raw_observation', 'candidate_finding', 'clustered_issue', 'repair_backlog_item', 'closed_by_evidence']);
  requireArrayIncludes(violations, rel, 'feedback_field_missing', policy.required_finding_fields, ['evidence_refs', 'freshness_or_recurrence', 'owner_guess', 'root_cause_hypothesis', 'concrete_next_action', 'operating_law']);
  requireArrayIncludes(violations, rel, 'feedback_law_missing', policy.allowed_operating_laws, ['usability', 'reliability', 'simplicity']);
  requireArrayIncludes(violations, rel, 'feedback_dedupe_field_missing', policy.dedupe_key_fields, ['law', 'owner_domain', 'evidence_family', 'root_cause_hypothesis', 'failure_signature']);
  for (const token of ['Promotion lane', 'Required release shape', 'Deduplication key', 'Noise controls']) {
    if (!md.includes(token)) violations.push({ kind: 'feedback_doc_section_missing', path: mdRel, detail: `Missing ${token}` });
  }
  for (const ref of policy.policy_refs || []) if (!exists(String(ref))) violations.push({ kind: 'feedback_policy_ref_missing', path: rel, detail: String(ref) });
}
function validateBoundary(violations: Violation[]): void {
  const rel = 'observability/sentinel/eval_runtime_boundary_policy.json';
  const mdRel = 'observability/sentinel/eval_runtime_boundary_policy.md';
  const policy = json(rel);
  const md = read(mdRel);
  if (policy.type !== 'kernel_sentinel_eval_runtime_boundary_policy') violations.push({ kind: 'boundary_policy_type_invalid', path: rel, detail: 'Wrong policy type.' });
  requireArrayIncludes(violations, rel, 'sentinel_scope_missing', policy.sentinel_owns, ['receipt_drift', 'missing_finalization', 'authority_leak', 'cadence_failure', 'structural_root_cause_cluster']);
  requireArrayIncludes(violations, rel, 'eval_scope_missing', policy.eval_owns, ['hallucination', 'wrong_tool_choice', 'missing_tool_call', 'bad_answer_quality', 'empty_response_behavior']);
  if (policy.handoff_rule?.sentinel_may_cite_response_failures_only_when_runtime_caused !== true) violations.push({ kind: 'boundary_handoff_invalid', path: rel, detail: 'Sentinel response-failure citation must be runtime-caused only.' });
  if (policy.handoff_rule?.shared_issue_requires_owner_split !== true) violations.push({ kind: 'boundary_shared_issue_invalid', path: rel, detail: 'Shared issues must require owner split.' });
  for (const token of ['Sentinel owns runtime-system failures', 'Eval owns response-quality failures', 'Handoff rule', 'Shared issue rule']) {
    if (!md.includes(token)) violations.push({ kind: 'boundary_doc_section_missing', path: mdRel, detail: `Missing ${token}` });
  }
  for (const ref of policy.policy_refs || []) if (!exists(String(ref))) violations.push({ kind: 'boundary_policy_ref_missing', path: rel, detail: String(ref) });
}
function validateLayerCapability(violations: Violation[]): void {
  const rel = 'observability/sentinel/layer_capability_enforcement_policy.json';
  const mdRel = 'observability/sentinel/layer_capability_enforcement_policy.md';
  const contractRel = 'docs/workspace/KERNEL_LAYER_CAPABILITY_CONTRACT.md';
  const placementRel = 'client/runtime/config/layer_placement_policy.json';
  const policy = json(rel);
  const md = read(mdRel);
  const contract = read(contractRel);
  const placement = json(placementRel);
  if (policy.type !== 'kernel_sentinel_layer_capability_enforcement_policy') violations.push({ kind: 'layer_capability_policy_type_invalid', path: rel, detail: 'Wrong policy type.' });
  if (policy.owner_domain !== 'observability') violations.push({ kind: 'layer_capability_owner_invalid', path: rel, detail: 'Owner must be observability.' });
  if (policy.contract_doc !== contractRel) violations.push({ kind: 'layer_capability_contract_doc_invalid', path: rel, detail: 'Contract doc path mismatch.' });
  requireArrayIncludes(
    violations,
    rel,
    'layer_capability_forbidden_capability_missing',
    policy.layer0_envelope?.forbidden_capabilities,
    ['web_retrieval', 'browser_automation', 'provider_orchestration', 'semantic_quality_diagnostics', 'query_rewriting'],
  );
  if (policy.sentinel_expectations?.must_flag_new_layer0_web_surface !== true) violations.push({ kind: 'layer_capability_web_expectation_invalid', path: rel, detail: 'Sentinel must flag new Layer 0 web surfaces.' });
  if (policy.sentinel_expectations?.must_flag_new_layer0_semantic_policy !== true) violations.push({ kind: 'layer_capability_semantic_expectation_invalid', path: rel, detail: 'Sentinel must flag new Layer 0 semantic policy.' });
  if (policy.sentinel_expectations?.must_track_legacy_layer_debt_separately_from_new_regressions !== true) violations.push({ kind: 'layer_capability_legacy_debt_expectation_invalid', path: rel, detail: 'Sentinel must distinguish legacy debt from new regressions.' });
  for (const token of ['Layer0 hardware envelope', 'Automation surfaces', 'Sentinel expectations', 'Legacy debt handling']) {
    if (!md.includes(token)) violations.push({ kind: 'layer_capability_doc_section_missing', path: mdRel, detail: `Missing ${token}` });
  }
  for (const token of ['Status: hard repo-wide policy', 'Layer 0 Hard Boundary', 'web retrieval', 'ops:layer-placement:check', 'ops:layer0:dependency-boundary:guard', 'ops:layer-capability:trend:guard']) {
    if (!contract.includes(token)) violations.push({ kind: 'layer_capability_contract_section_missing', path: contractRel, detail: `Missing ${token}` });
  }
  if (placement.contract_doc !== contractRel) violations.push({ kind: 'layer_capability_placement_contract_ref_invalid', path: placementRel, detail: 'Placement policy must point at the canonical contract doc.' });
  requireArrayIncludes(
    violations,
    placementRel,
    'layer_capability_path_rule_missing',
    placement.forbidden_path_markers_by_prefix?.[0]?.markers,
    ['web', 'browser', 'search', 'provider'],
  );
  requireArrayIncludes(
    violations,
    placementRel,
    'layer_capability_concept_rule_missing',
    placement.forbidden_concept_markers_by_prefix?.[0]?.markers,
    ['answerability', 'source quality', 'current_web_intent', 'web_tool_quality'],
  );
  for (const ref of policy.policy_refs || []) if (!exists(String(ref))) violations.push({ kind: 'layer_capability_policy_ref_missing', path: rel, detail: String(ref) });
}
function main(): void {
  const strict = boolFlag('strict', true);
  const outJson = flag('out-json', 'core/local/artifacts/kernel_sentinel_policy_guard_current.json');
  const outMd = flag('out-markdown', 'local/workspace/reports/KERNEL_SENTINEL_POLICY_GUARD_CURRENT.md');
  const violations: Violation[] = [];
  validateFeedback(violations);
  validateBoundary(violations);
  validateLayerCapability(violations);
  const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-policy-guard`;
  const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'observability', ok: violations.length === 0, type: 'kernel_sentinel_policy_guard', generated_at: new Date().toISOString(), strict, violations };
  ensureDir(outJson);
  fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd);
  fs.writeFileSync(abs(outMd), `# Kernel Sentinel Policy Guard\n\n- ok: ${payload.ok}\n- violations: ${violations.length}\n\n${violations.map((v) => `- ${v.kind}: ${v.path} ${v.detail}`).join('\n') || '- none'}\n`);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !payload.ok) process.exit(1);
}
main();
