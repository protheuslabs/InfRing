#!/usr/bin/env tsx

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type PackManifest = {
  version: number;
  artifact_groups?: Record<string, string[]>;
  category_completeness_min?: Record<string, number>;
  freshness_max_age_hours?: Record<string, number>;
  freshness_required_artifacts?: string[];
  required_artifacts: string[];
  optional_artifacts: string[];
};
type ProofPackSizePolicy = {
  policy_path: string;
  max_single_artifact_bytes: number;
  max_proof_pack_bytes: number;
  oversized_artifacts_must_be_compacted_refs: boolean;
  allowed_large_artifact_patterns: string[];
  compacted_ref_required_fields: string[];
};

const MANDATORY_RELEASE_PROOF_ARTIFACTS = [
  'core/local/artifacts/layer2_lane_parity_guard_current.json',
  'core/local/artifacts/layer2_receipt_replay_current.json',
  'core/local/artifacts/runtime_trusted_core_report_current.json',
  'core/local/artifacts/eval_regression_guard_current.json',
  'core/local/artifacts/eval_feedback_router_current.json',
  'core/local/artifacts/issue_candidate_contract_guard_current.json',
  'core/local/artifacts/issue_candidate_backlog_current.json',
  'core/local/artifacts/runtime_proof_reality_guard_current.json',
  'core/local/artifacts/runtime_soak_scenarios_current.json',
];
const MANDATORY_PASSING_RELEASE_PROOF_ARTIFACTS = [
  'core/local/artifacts/eval_regression_guard_current.json',
  'core/local/artifacts/eval_feedback_router_current.json',
  'core/local/artifacts/issue_candidate_contract_guard_current.json',
  'core/local/artifacts/issue_candidate_backlog_current.json',
  'core/local/artifacts/runtime_proof_reality_guard_current.json',
  'core/local/artifacts/runtime_soak_scenarios_current.json',
];
const DEFAULT_FRESHNESS_REQUIRED_ARTIFACTS = [
  'core/local/artifacts/runtime_proof_verify_current.json',
  'core/local/artifacts/gateway_runtime_chaos_gate_current.json',
  'core/local/artifacts/windows_installer_contract_guard_current.json',
  'core/local/artifacts/windows_install_reliability_current.json',
  'core/local/artifacts/installer_reliability_closure_guard_current.json',
  'core/local/artifacts/eval_regression_guard_current.json',
  'core/local/artifacts/eval_feedback_router_current.json',
  'core/local/artifacts/issue_candidate_contract_guard_current.json',
  'core/local/artifacts/issue_candidate_backlog_current.json',
  'core/local/artifacts/test_maturity_registry_current.json',
];
const DEFAULT_FRESHNESS_MAX_AGE_HOURS = 336;
const CURRENT_EVIDENCE_ALLOWED_SUFFIXES = [
  '_current.json',
  '_latest.json',
  'release_scorecard.json',
  'benchmark_matrix_run_latest.json',
  'support_bundle_latest.json',
  '.md',
  '.jsonl',
];
const CURRENT_EVIDENCE_ALLOWED_PATHS = new Set([
  'core/local/artifacts/gateway_manifest.json',
  'core/local/artifacts/layer2_parity_matrix.json',
]);

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/release_proof_pack_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'validation/release_gates/contracts/release_proof_pack_manifest.json',
      400,
    ),
    version: cleanText(readFlag(argv, 'version') || new Date().toISOString().slice(0, 10), 120),
  };
}

function ensureParent(absPath: string) {
  const parent = path.dirname(absPath);
  fs.mkdirSync(parent, { recursive: true });
}

function sha256File(absPath: string): string {
  const data = fs.readFileSync(absPath);
  return createHash('sha256').update(data).digest('hex');
}

function readProofPackSizePolicy(root: string): ProofPackSizePolicy {
  const policyPath = 'validation/release_gates/policies/proof_pack_artifact_size_policy.json';
  const absPath = path.resolve(root, policyPath);
  const parsed = JSON.parse(fs.readFileSync(absPath, 'utf8'));
  const policy = parsed.policy || {};
  return {
    policy_path: policyPath,
    max_single_artifact_bytes: Number(policy.max_single_artifact_bytes || 5_242_880),
    max_proof_pack_bytes: Number(policy.max_proof_pack_bytes || 52_428_800),
    oversized_artifacts_must_be_compacted_refs: policy.oversized_artifacts_must_be_compacted_refs !== false,
    allowed_large_artifact_patterns: Array.isArray(parsed.allowed_large_artifact_patterns)
      ? parsed.allowed_large_artifact_patterns.map(String)
      : [],
    compacted_ref_required_fields: Array.isArray(policy.compacted_ref_required_fields)
      ? policy.compacted_ref_required_fields.map(String)
      : [],
  };
}

function largeArtifactAllowed(relPath: string, sizePolicy: ProofPackSizePolicy): boolean {
  return sizePolicy.allowed_large_artifact_patterns.some((pattern) => relPath.endsWith(pattern));
}

function categoryLookup(manifest: PackManifest): Map<string, string> {
  const out = new Map<string, string>();
  const groups = manifest.artifact_groups || {};
  for (const [group, rows] of Object.entries(groups)) {
    for (const relPath of rows || []) {
      out.set(cleanText(relPath, 400), cleanText(group, 120));
    }
  }
  return out;
}

function dedupePaths(rows: string[] | undefined): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const row of rows || []) {
    const normalized = cleanText(row, 400);
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function duplicatePathEntries(section: string, rows: string[] | undefined) {
  const counts = new Map<string, number>();
  for (const row of rows || []) {
    const normalized = cleanText(row, 400);
    if (!normalized) continue;
    counts.set(normalized, (counts.get(normalized) || 0) + 1);
  }
  return Array.from(counts.entries())
    .filter(([, count]) => count > 1)
    .map(([pathToken, count]) => ({
      section,
      path: pathToken,
      count,
      recommendation: 'keep one canonical manifest entry; proof-pack assembly deduped this path for the current run',
    }));
}

function copyIntoPack(root: string, relPath: string, packRoot: string, sizePolicy: ProofPackSizePolicy) {
  const source = path.resolve(root, relPath);
  const exists = fs.existsSync(source);
  const destination = path.resolve(packRoot, relPath);
  let checksum = '';
  let sizeBytes = 0;
  let modifiedAt = '';
  let ageHours: number | null = null;
  let compacted = false;
  let originalSizeBytes = 0;
  let originalChecksum = '';
  if (exists) {
    ensureParent(destination);
    const stat = fs.statSync(source);
    originalSizeBytes = stat.size;
    originalChecksum = sha256File(source);
    modifiedAt = stat.mtime.toISOString();
    ageHours = Math.max(0, (Date.now() - stat.mtimeMs) / (60 * 60 * 1000));
    if (
      sizePolicy.oversized_artifacts_must_be_compacted_refs &&
      stat.size > sizePolicy.max_single_artifact_bytes &&
      !largeArtifactAllowed(relPath, sizePolicy)
    ) {
      compacted = true;
      const compactedRef = {
        type: 'proof_pack_compacted_artifact_ref',
        compacted_at: new Date().toISOString(),
        reason: 'source artifact exceeded proof-pack single-artifact byte budget',
        previous_artifact: relPath,
        retention_policy: {
          source_retained_in_place: true,
          source_path: relPath,
          source_size_bytes: stat.size,
          source_sha256: originalChecksum,
          max_single_artifact_bytes: sizePolicy.max_single_artifact_bytes,
          policy_path: sizePolicy.policy_path,
        },
        operator_summary: {
          artifact: relPath,
          action: 'inspect source artifact directly if raw evidence is needed; proof pack carries compacted reference only',
          compacted_ref_required_fields: sizePolicy.compacted_ref_required_fields,
        },
      };
      fs.writeFileSync(destination, `${JSON.stringify(compactedRef, null, 2)}\n`, 'utf8');
    } else {
      fs.copyFileSync(source, destination);
    }
    checksum = sha256File(destination);
    sizeBytes = fs.statSync(destination).size;
  }
  return {
    path: relPath,
    exists,
    source,
    destination,
    checksum,
    size_bytes: sizeBytes,
    source_modified_at: modifiedAt,
    source_age_hours: ageHours,
    compacted,
    original_size_bytes: originalSizeBytes,
    original_checksum: originalChecksum,
  };
}

function proofPackSizeBudgetFailures(artifactRows: Array<{ path: string; size_bytes: number; compacted?: boolean }>, sizePolicy: ProofPackSizePolicy) {
  const singleArtifactFailures = artifactRows
    .filter((row) => row.size_bytes > sizePolicy.max_single_artifact_bytes)
    .filter((row) => !largeArtifactAllowed(row.path, sizePolicy))
    .map((row) => ({
      path: row.path,
      size_bytes: row.size_bytes,
      max_bytes: sizePolicy.max_single_artifact_bytes,
      compacted: row.compacted === true,
      failure: 'proof_pack_single_artifact_budget_exceeded',
    }));
  const copiedArtifactBytes = artifactRows.reduce((sum, row) => sum + Number(row.size_bytes || 0), 0);
  const packFailures = copiedArtifactBytes > sizePolicy.max_proof_pack_bytes
    ? [{
        path: 'release_proof_pack',
        size_bytes: copiedArtifactBytes,
        max_bytes: sizePolicy.max_proof_pack_bytes,
        failure: 'proof_pack_total_budget_exceeded',
      }]
    : [];
  return { singleArtifactFailures, packFailures, copiedArtifactBytes };
}

function jsonArtifactStatus(absPath: string): { declares_status: boolean; passing: boolean; detail: string } {
  try {
    const payload = JSON.parse(fs.readFileSync(absPath, 'utf8'));
    if (payload && typeof payload === 'object' && Object.prototype.hasOwnProperty.call(payload, 'ok')) {
      return { declares_status: true, passing: payload.ok === true, detail: 'ok' };
    }
    if (
      payload &&
      typeof payload === 'object' &&
      payload.summary &&
      typeof payload.summary === 'object' &&
      Object.prototype.hasOwnProperty.call(payload.summary, 'pass')
    ) {
      return { declares_status: true, passing: payload.summary.pass === true, detail: 'summary.pass' };
    }
    return { declares_status: false, passing: true, detail: 'no_status_field' };
  } catch {
    return { declares_status: true, passing: false, detail: 'json_parse_failed' };
  }
}

function jsonArtifactPasses(absPath: string): boolean {
  return jsonArtifactStatus(absPath).passing;
}

function isCurrentEvidencePath(relPath: string): boolean {
  const normalized = cleanText(relPath, 400);
  if (normalized.startsWith('releases/proof-packs/')) return false;
  if (normalized.includes('/releases/proof-packs/')) return false;
  if (normalized.startsWith('validation/release_gates/proof_packs/')) return false;
  if (normalized.includes('/validation/release_gates/proof_packs/')) return false;
  if (CURRENT_EVIDENCE_ALLOWED_PATHS.has(normalized)) return true;
  return CURRENT_EVIDENCE_ALLOWED_SUFFIXES.some((suffix) => normalized.endsWith(suffix));
}

function blockerPriority(blocker: any): number {
  const classRank: Record<string, number> = {
    required_missing: 900,
    mandatory_artifact_failures: 850,
    required_failed_artifacts: 800,
    summary_consistency_failures: 760,
    category_threshold_failures: 720,
    stale_artifact_failures: 650,
    current_evidence_failures: 600,
    proof_pack_size_budget_failures: 590,
    manifest_duplicate_warnings: 100,
  };
  return Number(classRank[String(blocker?.class || '')] || 0);
}

function blockerOwner(blocker: any): string {
  const blockerClass = String(blocker?.class || '');
  if (blockerClass.includes('gateway')) return 'adapters/gateways';
  if (blockerClass.includes('runtime') || blockerClass.includes('required')) return 'core/kernel';
  if (blockerClass.includes('summary') || blockerClass.includes('manifest')) return 'release_governance/proof_pack';
  return 'release_governance/proof_pack';
}

function blockerTargetLayer(blocker: any): string {
  const blockerClass = String(blocker?.class || '');
  if (blockerClass.includes('gateway')) return 'gateway';
  if (blockerClass.includes('runtime') || blockerClass.includes('required')) return 'kernel';
  return 'release_governance';
}

function blockerReleaseGateEffect(blocker: any): string {
  return String(blocker?.severity || '') === 'release_blocking'
    ? 'blocks_release_until_closed'
    : 'non_blocking_hygiene';
}

function blockerEscalationTier(blocker: any): string {
  const priority = blockerPriority(blocker);
  if (priority >= 800) return 'release_blocker';
  if (priority >= 600) return 'operator_attention';
  return 'watchlist';
}

function blockerOperatorNextStep(blocker: any): string {
  const action = String(blocker?.action || '').trim();
  return action
    ? `${action}; rerun release proof-pack assembly and confirm this dedupe key disappears`
    : 'rerun release proof-pack assembly and inspect remaining top blockers';
}

function blockerTriageQueue(blocker: any): string {
  const tier = blockerEscalationTier(blocker);
  if (tier === 'release_blocker') return 'release_blockers';
  if (tier === 'operator_attention') return 'release_operator_attention';
  return 'release_watchlist';
}

function topBlockerActionable(blocker: any): boolean {
  const requiredText = [
    blocker?.source_report,
    blocker?.issue_lifecycle_state,
    blocker?.source_artifact_policy,
    blocker?.dedupe_key,
    blocker?.owner,
    blocker?.target_layer,
    blocker?.release_gate_effect,
    blocker?.escalation_tier,
    blocker?.operator_next_step,
    blocker?.triage_queue,
    blocker?.closing_evidence_required,
    blocker?.closure_verification_command,
  ];
  return (
    requiredText.every((value) => String(value || '').trim().length > 0) &&
    Number(blocker?.issue_contract_version || 0) === 1 &&
    Array.isArray(blocker?.source_artifacts) &&
    blocker.source_artifacts.length > 0 &&
    blocker.source_artifacts.every((row: unknown) => localArtifactPathOk(String(row || ''))) &&
    blocker?.safe_to_auto_file_issue === true &&
    blocker?.safe_to_auto_apply_patch === false &&
    blocker?.human_review_required === true &&
    blocker?.requires_operator_ack === true &&
    blocker?.autonomous_mitigation_allowed === false
  );
}

function localArtifactPathOk(pathToken: string): boolean {
  const normalized = pathToken.trim();
  return (
    normalized.length > 0 &&
    !path.isAbsolute(normalized) &&
    !normalized.startsWith('http://') &&
    !normalized.startsWith('https://') &&
    !normalized.includes('..')
  );
}

function markdown(report: any): string {
  const lines = [
    '# Release Proof Pack',
    '',
    `- version: ${report.version}`,
    `- pack_root: ${report.pack_root}`,
    `- required_missing: ${report.summary.required_missing}`,
    '',
    '| artifact | category | required | exists | sha256 |',
    '| --- | --- | :---: | :---: | --- |',
  ];
  if (report.summary) {
    lines.splice(
      5,
      0,
      `- stale_artifacts: ${Number(report.summary.stale_artifacts || 0)}`,
      `- failed_artifacts: ${Number(report.summary.failed_artifacts || 0)}`,
      `- historical_snapshot_artifacts: ${Number(report.summary.historical_snapshot_artifacts || 0)}`,
      `- summary_consistency_failures: ${Number(report.summary.summary_consistency_failures || 0)}`,
      `- category_required_missing_sum: ${Number(report.summary.category_required_missing_sum || 0)}`,
      `- category_artifact_count_sum: ${Number(report.summary.category_artifact_count_sum || 0)}`,
      `- category_required_total_sum: ${Number(report.summary.category_required_total_sum || 0)}`,
      `- release_blocking_issue_count: ${Number(report.summary.release_blocking_issue_count || 0)}`,
      `- top_blocker_count: ${Number(report.summary.top_blocker_count || 0)}`,
      `- primary_blocker_class: ${report.summary.primary_blocker_class || 'none'}`,
      `- primary_blocker_artifact: ${report.summary.primary_blocker_artifact || 'none'}`,
      `- primary_blocker_action: ${report.summary.primary_blocker_action || 'none'}`,
      `- primary_blocker_dedupe_key: ${report.summary.primary_blocker_dedupe_key || 'none'}`,
      `- primary_blocker_priority_score: ${Number(report.summary.primary_blocker_priority_score || 0)}`,
      `- primary_blocker_owner: ${report.summary.primary_blocker_owner || 'none'}`,
      `- primary_blocker_target_layer: ${report.summary.primary_blocker_target_layer || 'none'}`,
      `- primary_blocker_escalation_tier: ${report.summary.primary_blocker_escalation_tier || 'none'}`,
      `- primary_blocker_release_gate_effect: ${report.summary.primary_blocker_release_gate_effect || 'none'}`,
      `- primary_blocker_operator_next_step: ${report.summary.primary_blocker_operator_next_step || 'none'}`,
      `- primary_blocker_triage_queue: ${report.summary.primary_blocker_triage_queue || 'none'}`,
      `- primary_blocker_lifecycle_state: ${report.summary.primary_blocker_lifecycle_state || 'none'}`,
      `- primary_blocker_source_artifact_count: ${Number(report.summary.primary_blocker_source_artifact_count || 0)}`,
      `- primary_blocker_closure_verification_command: ${report.summary.primary_blocker_closure_verification_command || 'none'}`,
      `- top_blockers_actionable: ${report.summary.top_blockers_actionable ? 'true' : 'false'}`,
      `- top_blocker_actionability_failure_count: ${Number(report.summary.top_blocker_actionability_failure_count || 0)}`,
      `- top_blocker_action_count: ${Number(report.summary.top_blocker_action_count || 0)}`,
    );
  }
  for (const row of report.artifacts) {
    lines.push(
      `| ${row.path} | ${row.category} | ${row.required ? 'yes' : 'no'} | ${row.exists ? 'yes' : 'no'} | ${
        row.exists ? row.checksum : 'missing'
      } |`,
    );
  }
  lines.push('');
  lines.push('## Category summary');
  for (const group of report.category_summary) {
    const threshold =
      report?.category_completeness_min &&
      Object.prototype.hasOwnProperty.call(report.category_completeness_min, group.category)
        ? Number(report.category_completeness_min[group.category])
        : null;
    lines.push(
      `- ${group.category}: present=${group.present}/${group.total};required=${group.required_present}/${group.required_total};required_missing=${group.required_missing};required_completeness=${group.required_completeness.toFixed(
        3,
      )}${threshold == null ? '' : `;required_min=${threshold.toFixed(3)}`}`,
    );
  }
  lines.push('');
  lines.push('## Operator summary');
  if (report.operator_summary) {
    lines.push(`- pass: ${report.operator_summary.pass ? 'true' : 'false'}`);
    lines.push(`- primary_blocker: ${report.operator_summary.primary_blocker || 'none'}`);
    lines.push(`- issue_candidate_ready: ${report.operator_summary.issue_candidate_ready ? 'true' : 'false'}`);
    lines.push(`- next_actions: ${(report.operator_summary.next_actions || []).length}`);
  } else {
    lines.push('- unavailable');
  }
  lines.push('');
  if (report.issue_candidate) {
    lines.push('## Issue candidate');
    lines.push(`- title: ${report.issue_candidate.title}`);
    lines.push(`- severity: ${report.issue_candidate.severity}`);
    lines.push(`- fingerprint: ${report.issue_candidate.fingerprint}`);
    lines.push(`- next_actions: ${(report.issue_candidate.next_actions || []).length}`);
    lines.push('');
  }
  if (Array.isArray(report.top_blockers) && report.top_blockers.length > 0) {
    lines.push('## Top blockers');
    for (const blocker of report.top_blockers.slice(0, 10)) {
      lines.push(`- ${blocker.severity}: ${blocker.class} ${blocker.artifact} -> ${blocker.action}`);
    }
    lines.push('');
  }
  lines.push('## Manifest hygiene');
  const duplicateWarnings = Array.isArray(report.manifest_duplicate_warnings)
    ? report.manifest_duplicate_warnings
    : [];
  if (duplicateWarnings.length === 0) {
    lines.push('- duplicate_warnings: 0');
  } else {
    lines.push(`- duplicate_warnings: ${duplicateWarnings.length}`);
    for (const row of duplicateWarnings) {
      lines.push(
        `- ${row.section}: ${row.path} count=${row.count}; ${row.recommendation}`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const manifestRaw = fs.readFileSync(path.resolve(root, args.manifestPath), 'utf8');
  const manifest = JSON.parse(manifestRaw) as PackManifest;
  const sizePolicy = readProofPackSizePolicy(root);
  const categoryByPath = categoryLookup(manifest);
  const requiredArtifacts = dedupePaths(manifest.required_artifacts || []);
  const requiredArtifactSet = new Set(requiredArtifacts);
  const optionalArtifactsRaw = dedupePaths(manifest.optional_artifacts || []);
  const optionalArtifacts = optionalArtifactsRaw.filter((row) => !requiredArtifactSet.has(row));
  const manifestDuplicateWarnings = [
    ...duplicatePathEntries('required_artifacts', manifest.required_artifacts || []),
    ...duplicatePathEntries('optional_artifacts', manifest.optional_artifacts || []),
    ...Object.entries(manifest.artifact_groups || {}).flatMap(([group, rows]) =>
      duplicatePathEntries(`artifact_groups.${group}`, rows || []),
    ),
    ...optionalArtifactsRaw
      .filter((row) => requiredArtifactSet.has(row))
      .map((pathToken) => ({
        section: 'optional_artifacts',
        path: pathToken,
        count: 1,
        recommendation: 'remove optional duplicate because this path is already required evidence',
      })),
  ];

  const packRoot = path.resolve(root, 'releases', 'proof-packs', args.version);
  fs.mkdirSync(packRoot, { recursive: true });

  const artifactRows: Array<{
    path: string;
    category: string;
    required: boolean;
    exists: boolean;
    source: string;
    destination: string;
    checksum: string;
    size_bytes: number;
    source_modified_at: string;
    source_age_hours: number | null;
    compacted: boolean;
    original_size_bytes: number;
    original_checksum: string;
  }> = [];

  for (const normalized of requiredArtifacts) {
    artifactRows.push({
      ...copyIntoPack(root, normalized, packRoot, sizePolicy),
      category: cleanText(categoryByPath.get(normalized) || 'ungrouped', 120),
      required: true,
    });
  }
  for (const normalized of optionalArtifacts) {
    artifactRows.push({
      ...copyIntoPack(root, normalized, packRoot, sizePolicy),
      category: cleanText(categoryByPath.get(normalized) || 'ungrouped', 120),
      required: false,
    });
  }

  const requiredMissing = artifactRows.filter((row) => row.required && !row.exists).map((row) => row.path);
  const mandatoryArtifactFailures = MANDATORY_RELEASE_PROOF_ARTIFACTS.map((artifactPath) => {
    const row = artifactRows.find((candidate) => candidate.path === artifactPath);
    if (!row) {
      return {
        path: artifactPath,
        failure: 'missing_from_manifest',
      };
    }
    if (!row.required) {
      return {
        path: artifactPath,
        failure: 'not_required_in_manifest',
      };
    }
    if (!row.exists) {
      return {
        path: artifactPath,
        failure: 'required_artifact_missing',
      };
    }
    if (
      MANDATORY_PASSING_RELEASE_PROOF_ARTIFACTS.includes(artifactPath) &&
      !jsonArtifactPasses(row.source)
    ) {
      return {
        path: artifactPath,
        failure: 'required_artifact_not_passing',
      };
    }
    return null;
  }).filter((row): row is { path: string; failure: string } => !!row);
  const requiredFailedArtifacts = artifactRows
    .filter((row) => row.required && row.exists && row.path.endsWith('.json'))
    .map((row) => ({
      path: row.path,
      status: jsonArtifactStatus(row.source),
    }))
    .filter((row) => row.status.declares_status && !row.status.passing)
    .map((row) => ({
      path: row.path,
      failure: `required_artifact_failed:${row.status.detail}`,
    }));
  const freshnessMaxAgeHours = manifest.freshness_max_age_hours || {};
  const freshnessRequired = new Set(
    [
      ...DEFAULT_FRESHNESS_REQUIRED_ARTIFACTS,
      ...(Array.isArray(manifest.freshness_required_artifacts)
        ? manifest.freshness_required_artifacts
        : []),
    ].map((row) => cleanText(row, 400)).filter(Boolean),
  );
  const staleArtifactFailures = artifactRows
    .filter((row) => row.required && row.exists && freshnessRequired.has(row.path))
    .map((row) => {
      const category = cleanText(row.category, 120);
      const maxAge =
        Number(freshnessMaxAgeHours[row.path]) ||
        Number(freshnessMaxAgeHours[category]) ||
        DEFAULT_FRESHNESS_MAX_AGE_HOURS;
      const age = Number(row.source_age_hours || 0);
      return {
        path: row.path,
        category,
        age_hours: age,
        max_age_hours: maxAge,
        ok: age <= maxAge,
      };
    })
    .filter((row) => !row.ok);
  const currentEvidenceFailures = artifactRows
    .filter((row) => row.required)
    .filter((row) => !isCurrentEvidencePath(row.path))
    .map((row) => ({
      path: row.path,
      failure: row.path.includes('releases/proof-packs/')
        || row.path.includes('validation/release_gates/proof_packs/')
        ? 'historical_snapshot_path_used_as_current_evidence'
        : 'not_current_or_latest_evidence_path',
    }));
  const categoryCompletenessMin = manifest.category_completeness_min || {};
  const categories = Array.from(new Set(artifactRows.map((row) => row.category)));
  const categorySummary = categories.map((category) => {
    const rows = artifactRows.filter((row) => row.category === category);
    const present = rows.filter((row) => row.exists).length;
    const requiredRows = rows.filter((row) => row.required);
    const requiredPresent = requiredRows.filter((row) => row.exists).length;
    const requiredMissingCount = requiredRows.length - requiredPresent;
    const requiredCompleteness = requiredRows.length <= 0 ? 1 : requiredPresent / requiredRows.length;
    return {
      category,
      total: rows.length,
      present,
      required_total: requiredRows.length,
      required_present: requiredPresent,
      required_missing: requiredMissingCount,
      required_completeness: requiredCompleteness,
    };
  });
  const categoryThresholdFailures = Object.entries(categoryCompletenessMin)
    .map(([category, thresholdRaw]) => {
      const threshold = Number(thresholdRaw);
      if (!Number.isFinite(threshold)) return null;
      const summary = categorySummary.find((row) => row.category === category);
      const actual = summary ? Number(summary.required_completeness) : 0;
      const ok = !!summary && actual + Number.EPSILON >= threshold;
      return {
        id: 'proof_pack_category_completeness_below_threshold',
        category,
        threshold,
        actual,
        ok,
        detail: `${category}: actual=${actual.toFixed(3)};required_min=${threshold.toFixed(3)}`,
      };
    })
    .filter((row): row is { id: string; category: string; threshold: number; actual: number; ok: boolean; detail: string } => !!row)
    .filter((row) => !row.ok);
  const categoryRequiredMissingSum = categorySummary.reduce(
    (sum, row) => sum + Number(row.required_missing || 0),
    0,
  );
  const categoryArtifactCountSum = categorySummary.reduce((sum, row) => sum + Number(row.total || 0), 0);
  const categoryRequiredTotalSum = categorySummary.reduce((sum, row) => sum + Number(row.required_total || 0), 0);
  const proofPackSummaryConsistencyFailures = [
    ...(categoryRequiredMissingSum === requiredMissing.length
      ? []
      : [
          {
            id: 'proof_pack_summary_required_missing_mismatch',
            expected: requiredMissing.length,
            actual: categoryRequiredMissingSum,
            detail: `top_level_required_missing=${requiredMissing.length};category_required_missing_sum=${categoryRequiredMissingSum}`,
          },
        ]),
    ...(categoryArtifactCountSum === artifactRows.length
      ? []
      : [
          {
            id: 'proof_pack_summary_artifact_count_mismatch',
            expected: artifactRows.length,
            actual: categoryArtifactCountSum,
            detail: `artifact_rows=${artifactRows.length};category_artifact_count_sum=${categoryArtifactCountSum}`,
          },
        ]),
    ...(categoryRequiredTotalSum === requiredArtifacts.length
      ? []
      : [
          {
            id: 'proof_pack_summary_required_total_mismatch',
            expected: requiredArtifacts.length,
            actual: categoryRequiredTotalSum,
            detail: `required_artifacts=${requiredArtifacts.length};category_required_total_sum=${categoryRequiredTotalSum}`,
          },
        ]),
  ];
  const sizeBudget = proofPackSizeBudgetFailures(artifactRows, sizePolicy);
  const proofPackSizeBudgetFailuresRows = [
    ...sizeBudget.singleArtifactFailures,
    ...sizeBudget.packFailures,
  ];
  const blockerClassCounts = {
    required_missing: requiredMissing.length,
    mandatory_artifact_failures: mandatoryArtifactFailures.length,
    required_failed_artifacts: requiredFailedArtifacts.length,
    category_threshold_failures: categoryThresholdFailures.length,
    stale_artifact_failures: staleArtifactFailures.length,
    current_evidence_failures: currentEvidenceFailures.length,
    summary_consistency_failures: proofPackSummaryConsistencyFailures.length,
    proof_pack_size_budget_failures: proofPackSizeBudgetFailuresRows.length,
    manifest_duplicate_warnings: manifestDuplicateWarnings.length,
  };
  const releaseBlockingIssueCount =
    blockerClassCounts.required_missing +
    blockerClassCounts.mandatory_artifact_failures +
    blockerClassCounts.required_failed_artifacts +
    blockerClassCounts.category_threshold_failures +
    blockerClassCounts.stale_artifact_failures +
    blockerClassCounts.current_evidence_failures +
    blockerClassCounts.summary_consistency_failures +
    blockerClassCounts.proof_pack_size_budget_failures;
  const blockerSeverityCounts = {
    release_blocking: releaseBlockingIssueCount,
    hygiene: blockerClassCounts.manifest_duplicate_warnings,
  };
  const primaryBlockerClass =
    Object.entries(blockerClassCounts).find(([, count]) => Number(count) > 0)?.[0] || '';
  const topBlockers = [
    ...requiredMissing.map((artifact) => ({
      class: 'required_missing',
      severity: 'release_blocking',
      artifact,
      action: `produce required proof artifact ${artifact}`,
    })),
    ...mandatoryArtifactFailures.map((row) => ({
      class: 'mandatory_artifact_failures',
      severity: 'release_blocking',
      artifact: row.path,
      action: `repair mandatory proof artifact ${row.path}`,
      detail: row.failure,
    })),
    ...requiredFailedArtifacts.map((row) => ({
      class: 'required_failed_artifacts',
      severity: 'release_blocking',
      artifact: row.path,
      action: `repair failing required artifact ${row.path}`,
      detail: row.failure,
    })),
    ...categoryThresholdFailures.map((row) => ({
      class: 'category_threshold_failures',
      severity: 'release_blocking',
      artifact: row.category,
      action: `restore proof-pack category completeness for ${row.category}`,
      detail: row.detail,
    })),
    ...staleArtifactFailures.map((row) => ({
      class: 'stale_artifact_failures',
      severity: 'release_blocking',
      artifact: row.path,
      action: `refresh stale proof artifact ${row.path}`,
      detail: `age_hours=${row.age_hours};max_age_hours=${row.max_age_hours}`,
    })),
    ...currentEvidenceFailures.map((row) => ({
      class: 'current_evidence_failures',
      severity: 'release_blocking',
      artifact: row.path,
      action: `replace non-current proof evidence path ${row.path}`,
      detail: row.failure,
    })),
    ...proofPackSummaryConsistencyFailures.map((row) => ({
      class: 'summary_consistency_failures',
      severity: 'release_blocking',
      artifact: 'release_proof_pack_summary',
      action: 'repair proof-pack summary/category accounting mismatch',
      detail: row.detail,
    })),
    ...proofPackSizeBudgetFailuresRows.map((row) => ({
      class: 'proof_pack_size_budget_failures',
      severity: 'release_blocking',
      artifact: row.path,
      action: `compact or remove oversized proof-pack artifact ${row.path}`,
      detail: `${row.failure}:size_bytes=${row.size_bytes};max_bytes=${row.max_bytes}`,
    })),
    ...manifestDuplicateWarnings.map((row) => ({
      class: 'manifest_duplicate_warnings',
      severity: 'hygiene',
      artifact: row.path,
      action: row.recommendation,
      detail: row.section,
    })),
  ]
    .sort((left, right) => blockerPriority(right) - blockerPriority(left))
    .slice(0, 20)
    .map((row, index) => ({
      ...row,
      issue_contract_version: 1,
      source_report: 'release_proof_pack_assemble',
      issue_lifecycle_state: 'candidate_open',
      source_artifacts: [args.manifestPath, String(row.artifact || '')].filter(Boolean),
      source_artifact_policy: 'local_relative_paths_only',
      rank: index + 1,
      priority_score: blockerPriority(row),
      dedupe_key: `release_proof_pack:${row.class}:${row.artifact}`,
      owner: blockerOwner(row),
      target_layer: blockerTargetLayer(row),
      release_gate_effect: blockerReleaseGateEffect(row),
      escalation_tier: blockerEscalationTier(row),
      operator_next_step: blockerOperatorNextStep(row),
      triage_queue: blockerTriageQueue(row),
      safe_to_auto_file_issue: true,
      safe_to_auto_apply_patch: false,
      human_review_required: true,
      autonomous_mitigation_allowed: false,
      requires_operator_ack: true,
      reopen_policy: 'reopen_if_blocker_dedupe_key_recurs',
      close_on_absence_window: 'next_release_proof_pack_assembly_pass',
      closing_evidence_required: 'proof-pack assembly rerun reports pass=true and this blocker dedupe key is absent',
      closure_verification_command:
        'node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/release_proof_pack_assemble.ts --strict=0',
    }));
  const primaryBlocker = topBlockers[0] || null;
  const primaryBlockerAction = primaryBlocker?.action || '';
  const primaryBlockerArtifact = primaryBlocker?.artifact || '';
  const primaryBlockerDedupeKey = primaryBlocker?.dedupe_key || '';
  const primaryBlockerPriorityScore = Number(primaryBlocker?.priority_score || 0);
  const primaryBlockerOwner = primaryBlocker?.owner || '';
  const primaryBlockerTargetLayer = primaryBlocker?.target_layer || '';
  const primaryBlockerEscalationTier = primaryBlocker?.escalation_tier || '';
  const primaryBlockerReleaseGateEffect = primaryBlocker?.release_gate_effect || '';
  const primaryBlockerOperatorNextStep = primaryBlocker?.operator_next_step || '';
  const primaryBlockerTriageQueue = primaryBlocker?.triage_queue || '';
  const primaryBlockerLifecycleState = primaryBlocker?.issue_lifecycle_state || '';
  const primaryBlockerSourceArtifactCount = Array.isArray(primaryBlocker?.source_artifacts)
    ? primaryBlocker.source_artifacts.length
    : 0;
  const primaryBlockerClosureVerificationCommand = primaryBlocker?.closure_verification_command || '';
  const topBlockerActions = topBlockers.slice(0, 3).map((row) => ({
    rank: row.rank,
    class: row.class,
    artifact: row.artifact,
    action: row.action,
    owner: row.owner,
    target_layer: row.target_layer,
    escalation_tier: row.escalation_tier,
    release_gate_effect: row.release_gate_effect,
    operator_next_step: row.operator_next_step,
    triage_queue: row.triage_queue,
    issue_lifecycle_state: row.issue_lifecycle_state,
    source_artifacts: row.source_artifacts,
    source_artifact_policy: row.source_artifact_policy,
    closure_verification_command: row.closure_verification_command,
  }));
  const topBlockerActionabilityFailures = topBlockers.filter((row) => !topBlockerActionable(row));
  const pass =
    requiredMissing.length === 0 &&
    mandatoryArtifactFailures.length === 0 &&
    categoryThresholdFailures.length === 0 &&
    requiredFailedArtifacts.length === 0 &&
    staleArtifactFailures.length === 0 &&
    currentEvidenceFailures.length === 0 &&
    proofPackSummaryConsistencyFailures.length === 0 &&
    proofPackSizeBudgetFailuresRows.length === 0 &&
    topBlockerActionabilityFailures.length === 0;
  const topBlockersActionable = topBlockerActionabilityFailures.length === 0;
  const issueCandidate =
    pass && manifestDuplicateWarnings.length === 0
      ? null
      : {
          type: 'release_proof_pack_issue_candidate',
          schema_version: 1,
          generated_at: new Date().toISOString(),
          status: 'candidate',
          source: 'release_proof_pack_assemble',
          fingerprint: `release_proof_pack:${args.version}:${[
            ...requiredMissing,
            ...requiredFailedArtifacts.map((row) => row.path),
            ...staleArtifactFailures.map((row) => row.path),
            ...currentEvidenceFailures.map((row) => row.path),
            ...proofPackSummaryConsistencyFailures.map((row) => row.id),
            ...proofPackSizeBudgetFailuresRows.map((row) => row.path),
            ...manifestDuplicateWarnings.map((row) => `${row.section}:${row.path}`),
          ].join('|')}`,
          dedupe_key: `release_proof_pack:${[
            ...requiredMissing,
            ...requiredFailedArtifacts.map((row) => row.path),
            ...staleArtifactFailures.map((row) => row.path),
            ...currentEvidenceFailures.map((row) => row.path),
            ...proofPackSummaryConsistencyFailures.map((row) => row.id),
            ...proofPackSizeBudgetFailuresRows.map((row) => row.path),
            ...manifestDuplicateWarnings.map((row) => `${row.section}:${row.path}`),
          ].join('|')}`,
          owner: 'release_governance/proof_pack',
          route_to: 'release_blocker_backlog',
          labels: ['release-proof-pack', 'release-gate', pass ? 'manifest-hygiene' : 'release-blocker'],
          title: pass
            ? 'Release proof-pack manifest has duplicate evidence entries'
            : 'Release proof-pack is not release-ready',
          severity: pass ? 'low' : 'release_blocking',
          impact: pass
            ? 'proof-pack assembly remains usable, but duplicate manifest entries add release-governance noise'
            : 'release evidence cannot be trusted or published until proof-pack blockers are resolved',
          source_artifacts: [args.manifestPath],
          required_missing_count: requiredMissing.length,
          required_failed_artifact_count: requiredFailedArtifacts.length,
          stale_artifact_failure_count: staleArtifactFailures.length,
          current_evidence_failure_count: currentEvidenceFailures.length,
          summary_consistency_failure_count: proofPackSummaryConsistencyFailures.length,
          proof_pack_size_budget_failure_count: proofPackSizeBudgetFailuresRows.length,
          category_required_missing_sum: categoryRequiredMissingSum,
          category_artifact_count_sum: categoryArtifactCountSum,
          category_required_total_sum: categoryRequiredTotalSum,
          manifest_duplicate_warning_count: manifestDuplicateWarnings.length,
          blocker_class_counts: blockerClassCounts,
          blocker_severity_counts: blockerSeverityCounts,
          release_blocking_issue_count: releaseBlockingIssueCount,
          primary_blocker_class: primaryBlockerClass,
          primary_blocker_record: primaryBlocker,
          primary_blocker_action: primaryBlockerAction,
          primary_blocker_artifact: primaryBlockerArtifact,
          primary_blocker_dedupe_key: primaryBlockerDedupeKey,
          primary_blocker_priority_score: primaryBlockerPriorityScore,
          primary_blocker_owner: primaryBlockerOwner,
          primary_blocker_target_layer: primaryBlockerTargetLayer,
          primary_blocker_escalation_tier: primaryBlockerEscalationTier,
          primary_blocker_release_gate_effect: primaryBlockerReleaseGateEffect,
          primary_blocker_operator_next_step: primaryBlockerOperatorNextStep,
          primary_blocker_triage_queue: primaryBlockerTriageQueue,
          primary_blocker_lifecycle_state: primaryBlockerLifecycleState,
          primary_blocker_source_artifact_count: primaryBlockerSourceArtifactCount,
          primary_blocker_closure_verification_command: primaryBlockerClosureVerificationCommand,
          top_blocker_actionability_failure_count: topBlockerActionabilityFailures.length,
          top_blockers_actionable: topBlockersActionable,
          top_blocker_actions: topBlockerActions,
          top_blockers: topBlockers,
          automation_policy: {
            mode: 'proposal_only',
            requires_release_authority_receipt_before_apply: true,
            autonomous_release_unblock_allowed: false,
          },
          triage: {
            state: 'ready_for_issue_synthesis',
            safe_to_auto_file_issue: true,
            safe_to_auto_apply_patch: false,
            requires_release_authority_receipt_to_close: true,
          },
          next_actions: [
            ...requiredMissing.map((pathToken) => ({
              action: `produce required proof artifact ${pathToken}`,
              artifact: pathToken,
            })),
            ...mandatoryArtifactFailures.map((row) => ({
              action: `repair mandatory proof artifact ${row.path}`,
              artifact: row.path,
              detail: row.failure,
            })),
            ...requiredFailedArtifacts.map((row) => ({
              action: `repair failing required artifact ${row.path}`,
              artifact: row.path,
              detail: row.failure,
            })),
            ...categoryThresholdFailures.map((row) => ({
              action: `restore proof-pack category completeness for ${row.category}`,
              artifact: row.category,
              detail: row.detail,
            })),
            ...staleArtifactFailures.map((row) => ({
              action: `refresh stale proof artifact ${row.path}`,
              artifact: row.path,
              detail: `age_hours=${row.age_hours};max_age_hours=${row.max_age_hours}`,
            })),
            ...currentEvidenceFailures.map((row) => ({
              action: `replace non-current proof evidence path ${row.path}`,
              artifact: row.path,
              detail: row.failure,
            })),
            ...proofPackSummaryConsistencyFailures.map((row) => ({
              action: 'repair proof-pack top-level/category required_missing mismatch',
              artifact: 'release_proof_pack_summary',
              detail: row.detail,
            })),
            ...proofPackSizeBudgetFailuresRows.map((row) => ({
              action: `compact or remove oversized proof-pack artifact ${row.path}`,
              artifact: row.path,
              detail: `${row.failure}:size_bytes=${row.size_bytes};max_bytes=${row.max_bytes}`,
            })),
            ...manifestDuplicateWarnings.map((row) => ({
              action: row.recommendation,
              artifact: row.path,
              section: row.section,
            })),
          ],
          acceptance_criteria: [
            'required_missing_count is zero',
            'required_failed_artifact_count is zero',
            'stale_artifact_failure_count is zero',
            'current_evidence_failure_count is zero',
            'summary_consistency_failure_count is zero',
            'proof_pack_size_budget_failure_count is zero',
            'manifest_duplicate_warning_count is zero or explicitly accepted as non-blocking hygiene debt',
          ],
        };

  const packManifest = {
    ok: pass,
    type: 'release_proof_pack_manifest',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    version: args.version,
    pack_root: packRoot,
    source_manifest_path: args.manifestPath,
    category_completeness_min: categoryCompletenessMin,
    artifacts: artifactRows,
    required_missing: requiredMissing,
    mandatory_artifact_failures: mandatoryArtifactFailures,
    category_threshold_failures: categoryThresholdFailures,
    required_failed_artifacts: requiredFailedArtifacts,
    stale_artifact_failures: staleArtifactFailures,
    current_evidence_failures: currentEvidenceFailures,
    proof_pack_size_budget: {
      policy_path: sizePolicy.policy_path,
      max_single_artifact_bytes: sizePolicy.max_single_artifact_bytes,
      max_proof_pack_bytes: sizePolicy.max_proof_pack_bytes,
      copied_artifact_bytes: sizeBudget.copiedArtifactBytes,
      compacted_artifact_count: artifactRows.filter((row) => row.compacted === true).length,
      failure_count: proofPackSizeBudgetFailuresRows.length,
    },
    proof_pack_size_budget_failures: proofPackSizeBudgetFailuresRows,
    manifest_duplicate_warnings: manifestDuplicateWarnings,
    manifest_hygiene_contract: {
      assembly_dedupes_duplicate_paths: true,
      duplicate_entries_are_reported: true,
      duplicate_entries_are_non_blocking_hygiene_debt: true,
      duplicate_copy_prevention_enabled: true,
      optional_required_overlap_prevented: true,
      required_artifacts_unique_count: requiredArtifacts.length,
      optional_artifacts_unique_count: optionalArtifacts.length,
      duplicate_warning_count: manifestDuplicateWarnings.length,
    },
    summary_consistency_contract: {
      top_level_required_missing: requiredMissing.length,
      category_required_missing_sum: categoryRequiredMissingSum,
      category_artifact_count_sum: categoryArtifactCountSum,
      category_required_total_sum: categoryRequiredTotalSum,
      mismatch_count: proofPackSummaryConsistencyFailures.length,
      top_level_and_category_counts_must_match: true,
    },
    blocker_class_counts: blockerClassCounts,
    blocker_severity_counts: blockerSeverityCounts,
    primary_blocker_record: primaryBlocker,
    primary_blocker_action: primaryBlockerAction,
    primary_blocker_artifact: primaryBlockerArtifact,
    primary_blocker_dedupe_key: primaryBlockerDedupeKey,
    primary_blocker_priority_score: primaryBlockerPriorityScore,
    primary_blocker_owner: primaryBlockerOwner,
    primary_blocker_target_layer: primaryBlockerTargetLayer,
    primary_blocker_escalation_tier: primaryBlockerEscalationTier,
    primary_blocker_release_gate_effect: primaryBlockerReleaseGateEffect,
    primary_blocker_operator_next_step: primaryBlockerOperatorNextStep,
    primary_blocker_triage_queue: primaryBlockerTriageQueue,
    primary_blocker_lifecycle_state: primaryBlockerLifecycleState,
    primary_blocker_source_artifact_count: primaryBlockerSourceArtifactCount,
    primary_blocker_closure_verification_command: primaryBlockerClosureVerificationCommand,
    top_blocker_actionability_failures: topBlockerActionabilityFailures,
    top_blocker_actionability_failure_count: topBlockerActionabilityFailures.length,
    top_blockers_actionable: topBlockersActionable,
    top_blocker_actions: topBlockerActions,
    top_blockers: topBlockers,
    issue_candidate: issueCandidate,
    issue_candidate_contract: {
      candidate_schema_version: 1,
      safe_to_auto_file_issue: true,
      safe_to_auto_apply_patch: false,
      release_authority_receipt_required_to_close: true,
    },
    operator_summary: {
      pass,
      required_missing_count: requiredMissing.length,
      required_failed_artifact_count: requiredFailedArtifacts.length,
      mandatory_artifact_failure_count: mandatoryArtifactFailures.length,
      category_threshold_failure_count: categoryThresholdFailures.length,
      stale_artifact_failure_count: staleArtifactFailures.length,
      current_evidence_failure_count: currentEvidenceFailures.length,
      summary_consistency_failure_count: proofPackSummaryConsistencyFailures.length,
      proof_pack_size_budget_failure_count: proofPackSizeBudgetFailuresRows.length,
      category_required_missing_sum: categoryRequiredMissingSum,
      category_artifact_count_sum: categoryArtifactCountSum,
      category_required_total_sum: categoryRequiredTotalSum,
      manifest_duplicate_warning_count: manifestDuplicateWarnings.length,
      release_blocking_issue_count: releaseBlockingIssueCount,
      blocker_class_counts: blockerClassCounts,
      blocker_severity_counts: blockerSeverityCounts,
      primary_blocker_class: primaryBlockerClass,
      primary_blocker_record: primaryBlocker,
      primary_blocker_action: primaryBlockerAction,
      primary_blocker_artifact: primaryBlockerArtifact,
      primary_blocker_dedupe_key: primaryBlockerDedupeKey,
      primary_blocker_priority_score: primaryBlockerPriorityScore,
      primary_blocker_owner: primaryBlockerOwner,
      primary_blocker_target_layer: primaryBlockerTargetLayer,
      primary_blocker_escalation_tier: primaryBlockerEscalationTier,
      primary_blocker_release_gate_effect: primaryBlockerReleaseGateEffect,
      primary_blocker_operator_next_step: primaryBlockerOperatorNextStep,
      primary_blocker_triage_queue: primaryBlockerTriageQueue,
      primary_blocker_lifecycle_state: primaryBlockerLifecycleState,
      primary_blocker_source_artifact_count: primaryBlockerSourceArtifactCount,
      primary_blocker_closure_verification_command: primaryBlockerClosureVerificationCommand,
      top_blocker_actionability_failure_count: topBlockerActionabilityFailures.length,
      top_blockers_actionable: topBlockersActionable,
      issue_candidate_count: issueCandidate ? 1 : 0,
      top_blockers: topBlockers,
      top_blocker_actions: topBlockerActions,
      primary_blocker:
        requiredMissing[0] ||
        mandatoryArtifactFailures[0]?.path ||
        requiredFailedArtifacts[0]?.path ||
        categoryThresholdFailures[0]?.category ||
        staleArtifactFailures[0]?.path ||
        currentEvidenceFailures[0]?.path ||
        proofPackSummaryConsistencyFailures[0]?.id ||
        proofPackSizeBudgetFailuresRows[0]?.path ||
        manifestDuplicateWarnings[0]?.path ||
        '',
      issue_candidate_ready: issueCandidate !== null,
      next_actions: issueCandidate?.next_actions || [],
    },
    category_summary: categorySummary,
  };

  const packManifestPath = path.resolve(packRoot, 'manifest.json');
  ensureParent(packManifestPath);
  fs.writeFileSync(packManifestPath, `${JSON.stringify(packManifest, null, 2)}\n`, 'utf8');

  const reportPath = path.resolve(packRoot, 'README.md');
  writeTextArtifact(
    reportPath,
    markdown({
      ...packManifest,
      summary: {
        required_missing: requiredMissing.length,
        stale_artifacts: staleArtifactFailures.length,
        failed_artifacts: requiredFailedArtifacts.length + mandatoryArtifactFailures.length,
        historical_snapshot_artifacts: currentEvidenceFailures.length,
        summary_consistency_failures: proofPackSummaryConsistencyFailures.length,
        proof_pack_size_budget_failures: proofPackSizeBudgetFailuresRows.length,
        category_required_missing_sum: categoryRequiredMissingSum,
        category_artifact_count_sum: categoryArtifactCountSum,
        category_required_total_sum: categoryRequiredTotalSum,
        release_blocking_issue_count: releaseBlockingIssueCount,
        top_blocker_count: topBlockers.length,
        primary_blocker_class: primaryBlockerClass,
        primary_blocker_action: primaryBlockerAction,
        primary_blocker_artifact: primaryBlockerArtifact,
        primary_blocker_dedupe_key: primaryBlockerDedupeKey,
        primary_blocker_priority_score: primaryBlockerPriorityScore,
        primary_blocker_owner: primaryBlockerOwner,
        primary_blocker_target_layer: primaryBlockerTargetLayer,
        primary_blocker_escalation_tier: primaryBlockerEscalationTier,
        primary_blocker_release_gate_effect: primaryBlockerReleaseGateEffect,
        primary_blocker_operator_next_step: primaryBlockerOperatorNextStep,
        primary_blocker_triage_queue: primaryBlockerTriageQueue,
        primary_blocker_lifecycle_state: primaryBlockerLifecycleState,
        primary_blocker_source_artifact_count: primaryBlockerSourceArtifactCount,
        primary_blocker_closure_verification_command: primaryBlockerClosureVerificationCommand,
        top_blocker_actionability_failure_count: topBlockerActionabilityFailures.length,
        top_blockers_actionable: topBlockersActionable,
        top_blocker_action_count: topBlockerActions.length,
      },
      top_blockers: topBlockers,
    }),
  );

  const report = {
    ok: pass,
    type: 'release_proof_pack_assemble',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    version: args.version,
    pack_root: packRoot,
    source_manifest_path: args.manifestPath,
    summary: {
      artifact_count: artifactRows.length,
      required_missing: requiredMissing.length,
      mandatory_artifact_failure_count: mandatoryArtifactFailures.length,
      required_failed_artifact_count: requiredFailedArtifacts.length,
      stale_artifact_count: staleArtifactFailures.length,
      historical_or_noncurrent_artifact_count: currentEvidenceFailures.length,
      category_threshold_failure_count: categoryThresholdFailures.length,
      summary_consistency_failure_count: proofPackSummaryConsistencyFailures.length,
      proof_pack_size_budget_failure_count: proofPackSizeBudgetFailuresRows.length,
      category_required_missing_sum: categoryRequiredMissingSum,
      category_artifact_count_sum: categoryArtifactCountSum,
      category_required_total_sum: categoryRequiredTotalSum,
      release_blocking_issue_count: releaseBlockingIssueCount,
      primary_blocker_class: primaryBlockerClass,
      primary_blocker_record: primaryBlocker,
      primary_blocker_action: primaryBlockerAction,
      primary_blocker_artifact: primaryBlockerArtifact,
      primary_blocker_dedupe_key: primaryBlockerDedupeKey,
      primary_blocker_priority_score: primaryBlockerPriorityScore,
      primary_blocker_owner: primaryBlockerOwner,
      primary_blocker_target_layer: primaryBlockerTargetLayer,
      primary_blocker_escalation_tier: primaryBlockerEscalationTier,
      primary_blocker_release_gate_effect: primaryBlockerReleaseGateEffect,
      primary_blocker_operator_next_step: primaryBlockerOperatorNextStep,
      primary_blocker_triage_queue: primaryBlockerTriageQueue,
      primary_blocker_lifecycle_state: primaryBlockerLifecycleState,
      primary_blocker_source_artifact_count: primaryBlockerSourceArtifactCount,
      primary_blocker_closure_verification_command: primaryBlockerClosureVerificationCommand,
      top_blocker_actionability_failure_count: topBlockerActionabilityFailures.length,
      top_blockers_actionable: topBlockersActionable,
      blocker_severity_counts: blockerSeverityCounts,
      top_blocker_count: topBlockers.length,
      pass,
    },
    category_completeness_min: categoryCompletenessMin,
    artifacts: artifactRows,
    category_summary: categorySummary,
    mandatory_artifact_failures: mandatoryArtifactFailures,
    required_failed_artifacts: requiredFailedArtifacts,
    stale_artifact_failures: staleArtifactFailures,
    current_evidence_failures: currentEvidenceFailures,
    summary_consistency_failures: proofPackSummaryConsistencyFailures,
    proof_pack_size_budget: {
      policy_path: sizePolicy.policy_path,
      max_single_artifact_bytes: sizePolicy.max_single_artifact_bytes,
      max_proof_pack_bytes: sizePolicy.max_proof_pack_bytes,
      copied_artifact_bytes: sizeBudget.copiedArtifactBytes,
      compacted_artifact_count: artifactRows.filter((row) => row.compacted === true).length,
      failure_count: proofPackSizeBudgetFailuresRows.length,
    },
    proof_pack_size_budget_failures: proofPackSizeBudgetFailuresRows,
    blocker_class_counts: blockerClassCounts,
    blocker_severity_counts: blockerSeverityCounts,
    top_blockers: topBlockers,
    category_threshold_failures: categoryThresholdFailures,
    failures: [
      ...requiredMissing.map((detail) => ({ id: 'proof_pack_required_artifact_missing', detail })),
      ...mandatoryArtifactFailures.map((row) => ({
        id: 'proof_pack_mandatory_artifact_violation',
        detail: `${row.path}:${row.failure}`,
      })),
      ...requiredFailedArtifacts.map((row) => ({
        id: 'proof_pack_required_artifact_failed',
        detail: `${row.path}:${row.failure}`,
      })),
      ...staleArtifactFailures.map((row) => ({
        id: 'proof_pack_required_artifact_stale',
        detail: `${row.path}:age_hours=${row.age_hours.toFixed(3)};max_age_hours=${row.max_age_hours}`,
      })),
      ...currentEvidenceFailures.map((row) => ({
        id: 'proof_pack_current_evidence_contract_failed',
        detail: `${row.path}:${row.failure}`,
      })),
      ...proofPackSummaryConsistencyFailures.map((row) => ({ id: row.id, detail: row.detail })),
      ...proofPackSizeBudgetFailuresRows.map((row) => ({
        id: row.failure,
        detail: `${row.path}:size_bytes=${row.size_bytes};max_bytes=${row.max_bytes}`,
      })),
      ...categoryThresholdFailures.map((row) => ({ id: row.id, detail: row.detail })),
      ...topBlockerActionabilityFailures.map((row) => ({
        id: 'proof_pack_top_blocker_actionability_failed',
        detail: `${row.dedupe_key || row.class}:${row.artifact}`,
      })),
    ],
    artifact_paths: [packManifestPath, reportPath],
  };

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
