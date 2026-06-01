import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyRel = "observability/sentinel/sentinel_feedback_summary_policy.json";
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), "utf8")) as Json;
const inputs = (policy.inputs || {}) as Record<string, string>;
const outputRel = String(policy.output_path || "core/local/artifacts/kernel_sentinel_feedback_summary_current.json");
const strict = process.argv.includes("--strict=1");

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function writeJson(rel: string, payload: unknown): void {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function ageMs(payload: Json | null): number | null {
  const parsed = Date.parse(String(payload?.generated_at || ""));
  return Number.isFinite(parsed) ? Math.max(0, Date.now() - parsed) : null;
}

const issueQuality = readJson(inputs.issue_quality_report);
const stageRunner = readJson(inputs.stage_runner_report);
const freshGuard = readJson(inputs.fresh_evidence_guard);
const traceGuard = readJson(inputs.trace_completeness_guard);
const worktreeDanger = readJson(inputs.worktree_danger_report);
const candidates = Array.isArray(issueQuality?.candidates) ? (issueQuality.candidates as Json[]) : [];
const maxFindings = Math.max(1, Number(policy.max_summary_findings || 5));
const freshnessBudgetMs = Number(policy.freshness_budget_ms || 604_800_000);
const issueAgeMs = ageMs(issueQuality);
const stageAgeMs = ageMs(stageRunner);
const promotionReady = candidates.filter((row) => row.promotion_state === "promotion_ready");
const humanReview = candidates.filter((row) => row.promotion_state === "human_review");
const observationOnly = candidates.filter((row) => row.promotion_state === "observation_only");
const actionable = [...promotionReady, ...humanReview].slice(0, maxFindings);
const violations: string[] = [];

if (!issueQuality) violations.push("missing_issue_quality_report");
if (!stageRunner) violations.push("missing_stage_runner_report");
if (issueAgeMs === null || issueAgeMs > freshnessBudgetMs) violations.push("issue_quality_report_stale");
if (stageAgeMs === null || stageAgeMs > freshnessBudgetMs) violations.push("stage_runner_report_stale");
if (freshGuard && freshGuard.ok === false) violations.push("fresh_evidence_guard_failed");
if (traceGuard && traceGuard.ok === false) violations.push("trace_completeness_guard_failed");

const worktreeFindings = Array.isArray(worktreeDanger?.findings) ? (worktreeDanger.findings as Json[]) : [];
const blockingWorktreeFindings = worktreeFindings.filter((row) => {
  const severity = String(row.severity || "").toLowerCase();
  if (severity === "critical" || severity === "high") return true;
  return row.id !== "branch_divergence_present" && row.actionable === true;
});
if (blockingWorktreeFindings.length > 0) violations.push("worktree_danger_detected");
const topWorktreeFindings = worktreeFindings.slice(0, maxFindings).map((row) => ({
  id: row.id,
  severity: row.severity,
  owner_guess: row.owner_guess,
  root_cause_cluster_key: row.root_cause_cluster_key,
  evidence_refs: Array.isArray(row.evidence_refs) ? row.evidence_refs.slice(0, 5) : [],
  next_action: row.next_action,
}));
const actionableWorktreeCount = worktreeFindings.filter((row) => row.actionable === true).length;

const result = {
  trace_id: `observability:${new Date().toISOString()}:kernel-sentinel-feedback-summary`,
  parent_span_id: String(issueQuality?.trace_id || stageRunner?.trace_id || ""),
  source_domain: "observability",
  type: "kernel_sentinel_feedback_summary",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  policy_path: policyRel,
  status:
    blockingWorktreeFindings.length > 0
      ? "worktree_danger_needs_attention"
      : worktreeDanger?.ok === false
      ? "worktree_advisory"
      : violations.length > 0
      ? "needs_attention"
      : actionable.length > 0
        ? "actionable_feedback_ready_for_human_review"
        : "healthy_no_actionable_feedback",
  candidate_count: Number(issueQuality?.candidate_count || candidates.length || 0),
  promotion_ready_count: promotionReady.length,
  human_review_count: humanReview.length,
  observation_only_count: observationOnly.length,
  actionable_feedback_count: actionable.length,
  worktree_danger_count: Number(worktreeDanger?.finding_count || worktreeFindings.length || 0),
  actionable_worktree_danger_count: actionableWorktreeCount,
  blocking_worktree_danger_count: blockingWorktreeFindings.length,
  stage_completed: stageRunner?.ok === true && Number(stageRunner?.remaining_phase_count || 0) === 0,
  stage_remaining_phase_count: Number(stageRunner?.remaining_phase_count ?? -1),
  average_score: issueQuality?.average_score ?? null,
  report_ages_ms: {
    issue_quality: issueAgeMs,
    stage_runner: stageAgeMs,
  },
  top_actionable_findings: actionable.map((row) => ({
    id: row.id,
    promotion_state: row.promotion_state,
    confidence_band: row.confidence_band,
    owner_guess: row.owner_guess,
    root_cause_cluster_key: row.root_cause_cluster_key,
    score: row.score,
    evidence_refs: Array.isArray(row.evidence_refs) ? row.evidence_refs.slice(0, 5) : [],
    next_action: row.next_action,
  })),
  top_worktree_danger_findings: topWorktreeFindings,
  source_refs: {
    issue_quality_report: inputs.issue_quality_report,
    stage_runner_report: inputs.stage_runner_report,
    fresh_evidence_guard: inputs.fresh_evidence_guard,
    trace_completeness_guard: inputs.trace_completeness_guard,
    worktree_danger_report: inputs.worktree_danger_report,
  },
  violations,
};

writeJson(outputRel, result);
console.log(JSON.stringify(result, null, 2));
if (strict && violations.length > 0) process.exitCode = 1;
