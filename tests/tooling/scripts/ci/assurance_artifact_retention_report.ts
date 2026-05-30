import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type FileRow = {
  rel: string;
  bytes: number;
  mtime_ms: number;
  age_days: number;
  prefix: string;
  is_canonical_ref: boolean;
  raw_marker_hits: string[];
};

const root = process.cwd();
function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((item) => item.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 && process.argv[idx + 1] ? process.argv[idx + 1] : fallback;
}

const policyRel = arg("policy", "validation/conformance/contracts/assurance_artifact_retention_policy.json");
const policyPath = path.join(root, policyRel);
const outRel = arg("out-json", `validation/reports/assurance_artifact_retention_report_${new Date().toISOString().slice(0, 10)}.json`);
const outPath = path.join(root, outRel);

function readJson(rel: string): Json {
  return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
}

function walk(dir: string): string[] {
  const abs = path.join(root, dir);
  if (!fs.existsSync(abs)) return [];
  const out: string[] = [];
  for (const entry of fs.readdirSync(abs, { withFileTypes: true })) {
    const rel = path.join(dir, entry.name);
    const childAbs = path.join(root, rel);
    let stat: fs.Stats;
    try {
      stat = fs.lstatSync(childAbs);
    } catch {
      continue;
    }
    if (stat.isSymbolicLink()) continue;
    if (stat.isDirectory()) out.push(...walk(rel));
    else if (stat.isFile()) out.push(rel);
  }
  return out;
}

function prefixFor(rel: string): string {
  const base = path.basename(rel).replace(/\.(json|jsonl|md|txt)$/i, "");
  return base
    .replace(/_\d{4}-\d{2}-\d{2}.*$/, "")
    .replace(/_\d{8}T\d{6}Z?.*$/, "")
    .replace(/_\d{8}_\d{6}.*$/, "")
    .replace(/_\d{13}.*$/, "")
    .replace(/_current$/, "");
}

function isCanonicalRef(rel: string, suffixes: string[]): boolean {
  const base = path.basename(rel).replace(/\.(json|jsonl|md|txt)$/i, "");
  return suffixes.some((suffix) => base.endsWith(String(suffix)));
}

function markerHits(rel: string, markers: string[]): string[] {
  const abs = path.join(root, rel);
  let text = "";
  try {
    text = fs.readFileSync(abs, "utf8").slice(0, 2_000_000);
  } catch {
    return [];
  }
  return markers.filter((marker) => text.includes(marker));
}

function allowedExtension(rel: string, cfg: Json): boolean {
  const configured = Array.isArray(cfg.allowed_extensions) ? cfg.allowed_extensions.map(String) : [];
  if (configured.includes("*")) return true;
  const allowed = configured.length > 0 ? configured : [".json", ".jsonl", ".md", ".txt"];
  return allowed.some((ext) => rel.toLowerCase().endsWith(ext.toLowerCase()));
}

const policy = readJson(policyRel);
const markers = Array.isArray(policy.raw_evidence_markers) ? policy.raw_evidence_markers.map(String) : [];
const roots = Array.isArray(policy.roots) ? policy.roots : [];
const latestRefPolicy = policy.latest_ref_policy && typeof policy.latest_ref_policy === "object"
  ? policy.latest_ref_policy as Json
  : {};
const canonicalSuffixes = Array.isArray(latestRefPolicy.canonical_ref_suffixes)
  ? latestRefPolicy.canonical_ref_suffixes.map(String)
  : ["_current", "_latest"];
const latestRefIndexRel = arg("latest-index", String(latestRefPolicy.index_path || "core/local/artifacts/local_artifact_retention_latest_refs_current.json"));
const nowMs = Date.now();
const allFiles: FileRow[] = [];
const rootReports = roots.map((entry) => {
  const cfg = entry as Json;
  const rootPath = String(cfg.path || "");
  const rawEvidenceAllowed = cfg.raw_evidence_allowed === true;
  const cleanupStrategy = String(cfg.cleanup_strategy || "file_retention");
  const files: FileRow[] = walk(rootPath)
    .filter((rel) => allowedExtension(rel, cfg))
    .flatMap((rel) => {
      let stat: fs.Stats;
      try {
        stat = fs.statSync(path.join(root, rel));
      } catch {
        return [];
      }
      return [{
        rel,
        bytes: stat.size,
        mtime_ms: stat.mtimeMs,
        age_days: Math.max(0, (nowMs - stat.mtimeMs) / 86_400_000),
        prefix: prefixFor(rel),
        is_canonical_ref: isCanonicalRef(rel, canonicalSuffixes),
        raw_marker_hits: rawEvidenceAllowed ? [] : markerHits(rel, markers),
      }];
    });
  allFiles.push(...files);
  const totalBytes = files.reduce((sum, row) => sum + row.bytes, 0);
  const maxFileBytes = Number(cfg.max_file_bytes || 0);
  const maxTotalBytes = Number(cfg.max_total_bytes || 0);
  const retainLatest = Number(cfg.retain_latest_per_prefix || 0);
  const maxAgeDays = Number(cfg.max_age_days || 0);
  const byPrefix = new Map<string, FileRow[]>();
  for (const file of files) {
    const rows = byPrefix.get(file.prefix) || [];
    rows.push(file);
    byPrefix.set(file.prefix, rows);
  }
  const cleanupCandidates = new Map<string, { row: FileRow; reasons: string[] }>();
  function addCleanupCandidate(row: FileRow, reason: string): void {
    const existing = cleanupCandidates.get(row.rel);
    if (existing) {
      if (!existing.reasons.includes(reason)) existing.reasons.push(reason);
      return;
    }
    cleanupCandidates.set(row.rel, { row, reasons: [reason] });
  }
  const latestRefs = [];
  if (cleanupStrategy !== "delete_root_contents" && cleanupStrategy !== "report_only") {
    for (const [prefix, rows] of byPrefix.entries()) {
      rows.sort((a, b) => b.mtime_ms - a.mtime_ms);
      const newest = rows[0];
      const canonical = rows.find((row) => row.is_canonical_ref);
      latestRefs.push({
        prefix,
        newest: newest?.rel || null,
        canonical_ref: canonical?.rel || null,
        file_count: rows.length,
      });
      for (const row of rows.slice(retainLatest)) {
        if (!row.is_canonical_ref) addCleanupCandidate(row, "prefix_retention_window_exceeded");
      }
    }
  }
  if (maxAgeDays > 0 && cleanupStrategy !== "delete_root_contents" && cleanupStrategy !== "report_only") {
    for (const row of files) {
      if (row.age_days > maxAgeDays && !row.is_canonical_ref) addCleanupCandidate(row, "age_window_exceeded");
    }
  }
  const rootAgeDays = files.length > 0
    ? Math.max(0, (nowMs - Math.max(...files.map((row) => row.mtime_ms))) / 86_400_000)
    : 0;
  const directoryCleanupReasons: string[] = [];
  if (cleanupStrategy === "delete_root_contents") {
    if (maxTotalBytes > 0 && totalBytes > maxTotalBytes) directoryCleanupReasons.push("root_total_budget_exceeded");
    if (maxAgeDays > 0 && rootAgeDays > maxAgeDays) directoryCleanupReasons.push("root_age_window_exceeded");
  }
  const cleanupCandidateRows = Array.from(cleanupCandidates.values()).map(({ row, reasons }) => ({
    rel: row.rel,
    kind: "file",
    bytes: row.bytes,
    prefix: row.prefix,
    age_days: Number(row.age_days.toFixed(2)),
    reasons,
  }));
  if (cleanupStrategy === "delete_root_contents" && directoryCleanupReasons.length > 0 && totalBytes > 0) {
    cleanupCandidateRows.push({
      rel: rootPath,
      kind: "directory_contents",
      bytes: totalBytes,
      prefix: rootPath.replace(/[\/\\]/g, "_"),
      age_days: Number(rootAgeDays.toFixed(2)),
      reasons: directoryCleanupReasons,
    });
  }
  return {
    path: rootPath,
    cleanup_strategy: cleanupStrategy,
    file_count: files.length,
    total_bytes: totalBytes,
    max_total_bytes: maxTotalBytes,
    over_total_budget: maxTotalBytes > 0 && totalBytes > maxTotalBytes,
    oversize_files: files.filter((row) => maxFileBytes > 0 && row.bytes > maxFileBytes).map((row) => ({
      rel: row.rel,
      bytes: row.bytes,
    })),
    raw_marker_files: files.filter((row) => row.raw_marker_hits.length > 0).map((row) => ({
      rel: row.rel,
      hits: row.raw_marker_hits,
    })),
    cleanup_candidate_count: cleanupCandidateRows.length,
    cleanup_candidates: cleanupCandidateRows,
    canonical_latest_ref_count: files.filter((row) => row.is_canonical_ref).length,
    latest_refs: latestRefs,
  };
});

const latestByPrefix = new Map<string, FileRow>();
for (const row of allFiles) {
  const existing = latestByPrefix.get(row.prefix);
  if (!existing || row.mtime_ms > existing.mtime_ms) latestByPrefix.set(row.prefix, row);
}
const latestRefIndex = {
  trace_id: `observability:${new Date().toISOString()}:local-artifact-latest-refs`,
  span_id: `span:observability:${new Date().toISOString()}:local-artifact-latest-refs`,
  source_domain: String(latestRefPolicy.index_owner_domain || "observability"),
  type: "local_artifact_retention_latest_refs",
  generated_at: new Date().toISOString(),
  policy_path: policyRel,
  root_count: rootReports.length,
  ref_count: latestByPrefix.size,
  refs: Array.from(latestByPrefix.entries())
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([prefix, row]) => ({
      prefix,
      root_path: roots.map((entry) => String((entry as Json).path || "")).find((rootPath) => row.rel.startsWith(`${rootPath}/`) || row.rel === rootPath) || null,
      latest_path: row.rel,
      canonical_ref: row.is_canonical_ref ? row.rel : null,
      bytes: row.bytes,
      mtime_ms: row.mtime_ms,
      is_canonical_ref: row.is_canonical_ref,
    })),
};

const cleanupCandidates = rootReports.flatMap((row) => Array.isArray(row.cleanup_candidates) ? row.cleanup_candidates : []);
const cleanupCandidateTotal = cleanupCandidates.length;
const cleanupCandidateBytes = cleanupCandidates.reduce((sum, row) => sum + Number((row as Json).bytes || 0), 0);
const cleanupBudget = policy.cleanup_candidate_budget && typeof policy.cleanup_candidate_budget === "object"
  ? policy.cleanup_candidate_budget as Json
  : {};
const redAt = Number(cleanupBudget.max_total_candidates_before_red || 1000);
const yellowAt = Number(cleanupBudget.max_total_candidates_before_yellow || 500);
const cleanupCandidatePressure = {
  severity: cleanupCandidateTotal >= redAt ? "red" : cleanupCandidateTotal >= yellowAt ? "yellow" : "green",
  total: cleanupCandidateTotal,
  bytes: cleanupCandidateBytes,
  yellow_at: yellowAt,
  red_at: redAt,
};
const actions = policy.actions && typeof policy.actions === "object" ? policy.actions as Json : {};
const applyPolicy = policy.apply && typeof policy.apply === "object" ? policy.apply as Json : {};

const payload = {
  trace_id: `validation:${new Date().toISOString()}:assurance-artifact-retention`,
  span_id: `span:validation:${new Date().toISOString()}:assurance-artifact-retention`,
  source_domain: "validation",
  ok: true,
  type: "assurance_artifact_retention_report",
  generated_at: new Date().toISOString(),
  policy_path: policyRel,
  latest_ref_index_path: latestRefIndexRel,
  latest_ref_count: latestRefIndex.ref_count,
  cleanup_candidate_total: cleanupCandidateTotal,
  cleanup_candidate_bytes: cleanupCandidateBytes,
  cleanup_candidate_pressure: cleanupCandidatePressure,
  enforcement: {
    apply_script_path: String(applyPolicy.script_path || ""),
    live_cleanup_requires_ack: actions.live_cleanup_requires_ack === true,
    dry_run_default: actions.dry_run_default !== false,
    delete_automatically: actions.delete_automatically === true,
  },
  roots: rootReports,
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
if (latestRefPolicy.emit_index !== false) {
  const latestRefIndexPath = path.join(root, latestRefIndexRel);
  fs.mkdirSync(path.dirname(latestRefIndexPath), { recursive: true });
  fs.writeFileSync(latestRefIndexPath, `${JSON.stringify(latestRefIndex, null, 2)}\n`);
}
console.log(JSON.stringify({ ok: true, type: payload.type, report_path: outRel, latest_ref_index_path: latestRefIndexRel, roots: rootReports.length }, null, 2));
