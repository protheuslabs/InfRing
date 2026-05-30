import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

type Json = Record<string, unknown>;

const root = process.cwd();

function arg(name: string, fallback = ""): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((item) => item.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 && process.argv[idx + 1] ? process.argv[idx + 1] : fallback;
}

function flag(name: string): boolean {
  return arg(name, "0") === "1" || process.argv.includes(`--${name}`);
}

function readJson(rel: string): Json {
  return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
}

function safeRel(rel: string): boolean {
  const normalized = rel.replace(/\\/g, "/");
  if (!normalized || normalized.startsWith("/") || normalized.includes("..")) return false;
  const allowedRoots = [
    "core/local/artifacts/",
    "validation/reports/",
    "observability/reports/",
    "local/tmp/coding-agent-system-pulls",
    "target/debug/incremental",
    "target/debug/deps",
  ];
  return allowedRoots.some((allowed) => normalized === allowed.replace(/\/$/, "") || normalized.startsWith(allowed));
}

function removeDirectoryContents(abs: string): number {
  if (!fs.existsSync(abs)) return 0;
  let removed = 0;
  for (const entry of fs.readdirSync(abs)) {
    fs.rmSync(path.join(abs, entry), { recursive: true, force: true });
    removed += 1;
  }
  return removed;
}

function trackedFiles(): Set<string> {
  const result = spawnSync("git", ["ls-files"], { cwd: root, encoding: "utf8" });
  if (result.status !== 0) return new Set();
  return new Set(result.stdout.split(/\r?\n/).filter(Boolean));
}

function isTrackedFile(rel: string, tracked: Set<string>): boolean {
  return tracked.has(rel.replace(/\\/g, "/"));
}

function isCanonicalCurrentRef(rel: string): boolean {
  const base = path.basename(rel).replace(/\.(json|jsonl|md|txt)$/i, "");
  return base.endsWith("_current") || base.endsWith("_latest");
}

function directoryContainsTrackedFiles(rel: string, tracked: Set<string>): boolean {
  const normalized = rel.replace(/\\/g, "/").replace(/\/$/, "");
  return Array.from(tracked).some((file) => file === normalized || file.startsWith(`${normalized}/`));
}

const policyRel = arg("policy", "validation/conformance/contracts/assurance_artifact_retention_policy.json");
const reportRel = arg("report", "core/local/artifacts/assurance_artifact_retention_report_current.json");
const policy = readJson(policyRel);
const report = readJson(reportRel);
const applyPolicy = policy.apply && typeof policy.apply === "object" ? policy.apply as Json : {};
const maxDeletePerRun = Number(applyPolicy.max_delete_per_run || 500);
const apply = flag("apply");
const ack = flag("ack-retention-cleanup");
const dryRun = !apply || !ack;
const tracked = trackedFiles();

const candidates: Json[] = [];
for (const rootReport of Array.isArray(report.roots) ? report.roots as Json[] : []) {
  for (const candidate of Array.isArray(rootReport.cleanup_candidates) ? rootReport.cleanup_candidates as Json[] : []) {
    candidates.push({ ...candidate, root_path: rootReport.path });
  }
}

const actions: Json[] = [];
let deletedEntries = 0;
let reclaimedBytes = 0;
for (const candidate of candidates.slice(0, maxDeletePerRun)) {
  const rel = String(candidate.rel || "");
  const kind = String(candidate.kind || "file");
  const bytes = Number(candidate.bytes || 0);
  const action: Json = { rel, kind, bytes, dry_run: dryRun, ok: false, skipped: false };
  if (!safeRel(rel)) {
    action.skipped = true;
    action.reason = "unsafe_path";
    actions.push(action);
    continue;
  }
  const abs = path.join(root, rel);
  if (!fs.existsSync(abs)) {
    action.skipped = true;
    action.reason = "missing";
    actions.push(action);
    continue;
  }
  if (dryRun) {
    action.ok = true;
    action.skipped = true;
    action.reason = apply ? "missing_ack_retention_cleanup" : "dry_run";
    actions.push(action);
    continue;
  }
  if (kind === "directory_contents") {
    if (directoryContainsTrackedFiles(rel, tracked)) {
      action.ok = true;
      action.skipped = true;
      action.reason = "tracked_files_protected";
      actions.push(action);
      continue;
    }
    deletedEntries += removeDirectoryContents(abs);
  } else {
    if (isCanonicalCurrentRef(rel)) {
      action.ok = true;
      action.skipped = true;
      action.reason = "canonical_current_ref_protected";
      actions.push(action);
      continue;
    }
    if (isTrackedFile(rel, tracked)) {
      action.ok = true;
      action.skipped = true;
      action.reason = "tracked_file_protected";
      actions.push(action);
      continue;
    }
    fs.rmSync(abs, { recursive: false, force: true });
    deletedEntries += 1;
  }
  reclaimedBytes += bytes;
  action.ok = true;
  actions.push(action);
}

const payload = {
  trace_id: `validation:${new Date().toISOString()}:assurance-artifact-retention-apply`,
  span_id: `span:validation:${new Date().toISOString()}:assurance-artifact-retention-apply`,
  parent_span_id: String(report.span_id || report.trace_id || ""),
  source_domain: "validation",
  type: "assurance_artifact_retention_apply",
  ok: true,
  generated_at: new Date().toISOString(),
  policy_path: policyRel,
  report_path: reportRel,
  dry_run: dryRun,
  apply_requested: apply,
  ack_present: ack,
  candidate_count: candidates.length,
  action_count: actions.length,
  deleted_entries: deletedEntries,
  reclaimed_bytes_estimate: reclaimedBytes,
  actions,
};

const outRel = arg("out-json", String(applyPolicy.artifact_path || "core/local/artifacts/assurance_artifact_retention_apply_current.json"));
const outPath = path.join(root, outRel);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({
  ok: true,
  dry_run: payload.dry_run,
  candidate_count: payload.candidate_count,
  action_count: payload.action_count,
  deleted_entries: payload.deleted_entries,
  reclaimed_bytes_estimate: payload.reclaimed_bytes_estimate,
  out_json: outRel,
}, null, 2));
