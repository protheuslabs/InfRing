import fs from "node:fs";
import os from "node:os";
import path from "node:path";

type Json = Record<string, unknown>;
type Candidate = {
  id: string;
  source_id: string;
  kind: "file" | "directory";
  abs_path: string;
  display_path: string;
  bytes: number;
  role: string;
  cleanup_strategy: string;
};

const repoRoot = process.cwd();
const homeRoot = os.homedir();

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
  return JSON.parse(fs.readFileSync(path.join(repoRoot, rel), "utf8")) as Json;
}

function resolveBase(raw: string): string {
  return path.resolve(
    raw
      .replace(/^\$REPO(?=\/|$)/, repoRoot)
      .replace(/^\$HOME(?=\/|$)/, homeRoot),
  );
}

function safeDescendant(abs: string, base: string): boolean {
  const resolved = path.resolve(abs);
  const resolvedBase = path.resolve(base);
  const rel = path.relative(resolvedBase, resolved);
  return rel === "" || (!!rel && !rel.startsWith("..") && !path.isAbsolute(rel));
}

function displayPath(abs: string): string {
  const resolved = path.resolve(abs);
  if (safeDescendant(resolved, repoRoot)) return path.relative(repoRoot, resolved) || ".";
  if (safeDescendant(resolved, homeRoot)) return `$HOME/${path.relative(homeRoot, resolved)}`;
  return resolved;
}

function segmentMatches(pattern: string, name: string): boolean {
  if (!pattern.includes("*")) return pattern === name;
  const escaped = pattern
    .split("*")
    .map((part) => part.replace(/[|\\{}()[\]^$+?.]/g, "\\$&"))
    .join(".*");
  return new RegExp(`^${escaped}$`).test(name);
}

function expandPattern(baseAbs: string, relPattern: string): string[] {
  const segments = relPattern.split(/[\\/]+/).filter(Boolean);
  let current = [baseAbs];
  for (const segment of segments) {
    const next: string[] = [];
    for (const parent of current) {
      let entries: fs.Dirent[] = [];
      try {
        entries = fs.readdirSync(parent, { withFileTypes: true });
      } catch {
        continue;
      }
      for (const entry of entries) {
        if (!segmentMatches(segment, entry.name)) continue;
        const child = path.join(parent, entry.name);
        try {
          const stat = fs.lstatSync(child);
          if (stat.isSymbolicLink()) continue;
        } catch {
          continue;
        }
        next.push(child);
      }
    }
    current = next;
  }
  return current;
}

function sizeOf(abs: string): number {
  let stat: fs.Stats;
  try {
    stat = fs.lstatSync(abs);
  } catch {
    return 0;
  }
  if (stat.isSymbolicLink()) return 0;
  if (stat.isFile()) return stat.size;
  if (!stat.isDirectory()) return 0;
  let total = 0;
  let entries: string[] = [];
  try {
    entries = fs.readdirSync(abs);
  } catch {
    return 0;
  }
  for (const entry of entries) total += sizeOf(path.join(abs, entry));
  return total;
}

function collectCandidates(policy: Json): { candidates: Candidate[]; root_reports: Json[] } {
  const roots = Array.isArray(policy.candidate_roots) ? policy.candidate_roots as Json[] : [];
  const candidates: Candidate[] = [];
  const rootReports: Json[] = [];
  for (const root of roots) {
    const id = String(root.id || "");
    const baseRaw = String(root.base || "");
    const baseAbs = resolveBase(baseRaw);
    const kind = String(root.candidate_kind || "");
    const role = String(root.role || "");
    const cleanupStrategy = String(root.cleanup_strategy || "");
    const minBytes = Number(root.min_candidate_bytes || 0);
    const beforeCount = candidates.length;
    if (kind === "file_prefix") {
      const prefix = String(root.name_prefix || "");
      let entries: fs.Dirent[] = [];
      try {
        entries = fs.readdirSync(baseAbs, { withFileTypes: true });
      } catch {
        entries = [];
      }
      for (const entry of entries) {
        if (!entry.isFile() || !entry.name.startsWith(prefix)) continue;
        const abs = path.join(baseAbs, entry.name);
        const bytes = sizeOf(abs);
        if (bytes < minBytes) continue;
        candidates.push({
          id: `${id}:${entry.name}`,
          source_id: id,
          kind: "file",
          abs_path: abs,
          display_path: displayPath(abs),
          bytes,
          role,
          cleanup_strategy: cleanupStrategy,
        });
      }
    } else if (kind === "directory_patterns") {
      const patterns = Array.isArray(root.relative_patterns) ? root.relative_patterns.map(String) : [];
      for (const relPattern of patterns) {
        for (const abs of expandPattern(baseAbs, relPattern)) {
          let stat: fs.Stats;
          try {
            stat = fs.lstatSync(abs);
          } catch {
            continue;
          }
          if (!stat.isDirectory() || stat.isSymbolicLink()) continue;
          const bytes = sizeOf(abs);
          if (bytes < minBytes) continue;
          candidates.push({
            id: `${id}:${relPattern}:${displayPath(abs)}`,
            source_id: id,
            kind: "directory",
            abs_path: abs,
            display_path: displayPath(abs),
            bytes,
            role,
            cleanup_strategy: cleanupStrategy,
          });
        }
      }
    }
    const rows = candidates.slice(beforeCount);
    rootReports.push({
      id,
      base: baseRaw,
      resolved_base: displayPath(baseAbs),
      candidate_kind: kind,
      cleanup_strategy: cleanupStrategy,
      candidate_count: rows.length,
      candidate_bytes: rows.reduce((sum, row) => sum + row.bytes, 0),
    });
  }
  return { candidates, root_reports: rootReports };
}

function canDelete(candidate: Candidate, policy: Json): boolean {
  if (!candidate.abs_path || candidate.abs_path === "/" || candidate.abs_path === homeRoot || candidate.abs_path === repoRoot) return false;
  const roots = Array.isArray(policy.candidate_roots) ? policy.candidate_roots as Json[] : [];
  const source = roots.find((root) => String(root.id || "") === candidate.source_id);
  if (!source || source.apply_allowed !== true) return false;
  const baseAbs = resolveBase(String(source.base || ""));
  if (!safeDescendant(candidate.abs_path, baseAbs)) return false;
  if (candidate.kind === "file") {
    const prefix = String(source.name_prefix || "");
    return !!prefix && path.basename(candidate.abs_path).startsWith(prefix);
  }
  if (candidate.kind === "directory") {
    const rel = path.relative(baseAbs, candidate.abs_path).replace(/\\/g, "/");
    const patterns = Array.isArray(source.relative_patterns) ? source.relative_patterns.map(String) : [];
    return patterns.some((pattern) => {
      const expanded = expandPattern(baseAbs, pattern).map((row) => path.relative(baseAbs, row).replace(/\\/g, "/"));
      return expanded.includes(rel);
    });
  }
  return false;
}

function removeCandidate(candidate: Candidate): boolean {
  fs.rmSync(candidate.abs_path, { recursive: candidate.kind === "directory", force: true });
  return true;
}

const policyRel = arg("policy", "validation/conformance/contracts/host_storage_pressure_policy.json");
const strict = flag("strict");
const apply = flag("apply");
const ack = flag("ack-host-storage-cleanup");
const policy = readJson(policyRel);
const outRel = arg("out-json", String(policy.report_path || "core/local/artifacts/host_storage_pressure_guard_current.json"));
const maxDeletePerRun = Number(policy.max_delete_per_run || 200);
const failBytes = Number(policy.fail_if_candidate_bytes_exceeds || 0);
const before = collectCandidates(policy);
const dryRun = !apply || !ack;
const actions: Json[] = [];
let reclaimedBytes = 0;
let deletedEntries = 0;

for (const candidate of before.candidates.slice(0, maxDeletePerRun)) {
  const action: Json = {
    display_path: candidate.display_path,
    kind: candidate.kind,
    bytes: candidate.bytes,
    source_id: candidate.source_id,
    dry_run: dryRun,
    ok: false,
  };
  if (dryRun) {
    action.ok = true;
    action.skipped = true;
    action.reason = apply ? "missing_ack_host_storage_cleanup" : "dry_run";
    actions.push(action);
    continue;
  }
  if (!canDelete(candidate, policy)) {
    action.skipped = true;
    action.reason = "unsafe_candidate";
    actions.push(action);
    continue;
  }
  removeCandidate(candidate);
  deletedEntries += 1;
  reclaimedBytes += candidate.bytes;
  action.ok = true;
  actions.push(action);
}

const after = collectCandidates(policy);
const candidateBytesAfter = after.candidates.reduce((sum, row) => sum + row.bytes, 0);
const ok = !strict || candidateBytesAfter <= failBytes;
const payload = {
  type: "host_storage_pressure_guard",
  generated_at: new Date().toISOString(),
  trace_id: `validation:${new Date().toISOString()}:host-storage-pressure`,
  span_id: `span:validation:${new Date().toISOString()}:host-storage-pressure`,
  source_domain: "validation",
  owner_domain: "validation.host_storage",
  policy_path: policyRel,
  ok,
  strict,
  dry_run: dryRun,
  apply_requested: apply,
  ack_present: ack,
  before: {
    candidate_count: before.candidates.length,
    candidate_bytes: before.candidates.reduce((sum, row) => sum + row.bytes, 0),
    roots: before.root_reports,
    candidates: before.candidates.map((row) => ({
      display_path: row.display_path,
      kind: row.kind,
      bytes: row.bytes,
      role: row.role,
      cleanup_strategy: row.cleanup_strategy,
    })),
  },
  actions,
  deleted_entries: deletedEntries,
  reclaimed_bytes_estimate: reclaimedBytes,
  after: {
    candidate_count: after.candidates.length,
    candidate_bytes: candidateBytesAfter,
    roots: after.root_reports,
    candidates: after.candidates.map((row) => ({
      display_path: row.display_path,
      kind: row.kind,
      bytes: row.bytes,
      role: row.role,
      cleanup_strategy: row.cleanup_strategy,
    })),
  },
  violations: ok ? [] : [{
    kind: "host_storage_cleanup_candidates_present",
    candidate_count: after.candidates.length,
    candidate_bytes: candidateBytesAfter,
  }],
};

const outPath = path.join(repoRoot, outRel);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({
  ok,
  dry_run: payload.dry_run,
  candidate_count_before: payload.before.candidate_count,
  candidate_bytes_before: payload.before.candidate_bytes,
  deleted_entries: payload.deleted_entries,
  reclaimed_bytes_estimate: payload.reclaimed_bytes_estimate,
  candidate_count_after: payload.after.candidate_count,
  candidate_bytes_after: payload.after.candidate_bytes,
  out_json: outRel,
}, null, 2));

if (!ok) process.exit(1);
